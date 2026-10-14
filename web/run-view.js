import { renderStructure } from "./run-structure.js";
// Pure projections of persisted run evidence. This module never starts an action.
export const esc = (value) => String(value ?? "").replace(/[&<>"']/g, (char) =>
  ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char]);
const list = (value) => Array.isArray(value) ? value : [];
const entries = (value) => value && typeof value === "object" ? Object.entries(value) : [];
const pretty = (value) => JSON.stringify(value, null, 2) ?? "—";
const parse = (value) => {
  if (typeof value !== "string") return value;
  try { return JSON.parse(value); } catch { return value; }
};
const text = (value) => typeof value === "string" ? value : pretty(value);
const compact = (value, limit = 170) => {
  const content = text(value).replace(/\s+/g, " ");
  return content.length > limit ? content.slice(0, limit - 1) + "…" : content;
};
const label = (value) => String(value).replace(/_/g, " ").replace(/\b\w/g, (char) => char.toUpperCase());
const raw = (title, value) => `<details><summary>${esc(title)}</summary><pre>${esc(text(value))}</pre></details>`;
const stable = (value) => {
  if (Array.isArray(value)) return value.map(stable);
  if (value && typeof value === "object") return Object.fromEntries(Object.keys(value).sort().map((key) => [key, stable(value[key])]));
  return value;
};
const same = (left, right) => JSON.stringify(stable(left)) === JSON.stringify(stable(right));
const omit = (value, keys) => Object.fromEntries(entries(value).filter(([key]) => !keys.includes(key)));

export function experimentName(runOrKey) {
  const run = typeof runOrKey === "object" && runOrKey ? runOrKey : null;
  let key = run?.experiment ?? runOrKey;
  if (key === "institutions" && (run?.world?.ontology?.generation || entries(run?.world?.ontology?.modules).length)) key = "institutions_generated";
  return ({ flowers: "Flowers for Friday", bellwether: "Harbor Works", institutions: "Onto · Accounts", harbor_onto: "Onto · Harbor Works", institutions_generated: "Onto · Generated company", institutions_expansion: "Onto · Missing domain demo", onto_review: "Onto · Story review", records: "Records", recall: "Archive handoff" })[key] ?? run?.name ?? "Experiment";
}

export function traceFrames(data) {
  const workflows = new Map();
  return list(data?.frames).filter((frame) => {
    const kind = frame.event?.kind;
    return !!frame.event?.evidence?.action || ["seed", "fork", "outcome", "expansion", "denied", "error", "recovery", "intervention"].includes(kind);
  }).map((frame) => {
    const event = frame.event ?? {}, evidence = event.evidence ?? {};
    const id = evidence.action_id;
    if (event.kind === "action_blocked") workflows.set(id, {
      id: `action:${id}`, module: evidence.module, requires: evidence.requires ?? [], dependencies: evidence.dependencies ?? [],
    });
    const workflow = workflows.get(id);
    if (!workflow) return frame;
    const phase = event.kind === "action_blocked" ? "blocked" : event.kind === "action_ready" ? "ready" : actionState(frame);
    return { ...frame, workflow: { ...workflow, phase } };
  });
}

export function actionState(frame) {
  const { kind, evidence = {} } = frame?.event ?? {};
  if (kind === "action_blocked" || frame?.workflow?.phase === "blocked") return "blocked";
  if (kind === "action_ready") return "ready";
  const result = evidence.result, body = parse(result?.body);
  if (["denied", "error"].includes(kind) || result?.error != null || body?.error != null
    || [result, body, body?.result].some((value) => value?.committed === false)
    || result?.status >= 400 || (typeof result?.exit_status === "number" && result.exit_status !== 0)) return "failed";
  return result !== undefined ? "succeeded" : "pending";
}

function ontologyFacts(state) {
  return {
    instances: list(state?.snapshot?.instances).slice().sort((left, right) => String(left.id).localeCompare(String(right.id))),
    stories: state?.stories ?? [],
    modules: Object.fromEntries(entries(state?.modules).map(([name, module]) => [name, ontologyFacts(module)])),
  };
}

// Time and evidence advance on reads and refusals. Only represented world state
// drives the fact-card transition; a new receipt alone is not a state change.
export function worldSignature(world = {}) {
  return JSON.stringify(stable({
    ontology: world.ontology ? ontologyFacts(world.ontology) : null,
    business: world.business ? omit(world.business, ["clock", "journal", "checkins"]) : null,
    flowers: world.flowers ? omit(world.flowers, ["time"]) : null,
    resources: world.tools?.public_resources ?? {},
    workspace: world.tools?.workspace ?? {},
    messages: world.tools?.messages ?? [],
    documents: world.documents ?? {},
    hosting: world.hosting ?? null,
  }));
}

function actionTitle(action) {
  const input = action.inputs ?? {};
  if (action.kind === "onto.execute") return input.story || `Execute Story · ${input.module || "foundation"}`;
  if (action.kind === "onto.inspect") return `Inspect ${input.view || "institution"} · ${input.module || "foundation"}`;
  if (action.kind === "onto.expand") return `Request capability · ${input.module || action.target || "module"}`;
  if (action.kind === "http.request") {
    const body = parse(input.body);
    const module = /^https:\/\/institutions\.local\/modules\/([a-z0-9-]+)\/?$/.exec(input.url ?? "");
    if (input.method === "GET" && module) return `Discover ${module[1].replace(/-/g, " ")}`;
    return typeof body?.story === "string" ? body.story : `${input.method ?? "HTTP"} ${input.url ?? action.target ?? "request"}`;
  }
  const subjects = {
    "shell.exec": input.command, "file.read": input.path, "file.write": input.path,
    "mail.send": input.subject ?? input.to, "mail.read": input.message_id,
    "history.search": input.query, "history.read": input.index,
    search: input.query, wait: `${input.seconds ?? 0} seconds`,
  };
  if (action.kind === "finish") return "Agent’s final report";
  return `${action.kind ?? "Action"}${subjects[action.kind] != null ? ` · ${subjects[action.kind]}` : action.target ? ` · ${action.target}` : ""}`;
}

function resultSummary(result, action, kind) {
  const body = parse(result?.body);
  const receipt = [result, body, body?.result].find((value) => typeof value?.committed === "boolean");
  if (receipt) return { title: receipt.committed ? "Committed" : "Rejected", detail: receipt.error ?? receipt.reason ?? receipt.message ?? (receipt.committed ? "The command receipt records a committed change." : "The command receipt records no committed change.") };
  const error = result?.error ?? body?.error;
  if (error != null) return { title: "Action failed", detail: text(error) };
  if (kind === "denied") return { title: "Action denied", detail: "The recorded operation was denied." };
  if (action?.kind === "finish") return { title: "Agent finished", detail: "This is the agent’s report; the saved world records the observed effects." };
  if (typeof result?.status === "number") return { title: `HTTP ${result.status}`, detail: result.status >= 400 ? "The service returned an error response." : "The service returned this response." };
  if (typeof result?.exit_status === "number") return { title: `Exit ${result.exit_status}`, detail: compact(result.stdout || result.stderr || "The shell command returned.") };
  if (result === undefined) return { title: "Awaiting result", detail: "No completed result is recorded for this checkpoint." };
  return { title: "Result recorded", detail: compact(result) };
}

export function agentPreview(frame, index) {
  const event = frame.event ?? {};
  const action = event.evidence?.action;
  return {
    key: frame.workflow?.id ?? String(event.seq ?? index), index,
    title: compact(action ? actionTitle(action) : ["seed", "fork"].includes(event.kind)
      ? "The starting world" : event.summary || label(event.kind || "Checkpoint"), 160),
  };
}

export function renderAgent(frame, index = 0, data = {}) {
  if (!frame) return `<article class="stage-card story-card"><div class="card-meta">Agent</div><h2 class="story-title serif">Choose an experiment</h2><p class="story-outcome">Run or step to begin its execution.</p></article>`;
  const event = frame.event ?? {};
  const evidence = event.evidence ?? {};
  const action = evidence.action;
  const isSeed = ["seed", "fork"].includes(event.kind);
  const title = action ? actionTitle(action) : isSeed ? (event.kind === "fork" ? "Continue from a saved world" : "An experiment, ready to begin") : event.summary ?? label(event.kind ?? "Checkpoint");
  let outcome = action ? resultSummary(evidence.result, action, event.kind) : { title: isSeed ? "Objective" : label(event.kind ?? "Recorded"), detail: isSeed ? frame.world?.goal ?? "No objective recorded." : evidence.error ?? evidence.note ?? event.summary ?? "" };
  const workflow = frame.workflow, phase = workflow?.phase;
  if (phase === "blocked") outcome = { title: "Blocked · missing capability", detail: "Waiting for the world generator to compile the missing Ontolang." };
  if (phase === "ready") outcome = { title: "Capability ready · retrying", detail: "Ontolang compiled and published. Resuming the original action." };
  if (phase === "succeeded") outcome = { ...outcome, title: "Action completed", detail: "The original action succeeded with the new capability available." };
  const resourceState = workflow && frame.world?.ontology?.modules?.[workflow.module];
  const requirements = workflow ? `<section class="action-resources"><h3>${phase === "succeeded" ? "Resources available" : "Required resources"}</h3><ul>${workflow.requires.map((resource) => `<li><span class="resource-status">${resourceState ? "Available" : "Missing"}</span> <span title="${esc(resource)}">${esc(workflow.module)}</span></li>`).join("")}${workflow.dependencies.map((resource) => `<li><span class="resource-status">Dependency</span> <span title="${esc(resource)}">Shared foundation</span></li>`).join("")}</ul>${resourceState ? `<p>${list(resourceState.snapshot?.instances).length} recorded objects · ${list(resourceState.stories).length} executable Stories</p>` : ""}</section>` : "";
  const report = action?.kind === "finish" ? action.inputs?.message ?? action.text : null;
  const input = action?.inputs;
  const domain = input?.url || (action?.kind?.startsWith("onto.") ? input?.module || "Company foundation" : null) || action?.target || experimentName(data?.run);
  return `<article class="stage-card story-card${workflow ? " workflow-card" : ""}"><div class="card-meta"><span>Checkpoint ${esc(index + 1)}</span><span>${esc(event.kind ?? "seed")}${event.seq != null ? ` · event ${esc(event.seq)}` : ""}</span></div><h2 class="story-title serif">${esc(compact(title, 220))}</h2>${workflow ? "" : `<div class="story-domain">${esc(domain)}</div>`}<div class="story-outcome"><strong>${esc(outcome.title)}</strong><p>${esc(outcome.detail)}</p></div>${requirements}${report ? `<p class="story-report">${esc(report)}</p>` : ""}${action ? raw("Action inputs", action) : ""}${evidence.result !== undefined ? raw("Recorded result", evidence.result) : !isSeed && entries(evidence).length ? raw("Recorded evidence", evidence) : ""}${isSeed && data?.initial?.actor_instructions ? raw("Actor instructions", data.initial.actor_instructions) : ""}</article>`;
}

// Financial resources come from the selected snapshot and its modeled owner.
// Do not infer a personal allowance from accounts a clerk is authorized to control.
export function agentFunds(world = {}) {
  const actor = world.ontology?.definition?.actor;
  if (actor && world.harbor_onto) {
    const firms = list(world.ontology.snapshot?.instances).filter((instance) => instance.type === "Firm" && instance.fields?.owner === actor);
    const fields = ["freeCash", "balance", "obligations", "reserved", "contingency"];
    if (firms.length && firms.every((firm) => fields.every((key) => Number.isFinite(firm.fields[key])))) {
      const total = (key) => firms.reduce((sum, firm) => sum + firm.fields[key], 0);
      return { available: total("freeCash"), balance: total("balance"), obligations: total("obligations"), reserved: total("reserved") + total("contingency"), source: firms.map((firm) => firm.id).join(", ") };
    }
  }
  if (world.flowers && Number.isFinite(world.flowers.balance_cents))
    return { available: world.flowers.balance_cents, source: "Customer spending allowance" };
  return null;
}

// Only fields recorded on the configured actor belong in the generic panel.
// Owned institutional accounts are not automatically a personal spending budget.
function agentAttributes(world) {
  const actor = world.ontology?.definition?.actor;
  if (!actor) return "";
  const seen = new Set();
  return [["", world.ontology], ...entries(world.ontology.modules)].map(([scope, state]) => {
    if (state?.definition?.agent_state) return "";
    const instance = list(state?.snapshot?.instances).find((item) => item.id === actor);
    const fields = entries(instance?.fields).filter(([key, value]) => key !== "name" || value !== actor.split(":").slice(1).join(":"));
    const signature = JSON.stringify(stable(fields));
    if (!fields.length || seen.has(signature)) return "";
    seen.add(signature);
    return `<div class="agent-state-attributes" data-world-key="agent-fields:${esc(scope)}">${scope ? `<span class="agent-state-scope">${esc(scope)}</span>` : ""}<dl>${fields.map(([key, value]) => `<div data-world-key="${esc(key)}"><dt>${esc(label(key.replace(/([a-z])([A-Z])/g, "$1 $2")))}</dt><dd title="${esc(text(value))}">${esc(compact(value))}</dd></div>`).join("")}</dl></div>`;
  }).join("");
}

function declaredAgentState(world) {
  return [["", world.ontology], ...entries(world.ontology?.modules)].map(([scope, state]) => {
    const declared = state?.agent_state;
    if (!declared) return "";
    const fields = list(declared.shape?.fields).filter((field) => !(field.source?.instance === world.ontology?.definition?.actor && field.source?.field === "name" && declared.values?.[field.key] === world.ontology?.definition?.actor?.split(":").slice(1).join(":")));
    if (!fields.length) return "";
    return `<div class="agent-state-attributes" data-world-key="declared-agent:${esc(scope)}">${scope ? `<span class="agent-state-scope">${esc(scope)}</span>` : ""}<dl>${fields.map((field) => {
      const value = declared.values?.[field.key];
      const display = value === undefined ? "Unavailable" : field.format === "usd_cents" && Number.isFinite(value) ? dollars(value) : text(value);
      return `<div data-world-key="${esc(field.key)}"><dt>${esc(field.label)}</dt><dd title="${esc(display)}">${esc(compact(display))}</dd></div>`;
    }).join("")}</dl></div>`;
  }).join("");
}

export function renderAgentState(frame) {
  const world = frame?.world ?? {}, funds = world.ontology?.definition?.agent_state ? null : agentFunds(world);
  const identity = world.ontology?.definition?.actor || (world.flowers ? "Purchasing agent" : "Experiment agent");
  const metrics = funds?.balance !== undefined ? [["Cash", funds.balance], ["Obligations", funds.obligations], ["Reserved", funds.reserved]] : [];
  const proportion = funds?.balance > 0 ? Math.min(1, Math.max(0, funds.available / funds.balance)) : 0;
  const attributes = declaredAgentState(world) + (world.ontology?.definition?.agent_state ? "" : agentAttributes(world));
  const budget = funds ? `<div class="agent-budget" title="${esc(funds.source)}"><span>Available to spend</span><strong class="serif">${dollars(funds.available)}</strong></div>` : "";
  return `<section class="agent-state-card" data-world-key="agent-state"><div class="agent-state-heading"><div class="agent-avatar" aria-hidden="true">${esc(identity.split(":").at(-1).slice(0, 1))}</div><div><h3 class="serif">${esc(identity)}</h3></div></div>${budget}${metrics.length ? `<meter class="agent-budget-meter" min="0" max="1" value="${proportion}" aria-label="Share of cash available to spend">${Math.round(proportion * 100)}%</meter><dl class="agent-state-metrics">${metrics.map(([name, value]) => `<div data-world-key="${name}"><dt>${name}</dt><dd>${dollars(value)}</dd></div>`).join("")}</dl>` : ""}${attributes}${funds ? `<p class="agent-budget-source sr-text">${esc(funds.source)}</p>` : ""}</section>`;
}

function valueHtml(value, old, compare) {
  const current = value === undefined ? "Removed" : compact(value, 240);
  if (!compare || same(value, old)) return `<span>${esc(current)}</span>`;
  return `<del>${esc(old === undefined ? "Not present" : compact(old, 160))}</del><span aria-label="changed to"> → </span><strong>${esc(current)}</strong>`;
}

function factRows(current, previous, compare) {
  const keys = [...new Set([...entries(current).map(([key]) => key), ...(compare ? entries(previous).map(([key]) => key) : [])])];
  return keys.map((key) => `<div data-world-key="${esc(key)}" class="fact-row${compare && !same(current?.[key], previous?.[key]) ? " changed" : ""}"><span>${esc(label(key))}</span><div class="fact-value">${valueHtml(current?.[key], previous?.[key], compare)}</div></div>`).join("");
}

function record(title, fields, before, compare, identity = title) {
  return `<section data-world-key="${esc(identity)}" class="fact-record"><h3 class="fact-title serif">${esc(title)}</h3>${factRows(fields, before, compare)}</section>`;
}

function ontologySection(name, state, before, compare) {
  const instances = list(state?.snapshot?.instances);
  const old = new Map(list(before?.snapshot?.instances).map((instance) => [instance.id, instance]));
  const groups = new Map();
  for (const instance of instances) {
    const type = instance.type ?? "Record";
    if (!groups.has(type)) groups.set(type, []);
    groups.get(type).push(instance);
  }
  const records = [...groups].map(([type, values]) => `<details data-world-key="${esc(type)}" class="fact-group"${values.length <= 3 && groups.size <= 3 ? " open" : ""}><summary>${esc(type)} · ${values.length} records</summary>${values.map((instance) => record(instance.fields?.name ?? instance.id ?? type, instance.fields ?? omit(instance, ["id", "type"]), old.get(instance.id)?.fields, compare, instance.id)).join("")}</details>`).join("");
  const deleted = compare ? [...old].filter(([id]) => !instances.some((instance) => instance.id === id)).map(([, instance]) => record(instance.id, {}, instance.fields, true)).join("") : "";
  return `<section data-world-key="${esc(name || "institution-facts")}" class="snapshot institution-snapshot">${name ? `<h3 class="fact-title serif">${esc(name)}</h3>` : ""}<p class="fact-note">${instances.length} records · ${list(state?.stories).length} executable Stories</p>${records || '<p class="fact-note">No records in this snapshot.</p>'}${deleted}${raw("Executable Stories", state?.stories ?? [])}${raw("Complete snapshot", state?.snapshot ?? {})}</section>`;
}

function businessSection(business, previous, compare) {
  const organizations = entries(business.organizations).map(([id, organization]) => record(organization.name ?? id, organization, previous?.organizations?.[id], compare)).join("");
  const sections = ["offers", "contracts", "services", "alternatives", "ledger"].map((key) => {
    const value = business[key];
    if (!entries(value).length) return "";
    const records = entries(value).map(([id, item]) => {
      const prior = Array.isArray(previous?.[key]) && item?.id ? previous[key].find((row) => row.id === item.id) : previous?.[key]?.[id];
      return record(item?.id ?? item?.name ?? `${label(key)} ${Number.isNaN(Number(id)) ? id : Number(id) + 1}`, item, prior, compare);
    }).join("");
    return `<details class="fact-group"><summary>${label(key)} · ${entries(value).length} records</summary>${records}</details>`;
  }).join("");
  return `<section class="snapshot"><h3 class="fact-title serif">Organizations & agreements</h3><p class="fact-note">${entries(business.organizations).length} organizations · ${list(business.contracts).length} contracts. Cash and prices are integer cents.</p><details class="fact-group"${entries(business.organizations).length <= 3 ? " open" : ""}><summary>Organizations · ${entries(business.organizations).length} records</summary>${organizations}</details>${sections}</section>`;
}

const dollars = (cents) => Number.isFinite(cents) ? new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" }).format(cents / 100) : "Not recorded";
const arrival = (day, hour) => Number.isInteger(hour) && hour >= 0 && hour <= 23
  ? `${day ? `${day}, ` : ""}${hour % 12 || 12} ${hour < 12 ? "a.m." : "p.m."}` : day || "Not recorded";

function flowersSection(flowers, previous, compare) {
  const summary = (state) => {
    if (!state) return undefined;
    const order = state.order;
    const fields = { available_budget: dollars(state.balance_cents), order_status: order?.status ?? "Not placed" };
    if (order) {
      if (order.confirmation) fields.confirmation = order.confirmation;
      if (order.arrangement) fields.arrangement = order.arrangement;
      fields[order.status === "confirmed" ? "confirmed_total" : "order_total"] = dollars(order.total_cents);
      if (Number.isFinite(order.delivery_fee_cents)) fields.delivery_fee = dollars(order.delivery_fee_cents);
      fields.delivery = arrival(order.day, order.arrival_hour);
      if (order.address) fields.delivery_address = order.address;
      if (order.message) fields.card_message = order.message;
    } else if (state.recipient) fields.delivery_address = state.recipient;
    return fields;
  };
  const productFields = (product) => product ? {
    price: dollars(product.price_cents), description: product.description ?? "Not recorded",
    sympathy_appropriate: product.sympathy_appropriate === true ? "Yes" : product.sympathy_appropriate === false ? "No" : "Not recorded",
  } : undefined;
  const deliveryFields = (option) => option ? { fee: dollars(option.fee_cents), arrival: arrival("Friday", option.arrival_hour) } : undefined;
  const catalog = list(flowers.catalog).map((product) => record(product.name ?? product.id, productFields(product), productFields(list(previous?.catalog).find((old) => old.id === product.id)), compare)).join("");
  const delivery = list(flowers.delivery?.options).map((option) => record(option.label ?? option.id, deliveryFields(option), deliveryFields(list(previous?.delivery?.options).find((old) => old.id === option.id)), compare)).join("");
  return `<section class="snapshot">${record("Order & budget", summary(flowers), summary(previous), compare)}<details class="fact-group"><summary>Catalog · ${flowers.catalog ? `${list(flowers.catalog).length} arrangement${list(flowers.catalog).length === 1 ? "" : "s"}` : "not constructed"}</summary>${catalog || '<p class="fact-note">No catalog is recorded at this checkpoint.</p>'}</details><details class="fact-group"><summary>Delivery · ${flowers.delivery ? `${list(flowers.delivery.options).length} option${list(flowers.delivery.options).length === 1 ? "" : "s"}` : "not constructed"}</summary>${delivery || '<p class="fact-note">No delivery options are recorded at this checkpoint.</p>'}${flowers.delivery?.policies ? `<p class="fact-note">${esc(flowers.delivery.policies)}</p>` : ""}</details>${raw("Complete florist state", omit(flowers, ["time"]))}</section>`;
}

function resourcesSection(world, previous, compare) {
  const current = world.tools?.public_resources ?? {};
  const old = previous?.tools?.public_resources ?? {};
  const resources = entries(current).map(([url, resource]) => record(resource.title || url, { url, ...omit(resource, ["title"]) }, old[url] ? { url, ...omit(old[url], ["title"]) } : undefined, compare) + raw("Complete resource", resource)).join("");
  const files = world.tools?.workspace?.files ?? {};
  const oldFiles = previous?.tools?.workspace?.files ?? {};
  const filesHtml = entries(files).map(([path, contents]) => {
    const decode = (value) => Array.isArray(value) ? new TextDecoder().decode(new Uint8Array(value)) : text(value);
    const fields = { bytes: Array.isArray(contents) ? contents.length : text(contents).length, content: decode(contents) };
    const before = oldFiles[path] === undefined ? undefined : { bytes: Array.isArray(oldFiles[path]) ? oldFiles[path].length : text(oldFiles[path]).length, content: decode(oldFiles[path]) };
    return `${record(path, fields, before, compare)}${raw(`File · ${path}`, decode(contents))}`;
  }).join("");
  const messages = list(world.tools?.messages).map((message) => record(message.subject ?? message.id, omit(message, ["subject"]), list(previous?.tools?.messages).find((item) => item.id === message.id) ? omit(list(previous.tools.messages).find((item) => item.id === message.id), ["subject"]) : undefined, compare)).join("");
  const documents = entries(world.documents).map(([id, document]) => record(document.title ?? id, omit(document, ["title"]), previous?.documents?.[id] ? omit(previous.documents[id], ["title"]) : undefined, compare)).join("");
  return `${resources ? `<details class="snapshot"${world.ontology || world.business || world.flowers ? "" : " open"}><summary>Public resources · ${entries(current).length}</summary>${resources}</details>` : ""}${filesHtml ? `<section class="snapshot"><h3 class="fact-title serif">Workspace</h3>${filesHtml}</section>` : ""}${messages ? `<section class="snapshot"><h3 class="fact-title serif">Messages</h3>${messages}</section>` : ""}${documents ? `<section class="snapshot"><h3 class="fact-title serif">Documents</h3>${documents}</section>` : ""}${world.hosting ? raw("Hosted services", world.hosting) : ""}`;
}

// Source is immutable within a published module, but historical checkpoints may
// precede its publication. Read only the selected world's definitions.
export function sourceSignature(world = {}) {
  const ontology = world.ontology;
  return JSON.stringify(stable(ontology ? {
    foundation: { source: ontology.definition?.source ?? {}, agent_state: ontology.definition?.agent_state },
    modules: Object.fromEntries(entries(ontology.modules).map(([name, state]) => [name, { source: state.definition?.source ?? {}, agent_state: state.definition?.agent_state }])),
  } : null));
}

function renderOntolang(world) {
  const ontology = world.ontology;
  const modules = entries(ontology?.modules);
  const section = (key, name, state, module = false) => {
    const files = entries(state?.definition?.source).sort(([a], [b]) => {
      // Show a module's own source before its bundled foundation files.
      const shared = (file) => module && state.definition.source[file] === ontology.definition?.source?.[file];
      return Number(shared(a)) - Number(shared(b)) || a.localeCompare(b);
    });
    return `<section class="source-section" data-world-key="${esc(key)}"><h3>${esc(name)}</h3>${files.length ? files.map(([path, source], index) => `<details class="source-file" data-world-key="${esc(path)}"${index === 0 ? " open" : ""}><summary>${esc(path)}</summary><pre tabindex="0" aria-label="${esc(name)} · ${esc(path)}"><code>${esc(source)}</code></pre></details>`).join("") : '<p class="fact-note">No Ontolang source was recorded for this module.</p>'}${state?.definition?.agent_state ? raw("Agent state declaration", state.definition.agent_state) : ""}</section>`;
  };
  const body = ontology
    ? section("foundation", ontology.generation || modules.length ? "Foundation" : "Institution", ontology)
      + modules.map(([name, state]) => section(`module:${name}`, name, state, true)).join("")
    : '<p class="fact-note">This experiment does not use Ontolang. Select an Onto experiment to inspect its implementation.</p>';
  return `<article class="stage-card fact-card source-card" data-world-key="ontolang-source"><div class="card-meta"><span>World implementation</span><span>Read only</span></div><h2 class="fact-title serif">Ontolang</h2><p class="fact-note">${ontology ? "Source recorded at this checkpoint: the objects, rules, formulas and Stories that define the institution." : "No Ontolang implementation"}</p>${body}</article>`;
}

export function renderWorld(frame, previous, view = "facts", focus = "world") {
  const world = frame?.world ?? {};
  if (view === "ontolang") return renderOntolang(world);
  const before = previous?.world ?? previous;
  const compare = !!before;
  const unchanged = compare && worldSignature(world) === worldSignature(before);
  const domainView = view === "domains" || view === "structure" || view === "tree";
  let body = "";
  if (domainView) body = renderStructure(world, view, focus);
  else {
    if (world.ontology) {
      const modules = entries(world.ontology.modules);
      body += ontologySection(world.ontology.generation || modules.length ? "Company foundation" : "", world.ontology, before?.ontology, compare);
      body += modules.map(([name, state]) => ontologySection(name, state, before?.ontology?.modules?.[name], compare)).join("");
    }
    if (world.business) body += businessSection(world.business, before?.business, compare);
    if (world.flowers) body += flowersSection(world.flowers, before?.flowers, compare);
    body += resourcesSection(world, before, compare);
    if (!body) body = '<p class="fact-note">World facts will appear here once the experiment loads.</p>';
  }
  return `<article class="stage-card fact-card"><div class="card-meta"><h2>${domainView ? view === "tree" ? "Tree" : "Circle pack" : "Facts"}</h2><span>${esc(unchanged ? "Facts unchanged" : compare ? `Last change${frame.world_checkpoint ? ` · checkpoint ${frame.world_checkpoint}` : ""}` : "Initial snapshot")}</span></div>${body}</article>`;
}
