# Phase SM-4 — Terminology surface + Admin completion

- Status: in-progress
- Started: 2026-07-09
- Consumes: ADR-010; design `docs/design/sm-platform/` (04 digest §3–4, 07
  §1.6/1.10, 08 §4.2/4.5, 09 §SM-4)
- Compile required: yes

## Spec oracle (read per task — hard rule)

- `docs/specs/openehr/SM/docs/UML/classes/i_terminology_service.adoc` — the
  9 calls (ids/has/description/has_term/get_term/subsumes/
  value_set_validate/has_value_set/get_value_set; preconditions
  Pre_has_terminology/Pre_has_term/Pre_has_value_set; `at_date` temporal
  semantics)
- `terminology_description.adoc`, `terminology_extract.adoc` (+ its
  `create_terminology_code` fn), `term_code.adoc`, `defined_term.adoc`,
  `term_relationship.adoc`, `terminology_relation.adoc` (xor invariant
  `local_code /= Void xor external_code /= Void`)
- `i_admin_service.adoc` — list_contributions/contribution_count/
  versioned_composition_count/composition_version_count (each takes
  `a_service: PLATFORM_SERVICE` + optional time interval);
  physical_ehr_delete (done, SM-1 era); **physical_party_delete** ("along
  with related Party relationships"; error `party_id_does_not_exist`)
- `i_admin_archive.adoc` (archive_ehrs/archive_parties),
  `i_admin_dump_load.adoc` (export_ehrs/load_ehrs), `export_spec.adoc`,
  `export_format.adoc` (canonical xml/json), `compression_format.adoc`
  (zip/7z), `encoding_format.adoc` (**empty enum** — designed {Utf8},
  PORT NOTE), `dump_load_fail_report.adoc`
- `platform_service.adoc` (enum: Admin/Definitions/Ehr/Ehr_index/
  Demographic/Message/Query/System_log; **omits Terminology +
  Subject_proxy — spec defect, PORT NOTE**)
- Wire: none of this has an ITS-REST contract (admin is dev-branch only;
  terminology none) — native API + existing admin routes only; zero ECC
  drift gate (baseline 211/318).

## Fixed design decisions

- Terminology provider = the `openehr-term` bundle: terminology ids =
  `"openehr"` + the bundle's external code-set ids; single pinned version
  (`available_versions`; `at_date` accepted, answered from the pinned
  version — PORT NOTE); `subsumes` = identity only (the openEHR vocabulary
  is flat — PORT NOTE); value sets = the openEHR terminology groups (+
  code sets), `value_set_validate` = membership; `get_term` returns a
  `Terminology_extract` with the `Defined_term` rubric(s); external
  terminology servers (FHIR tx) are a future provider behind the same
  trait.
- Admin statistics: `a_service` maps Ehr → EHR-scoped contributions
  (`ehr_id IS NOT NULL`), Demographic → ehr-less; other enum values → 0 /
  empty with a PORT NOTE (not versioned-content services). Composition
  counts filter version commit times against the interval.
- `physical_party_delete`: physical cascade of the party VO + versions/
  nodes/attestations + PARTY_RELATIONSHIP VOs that reference it
  (source/target), + orphaned audits — one tx, mirroring the EHR delete.
- Archive: `archived_at timestamptz` on `vo_version`-owning objects is NOT
  the design — archive acts at the VO level: new `vo_archive` table
  (vo_id PK, archived_at, reason NULL) + reads excluding archived VOs?
  **No** — SM says "move to archival storage"; wire reads must not change
  (zero drift). Design: `vo_archive` marker table + `archive_ehrs`/
  `archive_parties` populate it; serving reads UNCHANGED this phase
  (PORT NOTE: archival storage tier = marker now, storage movement at
  P20 optimization; SM does not define the storage form).
- Dump/load (wave 3): filesystem export per `EXPORT_SPEC` (canonical JSON
  first; XML via openehr-its; zip via the workspace's available crates —
  check what's pinned; 7z → PORT NOTE unsupported if no crate),
  segment_split_size honored, `DUMP_LOAD_FAIL_REPORT` per entity; load
  fails duplicate EHR ids per spec.

## Tasks

- [ ] `TerminologyService` trait (ehrbase-sm; 9 calls, SM-cited) + extract
      types (`TerminologyDescription`/`TerminologyExtract`/`TermCode`/
      `DefinedTerm`/`TermRelationship`/`TerminologyRelation` with the xor
      invariant) + impl over `openehr-term` + tests
- [ ] Admin statistics: `PlatformService` enum (spec defect PORT-NOTEd) +
      `admin_list_contributions`/`admin_contribution_count`/
      `versioned_composition_count`/`composition_version_count` on
      `AdminService` + impl + tests
- [ ] `physical_party_delete` (+ relationships cascade) + tests
- [ ] Archive: `vo_archive` marker + `archive_ehrs`/`archive_parties`
      (`AdminArchive` trait) + tests (reads unchanged)
- [ ] Dump/load: `AdminDumpLoad` trait (`export_ehrs`/`load_ehrs`),
      `ExportSpec`/formats/`DumpLoadFailReport` types, filesystem
      round-trip test (export → wipe → load), duplicate-id failure
- [ ] ECC zero-drift run (211/318) + workspace gates

## Exit criteria

- [ ] Workspace green (build, nextest, clippy-neutral, fmt)
- [ ] ECC ≥ 211/318, zero regressions
- [ ] New trait methods doc-cite their SM calls
- [ ] Checkboxes ticked; PROGRESS updated at close

## Handoff

SM-3 merged (PR #33, develop 327dcb7b7). Branch
`claude/sm-phase-04-terminology-admin`. Both governing interfaces read
2026-07-09; design decisions fixed above.

## Wave 2 — app-crate redesign (ADR-011, owner priority; BEFORE dump/load + SM-5)

- [ ] Structural: remove all default `NotImplemented` trait bodies; delete
      `StubBackend`; rename `Backend` → `Platform`; `AppState<S: Platform>`
      generics through the router/dispatchers (no `Arc<dyn>`); delete the
      `ehrbase-rest::{backend,response}` shims; consolidate the nine test
      mocks into one shared test-support mock; dissolve pure-delegation
      `service/api/*` modules (impls live beside their service logic)
- [ ] Purity: `SmError` (CallStatusType-based) + SM-native params replace
      `ApiError`/generated `*Params` in every `ehrbase-sm` signature; the
      wire↔native mapping lives wholly in `ehrbase-rest`
- [ ] Gates: workspace green; ECC zero-drift 211/318 (wire byte-identical);
      wave-1 verification (terminology + admin tests) folded into the run
- [ ] Then wave 3: dump/load (unchanged scope)
