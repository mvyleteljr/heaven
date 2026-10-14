import test from "node:test";
import assert from "node:assert/strict";
import { renderAgent } from "../web/run-view.js";
import { createRunApp, templates, templateConfig } from "../web/run.js";

const world = { goal: "Carry out the selected experiment", finished: false, tools: { public_resources: {} } };
const seed = { world, system: "Act within the experiment", tools: [] };
const institution = { source: { "foundation.onto": "project Company\n" }, actor: "Clerk:Ada", epoch: "2026-01-01T00:00:00Z", public_facts: true };
const institutionSeed = { ...seed, institution, runtime_available: true };
const generation = { brief: "Build capabilities as they are discovered", max_modules: 8, repair_attempts: 2 };
const generatedSeed = {
  ...institutionSeed, generation,
  config: { actor_model: "selected/actor", world_model: "selected/generator", action_limit: 70, call_limit: 160, dollar_limit: 0.75, generator_settings: { max_tokens: 8192 } },
};
const config = { demo: true, actor_model: "openai/gpt-4.1-mini", world_model: "openai/gpt-4.1-mini", action_limit: 20, call_limit: 40, dollar_limit: 1 };
const frame = (seq, kind = "seed") => ({
  world: structuredClone(world),
  event: { seq, kind, summary: `Checkpoint ${seq}`, evidence: kind === "seed" ? {} : { action: { kind: "wait", inputs: { seconds: 1 } }, result: { waited: 1 } } },
});
function detail(id, seq = 1, overrides = {}) {
  return {
    run: { id, seq, name: id, experiment: "flowers", config: structuredClone(config), world: structuredClone(world), actions: 0, calls: 0, charged: 0, reserved: 0, status: "paused", busy: false, ...overrides },
    initial: { actor_instructions: "Saved instructions" }, observation: { tools: [] },
    events: [frame(seq).event], frames: [frame(seq)],
  };
}
function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}
const failure = (error, status = 500) => ({ responseError: error, status });
const flush = () => new Promise((resolve) => setImmediate(resolve));

// Exercise the real controller and render projections with a small DOM adapter.
// Every execution response is a local fixture; these tests make no network calls.
async function browser({ search = "", stored = "", keyAvailable = true, respond, clock = {} } = {}) {
  const elements = new Map();
  const element = (id) => {
    const selector = id.startsWith("#") ? id : `#${id}`;
    if (!elements.has(selector)) elements.set(selector, {
      value: "", innerHTML: "", textContent: "", hidden: false, disabled: false, dataset: {}, attributes: {},
      setAttribute(name, value) { this.attributes[name] = value; }, focus() { this.focused = true; },
    });
    return elements.get(selector);
  };
  let url = new URL(`http://localhost:3000/run${search}`);
  let preference = stored;
  let tick;
  const requests = [];
  const runs = new Map();
  const defaultResponse = (path, options) => {
    if (path === "runs") return [...runs.values()].map((value) => value.run);
    if (path === "health") return { key_available: keyAvailable };
    if (["institution-seed?template=generative", "institution-seed?template=story-review", "institution-seed?template=expansion-demo"].includes(path)) return generatedSeed;
    if (path.startsWith("institution-seed")) return institutionSeed;
    if (path.endsWith("-seed")) return seed;
    if (["flowers", "bellwether", "institutions"].includes(path) && options.method === "POST") {
      const body = JSON.parse(options.body);
      const saved = detail(`created-${runs.size + 1}`, 1, { name: body.name, config: body.config ?? body, experiment: path });
      runs.set(saved.run.id, saved);
      return saved.run;
    }
    const match = /^runs\/([^/?]+)(\/control)?/.exec(path);
    if (match) {
      const saved = runs.get(decodeURIComponent(match[1])) ?? detail(decodeURIComponent(match[1]));
      if (match[2]) return saved.run;
      return saved;
    }
    throw new Error(`Unexpected API path: ${path}`);
  };
  let responder = respond;
  const env = {
    ...clock,
    document: { querySelector: element, title: "" },
    localStorage: { getItem: () => preference, setItem: (_, value) => { preference = value; } },
    get location() { return url; },
    history: { replaceState: (_, __, next) => { url = new URL(next); } },
    setInterval: (callback) => { tick = callback; return 1; }, clearInterval() {},
    fetch: async (address, options = {}) => {
      const path = address.slice(5);
      requests.push({ path, options });
      const response = await (responder ? responder(path, options, defaultResponse) : defaultResponse(path, options));
      return response?.responseError
        ? { ok: false, status: response.status, json: async () => ({ error: response.responseError }) }
        : { ok: true, json: async () => structuredClone(response) };
    },
  };
  const app = createRunApp(env, {
    createCarousel: (root) => ({ update: (frames, index, data) => { root.innerHTML = renderAgent(frames[index], index, data); }, destroy() {} }),
    updateWorld: (root, html) => { root.innerHTML = html; },
  });
  await app.start();
  return { ...app, element, requests, respond: (fn) => { responder = fn; }, tick: () => tick(), url: () => url, stored: () => preference, posts: () => requests.filter(({ options }) => options.method === "POST") };
}

test("startup previews the free scripted experiment and honors explicit links over saved preference", async () => {
  const free = await browser({ keyAvailable: false });
  assert.equal(free.state.selection, "new:flowers-scripted");
  assert.equal(free.state.frames.length, 1);
  assert.equal(free.element("run").disabled, false);
  assert.equal(free.element("step").disabled, false);
  assert.equal(free.posts().length, 0);
  const linked = await browser({ stored: "new:bellwether", search: "?run=saved" });
  assert.equal(linked.state.selection, "run:saved");
  assert.equal(linked.state.data.run.id, "saved");
  assert.equal(linked.stored(), "run:saved");
  assert.equal(linked.url().searchParams.get("run"), "saved");
  assert.equal(linked.element("facts-tab").attributes["aria-selected"], "true");
});

for (const selected of templates) {
  test(`${selected.name}: Run and Step create and control the selected template exactly once`, async () => {
    for (const command of ["run", "step"]) {
      const b = await browser({ search: `?experiment=${selected.key}` });
      await b.element(command).onclick();
      const posts = b.posts();
      assert.equal(posts.length, 2);
      assert.equal(posts[0].path, selected.template ? "institutions" : selected.endpoint);
      assert.equal(posts[0].options.headers["X-Frontier-Client"], "research-ui");
      const body = JSON.parse(posts[0].options.body);
      const submittedConfig = selected.template ? body.config : body;
      assert.equal(submittedConfig.demo, !!selected.demo);
      assert.equal(submittedConfig.actor_system_prompt, seed.system);
      assert.equal(submittedConfig.actor_user_prompt, world.goal);
      if (selected.template) {
        assert.equal(body.template, selected.template);
        assert.deepEqual(body.institution, institution);
      } else {
        assert.equal(body.institution, undefined);
        assert.equal(body.config, undefined);
      }
      if (["generative", "story-review", "expansion-demo"].includes(selected.template)) {
        assert.deepEqual(body.generation, generation);
        for (const [key, value] of Object.entries(generatedSeed.config)) assert.deepEqual(body.config[key], value);
      }
      if (selected.key === "bellwether") assert.equal(submittedConfig.action_limit, 100);
      assert.equal(posts[1].path, "runs/created-1/control");
      assert.deepEqual(JSON.parse(posts[1].options.body), { command });
      assert.equal(b.state.selection, "run:created-1");
      assert.equal(b.url().searchParams.get("run"), "created-1");
      assert.equal(b.url().searchParams.has("experiment"), false);
    }
  });
}

test("template config preserves authored generator budgets without mutating the seed", () => {
  const source = structuredClone(generatedSeed);
  const configured = templateConfig(templates.find(({ template }) => template === "generative"), source);
  assert.equal(configured.call_limit, 160);
  assert.deepEqual(configured.generator_settings, { max_tokens: 8192 });
  assert.deepEqual(source, generatedSeed);
});

test("concurrent clicks neither create twice nor dispatch a second action", async () => {
  const b = await browser();
  const creation = deferred();
  b.respond((path, options, fallback) => path === "flowers" ? creation.promise : fallback(path, options));
  const first = b.control("step");
  assert.equal(b.element("run").disabled, true);
  assert.equal(b.element("step").disabled, true);
  assert.equal(b.element("experiment").disabled, true);
  await b.control("run");
  await b.select("new:bellwether");
  assert.equal(b.posts().length, 1);
  creation.resolve(detail("created").run);
  await first;
  assert.deepEqual(b.posts().map(({ path }) => path), ["flowers", "runs/created/control"]);
  assert.equal(b.state.selection, "run:created");

  const action = deferred();
  b.respond((path, options, fallback) => path.endsWith("/control") ? action.promise : fallback(path, options));
  const next = b.control("step");
  await b.control("step");
  assert.equal(b.posts().filter(({ path }) => path.endsWith("/control")).length, 2);
  action.resolve(detail("created").run);
  await next;
});

test("a paused but busy execution disables both action controls until polling completes", async () => {
  const b = await browser({ search: "?run=busy", respond: (path, options, fallback) => path.startsWith("runs/busy") ? detail("busy", 2, { busy: true }) : fallback(path, options) });
  assert.equal(b.element("step").disabled, true);
  assert.equal(b.element("run").disabled, true);
  assert.equal(b.element("step").textContent, "Stepping…");
  await b.control("step");
  await b.control("run");
  assert.equal(b.posts().length, 0);
  b.respond((path, options, fallback) => path.startsWith("runs/busy") ? detail("busy", 3) : fallback(path, options));
  await b.refresh();
  assert.equal(b.element("step").disabled, false);
});

test("running execution offers Pause while Step stays disabled", async () => {
  const b = await browser({ search: "?run=running", respond: (path, options, fallback) => path.startsWith("runs/running") ? detail("running", 2, { status: "running", busy: true }) : fallback(path, options) });
  assert.equal(b.element("run").textContent, "Pause");
  assert.equal(b.element("run").disabled, false);
  assert.equal(b.element("step").disabled, true);
  b.respond((path, options, fallback) => path.endsWith("/control") ? detail("running", 3, { busy: true }).run : path.startsWith("runs/running") ? detail("running", 3, { busy: true }) : fallback(path, options));
  await b.element("run").onclick();
  assert.deepEqual(JSON.parse(b.posts()[0].options.body), { command: "pause" });
  assert.equal(b.element("run").textContent, "Pausing…");
  assert.equal(b.element("step").disabled, true);
});

test("failed control retains the created execution so retry never creates another run", async () => {
  const b = await browser();
  b.respond((path, options, fallback) => path.endsWith("/control") ? failure("Action could not start") : fallback(path, options));
  await b.control("step");
  assert.equal(b.state.selection, "run:created-1");
  assert.equal(b.stored(), "run:created-1");
  assert.match(b.element("error").textContent, /Action could not start/);
  assert.equal(b.state.pending, false);
  b.respond((path, options, fallback) => fallback(path, options));
  await b.control("step");
  assert.equal(b.posts().filter(({ path }) => path === "flowers").length, 1);
  assert.equal(b.posts().filter(({ path }) => path.endsWith("/control")).length, 2);
});

test("failed creation leaves the draft selected and shows the server error", async () => {
  const b = await browser();
  b.respond((path, options, fallback) => path === "flowers" ? failure("Invalid configuration", 400) : fallback(path, options));
  await b.control("run");
  assert.equal(b.state.selection, "new:flowers-scripted");
  assert.equal(b.state.data, null);
  assert.equal(b.element("error").textContent, "Invalid configuration");
  assert.equal(b.element("error").hidden, false);
  assert.equal(b.posts().length, 1);
});

test("failed selection preserves the selected execution and its recorded evidence", async () => {
  const b = await browser({ search: "?run=original" });
  b.respond((path, options, fallback) => path.includes("institution-seed") ? failure("Missing runtime") : fallback(path, options));
  await b.select("new:institutions");
  assert.equal(b.state.selection, "run:original");
  assert.equal(b.state.data.initial.actor_instructions, "Saved instructions");
  assert.match(b.element("error").textContent, /Missing runtime/);
  assert.equal(b.element("experiment").disabled, false);
});

test("stale polling cannot overwrite a newer experiment selection", async () => {
  const b = await browser({ search: "?run=original" });
  const old = deferred();
  b.respond((path, options, fallback) => path.startsWith("runs/original") ? old.promise : fallback(path, options));
  const refresh = b.refresh();
  await flush();
  await b.select("new:bellwether");
  old.resolve(detail("original", 10));
  await refresh;
  assert.equal(b.state.selection, "new:bellwether");
  assert.equal(b.state.data, null);
  assert.equal(b.state.frames.length, 1);
  assert.equal(b.state.loading, false);
});

test("the most recent selection wins when an older seed request arrives later", async () => {
  const b = await browser();
  const old = deferred();
  b.respond((path, options, fallback) => path === "institution-seed?template=accounts" ? old.promise : fallback(path, options));
  const previous = b.select("new:institutions");
  await b.select("new:bellwether");
  old.resolve(institutionSeed);
  await previous;
  assert.equal(b.state.selection, "new:bellwether");
  assert.equal(b.state.seed.institution, undefined);
  assert.equal(b.element("experiment").value, "new:bellwether");
});

test("polling does not overlap and incremental evidence appends only once", async () => {
  const b = await browser({ search: "?run=saved" });
  const slow = deferred();
  const next = { ...detail("saved", 2), incremental: true, from_seq: 1, events: [frame(2, "observation").event], frames: [frame(2, "observation")] };
  delete next.initial;
  b.respond((path, options, fallback) => path.startsWith("runs/saved") ? slow.promise : fallback(path, options));
  const refresh = b.refresh();
  b.tick();
  b.tick();
  assert.equal(b.requests.filter(({ path }) => path.includes("after_seq=1")).length, 1);
  slow.resolve(next);
  await refresh;
  assert.equal(b.state.data.run.seq, 2);
  assert.equal(b.state.data.frames.length, 2);
  assert.equal(b.state.data.initial.actor_instructions, "Saved instructions");
  await b.refresh();
  assert.equal(b.state.data.frames.length, 2);
  assert.equal(b.state.data.events.length, 2);
  assert.equal(b.state.selected, 1);
});

test("replay and view changes never dispatch execution and retain the chosen historical frame", async () => {
  const two = detail("saved", 2);
  two.frames = [frame(1), frame(2, "observation")];
  const b = await browser({ search: "?run=saved", respond: (path, options, fallback) => path.startsWith("runs/saved") ? two : fallback(path, options) });
  b.element("previous").onclick();
  assert.equal(b.state.selected, 0);
  assert.equal(b.state.follow, false);
  b.element("domains-tab").onclick();
  assert.equal(b.state.view, "domains");
  await b.refresh();
  assert.equal(b.state.selected, 0);
  assert.equal(b.state.follow, false);
  assert.equal(b.element("latest").hidden, false);
  b.element("latest").onclick();
  assert.equal(b.state.selected, 1);
  assert.equal(b.state.follow, true);
  assert.equal(b.posts().length, 0);
});

test("missing key or Onto runtime prevents creating a live execution", async () => {
  const noKey = await browser({ search: "?experiment=flowers", keyAvailable: false });
  assert.equal(noKey.element("run").disabled, true);
  assert.match(noKey.element("error").textContent, /OPENROUTER_API_KEY/);
  await noKey.control("run");
  assert.equal(noKey.posts().length, 0);
  const noRuntime = await browser({ search: "?experiment=institutions", respond: (path, options, fallback) => path.startsWith("institution-seed") ? { ...institutionSeed, runtime_available: false, runtime_error: "Pinned runtime unavailable" } : fallback(path, options) });
  assert.equal(noRuntime.element("step").disabled, true);
  assert.equal(noRuntime.element("error").textContent, "Pinned runtime unavailable");
  await noRuntime.control("step");
  assert.equal(noRuntime.posts().length, 0);
});

test("another active run and exhausted limits prevent starting an additional execution", async () => {
  const active = await browser({ respond: (path, options, fallback) => path === "runs" ? [detail("other", 1, { status: "running", busy: true }).run] : fallback(path, options) });
  assert.equal(active.element("run").disabled, true);
  assert.match(active.element("error").textContent, /Another experiment is running/);
  await active.control("step");
  assert.equal(active.posts().length, 0);
  const exhausted = await browser({ search: "?run=done", respond: (path, options, fallback) => path.startsWith("runs/done") ? detail("done", 2, { actions: 20 }) : fallback(path, options) });
  assert.equal(exhausted.element("step").disabled, true);
  await exhausted.control("run");
  assert.equal(exhausted.posts().length, 0);
});

test("poll errors retain recorded evidence and a successful refresh clears the error", async () => {
  const b = await browser({ search: "?run=saved" });
  b.respond((path, options, fallback) => path === "runs" ? failure("Server unavailable") : fallback(path, options));
  await b.refresh();
  assert.equal(b.state.data.run.id, "saved");
  assert.match(b.element("error").textContent, /Could not refresh: Server unavailable/);
  b.respond((path, options, fallback) => fallback(path, options));
  await b.refresh();
  assert.equal(b.element("error").hidden, true);
});

test("failed detail read after Pause recovers actions completed since the previous poll", async () => {
  const b = await browser({ search: "?run=saved", respond: (path, options, fallback) => path.startsWith("runs/saved") ? detail("saved", 1, { status: "running" }) : fallback(path, options) });
  const latest = detail("saved", 4, { actions: 1 });
  latest.frames = [frame(1), frame(3, "observation"), frame(4, "control")];
  delete latest.frames[2].event.evidence.action;
  latest.events = latest.frames.map(({ event }) => event);
  let failDetail = true;
  b.respond((path, options, fallback) => {
    if (path === "runs/saved/control") return latest.run;
    if (path === "runs/saved" && failDetail) {
      failDetail = false;
      return failure("Temporary detail failure");
    }
    if (path.startsWith("runs/saved")) {
      if (path.includes("after_seq=4")) return { unchanged: true };
      if (path.includes("after_seq=1")) return { ...latest, incremental: true, from_seq: 1, frames: latest.frames.slice(1), events: latest.events.slice(1) };
      return latest;
    }
    return fallback(path, options);
  });
  await b.control("pause");
  assert.match(b.element("error").textContent, /Temporary detail failure/);
  await b.refresh();
  assert.equal(b.state.data.run.actions, 1);
  assert.deepEqual(b.state.frames.map(({ event }) => event.seq), [1, 3]);
});

test("Step back and Step forward inspect saved states before Step resumes execution at latest", async () => {
  const saved = detail("saved", 5, { actions: 2 });
  saved.frames = [frame(1), frame(3, "observation"), frame(5, "observation")];
  const b = await browser({ search: "?run=saved", respond: (path, options, fallback) => path.endsWith("/control") ? saved.run : path.startsWith("runs/saved") ? saved : fallback(path, options) });
  assert.equal(b.state.selected, 2);
  assert.equal(b.element("back").disabled, false);
  b.element("back").onclick();
  assert.equal(b.state.selected, 1);
  assert.equal(b.state.follow, false);
  assert.equal(b.element("step").textContent, "Step forward");
  b.element("back").onclick();
  assert.equal(b.state.selected, 0);
  assert.equal(b.element("back").disabled, true);
  await b.element("step").onclick();
  assert.equal(b.state.selected, 1);
  await b.element("step").onclick();
  assert.equal(b.state.selected, 2);
  assert.equal(b.state.follow, true);
  assert.equal(b.element("step").textContent, "Step");
  assert.equal(b.posts().length, 0);
  await b.element("step").onclick();
  assert.equal(b.posts().length, 1);
  assert.deepEqual(JSON.parse(b.posts()[0].options.body), { command: "step" });
});

test("historical forward stepping stays available for finished and actively running executions", async () => {
  for (const status of ["finished", "running"]) {
    const saved = detail("saved", 3, { actions: status === "finished" ? 20 : 1, status, busy: status === "running" });
    saved.frames = [frame(1), frame(3, "observation")];
    const b = await browser({ search: "?run=saved", respond: (path, options, fallback) => path.startsWith("runs/saved") ? saved : fallback(path, options) });
    assert.equal(b.element("step").disabled, true);
    b.element("back").onclick();
    assert.equal(b.element("step").textContent, "Step forward");
    assert.equal(b.element("step").disabled, false);
    await b.element("step").onclick();
    assert.equal(b.state.follow, true);
    assert.equal(b.element("step").disabled, true);
    assert.equal(b.posts().length, 0);
  }
});

test("history range inspects a checkpoint and polling preserves that event while appending", async () => {
  const saved = detail("saved", 5, { actions: 2 });
  saved.frames = [frame(1), frame(3, "observation"), frame(5, "observation")];
  const b = await browser({ search: "?run=saved", respond: (path, options, fallback) => path.startsWith("runs/saved") ? saved : fallback(path, options) });
  b.element("history").value = "1";
  b.element("history").oninput();
  assert.equal(b.state.frames[b.state.selected].event.seq, 3);
  assert.equal(b.state.follow, false);
  const next = { ...detail("saved", 7, { actions: 3 }), incremental: true, from_seq: 5, frames: [frame(7, "observation")], events: [frame(7, "observation").event] };
  b.respond((path, options, fallback) => path.startsWith("runs/saved") ? next : fallback(path, options));
  await b.refresh();
  assert.equal(b.state.frames.length, 4);
  assert.equal(b.state.frames[b.state.selected].event.seq, 3);
  assert.equal(Number(b.element("history").value), 1);
  assert.equal(Number(b.element("history").max), 3);
  assert.equal(b.state.follow, false);
  assert.equal(b.posts().length, 0);
});


test("persistent agent budget follows historical snapshots without execution", async () => {
  const saved = detail("harbor", 10, { experiment: "harbor_onto", status: "finished" });
  const initial = frame(1), later = frame(10, "outcome");
  initial.world.harbor_onto = later.world.harbor_onto = true;
  initial.world.ontology = { definition: { actor: "Business:Harbor Works" }, snapshot: { instances: [{ id: "Firm:harbor", type: "Firm", fields: { owner: "Business:Harbor Works", freeCash: 500000, balance: 500000, obligations: 0, reserved: 0, contingency: 0 } }] } };
  later.world.ontology = structuredClone(initial.world.ontology);
  later.world.ontology.snapshot.instances[0].fields.freeCash = 400000;
  saved.frames = [initial, later];
  const b = await browser({ search: "?run=harbor", respond: (path, options, fallback) => path.startsWith("runs/harbor") ? saved : fallback(path, options) });
  assert.match(b.element("agent-state").innerHTML, /\$4,000.00/);
  assert.equal(b.element("world-label").textContent, "Institution");
  b.navigate(0);
  assert.doesNotMatch(b.element("agent-state").innerHTML, /\$4,000.00/);
  assert.match(b.element("agent-state").innerHTML, /\$5,000.00/);
  b.navigate(1);
  assert.match(b.element("agent-state").innerHTML, /\$4,000.00/);
  assert.equal(b.posts().length, 0);
});

test("Ontolang source follows recorded publication history and tab keys never execute actions", async () => {
  const saved = detail("source", 2);
  const initial = frame(1);
  initial.world.ontology = { definition: { source: { "foundation.onto": "project Company\n" } }, modules: {} };
  const published = structuredClone(initial);
  published.event = { seq: 2, kind: "expansion", evidence: {} };
  published.world.ontology.modules.claims = { definition: { source: { "generated.onto": 'concept Claim\n  note "<script>alert(1)</script>"\n' } } };
  saved.frames = [initial, published];
  saved.run.world = published.world;
  const b = await browser({ search: "?run=source", respond: (path, options, fallback) => path.startsWith("runs/source") ? saved : fallback(path, options) });
  b.element("ontolang-tab").onclick();
  assert.equal(b.element("ontolang-tab").attributes["aria-selected"], "true");
  assert.equal(b.element("world-panel").attributes["aria-labelledby"], "ontolang-tab");
  assert.match(b.element("world-card").innerHTML, /generated\.onto/);
  assert.match(b.element("world-card").innerHTML, /&lt;script&gt;alert\(1\)&lt;\/script&gt;/);
  assert.doesNotMatch(b.element("world-card").innerHTML, /<script>/);
  b.navigate(0);
  assert.match(b.element("world-card").innerHTML, /foundation\.onto/);
  assert.doesNotMatch(b.element("world-card").innerHTML, /generated\.onto/);
  b.navigate(1);
  assert.match(b.element("world-card").innerHTML, /generated\.onto/);
  const key = (tab, value) => b.element(`${tab}-tab`).onkeydown({ key: value, preventDefault() {} });
  key("ontolang", "ArrowRight");
  assert.equal(b.state.view, "facts");
  key("facts", "ArrowLeft");
  assert.equal(b.state.view, "ontolang");
  assert.equal(b.element("ontolang-tab").focused, true);
  assert.equal(b.posts().length, 0);
});

test("batched generation replays blocked, world publication and retry without extra execution", async () => {
  const timers = new Map(); let timer = 0;
  const clock = { setTimeout: (fn) => { timers.set(++timer, fn); return timer; }, clearTimeout: (id) => timers.delete(id) };
  const b = await browser({ search: "?run=saved", clock });
  const action = { kind: "onto.expand", inputs: { module: "claims" } };
  const evidence = { action_id: 1, action, module: "claims", requires: ["https://institutions.local/modules/claims"], dependencies: [] };
  const blocked = { ...frame(2, "action_blocked"), event: { seq: 2, kind: "action_blocked", evidence } };
  const ready = { ...frame(4, "action_ready"), world: { ontology: { modules: { claims: { snapshot: { instances: [] } } } } }, event: { seq: 4, kind: "action_ready", evidence } };
  const done = { ...ready, event: { seq: 5, kind: "outcome", evidence: { action_id: 1, action, result: { module: "claims" } } } };
  const saved = detail("saved", 5); saved.frames = [frame(1), blocked, ready, done];
  b.respond((path, options, fallback) => path.startsWith("runs/saved") ? saved : fallback(path, options));
  await b.refresh();
  assert.equal(b.state.selected, 1);
  assert.match(b.element("agent-card").innerHTML, /Blocked · missing capability/);
  assert.equal(b.state.view, "domains");
  const advance = () => { const [id, fn] = timers.entries().next().value; timers.delete(id); fn(); };
  advance();
  assert.equal(b.state.selected, 2);
  assert.equal(b.state.worldFocus, "world", "publication keeps the Institution overview visible");
  assert.match(b.element("agent-card").innerHTML, /Capability ready · retrying/);
  advance();
  assert.equal(b.state.selected, 3);
  assert.equal(b.state.presenting, false);
  assert.match(b.element("agent-card").innerHTML, /Resources available/);
  b.navigate(1);
  assert.equal(b.state.worldFocus, "world");
  assert.equal(b.posts().length, 0);
  b.element("replay-expansion").onclick();
  assert.equal(b.state.selected, 1);
  assert.equal(b.state.follow, false);
  advance(); assert.equal(b.state.selected, 2);
  advance(); assert.equal(b.state.selected, 3);
  assert.equal(b.state.presenting, false);
  b.element("replay-expansion").onclick();
  b.navigate(0);
  assert.equal(timers.size, 0, "manual navigation cancels recorded playback");
  assert.equal(b.posts().length, 0);
  b.stop(); assert.equal(timers.size, 0);
});
