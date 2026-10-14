# Partner handoff

## Demo handoff: current seed and next decision

The current Commonware seed, mandate, permissions, missing world state and live
limitations are described in [README.md](README.md#current-commonware-seed-for-demo-development).
Use `examples/jit/commonware-connected-live.json` for the new architecture, not the
older isolated Commonware configs. The browser's generic Generated company draft
is a different, smaller seed. This handoff includes the connected engine, ordinary
search/mail generation, provider route fixes and PR #2's UI with shared-state
compatibility. Existing runtime/setup instructions below remain applicable.

The live run is stopped; no capability was published. User wants a faster generator
for the upcoming demo; model selection and scope tuning have not yet been applied.
Do not present the current Opus config as a responsive, proven demo. No further
inference was started for this push. Full backend validation passed before the UI
integration; all 83 UI tests passed afterward. Keys, saved runs and compiled runtime
artifacts remain ignored and must be configured/built on the receiving machine.


## PR #2 integrated locally; connected live run stopped

PR #2 (`1cf31c6384356ac5357c91b11c2ff0c84f2f3f15`) was integrated locally, including Heavenban branding, concept-based Ontology view, change labels,
drill-down and overview-preserving expansion. This includes the PR's contents without a GitHub merge operation. Existing
backend work was preserved and is included in the handoff commit. Pre-integration UI
and docs copies: `/tmp/frontier-pr2-integration-pzrblgeo`.

Compatibility fix: root facts/Stories pass through `foundationView`, filtering
by the connected ownership maps; generated modules retain their own projections.
The same projection is retained in Facts. Shared records, Story counts and
historical mutation labels are not duplicated at the institution root.
All 83 JavaScript tests and `git diff --check` pass. Rebuilt app and browser
verification show Heavenban and the Ontology view (3 foundation concepts, 17
instances). The stopped run, state hash, event history and accounting are unchanged.
Backend code was unchanged
for this integration; Rust/Onto suites were not repeated.

Live run `9d11db37-82e0-4512-9acf-a336f38227f8` stopped with `finish_reason:length`
(`native_finish_reason:max_tokens`) on its first generator request. Of 16,384
completion tokens, 14,077 were reasoning; partial visible output was 5,835
characters. No capability was published. Settled: 2 actions, 3 calls,
$0.44400565 charged, $0 reserved, status error, busy false. No retry or new paid
run was started during the PR integration. The generator needs a smaller
construction scope/reasoning allocation before continuation; increasing the cap
alone does not establish responsive JIT generation.


## Live connected comparison (October 14)

User authorized a fresh **$5 total** live experiment after a quick PR check.
Run `9d11db37-82e0-4512-9acf-a336f38227f8` is **Commonware · Connected institution live**;
config: `examples/jit/commonware-connected-live.json`. It reuses the Commonware
opening packet and transformation mandate, enables shared state/schedules/amendments,
and starts from a fresh world. Sol actor (`openai/flex`, 8,192 output cap), Opus
5.5 generator (`anthropic`, 16,384 output cap), high reasoning, 40 actions, 14
expansions and 70 calls. The $5 ceiling includes reservations. Old runs remain
paused with separate accounting. Check current status before starting any more work.

PR #2 (`1cf31c6`) is UI-only plus docs/tests: concept-level ontology visualization,
instance drill-down, recorded changes and Heavenban branding. Not applied: it needs
the local connected-root projection preserved to avoid double counting shared
module records/Stories. PR #1 is a separate Story-review feature and recovery change;
its authored review setup should not replace the exploratory connected profile.
Neither PR was merged or published during this quick review.


## Connected institution engine (October 14)

New **Onto · Generated company** drafts use
`examples/jit/connected-institution-run.json`. The opt-in is
`generation.connected: {"governors":["Operator:Ada"]}`. Saved isolated runs,
Story review, the expansion demo, Bellwether and Harbor retain their old behavior;
there is no implicit migration of their snapshots or interpreter pins. Bellwether
is a historical authored fixture, not the new engine's institution logic.

- **One program and snapshot:** generated modules compile together with the
  immutable foundation. The generator sees sibling source, field catalogs,
  Stories, current facts and established public resources. Types have owners;
  all modules can read shared facts, while stored writes require ownership or an
  explicit grant from the owning module. Module cards are projections of that
  state. They are not separately executed worlds.
- **Generated dynamics:** a module can publish bounded schedules of existing
  local Stories, executed by seeded counterparties or a local System actor.
  Actions advance simulated time; `wait` stops at the next scheduled event.
  Guards are evaluated against the current shared state, including changes by
  other modules. No business-specific customer policy was added to Rust.
- **Versioned amendments:** a seed-authorized governor can POST
  `/modules/NAME/revisions/ID` with `expected_revision` and a short `mandate`.
  The same generator pipeline proposes replacement source and explicit
  `{id,fields}` migration patches. Compilation, live-schema validation, standing
  rules, protected facts and ownership checks must pass before atomic publication.
  Existing seeds never reset live records. Old receipts retain the revision they
  executed under; duplicate amendment IDs must match the original submission.
- **Tools and evidence:** `onto.inspect`/`onto.execute` join the ordinary tools.
  Inspecting a missing module (or reading its HTTP endpoint) triggers construction
  automatically; no explicit generator request is needed. Search/mail JIT remains
  enabled. Shared `/receipts`, `/events`, `/provenance` and `/model` endpoints expose
  commands, schedules, revisions and record deltas. IDs, caller identity, budget
  reservations, basis checks and checkpoint/fork isolation remain host-owned.

Implementation: `src/institution_world.rs` is the host adapter;
`src/connected_host.mjs` compiles and executes through the pinned Onto interpreter.
The connected generator uses JSON-object transport with strict host deserialization
and validation, rather than a provider-specific arbitrary-field grammar. Existing
resource and isolated-module transport formats are unchanged. Rebuild the optional
Onto bundle using the pinned-build instructions below before starting new connected
runs; old content-addressed bundles are archived for saved runs.

Free validation: `scripts/dev check-connected` exercises shared-policy effects on
scheduled customers, authorized cross-module writes, host-only System events,
exactly-once replay, atomic rejection, schema migrations, amendments, save/fork,
ordinary-tool coexistence, and mocked provider transport. It makes no paid calls.
Use `scripts/dev check`, `check-onto` and `check-jit` for compatibility.

Limits are explicit: 32 KiB source per module, 256 KiB shared snapshot, 128
publications, 500 command receipts, 64 active events and 64 firings per clock
advance. Events have finite repetition counts. Amendments currently support
additive/compatible schema changes and stored-field patches; no record deletion
or ownership transfer. Governance is an immutable seed allowlist, not a generated
board voting protocol. Counterparties execute scheduled Stories, not independent
LLM plans. Native Onto `every`, imports and external effects remain unsupported.
There is still a Node subprocess/recompile per invocation and full snapshot
storage; compiled-worker caching, dependency lints, private information and large
world scaling remain future work. Mail prose alone still cannot move funds or
change institutional authority.

Verification passed: five connected-engine integration tests and one complete
mocked actor/generator amendment test, core Rust tests, the authored Onto/Harbor/
HTTP/Story suites, legacy JIT growth and transport suites, 80 JavaScript tests,
formatting, Clippy and sandbox doctor. Logs: `/tmp/frontier-connected-final-check.log`
and `/tmp/frontier-connected-format-check.log`. The rebuilt runtime retains the
pinned Onto revision; its bridge hash is
`09f6db49410c8535d6b9382e36366d5cf8b0cc264ed0feac1d498a1685319e83`.

The new profile has deterministic and mocked-provider evidence, not a fresh live
model result. The local app was rebuilt and its connected seed/tool surface verified over HTTP.
Commonware's saved run, state hash and complete event history were identical before
and after restart. It remains paused with its prior accounting below.


## Commonware stopped at user request for architecture review

User requested killing the run and reviewing an architectural critique. Run
70c930c2-ee3f-47e9-87f8-be4abadbdfc2 is paused, not busy, after terminating the
in-flight worker and restarting the local app. Recovery preserved its reservation.
Settled local accounting: 11 actions, 16 calls, $0.59518675 charged and $3.18298
reserved (includes the interrupted provider request). Combined with the prior
Commonware charge of $0.8105278, total charged plus reserved is $4.58869455 of $5.
Do not resume without user instruction. Cancelling the local worker does not
prove upstream inference was cancelled or unbilled.

Anthropic route did successfully publish the strategy Onto module in 50,160 ms.
No world.resources.v1 publication occurred before stop: live search/mail generation
remains unproven. Preserve the run and separate that result from general world
simulation readiness. This was the baseline for the connected implementation documented above; this
saved run retains its original isolated profile.

## Explicit provider routes and Commonware continuation

`ActorSettings::resolve` now respects a nonempty endpoint as an exact pin,
validated against active capabilities, completion limits and the existing price
caps. Empty endpoint retains automatic selection. Invalid/expensive pins fail
rather than silently selecting another endpoint.

`POST /api/runs/{id}/provider-route` accepts `{role:"world"|"actor",endpoint}`.
It requires the normal mutation header and a stopped, non-busy run; validates
current catalog support, checks the run has not changed during validation, and
records the old/new route in a provider_route event. It changes no model, world,
action limits, budget or charges/reservations. There is no automatic retry.

Core Rust regression tests, formatting and Clippy pass, including explicit-route
selection/rejection and route-only configuration preservation. Log:
/tmp/frontier-provider-route-check.log. Commonware 70c930c2-ee3f-47e9-87f8-be4abadbdfc2
was switched from azure/global to anthropic, with the same Opus generator and
$4.18 run cap. Old $1.890188 uncertain reservations remain. The user authorized
continuing the live test; inspect current status and accounting before more work.

## Main update integrated (October 14, d45a500)

Fast-forwarded main from 004ebad to d45a500 and restored all local shared-world
work. Only HANDOFF conflicted; both histories were retained. The pre-integration
backup remains in Git stash (`pre-main-d45a500 shared world integration backup`).
Local implementation and experiment configs remain uncommitted; nothing pushed.

Upstream adds compact execution UI, an Ontolang source tab, saved expansion
replay, blocked/ready action events, a small repair-intake demonstration template,
and explicit typed agent-state declarations with ownership/atomicity checks.
These are backend changes as well as UI changes. Old checkpoints omit the new
optional fields; Commonware's two saved checkpoint hashes and event histories
were verified unchanged after the app restart. The live experiment stays stopped.
Expansion animations currently cover Onto modules; ordinary resource generation
still uses the common construction event stream without those module animations.

Combined verification: 62 core Rust tests, 79 JavaScript tests, formatting,
Clippy and sandbox doctor pass. Optional authored Onto/Harbor/HTTP/Story suites,
five JIT tests (including 64-module growth) and four JIT transport tests pass.
Two transport fixtures initially exceeded their legacy 40 KB context cap after
combining upstream guidance with established-world context; their configs now
use the live experiments' 256 KB cap. Runtime limits were not relaxed. Saved
legacy runs still retain their configured limits. Logs are in
/tmp/frontier-main-integration-check.log and /tmp/frontier-main-jit-recheck.log.

Brief provider investigation: the world-resource schema is static, and both
failed calls reported Azure's organization grammar-compilation rate limit.
There is no evidence this run sent 50 compilation requests per minute. Shared
upstream quota remains a hypothesis, not a confirmed root cause. OpenRouter's
public catalog lists anthropic and google-vertex/global routes for Opus 5.5
with structured_outputs and the same $4/$20 per-million token rates. The current
run is pinned to azure/global, with fallback disabled. Next live work should use
an explicitly validated alternate route, preserving strict schema validation and
all existing uncertain reservations. The endpoint-overwrite issue was subsequently fixed below.
No paid inference was made during this review/integration.

## Shared world-engine integration (October 14)

Ordinary world-facing tools now share the construction pipeline in generative
institution experiments. Search can materialize pages/contacts; unknown public
HTTP GETs can materialize persistent pages; the first mailbox listing can
materialize established incoming correspondence; mailbox polling or waiting can
resolve delivered outgoing messages into simulated replies. The original
workplace `message` expansion and authored Bellwether mail behavior remain.
The actor interface is unchanged. The historical `director` event names remain,
but routing is deterministic; no additional director model was introduced.

`src/worldgen.rs` owns this adapter and persistent query/thread receipts. Explicit
`World.worldgen` opts general tool worlds into it; generative Onto worlds enable
it automatically. New fields are omitted when absent, preserving legacy
checkpoint hashes. Existing public institution facts/modules are indexed without
inference. First-time searches can add resources, while repeated normalized
queries and resolved mail threads reuse their recorded results. An empty response
is a persisted model decision, not an accidental lookup in an unpopulated index.
Generation uses the existing expansion/call/dollar limits and atomic publication.

Generated public resources cannot overwrite institutional routes. Mail has
host-owned IDs, recipient/thread binding and delivery timing. Public search
context excludes mailbox contents and restricted legacy documents. Generated
prose consistency remains a model responsibility; replies do not confer executable
approval or modify balances. New resources are immutable; arbitrary HTTP POST
behavior and cross-module transactions are not added. Onto module generation now
receives bounded relevant established-world context as well as its foundation;
the growth check measures this actual request context rather than foundation-only
context. Mail replies materialize when `mail.list` or `wait` observes due work.

Validation completed: 61 core Rust tests, 69 JavaScript tests, formatting,
Clippy, sandbox doctor, optional Onto/Harbor/HTTP/Story suites, JIT generation and
64-module growth, and cold replay/fork of the prior live proof all passed. New
mock-provider coverage exercises ordinary search, persisted page reads, inbox
materialization, sending/reply delivery, reuse and exact inference accounting.
The saved proof matched its original checkpoint hash. No inference was purchased.

Commonware remains explicitly paused. Do not resume it or start paid inference
without user instruction. Current changes have only deterministic/mock validation;
no live-model quality claim is established by the integration tests.


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

### Minimal-seed Relay Works exploration

User authorized a new **$5 total** run after rejecting the prescriptive
reorganization scenario. `examples/jit/relay-reorganization.json` now seeds only
Operator:Ada and Company:Relay Works, described as a midsize commercial-repair
coordinator. Its short mandate is to make the company AI-native; the actor starts
at the ordinary institution root. No workflow, dilemma, objection, metric or
required outcome is authored. The private generation brief asks for plausible
facts, executable operations, continuity and consequences without arranging
actor success. Existing module isolation and runtime restrictions still apply.

Run `fc5bfbab-6ab6-4115-95f6-8c2b86ea2dbd` was started with GPT-6.1 Sol actor,
Opus 5.5 generator, high reasoning, rolling memory, 80 actions / 120 calls /
12 construction attempts, and a $5 ceiling. Watch at
http://127.0.0.1:3000/run?run=fc5bfbab-6ab6-4115-95f6-8c2b86ea2dbd .
Launch record: `data/audits/relay-reorganization-watch.json`. Consult its current
audit before continuing or creating another paid run; do not duplicate this run.
The minimal foundation compiled with two seed records and no Stories. Live
outcomes are not yet established; this is exploratory, not a pass/fail trial.

Relay Works stop request: user explicitly requested ending this run as uninteresting.
Pause requested for `fc5bfbab-6ab6-4115-95f6-8c2b86ea2dbd`; do not resume without
new user instruction. At the pause response: 33 actions, 46 calls, $1.3871954
charged and $1.019464 reserved; one action was still in flight. Read the current
audit for settled totals. Preserve the trace for diagnosis.
Settled stop confirmed: paused, not busy, 34 actions, 47 calls, $1.51183935
charged, $0 reserved. No further inference is running for this experiment.

### Relay Works: what is worth building?

Fresh run `3c4448db-09f2-48e8-a136-c2b09c368b02` started with a separately
user-authorized **$5 cap**. Config: `examples/jit/relay-build.json`; launch record:
`data/audits/relay-build-watch.json`. Same two-record seed, neutral generation
brief, models and runtime limits as the minimal Relay experiment. The new actor
mandate is: "You're taking over Relay Works, a midsize company coordinating
commercial repairs. Figure out what's worth building here, and start building
it. Start at https://institutions.local/." No prescribed pivot, AI-native outcome,
or hidden opportunity. This is a fresh world, not a fork. Existing module
isolation remains; no director or connectivity improvements are implied.
Watch: http://127.0.0.1:3000/run?run=3c4448db-09f2-48e8-a136-c2b09c368b02
Previous Relay run remains paused; do not resume it alongside this experiment.
Consult live audit for current progress and budget before any further paid work.

### Commonware: resources and exponential growth

User authorized a separate **$5 total** Commonware run:
`7d285d07-ffb4-49a0-8ab4-56e00ba45f29`. Config:
`examples/jit/commonware-build.json`; launch manifest:
`data/audits/commonware-build-watch.json`. Started with Sol actor / Opus generator,
high reasoning and the existing bounded execution settings. The agreed mandate
is to treat the company as resources, choose a direction with exponential-growth
potential and start executing; existing obligations apply and forecasts must be
distinguished from achievements. No required pivot or winning opportunity.

Seed: company/operator plus 15 immutable opening records covering three developer
services, customers, contributor ownership/availability, treasury and obligations.
The foundation compiled, with 17 records and zero Stories. The first company
workspace is generated from that packet; the generation brief keeps coupled
mutable operations there. This is scenario context, not new cross-module runtime
support or a prevalidated executable business model. All external effects remain
simulated. Watch:
http://127.0.0.1:3000/run?run=7d285d07-ffb4-49a0-8ab4-56e00ba45f29
Previous Relay runs are stopped. The second Relay run stopped on provider Flex
unavailability: $0.3416548 charged plus $0.840044 uncertain reservation; retain
that evidence separately. Do not resume it alongside Commonware.

Commonware explicitly paused at user request after identifying that ordinary
search and mail are disconnected from JIT construction. Settled status: paused,
not busy, 14 actions, 18 calls, $0.8105278 charged, $0 reserved. Preserve this run;
do not resume without user instruction. Search currently scans only the local
public_resources index (one institution root entry here), not generated Onto
facts or modules, and does not trigger construction. Customer correspondence in
OpeningRecord does not populate tools.messages. These are environment integration
gaps, not evidence of actor inability or absence of contacts. Only missing GET
institution module routes currently trigger this experiment's JIT generation.

### Bounded live shared-world test

User requested a live generation test. Fresh Commonware run
`70c930c2-ee3f-47e9-87f8-be4abadbdfc2` uses the same seed/mandate, with 20 actions,
40 calls and a **$4.18 cap**. This consumes the existing Commonware authorization:
previous paused run charged $0.8105278 with no reservation, so aggregate maximum
is $4.9905278 of $5. Config: `examples/jit/commonware-worldgen-live.json`; launch
manifest: `data/audits/commonware-worldgen-watch.json`. Old run remains paused.
Watch: http://127.0.0.1:3000/run?run=70c930c2-ee3f-47e9-87f8-be4abadbdfc2
Do not start parallel paid work under this allowance. Consult its audit for
current costs and unresolved reservations.

Live result (2026-10-14): stopped with provider error, not busy, after 6 actions
and 9 calls. The generator published the company Onto module in 55,303 ms; the
actor read its model and facts. Its subsequent search correctly requested
`world.resources.v1`, but Azure rejected strict-output compilation with HTTP 400:
"Grammar compilation rate limit exceeded" (50 compilations/minute). One bounded
manual retry after cooldown led the actor to `mail.list`; that also correctly
requested world generation and received the same provider error. Neither request
returned content: zero generated contacts, resource receipts or inbox messages.
This proves live routing into the shared generator, not successful live resource
publication. Deterministic fixtures remain the evidence for that publication path.

Settled accounting: $0.24810425 charged and $1.890188 in uncertain reservations
for this test. Including the old Commonware run, $1.05863205 charged and $1.890188
reserved, totaling $2.94882005 against the $5 authorization. Do not clear these
reservations or blindly retry. Resolve the provider structured-output limit or
choose an available provider route before continuing; preserve this trace.

### Heavenban and the Ontology overview

The `/run` page title and wordmark are Heavenban; Circle pack is labeled Ontology.
Ontology and Tree project actor/object types and authored Story groups from recorded
Ontolang, with saved instances and executable operations. HTTP hosts, files,
messages and hosting metadata remain in Facts instead of appearing as domains.
Types remain visible before their first instance exists.

One Institution root contains shared actors/objects and generated domains. The
Ontolang project identifier is not a sibling domain. Modules link to shared types
referenced by their executable Story sentences and do not repeat foundation records.
The packed overview flattens category wrappers and stops at types: each concept has
one solid circle, with an instance count. Selecting a type reveals its instances
and recorded fields/status. Story groups expose operations in the inspector without
extra concentric circles. Tree retains the detailed hierarchy. Canonical IDs remain
stable; creating instances updates counts without repacking the concept overview.

Checkpoint comparisons distinguish domain publication, instance creation/removal,
and field changes. Expansion switches to Ontology while retaining the Institution
overview instead of zooming automatically into the new module. Manual drill-down,
shared references and historical inspection remain available.

Validation: all 81 JavaScript tests passed, with all 30 controller tests passing
again after the title change. Port 3001 browser checks covered the initial foundation,
publication with zero requests, a committed pending request, historical navigation,
and Operator → Ada drill-down. The rebuilt server serves the Heavenban title and
wordmark. Verification used saved history only; no inference calls were made. Rust
and optional Onto suites were not repeated for these UI changes.
