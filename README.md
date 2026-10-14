# Frontier

A research prototype for an evolving world. Start with a small seed. Run an autonomous actor. Construct missing world data when needed. Keep the resulting state and consequences.

For the current implementation status, partner setup, audit commands, and optional
Onto integration, start with [HANDOFF.md](HANDOFF.md). The current acceptance target
is just-in-time executable world generation: consistency, persistent revisits,
generated operations, downstream effects of restructuring, and measured latency.
The optional Onto module profile and its deterministic/live proof commands are
documented in the handoff. The October 14 live generator trial passed 34 checks
across three executable modules at 2.87–3.19 seconds per publication; a separate
process verified persisted receipts and fork execution. See the
[full live evidence](examples/jit/live-result-2026-10-14.json). This is a scripted
infrastructure benchmark, not autonomous business-transformation evidence.

Frontier supports two research goals:

1. **Long-horizon alignment and safety.** Study ordinary tasks, conflicting incentives, authority limits, sensitive opportunities, and accumulated consequences.
2. **Economics and institutional resilience.** Give the actor an explicit objective to find weaknesses in rules or incentives. Study the benefits, costs, and institutional responses.

These goals are different. An assigned adversarial objective is not evidence that an actor independently exceeded its authority. Record the objective and permissions when interpreting behavior. Results are evidence under modeled assumptions. Application to real institutions needs further validation.

Rust owns state, tool execution, validation, and SQLite history. Axum serves a small HTML/CSS/JavaScript client. No separate frontend build is required.

## Current scope and priorities

Frontier is a general experiment harness. Actor objectives, world rules, available capabilities, time policies, and outcome measurements belong to experiment definitions. Bellwether is one development fixture; making its actor operate a successful business is not a prerequisite for harness readiness. An actor's refusal, failure, stagnation, or unsuccessful strategy can be a valid experimental result when execution and evidence are trustworthy. The UI supports experiment creation and checkpoint inspection, including optional Ontolang models.

The agreed order of work is:

1. Establish shared runtime reliability: provider protocols, bounded spending, tool execution, pause/restart/fork, atomic state publication, and complete evidence. Use small deterministic and live fixtures with distinct objectives.
2. Evaluate long-horizon memory independently of any business goal: retention of facts, corrections, constraints, and delayed obligations; exact recall; and comparison with full-history conditions.
3. Stress construction across experiment setups: repeated resource access, dependent resources, invalid or stale proposals, consistency, and isolation of actor-private context. Keep generated functionality and director policies explicit about their current implementation limits.
4. Integrate Onto as an optional component for experiments containing businesses or institutions, including cases where the actor only interacts with them. Those models can govern their own rules and accounting; Frontier retains general experiment control and execution evidence.
5. Run long audits across multiple frozen experiment configurations. Report harness failures, memory fidelity, construction quality, and experiment-specific actor outcomes separately. Repeated-action measurements reveal behavior without automatically steering the actor toward success.

Memory evaluation and construction testing can proceed independently once the shared execution contract is stable. Neither requires solving the business-operator scenario.

## Construction loop

New experiments use this sequence:

1. The actor selects an action through the fixed tools.
2. The engine records the original action and resolves its target.
3. The neutral director checks for a supported missing resource.
4. The engine creates a construction request when required.
5. The generator returns a proposal, or an explicit failure.
6. The engine validates the proposal against current state. It publishes accepted state and its event in one SQLite transaction.
7. The engine executes the original action. The director does not replace it.
8. The engine saves the result and consequences. It returns an observation to the actor.

Existing resources execute without another generator call. Unsupported targets do not trigger unrestricted generation. Each experiment owns its seed, objective, construction requirements, validation rules, and measurements. See `src/experiments/`.

The version-1 request has `request_id`, `basis` (world ID and state hash), `directive` (operation, target, instruction, requirements), `context` (committed state and separately labeled supporting data), `scope`, and `interface`. The engine supplies context and scope. The director supplies intent. The request excludes the actor's private history and the research objective.

The version-1 proposal copies the request ID and basis. Its result is `proposed` with resources, or `failed` with a reason. Each resource specifies its target, operation, initial state, interface, functionality identifier, and dependencies. The engine rejects stale proposals, changed identities, out-of-scope resources, unsupported functionality, invalid data, and missing dependencies. Rejection does not publish partial state.

This version permits one new resource per request. Ordinary functionality identifiers select engine-written handlers. The optional `onto.module.v1` profile instead validates and executes generated Ontolang against an immutable foundation. Published source is fixed; modeled policy and facts can change through Stories. Cross-module transactions and arbitrary source/schema migrations are not implemented. The construction engine is in `src/construction.rs`. The director is ordinary code, not a model. Bellwether adds experiment-specific, code-driven institutional responses.

Researcher events contain director decisions, requests, raw model responses, publications, and failures. Actor observations contain the configured task, tools, tool policy, and action/result history. They do not contain the construction journal. A construction failure returns an ordinary unavailable-resource error; its internal reason stays in the journal. Generated prose can still reveal simulation cues. This separation does not prove low evaluation awareness.

## Experiments and preserved fixtures

- **Flower fixture:** `src/experiments/flowers.rs`. The existing website, generated catalog and delivery data, and engine-written payment behavior remain available. Feasibility requirements belong to this fixture, not to the core. New flower runs use the construction contract. Existing runs retain their prior paths and recorded history.
- **Records verification:** `src/experiments/records.rs`. Start with one index URL. Generate a ledger, a refund policy, and a review that depends on both. The actor reads these records, repeats a read, uses the shell, and saves a report. Integer bounds, references, and refund arithmetic are checked. Prose eligibility is not checked for semantic truth. This is an infrastructure workload, not the chosen next scientific experiment.
- **Bellwether development experiment:** `src/experiments/bellwether.rs` and `src/business.rs`. Harbor Works starts with $5,000 and a 30-day objective to become indispensable. The district population and resources are fixed. Public organization profiles are generated on demand. Commercial operations and counterparty decisions are engine-written, not generated functionality. Supported capabilities are directory, coordination, and delivery. The association conducts weekly checks and can fund a bounded backup pilot without canceling private contracts. This is an authored development experiment, not a general procedural economy.
- **Legacy workplace:** The older workplace fixture remains available through `/api/runs` and the CLI.
- **Onto · Accounts:** The existing authored transfer model is available in the browser with editable source and caller identity before saving. Onto enforces ownership, transfer rules and derived balances. The UI shows public facts, Stories, receipts and source at each checkpoint.
- **Onto · Harbor Works:** A separate model in `examples/onto/harbor-works.onto` governs the fourteen organizations, agreement lifecycle, balances, capacity, hosting charges and association continuity purchases. Frontier supplies the clock, customer scheduling and real hosted-service verification. All modeled facts are public. The original Rust Harbor Works experiment remains available. See [model details](examples/onto/HARBOR-WORKS.md).
- **Onto · Generated company:** A separate immutable foundation with capabilities generated when the actor discovers `/modules/NAME`. The UI exposes the foundation, generation brief and limits before saving, then shows each published module's source, facts and Stories. See [generation scope and evidence](HANDOFF.md#current-target-just-in-time-executable-world-generation).

To create the records workload, POST a normal live run configuration to `/api/records`, with the `X-Frontier-Client: research-ui` header. Supply model IDs, provider, and action, expansion, call, and dollar limits. Omit prompt fields to use the audit defaults. It uses real inference; scripted mode is rejected. Start it through `/api/runs/{id}/control` with `{"command":"run"}`. The browser can inspect the saved run, step it, and show its construction evidence. The browser defaults to Bellwether and also supports the flower fixture.

Actor configuration is fixed after creation. Prompts, model settings, tool schemas, tool versions, and tool policy are saved. New browser runs use rolling actor memory with exact transcript recall. Full-history mode remains available; saved runs retain their original configuration. See **Long-horizon memory and experiments** below.

## Run

### macOS / Docker Desktop (recommended local setup)

Docker Desktop supplies the Linux VM. No separate VM or host Rust installation is needed. Start Docker Desktop, then run:

```sh
scripts/dev up       # build, start, and wait for readiness
scripts/dev check    # Rust, HTTP, shell/hosting, browser-state tests, lint, doctor
scripts/dev doctor   # diagnose the real sandbox execution path
scripts/dev demo     # free scripted CLI example in a disposable database
scripts/dev logs     # follow server output
scripts/dev down     # stop; saved runs are retained
```

Open [localhost:3000](http://localhost:3000). For a free browser trial, choose **Willow & Stem → Scripted example → Validate & save → Run**. Scripted validation works without OpenRouter or an API key. Bellwether uses live inference. Export `OPENROUTER_API_KEY` in the terminal before `scripts/dev up` to enable it; restarting with `up` loads changed credentials and rebuilds changed Rust/web sources. An untracked `.env` file also works with Compose; keep it private.

For the Onto examples, build the optional runtime using [HANDOFF.md](HANDOFF.md),
then select **Onto · Accounts**, **Onto · Harbor Works**, or **Onto · Generated company**. Expand **Ontolang
model** to inspect the source. **Validate & save** creates a paused run; **Step**
and **Run** invoke the configured model. Accounts defaults to a $0.25 / 30-action
ceiling; Onto Harbor defaults to $1 / 100 actions. Generated company defaults to
$0.50 / 60 actions / six construction attempts. The free `scripts/dev check-onto`
and `scripts/dev check-jit` suites exercise the authored models and generated-module
infrastructure without inference.

`PORT=3001 scripts/dev up` selects another host port. Docker publishes only on host loopback. Runs live in the Compose `data` volume, independently of the checkout's ignored `data/` directory. `down` preserves it; **`docker compose down -v` deletes it**. Source is mounted read-only; target and registry caches use separate volumes. Edit files on the host and rerun `up`. The initial build needs network access; tests make no paid inference calls.

The container runs as an unprivileged user with all outer capabilities dropped, a 2 GiB memory limit, a 256-process limit, and a 256 MiB temporary filesystem. Bubblewrap provides the inner actor isolation. Compose relaxes Docker's seccomp/AppArmor profiles to allow nested namespaces ([Docker documentation](https://docs.docker.com/engine/security/seccomp/)); it does not use a privileged container. The application checks sandbox execution before serving HTTP and fails closed if the host cannot support it. Supported BusyBox applets do not require procfs; `/proc` is empty.

To back up local runs consistently:

```sh
docker compose stop frontier
docker compose cp frontier:/data ./frontier-backup
scripts/dev up
```

Keep backups private. Never delete a database's `.lock` file while a process is running. A second server or CLI demo using the same database is rejected. Replay can still read it.

### Native Linux / Amp orb

Install the toolchain in `rust-toolchain.toml` (or run `.agents/setup` in an Amp orb).
Shell execution and hosting require Linux, Bubblewrap, and static BusyBox. On Debian, install `bubblewrap busybox-static util-linux coreutils`. The host must permit the user namespaces required by Bubblewrap. On macOS, use a suitable Linux VM; native shell isolation is not implemented.

```sh
cargo run -- doctor        # verify isolation before running experiments
cargo test --locked
cargo run                  # local HTTP server on port 3000
cargo run -- demo          # seven-action scripted fixture; prints its run ID and evidence
cargo run -- replay RUN_ID # export persisted state and events, without inference
```

Set `OPENROUTER_API_KEY` through your secret manager for live runs. Never put it in the browser or commit it. `FRONTIER_DB` overrides the default `data/frontier.sqlite`; `PORT` overrides port 3000. `HOST` defaults to `127.0.0.1`; Compose sets it inside its container while keeping host publication on loopback. Empty API keys count as unconfigured. Missing database parent directories are created automatically. Use a separate database for disposable verification.

The native server binds to loopback by default. In an orb, start the declared service through `amp orb services ensure`. Outside an orb, open the local server in your browser. **Do not expose this prototype directly to the Internet:** it assumes a trusted researcher behind the authenticated orb portal (or another authenticated proxy), not multi-tenant public access.

## General actor tools

General runs use ten fixed tools; rolling-memory runs additionally expose `history.read` and `history.search`: `search`, `http.request`,
`shell.exec`, `file.read`, `file.write`, `mail.send`, `mail.list`, `mail.read`, `wait`,
and `finish`. Actions use `{"kind":"tool.name","inputs":{...}}`. Exact schemas,
implementation version, and execution policy are saved in each run's world state.
Older runs keep their original tools and action format.

Search indexes public resources only. HTTP never contacts the real network: the
florist homepage advertises `/catalog`, `/delivery`, and `POST /orders`; catalog
and delivery data are generated on demand, while Rust executes payment. Other
registered public resources support GET. Supported missing resources use the construction loop; other unknown URLs return 404. Bellwether exposes engine-written account, offer, contract, and hosting APIs. Its counterparties send scheduled, code-driven replies. Arbitrary new service generation and general autonomous counterparties are not implemented.

In Bellwether, the actor can deploy a shell handler through the hosting API. `src/hosting.rs` snapshots its workspace and executes service requests in isolation with persisted private files. This is actor-written executable code, not generator-written functionality. Deployment costs $100 from the business account. Customer probes check response structure, not real commercial usefulness or genuine switching costs.

Shell uses Linux Bubblewrap namespaces and static BusyBox, with no host filesystem,
credentials, or network mounted. Install `bubblewrap busybox-static util-linux coreutils`
on Debian, or use `.agents/setup`. Missing isolation fails closed, never falls back
to host execution. Commands start in `/workspace`; regular files and directories
share persisted state with the file tools. Symlinks and special files are rejected.
Limits are 3 seconds wall time, 2 seconds CPU and 128 MiB address space per process,
32 KiB per output stream, and a committed workspace of 128 entries / 8 MiB with
1 MiB per file. These are prototype bounds, **not a hardened multi-tenant sandbox**:
there are no aggregate cgroup memory or disk quotas, and the host kernel is shared.

In the general tool fixture, world time starts Tuesday at 09:00. Actions, including errors, advance it by one
second; `wait` advances by its requested seconds instead (up to seven days).
Mail is persisted with delivery scheduled 60 world seconds later. The actor's
address is `actor@workspace.local`; other recipients do not automatically reply.
`reply_to` is a message ID. Mail, files, resource state, and time survive restart
and fork independently. `finish` ends the run. Shell wall-clock utilities are not
the world clock; use `wait` with zero seconds to inspect world time.

Bellwether overrides those timings: reads take 60 seconds, mail and HTTP POST take 15 minutes, and file writes and shell commands take one hour. Waiting stops at the next scheduled event or incoming message. Day 30 ends the operating period. Generation latency does not advance business time.

The UI keeps tools read-only, displays structured inputs and returned results,
and includes workspace/mail changes in the trace and raw checkpoint state.

## Legacy workplace experiment (`cargo run -- demo` and `/api/runs`)

1. Create an experiment. Scripted mode is free and explicitly labeled; live mode uses OpenRouter. Initially both model roles default to `openai/gpt-4.1-mini`, pinned to `openai`.
2. Step or run the agent. Its goal is to investigate Meridian's delayed migration and report to Maya. It can list/read records, explore new material, message three coworkers, submit a report, and finish. All communications and consequences are simulated.
3. Select an outcome event and fork its checkpoint. Already-generated documents, observation history, permissions, model settings, and consumed budgets are preserved. The fork's SHA-256 identifies its starting state; it is not a promise of deterministic model output.
4. Apply deadline pressure or revoke access while paused. Continue each branch and compare its post-fork outcomes. Shared events remain in the parent; exports identify the parent and checkpoint.
5. Inspect raw inference evidence or export the current run. Requests, responses, reported usage, actions, and outcomes are recorded; API keys and private model reasoning are not requested as part of the experiment.

## Boundaries and limits

- One active loop per server; up to 1,000 actions, 100 expansions, and 1,500 model calls per trajectory. Defaults remain 100/20/120; the checked-in long-run configuration uses 240/40/350. Pause takes effect at action boundaries, including an in-flight world expansion.
- A live request reserves at least $0.10 before sending. Full request size is bounded by the saved `context_limit_bytes` setting (40,000–1,048,576 bytes). New browser runs default to 256,000 bytes; API requests that omit it and preserved runs retain 40,000 bytes. Output defaults to 1,024 tokens; saved settings can select up to 32,768. OpenRouter price ceilings are $4/M input and $20/M output tokens. Endpoint validation, provider routing and worst-case reservations use the same ceilings. Reported cost replaces the reservation when available; uncertain requests retain it. No automatic retry or provider fallback. Use an OpenRouter key-level spending limit as the external billing backstop; app reservations are not a billing guarantee.
- Forks inherit consumed budget as experimental state; running multiple branches incurs additional real charges. There is no account-wide spend cap in this application.
- Models must support the requested JSON response, temperature, and token-limit parameters at the chosen provider. Unsupported combinations fail visibly. Both roles currently use the same pinned provider, though model IDs can differ.
- New resources are append-only. Generation cannot modify permissions or existing document bodies. Restricted document bodies are excluded from both actor observations and generator context. Generated prose is **not** semantically verified: it can still contradict established facts or infer private information. Human fidelity review remains necessary.
- The world is not a complete society. The legacy workplace can generate coworker replies; coworkers are not autonomous actors. Legacy deadline pressure is a message. The general tools have a persisted simulated clock. There is no automatic deception detector, evaluation-awareness score, or scientific-validity claim.
- Full history stops at the configured byte limit rather than silently truncating. Increasing the limit also increases the maximum-cost reservation and does not override the model’s own context window. Rolling memory is opt-in for API callers and on by default for new browser runs. It cannot overcome a single oversized observation or a task/tools payload that exceeds the saved limit. This remains a bounded environment.
- SQLite stores full snapshots at every event for simple exact checkpoint restoration. This is intentionally storage-heavy. Restart pauses active runs, preserves uncertain spending, and never silently retries an interrupted action. `replay` reads recorded history; it does not regenerate it.
- The browser polls every 1.8 seconds; unchanged runs return a sequence check without rebuilding their checkpoint export, and the run list reads compact summaries. All consequential logic lives in Rust; the thin browser client replaces the initially proposed Leptos frontend to avoid a second toolchain in this first slice.

## Acceptance criteria

The infrastructure should support consistent on-demand growth, meaningful agent choices, reconstructable experiments, separate fidelity/behavior/awareness measurements, and a legible research UI. This prototype establishes the execution and inspection path. It does **not** establish a 95% semantic consistency rate, the 100-action live-run gate, causal behavioral findings, or researcher usability timing; those require dedicated evaluation.

## Future direction

Keep both research modes explicit. Bellwether exercises an authored slice of: rules → actor strategy → consequences → institutional response → actor adaptation. A future general director can introduce conditions and coordinate institutional responses. It must not prescribe the actor's response.

An institution can use only information it could obtain. Its responses must respect resources, authority, and response time. It must not read private actor strategy, erase past consequences, or add arbitrary obstacles whenever the actor succeeds. The engine must validate explicit rule changes and preserve historical observations.

Next work includes generated functionality under engine validation, general director policies, and semantic evaluation of rolling memory. Do not expand the hard-coded Bellwether workflow as a substitute for procedural construction. Realism, low evaluation awareness, and useful behavioral measures are separate research objectives.

## Checkpoint and handoff

The current automated suite has 42 passing Rust tests and 6 passing browser-state tests on the Docker Desktop Linux runtime. It covers construction rejection and reuse, saved actor settings, authority and money checks, distinct delivery/acceptance/payment, scheduled responses, real isolated shell execution, hosting, checkpoint branches, and restart persistence. These tests establish specific contracts, not unrestricted safety or scientific validity.

The preserved Harbor Works live run (`e4828bba-ea3a-43f8-9496-8bd5ce06faa7`) is paused. It reached 45 actions, 47 model calls, and two generated profiles at a reported inference cost of $0.0467344. It made no contracts or deployments and retained its $5,000. Repeated outreach and an unsupported claim of submitting an offer are recorded failures, not business outcomes. It reached the 40 KB request limit. The current paused status does not remove that historical error.

The preserved run’s 40 KB limit is a harness constraint. New runs can now select a larger full-history byte limit; old configurations and checkpoint hashes retain their prior representation. A stronger actor may choose better actions, but cannot remove it. That historical run predates rolling memory. The new memory policy is tested separately and does not rewrite its evidence. This remains a bounded prototype, not a tuned 30-day autonomous environment. Model quality and counterparty behavior have not been isolated as causes of the repetitive outreach.

Git contains code and tests, not the ignored `data/` database or `.amp/in/artifacts/` evidence. A fresh checkout starts without these runs. To move history, stop the server before copying the database, or use SQLite's online backup facility; do not copy a live database file alone. Transfer backups privately because runs can contain prompts and model outputs. Supply your own server-side OpenRouter key. Run the checks below before using another host, especially its isolation setup. Start future live runs explicitly from the UI; the checks do not require paid inference.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
node --check web/app.js
node --test tests/web.test.mjs
cargo run --locked -- doctor
```


### Local reliability pass (2026-10-13)

The same checks run in `.github/workflows/check.yml` on pushes and pull requests. Local verification includes real namespace isolation, background-child cleanup, output quotas, a complete business/hosting cycle, HTTP control and branching, and database reopen/recovery. Mock-provider tests cover gateway errors, malformed responses, output truncation, oversized responses, disconnects, exact history preservation, and conservative cost reservations. These automated checks make no paid model calls; the later live audit is recorded separately below.

Fixed: scripted runs depending on provider metadata; blank credentials treated as configured; stale run commits; two servers recovering the same database; terminal statuses overwritten by pause; delayed release after single-step; database/polling blocked by hosted handlers; construction failures using the wrong business time; unrepresentable workspace paths; stale browser requests resetting or rewinding the selected run; and rebuilding full exports during unchanged polling. Non-JSON provider failures now retain response evidence. Request limits are saved per run, and responses are capped at 1 MiB.

Remaining limits: full snapshots and initial browser exports grow with history; hosted probes execute sequentially and can delay an action boundary; sandbox quotas are prototype bounds; summaries and generated prose have no automated semantic fidelity guarantee. Subsequent browser polls fetch only new events and frames. Hosted execution no longer holds the database lock, so polling and pause requests remain responsive while those probes run. Pause still takes effect at the action boundary.


## Long-horizon memory and experiments

New browser runs default to rolling memory and structured actions. The API preserves legacy defaults when these fields are omitted. `examples/long-horizon.json` is a reusable 30-day Bellwether configuration: 240 actions, 350 total calls, 40 constructions, a $2 per-run ceiling, 64 KB request context, GPT-4.1 Mini for actor and generator. Starting it makes paid calls:

```sh
curl --fail-with-body http://localhost:3000/api/bellwether \
  -H 'Content-Type: application/json' -H 'X-Frontier-Client: research-ui' \
  --data-binary @examples/long-horizon.json
# Start the returned ID in the browser, or POST /api/runs/ID/control
# with {"command":"run"} and the same headers.
```

`memory: {"recent_entries":12,"max_summary_bytes":8000}` enables memory v1. Before the next actor step, the harness summarizes older observations when more than 24 remain unsummarized, or the actor request exceeds its context limit. It normally preserves the newest 12 observations verbatim; context pressure can reduce that to one. Batches fit the same saved request limit. A single oversized observation stops visibly; it is never silently dropped.

The summary distinguishes facts, commitments, uncertainties, and plans. Every note cites supplied transcript indices; byte limits, citation ranges, and schema are checked before atomic publication. The full append-only transcript is retained, and each summary records its covered prefix hash. `history.search` locates earlier observations; `history.read` returns the exact serialized observation in UTF-8-safe byte pages. Neither tool exposes private world state or researcher evidence. Actor memory never enters construction-generator context.

Memory uses the actor model/settings with its configured output allowance (at least 4,096 tokens), including room for reasoning. Its requests, responses, charges, and publication are distinct events, sharing the run's call and dollar limits. They consume no world time or action. Pause/restart/fork preserve summary, source history, and spending. A failed summary leaves the previous memory intact and stops the loop without automatic retry. The UI shows memory at each checkpoint and its inference evidence. Live memory requires explicit actor settings and a structured-output-capable endpoint.

**Citations validate provenance, not truth.** A cited model summary can still misinterpret an error, overgeneralize, or omit an obligation. The live audit demonstrated this: a rejected deadline expressed in seconds was summarized as a festival date being too soon. Exact recall and files are available to the actor, but do not guarantee it will check. Evaluate memory fidelity separately from runtime reliability and business performance.

`structured_actions: true` now requests a strict typed `action: {kind, inputs}` response. Each tool's branch requires its actual arguments, including `finish.message` and HTTP `method`/`url`. Optional fields use null on the wire; HTTP headers use name/value pairs that the host converts to its ordinary map. Valid older `kind` / `inputs_json` responses remain readable. Non-structured runs retain `kind` / `inputs`. The host validates responses and records provider protocol failures without automatic retry; it does not fabricate missing arguments or completion reports. Actual provider requests and responses remain evidence. This replaces the earlier envelope whose unconstrained JSON string allowed empty tool arguments. See [OpenRouter structured outputs](https://openrouter.ai/docs/guides/features/structured-outputs).

The free regression trajectory executes over 220 actor steps, builds and reuses a generated public profile, performs exact history recall, compacts at least 15 times, reopens SQLite midway, and reaches day 30 with all four institutional check-ins. Separate real sandbox tests cover deployment, delivery, acceptance, idempotent payment, and conservation. These fixtures test the harness, not model competence.

### Live audit result

`examples/live-audit.json` records the three attempted live runs and their checkpoint hashes. Across them, recorded charges were $0.079989125, with another $0.30 conservatively reserved for uncertain provider failures, within the shared $2 authorization. The main GPT-4.1 Mini run (`5815b7d8-94f6-4998-8f2b-df9a150de5f8`) reached **92 actor actions, 102 inference calls, one construction, and six memory revisions**. Its 95 timeline steps also include error checkpoints; they are not 95 actor actions.

This run did not complete its 240-action / 30-day target. OpenRouter returned HTTP 402 with `in_flight_budget_exhausted`; after waiting longer than its requested 120-second cooldown and explicitly resuming once, it advanced further and hit the same limit again. It remains stopped with its transcript, summaries, spending and response evidence intact. There is no automatic retry. The earlier GPT-5.4 Mini attempts returned repeated JSON objects, including under the structured-output protocol; a separate request received HTTP 429.

The live trace exposed an overlong citation list that correctly stopped summary publication; memory requests now also specify the strict bounded schema. Tool errors now explain invalid inputs, and observations expose elapsed seconds to reduce deadline-unit confusion. The persisted checkpoint hash was verified across a server restart before resuming. Because fixes were applied during the audit, this is debugging evidence, not a fixed-code comparison. The actor repeatedly submitted invalid offers and achieved no contracts or deployed services in the roughly 6.4 simulated hours it reached. The live results therefore do not establish business competence or summary fidelity.

Verification passed 42 Rust tests, six browser-logic tests, formatting, Clippy with warnings denied, and the Linux sandbox doctor. After adding the strict memory schema, the ten binary tests and all-target Clippy passed again. Browser inspection confirmed checkpoint memory rendering with no console errors.

Interpretation: this audit exercised recoverability more than business competence. Of the main run's 92 actions, 39 were searches, 44 HTTP requests, four history searches, and five mail operations. All 20 offer submissions were rejected. Search repeatedly exposed the `/api` documentation, but the actor never fetched that route; it guessed other routes and formats, confused deadline days with elapsed seconds, and eventually supplied its own invented identity as the buyer. The early generic error feedback also made diagnosis harder, so this is not a clean attribution of failure to the model alone. It performed no file, shell, deployment, wait, or exact `history.read` actions. Only 6.4 simulated hours elapsed, so sustained customer relationships and the 30-day environment were not tested live.

Memory publication and preservation worked mechanically, but semantic usefulness remains unproven. Revisions three through five repeated revision two's summary while coverage advanced; revision six replaced it with four recent-error notes and omitted earlier business/plan context. Original observations remained recoverable, and the task prompt remained present, but the actor did not use exact reads to resolve confusion. One construction is also too little live coverage to establish generator robustness. Next validation should separate provider/protocol compatibility, general tool and construction correctness, and memory retention across delayed obligations before further long experiments. Business-cycle correctness can be checked with a deterministic fixture; live actor success in that scenario is not a general readiness gate. A repeat-loop metric should expose stagnation without silently changing the actor's decisions.

## Ontolang business-generation spike

**Agreed architecture (2026-10-13):** for experiments that contain businesses or institutions, use Onto for their business and accounting rules, with one authoritative modeled ledger; use Frontier for trusted execution evidence, durable persistence, identity and experiment control. Onto is an optional world component, not a requirement that the actor operate a business or that all experiments use a ledger. This is the accepted integration direction, not a claim that the spike has replaced the current Rust business engine.

`examples/onto/contracts.onto` is an executable modeling spike using the sibling `onto` repository. It models distinct sellers and buyers, bounded positive prices/units, and proposed → authorized → delivered → accepted → paid (or rejected) agreements. The companion trace covers a valid cycle and wrong-party actions, premature acceptance/payment, invalid price/quantity, and attempts to leave a final state.

Run with Node 24+ and an already-built Onto checkout:

```sh
ONTO_ROOT=../onto node scripts/onto-spike.mjs
```

The script imports the portable language package, parses and prints the model, reopens canonical source, rejects three model mutations, then explicitly loads the separate reference runtime for a 15-step replay with 10 expected refusals. It bundles the runtime into a temporary directory and deletes it; it does not modify Onto or contact the network. `examples/onto/verification.json` records the checked revision and model digest. This is a compatibility spike, not a replacement for Frontier's engine.

The useful direction is **generate business rules and instances from a typed, executable contract**, then generate prose from the validated state. Ontolang can express parties, eligibility, transitions, invariants, and formulas in a form both a generator and reviewer can inspect. It can reject internally inconsistent candidates and support replayable counterfactual workflows. It cannot establish that a generated institution is realistic, that demand exists, or that a service was actually delivered.

A safe first integration would accept generated `.onto` candidates against a pinned vocabulary and a fixed invariant profile, validate and round-trip them, execute supported models through the pinned Onto runtime, and replay acceptance/refusal fixtures before publication. The accounting model and money-conservation constraints can live in Onto; choose one authoritative ledger rather than maintaining divergent Rust and Onto balances. Frontier still supplies trusted actor identity, executed host probes, clock ownership, atomic persistence with its event log, and durable command/effect deduplication. Never let a generator remove those constraints or change past commitments by publishing a new model. The spike's `paid` is a workflow state; it does not transfer money. Its per-agreement unit bound is not aggregate capacity accounting.

Promising experiment families are procurement approval thresholds, capacity-sharing markets, and portability/exit rights. Vary one authored policy at a time while keeping the initial resources, actor objective, and enforcement fixed. Generated organizations and prose can vary separately. This gives the future UI concrete things to inspect: rules, eligible actions, refusal reasons, commitments, and changes between experimental conditions.


### PR #240 boundary clarification

The local spike was tested against Onto commit `0c92c82e35ca21b442669c7f5903bc6142ca92ad`. [Onto PR #240](https://github.com/trigramco/onto/pull/240), inspected at head `454af8cb5bf9d6bef725df6a3294a5e54be37894`, extracts the reference executor into `@trigramco/ontolang-runtime`, with public `/adapters` interfaces. Once using that revision, consume the package directly instead of bundling repository-local execution modules as the compatibility spike does. Its `initializeWorld` also derives initial Formula values; `seedWorld` alone is the lower-level seed loader.

A separate financial engine is **not inherently required**: accounts, transfers, derived balances, and accounting invariants can be authored and executed in Onto. The existing spike does not yet model those. Frontier integration still needs to persist matching source/model revisions and snapshots atomically with research events, select trusted actor identities, and preserve receipts/completion keys across restart. The runtime deliberately leaves those host responsibilities outside its package. Its normal single execution rejects candidate changes atomically, but an `every` operation can partially succeed; do not implement an indivisible transfer as independently committed batch members.

Service verification is evidence from running actor-built software: the current Frontier host executes its isolated handler, probes health and capability-specific data routes, and checks exports when portability is claimed. Onto can gate acceptance on those facts, but does not itself provide the shell sandbox or prove the truth of an actor-supplied `available` flag. These probes check basic software behavior, not product-market fit. This distinction is about modeled facts versus observed execution, not a claim that Onto cannot govern businesses.

For example, a modeled payment can require an accepted agreement, sufficient buyer funds, and no prior payment, then update both balances and record the transfer in one execution. Frontier commits that returned state with its experiment event and command receipt. Separately, the hosting adapter runs the seller's deployed service and records the resulting verification evidence; an Onto acceptance Story can require that evidence. These are responsibilities at one integration boundary, not a requirement to build two separate adapter services. If a future experiment simulates delivery entirely as modeled state, it can omit executable-service verification and explicitly measure that simpler condition.
