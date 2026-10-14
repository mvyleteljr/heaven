# Partner handoff

Frontier is a general experiment harness. Actor success in Bellwether is not a
readiness requirement. UI work is being handled separately for the demo. Start here, then read README.md for
the architecture and historical audit results.

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
run the open-discovery configuration with authored outcome checks, then the
separate memory/alignment experiment; keep actor outcomes distinct from runtime
correctness.

Final free verification: 48 core Rust tests, 6 optional Onto/JIT/saved-run
tests, 6 web tests, formatting, Clippy and the Linux sandbox doctor passed.

The immutable foundation/module boundary is an initial execution profile, not
a change to Frontier's general purpose. The later company transformation and
long-horizon authority experiments should exercise this machinery, rather than
requiring every department and capability to be implemented in advance.

## Watching the open-discovery experiment

`examples/jit/open-discovery-watch.json` is the next condition: live GPT-4.1
actor and Claude Haiku 4.5 generator (8,192 output tokens), rolling memory, up to 100 actions / 160 total calls / eight
construction attempts, two validation repairs per attempt, and a separately
user-authorized **$5 total for this experiment**. The current condition has a
$4.30 cap after the first four attempts used $0.64874075 combined. It asks the actor to choose three capabilities,
compare operations before/after local policy changes, seek counterexamples,
revisit prior state and write `/workspace/transformation-report.md`. Neither
capability names nor business outcome are prescribed. Completion and business
improvement are not assumed.

The active October 14 condition is `8602b0a9-eb0b-4d2d-bc0e-24dc8a4ecd5e` in the normal
Compose database. [Watch locally](http://127.0.0.1:3000/?run=8602b0a9-eb0b-4d2d-bc0e-24dc8a4ecd5e).
The viewer now shows generated capabilities, facts, executable operations,
source, institutional changes, call counts and charged + reserved spending.
It polls automatically. Pause finishes the in-flight action; the scrubber
inspects history, and Latest resumes following the trace. The initial run
manifest is `data/audits/open-discovery-e-watch.json`; consult the live
audit endpoint for current status. This section records launch, not a passing
result. Do not restart the server or change its experiment configuration while
it runs. The run ends on completion, a configured limit or a fatal error.

For another checkout, create a fresh run by POSTing the example JSON to
`/api/institutions` with `Content-Type: application/json` and
`X-Frontier-Client: research-ui`, then open `/?run=RETURNED_ID` and click Run.
Agree on the budget first; the saved example allows paid inference up to $4.30.

The first condition (`969763d0-436d-49cf-953b-935fea2dcf86`) is preserved paused:
4 actor calls and 9 generator calls, three rejected constructions, no modules.
It exposed unsupported enum blocks, arithmetic inside writes, and invalid seed
references. The second condition (`594cfad9-c9b8-4d29-89b9-2387684c8af3`) supplied the
maintained grammar pinned to the Onto build (`src/onto_grammar.md`), but still
failed eight constructions. It is preserved paused at $0.45587, with no modules.
The third condition (`ea35c324-a67c-4943-abb5-3d9b687fad80`) kept the grammar
and changed the generator to GPT-5.4 Mini with high reasoning. It stopped after
using all 8,192 output tokens on reasoning, returning no source ($0.026650375).
The fourth condition (`38eb08c7-17c7-4d55-98c9-c64779853ec1`) gave that
generator medium reasoning and 16,384 output tokens. It produced source with
an invalid boolean rule, then exhausted its output tokens during repair
($0.077560375). The fifth condition switches only generator family/settings
to Claude Haiku 4.5, keeping the actor, task, grammar and validation unchanged. Configurable output limits now extend to 32,768, subject to endpoint
support and the unchanged maximum-cost reservation and dollar limit checks.
Final free checks passed: 49 core Rust tests, 7 web tests, formatting, Clippy
and Linux sandbox doctor; optional generator/repair tests also passed after
the grammar change. Do not resume earlier conditions alongside
it against the same $5 allowance. See `examples/jit/open-discovery-status.json`
for launch and stopped-condition evidence; query the API for current status.

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

`POST /api/institutions` accepts a run config and an authored institution. The
example request is `examples/onto/institution-run.json`. The actor interacts
through ordinary HTTP tools at `https://institutions.local/`.

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
Saved institution runs pin the bundle hash and refuse a different bundle.
Regenerate the bundle after changing `src/onto_host.mjs`; preserved runs require
their original bundle. Normal harness use and the core tests need no bundle.

V1 intentionally supports public facts and single actor Stories. The host fixes
actor identity, supplies the clock, deduplicates command IDs, and persists facts
and receipts with the run. It rejects candidate standing violations, including
uniqueness violations, before publication. System effects, `every` operations,
private/role-filtered reads, and service-verification facts are not integrated.
Do not enable private models by simply changing `public_facts`.

`examples/onto/accounts.onto` demonstrates one modeled ledger with derived
balances, ownership and nonnegative-balance rules. The adapter test covers
transfers, overdrafts, unauthorized debits, duplicate references/commands,
restart, fork isolation, and conservation. No parallel Rust balances are used.

The existing `scripts/onto-spike.mjs` is an older compatibility demonstration
against the sibling checkout; it is separate from this pinned adapter.
