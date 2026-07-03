- Phase: **0G — spec-driven code generation (ADR-004)**. The openEHR spec crates are now GENERATED from the vendored BMM meta-model; hand-transcription is retired. Read `docs/ADRs/ADR-004-spec-driven-codegen.md` and the "Code generation" section of `CLAUDE.md` before touching any `openehr-*` spec crate.
- State: `openehr-base`, `openehr-rm`, and `openehr-am` (both `am14` + `am24`) are generated and compile clean; `openehr-foundation` was folded into `openehr-base`; cross-crate refs resolve to `openehr_base::prelude::*`. Regenerate with `cargo run -p openehr-codegen -- emit`.
- Next actions (in priority order):
  1. Re-wire the interop **fidelity gate** in `openehr-serde`: delete the stale tests that reference the removed hand-written API, keep `tests/vendor/` (the EHRbase canonical-JSON corpus) + `schemas/`, and round-trip the corpus through the *generated* `openehr_rm` types (deserialize → re-serialize → normalized equality + ITS-JSON schema validation).
  2. Externalize the emitter's hardcoded override map to `codegen.toml` (seed from `docs/ROSETTA.md`).
  3. Canonical **XML** (ITS-XML) in `openehr-serde` via `quick-xml`, validated against the vendored XSDs.
  4. Add a drift-check CI step (regenerate + `git diff --exit-code`).
- Note: `openehr-term` is deliberately NOT generated (bundle + XML assets + access logic are not derivable from BMM). The `ehrbase-*` application crates are ported from EHRbase Java (not generated).
