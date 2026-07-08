# SM design — roadmap: building the full SM-aligned platform

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Owner ruling (2026-07-08): **full coverage, nothing deferred; EHR_EXTRACT is
Stage-1 scope.** Discipline unchanged: compiling + clippy-clean + tested per
phase; spec citations for every conformance-relevant decision; branches
`claude/sm-phase-NN-*`; ITS-REST/CNF stay the wire oracle.

## Relationship to the existing P-phases

The P-sequence (P17 FLAT/EhrScape → P18 integration → P19 conformance →
P20 optimization → P99) continues unchanged. The SM phases below interleave:
SM-1 is the structural refactor and lands before P18 (integration verifies
the new seam); SM-2…SM-6 add components and can proceed in parallel with
P18–P20 wherever they don't touch the conformance-gated wire. P17 absorbs
the SIM-B/SDF audit (it is FLAT work).

## SM-1 — repo layout + the native-API crate + EHR-core completion

*The structural move; behaviour-preserving except listed additions.*

- [x] **Workspace layout (owner ruling) — done 2026-07-08** on
      `claude/sm-platform-design`: application crates → **`app/*`**;
      dev/verification tooling → **`tools/*`** with renames
      (`ehrbase-conformance` → `conformance`, `ehrbase-bench` →
      `benchmark`); spec layer stays in `crates/`. Members =
      `["crates/*", "app/*", "tools/*"]`; path-deps, CI, scripts, docker
      runners, `.claude/rules` scopes, `CLAUDE.md`, `docs/architecture.md`
      all updated; workspace green (build + clippy-neutral + full nextest).
- [ ] `app/ehrbase-sm`: create crate; move the `Backend` trait family out
      of `ehrbase-rest::backend`, split the EHR mega-trait into
      `EhrService`/`EhrStatusService`/`EhrDirectoryService`/
      `EhrCompositionService`/`EhrContributionService`; `Backend` = alias.
      SM citations in every trait/method doc-comment.
- [ ] Shared types: `UpdateVersion<T>`, `UpdateAudit`, `Page`, `EhrSummary`
      (+ `contribution_count`/`composition_count`), execute specs,
      `CallStatus` error table (doc 08 §5); rewire `vobject` + dispatchers.
- [ ] `ValidityChecker` trait naming the existing validation choke points.
- [ ] EHR-core gaps: `list_contributions` (time-range + paging),
      `contribution_count`; attestation support in the contribution path
      (stop rejecting `666|attestation|`; store `UPDATE_VERSION.attestations`
      on the version).
- [ ] `SystemLog` facade over `ehrbase-audit` (component naming only).
- [ ] Query verification: `is_queryable` population gate test + RESULT_SET
      meta audit vs ITS-REST.
- Exit: workspace green, ECC conformance run unchanged (the refactor must
  not move the wire).

## SM-2 — Definitions service completion

- [ ] ADL 1.4 archetype store (upload/get/list/delete + regex matching +
      counts) beside the OPT store.
- [ ] OPT: `delete_opt`, `valid_opt` exposed, regex listing, counts.
- [ ] ADL2 (`DefinitionAdl2Service`): AUTHORED_ARCHETYPE ingest over
      `openehr-am::am24`, artefact CRUD + typed listings + counts; retire
      the `adl2` 501s.
- [ ] Stored queries: `valid_query` (parse via `openehr-query`),
      `delete_query`, `queries_count`; `store_query_set` contract filled by
      design (`// PORT NOTE:` — spec TODO).

## SM-3 — Demographic completion + EHR Index

- [ ] `PartyRelationshipService`: `Kind::PartyRelationship` in `vobject`,
      CRUD + at-time/at-version + versioned + revision history; wire under
      the demographic extension routes.
- [ ] `EhrIndexService`: `ehr_index` table (N:M, `ResourceStatus`,
      `LocationDesc`), five SM calls, audit-event emission; reconcile with
      the `ehr.subject_id` Primary fast path; duplicate-detection admin
      queries.

## SM-4 — Terminology surface + Admin completion

- [ ] `TerminologyService` over `openehr-term` (ids, description, term
      lookup → `TerminologyExtract`, `subsumes`, value-set validate/get);
      external-provider seam (FHIR tx adapter behind the same trait).
- [ ] Admin: `physical_party_delete` (+ relationships); per-service
      contribution/composition statistics; archive tier
      (`archive_ehrs`/`archive_parties`); dump/load (`export_ehrs`/
      `load_ehrs`, `ExportSpec` formats + compression + segmenting,
      `DumpLoadFailReport`, duplicate-id failure).

## SM-5 — Message service (EHR_EXTRACT + TDD)

- [ ] Codegen: emit the RM `ehr_extract` package from the vendored BMM
      (extend `emit`; drift gate covers it). Canonical JSON/XML for the
      extract types via the existing derive/emit-xml paths.
- [ ] `EhrExtractService`: `export_ehrs`, `export_ehr_extracts(spec)`,
      `import_ehr(id?, extract)`, `import_ehr_extract` — over `vobject`
      reads + `commit_contribution` replay (IMPORTED_VERSION semantics).
- [ ] `TddService`: `import_tdd` (TDD XML → OPT-guided COMPOSITION →
      validated commit), `import_tdds` batch with per-item fail report.
- [ ] `MessageService` umbrella trait + extension routes.

## SM-6 — Subject Proxy service

- [ ] Config stores (`sp_subject`, `sp_variable`, `sp_data_set`,
      `sp_binding`) + `sp_sample` history; `reset()`.
- [ ] Types: proxy/variable/data-set/sample/value hierarchy (doc 08 §3).
- [ ] `DataBinding` + `OpenehrFrame` executor (AQL via `QueryService` →
      `OpenehrSample{result: RESULT_SET}`); FHIR/HL7v2 frame adapter seams.
- [ ] `SubjectProxyService`: all 14 calls incl. data-set registration
      (JSON/YAML payloads), currency/freshness logic, canonical-name rules
      (no whitespace/unprintables), aliasing.
- [ ] Extension routes + YAML/JSON data-set and binding ingestion.

## Verification (every phase)

- Unit + e2e (testcontainers PG18) per component; the SM pre/post-conditions
  become test assertions (e.g. `Post_has_composition`, `not has_party` after
  delete).
- ECC conformance suite must stay green after every phase (SM-1 especially:
  zero wire drift).
- `spec-conformance-reviewer` pass before each phase close; spec citations
  in commits.
- Doc upkeep: `docs/architecture.md` gains the SM component map; each phase
  ticks its boxes here and in its `docs/plans/sm-phase-NN-*.md` file.

## Decision record

ADR-010 (SM-aligned service architecture) records the decision, the
SM-vs-ITS-REST precedence rule, the `ehrbase-sm` crate, and the full-scope
ruling. This roadmap is the executable form.
