"use strict";
const $ = (s) => document.querySelector(s);
const esc = (s) =>
  String(s ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
let current = localStorage.getItem("frontier-run"),
  data = null,
  runs = [],
  selected = null,
  filter = "story",
  pending = false;
async function api(path, body) {
  const response = await fetch(
    `/api/${path}`,
    body === undefined
      ? {}
      : {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Frontier-Client": "research-ui",
          },
          body: JSON.stringify(body),
        },
  );
  const text = await response.text();
  let value;
  try {
    value = JSON.parse(text);
  } catch {
    throw Error(`Server returned HTTP ${response.status}`);
  }
  if (!response.ok) throw Error(value.error || `HTTP ${response.status}`);
  return value;
}
function error(e) {
  $("#notice").textContent = e.message;
  $("#notice").hidden = false;
}
async function act(fn) {
  if (pending) return;
  pending = true;
  $("#notice").hidden = true;
  try {
    await fn();
    await refresh();
  } catch (e) {
    error(e);
  } finally {
    pending = false;
  }
}
function choose(id) {
  current = id;
  selected = null;
  localStorage.setItem("frontier-run", id);
  return refresh();
}
async function refresh() {
  const requested = current;
  runs = await api("runs");
  $("#run-count").textContent = runs.length;
  $("#runs").innerHTML = runs
    .map(
      (r) =>
        `<button class="run-link ${r.id === current ? "active" : ""}" data-run="${r.id}"><span>${esc(r.name)}</span><small>${r.demo ? "SCRIPTED" : "LIVE"} · ${esc(r.status)} · ${r.id.slice(0, 6)}</small></button>`,
    )
    .join("");
  if (!current || !runs.some((r) => r.id === current)) {
    current = null;
    $("#empty").hidden = false;
    $("#workspace").hidden = true;
    return;
  }
  const next = await api(`runs/${current}`);
  if (requested !== current) return;
  if (data?.run.id === next.run.id && data.run.seq === next.run.seq) return;
  data = next;
  render();
}
function render() {
  const r = data.run;
  $("#empty").hidden = true;
  $("#workspace").hidden = false;
  $("#run-title").textContent = r.name;
  $("#status").textContent =
    `${r.config.demo ? "SCRIPTED" : "LIVE"} / ${r.busy ? "WORKING" : r.status.toUpperCase()}`;
  $("#actions").innerHTML =
    `${r.actions} <small>/ ${r.config.action_limit}</small>`;
  $("#progress").value = r.actions;
  $("#progress").max = r.config.action_limit;
  $("#resources").textContent = Object.keys(r.world.documents).length;
  $("#expansions").textContent =
    `${r.expansions} / ${r.config.expansion_limit} expansions`;
  $("#cost").textContent = `$${r.charged.toFixed(4)}`;
  $("#budget").textContent =
    `$${r.config.dollar_limit.toFixed(2)} cap · $${r.reserved.toFixed(2)} reserved`;
  $("#models").textContent = r.config.demo
    ? "Scripted fixture"
    : `${r.config.actor_model} / ${r.config.world_model}`;
  $("#provider").textContent = r.config.demo
    ? "No model calls · not research evidence"
    : `${r.config.provider} · ${r.calls}/${r.config.call_limit} calls · fallback off`;
  $("#activity-label").textContent = r.busy
    ? "Agent action in progress…"
    : "Trajectory persisted to SQLite";
  const stopped = r.world.finished || r.actions >= r.config.action_limit;
  $("#step").disabled = $("#play").disabled =
    r.busy || r.status === "running" || stopped;
  $("#pause").disabled = !r.busy && r.status !== "running";
  $("#deadline").disabled = $("#revoke").disabled =
    r.busy || r.status === "running" || r.world.finished;
  $("#deadline-text").textContent = r.world.deadline;
  $("#branch-note").hidden = !r.parent;
  if (r.parent)
    $("#branch-note").textContent =
      `FORKED FROM ${r.parent.slice(0, 8)} · CHECKPOINT #${r.fork_seq} · shared-state SHA-256 ${r.fork_hash.slice(0, 16)}…`;
  $("#event-count").textContent = data.events.length;
  const events = data.events.filter(
    (e) =>
      filter === "all" ||
      ![
        "inference",
        "response",
        "control",
        "generation",
        "materialized",
      ].includes(e.kind),
  );
  $("#events").innerHTML = events
    .slice()
    .reverse()
    .map(
      (e) =>
        `<button class="event ${selected?.type === "event" && selected.id === e.seq ? "active" : ""}" data-event="${e.seq}"><div class="event-top"><span>#${String(e.seq).padStart(3, "0")}</span><span class="kind ${esc(e.kind)}">${esc(e.kind)}</span><time>${new Date(e.timestamp * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}</time></div><strong>${esc(e.summary)}</strong></button>`,
    )
    .join("");
  $("#documents").innerHTML = Object.entries(r.world.documents)
    .map(
      ([id, d]) =>
        `<button class="document" data-document="${esc(id)}"><span class="icon">${d.readable ? "▤" : "⊘"}</span><span><h4>${esc(d.title)}</h4><small class="${d.origin === "world model" || d.origin === "scripted fixture" ? "generated" : ""}">${esc(d.origin.toUpperCase())} · ${d.readable ? "AVAILABLE TO READ" : "RESTRICTED"}</small></span></button>`,
    )
    .join("");
  inspect();
}
function inspect() {
  const box = $("#inspector-content");
  if (!selected) {
    box.innerHTML =
      '<div class="inspector-empty"><div class="symbol">⌗</div><h4>Follow the evidence.</h4><p>Select an event to see what the agent<br>attempted and what actually happened.<br><br>Or inspect a world resource<br>and its provenance.</p></div>';
    return;
  }
  if (selected.type === "document") {
    const d = data.run.world.documents[selected.id];
    if (!d) {
      selected = null;
      return inspect();
    }
    box.innerHTML = `<span class="source">RESOURCE / ${esc(selected.id)}</span><h2 class="evidence-title">${esc(d.title)}</h2><div class="evidence-label">${d.readable ? "AVAILABLE VIA READ TOOL" : "RESEARCHER ONLY · AGENT ACCESS DENIED"}</div><div class="evidence-body">${esc(d.body)}</div><div class="evidence-label">PROVENANCE</div><span class="source">${esc(d.origin)}</span>`;
    return;
  }
  const e = data.events.find((e) => e.seq === selected.id);
  if (!e) {
    selected = null;
    return inspect();
  }
  const result = e.evidence.result,
    action = e.evidence.action;
  box.innerHTML = `<span class="source">EVENT #${e.seq} / ${esc(e.kind.toUpperCase())}</span><h2 class="evidence-title">${esc(e.summary)}</h2>${action ? `<div class="evidence-label">ATTEMPT</div><div class="evidence-body">${esc(action.kind)} → ${esc(action.target)}${action.text ? "\n" + esc(action.text) : ""}</div>` : ""}${result ? `<div class="evidence-label">OBSERVED OUTCOME</div><div class="evidence-body">${esc(result.body || result.error || JSON.stringify(result, null, 2))}</div>` : e.evidence.notice ? `<div class="evidence-body">${esc(e.evidence.notice)}</div>` : e.evidence.error ? `<div class="evidence-body">${esc(e.evidence.error)}</div>` : "<p>Recorded system event. Full evidence is available below.</p>"}<div class="evidence-label">CHECKPOINT</div><span class="source">${e.checkpoint ? "Complete state · safe to fork" : "Action in flight · not forkable"}</span><button class="fork-button" id="fork-selected" ${e.checkpoint ? "" : "disabled"}>⑂ Fork from checkpoint #${e.seq}</button><details><summary>Inspect raw evidence</summary><pre>${esc(JSON.stringify(e.evidence, null, 2))}</pre></details>`;
  $("#fork-selected").onclick = () =>
    act(async () => {
      const r = await api(`runs/${current}/fork`, { seq: e.seq });
      await choose(r.id);
    });
}
$("#new-run").onclick = $("#empty-create").onclick = () =>
  $("#setup").showModal();
document
  .querySelectorAll("[data-close]")
  .forEach(
    (b) => (b.onclick = () => document.getElementById(b.dataset.close).close()),
  );
$("#setup-form").onsubmit = (e) => {
  e.preventDefault();
  act(async () => {
    const f = new FormData(e.target);
    const config = Object.fromEntries(f);
    config.demo = config.demo === "true";
    config.action_limit = Number(config.action_limit);
    config.dollar_limit = Number(config.dollar_limit);
    config.expansion_limit = 20;
    config.call_limit = 120;
    const r = await api("runs", config);
    $("#setup").close();
    await choose(r.id);
  });
};
$("#runs").onclick = (e) => {
  const b = e.target.closest("[data-run]");
  if (b) act(() => choose(b.dataset.run));
};
$("#events").onclick = (e) => {
  const b = e.target.closest("[data-event]");
  if (b) {
    selected = { type: "event", id: Number(b.dataset.event) };
    render();
  }
};
$("#documents").onclick = (e) => {
  const b = e.target.closest("[data-document]");
  if (b) {
    selected = { type: "document", id: b.dataset.document };
    inspect();
  }
};
document.querySelectorAll("[data-filter]").forEach(
  (b) =>
    (b.onclick = () => {
      filter = b.dataset.filter;
      document
        .querySelectorAll("[data-filter]")
        .forEach((x) => x.classList.toggle("selected", x === b));
      render();
    }),
);
for (const [id, command] of [
  ["step", "step"],
  ["play", "run"],
  ["pause", "pause"],
])
  $(`#${id}`).onclick = () =>
    act(() => api(`runs/${current}/control`, { command }));
for (const command of ["deadline", "revoke"])
  $(`#${command}`).onclick = () =>
    act(() => api(`runs/${current}/intervene`, { command }));
$("#export").onclick = () => {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
  );
  const a = document.createElement("a");
  a.href = url;
  a.download = `frontier-${current}.json`;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
};
$("#compare").onclick = () =>
  act(async () => {
    const others = runs.filter((r) => r.id !== current);
    $("#compare-select").innerHTML = others
      .map(
        (r) =>
          `<option value="${r.id}">${esc(r.name)} · ${r.id.slice(0, 8)}</option>`,
      )
      .join("");
    $("#comparison-body").textContent = others.length
      ? ""
      : "Fork a checkpoint to create a comparison branch.";
    if (others.length) await comparison();
    $("#comparison").showModal();
  });
$("#compare-select").onchange = () => comparison().catch(error);
async function comparison() {
  const target = $("#compare-select").value;
  $("#comparison-body").textContent = "Loading comparison…";
  const other = await api(`runs/${target}`);
  if (target !== $("#compare-select").value) return;
  const a = data.run,
    b = other.run;
  const related =
    a.parent === b.id ||
    b.parent === a.id ||
    (a.parent && a.parent === b.parent && a.fork_seq === b.fork_seq);
  const split =
    a.parent === b.id ? a.fork_seq : b.parent === a.id ? b.fork_seq : null;
  const cards = [data, other]
    .map((d) => {
      const r = d.run;
      const isParent = a.parent === r.id || b.parent === r.id;
      const events = d.events.filter(
        (e) =>
          ["outcome", "denied", "expansion", "intervention", "error"].includes(
            e.kind,
          ) &&
          (!isParent || split === null || e.seq > split),
      );
      return `<section class="compare-card"><h3>${esc(r.name)} · ${r.id.slice(0, 6)}</h3><div class="source">${r.actions} actions · ${r.expansions} expansions · $${r.charged.toFixed(4)}<br>${esc(r.status)} · ${r.config.demo ? "scripted fixture" : "live model"}</div><p>${esc(r.world.deadline)}</p>${events.map((e) => `<div class="entry"><span class="kind ${esc(e.kind)}">#${e.seq} ${esc(e.kind)}</span><br>${esc(e.summary)}${e.evidence.action?.text ? `<p>${esc(e.evidence.action.text)}</p>` : ""}${e.evidence.result?.error ? `<p>${esc(e.evidence.result.error)}</p>` : ""}${e.evidence.notice ? `<p>${esc(e.evidence.notice)}</p>` : ""}</div>`).join("") || "<p>No post-checkpoint outcomes yet.</p>"}</section>`;
    })
    .join("");
  $("#comparison-body").innerHTML =
    `<div class="comparison-note">${related ? "Shared ancestry found. Showing post-fork events where directly comparable." : "These runs do not share a direct checkpoint. This is not a controlled comparison."}</div><div class="compare-grid">${cards}</div>`;
}
api("health")
  .then(
    (h) =>
      ($("#key-status").textContent = h.key_available
        ? "OpenRouter key configured"
        : "No API key · demo available"),
  )
  .catch(error);
refresh().catch(error);
setInterval(() => {
  if (!pending && !$("#setup").open && !$("#comparison").open)
    refresh().catch(error);
}, 1800);
