import { createAgentCarousel } from "./run-carousel.js";
import { reconcileWorld } from "./run-world.js";
import { esc, experimentName, traceFrames, renderWorld, renderAgentState, worldSignature } from "./run-view.js";

const runLabel = (run) => ["institutions", "harbor_onto", "institutions_generated", "onto_review"].includes(run.experiment) && !run.name?.startsWith("Onto")
  ? `${experimentName(run)}${run.name ? ` · ${run.name}` : ""}` : run.name || experimentName(run);

export const templates = [
  { key: "flowers-scripted", name: "Willow & Stem · scripted", endpoint: "flowers", seed: "flower-seed", demo: true },
  { key: "flowers", name: "Willow & Stem · live", endpoint: "flowers", seed: "flower-seed" },
  { key: "bellwether", name: "Harbor Works · original Rust", endpoint: "bellwether", seed: "bellwether-seed" },
  { key: "institutions", name: "Onto · Accounts", template: "accounts" },
  { key: "harbor_onto", name: "Onto · Harbor Works", template: "harbor" },
  { key: "institutions_generated", name: "Onto · Generated company", template: "generative" },
  { key: "onto_review", name: "Onto · Story review", template: "story-review" },
];

export function templateConfig(template, seed) {
  const business = template.key === "bellwether" || template.template === "harbor";
  const config = {
    actor_model: "openai/gpt-4.1-mini", world_model: "openai/gpt-4.1-mini", provider: "openai",
    actor_settings: { provider: "openrouter", temperature: 0.7, max_tokens: 1024, reasoning_effort: null },
    generator_settings: { provider: "openrouter", temperature: 0.7, max_tokens: 1024, reasoning_effort: null },
    action_limit: business ? 100 : template.template ? 30 : 20,
    expansion_limit: 10, call_limit: business ? 120 : template.template ? 180 : 40,
    dollar_limit: template.template === "accounts" ? 0.25 : 1,
    context_limit_bytes: 256000, memory: { recent_entries: 12, max_summary_bytes: 8000 },
    structured_actions: true,
    ...seed.config,
    demo: !!template.demo,
    actor_system_prompt: seed.system, actor_user_prompt: seed.world.goal,
  };
  return config;
}

// The controller owns selection and transport. Rendering consumes only recorded
// checkpoints, so replay never dispatches an execution command.
export function createRunApp(env = globalThis, { createCarousel = createAgentCarousel, updateWorld = reconcileWorld } = {}) {
  const $ = (id) => env.document.querySelector(`#${id}`);
  const state = {
    selection: "", runs: [], seed: null, data: null, frames: [], selected: 0, evidenceSeq: 0,
    follow: true, view: "facts", worldFocus: "world", pending: false, loading: false,
    keyAvailable: null, message: "", pollError: "", pauseRequested: false,
  };
  let version = 0, polling = false, timer, worldKey = "", agentStateKey = "";
  const carousel = createCarousel($("agent-card"), env);
  const savedId = () => state.selection.startsWith("run:") ? state.selection.slice(4) : null;
  const template = () => templates.find((item) => state.selection === `new:${item.key}`);
  const money = (value) => `$${Number(value ?? 0).toFixed(4).replace(/0+$/, "").replace(/\.$/, ".00")}`;
  const active = (run) => !!run && (run.busy || run.status === "running");
  const finished = (run) => !!run && (run.world.finished || ["finished", "exhausted"].includes(run.status)
    || run.actions >= run.config.action_limit || run.calls >= run.config.call_limit
    || (!run.config.demo && run.charged + run.reserved >= run.config.dollar_limit));
  async function api(path, body) {
    const response = await env.fetch(`/api/${path}`, body === undefined ? {} : {
      method: "POST", headers: { "Content-Type": "application/json", "X-Frontier-Client": "research-ui" },
      body: JSON.stringify(body),
    });
    const value = await response.json();
    if (!response.ok) throw new Error(value.error || `Request failed (${response.status}).`);
    return value;
  }
  function remember() {
    try { env.localStorage?.setItem("frontier-run-view", state.selection); } catch { /* Storage can be unavailable. */ }
    if (!env.history?.replaceState || !env.location?.href) return;
    const url = new URL(env.location.href);
    url.searchParams.delete("run");
    url.searchParams.delete("experiment");
    if (savedId()) url.searchParams.set("run", savedId());
    else if (template()) url.searchParams.set("experiment", template().key);
    env.history.replaceState(null, "", url);
  }
  function renderOptions() {
    const runs = [...state.runs];
    if (state.data && !runs.some((run) => run.id === state.data.run.id)) runs.unshift(state.data.run);
    const html = `<optgroup label="New experiment">${templates.map((item) =>
      `<option value="new:${item.key}">${esc(item.name)}</option>`).join("")}</optgroup>`
      + (runs.length ? `<optgroup label="Saved executions">${runs.map((run) =>
        `<option value="run:${esc(run.id)}">${esc(runLabel(run))} · ${esc(run.status)} · ${esc(run.id.slice(0, 8))}</option>`).join("")}</optgroup>` : "");
    if ($("experiment").innerHTML !== html) $("experiment").innerHTML = html;
    $("experiment").value = state.selection;
  }
  function blocker() {
    if (state.loading || state.pending) return "Wait for the current request.";
    if (!state.data && !state.seed) return "Select an experiment.";
    const run = state.data?.run;
    if (finished(run)) return "This execution has finished or reached its limit. Select a new experiment.";
    if (!run && state.seed?.runtime_available === false) return state.seed.runtime_error || "The Onto runtime is unavailable.";
    const config = run?.config || templateConfig(template(), state.seed);
    if (!config.demo && state.keyAvailable === false) return "Live execution needs a server-side OPENROUTER_API_KEY.";
    if (state.runs.some((other) => other.id !== savedId() && active(other)))
      return "Another experiment is running. Pause it before starting this one.";
    return "";
  }
  function render(animate = false) {
    const run = state.data?.run, draft = template();
    if (state.follow) state.selected = Math.max(0, state.frames.length - 1);
    state.selected = Math.max(0, Math.min(state.selected, state.frames.length - 1));
    renderOptions();
    const config = run?.config || (state.seed && draft ? templateConfig(draft, state.seed) : null);
    const label = run ? runLabel(run) : draft?.name || "Select an experiment";
    $("goal").textContent = label;
    env.document.title = `${label} · Frontier`;
    $("objective").textContent = run?.world.goal || state.seed?.world.goal || "Choose an experiment, then run it or advance one action at a time.";
    $("objective").hidden = false;
    $("experiment").disabled = state.pending || state.loading;
    const reason = blocker();
    const running = run?.status === "running";
    const reviewing = !!run && !state.follow && state.selected < state.frames.length - 1;
    $("run").textContent = state.pending ? "Working…" : running ? "Pause" : state.pauseRequested && run?.busy ? "Pausing…" : "Run";
    $("step").textContent = reviewing ? "Step forward" : run?.busy && !running && !state.pauseRequested ? "Stepping…" : "Step";
    $("run").disabled = state.pending || state.loading || (running ? false : !!reason || active(run));
    $("step").disabled = reviewing ? state.pending || state.loading : !!reason || active(run);
    $("back").disabled = state.pending || state.loading || state.selected === 0;
    $("run").title = running ? "Pause after the current action finishes" : reason || "Run until finished, paused, or a limit is reached";
    $("step").title = reviewing ? "Inspect the next recorded state; no action is executed" : reason || (active(run) ? "Wait for the current action to finish" : "Execute one agent action");
    $("back").title = "Inspect the previous recorded state; no action is executed";
    $("status").textContent = state.loading ? "Loading…" : state.pending ? "Updating…" : running ? "Running" : run?.busy
      ? state.pauseRequested ? "Pausing after this action" : "Executing one action" : run?.status === "error" ? "Stopped after error"
        : run?.status === "exhausted" ? "Limit reached" : finished(run) ? "Finished" : run ? "Paused" : "Ready";
    $("status").dataset.state = running || run?.busy ? "running" : run?.status || "ready";
    $("run-meta").textContent = config ? config.demo
      ? `Scripted · no model calls${run ? ` · ${run.actions} / ${config.action_limit} actions` : ""}`
      : `Live · ${config.actor_model} · ${money(config.dollar_limit)} run limit${run ? ` · ${money(run.charged)} charged${run.reserved ? ` + ${money(run.reserved)} reserved` : ""} · ${run.actions} / ${config.action_limit} actions · ${run.calls} calls` : ""}` : "";
    const runError = run?.status === "error" ? [...(state.data.events || [])].reverse().find((event) => event.kind === "error")?.evidence?.error : "";
    const warning = state.message || state.pollError || runError || (!state.loading && !state.pending && !active(run) && !finished(run) ? reason : "");
    $("error").textContent = warning;
    $("error").hidden = !warning;
    const frame = state.frames[state.selected];
    $("agent-meta").textContent = run ? config.demo ? "Scripted actor" : "Live actor" : "Starting state";
    $("world-label").textContent = frame?.world?.ontology ? "Institution" : "World";
    $("world-meta").textContent = frame?.world?.ontology ? "Ontolang" : state.follow ? "Latest checkpoint" : "Recorded checkpoint";
    $("position").textContent = frame ? `${run ? "Checkpoint" : "Preview"} ${state.selected + 1}${run ? ` of ${state.frames.length} · ${state.follow ? "following latest" : "viewing history"}` : " · run or step to begin"}` : "No execution selected";
    $("previous").disabled = !frame || state.selected === 0;
    $("next").disabled = !frame || state.selected >= state.frames.length - 1;
    $("history").disabled = state.frames.length < 2;
    $("history").max = Math.max(0, state.frames.length - 1);
    $("history").value = state.selected;
    $("history").setAttribute("aria-valuetext", `Checkpoint ${state.selected + 1} of ${state.frames.length}`);
    $("latest").hidden = !run || state.follow;
    $("latest").disabled = !run || state.follow;
    for (const view of ["facts", "tree", "domains"]) {
      $(`${view}-tab`).setAttribute("aria-selected", String(view === state.view));
      $(`${view}-tab`).tabIndex = view === state.view ? 0 : -1;
    }
    $("world-panel").setAttribute("aria-labelledby", `${state.view}-tab`);
    if (!frame) return;
    const nextAgentStateKey = `${state.selection}:${frame.event.seq}`;
    if (nextAgentStateKey !== agentStateKey) {
      updateWorld($("agent-state"), renderAgentState(frame));
      agentStateKey = nextAgentStateKey;
    }
    carousel.update(state.frames, state.selected, state.data || {}, state.selection, animate);
    // Read-only actions and refusals do not move the world card. The comparison
    // frame is kept with each changed world, so its field deltas remain visible.
    const signature = worldSignature(frame.world);
    let changedAt = state.selected;
    while (changedAt > 0 && worldSignature(state.frames[changedAt - 1].world) === signature) --changedAt;
    const nextWorldKey = `${state.selection}:${state.view}:${state.worldFocus}:${state.frames[changedAt].event.seq}:${signature}`;
    if (nextWorldKey !== worldKey) {
      updateWorld($("world-card"), renderWorld({ ...frame, world_checkpoint: changedAt + 1 }, state.frames[changedAt - 1], state.view, state.worldFocus));
      worldKey = nextWorldKey;
    }
  }

  function applyData(next) {
    if (next.unchanged) return;
    const selectedSeq = !state.follow ? state.frames[state.selected]?.event.seq : null;
    if (next.incremental) {
      if (!state.data || next.from_seq !== state.evidenceSeq) return;
      state.data = { ...state.data, ...next, events: [...state.data.events, ...next.events], frames: [...state.data.frames, ...next.frames] };
    } else state.data = next;
    state.evidenceSeq = next.run.seq;
    state.frames = traceFrames(state.data);
    if (selectedSeq != null) {
      const index = state.frames.findIndex((frame) => frame.event.seq === selectedSeq);
      if (index !== -1) state.selected = index;
    }
    if (!active(state.data.run)) state.pauseRequested = false;
  }
  async function select(selection) {
    if (state.pending) return;
    const token = ++version;
    state.loading = true;
    state.message = "";
    state.pollError = "";
    render();
    try {
      const item = templates.find((entry) => selection === `new:${entry.key}`);
      if (!item && !selection.startsWith("run:")) throw new Error("Unknown experiment.");
      const next = await api(item ? item.seed || `institution-seed?template=${item.template}` : `runs/${encodeURIComponent(selection.slice(4))}`);
      if (token !== version) return;
      state.selection = selection;
      state.seed = item ? next : null;
      state.data = null;
      state.evidenceSeq = 0;
      if (item) state.frames = [{ world: next.world, event: { seq: 0, kind: "seed", summary: "Experiment starting state", evidence: {} } }];
      else applyData(next);
      state.selected = 0;
      state.follow = true;
      state.pauseRequested = false;
      worldKey = agentStateKey = "";
      $("agent-state").innerHTML = "";
      state.worldFocus = "world";
      $("world-card").innerHTML = "";
      remember();
    } catch (error) {
      if (token === version) state.message = error.message;
    } finally {
      if (token === version) { state.loading = false; render(); }
    }
  }
  async function refresh() {
    if (polling || state.pending || state.loading) return;
    polling = true;
    const token = version, id = savedId(), seq = state.evidenceSeq;
    try {
      const [runs, next, health] = await Promise.all([
        api("runs"), id ? api(`runs/${encodeURIComponent(id)}${seq ? `?after_seq=${seq}&incremental=true` : ""}`) : null,
        api("health"),
      ]);
      if (token !== version) return;
      state.runs = runs;
      state.keyAvailable = health.key_available;
      if (next) applyData(next);
      state.pollError = "";
      render(true);
    } catch (error) {
      if (token === version) { state.pollError = `Could not refresh: ${error.message}`; render(); }
    } finally { polling = false; }
  }
  async function control(command) {
    if (state.pending || state.loading) return;
    if (command !== "pause" && (blocker() || active(state.data?.run))) return;
    ++version;
    state.pending = true;
    state.message = "";
    render();
    try {
      if (!savedId()) {
        const item = template(), seed = state.seed;
        const config = templateConfig(item, seed);
        const created = await api(item.template ? "institutions" : item.endpoint, item.template
          ? { name: item.name, template: item.template, config, institution: seed.institution, ...(seed.generation ? { generation: seed.generation } : {}) }
          : { name: item.name, ...config });
        // Remember successful creation before starting. A failed command must
        // keep this run selected, never create a second paid execution on retry.
        state.selection = `run:${created.id}`;
        state.runs.unshift(created);
        state.data = { run: created, frames: state.frames, events: [] };
        state.evidenceSeq = 0;
        state.seed = null;
        remember();
      }
      const id = savedId();
      const controlled = await api(`runs/${encodeURIComponent(id)}/control`, { command });
      state.data.run = controlled;
      state.follow = true;
      state.pauseRequested = command === "pause";
      applyData(await api(`runs/${encodeURIComponent(id)}`));
    } catch (error) { state.message = error.message; }
    finally { state.pending = false; render(true); }
  }
  function navigate(index) {
    state.selected = Math.max(0, Math.min(index, state.frames.length - 1));
    state.follow = state.selected === state.frames.length - 1;
    render(true);
    $("announcement").textContent = $("position").textContent;
  }
  function setView(view) { state.view = view; render(); }
  async function start() {
    $("experiment").onchange = () => select($("experiment").value);
    $("run").onclick = () => control(state.data?.run.status === "running" ? "pause" : "run");
    $("step").onclick = () => !state.follow && state.selected < state.frames.length - 1 ? navigate(state.selected + 1) : control("step");
    $("back").onclick = () => navigate(state.selected - 1);
    $("history").oninput = () => navigate(Number($("history").value));
    $("previous").onclick = () => navigate(state.selected - 1);
    $("next").onclick = () => navigate(state.selected + 1);
    $("latest").onclick = () => navigate(state.frames.length - 1);
    $("agent-card").onclick = (event) => {
      const button = event.target.closest("[data-checkpoint]");
      if (button) navigate(Number(button.dataset.checkpoint));
    };
    for (const view of ["facts", "tree", "domains"]) {
      $(`${view}-tab`).onclick = () => setView(view);
      $(`${view}-tab`).onkeydown = (event) => {
        if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
          event.preventDefault();
          const views = ["facts", "tree", "domains"];
          const next = event.key === "Home" ? views[0] : event.key === "End" ? views.at(-1) : views[(views.indexOf(view) + (event.key === "ArrowRight" ? 1 : 2)) % views.length];
          setView(next); $(`${next}-tab`).focus();
        }
      };
    }
    $("world-card").onclick = (event) => {
      const target = event.target.closest("[data-world-focus]");
      if (!target) return;
      state.worldFocus = target.dataset.worldFocus;
      render();
      $("world-card").querySelector('[aria-current="location"]')?.focus({ preventScroll: true });
    };
    $("world-card").onkeydown = (event) => {
      if (["Enter", " "].includes(event.key) && event.target.matches('g[data-world-focus]')) {
        event.preventDefault(); event.target.dispatchEvent(new env.MouseEvent("click", { bubbles: true }));
      }
    };
    let stored;
    try { stored = env.localStorage?.getItem("frontier-run-view"); } catch { /* Optional preference. */ }
    const params = new URLSearchParams(env.location?.search || "");
    const selection = params.get("run") ? `run:${params.get("run")}` : params.get("experiment")
      ? `new:${params.get("experiment")}` : stored || "new:flowers-scripted";
    try {
      const [runs, health] = await Promise.all([api("runs"), api("health")]);
      state.runs = runs; state.keyAvailable = health.key_available;
    } catch (error) { state.message = error.message; }
    await select(selection);
    timer = env.setInterval(() => { void refresh(); }, 1500);
    return state;
  }
  return { start, select, refresh, control, navigate, setView, state, stop: () => { env.clearInterval(timer); carousel.destroy(); } };
}

if (typeof document !== "undefined") void createRunApp().start();
