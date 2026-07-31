---
name: product-rename-ferroehr
description: "The product is FerroEHR (issue #1353): the full rename was EXECUTED 2026-07-31 — crates/binary/env-prefix/base-path/images all carry the new name"
metadata:
  node_type: memory
  type: project
  originSessionId: dcbecbcf-230c-4415-b3e6-d95dc66e6703
  modified: 2026-07-31T12:27:18.188Z
---

The product is **FerroEHR** (ferro/ferrum → Rust; "EHR" kept for category recognition). Decision + full rename both landed 2026-07-31 (issue #1353): repo `github.com/rubentalstra/FerroEHR`, app crates `ferroehr`/`ferroehr-rest`/`ferroehr-server`/`ferroehr-admin-ui`, binary `ferroehr`, env prefix `FERROEHR_*`, REST base path `/ferroehr/rest/openehr/v1`, conformance SUT id `ferroehr`, Helm chart + OCI images `ferroehr*`. The generated `openehr-*` spec crates keep their names (spec-versioned, not brand).

**Why:** distinct identity from vitagroup's EHRbase (the Java prior art — references to THAT product deliberately keep the EHRbase name: `docs/conformance/ehrbase-java/`, `docker/sut-ehrbase-java.yml`, upstream fixture names, "EHRbase is prior art" prose). "openEHR" must NOT appear in the brand (Foundation trademark); prose says "an openEHR® CDR" with the attribution line (README §Acknowledgments). Never call the project by its pre-rename working name in public-facing text — it was never released publicly.

**How to apply:** brand assets live in `assets/brand/` (tokens.css = colour source of truth; "Fe element tile" logo, "Oxide & Iron" palette). Still open: local FOLDER rename `~/RustroverProjects/ehrbase-rs` → `ferroehr` (must happen with no live sessions; recreate the harness memory symlink after), formal EUIPO/USPTO trademark search, registering ferroehr.com/.org (`ferroehr.eu` is held; GitHub org `FerroEHR` parked).
