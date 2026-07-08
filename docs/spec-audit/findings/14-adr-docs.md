# 14 — ADR / documentation fact-check

## Summary

The ADRs and project docs carry a layer of pre-ADR-008 (and pre-ADR-004)
statements that now actively contradict how the project works. Verified against
the committed codebase (`crates/`, `app/ehrbase/migrations/`, root
`Cargo.toml`, `rust-toolchain.toml`) and the merged git history
(`phase-11..phase-15` = PRs #15–#19, on `develop`). The recurring problems:

1. **ADR-007 is entirely superseded but still reads "accepted".** It documents a
   squashed-EHRbase-Flyway baseline (`0001_baseline.sql`), a
   `tests/resources/legacy_schema/` fixture, and a
   `baseline_schema_is_identical_to_legacy_flyway_chain` equality gate. None of
   these exist: the shipped schema is the greenfield ADR-008 `0001_schema.sql`
   (`node` + temporal `vo_version` + `ext` fns), the legacy fixture is deleted,
   and the gate test is gone. (F-14-01, major.)
2. **`current-phase.md` is a phase behind reality and self-contradictory.** It
   names "P13 — Template ingestion (in review)" as the current phase with an
   open PR on `claude/phase-13-…`; in fact P13/P14/P15 are all merged (#17/#18/#19)
   and the current phase is P16 (AQL engine). The same file says both "P12 in
   progress" and "P12 done (all items)". (F-14-02, F-14-03, major.)
3. **ADR-006 §3/§4 and ADR-001/ADR-002 mechanisms are superseded but
   unannotated** — "follow EHRbase's algorithm", "EHRbase v2 schema reused
   verbatim", `current + _history` versioning, the parity harness (ADR-006);
   the `openehr-foundation` crate and the `TypeTag` self-tagging mechanism
   (ADR-001/002, both gone — replaced by `openehr-base` and
   `#[derive(OpenEhrType)]`). (F-14-04, F-14-05, F-14-06, major.)
4. **Stale statuses + version drift + dead references** across the phase files,
   CLAUDE.md, architecture.md, VERSIONS.md, postgres-features.md (F-14-07…17).

Scope: `docs/ADRs/ADR-001..008`, `docs/architecture.md`, `docs/VERSIONS.md`,
`docs/postgres-features.md`, `CLAUDE.md`, `docs/plans/current-phase.md` +
phase files P13–P20, and the PORT_MASTER_PLAN.md amendment banners.
ADR-008 itself is accurate and current (it is the authority). PROGRESS.md is
noted (F-14-17) though outside the primary target set.

## Findings

### F-14-01: ADR-007 fully superseded by ADR-008; documents non-existent files
- **Severity:** major
- **Doc:** `docs/ADRs/ADR-007-squashed-baseline-migrations.md` (whole ADR; Status line)
- **Problem:** Status reads `accepted`. The ADR's entire subject — a squashed
  EHRbase-Flyway baseline `migrations/{ext,ehr}/0001_baseline.sql`, the
  `app/ehrbase/tests/resources/legacy_schema/` fixture, and the
  `baseline_schema_is_identical_to_legacy_flyway_chain` gate — was replaced by
  ADR-008 §2 ("ADR-007's *shipped schema content* is replaced"). Verified: the
  actual migrations are `ehr/0001_schema.sql` (greenfield `node` + temporal
  `vo_version` + `contribution`/`audit`/`template_store`/`stored_query`/`item_tag`,
  header cites ADR-008 + the P10 spike) and `ext/0001_openehr_functions.sql`;
  `tests/resources/legacy_schema/` does **not** exist; no such test exists. A
  reader taking ADR-007 at face value would look for 17 EHRbase tables and a
  Flyway equality gate that are not in the tree.
- **Fix:** Change Status to superseded-by-ADR-008 and add a dated amendment
  banner at the top: only the sqlx-migrator / testcontainer / baseline-per-schema
  *infrastructure* is retained; the EHRbase-baseline schema content, the legacy
  fixture, and the equality gate are gone (replaced by the greenfield
  `0001_schema.sql`).
- [x] fixed

### F-14-02: `current-phase.md` still names P13 "in review" as the current phase
- **Severity:** major
- **Doc:** `docs/plans/current-phase.md` (lines 1, 4, 11)
- **Problem:** Says "**Current phase: P13 — Template ingestion (in review)** …
  On branch `claude/phase-13-template-ingestion`, PR open, awaiting review before
  advancing to P14." Git history shows P13 (`9f11963f0`, #17), P14 (`3c913dda4`,
  #18) and P15 (`0906f1937`, #19) are all merged into `develop`. The current
  phase is **P16 (AQL engine)**. The build-order line still marks P13 as current
  ("→ **P13** templates (in review) →").
- **Fix:** Rewrite the pointer to P16; mark P13/P14/P15 done in the build-order
  line; drop the "PR open / branch" language.
- [x] fixed

### F-14-03: `current-phase.md` internally contradicts itself on P12
- **Severity:** major
- **Doc:** `docs/plans/current-phase.md` (lines 6, 8, 9, 11)
- **Problem:** Line 6 says "**P12 in progress (2026-07-05)**"; lines 8–9 and the
  build-order line (11) say "**P12 done (all items)**" and "P12 ✅". P12 is merged
  (`2e54d7bbc`, #16). The "in progress" framing is stale.
- **Fix:** Removed as part of the P16 rewrite of the file (P12 is stated done).
- [x] fixed

### F-14-04: ADR-006 §3/§4 superseded by ADR-008 but Status unannotated
- **Severity:** major
- **Doc:** `docs/ADRs/ADR-006-application-port-philosophy.md` (Status line; §3, §4)
- **Problem:** Status reads `accepted`. §3 ("bespoke server logic **follows
  EHRbase's algorithm as the reference**"; "composition versioning (current +
  `_history`)"), §4 ("the real EHRbase v2 schema is **reused verbatim**"), and
  the "parity harness … `USE_REFERENCE_EHRBASE=1`" acceptance instrument are all
  superseded by ADR-008 (own PG18 storage/engine, temporal `vo_version` — no
  `_history` pairs, openEHR CNF conformance replacing the parity harness).
  ADR-008's own header declares this, but ADR-006 carries no back-reference, so
  reading ADR-006 alone gives the retired parity/verbatim-schema picture. (The
  idiomatic-modern-Rust-app decision and the stack in §2 still stand.)
- **Fix:** Change Status to note §3/§4 superseded by ADR-008 and add a dated
  amendment banner at the top pointing to ADR-008 for storage/engine/acceptance;
  §1/§2/§5/§6 stand.
- [x] fixed

### F-14-05: ADR-002 `TypeTag` mechanism superseded by `#[derive(OpenEhrType)]`
- **Severity:** major
- **Doc:** `docs/ADRs/ADR-002-canonical-json-self-tagging.md` (Status line; whole Decision)
- **Problem:** Status reads `accepted`. The entire decision — a
  `TypeTag<Self>` first-field ZST living in
  `openehr_foundation::serde_support` — is gone. Verified: `grep -r TypeTag
  crates/` returns nothing; `openehr-foundation` no longer exists; canonical
  `_type` (de)serialization is now supplied by `#[derive(OpenEhrType)]`
  (`openehr-derive`) on the generated types (ADR-004). ADR-002 also frames the
  acceptance bar as "behavioural parity with EHRbase … (P18)", retired by
  ADR-008. The schema path it cites is also stale (see F-14-11).
- **Fix:** Change Status to superseded-by-ADR-004 (mechanism) / ADR-008 (parity
  framing) and add an amendment banner: `TypeTag` → `#[derive(OpenEhrType)]`;
  `openehr-foundation` folded into `openehr-base`; parity retired for conformance.
- [x] fixed

### F-14-06: ADR-001 references the dead `openehr-foundation` crate; superseded as conventions
- **Severity:** major
- **Doc:** `docs/ADRs/ADR-001-spec-transcription-shapes.md` (Status line; §2, Refinements)
- **Problem:** Status reads `accepted`. ADR-004 header declares it supersedes
  ADR-001 "as *hand-authoring conventions*" — the spec crates are now generated
  from BMM, not hand-transcribed with these shapes. ADR-001 cites worked
  examples in `crates/openehr-foundation/src/...` (crate gone, folded into
  `openehr-base`) and the `TYPE_NAME` const / deferred-serde plan replaced by the
  derive. The MI/covariance/generic *outcomes* still describe the emitter's
  choices, but the "transcribers copy shapes mechanically" framing is retired.
- **Fix:** Change Status to superseded-in-part by ADR-004 (as conventions) and
  add an amendment banner: generation replaced hand-transcription;
  `openehr-foundation` → `openehr-base`; the emission choices now live in the
  generator (ADR-004 §3).
- [x] fixed

### F-14-07: phase-13/14/15 file Status fields are stale (all merged)
- **Severity:** minor
- **Doc:** `docs/plans/phase-13-template-ingestion.md`,
  `phase-14-webtemplate.md`, `phase-15-validation.md` (Status lines)
- **Problem:** P13 Status = "not-started", P14 = "in-progress (PR-A … PR-B …)",
  P15 = "not-started". All three are merged to `develop` (#17/#18/#19). Later
  phase files (P16–P20) still read "not-started", which is correct.
- **Fix:** Set P13/P14/P15 Status to done with the merge-commit/PR reference.
- [x] fixed

### F-14-08: CLAUDE.md dependency-version narrative drifted from the manifest
- **Severity:** minor
- **Doc:** `CLAUDE.md` "Tech stack (pinned)" section
- **Problem:** ~23 version numbers in the orientation prose lag the authoritative
  `[workspace.dependencies]` (CLAUDE.md itself says the manifest wins, so these
  are stale not authoritative): axum-extra 0.10→0.12, axum-server 0.7→0.8,
  tower-http 0.6→0.7, jsonwebtoken 9→10, tower-sessions 0.14→0.15, axum-login
  0.17→0.18, quick-xml 0.37→0.41, serde_jcs 0.1→0.2, chumsky 0.10→0.13, logos
  0.15→0.16, fancy-regex 0.14→0.18, ariadne 0.5→0.6, garde 0.22→0.23,
  metrics-exporter-prometheus 0.16→0.18, axum-prometheus 0.8→0.10, tower_governor
  0.4→0.8, governor 0.6→0.10, reqwest 0.12→0.13, jsonschema 0.26→0.46, rstest
  0.23→0.26, mockall 0.13→0.15, fake 3→5, testcontainers 0.24→0.27.
- **Fix:** Update the CLAUDE.md prose to match the manifest.
- [x] fixed

### F-14-09: architecture.md build sequence shows only P09 done
- **Severity:** minor
- **Doc:** `docs/architecture.md` "Build sequence & stages" + intro bullets
- **Problem:** The Stage-1 line reads "**P09** persistence infra ✅ → **P10**
  storage foundation → **P11** REST+auth → …" with only P09 ticked. P09–P15 are
  all merged/done (#15–#19).
- **Fix:** Tick P09–P15 (✅) in the build-sequence line.
- [x] fixed

### F-14-10: `scripts/conformance.sh` referenced as if it exists
- **Severity:** minor
- **Doc:** `CLAUDE.md` "Build and test" code block; `docs/architecture.md` "Verification"
- **Problem:** Both present `scripts/conformance.sh` as a runnable command. The
  `scripts/` dir contains only `check-codegen-drift.sh`, `install-hooks.sh`,
  `vendor-spec-docs.sh`. The runner is future work (phase-19 says "*a*
  conformance runner (`scripts/conformance.sh`)" is to be built). The CLAUDE.md
  comment "built out from P12 (smoke) to P19" implies a smoke version exists; it
  does not.
- **Fix:** Mark the command as not-yet-present (planned P19) in both docs.
- [x] fixed

### F-14-11: ADR-002 cites a stale schema path
- **Severity:** minor
- **Doc:** `docs/ADRs/ADR-002-canonical-json-self-tagging.md` (Context)
- **Problem:** Cites `crates/openehr-its/schemas/openehr_rm_1.1.0_all.json`. The
  file is at `crates/openehr-its/schemas/json/openehr_rm_1.1.0_all.json` (note the
  `json/` segment; VERSIONS.md has the same short path).
- **Fix:** Noted in the ADR-002 amendment banner (the ADR body is historical and
  left intact per the ADR convention); the live path is corrected in VERSIONS.md.
- [x] fixed

### F-14-12: postgres-features.md still says "current + `_history`"
- **Severity:** minor
- **Doc:** `docs/postgres-features.md` (PG18 table rows for temporal keys / RETURNING; partition row)
- **Problem:** Rows describe "versioned rows (current + `_history`)" and
  "`audit_details`/`_history`" — the EHRbase current/history model. ADR-008
  replaced this with one temporal `vo_version` table (no `_history` pairs). The
  PG-feature content itself is still valid; only the schema framing is stale.
- **Fix:** Reword the `_history` references to the temporal `vo_version` model.
- [x] fixed

### F-14-13: VERSIONS.md frames EHRbase parity as the acceptance path
- **Severity:** minor
- **Doc:** `docs/VERSIONS.md` "EHRbase reference point" + the "Parity note"
- **Problem:** "Treat v2.33.0 as the operative behavioural baseline for **parity
  testing**" and "Track this divergence as a Stage-1 **REST parity**
  consideration". ADR-008 retired the parity harness; the acceptance instrument
  is the CNF conformance schedule + the fidelity round-trip gates. EHRbase is
  prior art. (The RM-version divergence itself is real and worth keeping.)
- **Fix:** Add an ADR-008 note that parity testing is retired (conformance is the
  target; EHRbase = prior art) and correct the ITS-JSON schema path.
- [x] fixed

### F-14-14: ADR-003 references the dead `openehr-foundation` crate
- **Severity:** minor
- **Doc:** `docs/ADRs/ADR-003-spec-gap-policies.md` (Context, Decision §6, Consequences)
- **Problem:** Cites `openehr-foundation` (as a crate the policies span and as
  gaining the `url` dependency). That crate was folded into `openehr-base`
  (ADR-004). The behaviour policies themselves still govern the hand-written
  `*_impl.rs` files (ADR-004 confirms ADR-003 stands), so only the crate name is
  stale.
- **Fix:** Add a one-line amendment banner: `openehr-foundation` folded into
  `openehr-base`; policies still govern the `*_impl.rs` behaviour layer.
- [x] fixed

### F-14-15: ADR-004/005 Consequences frame EHRbase parity as Stage-1 acceptance
- **Severity:** minor
- **Doc:** `docs/ADRs/ADR-004-spec-driven-codegen.md`,
  `docs/ADRs/ADR-005-its-codegen.md` (Consequences / Context)
- **Problem:** Both were written before ADR-008 and refer to "the actual EHRbase
  port", "behaviour-parity with stock EHRbase", "P18 REST-parity", and (ADR-004)
  "the old hand-written tests in `openehr-its/tests` … are currently broken".
  ADR-008 retired parity for CNF conformance; the fidelity gates are now green
  (CLAUDE.md: "all fidelity gates green"). The codegen decisions themselves are
  current and correct.
- **Fix:** Add a short dated note to each pointing at ADR-008 for the acceptance
  target; leave the (accurate) codegen decisions intact.
- [x] fixed

### F-14-16: CLAUDE.md "Compile status" lists P09–P20 as remaining app work
- **Severity:** info
- **Doc:** `CLAUDE.md` "Build and test" → Compile-status paragraph
- **Problem:** "The `ehrbase-*` application crates are the remaining work (the
  Stage-1 app build, `docs/plans/` phases 09–20) and are built compiling per
  phase." P09–P15 are done; the remaining work is P16–P20 (+P99).
- **Fix:** Narrow the "remaining" range to P16–P20.
- [x] fixed

### F-14-17: PROGRESS.md is stale (related; outside the primary target set)
- **Severity:** info
- **Doc:** `docs/PROGRESS.md` (Stage-1 table)
- **Problem:** P12 = "in progress"; P13/P14/P15 = "not-started" (all merged); the
  P09 note cites "squashed `0001_baseline.sql` per schema + schema-equality gate
  vs the legacy Flyway chain (ADR-007)" — the ADR-007 artifacts that F-14-01 shows
  are gone. Not in this audit's assigned file set; recorded so the owner can have
  the `phase-done` flow (or a follow-up) reconcile it with the corrected
  `current-phase.md`.
- **Fix:** OPEN — left to the `phase-done` workflow / owner. Flagged only.
- [ ] fixed

## OPEN questions for the owner

- **Q1 (PROGRESS.md, F-14-17):** Should PROGRESS.md be reconciled here (P12 done,
  P13/14/15 done, P09 note de-ADR-007'd), or is that owned by the `phase-done`
  skill / a separate pass? It was outside this audit's assigned file set, so it
  was left untouched.
- **Q2 (ADR-007 disposition):** ADR-007's decision is fully replaced, but its
  *infrastructure* (sqlx two-schema migrators, testcontainer gate, baseline-per-
  schema method) is retained by ADR-008. Confirm the chosen framing — Status
  "superseded by ADR-008 (infrastructure retained)" — is how you want superseded
  ADRs labeled, vs a bare "superseded".
