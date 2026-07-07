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
- [~] 3. master06 (EHR + EHR_STATUS, **21/21 done**), master09 (DIRECTORY,
      **11/11 core done**), master05 (query provisioning, **3 done**), master04
      (OPT provisioning, **upload-valid + invalid + list done**) — all
      fixture-driven, green e2e against the self-hosted SUT. master07/08/11/
      content/admin/demographic/SIGN modules scaffolded (design §4.1 layout) with
      empty `entries()`. Also landed: `fixtures.rs` — typed read-only access to
      the ENTIRE vendored `test_data_sets` corpus (valid + invalid, every
      category) + the RM-1.2.0 `EHR_STATUS` overlay (§6) + the
      `composition_validation_lib` typed mutators (`content/mutate.rs`); corpus
      pinned by a guard test.
- [~] 4. master04/05 (DEFINITION, 22) and master08 (CONTRIBUTION, 31).
      **master07 (COMPOSITION): 28/31 transcribed** (the 3 `has_composition-*`
      cases have no ITS-REST endpoint → `NotYetTranscribed`); event/persistent
      create + read round-trips run under JSON **and** XML. **master08
      (CONTRIBUTION): 22/31 transcribed** (`has_contribution-*` ×4 +
      `list_contributions-*` ×5 have no endpoint → `NotYetTranscribed`).
      Both fixture-driven (compositions `CANONICAL_JSON`/`CANONICAL_XML`,
      contributions `valid` + `invalid`, the OPTs they reference) and green
      e2e against the self-hosted SUT except the recorded `F-open-3..8`
      findings.
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
- **F-open-2 (upload_opt, invalid OPT):** of the 18 vendored invalid `.opt`
  templates, 12 are rejected — 6 invalid OPTs (e.g. `alien_tags`,
  `multiple_elements` duplicate template-id/concept/definition,
  `removed_mandatory_elements`) are accepted by OPT 1.4 upload. CNF master04
  `upload_opt-invalid_opt` requires rejection. Needs an `F-AA-NN` + a stricter
  OPT ingest validation pass.

### master07 (COMPOSITION) — surfaced by the transcribed cases

- **F-open-3 (create/update_composition, mandatory RM attribute not enforced):**
  a COMPOSITION missing a mandatory RM attribute (`composer` [1]) is **accepted**
  with `201` on commit. The commit path validates `data` as a raw `Value`
  (`validate_composition_for_commit` → `openehr_flat::validate_rm_and_terminology`
  + template conformance), which does not enforce mandatory-attribute *presence*
  (that would come from typed `Composition` deserialization, which the commit
  path never performs). Surfaces master07 `create_composition-invalid_event`'s
  intent and master08 `commit_contribution-invalid_composition`/
  `two_commits_second_invalid` (below). CNF master07 §create_composition-invalid_*
  + RM `COMPOSITION` invariants require rejection (`composition_create.yaml` 422).
- **F-open-4 (update_composition-wrong_template):** updating an existing event
  COMPOSITION with a body referencing a **different** `template_id`
  (`persistent_minimal.en.v1` over `nested.en.v1`) is accepted with `200`. CNF
  master07 `update_composition-wrong_template` requires rejection on the
  `template_id` mismatch (`composition_update.yaml` 422). No template-continuity
  check on update.
- **F-open-5 (create_composition-same_opt_twice):** a second `create` for the
  same persistent OPT in one EHR is accepted with `201` (persistent
  single-instance not enforced). CNF master07 `create_composition-same_opt_twice`
  expects a negative response. NOTE: the schedule itself flags this as
  spec-ambiguous ("under debate in the openEHR SEC … lack of information in the
  openEHR specifications; some implementations permit … and some others not"), so
  this is a divergence from the CNF case's *stated* criterion, recorded honestly
  rather than weakened away.
- **F-open-6 (get_versioned_composition, XML):** `GET versioned_composition` with
  `Accept: application/xml` returns `406` ("canonical XML for this response is
  available once typed payloads land (P12)"). Canonical XML is a claimed
  STANDARD-profile data format; `VERSIONED_COMPOSITION` has no canonical-XML
  serializer yet. The JSON variant passes. CNF master07
  `get_versioned_composition` under XML requires `200`
  (`versioned_composition_get.yaml`).

### master08 (CONTRIBUTION) — surfaced by the transcribed cases

- **F-open-3 (shared):** `commit_contribution-invalid_composition` and
  `-two_commits_second_invalid` are accepted with `201` — same root cause as
  F-open-3 (a COMPOSITION VERSION missing a mandatory RM attribute is not
  rejected on the CONTRIBUTION commit path). CNF master08 C.2/C.8 require
  rejection (and C.8 requires the whole commit to fail atomically).
- **F-open-7 (commit_contribution-ehr_status_invalid_change_type):** a
  CONTRIBUTION with a `VERSION<EHR_STATUS>` whose `change_type = 249|creation|`
  is accepted with `201` even though the EHR already has its (mandatory,
  singleton) `EHR_STATUS`. CNF master08 D.3 requires rejection ("the `EHR_STATUS`
  already existing for the EHR"); RM `EHR.ehr_status` is `[1]`. The commit path
  does not reject a second `EHR_STATUS` creation.
- **F-open-8 (commit_contribution-fail_create_existing_directory):** a
  CONTRIBUTION creating a directory (`VERSION<FOLDER>`, `change_type =
  creation`) when the EHR already has a root directory is accepted with `201`.
  CNF master08 E.2 requires rejection ("wrong `change_type` because the root
  `FOLDER` already exists"). The dedicated `directory_create` endpoint returns
  `409` for this, so the CONTRIBUTION path is inconsistent with it.

All eight `F-open-*` are surfaced by the runner asserting the ITS-REST/CNF
expectation, never fabricated and never weakened to green a run (design §4.5).

## Handoff for next session

Steps 1–2 landed: the crate parses the schedule (323 raw / 322 identified / 57
placeholders / 1 duplicate), classifies every id (coverage guard + committed
`inventory/schedule-cases.txt` snapshot), runs a SUT (external + self-hosted),
and generates the report set — proven end-to-end by the one implemented case
`I_EHR_SERVICE.create_ehr-main` (16/16 data sets). Next: step 3 — transcribe the
rest of master06 then master07, growing the registry and shrinking the not-yet
count. Failures are findings (`F-AA-NN`), never exclusions.
