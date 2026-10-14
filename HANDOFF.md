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
into `generation: {brief, max_modules}` when created. Its foundation has no
Stories and is read-only. A GET of `https://institutions.local/modules/NAME`
constructs a missing module through the ordinary director/generator/publication
pipeline. NAME is a previously unspecified capability, not an enumerated Rust
handler. The generator supplies Onto source; the trusted interpreter compiles
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
modeling or autonomous actor competence. It allows three inference calls with a
$0.50 ceiling. This is a paid command; use new output/database filenames and
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
build digest. A provider or assertion failure stops the trial without retry.
A killed process can leave an incomplete report; inspect its database before
any further calls. Use `frontier replay RUN_ID` against that same database to
inspect persisted evidence without inference.

Live attempts on October 14 have not generated a capability yet. The first
uncovered a proof-runner endpoint-resolution bug (HTTP 404), now fixed; the
second was refused by OpenRouter HTTP 402 because the key could afford only
3,961 of the requested 4,096 output tokens. Each retains a $0.10 uncertain
reservation. Combined charges/reservations, including earlier audits, are
$0.692871525 against the original $2 authorization. No automatic retries or
budget increases were made. Read-only provider diagnostics confirmed that the key limit is not exhausted,
but the account associated with the configured key reports no funded credits.
Await provider funding/workspace resolution before claiming live generation or
live latency passes.

The immutable foundation/module boundary is an initial execution profile, not
a change to Frontier's general purpose. The later company transformation and
long-horizon authority experiments should exercise this machinery, rather than
requiring every department and capability to be implemented in advance.

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
