import { hierarchy, pack } from "./vendor/d3-hierarchy.js";

const esc = (value) => String(value ?? "").replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[char]));
const entries = (value) => Object.entries(value ?? {});
const label = (value) => String(value).replace(/_/g, " ").replace(/^./, (char) => char.toUpperCase());
const display = (value) => typeof value === "string" ? value : JSON.stringify(value);

// Structure reflects recorded containment, not inferred relationships between entities.
export function worldTree(world = {}) {
  const node = (parent, id, name, kind, children = [], fields) => ({ id: `${parent}/${encodeURIComponent(id)}`, name: String(name), kind, children, fields });
  const root = { id: "world", name: "World", kind: "world", children: [] };
  function records(parent, id, name, values) {
    const group = node(parent, id, name, "domain");
    group.children = entries(values).map(([key, value]) => node(group.id, value?.id ?? key, value?.name ?? value?.title ?? value?.label ?? value?.id ?? key, "record", [], value));
    return group;
  }
  function institution(id, name, state) {
    const domain = node(root.id, id, name, "domain");
    const groups = new Map();
    for (const instance of state.snapshot?.instances ?? []) {
      const type = instance.type ?? "Record";
      if (!groups.has(type)) groups.set(type, node(domain.id, `type:${type}`, type, "type"));
      const group = groups.get(type);
      group.children.push(node(group.id, instance.id, instance.fields?.name ?? instance.id ?? type, "record", [], instance.fields ?? instance));
    }
    domain.children = [...groups.values()];
    if (state.stories?.length) {
      const stories = node(domain.id, "stories", "Executable Stories", "operations");
      stories.children = state.stories.map((story, index) => node(stories.id, story.id ?? index, story.story ?? story.id ?? "Story", "Story", [], story));
      domain.children.push(stories);
    }
    return domain;
  }
  if (world.ontology) {
    root.children.push(institution("foundation", "Company foundation", world.ontology));
    root.children.push(...entries(world.ontology.modules).map(([name, state]) => institution(`module:${name}`, name, state)));
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
  const hosts = new Map();
  for (const [url, resource] of entries(world.tools?.public_resources)) {
    let host;
    try { host = new URL(url).host; } catch { host = "Public resources"; }
    if (!hosts.has(host)) hosts.set(host, node(root.id, `host:${host}`, host, "domain"));
    const group = hosts.get(host);
    group.children.push(node(group.id, url, resource.title ?? url, "resource", [], { url, ...resource }));
  }
  root.children.push(...hosts.values());
  for (const [id, name, values] of [
    ["workspace", "Workspace", world.tools?.workspace?.files], ["messages", "Mailbox", world.tools?.messages],
    ["documents", "Documents", world.documents], ["hosting", "Hosted services", world.hosting],
  ]) if (entries(values).length) root.children.push(records(root.id, id, name, values));
  function count(current) {
    current.count = current.children.length ? current.children.reduce((sum, child) => sum + count(child), 0) : current.fields === undefined ? 0 : 1;
    return current.count;
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

export function renderTree(root, depth = 0) {
  return `<ul class="domain-list world-tree" data-world-key="${esc(root.id)}"><li data-world-key="${esc(root.id)}"><details${depth < 2 ? " open" : ""}><summary><span>${esc(root.name)}</span><small>${esc(root.kind)}${root.children.length ? ` · ${root.count} items` : ""}</small></summary>${root.children.length ? root.children.map((child) => renderTree(child, depth + 1)).join("") : fieldsHtml(root.fields)}</details></li></ul>`;
}

export function packDomain(root) {
  const layout = hierarchy(root).sum((node) => node.children.length ? 0 : 1);
  // Stable source order keeps equal-size records together across checkpoints.
  return pack().size([600, 600]).padding(12)(layout);
}

export function renderStructure(world, view, requestedFocus = "world") {
  const root = worldTree(world);
  if (view === "tree") return renderTree(root);
  const focus = findDomain(root, requestedFocus) ?? root;
  const ancestors = [];
  let current = focus.id;
  while (current) {
    const node = findDomain(root, current);
    if (node) ancestors.unshift(node);
    current = current.includes("/") ? current.slice(0, current.lastIndexOf("/")) : "";
  }
  const breadcrumbs = `<nav class="domain-path" aria-label="Domain location">${ancestors.map((node, index) => `${index ? '<span aria-hidden="true">/</span>' : ""}<button type="button" data-world-focus="${esc(node.id)}"${node === focus ? ' aria-current="location"' : ""}>${esc(node.name)}</button>`).join("")}</nav>`;
  const layout = packDomain(focus);
  const shapes = layout.descendants().map((node) => {
    const { x, y, r, depth, data } = node;
    return `<g data-world-key="${esc(data.id)}"${depth ? ` data-world-focus="${esc(data.id)}" role="button" tabindex="0" aria-label="Inspect ${esc(data.name)}, ${data.count} items"` : ""}><title>${esc(data.name)} · ${data.count} items</title><circle cx="${x}" cy="${y}" r="${r}" class="pack-circle depth-${depth % 4}"/></g>`;
  }).join("");
  const labels = layout.children?.filter((node) => node.r > 30).map(({ x, y, r, data }) => {
    const limit = Math.min(22, Math.floor(r / 4));
    const name = data.name.length > limit ? `${data.name.slice(0, limit - 1)}…` : data.name;
    return `<text class="pack-label" x="${x}" y="${y - r + Math.min(28, r)}" text-anchor="middle" font-size="${Math.min(16, r / 4)}">${esc(name)}</text>`;
  }).join("") ?? "";
  const diagram = focus.children.length ? `<figure class="domain-map"><svg viewBox="0 0 600 600" role="group" aria-label="Nested domains in ${esc(focus.name)}">${shapes}${labels}</svg><figcaption class="fact-note">Leaf circles represent recorded items; empty domains retain a placeholder. Enclosing circles group domains. Select a circle to inspect it.</figcaption></figure><ul class="pack-children">${focus.children.map((child) => `<li data-world-key="${esc(child.id)}"><button type="button" data-world-focus="${esc(child.id)}"><span>${esc(child.name)}</span><small>${child.count} items <span aria-hidden="true">↗</span></small></button></li>`).join("")}</ul>` : `<section class="domain-inspector"><h3 class="fact-title serif">${esc(focus.name)}</h3>${fieldsHtml(focus.fields)}</section>`;
  return `<section data-world-key="circle-pack">${breadcrumbs}${diagram}<details class="all-domains"><summary>Browse all domains</summary>${renderTree(root)}</details></section>`;
}
