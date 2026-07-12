---
name: official-cli-tooling-first
description: "Ruben demands official CLIs/tools for every workflow step — never hand-roll what a tool provides (sqlx-cli for migrations, cargo tooling, etc.)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: f34b6e9d-07db-4e26-bbb0-53471ae7eb9f
---

Always use the official CLI or tool for any workflow step before doing it by hand: `sqlx migrate add --sequential` to create migration files (never hand-name them), cargo/rustup tooling, `gh` for GitHub, etc.

**Why:** Ruben was upset ("you are free wheeling it and this is so so so bad") when migration files were hand-named instead of created via sqlx-cli, even though the result was convention-compatible. He values the official path itself, not just an equivalent outcome — it guarantees conventions stay correct as tools evolve.

**How to apply:** Before creating any artifact a tool manages (migrations, scaffolds, configs, releases), check for and use the official CLI first; mention which CLI was used. Applies on top of the repo's existing "don't hand-roll what a crate provides" rule ([[codegen-pivot-and-crate-naming]]).
