# Partner handoff

Frontier is a general experiment harness. Actor success in Bellwether is not a
readiness requirement. UI work is deferred. Start here, then read README.md for
the architecture and historical audit results.

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

The new general suite has **not been run live** in this handoff. Its configuration
and runner are committed; it covers full-history and rolling-memory handoff
conditions, dependent records construction, and flower resource reuse:

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
not a statistically supported model or memory comparison. Generated executable
functionality, general director policies, and private institution views remain
future work.

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
