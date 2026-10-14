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
let id = new URLSearchParams(globalThis.location?.search ?? "").get("run") || localStorage.getItem("flower-run"),
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
function rememberRun() {
  localStorage.setItem("flower-run", id || "");
  if (globalThis.history?.replaceState && globalThis.location?.href) {
    const url = new URL(globalThis.location.href);
    if (id) url.searchParams.set("run", id);
    else url.searchParams.delete("run");
    globalThis.history.replaceState(null, "", url);
  }
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
let institutionDefinition = null;
let agentStateDraft = "";
let generationPolicy = null;
let seedExperiment = null;
const institutionTemplate = (experiment = $("#experiment").value) =>
  ({ institutions: "accounts", harbor_onto: "harbor", harbor_review: "harbor-review", harbor_review_v2: "harbor-review-v2", institutions_generated: "generative", onto_review: "story-review" })[experiment] ?? null;
const experimentName = (experiment) =>
  ({ bellwether: "Harbor Works", institutions: "Onto · Accounts", institutions_generated: "Onto · Generated company", onto_review: "Onto · Story review", harbor_review: "Harbor Works · Story review", harbor_review_v2: "Harbor Works · Story review v2", harbor_onto: "Onto · Harbor Works", flowers: "Flowers for Friday" })[experiment] ?? "World trace";
const displayedExperiment = (run) => run.experiment === "institutions" &&
  (run.world.ontology?.generation || Object.keys(run.world.ontology?.modules ?? {}).length)
  ? "institutions_generated" : run.experiment ?? "flowers";
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
  // Experiment selection navigates to a new draft; it never edits a saved run.
  $("#experiment").disabled = pending || !!editingPrompt;
  $("#configure-new").disabled = !id || pending || !!editingPrompt;
  $("#run-navigation-note").textContent = id
    ? "Saved settings stay fixed. Choose another experiment or New run to configure a fresh run."
    : "Choose an experiment, configure it, then Validate & save. Saved run opens a previous run.";
  $("#mode option[value='true']").disabled =
    $("#experiment").value !== "flowers";
  capabilities();
  const onto = institutionTemplate();
  const runtimeUnavailable = !id && onto && seed?.runtime_available === false;
  $("#new").disabled = !!id || pending || !!editingPrompt || !!runtimeUnavailable;
  $("#institution-source").readOnly = !!id || pending || ["harbor", "harbor-review", "harbor-review-v2"].includes(onto);
  $("#institution-agent-state").readOnly = !!id || pending || ["harbor", "harbor-review", "harbor-review-v2"].includes(onto);
  $("#institution-file").disabled = pending;
  for (const field of ["institution-actor", "institution-epoch"])
    $("#" + field).disabled = !!id || pending || ["harbor", "harbor-review", "harbor-review-v2"].includes(onto);
  for (const field of ["generation-brief", "generation-max-modules", "generation-repairs"])
    $("#" + field).disabled = !!id || pending;
  $("#runs").disabled = pending || !!editingPrompt;
  document.querySelectorAll(".edit-prompt").forEach((button) => {
    button.hidden = !!id;
    button.disabled = pending || !!editingPrompt;
  });
  $("#settings-status").textContent = pending
    ? "Working…"
    : id
      ? "Inspecting saved settings · read-only"
      : runtimeUnavailable
        ? "Configure the Onto runtime to validate this model. Other experiments remain available."
        : onto
          ? `Validate & save checks the Ontolang model and creates a paused run. Live run ceiling: ${onto === "accounts" ? "$0.25 · 30 actions" : ["generative", "story-review"].includes(onto) ? "$0.50 · 60 actions · 6 expansions" : ["harbor-review", "harbor-review-v2"].includes(onto) ? "$1.00 · 60 actions · direct Stories" : "$1.00 · 100 actions"}.`
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
function resetModelSettings() {
  for (const [role, prefix] of [["actor", ""], ["generator", "generator-"]]) {
    const field = $("#" + role);
    const model = "openai/gpt-6.1-sol";
    if (!Array.from(field.options).some((o) => o.value === model))
      field.add(new Option(model, model));
    field.value = model;
    $("#" + prefix + "temperature").value = "";
    $("#" + prefix + "max-tokens").value = 4096;
    $("#" + prefix + "reasoning").value = "";
  }
}
function frames() {
  return (
    data?.frames || [
      { world: seed.world, event: { evidence: {}, kind: "seed" } },
    ]
  );
}
function modelEditor(definition) {
  $("#institution-model").hidden = !definition;
  if (!definition) return;
  const files = Object.keys(definition.source);
  const selector = $("#institution-file");
  const selectedFile = files.includes(selector.value) ? selector.value : files[0];
  if (Array.from(selector.options, (o) => o.value).join("\n") !== files.join("\n"))
    selector.replaceChildren(...files.map((file) => new Option(file, file)));
  selector.value = selectedFile;
  const source = definition.source[selectedFile] ?? "";
  if ($("#institution-source").value !== source)
    $("#institution-source").value = source;
  const shapeSource = id ? (definition.agent_state ? pretty(definition.agent_state) : "") : agentStateDraft;
  if ($("#institution-agent-state").value !== shapeSource) $("#institution-agent-state").value = shapeSource;
  $("#institution-actor").value = definition.actor;
  $("#institution-epoch").value = definition.epoch;
  $("#institution-status").textContent = id
    ? "Onto runtime · all model facts are public in this experiment."
    : seed.runtime_available === false
      ? `Onto runtime unavailable: ${seed.runtime_error ?? "Run scripts/build-onto.mjs and configure FRONTIER_ONTO_RUNTIME."}`
      : "Onto runtime ready · all model facts are public in this experiment.";
  const capabilities = !id ? seed.capabilities : null;
  $("#institution-capabilities").textContent = capabilities
    ? typeof capabilities === "string" ? capabilities : pretty(capabilities)
    : "The host fixes the caller identity and enforces the model's Stories and rules.";
  $("#institution-source-note").textContent = id
    ? "Saved model · read-only. Facts and receipts below follow the selected checkpoint."
    : ["harbor-review", "harbor-review-v2"].includes(institutionTemplate())
      ? "Authored Harbor rules + synthetic review fixtures · read-only. Direct Stories; customer and verifier identities remain unavailable."
    : institutionTemplate() === "harbor"
      ? "Authored Harbor Works template · read-only. Its counterparties and hosting behavior depend on this model."
      : ["generative", "story-review"].includes(institutionTemplate())
        ? "Edit the foundation before validating. It must contain no Stories and becomes read-only after saving. Generated capabilities can add their own facts and executable Stories."
      : "Edit source and caller before validating. The preview below uses the template's initial facts until you validate & save your edits. Money values in stored facts are integer cents.";
}
function generationEditor(policy) {
  $("#institution-generation").hidden = !policy;
  if (!policy) return;
  $("#generation-brief").value = policy.brief;
  $("#generation-max-modules").value = policy.max_modules;
  $("#generation-repairs").value = policy.repair_attempts ?? 0;
}
function generatedInstitutionWorld(institution, previous) {
  const modules = Object.entries(institution.modules ?? {});
  const facts = (state) => (state.snapshot?.instances ?? []).map((fact) =>
    `<details><summary>${esc(fact.id ?? fact.type)}</summary><pre>${esc(pretty(fact))}</pre></details>`).join("");
  return `<div class="node root"><strong>Company foundation</strong><span>${modules.length} capabilities · shared commitments are read-only</span>${facts(institution)}</div>` +
    modules.map(([name, module]) => {
      const before = previous?.modules?.[name];
      const receipts = Object.values(module.receipts ?? {});
      const accepted = receipts.filter((r) => r.result?.committed).length;
      return `<div class="node ${pretty(before) !== pretty(module) ? "changed" : ""}"><strong>${esc(name)}</strong><span>${module.snapshot?.instances?.length ?? 0} facts · ${accepted} accepted / ${receipts.length} recorded commands</span><details><summary>Current facts</summary>${facts(module)}</details><details><summary>Executable operations</summary><pre>${esc(pretty(module.stories))}</pre></details><details><summary>Command receipts</summary><pre>${esc(pretty(module.receipts ?? {}))}</pre></details><details><summary>Ontolang source</summary><pre>${esc(Object.entries(module.definition?.source ?? {}).map(([file, source]) => file + "\n" + source).join("\n\n"))}</pre></details></div>`;
    }).join("") + (modules.length ? "" : '<div class="node missing">Capabilities appear here as the actor discovers them.</div>');
}
function institutionWorld(ontology, elapsed) {
  const instances = ontology.snapshot?.instances ?? [];
  const groups = new Map();
  for (const instance of instances) {
    if (!groups.has(instance.type)) groups.set(instance.type, []);
    groups.get(instance.type).push(instance);
  }
  const valueText = (value) => typeof value === "string" ? value : pretty(value);
  const balances = instances.filter((instance) => Number.isFinite(instance.fields?.balance));
  const stories = Array.isArray(ontology.stories) ? ontology.stories : [];
  const receipts = Object.entries(ontology.receipts ?? {});
  return `<div class="institution-world"><div class="node root"><strong>Onto institution · day ${Math.floor((elapsed ?? 0) / 86400)}</strong><span>Caller: ${esc(ontology.definition.actor)} · ${instances.length} public records · ${receipts.length} command receipts</span></div>${balances.length ? `<div class="institution-balances">${balances.map((instance) => `<div class="node"><strong>${esc(instance.fields.name ?? instance.id)}</strong><span>Balance · ${money(instance.fields.balance)}</span></div>`).join("")}</div>` : ""}${Array.from(groups, ([type, records]) => `<details class="institution-facts" open><summary>${esc(type)} · ${records.length} records</summary>${records.map((instance) => `<div class="node"><strong>${esc(instance.id)}</strong><dl>${Object.entries(instance.fields ?? {}).map(([key, value]) => `<dt>${esc(key)}</dt><dd>${esc(valueText(value))}</dd>`).join("")}</dl></div>`).join("")}</details>`).join("")}<details open><summary>Model Stories · ${stories.length}</summary><p class="field-note">Each Story’s permissions still apply. Some Stories run only through trusted counterparties or service checks.</p>${stories.length ? `<ul>${stories.map((story) => `<li>${esc(story.story ?? story.id)}${story.participants?.length ? `<br><small>Participants: ${esc(story.participants.join(", "))}</small>` : ""}</li>`).join("")}</ul>` : "<p>No single-actor Stories are exposed.</p>"}</details><details open><summary>Command receipts · ${receipts.length}</summary>${receipts.length ? receipts.map(([key, receipt]) => `<details><summary>${esc(key)} · ${receipt.result?.committed === true ? "Committed" : receipt.result?.committed === false ? "Rejected" : "Recorded"}</summary><pre>${esc(pretty(receipt.result))}</pre></details>`).join("") : "<p>No commands yet.</p>"}</details></div>`;
}
function render() {
  $("#run-view-link").href = id ? `/run?run=${encodeURIComponent(id)}` : "/run";
  const fs = frames();
  if (follow) selected = fs.length - 1;
  selected = Math.min(selected, fs.length - 1);
  const frame = fs[selected],
    w = frame.world.flowers ?? {},
    prev = selected ? (fs[selected - 1].world.flowers ?? {}) : null,
    evidence = frame.event.evidence;
  $("h1").textContent =
    data?.run.name ??
    seed.name ?? experimentName($("#experiment").value);
  document.title = `${$("h1").textContent} · World trace`;
  const definition = id ? frame.world.ontology?.definition : institutionDefinition;
  const generationSettings = id ? frame.world.ontology?.generation : generationPolicy;
  modelEditor(definition);
  generationEditor(generationSettings);
  $("#experiment-description").textContent = data
    ? frame.world.ontology
      ? `${experimentName(displayedExperiment(data.run))} · Ontolang owns the institution's facts and rules.`
      : data.run.experiment === "bellwether" ? "Original Harbor Works · Rust business simulation." : ""
    : seed.description ?? (seed.world.business ? "Original Harbor Works · Rust business simulation." : "");
  $("#system").textContent = data?.initial.actor_instructions ?? prompts.system;
  $("#director-policy").textContent = definition
    ? ["harbor_review", "harbor_review_v2"].includes(data?.run.experiment ?? $("#experiment").value) ? "Onto · direct Harbor Story execution; synthetic fixtures; no shell, HTTP or generation"
      : (data?.run.experiment ?? $("#experiment").value) === "onto_review" ? "Onto · inspect, execute Stories and compile model additions; no shell or general HTTP"
      : generationSettings ? "Onto · generate executable capabilities as they are discovered; preserve the shared foundation"
      : institutionTemplate() === "harbor" ? "Onto · authored commercial rules, hosting & counterparties" : "Onto · authored account rules & public facts"
    : frame.world.business
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
    fs[0].world.ontology ??
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
  if (frame.world.ontology) {
    const institution = frame.world.ontology;
    const previous = selected ? fs[selected - 1].world.ontology : institution;
    $("#world").innerHTML = institution.generation || Object.keys(institution.modules ?? {}).length
      ? generatedInstitutionWorld(institution, previous)
      : institutionWorld(institution, frame.world.tools?.elapsed_seconds);
  } else if (definition) {
    $("#world").innerHTML = '<div class="node missing">Initial facts will be available when the Onto runtime validates this model.</div>';
  }
  $("#state").textContent = pretty(
    frame.world.business || frame.world.ontology
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
  if (selected && frame.world.ontology) {
    const before = fs[selected - 1].world.ontology?.modules ?? {};
    for (const [name, module] of Object.entries(frame.world.ontology.modules ?? {})) {
      if (!before[name]) $("#changes").innerHTML += `<p>Capability generated: ${esc(name)}</p>`;
      else if (pretty(before[name].snapshot) !== pretty(module.snapshot))
        $("#changes").innerHTML += `<p>${esc(name)}: institutional facts changed.</p>`;
      else if (pretty(before[name].receipts) !== pretty(module.receipts))
        $("#changes").innerHTML += `<p>${esc(name)}: command result recorded.</p>`;
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
  if (selected && frame.world.ontology) {
    const before = fs[selected - 1].world.ontology;
    const changes = [];
    if (pretty(before?.snapshot) !== pretty(frame.world.ontology.snapshot))
      changes.push("Institution facts changed.");
    if (pretty(before?.receipts) !== pretty(frame.world.ontology.receipts))
      changes.push("Institution command receipts changed.");
    if (changes.length) {
      if ($("#changes").innerHTML === "<p>No world state changes.</p>") $("#changes").innerHTML = "";
      $("#changes").innerHTML += changes.map((change) => `<p>${change}</p>`).join("");
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
          "construction_validation",
          "institution",
        ].includes(e.kind)),
  );
  $("#generation").hidden = generation.length === 0;
  $("#generation-data").textContent = pretty(generation.map((e) => e.evidence));
  const working = data?.run.busy || data?.run.status === "running";
  $("#status").textContent = data
    ? `${data.run.config.demo ? "Scripted" : "Live"} · ${working ? "working…" : data.run.status}`
    : "Not started";
  const latest = data?.events?.at(-1);
  const phase = latest?.kind === "inference" ? `Calling ${latest.evidence.role ?? "model"}`
    : latest?.kind === "construction_request" ? "Generating a capability"
    : latest?.kind === "construction_validation" ? "Validating / repairing generated source"
    : latest?.summary ?? "Waiting for first action";
  $("#live-progress").textContent = data
    ? `${data.run.actions} / ${data.run.config.action_limit} actions · ${data.run.calls ?? 0} calls · $${((data.run.charged ?? 0) + (data.run.reserved ?? 0)).toFixed(3)} spent + reserved / $${data.run.config.dollar_limit ?? 0} · ${working ? phase : data.run.status}`
    : "";
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
      rememberRun();
      await loadSeed("bellwether");
      if (version !== refreshVersion || id) return;
      if (!seed.config) resetModelSettings();
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
        $("#experiment").value = displayedExperiment(next.run);
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
  const experiment = $("#experiment").value;
  if (experiment !== seedExperiment)
    throw Error("Load this experiment's seed before saving.");
  const template = institutionTemplate(experiment);
  if (template) {
    if (agentStateDraft.trim()) {
      try { institutionDefinition.agent_state = JSON.parse(agentStateDraft); }
      catch { throw Error("Agent state declaration must be valid JSON."); }
      if (!institutionDefinition.agent_state || !Array.isArray(institutionDefinition.agent_state.fields))
        throw Error("Agent state declaration must contain a fields array.");
    } else if (institutionDefinition.agent_state) {
      throw Error('Declare agent state fields, or use {"fields":[]} for no fields.');
    }
  }
  const business = experiment === "bellwether" || template === "harbor";
  const generative = ["generative", "story-review"].includes(template);
  const harborReview = ["harbor-review", "harbor-review-v2"].includes(template);
  if (template && seed.runtime_available === false)
    throw Error("Configure the Onto runtime before validating this model.");
  if (generative) {
    for (const field of ["generation-brief", "generation-max-modules", "generation-repairs"])
      if (!$("#" + field).checkValidity())
        throw Error("Check the capability generation settings.");
  }
  const config = {
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
    demo: template ? false : $("#mode").value === "true",
    action_limit: business ? 100 : generative || harborReview ? 60 : template ? 30 : 20,
    expansion_limit: harborReview ? 0 : generative ? 6 : 10,
    call_limit: business ? 120 : generative || harborReview ? 90 : template ? 180 : 40,
    dollar_limit: template === "accounts" ? 0.25 : generative ? 0.5 : 1,
    context_limit_bytes: Number($("#context-limit").value),
    memory: $("#memory-mode").value === "rolling" ? {recent_entries:12,max_summary_bytes:8000} : null,
    structured_actions: $("#action-format").value === "structured",
  };
  const name = $("#run-name").value;
  const r = await api(template ? "institutions" : experiment,
    template ? { name, template, config, institution: institutionDefinition,
      ...(generative ? {generation: generationPolicy} : {}) } : { name, ...config });
  id = r.id;
  data = null;
  follow = true;
  rememberRun();
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
const newRunExperiment = () =>
  ["records", "recall"].includes($("#experiment").value) ? "bellwether" : $("#experiment").value;
async function startDraft(experiment) {
  const previousExperiment = data ? displayedExperiment(data.run) : seedExperiment;
  try {
    // Keep the current view intact until the replacement seed loads successfully.
    await loadSeed(experiment);
  } catch (e) {
    $("#experiment").value = previousExperiment;
    $("#runs").value = id || "";
    throw e;
  }
  if (id && !seed.config) resetModelSettings();
  id = null;
  data = null;
  $("#runs").value = "";
  $("#run-name").value = "";
  if (!seed.config) {
    $("#context-limit").value = 256000;
    $("#memory-mode").value = "rolling";
    $("#action-format").value = "structured";
  }
  follow = true;
  $("#settings").open = true;
  rememberRun();
  render();
}
$("#configure-new").onclick = () => act(() => startDraft(newRunExperiment()));
$("#runs").onchange = () =>
  act(async () => {
    const next = $("#runs").value;
    if (!next) return startDraft(newRunExperiment());
    id = next;
    data = null;
    follow = true;
    $("#settings").open = true;
    rememberRun();
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
  const template = institutionTemplate(experiment);
  let next;
  try {
    next = await api(
      template ? `institution-seed?template=${template}` : experiment === "bellwether" ? "bellwether-seed" : "flower-seed",
    );
  } catch (error) {
    if (seedExperiment) $("#experiment").value = seedExperiment;
    throw error;
  }
  $("#experiment").value = experiment;
  seedExperiment = experiment;
  seed = next;
  institutionDefinition = seed.institution ? structuredClone(seed.institution) : null;
  agentStateDraft = institutionDefinition?.agent_state ? pretty(institutionDefinition.agent_state) : "";
  generationPolicy = seed.generation ? structuredClone(seed.generation) : null;
  prompts = { system: seed.system, task: seed.world.goal };
  if (seed.config) showSettings(seed.config);
  if (experiment !== "flowers") $("#mode").value = "false";
}
$("#institution-agent-state").oninput = () => {
  if (!id && ["accounts", "generative", "story-review"].includes(institutionTemplate()))
    agentStateDraft = $("#institution-agent-state").value;
};
$("#institution-source").oninput = () => {
  if (!id && ["accounts", "generative", "story-review"].includes(institutionTemplate()) && institutionDefinition)
    institutionDefinition.source[$("#institution-file").value] = $("#institution-source").value;
};
$("#institution-file").onchange = () => {
  modelEditor(id ? frames()[selected].world.ontology?.definition : institutionDefinition);
};
for (const field of ["actor", "epoch"])
  $("#institution-" + field).oninput = () => {
    if (!id && ["accounts", "generative", "story-review"].includes(institutionTemplate()) && institutionDefinition)
      institutionDefinition[field] = $("#institution-" + field).value;
  };
for (const [field, property] of [["brief", "brief"], ["max-modules", "max_modules"], ["repairs", "repair_attempts"]])
  $("#generation-" + field).oninput = () => {
    if (!id && ["generative", "story-review"].includes(institutionTemplate()) && generationPolicy)
      generationPolicy[property] = field === "brief" ? $("#generation-" + field).value : Number($("#generation-" + field).value);
  };
$("#experiment").onchange = () =>
  act(() => startDraft($("#experiment").value));
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
