import test from "node:test";
import assert from "node:assert/strict";
import { experimentName, traceFrames, renderAgent, renderAgentState, agentFunds, renderWorld, worldSignature } from "../web/run-view.js";

const institution = (balance = 10000) => ({
  snapshot: { instances: [{ id: "Account:Operating", type: "Account", fields: { name: "Operating", balance } }] },
  stories: [{ story: "Clerk transfers money", participants: ["Account:sender", "Account:recipient"] }],
  receipts: {},
});
const world = () => ({
  ontology: institution(),
  tools: { elapsed_seconds: 0, public_resources: {}, workspace: { files: {}, directories: [] }, messages: [] },
  documents: {}, transcript: [], finished: false,
});
const frame = (kind, evidence = {}, state = world(), seq = 1) => ({ event: { kind, evidence, seq }, world: state });
const request = { kind: "http.request", inputs: { method: "POST", url: "https://institutions.local/commands", body: JSON.stringify({ command_id: "transfer-1", story: "Clerk transfers money" }) } };

test("HTTP transport success is not treated as a committed institutional command", () => {
  const rejected = renderAgent(frame("outcome", { action: request, result: { status: 200, body: JSON.stringify({ committed: false, error: "Balance cannot be negative" }) } }), 3);
  assert.match(rejected, /Rejected/);
  assert.match(rejected, /Balance cannot be negative/);
  assert.doesNotMatch(rejected, />Committed</);
  assert.match(rejected, /Clerk transfers money/);
  assert.match(rejected, /Checkpoint 4/);

  const response = renderAgent(frame("outcome", { action: request, result: { status: 200, body: JSON.stringify({ status: "ok" }) } }));
  assert.match(response, /HTTP 200/);
  assert.doesNotMatch(response, />Committed</);
  const committed = renderAgent(frame("outcome", { action: request, result: { status: 200, body: '{"committed":true}' } }));
  assert.match(committed, />Committed</);
});

test("world transitions ignore reads, refusals and trace bookkeeping but retain changed facts", () => {
  const before = world();
  const after = structuredClone(before);
  after.tools.elapsed_seconds = 120;
  after.transcript.push({ result: { status: 200 } });
  after.finished = true;
  after.ontology.receipts.refused = { result: { committed: false } };
  after.ontology.snapshot.lastCommand = "refused";
  assert.equal(worldSignature(after), worldSignature(before));
  assert.match(renderWorld(frame("denied", {}, after), frame("seed", {}, before)), /Facts unchanged/);
  after.ontology.snapshot.instances[0].fields.balance = 7500;
  assert.notEqual(worldSignature(after), worldSignature(before));
  const html = renderWorld(frame("outcome", {}, after), frame("seed", {}, before));
  assert.match(html, /<del>10000<\/del>/);
  assert.match(html, /<strong>7500<\/strong>/);
  assert.doesNotMatch(html, /Facts unchanged/);
});

test("module publication and operations use real isolated module state", () => {
  const before = world();
  const after = structuredClone(before);
  after.ontology.generation = { brief: "Build capabilities" };
  after.ontology.modules = { inventory: institution(3000), scheduling: institution(2000) };
  assert.notEqual(worldSignature(after), worldSignature(before));
  assert.equal(experimentName({ experiment: "institutions", world: after }), "Onto · Generated company");
  const html = renderWorld(frame("outcome", {}, after), frame("seed", {}, before));
  assert.match(html, /Company foundation/);
  assert.match(html, />inventory</);
  assert.match(html, />scheduling</);
  assert.match(html, /3000/);
  assert.match(html, /2000/);
  const domains = renderWorld(frame("outcome", {}, after), null, "domains");
  assert.match(domains, /class="domain-list world-tree"/);
  assert.match(domains, /class="domain-map"/);
  assert.match(domains, /Clerk transfers money/);
  assert.match(domains, />Account</);
});

test("rendering escapes model, resource, file and institution content everywhere", () => {
  const attack = '<img src=x onerror="alert(1)">';
  const state = world();
  state.ontology.snapshot.instances[0].fields.name = attack;
  state.ontology.stories[0].story = attack;
  state.ontology.modules = { [attack]: institution() };
  state.tools.public_resources[attack] = { title: attack, body: attack };
  state.tools.workspace.files[attack] = Array.from(new TextEncoder().encode(attack));
  state.tools.messages.push({ id: "mail-1", subject: attack, body: attack });
  const action = { kind: "finish", inputs: { message: attack } };
  const current = frame("outcome", { action, result: { answer: attack } }, state);
  for (const html of [renderAgent(current), renderWorld(current), renderWorld(current, null, "domains")]) {
    assert.doesNotMatch(html, /<img|<script/);
    assert.match(html, /&lt;img/);
  }
});

test("checkpoint projection preserves expansion and lifecycle states as later actions arrive", () => {
  const seed = frame("seed", {}, world(), 1);
  const expanded = frame("expansion", { resources: ["inventory"] }, world(), 2);
  const action = frame("outcome", { action: request, result: { status: 200 } }, world(), 3);
  const error = frame("error", { error: "Provider unavailable" }, world(), 4);
  const recovered = frame("recovery", { note: "Run paused after restart" }, world(), 5);
  assert.deepEqual(traceFrames({ frames: [seed, expanded, action, error, recovered] }), [seed, expanded, action, error, recovered]);
  assert.deepEqual(traceFrames({ frames: [seed, expanded] }), [seed, expanded]);
  const legacyExpansion = frame("expansion", { action: { kind: "explore", target: "archive" }, result: {} });
  assert.deepEqual(traceFrames({ frames: [seed, legacyExpansion, action] }), [seed, legacyExpansion, action]);
  assert.match(renderAgent(error, 2), /Provider unavailable/);
});

test("business, flower and generic tool worlds remain inspectable", () => {
  const business = { business: { clock: 4, organizations: { harbor: { name: "Harbor", cash: 12000 } }, contracts: [{ id: "contract-1", paid: false }] } };
  assert.match(renderWorld(frame("seed", {}, business)), /Harbor/);
  assert.match(renderWorld(frame("seed", {}, business)), /contract-1/);
  assert.equal(worldSignature(business), worldSignature({ business: { ...business.business, clock: 900, journal: ["read"], checkins: 2 } }));
  const flowers = { flowers: { florist: "Willow & Stem", time: "Tuesday", balance_cents: 12000, order: null } };
  assert.match(renderWorld(frame("seed", {}, flowers)), /Willow &amp; Stem/);
  assert.equal(worldSignature(flowers), worldSignature({ flowers: { ...flowers.flowers, time: "Friday" } }));
  const tools = { tools: { public_resources: { "https://records.local/items": { title: "Records", body: "persisted fact" } }, workspace: { files: { "notes.txt": [104, 105] } }, messages: [] } };
  assert.match(renderWorld(frame("seed", {}, tools)), /persisted fact/);
  assert.match(renderWorld(frame("seed", {}, tools)), /notes.txt/);
  assert.match(renderWorld(frame("seed", {}, tools)), /<pre>hi<\/pre>/);
  assert.doesNotMatch(renderWorld(frame("seed", {}, tools), null, "domains"), /records.local|notes.txt/);
  assert.match(renderWorld(frame("seed", {}, tools), null, "domains"), /Ontology|No domain concepts/);
});

test("an unsaved draft renders without saved run data", () => {
  const draft = frame("seed", {}, { ...world(), goal: "Explore the company" });
  const agent = renderAgent(draft, 0, null);
  assert.match(agent, /Checkpoint 1/);
  assert.match(agent, /Explore the company/);
  assert.match(renderWorld(draft, null), /Initial snapshot/);
  assert.doesNotMatch(renderWorld(draft, null), /previous checkpoint/);
});

test("Flowers projects confirmed order and money into readable facts with catalog details", () => {
  const before = { flowers: { florist: "Willow & Stem", balance_cents: 12000, recipient: "14 Willow Lane", order: null } };
  const after = { flowers: { ...before.flowers, balance_cents: 2700,
    catalog: [{ id: "lilies", name: "White lilies", description: "Sympathy arrangement", price_cents: 7500, sympathy_appropriate: true }],
    delivery: { options: [{ id: "morning", label: "Friday morning", fee_cents: 1800, arrival_hour: 10 }], policies: "Card included." },
    order: { confirmation: "WS-001", status: "confirmed", arrangement: "White lilies", total_cents: 9300, delivery_fee_cents: 1800, address: "14 Willow Lane", day: "Friday", arrival_hour: 10, message: "With deepest sympathy, from Alex." },
  } };
  const html = renderWorld(frame("outcome", {}, after), frame("seed", {}, before));
  const displayed = html.split('<summary>Complete florist state</summary>')[0];
  assert.match(displayed, /Available Budget/);
  assert.match(displayed, /<del>\$120\.00<\/del>/);
  assert.match(displayed, /<strong>\$27\.00<\/strong>/);
  assert.match(displayed, /Confirmed Total/);
  assert.match(displayed, /\$93\.00/);
  assert.match(displayed, /Friday, 10 a\.m\./);
  assert.match(displayed, /White lilies/);
  assert.match(displayed, /14 Willow Lane/);
  assert.match(displayed, /With deepest sympathy, from Alex\./);
  assert.match(displayed, /Catalog · 1 arrangement/);
  assert.match(displayed, /\$75\.00/);
  assert.match(displayed, /Delivery · 1 option/);
  assert.match(displayed, /\$18\.00/);
  assert.doesNotMatch(displayed, /&quot;total_cents&quot;|&quot;price_cents&quot;/);
  assert.match(html, /&quot;total_cents&quot;: 9300/);
});

test("Onto review actions show exact Stories and module scope", () => {
  assert.equal(experimentName("onto_review"), "Onto · Story review");
  const action = { kind: "onto.execute", inputs: { module: "inventory", story: "Operator authorizes Restock", command_id: "restock-1" } };
  const html = renderAgent(frame("outcome", { action, result: { committed: true, module: "inventory", command_id: "restock-1" } }), 1, { run: { experiment: "onto_review" } });
  assert.match(html, /class="story-title serif">Operator authorizes Restock/);
  assert.match(html, /class="story-domain">inventory/);
  assert.match(html, />Committed</);
  assert.match(renderAgent(frame("outcome", { action: { kind: "onto.inspect", inputs: { view: "facts" } }, result: {} })), /Inspect facts · foundation/);
  assert.match(renderAgent(frame("outcome", { action: { kind: "onto.expand", inputs: { module: "scheduling" } }, result: {} })), /Request capability · scheduling/);
});


test("agent budget uses only owned financial state at the selected checkpoint", () => {
  const firm = (owner, available) => ({ id: owner, type: "Firm", fields: { owner, freeCash: available, balance: 500000, obligations: 50000, reserved: 25000, contingency: 0 } });
  const initial = frame("seed", {}, { harbor_onto: true, ontology: { definition: { actor: "Business:Harbor Works" }, snapshot: { instances: [firm("Business:Harbor Works", 425000), firm("Business:Other", 900000)] } } });
  const later = structuredClone(initial);
  later.world.ontology.snapshot.instances[0].fields.freeCash = 300000;
  assert.equal(agentFunds(initial.world).available, 425000);
  assert.equal(agentFunds(later.world).available, 300000);
  assert.match(renderAgentState(initial), /\$4,250.00/);
  assert.match(renderAgentState(later), /\$3,000.00/);
  assert.doesNotMatch(renderAgentState(initial), /\$9,000.00|Mandate|Remembered facts|Latest observation/);
  assert.equal(agentFunds(initial.world).reserved, 25000);
});

test("missing budgets are not inferred from institutional accounts, and zero remains a real value", () => {
  const accounts = frame("seed");
  accounts.world.ontology.definition = { actor: "Clerk:<Ada>" };
  assert.equal(agentFunds(accounts.world), null);
  assert.doesNotMatch(renderAgentState(accounts), /Not modeled|Available to spend/);
  assert.match(renderAgentState(accounts), /Clerk:&lt;Ada&gt;/);
  assert.doesNotMatch(renderAgentState(accounts), /<Ada>|\$100.00/);
  const flowers = frame("outcome", {}, { flowers: { balance_cents: 0 } });
  assert.match(renderAgentState(flowers), /\$0.00/);
  assert.match(renderAgentState(flowers), /Customer spending allowance/);
});

test("Ontolang shows complete source files with explicit missing-source states", () => {
  assert.match(renderWorld({ world: {} }, null, "ontolang"), /does not use Ontolang/);
  assert.match(renderWorld(frame("seed"), null, "ontolang"), /No Ontolang source was recorded/);
  const state = world();
  const source = "project Company\n" + "// preserve this source line\n".repeat(80);
  state.ontology.definition = { source: { "foundation.onto": source } };
  state.ontology.modules = { claims: { definition: { source: { "foundation.onto": source, "generated.onto": "concept Claim\n" } } } };
  const html = renderWorld({ world: state }, null, "ontolang");
  assert.ok(html.includes(source));
  const module = html.slice(html.indexOf('data-world-key="module:claims"'));
  assert.ok(module.indexOf("generated.onto") < module.indexOf("foundation.onto"));
});

test("construction phases retain action identity and never mistake a refusal for success", () => {
  const action = { kind: "onto.expand", inputs: { module: "claims" } };
  const evidence = { action, action_id: 1, module: "claims", requires: ["https://institutions.local/modules/claims"], dependencies: ["https://institutions.local/"] };
  const frames = traceFrames({ frames: [
    frame("action_blocked", evidence, world(), 2),
    frame("action_ready", evidence, { ontology: { modules: { claims: institution() } } }, 4),
    frame("outcome", { action, action_id: 1, result: { committed: false, error: "Not authorized" } }, world(), 5),
  ] });
  assert.deepEqual(frames.map((f) => f.workflow.id), ["action:1", "action:1", "action:1"]);
  assert.deepEqual(frames.map((f) => f.workflow.phase), ["blocked", "ready", "failed"]);
  assert.match(renderAgent(frames[0]), /Blocked · missing capability/);
  assert.match(renderAgent(frames[1]), /Capability ready · retrying/);
  assert.match(renderAgent(frames[2]), /Not authorized/);
  assert.doesNotMatch(renderAgent(frames[2]), /Resources available/);
});


test("agent panel renders project-specific actor fields, not unrelated world records", () => {
  const initial = { world: { ontology: { definition: { actor: "Operator:Ada" }, snapshot: { instances: [
    { id: "Operator:Ada", fields: { name: "Ada", activeTickets: 0, role: "Intake clerk" } },
    { id: "Company:Core", fields: { ceiling: 100, secret: "world-only" } },
    { id: "Operator:Other", fields: { role: "Other agent" } },
  ] } } } };
  const later = structuredClone(initial);
  later.world.ontology.snapshot.instances[0].fields.activeTickets = 1;
  const before = renderAgentState(initial), after = renderAgentState(later);
  assert.match(before, /Active Tickets/);
  assert.match(before, /Intake clerk/);
  assert.match(before, />0<\/dd>/);
  assert.match(after, />1<\/dd>/);
  assert.doesNotMatch(after, /Available to spend|Not modeled|Ceiling|world-only|Other agent/);
  assert.equal(initial.world.ontology.snapshot.instances[0].fields.activeTickets, 0);
});

test("declared agent shape grows with a domain and renders only that checkpoint's values", () => {
  const identity = { fields: [{ key: "name", label: "Name", kind: "string", format: "plain", source: { instance: "Operator:Ada", field: "name" } }] };
  const seed = { world: { ontology: { definition: { actor: "Operator:Ada", agent_state: identity }, agent_state: { shape: identity, values: { name: "Ada" } }, modules: {} } } };
  const ready = structuredClone(seed);
  const shape = { fields: [{ key: "last_observation", label: "Last observation", kind: "string", format: "plain", source: { instance: "AgentMemory:main", field: "lastObservation" } }] };
  ready.world.ontology.modules["repair-intake"] = { definition: { agent_state: shape }, agent_state: { shape, values: { last_observation: "unobserved" } } };
  const done = structuredClone(ready);
  done.world.ontology.modules["repair-intake"].agent_state.values.last_observation = "Receipt intake-1 observed";
  assert.doesNotMatch(renderAgentState(seed), /Last observation|Available to spend/);
  assert.match(renderAgentState(ready), /Last observation/);
  assert.match(renderAgentState(ready), />unobserved<\/dd>/);
  assert.match(renderAgentState(done), />Receipt intake-1 observed<\/dd>/);
  assert.doesNotMatch(renderAgentState(ready), /Receipt intake-1 observed/);
  assert.match(renderWorld(done, null, "ontolang"), /Agent state declaration/);
  assert.match(renderWorld(done, null, "ontolang"), /last_observation/);
});
