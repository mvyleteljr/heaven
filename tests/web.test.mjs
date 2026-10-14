import { readFile } from "node:fs/promises";
import test from "node:test";
import assert from "node:assert/strict";

const source = await readFile(new URL("../web/app.js", import.meta.url), "utf8");
const html = await readFile(new URL("../web/index.html", import.meta.url), "utf8");
const model = "openai/gpt-4.1-mini";
const seed = { world: { goal: "new task", tools: { public_resources: {} } }, system: "new system", tools: [] };
const config = { demo: true, actor_model: model, world_model: model, action_limit: 20 };
const detail = (id, seq = 1) => ({
  run: { id, seq, name: id, experiment: "flowers", config, world: seed.world, actions: 0, status: "paused" },
  initial: { actor_instructions: "saved system" },
  observation: { tools: [] }, events: [],
  frames: [{ world: seed.world, event: { seq, kind: "seed", evidence: {} } }],
});
const institution = {
  source: { "accounts.onto": 'project Accounts\n// <script>alert("source")</script>\n', "notes.onto": "// Keep this supporting file\n" },
  actor: "Clerk:Ada", epoch: "2026-01-01T00:00:00Z", public_facts: true,
};
const ontology = {
  definition: institution,
  runtime_sha256: "pinned-runtime",
  snapshot: { instances: [
    { id: "Clerk:Ada", type: "Clerk", fields: { name: "Ada" } },
    { id: "Account:Operating", type: "Account", fields: { name: "Operating <img src=x onerror=alert(1)>", balance: 10000, owner: "Clerk:Ada" } },
  ] },
  stories: [{ story: "Clerk creates Transfer <safe>", participants: ["Account:sender", "Account:recipient"] }],
  receipts: {},
};
const institutionSeed = {
  ...seed, name: "Onto · Accounts", description: "A public account model",
  world: { ...seed.world, ontology }, institution, runtime_available: true,
};
function deferred() {
  let resolve;
  const promise = new Promise((r) => { resolve = r; });
  return { promise, resolve };
}

// Minimal DOM adapter: exercise the actual application startup, event handlers,
// and asynchronous polling without adding a browser dependency to the harness.
async function browser(initial = "", search = "") {
  class Option {
    constructor(text, value = text) { this.text = text; this.value = value; }
  }
  const elements = new Map();
  const element = (selector) => {
    if (!elements.has(selector)) elements.set(selector, {
      value: "", options: [], hidden: false, disabled: false, innerHTML: "", textContent: "",
      replaceChildren(...options) { this.options = options; },
      add(option) { this.options.push(option); },
      checkValidity() { return true; }, focus() {},
    });
    return elements.get(selector);
  };
  for (const id of ["actor", "generator"]) {
    const initialModel = html.match(new RegExp(`<select id="${id}">\\s*<option value="([^"]+)"`))[1];
    element(`#${id}`).value = initialModel;
    element(`#${id}`).options = [new Option(initialModel)];
  }
  element("#context-limit").value = 256000;
  element("#max-tokens").value = element("#generator-max-tokens").value = 1024;
  let stored = initial;
  const linked = new URLSearchParams(search).get("run");
  let url = new URL("http://localhost:3000/" + search);
  let responder = (path) => path === "runs" ? [...new Set([stored, linked].filter(Boolean))].map((id) => ({id, flowers:true}))
    : path.startsWith("runs/") ? detail(path.slice(5).split("?")[0])
    : path === "models" ? [] : path === "health" ? {key_available:false} : seed;
  const requests = [];
  let tick;
  const api = await new (Object.getPrototypeOf(async function() {}).constructor)(
    "document", "localStorage", "fetch", "Option", "setInterval", "globalThis",
    source + "\nreturn {refresh, act, state:()=>({id,data}), select:async(value)=>{$('#runs').value=value; await $('#runs').onchange();}};",
  )(
    { querySelector: element, querySelectorAll: () => [] },
    { getItem: () => stored, setItem: (_, value) => { stored = value; }, removeItem: () => { stored = ""; } },
    async (url, options) => {
      const path = url.slice(5);
      requests.push({path,options});
      const value = await responder(path, options);
      return {ok:true, json:async()=>value};
    }, Option, (callback) => {tick=callback;},
    {get location() {return url;}, history: {replaceState: (_, __, next) => {url = new URL(next);}}},
  );
  return { ...api, element, requests, tick:()=>tick(), respond: (fn) => {responder=fn;}, stored:()=>stored, url:()=>url,
    experiment: async (value) => { element("#experiment").value = value; await element("#experiment").onchange(); },
  };
}

test("slow polling completes without starting overlapping requests", async () => {
  const b = await browser("first");
  const slow = deferred();
  let lists = 0;
  b.respond((path) => path === "runs" ? (++lists, slow.promise) : detail("first", 2));
  const refresh = b.refresh();
  b.tick();
  b.tick();
  assert.equal(lists, 1);
  slow.resolve([{id:"first",flowers:true}]);
  await refresh;
  assert.equal(b.state().data.run.seq, 2);
});

test("out-of-order polling never rewinds a saved run", async () => {
  const b = await browser("first");
  const slow = deferred();
  let count = 0;
  b.respond((path) => path === "runs" ? [{id:"first",flowers:true}]
    : ++count === 1 ? slow.promise : detail("first", 3));
  const older = b.refresh();
  await new Promise((r) => setImmediate(r));
  await b.refresh();
  slow.resolve(detail("first", 2));
  await older;
  assert.equal(b.state().data.run.seq, 3);
});

test("an old run-list response cannot clear a newly selected run", async () => {
  const b = await browser("first");
  const slow = deferred();
  let count = 0;
  b.respond((path) => path === "runs" ? (++count === 1 ? slow.promise : [{id:"second",flowers:true}]) : detail("second"));
  const older = b.refresh();
  await b.select("second");
  slow.resolve([{id:"first",flowers:true}]);
  await older;
  assert.equal(b.state().id, "second");
  assert.equal(b.state().data.run.id, "second");
});

test("missing saved runs reset local storage and restore editable defaults", async () => {
  const b = await browser("first");
  b.respond((path) => path === "runs" ? [] : seed);
  await b.refresh();
  assert.equal(b.state().id, null);
  assert.equal(b.stored(), "");
  assert.equal(b.element("#system").textContent, "new system");
  assert.equal(b.element("#new").disabled, false);
});

test("new runs reset saved context limits and pending actions disable controls", async () => {
  const b = await browser("first");
  assert.equal(b.element("#context-limit").value, 40000);
  assert.equal(b.element("#actor").value, model);
  assert.equal(b.element("#generator").value, model);
  const work = deferred();
  const pending = b.act(() => work.promise);
  assert.equal(b.element("#step").disabled, true);
  assert.equal(b.element("#pause").disabled, true);
  work.resolve();
  await pending;
  await b.select("");
  assert.equal(b.element("#context-limit").value, 256000);
  assert.equal(b.element("#context-limit").disabled, false);
  assert.equal(b.element("#memory-mode").value, "rolling");
  assert.equal(b.element("#action-format").value, "structured");
  for (const [role, prefix] of [["actor", ""], ["generator", "generator-"]]) {
    assert.equal(b.element(`#${role}`).value, "openai/gpt-6.1-sol");
    assert.equal(b.element(`#${prefix}temperature`).value, "");
    assert.equal(b.element(`#${prefix}max-tokens`).value, 4096);
  }
});

test("incremental polling appends trace once and retains initial evidence", async () => {
  const b = await browser("first");
  const next = detail("first", 2);
  next.incremental = true;
  next.from_seq = 1;
  delete next.initial;
  b.respond((path) => path === "runs" ? [{id:"first",flowers:true}] : next);
  await b.refresh();
  assert.equal(b.state().data.frames.length, 2);
  assert.equal(b.state().data.initial.actor_instructions, "saved system");
  assert.equal(b.state().data.run.seq, 2);
  await b.refresh();
  assert.equal(b.state().data.frames.length, 2);
});

test("Accounts loads editable Ontolang with escaped grouped facts and Stories", async () => {
  const b = await browser();
  b.respond((path) => path === "runs" ? [] : institutionSeed);
  await b.experiment("institutions");
  assert.ok(b.requests.some((r) => r.path === "institution-seed?template=accounts"));
  assert.equal(b.element("#institution-model").hidden, false);
  assert.equal(b.element("#institution-source").readOnly, false);
  assert.equal(b.element("#institution-source").value, institution.source["accounts.onto"]);
  assert.equal(b.element("#institution-actor").value, "Clerk:Ada");
  assert.match(b.element("#world").innerHTML, /Balance · \$100.00/);
  assert.match(b.element("#world").innerHTML, /Account · 1 records/);
  assert.match(b.element("#world").innerHTML, /&lt;img src=x onerror=alert\(1\)&gt;/);
  assert.match(b.element("#world").innerHTML, /Clerk creates Transfer &lt;safe&gt;/);
  assert.doesNotMatch(b.element("#world").innerHTML, /<img|<safe>/);
  assert.match(b.element("#settings-status").textContent, /\$0.25 · 30 actions/);
  assert.doesNotMatch(b.element("#world").innerHTML, /Company foundation|shared commitments are read-only/);
  assert.equal(b.element("#institution-generation").hidden, true);
  assert.deepEqual(JSON.parse(b.element("#state").textContent).ontology, ontology);
});

test("Accounts saves edited source and caller with all source files and nested config", async () => {
  const b = await browser();
  let saved = false;
  const next = detail("accounts-saved");
  next.run.experiment = "institutions";
  next.run.world = next.frames[0].world = institutionSeed.world;
  b.respond((path) => path === "runs" ? saved ? [{ id: next.run.id, experiment: "institutions" }] : []
    : path === "institutions" ? (saved = true, { id: next.run.id })
    : path.startsWith("runs/") ? next : institutionSeed);
  await b.experiment("institutions");
  b.element("#institution-source").value = "project Edited\n";
  b.element("#institution-source").oninput();
  b.element("#institution-actor").value = "Clerk:Edited";
  b.element("#institution-actor").oninput();
  b.element("#institution-file").value = "notes.onto";
  b.element("#institution-file").onchange();
  assert.equal(b.element("#institution-source").value, institution.source["notes.onto"]);
  b.element("#institution-file").value = "accounts.onto";
  b.element("#institution-file").onchange();
  assert.equal(b.element("#institution-source").value, "project Edited\n");
  b.element("#run-name").value = "My account model";
  await b.element("#new").onclick();
  const request = JSON.parse(b.requests.find((r) => r.path === "institutions").options.body);
  assert.equal(request.template, "accounts");
  assert.equal(request.name, "My account model");
  assert.equal(request.config.demo, false);
  assert.equal(request.config.action_limit, 30);
  assert.equal(request.config.dollar_limit, 0.25);
  assert.equal(request.config.actor_user_prompt, "new task");
  assert.deepEqual(request.institution.source, { ...institution.source, "accounts.onto": "project Edited\n" });
  assert.equal(request.institution.actor, "Clerk:Edited");
  assert.equal(request.institution.public_facts, true);
  assert.equal(b.state().id, next.run.id);
  assert.equal(b.element("#institution-source").readOnly, true);
  assert.equal(b.element("#institution-actor").disabled, true);
});

test("Harbor Onto uses a separate authored template and preserves original creation", async () => {
  const b = await browser();
  b.respond((path) => path === "runs" ? [] : path === "institutions" ? { id: "harbor" } : institutionSeed);
  await b.experiment("harbor_onto");
  assert.ok(b.requests.some((r) => r.path === "institution-seed?template=harbor"));
  assert.equal(b.element("#institution-source").readOnly, true);
  assert.equal(b.element("#institution-actor").disabled, true);
  b.element("#institution-source").value = "changed by script";
  b.element("#institution-source").oninput();
  await b.element("#new").onclick();
  const ontoRequest = JSON.parse(b.requests.find((r) => r.path === "institutions").options.body);
  assert.equal(ontoRequest.template, "harbor");
  assert.equal(ontoRequest.config.action_limit, 100);
  assert.equal(ontoRequest.config.dollar_limit, 1);
  assert.deepEqual(ontoRequest.institution, institution);
  await b.select("");
  await b.experiment("bellwether");
  b.respond((path) => path === "runs" ? [] : path === "bellwether" ? { id: "original" } : seed);
  await b.element("#new").onclick();
  const original = JSON.parse(b.requests.find((r) => r.path === "bellwether").options.body);
  assert.equal(original.action_limit, 100);
  assert.equal(original.config, undefined);
  assert.equal(original.institution, undefined);
  assert.equal(b.element("#institution-model").hidden, true);
});

test("saved institution checkpoints display receipts, source and complete state", async () => {
  const b = await browser();
  const first = { ...institutionSeed.world, ontology: structuredClone(ontology) };
  const second = structuredClone(first);
  second.ontology.snapshot.instances[1].fields.balance = 7500;
  second.ontology.receipts = {
    "transfer-1": { result: { committed: true, created: "Transfer:one" } },
    "<img rejected>": { result: { committed: false, violations: ["<script>unsafe</script>"] } },
  };
  const saved = detail("institution-saved", 2);
  saved.run.experiment = "harbor_onto";
  saved.run.world = second;
  saved.frames = [
    { world: first, event: { seq: 1, kind: "seed", evidence: {} } },
    { world: second, event: { seq: 2, kind: "action", evidence: {} } },
  ];
  b.respond((path) => path === "runs" ? [{ id: saved.run.id, experiment: "harbor_onto" }] : saved);
  await b.select(saved.run.id);
  assert.equal(b.element("#experiment").value, "harbor_onto");
  assert.equal(b.element("#institution-source").value, institution.source["accounts.onto"]);
  assert.match(b.element("#world").innerHTML, /transfer-1 · Committed/);
  assert.match(b.element("#world").innerHTML, /&lt;img rejected&gt; · Rejected/);
  assert.match(b.element("#world").innerHTML, /&lt;script&gt;unsafe&lt;\/script&gt;/);
  assert.doesNotMatch(b.element("#world").innerHTML, /<script>|<img/);
  assert.deepEqual(JSON.parse(b.element("#state").textContent), second);
  assert.match(b.element("#changes").innerHTML, /Institution facts changed/);
  assert.match(b.element("#changes").innerHTML, /Institution command receipts changed/);
  assert.doesNotMatch(b.element("#changes").innerHTML, /No world state changes/);
  b.element("#scrubber").value = 0;
  b.element("#scrubber").oninput();
  assert.match(b.element("#world").innerHTML, /Command receipts · 0/);
  assert.match(b.element("#world").innerHTML, /Balance · \$100.00/);
  assert.deepEqual(JSON.parse(b.element("#state").textContent), first);
});

test("unavailable runtime blocks only Onto creation and retains source inspection", async () => {
  const b = await browser();
  b.respond((path) => path === "runs" ? [] : path.startsWith("institution-seed")
    ? { ...institutionSeed, world: seed.world, runtime_available: false, runtime_error: "Bundle missing" } : seed);
  await b.experiment("institutions");
  assert.equal(b.element("#new").disabled, true);
  assert.match(b.element("#institution-status").textContent, /Bundle missing/);
  assert.equal(b.element("#institution-source").value, institution.source["accounts.onto"]);
  await b.element("#new").onclick();
  assert.equal(b.requests.some((r) => r.path === "institutions"), false);
  await b.experiment("bellwether");
  assert.equal(b.element("#new").disabled, false);
  assert.equal(b.element("#institution-model").hidden, true);
});

test("model validation errors keep the draft editable and do not create a run", async () => {
  const b = await browser();
  b.respond((path) => {
    if (path === "institutions") throw Error("Onto refused: invalid source");
    return path === "runs" ? [] : institutionSeed;
  });
  await b.experiment("institutions");
  b.element("#institution-source").value = "invalid draft";
  b.element("#institution-source").oninput();
  await b.element("#new").onclick();
  assert.equal(b.state().id, null);
  assert.equal(b.element("#institution-source").value, "invalid draft");
  assert.equal(b.element("#institution-source").readOnly, false);
  assert.equal(b.element("#new").disabled, false);
  assert.match(b.element("#error").textContent, /invalid source/);
});

test("a failed seed switch restores the experiment and preserves its edited draft", async () => {
  const b = await browser();
  b.respond((path) => {
    if (path === "institution-seed?template=harbor") throw Error("Seed unavailable");
    return path === "runs" ? [] : institutionSeed;
  });
  await b.experiment("institutions");
  b.element("#institution-source").value = "project MyEditedAccounts\n";
  b.element("#institution-source").oninput();
  b.element("#institution-actor").value = "Clerk:Edited";
  b.element("#institution-actor").oninput();
  await b.experiment("harbor_onto");
  assert.equal(b.element("#experiment").value, "institutions");
  assert.equal(b.element("#institution-source").value, "project MyEditedAccounts\n");
  assert.equal(b.element("#institution-source").readOnly, false);
  assert.match(b.element("#error").textContent, /Seed unavailable/);
  b.respond((path) => path === "institutions" ? { id: "edited" } : path === "runs" ? [] : seed);
  await b.element("#new").onclick();
  const request = JSON.parse(b.requests.find((r) => r.path === "institutions").options.body);
  assert.equal(request.template, "accounts");
  assert.equal(request.institution.actor, "Clerk:Edited");
  assert.equal(request.institution.source["accounts.onto"], "project MyEditedAccounts\n");
  assert.equal(request.institution.source["notes.onto"], institution.source["notes.onto"]);
});

test("institution viewer exposes generated state, changes and live spending", async () => {
  const b = await browser();
  const next = detail("first", 2);
  next.run.experiment = "institutions";
  next.run.calls = 3; next.run.charged = 0.02; next.run.reserved = 0.1;
  next.run.config = {...config, dollar_limit: 5};
  next.frames[0].world = {...seed.world, ontology: {snapshot:{instances:[]}, modules:{claims:{
    snapshot:{instances:[{id:"Policy:main",type:"Policy",limit:10}]},
    receipts:{one:{result:{committed:true}}}, stories:["Operator expands Policy"],
    definition:{source:{"generated.onto":"<unsafe>"}},
  }}}};
  next.run.world = next.frames[0].world;
  b.respond((path) => path === "runs" ? [{id:"first",experiment:"institutions"}] : next);
  await b.select("first");
  assert.match(b.element("#world").innerHTML, /claims/);
  assert.match(b.element("#world").innerHTML, /Policy:main/);
  assert.match(b.element("#world").innerHTML, /1 accepted/);
  assert.match(b.element("#world").innerHTML, /&lt;unsafe&gt;/);
  assert.match(b.element("#state").textContent, /ontology/);
  assert.match(b.element("#live-progress").textContent, /\$0.120.*\$5/);
  assert.equal(b.element("#experiment").value, "institutions_generated");
});

test("generated capabilities have a separate editable foundation and save bounded generation settings", async () => {
  const b = await browser();
  const generation = {brief: "Repair company", max_modules: 128, repair_attempts: 0};
  const generatedSeed = {
    ...institutionSeed,
    world: {...seed.world, ontology: {...ontology, generation, stories: [], modules: {}}},
    generation,
    config: {...config, demo: false, actor_settings: {max_tokens: 4096}, generator_settings: {max_tokens: 4096}},
  };
  b.respond((path) => path === "runs" ? [] : path === "institutions" ? {id: "generated"} : generatedSeed);
  await b.experiment("institutions_generated");
  assert.ok(b.requests.some((r) => r.path === "institution-seed?template=generative"));
  assert.equal(b.element("#institution-generation").hidden, false);
  assert.equal(b.element("#institution-source").readOnly, false);
  assert.equal(b.element("#generation-brief").value, generation.brief);
  assert.equal(b.element("#generator-max-tokens").value, 4096);
  assert.match(b.element("#world").innerHTML, /Company foundation/);
  assert.match(b.element("#settings-status").textContent, /\$0.50 · 60 actions · 6 expansions/);
  b.element("#generation-brief").value = "Revised repair company";
  b.element("#generation-brief").oninput();
  b.element("#generation-repairs").value = 1;
  b.element("#generation-repairs").oninput();
  b.element("#institution-source").value = "project NewFoundation\n";
  b.element("#institution-source").oninput();
  await b.element("#new").onclick();
  const request = JSON.parse(b.requests.find((r) => r.path === "institutions").options.body);
  assert.equal(request.template, "generative");
  assert.deepEqual(request.generation, {...generation, brief: "Revised repair company", repair_attempts: 1});
  assert.equal(request.institution.source["accounts.onto"], "project NewFoundation\n");
  assert.equal(request.config.action_limit, 60);
  assert.equal(request.config.expansion_limit, 6);
  assert.equal(request.config.call_limit, 90);
  assert.equal(request.config.dollar_limit, 0.5);
  assert.equal(request.config.demo, false);
});

test("story review creates a separate template and preserves it when reopening", async () => {
  const b = await browser();
  const generation = {brief: "Review procurement", max_modules: 6, repair_attempts: 0};
  const reviewSeed = {...institutionSeed, generation, config: {...config, demo: false, structured_actions: true},
    world: {...seed.world, ontology: {...ontology, generation, stories: []}}};
  b.respond((path) => path === "runs" ? [] : path === "institutions" ? {id: "review"} : reviewSeed);
  await b.experiment("onto_review");
  assert.ok(b.requests.some((r) => r.path === "institution-seed?template=story-review"));
  assert.match(b.element("#director-policy").textContent, /execute Stories/);
  b.element("#generation-brief").value = "Revised institutional review";
  b.element("#generation-brief").oninput();
  await b.element("#new").onclick();
  const request = JSON.parse(b.requests.find((r) => r.path === "institutions").options.body);
  assert.equal(request.template, "story-review");
  assert.equal(request.generation.brief, "Revised institutional review");
  const saved = detail("review");
  saved.run.experiment = "onto_review";
  saved.run.world = saved.frames[0].world = reviewSeed.world;
  b.respond((path) => path === "runs" ? [{id: "review", experiment: "onto_review"}] : saved);
  await b.select("review");
  assert.equal(b.element("#experiment").value, "onto_review");
  assert.match(b.element("#director-policy").textContent, /execute Stories/);
});

for (const [experiment, template] of [["harbor_review", "harbor-review"], ["harbor_review_v2", "harbor-review-v2"]]) {
test(`${experiment} saves direct execution separately with fixed source and no generation`, async () => {
  const b = await browser();
  const reviewSeed = {...institutionSeed, name: "Harbor Works · Story review", config: {...config, demo: false, structured_actions: true, expansion_limit: 0, memory: null}};
  b.respond((path) => path === "runs" ? [] : path === "institutions" ? {id: "harbor-review"} : reviewSeed);
  await b.experiment(experiment);
  assert.ok(b.requests.some((r) => r.path === `institution-seed?template=${template}`));
  assert.equal(b.element("#institution-source").readOnly, true);
  assert.equal(b.element("#institution-actor").disabled, true);
  assert.equal(b.element("#institution-generation").hidden, true);
  assert.match(b.element("#director-policy").textContent, /direct Harbor Story execution/);
  assert.match(b.element("#settings-status").textContent, /\$1.00 · 60 actions/);
  b.element("#institution-source").value = "tampered source";
  b.element("#institution-source").oninput();
  await b.element("#new").onclick();
  const request = JSON.parse(b.requests.find((r) => r.path === "institutions").options.body);
  assert.equal(request.template, template);
  assert.equal(request.generation, undefined);
  assert.deepEqual(request.institution, institution);
  assert.equal(request.config.action_limit, 60);
  assert.equal(request.config.expansion_limit, 0);
  assert.equal(request.config.call_limit, 90);
  assert.equal(request.config.dollar_limit, 1);
  const saved = detail("harbor-review");
  saved.run.experiment = experiment;
  saved.run.world = saved.frames[0].world = reviewSeed.world;
  b.respond((path) => path === "runs" ? [{id: "harbor-review", experiment}] : saved);
  await b.select("harbor-review");
  assert.equal(b.element("#experiment").value, experiment);
  assert.match(b.element("#director-policy").textContent, /direct Harbor Story execution/);
});

}

test("saved generated runs expose fixed generation settings and restore the correct new template", async () => {
  const b = await browser();
  const generation = {brief: "Saved company", max_modules: 12, repair_attempts: 2};
  const saved = detail("generated-saved");
  saved.run.experiment = "institutions";
  saved.run.world = saved.frames[0].world = {...seed.world, ontology: {...ontology, generation, stories: []}};
  b.respond((path) => path === "runs" ? [{id: saved.run.id, experiment: "institutions"}] : saved);
  await b.select(saved.run.id);
  assert.equal(b.element("#experiment").value, "institutions_generated");
  assert.equal(b.element("#generation-brief").value, "Saved company");
  assert.equal(b.element("#generation-brief").disabled, true);
  assert.equal(b.element("#generation-repairs").value, 2);
  assert.match(b.element("#director-policy").textContent, /generate executable capabilities/);
  b.respond((path) => path === "runs" ? [] : {...institutionSeed, generation});
  await b.select("");
  assert.ok(b.requests.some((r) => r.path === "institution-seed?template=generative"));
  assert.equal(b.element("#generation-brief").disabled, false);
});

test("deep links take precedence and follow run selection without stale reload targets", async () => {
  const b = await browser("stored", "?run=linked");
  assert.equal(b.state().id, "linked");
  b.respond((path) => path === "runs" ? [{id: "second", flowers: true}] : path.startsWith("runs/") ? detail("second") : seed);
  await b.select("second");
  assert.equal(b.url().searchParams.get("run"), "second");
  await b.select("");
  assert.equal(b.url().searchParams.has("run"), false);
});

test("choosing another experiment from a saved run opens a draft without mutating the run", async () => {
  const b = await browser("saved", "?run=saved");
  const original = structuredClone(b.state().data);
  assert.equal(b.element("#experiment").disabled, false);
  assert.equal(b.element("#configure-new").disabled, false);
  const previousRequests = b.requests.length;
  b.respond((path) => path === "runs" ? [{id: "saved", flowers: true}]
    : path.startsWith("runs/") ? original : institutionSeed);
  await b.experiment("institutions");
  assert.equal(b.state().id, null);
  assert.equal(b.state().data, null);
  assert.equal(b.stored(), "");
  assert.equal(b.url().searchParams.has("run"), false);
  assert.equal(b.element("#experiment").value, "institutions");
  assert.equal(b.element("#step").disabled, true);
  assert.equal(b.element("#run").disabled, true);
  assert.equal(b.element("#new").disabled, false);
  assert.equal(b.element("#institution-source").readOnly, false);
  assert.ok(b.requests.slice(previousRequests).every(r => !r.options?.method));
  await b.select("saved");
  assert.deepEqual(b.state().data, original);
});

test("failed saved-to-draft navigation preserves saved settings and link", async () => {
  const b = await browser();
  const saved = detail("accounts");
  saved.run.experiment = "institutions";
  saved.run.world = saved.frames[0].world = {...seed.world, ontology};
  b.respond((path) => path === "runs" ? [{id: "accounts", experiment: "institutions"}] : saved);
  await b.select("accounts");
  b.respond((path) => {
    if (path.startsWith("institution-seed")) throw Error("Seed temporarily unavailable");
    return path === "runs" ? [{id: "accounts", experiment: "institutions"}] : saved;
  });
  await b.experiment("harbor_onto");
  assert.equal(b.state().id, "accounts");
  assert.deepEqual(b.state().data, saved);
  assert.equal(b.element("#experiment").value, "institutions");
  assert.equal(b.element("#runs").value, "accounts");
  assert.equal(b.element("#institution-source").readOnly, true);
  assert.equal(b.stored(), "accounts");
  assert.equal(b.url().searchParams.get("run"), "accounts");
  assert.match(b.element("#error").textContent, /Seed temporarily unavailable/);
});

test("new draft remains visible when refreshing the saved-run list fails", async () => {
  const b = await browser("saved");
  b.respond((path) => {
    if (path === "runs") throw Error("Saved-run list temporarily unavailable");
    return institutionSeed;
  });
  await b.experiment("institutions");
  assert.equal(b.state().id, null);
  assert.equal(b.element("h1").textContent, institutionSeed.name);
  assert.equal(b.element("#status").textContent, "Not started");
  assert.equal(b.element("#new").disabled, false);
  assert.equal(b.element("#step").disabled, true);
  assert.match(b.element("#error").textContent, /Saved-run list temporarily unavailable/);
});

test("late saved polling cannot restore a run after New run is chosen", async () => {
  const b = await browser("saved");
  const late = deferred();
  let lists = 0;
  b.respond((path) => path === "runs" ? (++lists === 1 ? late.promise : [{id: "saved", flowers: true}])
    : path.startsWith("runs/") ? detail("saved", 10) : seed);
  const pendingPoll = b.refresh();
  await b.element("#configure-new").onclick();
  late.resolve([{id: "saved", flowers: true}]);
  await pendingPoll;
  assert.equal(b.state().id, null);
  assert.equal(b.state().data, null);
  assert.equal(b.element("#experiment").value, "flowers");
  assert.equal(b.element("#run-name").value, "");
  assert.equal(b.element("#new").disabled, false);
});


test("agent state declaration edits survive polling and invalid JSON prevents saving", async () => {
  const b = await browser();
  const shape = { fields: [{ key: "name", label: "Name", kind: "string", format: "plain", source: { instance: "Clerk:Ada", field: "name" } }] };
  const custom = structuredClone(institutionSeed);
  custom.institution.agent_state = shape;
  let saved = false;
  const next = detail("declared-agent");
  next.run.experiment = "institutions";
  next.run.world = next.frames[0].world = custom.world;
  b.respond((path) => path === "runs" ? saved ? [{ id: next.run.id, experiment: "institutions" }] : []
    : path === "institutions" ? (saved = true, { id: next.run.id })
    : path.startsWith("runs/") ? next : custom);
  await b.experiment("institutions");
  assert.deepEqual(JSON.parse(b.element("#institution-agent-state").value), shape);
  b.element("#institution-agent-state").value = "{invalid";
  b.element("#institution-agent-state").oninput();
  await b.refresh();
  assert.equal(b.element("#institution-agent-state").value, "{invalid");
  await b.element("#new").onclick();
  assert.equal(b.requests.filter((r) => r.path === "institutions").length, 0);
  shape.fields[0].label = "Agent name";
  b.element("#institution-agent-state").value = JSON.stringify(shape);
  b.element("#institution-agent-state").oninput();
  await b.element("#new").onclick();
  const submitted = JSON.parse(b.requests.find((r) => r.path === "institutions").options.body);
  assert.deepEqual(submitted.institution.agent_state, shape);
  assert.equal(b.element("#institution-agent-state").readOnly, true);
});
