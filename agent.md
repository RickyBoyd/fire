# Agent Notes: FIRE Simulator

## Project Context

This is a Rust Monte Carlo retirement simulator with a browser UI.

- Backend/API: `src/main.rs`
  - Serves static UI assets and `/api/simulate`
  - Parses/validates query params and builds model inputs
- Core engine: `src/core.rs`
  - Runs retirement/coast simulations
  - Applies growth, contributions, withdrawals, UK tax and CGT logic
  - Supports multiple withdrawal strategies (Guardrails, Guyton-Klinger, VPW, Floor+Upside, Bucket)
- Frontend: `web/index.html`, `web/app.js`, `web/styles.css`
  - Form-based input, results table/cards/chart, localStorage presets
  - Simulation runs only when user clicks **Run Simulation**

Important output behavior:

- Pot outputs are reported in real (inflation-adjusted) terms.
- Failed scenarios are represented with terminal pot values of `0`.

## Quality Requirements

All code changes should:

1. Be formatted with `cargo fmt`
2. Pass strict linting with `cargo clippy -- -W clippy::all`
3. Keep tests passing

## How To Run

Run the server:

```bash
cargo run -- serve 8080
```

Open:

- `http://127.0.0.1:8080/`

## Test Commands

Run all tests:

```bash
cargo test
```

Run property tests only:

```bash
cargo test prop_
```

Run snapshot (golden) tests only:

```bash
cargo test golden_snapshot_
```

Regenerate golden snapshots (when intentional output changes are made):

```bash
UPDATE_GOLDEN=1 cargo test golden_snapshot_ -- --nocapture
```

Run JS syntax check:

```bash
node --check web/app.js
```

## Recommended Pre-PR Checklist

```bash
cargo fmt
cargo clippy -- -W clippy::all
cargo test
node --check web/app.js
```

