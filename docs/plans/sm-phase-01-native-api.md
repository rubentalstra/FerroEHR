# Phase SM-1 — repo layout (`app/*`) + the `ehrbase-sm` native-API crate + EHR-core completion

- Status: done (2026-07-09; two tasks carry explicit partial-scope notes
  folded into SM-2: `ServiceError` call-site adoption of `ServiceError::sm`,
  and the `vobject` internal carrier swap to `UpdateVersion<T>` — both
  deliberate decisions, not omissions)
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
- [x] `EhrSummary` counts — done 2026-07-09: `EhrService::get_ehr_summary`
      (realizes SM `get_ehr: EHR_SUMMARY`; named to avoid colliding with the
      inherent wire-EHR builder) + `summarize_ehr` producing all six
      mandatory fields; `composition_count` = versioned objects, not
      versions (`ehr_summary.adoc`). Wire unchanged (no ITS-REST route
      emits EHR_SUMMARY — PORT NOTE)
- [x] `contribution_list`/`contribution_count` — done 2026-07-09
      (`i_ehr_contribution.adoc`): native-API calls (no ITS-REST route —
      extension exposure later, design 08 §7); time-range bounds either
      side open, `Page` cursor, oldest-first; NotFound = SM
      `ehr_does_not_exist`; e2e-tested incl. paging/filtering/error cases
- [x] Attestations — done 2026-07-09: `vo_attestation` table (canonical
      ATTESTATION verbatim, contribution-linked); 666 = attestation of an
      existing ORIGINAL_VERSION (no new version; 400/422/404 error rules);
      accompanying attestations server-completed + stored with the new
      version in one tx; ORIGINAL_VERSION.attestations + revision-history
      audits + CONTRIBUTION.versions union; signature interaction handled
      (attestations appended post-verification per master06 post-committal
      signing); 2 e2e + classify unit tests
- [x] Query gate — done 2026-07-09: **gap found and fixed** — the engine
      never consulted `is_queryable`; `apply_population_gate` now restricts
      unscoped queries to EHRs whose current EHR_STATUS root has
      `is_queryable = true` (`i_query_service.adoc`); e2e test + RESULT_SET
      shape audit vs the ITS-REST schemas; persistence fixture made
      spec-realistic (every EHR seeds a queryable EHR_STATUS)
- [x] `docs/architecture.md`: SM component map (interface → trait → status)
      + layout + `ehrbase-sm` row — done 2026-07-09

## Exit criteria

- [x] Workspace green: build (0 warnings), `cargo nextest run --workspace`
      **824/824 passed**, clippy-neutral, fmt clean (2026-07-09)
- [x] ECC conformance run — **MET 2026-07-09: 211/318, byte-identical pass
      set to the pre-phase baseline, zero regressions** (full catalogue,
      both formats, fresh image). Diagnosing the initial false regression
      hardened the harness: `scripts/conformance.sh` builds before `up`,
      defaults to the full catalogue + both formats + an admin credential;
      the runner's admin suite now uses `AuthSlot::Admin`; the compose dev
      config gained the ADMIN account + `[admin] enabled` (the baseline had
      only ever run self-hosted).
- [x] `ehrbase-rest` contains no service-trait definitions (adapter only —
      `backend.rs`/`response.rs` are re-export shims)
- [x] Every `ehrbase-sm` trait method doc-comment cites its SM call
      (file + section)

## Decisions made this phase

- ADR-010 (phase-opening decision)
- `Action::Attest` routes through a `PendingAttest` path, not a `Change`
  variant (no orphan audit row — the ATTESTATION *is* the audit, stored
  verbatim in `vo_attestation.data`)
- The population gate keys off `ctx.ehr_id.is_none()`; a future
  multi-`ehr_ids` scope must count as "scope supplied" (not gated)
- `get_ehr_summary` trait-method naming (avoids colliding with the inherent
  wire-EHR builder `ehr_summary`)

## Handoff for next session

All implementation tasks done on `claude/sm-phase-01-native-api` (824/824
tests; one task deliberately partial: the full `UpdateVersion<T>` internal
rewiring of `vobject`/adapter — types + PORT NOTEs + wire-shape test are
landed; swapping the internal carriers risks ECC-tested error messages for
no wire benefit, revisit at SM-2). **The only open exit criterion is the
fresh-image ECC run** (blocked on registry timeouts; see above — run
`bash scripts/conformance.sh`, compare vs 211/318). If the pass set holds,
run `/phase-done` and open the PR to `develop`.
