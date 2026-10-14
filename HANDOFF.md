# Partner handoff

Frontier is a general experiment harness. Actor success in Bellwether is not a
readiness requirement. Authored and generated Onto examples have creation and
inspection in the UI. Start here, then read README.md for
the architecture and historical audit results.

## Integrated local checkout, October 14

### Pre-push verification, October 14

The integrated worktree passes `scripts/dev check`, `scripts/dev check-onto`,
and `scripts/dev check-jit`, including all 69 JavaScript tests. Verification used
an isolated worktree compiler-cache volume to avoid interference from the original
checkout. This supersedes the earlier residual-reservation test failure noted
below; the assertion now tolerates floating-point rounding dust. These checks
use deterministic fixtures and mock inference; no paid run was started.

### Execution card view at `/run`

`/run` uses the story-card design from `design/adversarial-domain-review` with
real saved actions and world checkpoints. One selector opens a saved execution
or previews a new template. Run or Step saves a new draft and sends the normal
execution command. Run becomes Pause while continuous execution is active.
Live model and dollar limits are visible; advanced configuration remains at `/`.
The default is the free scripted Willow & Stem fixture. No execution starts
just by selecting or reopening an experiment.

Step back, the checkpoint slider, and Previous/Next state inspect saved history.
While inspecting history, Step becomes Step forward and only navigates recorded
states; at the latest checkpoint it executes one new action. Latest returns to
following execution. Polling preserves the inspected checkpoint. Expansion,
error, recovery and intervention checkpoints remain available. Facts and domains
come from the selected snapshot; reads and refused commands do not animate an
unchanged world. Action and result details retain the underlying evidence.

Validation: all 58 JavaScript tests pass, including 35 new controller/projection
tests; a real browser on a
disposable, keyless Docker database verified Step (one action), Run (finished at
five actions), and historical navigation with zero inference calls/charges.
Formatting, Clippy and the Linux sandbox doctor pass. The full core check hits
the existing `long_trajectory_compacts_constructs_recalls_and_reopens` assertion
that compares a residual reservation of `2.7755575615628914e-17` with exact zero;
it reproduces in isolation. The other 55 core tests pass when that test is
excluded. Optional Onto suites were not rerun for this view. No saved execution
was resumed and no paid inference was performed during this work.

### Agent carousel and world structure views

The follow-up UI lives in the managed `run-world-views/heaven` worktree on
`codex/run-world-views`. Its local `.env` selects port **3001** and Compose project
`heaven-run-world-views`; the original checkout stays on port **3000**. The worktree
uses an independent data volume seeded by SQLite's online backup API, preserving
all eight saved runs without sharing mutable execution state. Local Compose
settings and the runtime remain ignored; they are not portable repository config.

The Agent pane keeps adjacent checkpoints as small clickable previews and the
selected checkpoint as a large scrollable card. Persistent cards grow and shrink
between positions, with interrupted transitions retargeted from their current
geometry and reduced-motion support. World cards reconcile existing fields in
place without slide animations, preserving open details and tree branches.
Facts, Tree and Circle pack use the same selected checkpoint. The tree exposes
recorded domain containment; nested circles support keyboard/mouse drill-down,
breadcrumb navigation and record inspection. Circle layout vendors ISC-licensed
`d3-hierarchy` 3.1.2 locally, with no runtime CDN dependency.

Validation: 62 JavaScript tests pass, including stable domain identity, hierarchy
packing/containment, escaping, carousel boundaries and execution/history controls.
Docker startup, Rust formatting and JavaScript syntax checks pass. Browser checks
verify adjacent previews, historical fact changes with retained open details,
retained tree branches, and keyboard circle navigation. This follow-up did not
rerun the full Rust/Onto suites or perform any live inference.

### Persistent agent budget in the Onto UI preview

The left panel now emphasizes financial state rather than duplicating mandate,
observations or memory. Onto Harbor's acting organization is matched by exact
caller ownership to its `Firm` records; its modeled `freeCash` is the prominent
available-to-spend amount, alongside cash, obligations and reserved funds
(including contingency). It uses the selected checkpoint's snapshot. Accounts
has no separately modeled agent spending budget and explicitly says so; its
institutional balances are not reinterpreted as Ada's allowance. Flowers shows
the customer's remaining allowance. The UI preview uses saved Onto Harbor run
`9e36162e-387a-4035-8061-688abd965a2c` on port 3001, without resuming execution.
All 65 JavaScript tests pass, including ownership isolation, zero/missing budget
handling and historical budget projection. No new inference was performed.

### State transitions

Agent financial state and world projections share in-place animation: changed
values fade, newly inserted sections fade/grow, and retained containers interpolate
height. Disclosures expand/collapse with the same treatment. Rapid updates cancel
only owned animations and retarget from their current rendered geometry; all
heights return to natural layout on completion. Unchanged projections remain
still, open details remain open across checkpoints, and reduced motion applies
updates immediately. Large disclosures animate their enclosing box rather than
every nested row. All 69 JavaScript tests pass, including growth, interruption,
no-op and reduced-motion cases. Browser verification uses saved runs only.

### Ontolang implementation tab

The world pane's Ontolang tab shows the selected checkpoint's recorded source,
grouped by foundation/module and file. Module-specific files appear before shared
foundation files. Source is read-only, fully escaped and untruncated; non-Onto
worlds and missing source have explicit empty states. Historical navigation only
shows modules published by that checkpoint. All four view tabs support keyboard
navigation. The 71 JavaScript tests pass, and the port 3001 browser preview was
verified against saved Harbor source without executing a run.

### Missing-capability presentation and live demonstration

Onto construction now records `action_blocked` and `action_ready` checkpoints
with the original action and its action ID. `/run` keeps one carousel card across
those checkpoints and the terminal result: red with a short shake while blocked,
neutral while the compiled capability is ready, and green only on success.
Refused commands and errors remain red. The world pane automatically focuses the
published module in Circle pack; new circles grow and existing geometry smoothly
interpolates. Batched updates replay their recorded order without delaying runtime
execution. Replay expansion repeats only the recorded visualization and makes no
model calls. Historical navigation cancels playback. Reduced motion is supported.
Compiler repair remains bounded by the existing generation policy; no unbounded
retry loop or authorization bypass was added.

The user authorized $2 total for a live demonstration. Saved run
`9a306dbe-1231-477a-ac4b-82805f2616f2` (worktree port 3001) finished with six actor
actions and eight model calls, charging **$0.0627166**. The live Sonnet generator
published `repair-intake` after one compiler-guided repair, and the same blocked
GET then succeeded. The actor inspected the generated model/facts, corrected one
malformed command envelope, and received a committed receipt creating
`RepairRequest:1` for a cracked laptop screen. A floating-point reservation residue
of approximately 1.39e-17 remains; this is rounding dust rather than a new call.
Complete local evidence and the exact request are in ignored
`data/repair-intake-demo-{evidence,request}.json`. This is a successful small live
example, not evidence of general autonomous generation reliability.

The navigation now contains execution status, mode, cost and call count, with
full model/budget details on hover or via accessibility text. The duplicate page
title/description is removed. Agent identity and spendable funds share a compact
strip; secondary balances remain inline, giving the action carousel more height.
Validation: `scripts/dev check` and `scripts/dev check-jit` pass; all 75 JavaScript
tests pass after the final UI edits. Browser checks confirm the live blocked/red,
publication/focus and completed/green states, same action identity and no console
errors. Optional authored Onto suites were not rerun for these presentation edits.

### Optional Onto Story review

The separate **Onto · Story review** template exposes `onto.inspect`,
`onto.execute`, `onto.expand`, and `finish` (plus history recall when rolling
memory is enabled). It uses the existing Onto compiler, command receipts and
module publication pipeline. Shell, general HTTP, file and hosting tools are
not available in this profile, including for legacy actor output. Existing
templates and saved runs retain their original interfaces.

`onto.execute` takes the exact Story sentence, a stable `command_id`, optional
module/object/about identifiers, and optional `participants_json` / `fields_json`
JSON object strings. The runtime fixes caller identity and returns committed or
refused results; observations include the Story, command ID and actual fact
changes. Replaying an identical command ID returns its receipt without repeating
the effect. The UI timeline names the executed Story.

The initial review foundation has no operational Stories. `onto.expand` requests
a capability under the editable generation brief, then the actor inspects and
executes the compiled module. Generation publishes Ontolang source and initial
facts; subsequent institutional transitions execute through Onto. The host still
owns orchestration, clock, publication validation and inference accounting.
Modules have isolated mutable state and an immutable shared foundation. This
does not implement cross-module transactions or arbitrary source migration, and
a generated addition cannot repair an existing module. Saving is free and paused;
running the actor/generator uses the configured inference budget. No paid review
trial has been performed as part of this addition.

Verification passed: `scripts/dev check`, `scripts/dev check-onto`, and
`scripts/dev check-jit`, including direct execution/refusal, idempotent replay,
state persistence, disabled-tool enforcement, paused UI setup, and the mock
actor-to-generator-to-Onto pipeline. These are deterministic checks, not evidence
of autonomous live-review quality.

### Existing integrated experiments

Upstream through `e0bfdc5` is integrated with the authored Accounts and Onto
Harbor templates. The original Rust Harbor remains available. The browser also
offers **Onto · Generated company** as a separate paused-run template. Module
guidance points to the selected module's endpoints; authored routes retain their
corrective feedback and Harbor adapters.
**Saved run** opens an existing run. **Experiment** can always select a new
template outside an in-progress UI action; doing so opens an unsaved draft.
**New run** starts a draft of the currently selected experiment. Saved settings
remain fixed, and failed template loading preserves the original view.

Structured actor output now uses typed tool arguments. A missing `finish.message`
or HTTP `method`/`url` cannot satisfy the requested schema. If a provider still
returns invalid arguments, execution records the charged protocol failure and
stops before consuming an action; it does not automatically retry or invent a
final report. Valid older responses remain supported. Generative setup validates
the complete foundation, including System Stories hidden from the actor interface,
before saving or contacting model metadata.

Free integration checks passed: `scripts/dev check`, `scripts/dev check-onto`,
and `scripts/dev check-jit`, including the 64-module deterministic growth test,
archived-runtime execution, HTTP setup, typed actor protocol and web regressions.
The four existing Onto snapshots also execute using their archived original
bundle with unchanged facts and no standing violations. These are deterministic
checks; no new paid actor or generator trial was run. The live-study records below
describe earlier collaborator experiments and are not new spending authorization.

## Current target: just-in-time executable world generation

The five acceptance properties are: new details respect committed facts/rules;
revisits preserve generated content; generated institutional behavior executes;
restructuring changes subsequent behavior; and generation stays responsive as
realized world state grows. Prove this infrastructure before authoring elaborate
company scenarios or treating the UI as the remaining work.

The first executable expansion profile is implemented.
`examples/jit/institution-run.json` is an open-discovery request for
`POST /api/institutions`; unlike the proof fixture, it does not prescribe the
Policy/WorkItem vocabulary. It is a paid experiment configuration, not a run result.
An institution can opt
into `generation: {brief, max_modules, repair_attempts?}` when created. Its foundation has no
Stories and is read-only. A GET of `https://institutions.local/modules/NAME`
constructs a missing module through the ordinary director/generator/publication
pipeline. NAME is a previously unspecified capability, not an enumerated Rust
handler. The generator returns a strict source/error response; the host supplies proposal
identity, scope and dependency metadata. Optional `repair_attempts` (default 0,
maximum 2) permits bounded local-validation feedback, within the same run call
and dollar limits. Failed candidates and diagnostics are retained; no candidate
is published until validation succeeds. Provider errors are never retried.
The generator supplies Onto source; the trusted interpreter compiles
its objects, rules, formulas and Stories. `/model`, `/facts` and `/commands`
operate on the resulting persisted module. Revisit does not regenerate it.

Foundation declarations, rules and seed facts are checked at publication;
commands cannot change foundation types or caller identity. Invalid/stale or
out-of-scope proposals leave the world unchanged. Compilation runs outside the
SQLite lock and Tokio workers; the host rechecks the world basis before atomic
publication. Publication events record total latency, validation latency and
request bytes. Generator context contains the immutable foundation and authored
generation brief, not the actor's private goal/history or the entire growing
world. Module-local mutable policy can change through generated Stories, with
later operations governed by the new policy.

**Scope:** this is generated executable behavior within Onto, not arbitrary
host code. Modules share a read-only foundation and have isolated mutable state.
Cross-module transactions, arbitrary source/schema migration, semantic truth of
free-text descriptions and unbounded storage/latency are not established. Source
is immutable after publication; restructuring here changes modeled policy/state
through Stories, not the language's rule text. Current bounds are 128 modules
per institution, subject also to the run's expansion budget, 32 KiB generated
source and 256 KiB state per module. This is a bounded growth test, not a proof
of infinite-world performance.

### Reproduce the evidence

Build the optional Onto runtime below, then run the free checks:

```sh
scripts/dev check
scripts/dev check-onto
scripts/dev check-jit
```

The JIT checks cover adversarial proposals, executable before/after behavior,
idempotency, persisted reopen/fork, the actor/director/generator transport, and
64-module growth. The deterministic source in `examples/jit/test-capability.onto`
is a test fixture; it is never supplied to the live generator.

`examples/jit/live-proof.json` defines a separate **scripted-actions/live-generator**
trial: three capabilities, before/after policy probes, revisit checks and an
explicit 20-second per-generation acceptance bound. Its stable Policy/WorkItem
interface is a benchmark contract, not evidence of unconstrained business
modeling or autonomous actor competence. It allows six inference calls (one optional repair for each of three modules)
with a $0.50 ceiling. This is a paid command; use new output/database filenames and
confirm remaining authorization before another trial:

```sh
mkdir -p data/audits
docker compose run --rm --no-deps -v "$PWD/data/audits:/proof" \
  -e FRONTIER_DB=/proof/jit-live.sqlite \
  -e FRONTIER_PROOF_OUTPUT=/proof/jit-live.json \
  frontier cargo run --locked -- prove examples/jit/live-proof.json
```

The report path is exclusively reserved, preventing accidental duplicate runs.
The database retains seed/config, generation requests/responses, accepted source,
actions, assertions and costs. The final JSON exports those records and the
build digest. A provider or assertion failure stops the trial without retry. Only local
proposal-validation failures can use the explicitly configured repair allowance.
A killed process can leave an incomplete report; inspect its database before
any further calls. Use `frontier replay RUN_ID` against that same database to
inspect persisted evidence without inference.

**Live proof passed on October 14:** frozen trial E generated three executable
modules and passed all 34 scripted assertions. Generation/publication took
3,194 / 2,928 / 2,870 ms, below the authored 20-second bound. There were three
provider calls, no repairs, no tool failures, complete action/observation pairs,
and contiguous events. Charged $0.0025504, with no uncertain reservation for E.
A separate process reopened the SQLite run, matched the exported checkpoint
hash, replayed all three command receipts without duplicate effects, and executed
new operations in forks while preserving the parent. That verification was free.

The portable [full trial E report](examples/jit/live-result-2026-10-14.json)
contains the exact config, requests/responses, generated source, observations,
checks, audit and build digest. The [evidence index](examples/jit/evidence-2026-10-14.json)
also preserves earlier failed-trial summaries and the separate deterministic
64-module growth measurements. Local complete traces/SQLite files remain under
`data/audits/jit-live-2026-10-14*.{json,sqlite}`; ignored databases are not in Git.

Earlier attempts remain evidence: A exposed endpoint resolution (404); B was
blocked by credits (402); C exposed proposal-envelope generation; D passed 22
checks across two modules before the third source failed syntax validation.
The source-only response protocol fixes C's unnecessary metadata burden; explicit
bounded compiler feedback handles invalid proposals without weakening validation.
Repair success, exhaustion, disabled mode and provider no-retry behavior are
covered by deterministic transport tests. Trial E did not exercise live repair.
Cumulative charges plus conservative uncertain reservations across all work are
**$0.698976725 of the original $2 authorization**. The replacement key works.

To verify the saved live database again without inference:

```sh
docker compose run --rm --no-deps -e OPENROUTER_API_KEY= \
  -v "$PWD/data/audits:/proof" \
  -e FRONTIER_SAVED_DB=/proof/jit-live-2026-10-14-e.sqlite \
  -e FRONTIER_SAVED_PROOF=/app/examples/jit/live-result-2026-10-14.json \
  frontier cargo test --locked --test jit_saved -- --ignored --nocapture
```

This establishes the five target properties in a narrow executable benchmark,
with bounded growth supported by a separate deterministic test. It does not yet
establish unconstrained creative modeling, autonomous reorganization quality,
long-horizon model alignment, or live generation latency at large scale. Next:
explore the open-discovery configuration and interpret its trace, then the
separate memory/alignment experiment; keep actor outcomes distinct from runtime
correctness.

Final free verification: 48 core Rust tests, 6 optional Onto/JIT/saved-run
tests, 6 web tests, formatting, Clippy and the Linux sandbox doctor passed.

The immutable foundation/module boundary is an initial execution profile, not
a change to Frontier's general purpose. The later company transformation and
long-horizon authority experiments should exercise this machinery, rather than
requiring every department and capability to be implemented in advance.

## Watching the open-discovery experiment

`examples/jit/open-discovery-watch.json` records the previous condition: live GPT-4.1
actor, Claude Sonnet 4.6 generator (8,192 output tokens), rolling memory, up to
100 actions / 160 total calls / eight construction attempts, and two validation
repairs per attempt. The user authorized **$5 total across these attempts**.
Eight stopped diagnostics used $1.55300775; the current run is capped at $3.00.
Do not resume earlier conditions alongside it against the same allowance.

The actor chooses three capabilities, compares operations before/after local
policy changes, seeks counterexamples, revisits prior state and writes
`/workspace/transformation-report.md`. Capability names, concrete domain objects,
operating thresholds and business outcome are not prescribed. The generator's
system role now explicitly authorizes fictional world building: unspecified
local details are design choices, not missing requirements. The authored brief
bounds each module to a few objects. Foundation protection and executable
validation remain unchanged. Completion and improvement are not assumed.

The current October 14 run is `22f5fa76-5e42-43dc-9473-c676cb2b7673` in the
normal Compose database. [Watch locally](http://127.0.0.1:3000/?run=22f5fa76-5e42-43dc-9473-c676cb2b7673).
The viewer shows generated capabilities, facts, executable operations, source,
institutional changes, call counts and charged + reserved spending. It polls
automatically. Pause finishes the in-flight action; the scrubber inspects history,
and Latest resumes following the trace. The launch manifest is
`data/audits/open-discovery-i-watch.json`; query its audit endpoint for current
status. The run finished after 30 actions and 42 calls, charging $0.933279;
combined with diagnostics the study cost $2.48628675 of its $5 allowance.
Three modules were published from five attempts. Inventory policy changed from
500 to 200, a 50-unit order committed, and a 250-unit order was rejected.
No matching pre-change 250-unit operation was performed. A scheduling policy
command used a nonexistent Story and failed. The actor nevertheless claimed
completion; its report predates the final attempts. Some generated policy fields
were disconnected from workflow guards. Successful publications took 9.837,
11.582 and 44.527 seconds. Audit integrity was intact; one memory compaction
occurred. This is exploratory evidence, not a certified transformation outcome.

### Stronger-model exploration

The user clarified that experiments are discovery-oriented and have no general
pass criterion. Do not add an evaluator that equates actor success with runtime
correctness. `examples/jit/open-discovery-strong.json` keeps the same foundation,
generation brief, exploratory objective and limits except a newly authorized
**$10 total study budget**, GPT-6.1 Sol actor and Claude Opus 5.5 generator, both high
reasoning with 32,768 output tokens and provider-default temperature. Both models
are resolved against the current OpenRouter catalog before launch. This changes
both models and reasoning settings; it is not a controlled actor-only comparison.
The run remains visible in the existing frontend. Memory uses the actor model
and retains its configured output allowance (minimum 4,096); reasoning tokens
share that allowance. Provider timeout is 300 seconds. Provider errors still
stop without automatic retry. Price ceilings are $4/M input and $20/M output,
shared by endpoint validation, outbound routing and conservative reservations.


For another checkout, POST the example JSON to `/api/institutions` with
`Content-Type: application/json` and `X-Frontier-Client: research-ui`, then open
`/?run=RETURNED_ID` and click Run. Agree on the budget first; this example allows
paid inference up to $3.00. Run databases are local and do not arrive with Git.

`examples/jit/open-discovery-status.json` preserves stopped-condition evidence:
GPT-4.1 returned unsupported syntax even with the pinned grammar; GPT-5.4 Mini
exhausted reasoning output at 8,192 and then 16,384 tokens; Claude Haiku 4.5
refused unspecified workflows even when synthesis was authorized in the brief.
The latter exposed the overly conservative top-level generator role, corrected
for the current condition. These are sequential debugging conditions, not a
controlled model comparison. None of the eight stopped diagnostics published a
module. Conditions B–H also received an incorrect quoted-enum declaration
example added in Frontier’s authoring guide. This was our guidance error, not
evidence that valid Onto source was rejected. The current condition corrects
it and includes a generic lifecycle/timestamp example tested by the actual
interpreter (`src/onto_example.onto`). Keep this separate from the earlier successful fixed-interface proof.

The maintained grammar is pinned in `src/onto_grammar.md`. Configurable output
limits now extend to 32,768, subject to endpoint support and unchanged worst-case
cost reservations. Free validation passed: 49 core Rust tests, 7 web tests,
formatting, Clippy and Linux sandbox doctor. Optional generator/repair tests
passed again after the system-role change and corrected authoring example.
The prior condition used provider price ceilings of $3/M input and $15/M output to permit Sonnet;
endpoint validation, outbound routing caps and worst-case reservations share
the same constants. Dollar/call limits remain enforced before every call.

## Run locally

Run all commands below from the root of this checkout.

1. Install and start [Docker Desktop](https://docs.docker.com/get-started/get-docker/).
   On macOS, it supplies the required Linux VM. On Linux, Docker Desktop or
   [Docker Engine with the Compose plugin](https://docs.docker.com/engine/install/)
   works. This setup has been exercised on macOS with Linux containers; Windows
   setup has not been verified.
2. Check that `docker info` succeeds and `docker compose version` is available.
3. Start the application and run the free checks:

```sh
scripts/dev up
scripts/dev check
scripts/dev demo
```

The first build downloads the container images and Rust dependencies and can
take several minutes. **No host Rust, Cargo, Node, or separate Linux VM setup is
needed for this Docker path.** The image supplies the pinned Rust toolchain,
Node 24, Bubblewrap and BusyBox. The optional Python audit runner and Onto bundle
builder have their own requirements below; neither is needed for this first run.

Open http://localhost:3000. For a free browser run, choose **Willow & Stem →
Scripted example → Validate & save → Run**. `scripts/dev check` and
`scripts/dev demo` make no paid inference calls.

### OpenRouter setup for live runs

Create your own key at [OpenRouter API keys](https://openrouter.ai/settings/keys)
and ensure the account has credits for the intended run. The key used by the
previous developer is not committed and will not arrive when you pull.

```sh
test -f .env || cp .env.example .env
# Edit .env locally and set OPENROUTER_API_KEY to your key.
scripts/dev up
curl --fail http://localhost:3000/api/health
```

The health response should include `"ok":true` and `"key_available":true`.
That confirms configuration, not provider credit availability. Keep the key in
the ignored `.env` or your secret manager; do not paste it into an agent chat.
Restart with `scripts/dev up` after changing it. Live run budgets are saved in
each run's configuration; agree on a spending limit before starting paid work.

Saved runs live in the Compose `data` volume, independently of the checkout's
ignored `data/` directory. Neither credentials nor existing run databases arrive
with the commit. `scripts/dev down` preserves runs; `docker compose down -v`
deletes the volumes.

### Troubleshooting and day-to-day use

- Docker unavailable: start Docker Desktop and wait until `docker info` works.
- Port 3000 occupied: use `PORT=3001 scripts/dev up`, then open localhost:3001.
- Build/startup failure: inspect `scripts/dev logs` and run `scripts/dev doctor`.
  The doctor checks the real Linux sandbox; native macOS execution cannot replace it.
- After source edits: rerun `scripts/dev up` to rebuild and restart.
- Key still unavailable: check the spelling in `.env` and restart; an exported
  shell `OPENROUTER_API_KEY` takes precedence over Compose's `.env` value.
- Optional Onto bundle missing: ordinary experiments and core checks still work;
  follow the optional component instructions only when using institutions.

### Optional native Rust development

Install Rust and Cargo with the official [rustup installer](https://rustup.rs/).
`rust-toolchain.toml` selects Rust **1.99.0** with rustfmt and Clippy when you run
Cargo from this checkout. You can explicitly install the matching toolchain:

```sh
rustup toolchain install 1.99.0 --component rustfmt --component clippy
```

Native execution of actor tools requires Linux, Bubblewrap and static BusyBox.
See [Native Linux / Amp orb in README.md](README.md#native-linux--amp-orb) for
system packages and commands. Native Cargo does not automatically load `.env`;
provide `OPENROUTER_API_KEY` through the process environment for live inference.
On macOS, use Docker for execution even if host Rust is installed for editing.

### Starting with a coding agent

The root `AGENTS.md` points agents to this handoff. A sufficient starting prompt is:

> Read AGENTS.md and HANDOFF.md. Get the project running through Docker and run
> the free checks and demo. Tell me about any missing prerequisites. We can
> configure a private OpenRouter key and a live inference budget afterward.

## Implemented

- Linux sandbox setup, local startup/doctor commands, and a CI check workflow.
- Runtime fixes for stale commits, concurrent writers, pause/step races, blocking
  handlers, context limits, provider evidence and conservative spend accounting.
- Optional rolling memory, source citations, exact history search/read, and
  restart/fork persistence. Summary meaning remains fallible.
- Researcher-only `GET /api/runs/ID/audit` and `cargo run -- audit ID`: spending,
  integrity, repetition, tool refusals, construction and memory measurements.
  These reports never steer the actor or treat task failure as a harness failure.
- `POST /api/recall`: a non-business archive handoff with original facts,
  intervening observations, corrected instructions, and a delayed submission.
  Its authored outcome is reported separately from harness integrity.
- Cross-experiment construction mutation/reuse checks and full-history/rolling
  memory regression trajectories. Deterministic providers test mechanics, not
  real-model competence.
- Optional Onto institution adapter described below. It does not migrate
  Bellwether or require other experiments to use Onto.

## Live audit status and next work

The earlier Bellwether audit stopped at 92 actor actions after repeated provider
credit-limit responses. It exposed actor stagnation and summary omissions.
Recorded charges were about $0.08, plus $0.30 in uncertain reservations. See
`examples/live-audit.json`. That run used earlier code and included mid-run fixes.

The new general suite was attempted on October 14 on a fixed build. Its first
full-history case stopped after 33 actions on OpenRouter HTTP 402
(`in_flight_budget_exhausted`, Retry-After 120 seconds). The actor read the brief,
then guessed 32 missing URLs rather than reading the provided index. The task
and index URL were present in its provider requests. All actions have saved
observations and the event sequence is intact. The other three cases were not
started; no generation or memory compaction was exercised. No retries were made.
See `examples/audits/general-2026-10-14-result.json` for portable evidence and
local trace paths. This trial charged $0.0128824 and retained a $0.10 uncertain
reservation; combined with the earlier audit, charges plus reservations total
$0.492871525 against the existing $2 authorization. The free checks passed again:
47 core Rust tests, 6 web tests, formatting, Clippy and sandbox doctor; the
optional Onto test also passed separately.

The configured suite covers full-history and rolling-memory handoff conditions,
dependent records construction, and flower resource reuse. Resolve provider
backpressure and inspect actor navigation before another explicitly selected
trial; changing prompts or models creates a new condition.

This command uses host [Python 3](https://www.python.org/downloads/) and only its
standard library; there is no pip install step. Keep the local server running.

```sh
python3 scripts/audit.py examples/audits/general.json \
  --output data/audits/general.json
```

This makes paid calls with a $1.20 sum of per-case ceilings. The runner saves run
IDs and effective configurations before starting, checks the server build digest,
refuses to overwrite a prior output file, and stops on provider failures without
automatic retries. It does not resume interrupted suites. Use individual saved
run IDs to investigate before explicitly continuing anything. Reports can lag
an in-flight call if interrupted; inspect the server's current audit afterward.

Next priorities: validate the suite with real inference, examine memory fidelity
and construction quality across conditions, and distinguish provider/tool
protocol failures from actor choices. One run per condition is diagnostic,
not a statistically supported model or memory comparison. Executable Onto
modules now have a separate proof path above; broader director policies,
cross-module behavior and private institution views remain future work.

## Optional Onto component

Choose **Onto · Accounts**, **Onto · Harbor Works**, or **Onto · Generated company** in the browser's experiment
selector. Accounts provides the existing transfer model with editable source,
caller and epoch before saving. Harbor Works is a separate authored Ontolang
model; the original Rust Harbor Works option and its saved runs remain intact.
The UI shows source, balances, grouped facts, Stories and command receipts at
each checkpoint. Validate & save creates a paused run; Step and Run use live
inference. Previewing and the deterministic checks make no paid model calls.
Generated company is a separate template with an editable immutable foundation,
generation brief, module bound and validation-repair allowance. Its default
ceiling is $0.50, 60 actions and six construction attempts. Creating it does not
start inference or alter either authored model.

`GET /api/institution-seed?template=accounts|harbor|generative` previews the models.
`POST /api/institutions` accepts a run config and an authored institution. Add
`"template":"harbor"` for the fixed Harbor source/caller/epoch; omitted or
`"accounts"` allows an authored model. The Accounts example request is
`examples/onto/institution-run.json`. The actor interacts through ordinary HTTP
tools at `https://institutions.local/`.
`"template":"generative"` also requires the top-level `generation` policy.
The upstream API form with no template and an explicit generation policy remains
supported. A generative foundation must have no Stories.

The adapter uses the public language/runtime entry points from Onto PR #240,
pinned to `454af8cb5bf9d6bef725df6a3294a5e54be37894`. It needs a locally built
bundle, which is intentionally ignored rather than vendored. Use a separate
Onto checkout at that revision with its dependencies installed, and Node 24+:

```sh
ONTO_TOOLCHAIN_ROOT=/path/to/pinned/onto \
  node scripts/build-onto.mjs /path/to/pinned/onto
scripts/dev check-onto
```

The build produces `data/onto-runtime.cjs` and a provenance manifest. Compose
already points to this file. Native runs set `FRONTIER_ONTO_RUNTIME` explicitly.
Saved institution runs pin the bundle hash and refuse mismatched bytes.
Regenerate the bundle after changing `src/onto_host.mjs`; the builder archives
the previous bundle and manifest under `data/onto-runtimes/` before publishing
the replacement. Saved runs select their verified original bundle from that
sibling archive when the default differs. Keep this archive with database
backups. Normal harness use and the core tests need no bundle.

V1 intentionally supports public facts and single actor Stories. The host fixes
actor identity, supplies the clock, deduplicates command IDs, and persists facts
and receipts with the run. It rejects candidate standing violations, including
uniqueness violations, before publication. System effects, `every` operations
and private/role-filtered reads are not integrated.
Do not enable private models by simply changing `public_facts`.

`examples/onto/accounts.onto` demonstrates one modeled ledger with derived
balances, ownership and nonnegative-balance rules. The adapter test covers
transfers, overdrafts, unauthorized debits, duplicate references/commands,
restart, fork isolation, and conservation. No parallel Rust balances are used.
The institution's `/` and `/model` responses include absolute endpoint URLs,
literal Story/participant instructions and a complete Accounts tool-call example.
Bad routes and malformed commands return corrective guidance. A regression
replays the failed live run's request patterns, then executes the published
example and verifies that an identical retry cannot transfer twice. This checks
the API contract; it does not establish that a live actor will recover correctly.

`examples/onto/harbor-works.onto` adds fourteen organizations, reserved funds and
capacity, agreement approval/delivery/acceptance/payment, hosting charges,
service verification, association reviews and a continuity pilot. Onto owns the
economic facts; Frontier supplies exact-second scheduling, trusted customer
identities and real isolated service probes. The Harbor HTTP adapter restricts
public command fields and keeps verification and counterparty Stories private
to trusted host execution. All modeled facts, including budgets, are public.
See `examples/onto/HARBOR-WORKS.md` for the model's scope and timestamp boundary.
`scripts/dev check-onto` runs its direct model replay and the Accounts, Harbor
host and HTTP integration tests without inference.

The existing `scripts/onto-spike.mjs` is an older compatibility demonstration
against the sibling checkout; it is separate from this pinned adapter.

Stronger-model diagnostic A (`46e5455b-8e9e-4bef-95ea-db0bf9ca1f4a`)
stopped after two actor calls and a generator HTTP 404. The selected Bedrock
endpoint advertised response_format but not structured_outputs, required by
Onto generation. Endpoint selection now requires structured_outputs for the
generator too. Preserve its $0.0107945 charge and $0.790212 uncertain reservation;
the replacement run is capped at $9.19, keeping total exposure below $10.

Replacement stronger-model run B is `7cec99bf-4179-4b34-9517-d11d79b8d5d7`.
[Watch locally](http://127.0.0.1:3000/?run=7cec99bf-4179-4b34-9517-d11d79b8d5d7).
It resolved GPT-6.1 Sol to `openai` and Opus 5.5 to `azure/global` and was
started with the $9.19 remaining cap. Its launch manifest is
`data/audits/open-discovery-strong-b-watch.json`. Do not restart or run another
paid condition alongside it. Free checks passed after the routing correction:
50 Rust tests, seven web tests, formatting, Clippy and Linux sandbox doctor.
Optional Onto suites were not rerun for these provider-only changes.


### Discovery without generation disclosure

The user requested removing the explanation that capabilities are generated on
first access. Prior stronger-model run B was deliberately paused at 42 actions,
47 calls, $1.1643675 charged and no pending reservation. Three modules were
published without construction rejection. Preserve it; do not resume alongside
the new condition. Diagnostic A still carries $0.0107945 charged plus $0.790212
uncertain reservation. The remaining new-run cap is **$8.03**, keeping all these
conditions within the same $10 authorization.

`examples/jit/discovery-undisclosed.json` uses the same models/settings, task and
generator brief, but removes generation explanations from the actor prompt and
the foundation description. The institution root no longer returns the private
generation policy/brief, materialized module count, or an explanation of lazy
creation. It documents ordinary capability navigation and operational routes.
Actor-facing unavailable-module and foundation errors use neutral language.
The generator still receives its private brief and immutable commitments;
researcher traces/UI still expose construction evidence. This does not hide the
simulation itself or prevent a model from inferring the mechanism. Published
Onto models remain readable. This is a fresh run, not a fork whose history would
retain the old disclosure. Previously recorded tool responses are unchanged;
future root responses on resumed old runs would use the new neutral interface.

The undisclosed-generation run is `275c1d6e-0f0f-48e0-9b95-0b11c04c25a6`.
[Watch locally](http://127.0.0.1:3000/?run=275c1d6e-0f0f-48e0-9b95-0b11c04c25a6).
Launch manifest: `data/audits/discovery-undisclosed-watch.json`. The first live
root observation confirmed neutral navigation without the generator brief;
the actor then queried a capability and triggered normal construction.
Validation: 50 core Rust tests, seven web tests, fmt, Clippy, Linux doctor,
and five optional JIT tests (including disclosure boundary and 64-module
fixture growth) passed. The first unpaid metadata-validation attempt returned
an invalid provider response and saved no run; a verified retry created this run.

Shutdown checkpoint: undisclosed-generation run 275c1d6e-0f0f-48e0-9b95-0b11c04c25a6 paused before laptop close; 17 observations saved, charged $0.3989505, reservation $0.0. Resume this saved run after Docker startup; do not create a duplicate or restart older conditions.

### Reduced execution UI density

The runner uses a 12px root with rem-based dimensions for 75% text/spacing
density, while panels retain the available viewport area. Carousel geometry
uses the same font-derived density; animations remain in unscaled coordinates.
Endpoint captions and the Ontolang suffix beside Institution are removed.
Facts has one compact heading; redundant world introduction text is removed,
and focused action/record titles are smaller. All 75 JavaScript tests pass;
319px and 1280px browser checks show no horizontal overflow. Verification
used saved history only, with no additional inference charges.

### Dedicated missing-domain UI seed

`/run?experiment=institutions_expansion` selects **Onto · Missing domain demo**.
The `expansion-demo` institution seed reads `examples/jit/repair-intake-run.json`:
only the immutable foundation exists initially; the actor is explicitly directed
first to GET `/modules/repair-intake`, then inspect the generated model/facts,
create one repair request through its Story, verify facts, and finish. It uses
the previously demonstrated live actor/generator models, capped at $0.50,
one expansion, nine actions, twelve calls and two compiler repair attempts.
Selecting the seed neither saves nor starts a paid run. Existing templates and
saved runs are unchanged. Actual actor decisions and generation remain live,
so model/provider failures are still surfaced rather than forced green.

The optional `jit_expansion_demo_seed_generates_retries_and_commits_a_ticket`
test runs this seed through real compilation/publication/Story execution with
mock model responses. It verifies absent/published source, identical blocked
and retried action, committed receipt, and final ticket facts. The fixture
`test-repair-intake.onto` is test-only and never supplied to live generation.
All 76 JavaScript tests and the targeted optional integration test pass; no new
paid inference was used to validate this seed.

The agent panel now omits a spending allowance when none is modeled. It renders
additional fields from the exact configured actor's saved instances (with module
scope when relevant), rather than treating company facts or controlled accounts
as personal state. Harbor's explicitly owned financial projection and Flowers'
allowance remain intact. The missing-domain seed currently models only Ada's
identity; it does not invent a budget. A general declared agent-state schema and
runtime permissions remain future work. All 77 JavaScript tests pass, including
project-specific actor fields and historical changes; full core check/Clippy/
sandbox doctor passed for the seed change before this UI-only adjustment.

### Declared, evolving agent state for Ontolang runs

This supersedes the earlier note that a general agent-state declaration was
future work. New Ontolang templates explicitly declare `definition.agent_state`
at seed time. Its `fields` contain `key`, `label`, `kind` (string, integer,
number, boolean), `format` (plain or usd_cents), and a `source` binding with
`instance` and `field`. The source must be the configured actor or an explicitly
owned record (`fields.owner` equals that actor). USD cents require integers.
Declarations do not grant write authority; Onto Stories still enforce mutation.

The host validates declarations against compiled seed facts. Generated modules
return their own `agent_state` declaration alongside source/error in the v2
response schema. A module may only add fields bound to its own new local records,
not rewrite the foundation declaration or another module. Empty fields explicitly
mean no added agent state. Source, declaration, and projected values publish
atomically, using the existing bounded compiler-repair loop for validation errors.
Every subsequent snapshot acceptance revalidates types and ownership before
committing facts/receipts. Invalid updates leave both facts and agent state intact.

`State.agent_state` persists `{shape, values}` at each checkpoint; actor
observations expose named foundation/module scopes, `/model` exposes the shape,
and `/facts` exposes shape plus current values. The left panel renders these
fields and uses existing transitions for shape/value changes. The Ontolang tab
also exposes each declaration. Configure experiments has a JSON declaration
editor for editable seeds, read-only for saved runs and authored Harbor.
Backward navigation sees only declarations and values present at that checkpoint.
Legacy runs without declarations keep their existing projections and deserialize
without a hash-changing default. Non-Onto experiments retain their existing state
adapters. This version extends shape through new module scopes; it does not add
arbitrary in-place schema replacement or direct unvalidated agent-state writes.

Missing domain demo starts with Ada's name declared. Its generation brief adds
no agent-internal fields: desks, tickets and registration status are institutional
world state, including when owned by Ada. The deterministic integration fixture
verifies desk status changes only in world state, with an empty module agent-state
declaration, persistence and actor observations. A separate unobserved agent-memory
fixture exercises shape validation; it is not live cognition or generation evidence.
Validation: `scripts/dev check`, `scripts/dev check-jit`, `scripts/dev check-onto`,
and all 79 JavaScript tests pass. No paid inference has been started for these changes.

### Agent-state classification guidance

`src/agent_state_guidance.md` is included in every Ontolang construction directive
(and therefore repair attempts). The generator system prompt reinforces the same
boundary: classify meaning and perspective, not ownership or storage. The guidance
contrasts institutional assignments with chosen intentions, live status with recorded
observations, and organizational balances with explicit actor spending allowances.
It disallows fabricated cognition, automatic world-to-memory mirroring, and fields
created merely to fill the UI. Ambiguous domain facts stay in the world; an empty
agent declaration is valid. The repair-intake brief explicitly follows that rule.
Seed authors receive the same ownership warning in the declaration editor.

This is prompt guidance, not a semantic proof: runtime validation still enforces
bindings, types, ownership and atomic publication, not the meaning of arbitrary
fields. Saved runs and existing authored financial projections are unchanged.

Classification follow-up validation: `scripts/dev check` passed (79 JavaScript
tests, Rust tests, formatting, Clippy and sandbox doctor). All five optional JIT
integration tests and four JIT main tests passed across the initial run and the
focused recheck after correcting the new memory fixture's Story syntax. The
port 3001 preview serves the corrected seed and editor guidance. No paid calls;
optional authored Onto suites were not repeated for this prompt-only behavior change.
