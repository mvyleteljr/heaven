import { hierarchy, pack } from "./vendor/d3-hierarchy.js";

const esc = (value) => String(value ?? "").replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[char]));
const entries = (value) => Object.entries(value ?? {});
const label = (value) => String(value).replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/[_-]/g, " ").replace(/^./, (char) => char.toUpperCase());
const display = (value) => typeof value === "string" ? value : JSON.stringify(value);

// Read declaration headings from recorded, compiler-printed Ontolang. This is a
// display index, not a compiler: executable operations still come from state.stories.
function concepts(state) {
  const types = new Map(), storyGroups = new Map();
  for (const source of Object.values(state.definition?.source ?? {})) {
    let group, beatIndent;
    for (const line of String(source).split(/\r?\n/)) {
      const declaration = /^(actor|object|story|project)\s+([A-Za-z_][\w-]*)\s*(?:#.*)?$/.exec(line);
      if (declaration) {
        const [, kind, name] = declaration;
        group = kind === "story" ? name : undefined;
        beatIndent = undefined;
        if (kind === "actor" || kind === "object") types.set(name, kind);
      } else if (/^\S/.test(line) && !line.startsWith("#")) {
        group = undefined;
      } else if (group && /^\s+[^\s#]/.test(line)) {
        const indent = line.length - line.trimStart().length;
        beatIndent ??= indent;
        if (indent === beatIndent) storyGroups.set(line.trim(), group);
      }
    }
  }
  return { types, storyGroups };
}

// Domain concepts only. HTTP resources, files and transport hosts remain in Facts.
export function worldTree(world = {}) {
  const node = (parent, id, name, kind, children = [], fields) => ({ id: `${parent}/${encodeURIComponent(id)}`, name: String(name), kind, children, fields });
  const root = { id: "world", name: world.ontology ? "Institution" : "World", kind: "world", children: [] };
  function records(parent, id, name, values) {
    const group = node(parent, id, name, "domain");
    group.children = entries(values).map(([key, value]) => node(group.id, value?.id ?? key, value?.name ?? value?.title ?? value?.label ?? value?.id ?? key, "record", [], value));
    return group;
  }
  const foundation = concepts(world.ontology ?? {});
  function institution(id, name, state, module = false) {
    const domain = module ? node(root.id, id, name, "domain") : root;
    const model = concepts(state), groups = new Map();
    const inheritedTypes = new Set(module ? [...foundation.types.keys(), ...(state.protected_types ?? [])] : []);
    const inheritedRecords = new Set(module ? (state.protected ?? []).map((record) => record.id) : []);
    const actors = node(domain.id, "actors", module ? "Actors" : "Shared actors", "category");
    const objects = node(domain.id, "objects", module ? "Objects" : "Shared objects", "category");
    function typeGroup(type, kind = "object") {
      if (!groups.has(type)) {
        const category = kind === "actor" ? actors : objects;
        const group = node(category.id, `type:${type}`, type, kind, [], { type, kind });
        category.children.push(group);
        groups.set(type, group);
      }
      return groups.get(type);
    }
    for (const [type, kind] of model.types) if (!inheritedTypes.has(type)) typeGroup(type, kind);
    for (const instance of state.snapshot?.instances ?? []) {
      const type = instance.type ?? "Record";
      if (inheritedTypes.has(type) || inheritedRecords.has(instance.id)) continue;
      const group = typeGroup(type, model.types.get(type));
      group.children.push(node(group.id, instance.id, instance.fields?.name ?? instance.id ?? type, "instance", [], instance.fields ?? instance));
    }
    domain.children = [actors, objects].filter((group) => group.children.length);
    const stories = new Map(), storyCategory = node(domain.id, "stories", "Stories", "category");
    for (const [index, story] of (state.stories ?? []).entries()) {
      const title = model.storyGroups.get(story.story) ?? "Stories";
      if (!stories.has(title)) stories.set(title, node(storyCategory.id, `story:${title}`, label(title), "Story group", [], { operations: [] }));
      const group = stories.get(title);
      group.fields.operations.push(story.story ?? story.id ?? `Story ${index + 1}`);
    }
    storyCategory.children = [...stories.values()];
    if (storyCategory.children.length) domain.children.push(storyCategory);
    // A reference is not containment: shared actors remain at Institution scope.
    if (module) {
      const used = new Set((state.stories ?? []).flatMap((story) => String(story.story ?? "").match(/[A-Za-z_][\w-]*/g) ?? []));
      domain.references = [...foundation.types].filter(([type]) => used.has(type)).map(([type, kind]) => ({
        name: type, id: `${root.id}/${kind === "actor" ? "actors" : "objects"}/${encodeURIComponent(`type:${type}`)}`,
      }));
    }
    return domain;
  }
  if (world.ontology) {
    institution("foundation", "Institution", world.ontology);
    root.children.push(...entries(world.ontology.modules).map(([name, state]) => institution(`module:${name}`, label(name), state, true)));
  }
  if (world.business) {
    const business = node(root.id, "business", "Harbor Works", "domain");
    business.children = ["organizations", "offers", "contracts", "services", "alternatives", "ledger"].filter((key) => entries(world.business[key]).length)
      .map((key) => records(business.id, key, label(key), world.business[key]));
    root.children.push(business);
  }
  if (world.flowers) {
    const flowers = world.flowers, domain = node(root.id, "flowers", flowers.florist ?? "Florist", "domain");
    const fields = Object.fromEntries(entries(flowers).filter(([key]) => !["time", "catalog", "delivery", "order"].includes(key)));
    domain.children.push(node(domain.id, "shop", "Shop & budget", "record", [], fields));
    if (flowers.catalog) domain.children.push(records(domain.id, "catalog", "Catalog", flowers.catalog));
    if (flowers.delivery) {
      const delivery = records(domain.id, "delivery", "Delivery", flowers.delivery.options);
      if (flowers.delivery.policies) delivery.children.push(node(delivery.id, "policies", "Policies", "record", [], { policies: flowers.delivery.policies }));
      domain.children.push(delivery);
    }
    if (flowers.order) domain.children.push(node(domain.id, "order", "Order", "record", [], flowers.order));
    root.children.push(domain);
  }
  function count(current) {
    current.children.forEach(count);
    current.instanceCount = ["instance", "record"].includes(current.kind) ? 1 : current.children.reduce((sum, child) => sum + child.instanceCount, 0);
    current.typeCount = ["actor", "object"].includes(current.kind) ? 1 : current.children.reduce((sum, child) => sum + child.typeCount, 0);
    current.storyCount = current.kind === "Story group" ? current.fields.operations.length : current.children.reduce((sum, child) => sum + child.storyCount, 0);
    current.count = current.instanceCount + current.storyCount;
  }
  count(root);
  return root;
}

export function findDomain(root, id) {
  if (root.id === id) return root;
  for (const child of root.children) { const found = findDomain(child, id); if (found) return found; }
  return null;
}

function fieldsHtml(fields) {
  if (fields === undefined) return '<p class="fact-note">No records in this domain at this checkpoint.</p>';
  const pairs = fields !== null && typeof fields === "object" ? entries(fields) : [["Value", fields]];
  return `<dl class="domain-fields">${pairs.map(([key, value]) => `<div data-world-key="${esc(key)}"><dt>${esc(label(key))}</dt><dd>${esc(display(value))}</dd></div>`).join("")}</dl>`;
}

const plural = (n, word) => `${n} ${word}${n === 1 ? "" : "s"}`;
function summary(node) {
  if (["actor", "object"].includes(node.kind)) return `${node.kind} · ${plural(node.instanceCount, "instance")}`;
  if (node.kind === "instance") return String(node.fields?.status ?? "instance");
  if (node.kind === "Story group") return plural(node.storyCount, "action");
  return [node.typeCount ? plural(node.typeCount, "type") : "", plural(node.instanceCount, "instance"), node.storyCount ? plural(node.storyCount, "action") : ""].filter(Boolean).join(" · ");
}
function referencesHtml(node) {
  return node.references?.length ? `<p class="concept-references">Uses shared ${node.references.map((ref) => `<button type="button" data-world-focus="${esc(ref.id)}">${esc(ref.name)}</button>`).join(", ")}</p>` : "";
}
function flatten(root) { return [root, ...root.children.flatMap(flatten)]; }
function changesHtml(root, previous) {
  if (!previous) return "";
  const prior = new Map(flatten(worldTree(previous)).map((node) => [node.id, node]));
  const changes = [];
  for (const node of flatten(root)) {
    const old = prior.get(node.id);
    if (node.kind === "domain" && !old) changes.push(`Domain added: ${node.name} — ${summary(node)}.${node.storyCount ? " Stories available." : ""}`);
    if (node.kind === "instance" && !old) changes.push(`Instance created: ${node.name}${node.fields?.status != null ? ` · status: ${node.fields.status}` : ""}`);
    if (node.kind === "instance" && old) {
      for (const [field, value] of entries(node.fields)) if (JSON.stringify(old.fields?.[field]) !== JSON.stringify(value))
        changes.push(`${node.name} · ${label(field)}: ${display(old.fields?.[field]) ?? "unset"} → ${display(value)}`);
      for (const field of Object.keys(old.fields ?? {})) if (!(field in (node.fields ?? {}))) changes.push(`${node.name} · ${label(field)} removed`);
    }
  }
  const ids = new Set(flatten(root).map((node) => node.id));
  for (const node of prior.values()) if (node.kind === "instance" && !ids.has(node.id)) changes.push(`Instance removed: ${node.name}`);
  return changes.length ? `<ul class="ontology-changes" aria-label="Recorded world changes">${changes.map((change) => `<li data-world-key="${esc(change)}">${esc(change)}</li>`).join("")}</ul>` : "";
}

export function renderTree(root, depth = 0) {
  return `<ul class="domain-list world-tree" data-world-key="${esc(root.id)}"><li data-world-key="${esc(root.id)}"><details${depth < 2 ? " open" : ""}><summary><span>${esc(root.name)}</span><small>${esc(summary(root))}</small></summary>${referencesHtml(root)}${root.children.length ? root.children.map((child) => renderTree(child, depth + 1)).join("") : fieldsHtml(root.fields)}</details></li></ul>`;
}

// The overview shows concepts, not every storage/grouping layer. Instances are
// revealed only when their actor/object type is the selected focus.
function packProjection(node, focusId) {
  const children = ["actor", "object"].includes(node.kind) && node.id !== focusId ? []
    : node.children.flatMap((child) => child.kind === "category" ? child.children : [child]);
  return { ...node, children: children.map((child) => packProjection(child, focusId)) };
}

export function packDomain(root) {
  const layout = hierarchy(packProjection(root, root.id)).sum((node) => node.children.length ? 0 : 1);
  // Stable source order keeps equal-size records together across checkpoints.
  return pack().size([600, 600]).padding(12)(layout);
}

export function renderStructure(world, view, requestedFocus = "world", previous) {
  const root = worldTree(world);
  if (view === "tree") return `${changesHtml(root, previous)}${renderTree(root)}`;
  const focus = findDomain(root, requestedFocus) ?? root;
  const ancestors = [];
  let current = focus.id;
  while (current) {
    const node = findDomain(root, current);
    if (node && (node.kind !== "category" || node === focus)) ancestors.unshift(node);
    current = current.includes("/") ? current.slice(0, current.lastIndexOf("/")) : "";
  }
  const breadcrumbs = `<nav class="domain-path" aria-label="Domain location">${ancestors.map((node, index) => `${index ? '<span aria-hidden="true">/</span>' : ""}<button type="button" data-world-focus="${esc(node.id)}"${node === focus ? ' aria-current="location"' : ""}>${esc(node.name)}</button>`).join("")}</nav>`;
  const layout = packDomain(focus);
  const shapes = layout.descendants().map((node) => {
    const { x, y, r, depth, data } = node;
    return `<g data-world-key="${esc(data.id)}"${depth ? ` data-world-focus="${esc(data.id)}" role="button" tabindex="0" aria-label="Inspect ${esc(data.name)}, ${esc(summary(data))}"` : ""}><title>${esc(data.name)} · ${esc(summary(data))}</title><circle cx="${x}" cy="${y}" r="${r}" class="pack-circle depth-${depth % 4} kind-${data.kind === "Story group" ? "story" : data.kind}"/></g>`;
  }).join("");
  const labels = layout.descendants().filter((node) => node.depth > 0 && node.r > 24 && !(node.data.kind === "category" && node.data.children.length === 1)).map(({ x, y, r, data }) => {
    const limit = Math.min(32, Math.floor(r / 4));
    const shorten = (text) => text.length > limit ? `${text.slice(0, limit - 1)}…` : text;
    const secondary = ["actor", "object"].includes(data.kind) ? plural(data.instanceCount, "instance")
      : data.kind === "instance" ? data.fields?.status ?? "" : "";
    const top = data.children.length ? y - r + Math.min(23, r) : y + (secondary ? -5 : 4);
    return `<text class="pack-label" x="${x}" y="${top}" text-anchor="middle" font-size="${Math.min(16, r / 4)}">${esc(shorten(data.name))}</text>${secondary ? `<text class="pack-detail" x="${x}" y="${top + 16}" text-anchor="middle" font-size="12">${esc(shorten(String(secondary)))}</text>` : ""}`;
  }).join("");
  const diagram = focus === root && !root.children.length ? '<p class="fact-note">No domain concepts are modeled at this checkpoint.</p>' : focus.children.length ? `<figure class="domain-map"><svg viewBox="0 0 600 600" role="group" aria-label="Nested domains in ${esc(focus.name)}">${shapes}${labels}</svg><figcaption class="fact-note">Select a concept to inspect its instances or actions.</figcaption></figure><ul class="pack-children">${layout.data.children.map((child) => `<li data-world-key="${esc(child.id)}"><button type="button" data-world-focus="${esc(child.id)}"><span>${esc(child.name)}</span><small>${esc(summary(child))} <span aria-hidden="true">↗</span></small></button>${referencesHtml(child)}</li>`).join("")}</ul>` : `<section class="domain-inspector"><h3 class="fact-title serif">${esc(focus.name)}</h3><p class="fact-note">${esc(summary(focus))}</p>${fieldsHtml(focus.fields)}</section>`;
  return `<section data-world-key="circle-pack">${breadcrumbs}${referencesHtml(focus)}${changesHtml(root, previous)}${diagram}<details class="all-domains"><summary>Browse all domains</summary>${renderTree(root)}</details></section>`;
}
