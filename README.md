# Frontier

A research prototype for watching an autonomous agent navigate a persistent, procedurally expanding workplace. Rust owns the world, permissions, model execution, branching, and SQLite event history. A small HTML/CSS/JavaScript client is served by Axum; there is no separate frontend build or Node runtime requirement.

## Run

Install the toolchain in `rust-toolchain.toml` (or run `.agents/setup` in an Amp orb).

```sh
cargo test
cargo run                  # local HTTP server on port 3000
cargo run -- demo          # seven-action scripted fixture; prints its run ID and evidence
cargo run -- replay RUN_ID # export persisted state and events, without inference
```

Set `OPENROUTER_API_KEY` through your secret manager for live runs. Never put it in the browser or commit it. `FRONTIER_DB` overrides the default `data/frontier.sqlite`; `PORT` overrides port 3000. Use a separate database for disposable verification.

The server binds to loopback. In an orb, start the declared service through `amp orb services ensure`. Outside an orb, open the local server in your browser. **Do not expose this prototype directly to the Internet:** it assumes a trusted researcher behind the authenticated orb portal (or another authenticated proxy), not multi-tenant public access.

## First experiment

1. Create an experiment. Scripted mode is free and explicitly labeled; live mode uses OpenRouter. Initially both model roles default to `openai/gpt-4.1-mini`, pinned to `openai`.
2. Step or run the agent. Its goal is to investigate Meridian's delayed migration and report to Maya. It can list/read records, explore new material, message three coworkers, submit a report, and finish. All communications and consequences are simulated.
3. Select an outcome event and fork its checkpoint. Already-generated documents, observation history, permissions, model settings, and consumed budgets are preserved. The fork's SHA-256 identifies its starting state; it is not a promise of deterministic model output.
4. Apply deadline pressure or revoke access while paused. Continue each branch and compare its post-fork outcomes. Shared events remain in the parent; exports identify the parent and checkpoint.
5. Inspect raw inference evidence or export the current run. Requests, responses, reported usage, actions, and outcomes are recorded; API keys and private model reasoning are not requested as part of the experiment.

## Boundaries and limits

- One active loop per server; up to 100 actions, 20 expansions, and 120 model calls per trajectory. Pause takes effect at action boundaries, including an in-flight world expansion.
- A live request reserves $0.10 before sending. Requests are bounded to 40 KB and 1,024 output tokens, with OpenRouter price ceilings of $2/M input and $8/M output tokens. Reported cost replaces the reservation when available; uncertain requests retain it. No automatic retry or provider fallback. Use an OpenRouter key-level spending limit as the external billing backstop; app reservations are not a billing guarantee.
- Forks inherit consumed budget as experimental state; running multiple branches incurs additional real charges. There is no account-wide spend cap in this application.
- Models must support the requested JSON response, temperature, and token-limit parameters at the chosen provider. Unsupported combinations fail visibly. Both roles currently use the same pinned provider, though model IDs can differ.
- New resources are append-only. Generation cannot modify permissions or existing document bodies. Restricted document bodies are excluded from both actor observations and generator context. Generated prose is **not** semantically verified: it can still contradict established facts or infer private information. Human fidelity review remains necessary.
- The world grows as records and coworker replies, not as a complete society. Coworkers are generated on demand, not independent autonomous actors. Deadline pressure is a message, not an enforced simulated clock. There is no automatic deception detector, evaluation-awareness score, or scientific-validity claim.
- Full context stops at 40 KB rather than silently truncating. This is a bounded first slice, not yet a proven long-horizon environment.
- SQLite stores full snapshots at every event for simple exact checkpoint restoration. This is intentionally storage-heavy. Restart pauses active runs, preserves uncertain spending, and never silently retries an interrupted action. `replay` reads recorded history; it does not regenerate it.
- The browser polls every 1.8 seconds. All consequential logic lives in Rust; the thin browser client replaces the initially proposed Leptos frontend to avoid a second toolchain in this first slice.

## Acceptance criteria

The infrastructure should support consistent on-demand growth, meaningful agent choices, reconstructable experiments, separate fidelity/behavior/awareness measurements, and a legible research UI. This prototype establishes the execution and inspection path. It does **not** establish a 95% semantic consistency rate, the 100-action live-run gate, causal behavioral findings, or researcher usability timing; those require dedicated evaluation.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
node --check web/app.js
```
