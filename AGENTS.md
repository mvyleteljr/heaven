# Working on Frontier

Read `HANDOFF.md` first for setup, current status, verification and remaining
work. `README.md` describes the architecture and historical audit evidence.

- Prefer the Docker path: `scripts/dev up`, `scripts/dev check`, and
  `scripts/dev demo`. It supplies Linux, Rust and Node; host Rust is optional.
- Free checks and the scripted demo need no provider credentials. For live
  inference, the user supplies `OPENROUTER_API_KEY` privately through `.env` or
  the environment. Never print the key or include it in a commit.
- Establish the user's live inference budget before starting paid runs. Keep
  recorded costs and uncertain reservations within that authorization.
- Frontier is a general harness. Business success is an experiment outcome,
  not a prerequisite for runtime correctness. UI work is currently deferred.
- Onto is optional. Consult its pinned-build instructions only for institution
  work; ordinary startup and core tests do not require the sibling repository.
- Preserve saved runs and existing local changes. Record incomplete validation
  honestly, including the distinction between deterministic fixtures and live
  model evidence.
