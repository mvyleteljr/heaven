const $ = (s) => document.querySelector(s);
const esc = (s) =>
  String(s ?? "").replace(
    /[&<>"']/g,
    (c) =>
      ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[
        c
      ],
  );
const pretty = (x) => JSON.stringify(x, null, 2);
const money = (cents) =>
  new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" }).format(
    cents / 100,
  );
let id = localStorage.getItem("flower-run"),
  data = null,
  seed,
  selected = 0,
  follow = true,
  pending = false;
let refreshVersion = 0;
let refreshes = 0;
async function api(path, body) {
  const r = await fetch(
    "/api/" + path,
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
  const value = await r.json();
  if (!r.ok) throw Error(value.error);
  return value;
}
function error(e) {
  $("#error").textContent = e.message;
  $("#error").hidden = false;
}
async function act(fn) {
  if (pending) return;
  ++refreshVersion;
  pending = true;
  settingsState();
  $("#error").hidden = true;
  try {
    await fn();
    await refresh();
  } catch (e) {
    error(e);
  } finally {
    pending = false;
    settingsState();
  }
}
let models = [];
let prompts;
let editingPrompt = null;
const settingIds = [
  "experiment",
  "run-name",
  "mode",
  "actor",
  "temperature",
  "max-tokens",
  "reasoning",
  "generator",
  "generator-temperature",
  "generator-max-tokens",
  "generator-reasoning",
  "context-limit",
  "memory-mode",
  "action-format",
];
function capabilities() {
  for (const [modelId, prefix] of [
    ["actor", ""],
    ["generator", "generator-"],
  ]) {
    const model = models.find((m) => m.id === $("#" + modelId).value);
    const efforts = model?.reasoning_efforts || [];
    const unsupported = model && !efforts.length;
    const field = $("#" + prefix + "reasoning");
    const value = field.value;
    const options = [
      new Option("Provider default (omitted)", ""),
      ...efforts.map((effort) => new Option(effort, effort)),
    ];
    if (id && value && !efforts.includes(value))
      options.push(new Option(value + " (saved)", value));
    if (
      Array.from(field.options, (o) => o.value).join() !==
      options.map((o) => o.value).join()
    )
      field.replaceChildren(...options);
    field.value = id || efforts.includes(value) ? value : "";
    field.disabled = !!id || pending || !model || unsupported;
    $("#" + prefix + "reasoning-note").textContent = unsupported
      ? model.supported_parameters?.includes("reasoning")
        ? "Effort selection unavailable"
        : "Reasoning not supported"
      : !model
        ? "Capabilities unavailable"
        : "";
    const noTemperature =
      model && !model.supported_parameters?.includes("temperature");
    $("#" + prefix + "temperature").disabled =
      !!id || pending || !!noTemperature;
    if (!id && model) {
      if (unsupported) $("#" + prefix + "reasoning").value = "";
      if (noTemperature) $("#" + prefix + "temperature").value = "";
    }
  }
}
function settingsState() {
  settingIds.forEach((key) => ($("#" + key).disabled = !!id || pending));
  $("#mode option[value='true']").disabled =
    $("#experiment").value !== "flowers";
  capabilities();
  $("#new").disabled = !!id || pending || !!editingPrompt;
  $("#runs").disabled = pending || !!editingPrompt;
  document.querySelectorAll(".edit-prompt").forEach((button) => {
    button.hidden = !!id;
    button.disabled = pending || !!editingPrompt;
  });
  $("#settings-status").textContent = pending
    ? "Working…"
    : id
      ? "Inspecting saved settings · read-only"
      : "Configure a new run, then validate & save.";
  const working = data?.run.busy || data?.run.status === "running";
  $("#step").disabled = pending || !id || !!working || !!data?.run.world.finished || data?.run.actions >= data?.run.config.action_limit;
  $("#run").disabled = $("#step").disabled;
  $("#pause").disabled = pending;
}
function showSettings(config) {
  const settings = config.actor_settings;
  const values = {
    "memory-mode": config.memory ? "rolling" : "full",
    "action-format": config.structured_actions ? "structured" : "legacy",
    "context-limit": config.context_limit_bytes ?? 40000,
    mode: String(config.demo),
    actor: config.actor_model,
    generator: config.world_model,
    "generator-temperature": config.generator_settings
      ? (config.generator_settings.temperature ?? "")
      : 0.7,
    "generator-max-tokens": config.generator_settings?.max_tokens ?? 1024,
    "generator-reasoning": config.generator_settings?.reasoning_effort ?? "",
    temperature: settings ? (settings.temperature ?? "") : 0.7,
    "max-tokens": settings?.max_tokens ?? 1024,
    reasoning: settings?.reasoning_effort ?? "",
  };
  Object.entries(values).forEach(([key, value]) => {
    const field = $("#" + key);
    if (
      ["actor", "generator", "reasoning", "generator-reasoning"].includes(
        key,
      ) &&
      !Array.from(field.options).some((o) => o.value === value)
    )
      field.add(new Option(value, value));
    field.value = value;
  });
}
function frames() {
  return (
    data?.frames || [
      { world: seed.world, event: { evidence: {}, kind: "seed" } },
    ]
  );
}
function render() {
  const fs = frames();
  if (follow) selected = fs.length - 1;
  selected = Math.min(selected, fs.length - 1);
  const frame = fs[selected],
    w = frame.world.flowers ?? {},
    prev = selected ? (fs[selected - 1].world.flowers ?? {}) : null,
    evidence = frame.event.evidence;
  $("h1").textContent =
    data?.run.name ??
    (seed.world.business ? "Harbor Works" : "Flowers for Friday");
  document.title = `${$("h1").textContent} · World trace`;
  $("#system").textContent = data?.initial.actor_instructions ?? prompts.system;
  $("#director-policy").textContent = frame.world.business
    ? "Bellwether · bounded counterparties & scheduled check-ins"
    : data && !data.run.experiment
      ? "Legacy run · no director"
      : "Neutral · construct missing resources";
  $("#task").textContent = data ? fs[0].world.goal : prompts.task;
  const configuredTools = data?.observation.tools ?? seed.tools;
  $("#tools").textContent = Array.isArray(configuredTools)
    ? configuredTools
        .map(
          (tool) =>
            `${tool.name}(${[
              ...tool.input_schema.required,
              ...Object.keys(tool.input_schema.properties).filter(
                (key) => !tool.input_schema.required.includes(key),
              ),
            ]
              .map(
                (key) =>
                  key + (tool.input_schema.required.includes(key) ? "" : "?"),
              )
              .join(", ")})`,
        )
        .join("\n")
    : configuredTools.split("; ").join("\n");
  $("#seed").textContent = pretty(
    fs[0].world.business ??
      fs[0].world.flowers ??
      fs[0].world.tools?.public_resources,
  );
  $("#seed-description").hidden = !frame.world.flowers;
  $("#position").textContent = `Step ${selected} of ${fs.length - 1}`;
  $("#scrubber").max = fs.length - 1;
  $("#scrubber").value = selected;
  const last = fs.length - 1,
    interval = Math.max(1, Math.ceil(last / 10));
  $("#ticks").innerHTML = fs
    .map((_, i) =>
      (i % interval === 0 && last - i >= interval / 2) || i === last
        ? `<span style="left:${last ? (100 * i) / last : 0}%">${i}</span>`
        : "",
    )
    .join("");
  $("#world-title").textContent = `World at step ${selected}`;
  const changed = (key) => prev && pretty(w[key]) !== pretty(prev[key]);
  const node = (key, title, body) =>
    `<div class="node ${w[key] ? "" : "missing"} ${changed(key) ? "changed" : ""}"><strong>${title}</strong>${body}</div>`;
  $("#world").innerHTML =
    `<div class="node root"><strong>${esc(w.florist)}</strong><span>${esc(w.time)} · website & ordering service</span></div><div class="branches">${node("catalog", "Catalog", w.catalog ? w.catalog.map((p) => `<p>${esc(p.name)}<br><span>${money(p.price_cents)} · ${p.sympathy_appropriate ? "Sympathy" : "Other occasion"}</span></p>`).join("") : "<span>Not constructed</span>")}${node("delivery", "Delivery & policies", w.delivery ? w.delivery.options.map((d) => `<p>${esc(d.label)}<br><span>${money(d.fee_cents)} · Friday, ${d.arrival_hour}:00</span></p>`).join("") : "<span>Not constructed</span>")}</div><div class="order">${node("order", "Order", w.order ? `<span>${esc(w.order.confirmation)} · ${esc(w.order.status)}</span><p>${esc(w.order.arrangement)}<br>${money(w.order.total_cents)} total · Friday, ${w.order.arrival_hour}:00<br>${esc(w.order.address)}</p>` : "<span>Not placed</span>")}</div><div class="wallet"><span>Payment method · available</span><strong>${money(w.balance_cents)}</strong></div>`;
  if (!frame.world.flowers) {
    const resources = frame.world.tools?.public_resources ?? {};
    const previous = selected
      ? (fs[selected - 1].world.tools?.public_resources ?? {})
      : resources;
    $("#world").innerHTML = Object.entries(resources)
      .map(
        ([url, resource]) =>
          `<div class="node ${!previous[url] ? "changed" : ""}"><strong>${esc(resource.title)}</strong><span>${esc(url)}</span><details><summary>Content</summary><pre>${esc(resource.body)}</pre></details></div>`,
      )
      .join("");
  }
  if (frame.world.business) {
    const b = frame.world.business,
      contracts = b.contracts.filter((c) => c.seller === "harbor"),
      paid = contracts.filter((c) => c.paid),
      accepted = contracts.filter((c) => c.accepted);
    $("#world").innerHTML =
      `<div class="node root"><strong>Harbor Works · day ${Math.floor(frame.world.tools.elapsed_seconds / 86400)} / 30</strong><span>${money(b.organizations.harbor.cash)} cash · ${contracts.length} agreements · ${accepted.length} accepted · ${paid.length} paid</span></div><div class="branches">${contracts.map((c) => `<div class="node"><strong>${esc(b.organizations[c.buyer].name)}</strong><span>${esc(c.capability)} · ${money(c.price_cents)}<br>${c.paid ? "Paid" : c.accepted ? "Accepted · unpaid" : c.delivered_at !== null ? "Awaiting customer validation" : "Delivery owed"}<br>Due day ${(c.deadline / 86400).toFixed(1)} · ${esc(c.service ?? "No service delivered")}</span></div>`).join("") || '<div class="node missing">No customer agreements yet.</div>'}</div><div class="node"><strong>District · ${Object.keys(b.organizations).length - 1} other organizations</strong><span>${b.checkins} / 4 scheduled check-ins · ${money(b.backup_spent)} spent on alternatives</span></div><details><summary>Public resources · ${Object.keys(frame.world.tools.public_resources).length}</summary>${$("#world").innerHTML}</details>`;
  }
  $("#state").textContent = pretty(
    frame.world.business
      ? frame.world
      : frame.world.tools
        ? { ...w, tools: frame.world.tools }
        : w,
  );
  $("#memory-state").textContent = frame.memory
    ? pretty(frame.memory)
    : "Full history; no rolling summary at this checkpoint.";
  const action = evidence.action;
  $("#action").textContent = action
    ? action.inputs
      ? `${action.kind}\n${pretty(action.inputs)}`
      : `${action.kind}(${action.target || ""})${action.text ? "\n" + action.text : ""}`
    : selected === 0
      ? "No action yet."
      : frame.event.summary;
  let result = evidence.result || evidence;
  if (action?.kind === "http.request" && typeof result.body === "string") {
    try {
      result = { ...result, body: JSON.parse(result.body) };
    } catch {
      /* Keep non-JSON bodies as text. */
    }
  }
  $("#result").textContent =
    selected === 0
      ? "The agent has the task and tools. World details must be discovered through tools."
      : pretty(result);
  const changes = prev ? Object.keys(w).filter((k) => changed(k)) : [];
  $("#changes").innerHTML = changes.length
    ? changes
        .map(
          (k) =>
            `<p>${esc(k === "balance_cents" ? `Balance: ${money(prev[k])} → ${money(w[k])}` : k === "catalog" ? `Catalog created · ${w.catalog.length} bouquets` : k === "delivery" ? `Delivery service created · ${w.delivery.options.length} options and policies` : k === "order" ? `Order ${w.order.confirmation} confirmed · ${money(w.order.total_cents)}` : k + " changed")}</p>`,
        )
        .join("")
    : "";
  if (selected && frame.world.tools) {
    for (const [key, label] of [
      ["workspace", "Workspace files"],
      ["messages", "Mailbox"],
      ["public_resources", "Public resources"],
      ["elapsed_seconds", "World time"],
    ]) {
      if (
        pretty(frame.world.tools[key]) !==
        pretty(fs[selected - 1].world.tools?.[key])
      )
        $("#changes").innerHTML +=
          `<p>${label}${key === "elapsed_seconds" ? `: ${fs[selected - 1].world.tools?.elapsed_seconds ?? 0} → ${frame.world.tools.elapsed_seconds} seconds` : " changed."}</p>`;
    }
  }
  if (!$("#changes").innerHTML)
    $("#changes").innerHTML = "<p>No world state changes.</p>";
  if (selected && frame.world.business) {
    const before = fs[selected - 1].world.business,
      after = frame.world.business;
    for (const key of [
      "organizations",
      "offers",
      "contracts",
      "services",
      "ledger",
      "checkins",
    ]) {
      if (pretty(before?.[key]) !== pretty(after[key]))
        $("#changes").innerHTML += `<p>${esc(key)} changed.</p>`;
    }
  }
  const priorSeq = selected ? fs[selected - 1].event.seq : 0;
  const generation = (data?.events || []).filter(
    (e) =>
      e.seq > priorSeq &&
      e.seq <= frame.event.seq &&
      (e.evidence.role === "world" || e.evidence.role === "memory" || e.kind === "memory" ||
        [
          "director",
          "construction_request",
          "published",
          "construction_rejected",
          "institution",
        ].includes(e.kind)),
  );
  $("#generation").hidden = generation.length === 0;
  $("#generation-data").textContent = pretty(generation.map((e) => e.evidence));
  const working = data?.run.busy || data?.run.status === "running";
  $("#status").textContent = data
    ? `${data.run.config.demo ? "Scripted" : "Live"} · ${working ? "working…" : data.run.status}`
    : "Not started";
  $("#pause").hidden = !working;
  $("#run").hidden = !!working;
  $("#step").disabled =
    !id ||
    !!working ||
    !!data?.run.world.finished ||
    data?.run.status === "exhausted";
  $("#run").disabled = $("#step").disabled;
  settingsState();
}
async function refresh() {
  ++refreshes;
  try {
    const version = ++refreshVersion;
    const request = id;
    const runs = (await api("runs")).filter((r) => r.flowers || r.experiment);
    if (version !== refreshVersion || request !== id) return;
    if (id && !runs.some((r) => r.id === id)) {
      id = null;
      data = null;
      $("#run-name").value = "";
      $("#context-limit").value = 256000;
      $("#memory-mode").value = "rolling";
      $("#action-format").value = "structured";
      localStorage.removeItem("flower-run");
      await loadSeed("bellwether");
      if (version !== refreshVersion || id) return;
    }
    $("#runs").innerHTML =
      '<option value="">New run — configure</option><optgroup label="Inspect saved run">' +
      runs
        .map(
          (r) =>
            `<option value="${esc(r.id)}">Inspect · ${esc(r.name)} · ${r.id.slice(0, 6)}</option>`,
        )
        .join("") +
      "</optgroup>";
    $("#runs").value = id || "";
    if (id) {
      const query = data?.run.id === id ? `?after_seq=${data.run.seq}&incremental=true` : "";
      const next = await api(`runs/${id}${query}`);
      if (version !== refreshVersion || request !== id) return;
      if (next.unchanged) return;
      if (data?.run.id === next.run.id && data.run.seq >= next.run.seq) return;
      if (data?.run.id !== next.run.id) {
        $("#experiment").value = next.run.experiment ?? "flowers";
        showSettings(next.run.config);
        prompts = {
          system: next.initial.actor_instructions,
          task: next.frames[0].world.goal,
        };
        $("#run-name").value = next.run.name;
      }
      if (next.incremental) {
        if (data?.run.id !== next.run.id || data.run.seq !== next.from_seq) return;
        data = {...data,...next,events:[...data.events,...next.events],frames:[...data.frames,...next.frames]};
      } else { data = next; }
    }
    render();
  } finally {
    --refreshes;
  }
}
async function create() {
  for (const key of settingIds)
    if (!$("#" + key).checkValidity())
      throw Error("Check the model settings values.");
  const bellwether = $("#experiment").value === "bellwether";
  const r = await api($("#experiment").value, {
    name: $("#run-name").value,
    actor_system_prompt: prompts.system,
    actor_user_prompt: prompts.task,
    actor_model: $("#actor").value,
    actor_settings: {
      provider: "openrouter",
      temperature:
        $("#temperature").value === "" ? null : Number($("#temperature").value),
      max_tokens: Number($("#max-tokens").value),
      reasoning_effort: $("#reasoning").value || null,
    },
    world_model: $("#generator").value,
    generator_settings: {
      provider: "openrouter",
      temperature:
        $("#generator-temperature").value === ""
          ? null
          : Number($("#generator-temperature").value),
      max_tokens: Number($("#generator-max-tokens").value),
      reasoning_effort: $("#generator-reasoning").value || null,
    },
    provider: "openai",
    demo: $("#mode").value === "true",
    action_limit: bellwether ? 100 : 20,
    expansion_limit: 10,
    call_limit: bellwether ? 120 : 40,
    dollar_limit: 1,
    context_limit_bytes: Number($("#context-limit").value),
    memory: $("#memory-mode").value === "rolling" ? {recent_entries:12,max_summary_bytes:8000} : null,
    structured_actions: $("#action-format").value === "structured",
  });
  id = r.id;
  data = null;
  follow = true;
  localStorage.setItem("flower-run", id);
}
$("#new").onclick = () => act(create);
for (const key of ["system", "task"]) {
  const button = $(`.edit-prompt[data-prompt="${key}"]`);
  const close = () => {
    editingPrompt = null;
    $("#" + key + "-editor").hidden = true;
    $("#" + key).hidden = false;
    render();
    button.focus();
  };
  button.onclick = () => {
    editingPrompt = key;
    $("#" + key + "-input").value = prompts[key];
    $("#" + key).hidden = true;
    $("#" + key + "-editor").hidden = false;
    settingsState();
    $("#" + key + "-input").focus();
  };
  $("#" + key + "-editor").onsubmit = (event) => {
    event.preventDefault();
    prompts[key] = $("#" + key + "-input").value;
    close();
  };
  $(`.cancel-prompt[data-prompt="${key}"]`).onclick = close;
}
$("#runs").onchange = () =>
  act(async () => {
    id = $("#runs").value || null;
    data = null;
    if (!id) {
      $("#run-name").value = "";
      $("#context-limit").value = 256000;
      $("#memory-mode").value = "rolling";
      $("#action-format").value = "structured";
      await loadSeed(
        $("#experiment").value === "records"
          ? "bellwether"
          : $("#experiment").value,
      );
    }
    follow = true;
    $("#settings").open = true;
    localStorage.setItem("flower-run", id || "");
  });
for (const command of ["step", "run", "pause"])
  $("#" + command).onclick = () =>
    act(async () => {
      if (!id) throw Error("Validate and save the settings before running.");
      follow = true;
      await api(`runs/${id}/control`, { command });
    });
$("#scrubber").oninput = () => {
  selected = Number($("#scrubber").value);
  follow = false;
  render();
};
$("#latest").onclick = () => {
  follow = true;
  render();
};
async function loadSeed(experiment) {
  $("#experiment").value = experiment;
  seed = await api(
    experiment === "bellwether" ? "bellwether-seed" : "flower-seed",
  );
  prompts = { system: seed.system, task: seed.world.goal };
  if (experiment === "bellwether") $("#mode").value = "false";
}
$("#experiment").onchange = () =>
  act(async () => {
    await loadSeed($("#experiment").value);
    data = null;
    follow = true;
  });
await loadSeed("bellwether").catch((e) => {
  error(e);
  throw e;
});
prompts = { system: seed.system, task: seed.world.goal };
for (const key of ["actor", "generator"]) $("#" + key).onchange = capabilities;
api("models")
  .then((catalog) => {
    models = catalog.sort((a, b) => a.id.localeCompare(b.id));
    for (const key of ["actor", "generator"]) {
      const select = $("#" + key),
        value = select.value;
      select.replaceChildren(...models.map((m) => new Option(m.id, m.id)));
      if (!models.some((m) => m.id === value))
        select.add(new Option(value, value));
      select.value = value;
    }
    $("#catalog-status").hidden = true;
    capabilities();
  })
  .catch(() => {
    $("#catalog-status").textContent =
      "Model list unavailable. Reload to retry; saved models remain available.";
    capabilities();
  });
await refresh().catch(error);
api("health").then((health) => {
  $("#key-status").hidden = health.key_available;
}).catch(error);
if (!id) $("#settings").open = true;
setInterval(() => {
  if (!pending && !refreshes) refresh().catch(error);
}, 1800);
