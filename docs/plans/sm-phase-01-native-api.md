# Phase SM-1 — repo layout (`app/*`) + the `ehrbase-sm` native-API crate + EHR-core completion

- Status: not-started
- Owner: —
- Consumes: ADR-010; design set `docs/design/sm-platform/` (esp. 08 §2–§5,
  09 §SM-1); spec oracle `docs/specs/openehr/SM/docs/openehr_platform/`
- Compile required: yes (compiling, tested increments — ADR-006)

## Objectives

Make the two-layer split physical and the SM Platform Service Model the
explicit service seam: move the application crates to `app/*` (spec layer
stays in `crates/*`), stand up
`app/ehrbase-sm` as the native API (traits + shared types + error table),
migrate the `Backend` trait family out of `ehrbase-rest`, split the EHR
mega-trait along SM interface boundaries, and close the EHR-core gaps
(contribution listing/count, attestations, EHR_SUMMARY counts). Behaviour-
preserving at the wire: the ECC conformance run must not move.

## Preconditions

- [x] ADR-010 accepted; design set merged (`docs/design/sm-platform/`)
- [ ] P17 status checked (SM-1 may land before or after P17; no file overlap
      expected beyond `ehrbase-rest` dispatch glue)

## Scope

In: the `app/*` workspace-layout move, crate creation, trait migration/split,
shared types (`UpdateVersion<T>`,
`UpdateAudit`, `Page`, `EhrSummary`+counts, execute specs, `CallStatus`
table), `ValidityChecker` + `SystemLog` facades, `list_contributions`/
`contribution_count`, attestation storage in the contribution path,
`is_queryable` gate test + RESULT_SET meta audit.
Out: new components (EHR Index, Terminology, Message, Subject Proxy — SM-3…
SM-6), ADL2/archetype store (SM-2), any wire-shape change.

## Tasks

- [x] **Workspace layout move — done 2026-07-08** (on
      `claude/sm-platform-design`): application crates `git mv`'d to
      `app/*`; dev/verification tooling to `tools/*` with renames
      (`ehrbase-conformance` → `conformance`, `ehrbase-bench` →
      `benchmark`); members = `["crates/*", "app/*", "tools/*"]`;
      path-deps, cross-crate fixture paths, CI, scripts, docker runners,
      `.claude/rules` scopes, `CLAUDE.md`, `docs/architecture.md` updated.
      Gate held: build green, nextest 813/813.
- [x] `app/ehrbase-sm` scaffold — done 2026-07-09 (deps: openehr-base/rm/its/
      flat + async-trait/serde/jiff; no thiserror needed yet)
- [x] Move `ServiceResponse`/`ResourceMeta`, `AqlQueryRequest`/`QueryOutcome`,
      `PartyKind` into `ehrbase-sm::types`; `ehrbase-rest` `backend.rs`/
      `response.rs` are re-export shims — done 2026-07-09
- [ ] Define `CallStatus` + the SM↔`ServiceError`↔HTTP table (doc 08 §5);
      map `ServiceError` through it — *table + `CallStatusType`/`CallStatus`
      landed in `ehrbase-sm::error` (28 statuses, unit-tested); the
      `ServiceError` rewiring in `ehrbase` remains*
- [x] Split `EhrService` into `EhrService`/`EhrStatusService`/
      `EhrDirectoryService`/`EhrCompositionService`/`EhrContributionService`
      (SM citations in doc-comments); `Backend` alias updated (duplicate
      `DemographicService` bound dropped); dispatchers + 12 test files
      rewired — done 2026-07-09, 817/817 tests
- [x] Move `DemographicService`/`AdminService`/`QueryService`/
      `WebTemplateService` + generated `DefinitionApi` bound into the alias
      from `ehrbase-sm` — done 2026-07-09
- [ ] `UpdateVersion<T>` + `UpdateAudit` types; `vobject` constructors take
      them; ITS-REST adapter builds them from body + headers. Honour the
      three wire divergences per design 08 §3 (review F2): `commit_audit`
      field name, partial `UpdateAttestation` items, `signature` field —
      each with a `// PORT NOTE:` citing
      `ITS-REST/specifications/schemas/common/UpdateVersion.yaml` — *types
      landed in `ehrbase-sm::types` with the three PORT NOTEs + a wire-shape
      deserialization test; the `vobject`/adapter rewiring remains*
- [ ] `ValidityChecker` trait over the existing validation choke points —
      *trait landed (`services/validity.rs`); wiring `EhrbaseService`'s
      validate choke points to it remains*
- [x] `SystemLog` facade naming `ehrbase-audit` as the SM component
      (`services/system_log.rs`, doc-cites the `I_SYSTEM_LOG` stub + ATNA)
- [ ] `EhrSummary` gains `contribution_count`/`composition_count`
      (`i_ehr_service.adoc` EHR_SUMMARY); wire into `ehr_summary`
- [ ] `list_contributions(ehr_id, time_range, page)` +
      `contribution_count(ehr_id, time_range)` (`i_ehr_contribution.adoc`)
- [ ] Attestations: accept `666|attestation|` contributions; persist
      `UPDATE_VERSION.attestations` on the version; revision-history
      exposure (`update_version.adoc`, `master03` §Version Update Semantics)
- [ ] Query gate: e2e test that population queries exclude
      `is_queryable = false` EHRs (`i_query_service.adoc` `ehr_ids` doc);
      RESULT_SET meta audit vs ITS-REST shape
- [ ] `docs/architecture.md`: add the SM component map, the `app/*` vs
      `crates/*` layout, and the `ehrbase-sm` crate row

## Exit criteria

- [ ] Workspace green: build, `cargo nextest run --workspace`, clippy, fmt
- [ ] ECC conformance run: identical pass set to the pre-phase baseline
      (zero wire drift), except new passes from attestation support
- [ ] `ehrbase-rest` contains no service-trait definitions (adapter only)
- [ ] Every `ehrbase-sm` trait method doc-comment cites its SM call
      (file + section)

## Decisions made this phase

- ADR-010 (phase-opening decision)

## Handoff for next session

Design complete on `claude/sm-platform-design` (design set + ADR-010 + this
plan). Next action: start the first task (`ehrbase-sm` scaffold) on
`claude/sm-phase-01-*`, or continue P17 first — owner's call on ordering
(doc 09 allows either).
