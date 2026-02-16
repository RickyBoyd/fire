# FIRE Simulator

Monte Carlo retirement simulator for ISA, taxable investments, pension, and cash buffer with configurable withdrawal strategies and UK tax handling.

## Quick Start

```bash
cargo run -- serve 8080
```

Open:

- `http://127.0.0.1:8080/`

## Deploy to Render (Web Service)

This repo includes `render.yaml` for a Render Blueprint Web Service.

1. Push the repo to GitHub.
2. In Render, choose `New +` -> `Blueprint`.
3. Select the repo and apply the blueprint.

Render will build with:

- `cargo build --release`

And start with:

- `./target/release/fire serve $PORT`

The app reads `PORT` automatically when no explicit port argument is provided.

## Full Documentation

See:

- `docs/PROJECT_DOCUMENTATION.md`

That document covers architecture, simulation flow, formulas, tax logic, withdrawal strategies, API model, UI behavior, and extension guidance.
