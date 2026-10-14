# Partner handoff

Frontier is a general experiment harness. Actor success in Bellwether is not a
readiness requirement. Onto examples now have creation and inspection in the UI.
Start here, then read README.md for
the architecture and historical audit results.

## Run locally

Docker Desktop supplies the Linux VM on macOS. On Linux, use Docker Compose.

```sh
scripts/dev up
scripts/dev check
```

Open http://localhost:3000. A provider key is unnecessary for checks or the
scripted flower example. For live inference, put `OPENROUTER_API_KEY` in an
ignored `.env`, then run `scripts/dev up` again. Saved runs live in the Compose
data volume; credentials and that volume are not part of this commit.

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

Choose **Onto · Accounts** or **Onto · Harbor Works** in the browser's experiment
selector. Accounts provides the existing transfer model with editable source,
caller and epoch before saving. Harbor Works is a separate authored Ontolang
model; the original Rust Harbor Works option and its saved runs remain intact.
The UI shows source, balances, grouped facts, Stories and command receipts at
each checkpoint. Validate & save creates a paused run; Step and Run use live
inference. Previewing and the deterministic checks make no paid model calls.

`GET /api/institution-seed?template=accounts|harbor` previews the models.
`POST /api/institutions` accepts a run config and an authored institution. Add
`"template":"harbor"` for the fixed Harbor source/caller/epoch; omitted or
`"accounts"` allows an authored model. The Accounts example request is
`examples/onto/institution-run.json`. The actor interacts through ordinary HTTP
tools at `https://institutions.local/`.

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
