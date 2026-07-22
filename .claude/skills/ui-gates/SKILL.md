---
name: ui-gates
description: >
  Runs the full admin-UI quality-gate battery for app/ehrbase-admin-ui:
  clippy on native AND wasm32 targets, nextest, leptosfmt + cargo fmt,
  and a cargo-leptos build. Use before committing any admin-UI change,
  when the user asks to "check the UI", or as the done-gate a ui-implementer
  task must pass.
allowed-tools: [Bash, Read, Grep, Glob]
---

# /ui-gates

Run every gate the admin console must pass (defined in
`.claude/rules/leptos-ui.md` §10). Stop and report on the first hard
failure; run the cheap gates first.

## Preconditions

- `app/ehrbase-admin-ui` must exist; if it doesn't, say so and stop (it is the
  shipped console crate; see tracker issue #152 for the open scope).
- Target-dir discipline from CLAUDE.md applies: shared `./target`, no
  ad-hoc `RUSTFLAGS`, no flag variation between runs.
- Tooling presence: `rustup target list --installed | grep wasm32` (install
  with `rustup target add wasm32-unknown-unknown` if missing);
  `cargo leptos --version`; `leptosfmt --version` (report if missing —
  install is `cargo install --locked cargo-leptos leptosfmt`, ask before
  installing).

## The battery (in order)

```bash
# 1. Format (fast, catches drift)
cargo fmt -p ehrbase-admin-ui --check
leptosfmt --check app/ehrbase-admin-ui/src

# 2. Clippy — BOTH compilation targets (the wasm pass catches
#    server-only deps leaking past the ssr feature gate)
cargo clippy -p ehrbase-admin-ui --all-targets
cargo clippy -p ehrbase-admin-ui --lib --target wasm32-unknown-unknown --no-default-features --features hydrate

# 3. Tests
cargo nextest run -p ehrbase-admin-ui

# 4. Full build (server bin + WASM + assets) — only when the change
#    touches the build surface (Cargo.toml, styles, assets, features);
#    otherwise report it as skipped-with-reason
cargo leptos build

# 5. E2E journeys (merge-gating in CI; local requires Docker) — Rust-native
#    thirtyfour/WebDriver over the composed stack (.claude/rules/leptos-ui.md §10)
bash scripts/ui-e2e.sh
```

Stage 5 locally: run when Docker is available (`docker info`); otherwise
report `SKIPPED(no docker)` — but state explicitly that CI's `ui-e2e` job
WILL run it and gates the merge, so a skip here is not a pass.

Adjust the exact feature flags to the crate's `Cargo.toml` (read it first —
the `ssr`/`hydrate` feature names are the convention, not a guess).

## Report

One line per gate: PASS / FAIL / SKIPPED(reason), with the failing output
excerpted verbatim on failure. Never mark a gate green you did not run.
A FAIL is never fixed by weakening the gate (removing a lint, deleting a
test, dropping the wasm pass) — fix the code.
