import { readFile } from "node:fs/promises";
import test from "node:test";
import assert from "node:assert/strict";

const source = await readFile(new URL("../web/app.js", import.meta.url), "utf8");
const model = "openai/gpt-4.1-mini";
const seed = { world: { goal: "new task", tools: { public_resources: {} } }, system: "new system", tools: [] };
const config = { demo: true, actor_model: model, world_model: model, action_limit: 20 };
const detail = (id, seq = 1) => ({
  run: { id, seq, name: id, experiment: "flowers", config, world: seed.world, actions: 0, status: "paused" },
  initial: { actor_instructions: "saved system" },
  observation: { tools: [] }, events: [],
  frames: [{ world: seed.world, event: { seq, kind: "seed", evidence: {} } }],
});
function deferred() {
  let resolve;
  const promise = new Promise((r) => { resolve = r; });
  return { promise, resolve };
}

// Minimal DOM adapter: exercise the actual application startup, event handlers,
// and asynchronous polling without adding a browser dependency to the harness.
async function browser(initial = "") {
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
    element(`#${id}`).value = model;
    element(`#${id}`).options = [new Option(model)];
  }
  element("#context-limit").value = 256000;
  element("#max-tokens").value = element("#generator-max-tokens").value = 1024;
  let stored = initial;
  let responder = (path) => path === "runs" ? (stored ? [{id:stored, flowers:true}] : [])
    : path.startsWith("runs/") ? detail(path.slice(5).split("?")[0])
    : path === "models" ? [] : path === "health" ? {key_available:false} : seed;
  const requests = [];
  let tick;
  const api = await new (Object.getPrototypeOf(async function() {}).constructor)(
    "document", "localStorage", "fetch", "Option", "setInterval",
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
  );
  return { ...api, element, requests, tick:()=>tick(), respond: (fn) => {responder=fn;}, stored:()=>stored };
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

test("institution viewer exposes generated state, changes and live spending", async () => {
  const b = await browser("first");
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
  await b.refresh();
  assert.match(b.element("#world").innerHTML, /claims/);
  assert.match(b.element("#world").innerHTML, /Policy:main/);
  assert.match(b.element("#world").innerHTML, /1 accepted/);
  assert.match(b.element("#world").innerHTML, /&lt;unsafe&gt;/);
  assert.match(b.element("#state").textContent, /ontology/);
  assert.match(b.element("#live-progress").textContent, /\$0.120.*\$5/);
});
