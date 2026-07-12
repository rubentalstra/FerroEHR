# W-3f — Platform-crate redesign (`app/ehrbase`)

The complete redesign and rewrite of the `ehrbase` platform crate — the
binary + `Platform` implementation: storage, service layer, AQL engine,
versioning. `ehrbase-sm` (W-3c) and `ehrbase-rest` (W-3e) were rebuilt
specification-first and merged; this crate is the last unstructured one
(~36.5k lines, a flat `service/` grab-bag, files of 1,000–2,100 lines).

Session prompt (verbatim mandate): `docs/plans/w3f-platform-redesign-PROMPT.md`.
Method identical to W-3c/W-3e: register first, then big-bang rewrite, one
fix pass, deferred checks last (nextest → clippy → ECC).

## Oracles (precedence order)

1. **The FULL BASE component** (`docs/specs/openehr/BASE/docs/`):
   `architecture_overview/` (structural oracle — read the chapter before
   designing each area; §Integrity dissolves `signing/` into versioning),
   `base_types/` (identification law), `foundation_types/` (primitive
   semantics), `resource/` (AUTHORED_RESOURCE). W-3a distillation index:
   `docs/spec-audit/architecture-overview/CHECKLIST.md` (149 rows).
2. **The SM chapter map** — impl side mirrors `app/ehrbase-sm/src/services/`
   (admin, definition, demographic, ehr, ehr_index, message, query,
   subject_proxy, system_log, terminology). Registers:
   `docs/design/sm-platform/` per-chapter G-rows; open impl-side rows land
   here (absorbs W-3d).
3. **RM common change-control** (`docs/specs/openehr/RM/docs/common/`) for
   the versioning core; `docs/design/aql-engine.md` +
   `docs/architecture.md` §Storage for spec-silent internals (flagged:
   "no openEHR spec governs this — our own design").

## Standing rulings

- **Spec→code mapping (owner, 2026-07-12): map the spec ONTO the code, never
  the code onto the spec.** Each register's skeleton is the spec's own
  structure — enumerate requirements section-by-section from the oracle
  chapters first, then map existing code onto each item
  (conformant/divergent/missing); code with no spec home is classified
  spec-silent-flagged / extension-quarantine / delete. The target design
  derives from the spec's decomposition, never from the current file layout.
- Register first; then chapter/area-at-a-time fresh authoring — never
  migrate legacy files; audited-faithful logic may carry but every file is
  re-grounded and re-verified.
- Intermediate steps need not compile; ONE fix pass at the end.
- Parallel Opus workers, disjoint file ownership, build lanes
  `target/agent-t1..t4`; cross-folder needs = `// TODO(w3f-integrate):`.
  **Max 2 workers running at the same time (owner, 2026-07-12 — token
  budget); the rewrite proceeds in pairs.**
- Zero-TODO mandate at close; `dead_code`/`clippy::todo` deny.
- Spec citations only in code; spec-silent design flagged; no import
  renaming; `urlencoding` for percent-coding; official CLIs.
- **Code first — ALL of it.** Deferred checks only when everything is
  rewritten, in order: (1) full nextest triage (update stale pre-rewrite
  assertions spec-correctly, never weaken), (2) workspace clippy,
  (3) ECC last — adjudicate the CASE where the instrument contradicts the
  vendored spec (B5 process, spec-cited); honest re-baseline from 341/315/0.
- Schema settled: `0001_baseline.sql` changes only on a register G-row.
- `Platform` trait surface (`ehrbase-sm`) is the fixed contract.

## Target structure (directional; the register refines it)

```
app/ehrbase/src/
├── main.rs / lib.rs      binary + crate map
├── db/                   pool, migrators (exists — re-ground docs)
├── storage/              node codec + decomposed node model (spec-silent, flagged)
├── versioning/           RM common change control + AO §Versioning/§Integrity
│                         (VERSIONED_OBJECT lifecycle, CONTRIBUTION, audits,
│                         attestations, digital signature — signing/ dissolves here)
├── service/              one folder per SM chapter, mirroring ehrbase-sm
├── aql/                  the engine (re-ground + split sql.rs)
├── validation/           opt_validation + adl2_validation along AM boundaries
├── templates/            OPT ingestion + WebTemplate cache
├── system_log/           ATNA emitter (exists)
└── extensions/           enterprise, spec-silent, quarantined + flagged
```

## Tasks

### Stage 1 — the register (`docs/design/platform/`)

- [x] R1 Register: versioning + integrity (incl. signing dissolution) — `01-versioning-integrity.md`
- [x] R2 Register: storage / node codec / db — `02-storage.md`
- [x] R3 Register: service/ehr (EHR, EHR_STATUS, composition, directory, contribution wiring, item tags) — `03-service-ehr.md`
- [x] R4 Register: service/demographic + ehr_index — `04-service-demographic-ehr-index.md`
- [x] R5 Register: service/definition + query (stored queries, AQL service seam) — `05-service-definition-query.md`
- [x] R6 Register: service/message + admin (extract, TDD, dump/load) — `06-service-message-admin.md`
- [x] R7 Register: service/subject_proxy + terminology + validity — `07-service-subject-proxy-terminology-validity.md`
- [x] R8 Register: aql engine — `08-aql.md`
- [x] R9 Register: validation (OPT + ADL2) — `09-validation.md`
- [x] R10 Register: templates (OPT ingestion, WebTemplate, AUTHORED_RESOURCE) — `10-templates.md`
- [x] R11 Register: system_log + telemetry — `11-system-log.md`
- [x] R12 Register: extensions/enterprise quarantine (events, fhir, multimedia/S3, tenancy, ehr_access cache, event subscriptions) — `12-extensions.md`
- [x] R13 Register README: crate map, area→oracle table, cross-register integration seams — `README.md`

### Stage 2 — the rewrite (big-bang, no intermediate fix passes)

- [x] W1 `versioning/` authored fresh (change control, audits, attestations, signature per §Integrity; signing/ dissolved) — 12 files, G-01/G-02/G-09 fixed; legacy deletion + CommitEnv wiring at W10
- [x] W2 `storage/` authored fresh (node codec + node model split out of service/vobject.rs) — node_repo + version_repo + lean ReadRow; G-S1..S6 fixed; db/ re-grounded
- [x] W3 `service/` re-authored one folder per SM chapter (mirroring ehrbase-sm) — ehr, demographic, ehr_index, definition, query, message, admin, terminology, validity, subject_proxy (re-ground); api/ impls collapsed into chapters
- [x] W4 `aql/` re-grounded; `sql.rs` split — sql/{mod,from,expr,value,predicate,select}.rs ≤505 lines; OR-CONTAINS implemented (blueprint claim was false), NOT CONTAINS generalized, MIN/MAX + Raw coercion fixed
- [x] W5 `validation/` authored fresh along AM boundaries — opt/{invariants,rm_conformance,primitive,terminology,interval} + adl2/; BASE MultiplicityInterval::has consumed; openehr-lang::odin found nonexistent (hand-rolled reader kept, PORT NOTE)
- [x] W6 `templates/` authored fresh (+ resource/ oracle) — {mod,identity,ingest,store,runtime}.rs; G-T04 case-insensitive template_id law landed
- [x] W7 `system_log/` re-grounded — citations re-anchored to DICOM PS3.15/RFC 3881/5424-5426; ObjectClass::Extract added (additive); telemetry flagged spec-silent
- [x] W8 `extensions/` quarantine assembled + flagged — events, subscriptions, fhir (mapping split 3-way), multimedia, tenancy; off = byte-identical invariant preserved
- [ ] W9 `main.rs`/`lib.rs`/`db/` re-grounded; crate map documented
- [ ] W10 The ONE fix pass — workspace compiles, all `TODO(w3f-integrate)` resolved
- [ ] W11 Zero-TODO sweep: inventory + eliminate every actionable marker

### Stage 3 — deferred checks (in this order, only when ALL code is rewritten)

- [ ] C1 Full `cargo nextest run --workspace` triage (stale W-3c/W-3e expectations updated spec-correctly, never weakened)
- [ ] C2 Workspace clippy green under deny rules
- [ ] C3 ECC full run LAST (`scripts/conformance.sh`); case adjudications where the instrument is wrong (spec-cited); honest re-baseline from 341/315/0

### Stage 4 — close

- [ ] X1 Register G-rows all closed in code or re-verified cited PORT NOTE
- [ ] X2 No file > ~700 lines without documented reason; every area maps to its oracle or sits in extensions/ flagged
- [ ] X3 Changelog entry + website book same-PR updates
- [ ] X4 WORKLIST row W-3f closed with merged PR; plan closed

## Exit criteria

- [ ] `docs/design/platform/` register complete (every area audited, G-rows cited)
- [ ] Every `src/` area maps to its Architecture-Overview/SM oracle (or `extensions/` flagged); signing dissolved into versioning; no file > ~700 lines without documented reason
- [ ] Every register G-row closed in code or a re-verified cited PORT NOTE
- [ ] Zero actionable TODO markers; dead-code/todo denies green
- [ ] Workspace build + full nextest + clippy green; changelog + book updated
- [ ] LAST: ECC run — honest re-baseline, spec-cited case adjudications where the instrument (not the server) is wrong
