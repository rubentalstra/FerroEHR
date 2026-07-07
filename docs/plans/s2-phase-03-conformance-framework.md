# Phase S2-03 — openEHR CNF Conformance Framework

- Status: in-progress
- Started: 2026-07-07   Owner: —
- Consumes (spec/layer): openEHR CNF Platform Conformance Test Schedule
  (`docs/specs/openehr/CNF/`); ITS-REST 1.0.3; AQL 1.1 (ADR-008 acceptance
  instrument)
- Compile required: yes (compiling, tested increments)

## Objectives

Build the ADR-008 acceptance instrument: a runner that executes the official
openEHR Platform Conformance Test Schedule against a running server, a committed
per-test-case results matrix, and a generated, honestly-scoped Conformance
Statement. Binding design: `docs/design/conformance-framework.md` (v2).

## Preconditions

- [x] Access-control (RBAC/ABAC) + version-signing implemented (the STANDARD
      profile's Signing capability is reachable).
- [x] Seed scaffold `crates/ehrbase-conformance` (Cargo.toml + lib.rs + module
      seams).

## Scope

In: the `ehrbase-conformance` crate (case model, schedule parser, registry +
coverage guard, SUT client + modes, assertions, report generation, CLI,
`scripts/conformance.sh`), the transcribed schedule cases (grown
chapter-by-chapter), the runner-defined `SIGN-*` cases, and the generated
`docs/conformance/` artifacts.

Out: CI wiring (the two tiers — done by the orchestrator after review);
FLAT/STRUCTURED/EhrScape (not CNF-gated — `openehr-flat` suite); benchmarks
(P20).

## Tasks (design §8)

- [x] 1. Crate scaffold + case model + registry + coverage guard + inventory
      snapshot (the honest zero state: every schedule id classified, 0/322
      implemented).
- [x] 2. SUT client + modes (External / SelfHosted) + CLI + `scripts/conformance.sh`;
      prove both modes with one hand-picked case
      (`I_EHR_SERVICE.create_ehr-main`) end-to-end incl. RESULTS.md / badge.
- [~] 3. master06 (EHR + EHR_STATUS, **21/21 done**, fixture-driven, green e2e);
      master07 (COMPOSITION, 31) still to do. Also landed: `fixtures.rs` — typed
      read-only access to the ENTIRE vendored `test_data_sets` corpus (valid +
      invalid, every category) + the RM-1.2.0 `EHR_STATUS` overlay (§6), corpus
      pinned by a guard test.
- [ ] 4. master04/05 (DEFINITION, 22) and master08 (CONTRIBUTION, 31).
- [ ] 5. master09 (DIRECTORY, 37) — completes the CORE + directory surface.
- [ ] 6. Query: master11's real cases + the `QUERY-FIXTURE-*` corpus with
      golden-result diffing.
- [ ] 7. Content chapters (master15–17, 119) — table-driven against the
      validation service.
- [ ] 8. OPTIONS chapters we implement (master12 admin subset, master10
      demographic) + the `SIGN-*` capability cases; wire the two CI tiers;
      first committed `docs/conformance/` + README badge.
- [ ] 9. The first STANDARD-profile Conformance Statement (REST, JSON + XML, RM
      1.2.0; RBAC on, ABAC off; deviations register).

## Exit criteria

- [ ] The CNF schedule passes with documented exceptions only; the deviation
      register is complete (the phase-19 finish line, ADR-008).
- [ ] `docs/conformance/` (results.json, RESULTS.md, CONFORMANCE_STATEMENT.md,
      badge.json) is generated from a run and committed.

## Decisions made this phase

- The `master03` documentation-template heading (angle-bracket id) is dropped
  from the inventory (322 real cases from 323 raw heading matches); the 57
  `aaaa`/`bbbb` placeholders are `Excluded(UpstreamPlaceholder)`; the one real
  duplicate (`CONT-DV_TEXT-validate_open`, master17.2) keeps its first
  occurrence and keys the second `…#2` → `Excluded(UpstreamDuplicate)`.
- master10/12/13 are 100% placeholder headings upstream — they carry no real
  transcribable ids (so the demographic/admin/messaging capabilities are
  entirely runner-defined or OPTIONS-scoped work, not schedule transcription).
- No `I_DEFINITION_ADL2` ids exist in the current vendored schedule, so the
  `Adl2Returns501` exclusion rule matches zero cases today (kept for a future
  re-vendor).
- The fixture-derived negative case
  `FIXTURE-I_EHR_SERVICE.create_ehr-invalid_status` is a **FixtureDerived**
  case (outside the 322 schedule inventory by design §3.4); the coverage guard
  therefore only requires `Schedule`-provenance registry ids ⊆ inventory.

## Findings (design §4.5 — failures are findings, never exclusions)

- **F-open-1 (create_ehr, invalid `EHR_STATUS`):** of the 11 vendored
  `ehr/invalid` data sets, only 2 are rejected by the SUT — 9 invalid
  `EHR_STATUS` payloads (e.g. missing `_type`, missing/empty subject id or
  namespace, missing `is_modifiable`/`is_queryable`) are accepted with `201`.
  CNF master06 §Test Data Sets (INVALID class 2) requires rejection. Needs an
  `F-AA-NN` write-up + fix in EHR create validation before the CORE claim can
  be made. Surfaced by the runner, not fabricated.

## Handoff for next session

Steps 1–2 landed: the crate parses the schedule (323 raw / 322 identified / 57
placeholders / 1 duplicate), classifies every id (coverage guard + committed
`inventory/schedule-cases.txt` snapshot), runs a SUT (external + self-hosted),
and generates the report set — proven end-to-end by the one implemented case
`I_EHR_SERVICE.create_ehr-main` (16/16 data sets). Next: step 3 — transcribe the
rest of master06 then master07, growing the registry and shrinking the not-yet
count. Failures are findings (`F-AA-NN`), never exclusions.
