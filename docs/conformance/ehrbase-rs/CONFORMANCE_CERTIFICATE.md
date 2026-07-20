# ehrbase-rs — Conformance Certificate (generated)

## Assessment basis (read first)

- **Assessor:** self-assessment via the ehrbase-rs Conformance Catalogue (ECC) framework
- **This is NOT an official openEHR conformance certification.** No official openEHR certification program exists; this artefact is a self-assessment produced by an independent framework.
- **Machine-computed:** every verdict below is a pure function of the attached run (`results.json`) — never hand-asserted.
- **ECC framework version:** 3.4.0 · catalogue `inventory/ecc-catalog.tsv`
- **Machine record:** `results.json` (this directory)
- **Run date:** 2026-07-20T20:46:30.858894Z

## System Under Test (SUT)

| | |
|---|---|
| Solution | ehrbase-rs ehrbase-rs 3.4.0 @ `http://localhost:8080/ehrbase/rest/openehr/v1` |
| Vendor | ehrbase-rs |
| Assessor | self-assessment via the ehrbase-rs Conformance Catalogue (ECC) framework |
| Infrastructure | reference corpus openEHR/specifications-CNF@33251d2a; SUT auth mode basic |
| Date | 2026-07-20T20:46:30.858894Z |

## Scope of Test

| | |
|---|---|
| Functional | Core (PASS), Standard (PASS), Options (OBTAINED) |
| Sec & Priv | Signing pass, Anonymous EHRs pass |
| Ext Data Fmt | json, xml |

### Scope exclusions (adjudicated not-applicable)

The following capabilities are excluded from this claim per the committed fairness register (adjudicated extensions / RM-version-sensitive comparisons); the claim is scoped to the applicable capabilities.

- **ECC-ADM-007** Admin list contributions — NativeApiOnly: I_ADMIN_SERVICE.list_contributions is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §list_contributions (TBD stub); SM I_ADMIN_SERVICE.list_contributions — no ITS-REST admin route)_
- **ECC-ADM-008** Admin contribution count — NativeApiOnly: I_ADMIN_SERVICE.contribution_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §contribution_count (TBD stub); SM I_ADMIN_SERVICE.contribution_count — no ITS-REST admin route)_
- **ECC-ADM-009** Admin versioned composition count — NativeApiOnly: I_ADMIN_SERVICE.versioned_composition_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §versioned_composition_count (TBD stub); SM I_ADMIN_SERVICE.versioned_composition_count — no ITS-REST admin route)_
- **ECC-ADM-010** Admin composition version count — NativeApiOnly: I_ADMIN_SERVICE.composition_version_count is exercised by app/ehrbase/tests/service_contribution.rs::contribution_listing_count_and_ehr_summary — no ITS-REST admin route reaches it _(cite: CNF master12 §composition_version_count (TBD stub); SM I_ADMIN_SERVICE.composition_version_count — no ITS-REST admin route)_
- **ECC-ADM-011** Admin export EHRs (dump/load) — NativeApiOnly: I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs is exercised by app/ehrbase/tests/service_dump_load.rs::export_then_load_into_fresh_db_round_trips_byte_equal — no ITS-REST admin route reaches it _(cite: CNF master12 §export_ehrs (TBD stub); SM I_ADMIN_DUMP_LOAD.export_ehrs/load_ehrs — no ITS-REST admin route)_
- **ECC-ADM-012** Admin archive EHRs — NativeApiOnly: I_ADMIN_ARCHIVE.archive_ehrs is exercised by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged — no ITS-REST admin route reaches it _(cite: CNF master12 §archive_ehrs (TBD stub); SM I_ADMIN_ARCHIVE.archive_ehrs — no ITS-REST admin route)_
- **ECC-ADM-013** Admin physical party delete — NoRestBinding: I_ADMIN_SERVICE.physical_party_delete has no ITS-REST route and acts on the demographic extension; exercised natively by app/ehrbase/tests/service_admin.rs::physical_party_delete_cascades_relationships_and_spares_partner _(cite: CNF master12 §physical_party_delete (TBD stub); SM I_ADMIN_SERVICE.physical_party_delete acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-ADM-014** Admin archive parties — NoRestBinding: I_ADMIN_ARCHIVE.archive_parties has no ITS-REST route and acts on the demographic extension; the archive path is proven natively by app/ehrbase/tests/service_admin.rs::archive_marks_vos_idempotently_and_reads_stay_unchanged _(cite: CNF master12 §archive_parties (TBD stub); SM I_ADMIN_ARCHIVE.archive_parties acts on demographic PARTYs (ehrbase-rs demographic extension) — no ITS-REST admin route)_
- **ECC-MSG-001** EHR Extract — export whole EHR (export_ehrs) — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs; RM EHR Extract IM (X_VERSIONED_*); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub, listed twice — authoring duplicate))_
- **ECC-MSG-002** EHR Extract — spec-driven export (export_ehr_extracts) — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehr_extracts (EXTRACT_ENTITY_MANIFEST + EXTRACT_VERSION_SPEC); CNF master13 §I_EHR_EXTRACT.export_ehr_extracts (TBD stub))_
- **ECC-MSG-003** EHR Extract — export of unknown EHR fails — NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.export_ehrs (ehr_id_does_not_exist precondition); CNF master13 §I_EHR_EXTRACT.export_ehr (TBD stub))_
- **ECC-MSG-004** EHR Extract — import whole-EHR clone reusing source id — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr; RM common master06 §Copying Case 1 (reuse source EHR identifier); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-005** EHR Extract — import whole EHR into a caller-fixed id — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (same patient in another EHR service); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-006** EHR Extract — import into a duplicate target id fails — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr (ehr_create_fail_duplicate_id); RM common master06 §Copying; CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-007** EHR Extract — import extract into an existing EHR — NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import — Messaging has no ITS-REST binding _(cite: SM I_EHR_EXTRACT_SERVICE.import_ehr_extract; RM common master06 §Copying Case 2 (first receipt clones VERSIONED_OBJECT; re-import is a conflict); CNF master13 (import subsection absent — RM-backed))_
- **ECC-MSG-008** TDD — import a TDD as a committed COMPOSITION — NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd; TDD → COMPOSITION over OPT/WebTemplate (openehr_flat::tdd::from_tdd); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-009** TDD — import rejects malformed / non-TDD / unknown EHR / unknown template — NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdd (typed envelope rejections); CNF master13 §I_TDD.import_tdd (TBD stub))_
- **ECC-MSG-010** TDD — batch import commits all, fail-fast on error — NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding _(cite: SM I_TDD_SERVICE.import_tdds; CNF master13 §I_TDD.import_tdds (TBD stub))_

## Detailed Test Report

One row per ECC case. *Conformance point* is the CNF-schedule `<SERVICE>.<operation>` trace where the case concretizes one, else the ITS-REST binding (an ECC-original case is never presented as schedule-conformant — see the report's ECC-original section). Results are per data format; a format not run shows `—`. (There is no protobuf technology under test — the CNF template's protobuf column is omitted.)

| openEHR Component | Capability | Conformance point | Test Case | JSON | XML |
|---|---|---|---|---|---|
| EHR service | EhrOperations | I_EHR_SERVICE.has_ehr-existing_ehr_id (master06 §has_ehr) | ECC-EHR-001 — EHR existence check — existing EHR id | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.has_ehr-existing_subject_id (master06 §has_ehr) | ECC-EHR-002 — EHR existence check — existing subject id | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.has_ehr-non_existing_ehr_id (master06 §has_ehr) | ECC-EHR-003 — EHR existence check — non existing EHR id | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.has_ehr-non_existing_subject_id (master06 §has_ehr) | ECC-EHR-004 — EHR existence check — non existing subject id | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.create_ehr-main (master06 §create_ehr) | ECC-EHR-005 — Create EHR — main (valid data-set matrix) | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.create_ehr-same_ehr_twice (master06 §create_ehr) | ECC-EHR-006 — Create EHR — same EHR twice | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.create_ehr-two_ehrs_same_patient (master06 §create_ehr) | ECC-EHR-007 — Create EHR — two EHRs same patient | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.get_ehr-existing_ehr_by_ehr_id (master06 §get_ehr) | ECC-EHR-008 — Get EHR — existing EHR by EHR id | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.get_ehr-existing_ehr_by_subject_id (master06 §get_ehr) | ECC-EHR-009 — Get EHR — existing EHR by subject id | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_ehr_id (master06 §get_ehr) | ECC-EHR-010 — Get EHR — get EHR by invalid EHR id | pass | — |
| EHR service | EhrOperations | I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_subject_id (master06 §get_ehr) | ECC-EHR-011 — Get EHR — get EHR by invalid subject id | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.get_ehr_status-get_by_ehr_id (master06 §get_ehr_status) | ECC-STA-001 — Get EHR_STATUS — get by EHR id | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.get_ehr_status-bad_ehr (master06 §get_ehr_status) | ECC-STA-002 — Get EHR_STATUS — bad EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.set_ehr_queryable-existing_ehr (master06 §set_ehr_queryable) | ECC-STA-003 — Set EHR_STATUS is_queryable — existing EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.set_ehr_queryable-bad_ehr (master06 §set_ehr_queryable) | ECC-STA-004 — Set EHR_STATUS is_queryable — bad EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.set_ehr_modifiable-existing_ehr (master06 §set_ehr_modifiable) | ECC-STA-005 — Set EHR_STATUS is_modifiable — existing EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.set_ehr_modifiable-bad_ehr (master06 §set_ehr_modifiable) | ECC-STA-006 — Set EHR_STATUS is_modifiable — bad EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.clear_ehr_queryable-existing_ehr (master06 §clear_ehr_queryable) | ECC-STA-007 — Clear EHR_STATUS is_queryable — existing EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.clear_ehr_queryable-bad_ehr (master06 §clear_ehr_queryable) | ECC-STA-008 — Clear EHR_STATUS is_queryable — bad EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.clear_ehr_modifiable-existing_ehr (master06 §clear_ehr_modifiable) | ECC-STA-009 — Clear EHR_STATUS is_modifiable — existing EHR | pass | — |
| EHR_STATUS | EhrStatus | I_EHR_STATUS.clear_ehr_modifiable-bad_ehr (master06 §clear_ehr_modifiable) | ECC-STA-010 — Clear EHR_STATUS is_modifiable — bad EHR | pass | — |
| EHR service | EhrOperations | POST /ehr | ECC-EHR-012 — Create EHR — reject invalid EHR_STATUS data sets | pass | — |
| EHR service | AnonymousEhrs | POST /ehr | ECC-EHR-013 — Create anonymous (subject-less) EHR | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.create_composition-event (master07 §create_composition) | ECC-COM-001 — Create composition — event | pass | pass |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.create_composition-persistent (master07 §create_composition) | ECC-COM-002 — Create composition — persistent | pass | pass |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.create_composition-same_opt_twice (master07 §create_composition) | ECC-COM-003 — Create composition — same OPT twice | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.create_composition-invalid_event (master07 §create_composition) | ECC-COM-004 — Create composition — invalid event | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.create_composition-invalid_persistent (master07 §create_composition) | ECC-COM-005 — Create composition — invalid persistent | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.create_composition-event_bad_opt (master07 §create_composition) | ECC-COM-006 — Create composition — event bad OPT | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.create_composition-event_bad_ehr (master07 §create_composition) | ECC-COM-007 — Create composition — event bad EHR | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.has_composition (master07 §has_composition) | ECC-COM-032 — Composition existence check — existing composition | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.has_composition-bad_composition (master07 §has_composition) | ECC-COM-011 — Composition existence check — bad composition | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.has_composition-bad_ehr (master07 §has_composition) | ECC-COM-012 — Composition existence check — bad EHR | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_latest (master07 §get_composition_latest) | ECC-COM-008 — Get latest composition | pass | pass |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_latest-bad_composition (master07 §get_composition_latest) | ECC-COM-009 — Get latest composition — bad composition | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_latest-bad_ehr (master07 §get_composition_latest) | ECC-COM-010 — Get latest composition — bad EHR | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_at_time (master07 §get_composition_at_time) | ECC-COM-013 — Get composition at time | pass | pass |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_at_time-no_time_arg (master07 §get_composition_at_time) | ECC-COM-014 — Get composition at time — no time arg | pass | pass |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_at_time-bad_composition (master07 §get_composition_at_time) | ECC-COM-015 — Get composition at time — bad composition | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_at_time-bad_ehr (master07 §get_composition_at_time) | ECC-COM-016 — Get composition at time — bad EHR | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_at_times (master07 §get_composition_at_time) | ECC-COM-017 — Get composition at multiple times | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_version (master07 §get_composition_version) | ECC-COM-018 — Get composition version | pass | pass |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_version-bad_version (master07 §get_composition_version) | ECC-COM-019 — Get composition version — bad version | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_version-bad_ehr (master07 §get_composition_version) | ECC-COM-020 — Get composition version — bad EHR | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.get_composition_versions (master07 §get_composition_version) | ECC-COM-021 — Get composition versions | pass | — |
| COMPOSITION | Versioning | I_EHR_COMPOSITION.get_versioned_composition (master07 §get_versioned_composition) | ECC-COM-022 — Get versioned composition | pass | pass |
| COMPOSITION | Versioning | I_EHR_COMPOSITION.get_versioned_composition-non_existent (master07 §get_versioned_composition) | ECC-COM-023 — Get versioned composition — non existent | pass | — |
| COMPOSITION | Versioning | I_EHR_COMPOSITION.get_versioned_composition-bad_ehr (master07 §get_versioned_composition) | ECC-COM-024 — Get versioned composition — bad EHR | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.update_composition-event (master07 §update_composition) | ECC-COM-025 — Update composition — event | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.update_composition-persistent (master07 §update_composition) | ECC-COM-026 — Update composition — persistent | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.update_composition-non_existent (master07 §update_composition) | ECC-COM-027 — Update composition — non existent | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.update_composition-wrong_template (master07 §update_composition) | ECC-COM-028 — Update composition — wrong template | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.delete_composition-event (master07 §delete_composition) | ECC-COM-029 — Delete composition — event | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.delete_composition-persistent (master07 §delete_composition) | ECC-COM-030 — Delete composition — persistent | pass | — |
| COMPOSITION | CompositionOps | I_EHR_COMPOSITION.delete_composition-non_existent (master07 §delete_composition) | ECC-COM-031 — Delete composition — non existent | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-valid_composition (master08 §Test Cases) | ECC-CTB-001 — Commit contribution — valid composition | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-invalid_composition (master08 §Test Cases) | ECC-CTB-002 — Commit contribution — invalid composition | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-empty (master08 §Test Cases) | ECC-CTB-003 — Commit contribution — empty | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions (master08 §Test Cases D) | ECC-CTB-004 — Commit contribution — valid invalid compositions | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-non_exiting_opt (master08 §Test Cases) | ECC-CTB-005 — Commit contribution — non exiting OPT | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-event_composition (master08 §Test Cases) | ECC-CTB-006 — Commit contribution — event composition | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-persistent_composition (master08 §Test Cases) | ECC-CTB-007 — Commit contribution — persistent composition | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-delete (master08 §Test Cases) | ECC-CTB-008 — Commit contribution — delete | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_invalid (master08 §Test Cases) | ECC-CTB-009 — Commit contribution — two commits second invalid | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-two_commits_second_creation (master08 §Test Cases) | ECC-CTB-010 — Commit contribution — two commits second creation | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-minimal_ehr_status (master08 §EHR_STATUS CONTRIBUTION Commit) | ECC-CTB-011 — Commit contribution — minimal EHR status | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-full_ehr_status (master08 §EHR_STATUS CONTRIBUTION Commit) | ECC-CTB-012 — Commit contribution — full EHR status | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-ehr_status_invalid_change_type (master08 §EHR_STATUS CONTRIBUTION Commit) | ECC-CTB-013 — Commit contribution — EHR status invalid change type | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-invalid_ehr_status (master08 §EHR_STATUS CONTRIBUTION Commit) | ECC-CTB-014 — Commit contribution — invalid EHR status | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-valid_directory (master08 §FOLDER CONTRIBUTION Commit) | ECC-CTB-015 — Commit contribution — valid directory | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-fail_create_existing_directory (master08 §FOLDER CONTRIBUTION Commit) | ECC-CTB-016 — Commit contribution — fail create existing directory | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-fail_modify_non_existing_directory (master08 §FOLDER CONTRIBUTION Commit) | ECC-CTB-017 — Commit contribution — fail modify non existing directory | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.commit_contribution-update_existing_directory (master08 §FOLDER CONTRIBUTION Commit) | ECC-CTB-018 — Commit contribution — update existing directory | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.get_contribution-existing (master08 §get_contribution) | ECC-CTB-019 — Get contribution — existing | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.get_contribution-empty_ehr (master08 §get_contribution) | ECC-CTB-020 — Get contribution — empty EHR | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.get_contribution-bad_ehr (master08 §get_contribution) | ECC-CTB-021 — Get contribution — bad EHR | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.get_contribution-bad_contribution (master08 §get_contribution) | ECC-CTB-022 — Get contribution — bad contribution | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.has_contribution-existing (master08 §has_contribution) | ECC-CTB-023 — Contribution existence check — existing | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.has_contribution-bad_contribution (master08 §has_contribution) | ECC-CTB-024 — Contribution existence check — bad contribution | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.has_contribution-bad_ehr (master08 §has_contribution) | ECC-CTB-025 — Contribution existence check — bad EHR | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.has_contribution-empty_ehr (master08 §has_contribution) | ECC-CTB-026 — Contribution existence check — empty EHR | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions-empty (master08 §list_contributions) | ECC-CTB-027 — List contributions — empty | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions-non_existing_ehr (master08 §list_contributions) | ECC-CTB-028 — List contributions — non existing EHR | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions-post_commit (master08 §list_contributions) | ECC-CTB-029 — List contributions — post commit | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions-ehr_containing_directory (master08 §list_contributions) | ECC-CTB-030 — List contributions — EHR containing directory | pass | — |
| CONTRIBUTION (change sets) | ChangeSets | I_EHR_CONTRIBUTION.list_contributions-ehr_containing_ehr_status (master08 §list_contributions) | ECC-CTB-031 — List contributions — EHR containing EHR status | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_directory-empty_ehr (master09 §has_directory) | ECC-DIR-012 — Directory existence check — empty EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_directory-ehr_with_directory (master09 §has_directory) | ECC-DIR-013 — Directory existence check — EHR with directory | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_directory-bad_ehr (master09 §has_directory) | ECC-DIR-014 — Directory existence check — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_path-ehr_root_directory (master09 §has_path) | ECC-DIR-015 — Directory path existence check — EHR root directory | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_path-folder_structure (master09 §has_path) | ECC-DIR-016 — Directory path existence check — folder structure | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_path-empty_ehr (master09 §has_path) | ECC-DIR-017 — Directory path existence check — empty EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_path-bad_ehr (master09 §has_path) | ECC-DIR-018 — Directory path existence check — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.create_directory-empty_ehr (master09 §create_directory) | ECC-DIR-001 — Create directory — empty EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.create_directory-ehr_with_directory (master09 §create_directory) | ECC-DIR-002 — Create directory — EHR with directory | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.create_directory-bad_ehr (master09 §create_directory) | ECC-DIR-003 — Create directory — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory-empty_ehr (master09 §get_directory) | ECC-DIR-022 — Get directory — empty EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory-ehr_root_directory (master09 §get_directory) | ECC-DIR-004 — Get directory — EHR root directory | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory-directory_with_structure (master09 §get_directory) | ECC-DIR-023 — Get directory — directory with structure | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory-bad_ehr (master09 §get_directory) | ECC-DIR-005 — Get directory — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory (master09 §get_directory_at_time) | ECC-DIR-006 — Get directory at time — EHR with directory | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-bad_ehr (master09 §get_directory_at_time) | ECC-DIR-007 — Get directory at time — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.update_directory-ehr_with_directory (master09 §update_directory) | ECC-DIR-008 — Update directory — EHR with directory | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.update_directory-bad_ehr (master09 §update_directory) | ECC-DIR-009 — Update directory — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.delete_directory-ehr_with_directory (master09 §delete_directory) | ECC-DIR-010 — Delete directory — EHR with directory | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.delete_directory-bad_ehr (master09 §delete_directory) | ECC-DIR-011 — Delete directory — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_directory_version-empty_ehr (master09 §has_directory_version) | ECC-DIR-019 — Directory version existence check — empty EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_directory_version-directory_with_two_versions (master09 §has_directory_version) | ECC-DIR-020 — Directory version existence check — directory with two versions | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.has_directory_version-bad_ehr (master09 §has_directory_version) | ECC-DIR-021 — Directory version existence check — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_empty_time (master09 §get_directory_at_time) | ECC-DIR-024 — Get directory at time — EHR with directory empty time | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_versions (master09 §get_directory_at_time) | ECC-DIR-025 — Get directory at time — EHR with directory versions | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-ehr_with_directory_versions_empty_time (master09 §get_directory_at_time) | ECC-DIR-026 — Get directory at time — EHR with directory versions empty time | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-empty_ehr (master09 §get_directory_at_time) | ECC-DIR-027 — Get directory at time — empty EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-empty_ehr_empty_time (master09 §get_directory_at_time) | ECC-DIR-028 — Get directory at time — empty EHR empty time | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_time-multiple_versions_first (master09 §get_directory_at_time) | ECC-DIR-029 — Get directory at time — multiple versions first | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_version-bad_ehr (master09 §get_directory_at_version) | ECC-DIR-030 — Get directory at version — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_version-directory_with_two_versions (master09 §get_directory_at_version) | ECC-DIR-031 — Get directory at version — directory with two versions | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.get_directory_at_version-empty_ehr (master09 §get_directory_at_version) | ECC-DIR-032 — Get directory at version — empty EHR | pass | — |
| DIRECTORY (FOLDER) | Versioning | I_EHR_DIRECTORY.get_versioned_directory-empty_ehr (master09 §get_versioned_directory) | ECC-DIR-033 — Get versioned directory — empty EHR | pass | — |
| DIRECTORY (FOLDER) | Versioning | I_EHR_DIRECTORY.get_versioned_directory-directory_with_two_versions (master09 §get_versioned_directory) | ECC-DIR-034 — Get versioned directory — directory with two versions | pass | — |
| DIRECTORY (FOLDER) | Versioning | I_EHR_DIRECTORY.get_versioned_directory-bad_ehr (master09 §get_versioned_directory) | ECC-DIR-035 — Get versioned directory — bad EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.update_directory-empty_ehr (master09 §update_directory) | ECC-DIR-036 — Update directory — empty EHR | pass | — |
| DIRECTORY (FOLDER) | DirectoryOps | I_EHR_DIRECTORY.delete_directory-empty_ehr (master09 §delete_directory) | ECC-DIR-037 — Delete directory — empty EHR | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.validate_opt-valid_opt (master04 §validate_opt) | ECC-TPL-011 — Validate OPT — valid OPT | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.validate_opt-invalid_opt (master04 §validate_opt) | ECC-TPL-012 — Validate OPT — invalid OPT | pass | — |
| Template / OPT provisioning | Adl14ArchetypeProvisioning | I_DEFINITION_ADL14.upload_opt-valid_opt (master04 §upload_opt) | ECC-TPL-001 — Upload OPT — valid OPT (provisions ADL 1.4 archetypes) | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.upload_opt-invalid_opt (master04 §upload_opt) | ECC-TPL-002 — Upload OPT — invalid OPT | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict (master04 §upload_opt) | ECC-TPL-004 — Upload OPT — valid OPT twice conflict | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.upload_opt-valid_opt_twice_no_conflict (master04 §upload_opt) | ECC-TPL-005 — Upload OPT — valid OPT twice no conflict | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.get_opt-retrieve_single (master04 §get_opt) | ECC-TPL-006 — Get OPT — retrieve single | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.get_opt-retrieve_fail (master04 §get_opt) | ECC-TPL-009 — Get OPT — retrieve fail | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.get_opt-retrieve_latest_version (master04 §get_opt) | ECC-TPL-007 — Get OPT — retrieve latest version | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.get_opt-retrieve_specific_version (master04 §get_opt) | ECC-TPL-008 — Get OPT — retrieve specific version | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.get_opts-retrieve_all (master04 §get_opts) | ECC-TPL-010 — List OPTs — retrieve all | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts (master04 §get_opts) | ECC-TPL-003 — List OPTs — retrieve all no OPTs | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | GET /definition/template/adl1.4/{template_id}/example → POST /ehr/{ehr_id}/composition | ECC-TPL-017 — Example COMPOSITION round-trips (ADL 1.4 example → commit) | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt-delete_existing (master04 §delete_opt) | ECC-TPL-014 — Delete OPT — delete existing | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt-delete_latest_version (master04 §delete_opt) | ECC-TPL-015 — Delete OPT — delete latest version | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt-delete_specific_version (master04 §delete_opt) | ECC-TPL-016 — Delete OPT — delete specific version | pass | — |
| Template / OPT provisioning | Adl14OptProvisioning | I_DEFINITION_ADL14.delete_opt-delete_non_existing (master04 §delete_opt) | ECC-TPL-013 — Delete OPT — delete non existing | pass | — |
| Stored-query provisioning | QueryProvisioning | PUT /definition/query/{qualified_query_name}/{version} | ECC-SQR-001 — Store stored query — valid | pass | — |
| Stored-query provisioning | QueryProvisioning | PUT /definition/query/{qualified_query_name}/{version} | ECC-SQR-007 — Store stored query — invalid | pass | — |
| Stored-query provisioning | QueryProvisioning | PUT /definition/query/{qualified_query_name}/{version} | ECC-SQR-006 — Store stored query — bad formalism | pass | — |
| Stored-query provisioning | QueryProvisioning | GET /definition/query/{qualified_query_name}/{version} | ECC-SQR-008 — Stored query existence check — existing | pass | — |
| Stored-query provisioning | QueryProvisioning | GET /definition/query/{qualified_query_name} | ECC-SQR-002 — List stored queries — non empty | pass | — |
| Stored-query provisioning | QueryProvisioning | GET /definition/query | ECC-SQR-004 — List stored queries — empty | pass | — |
| Stored-query provisioning | QueryProvisioning | GET /definition/query | ECC-SQR-005 — List stored queries — select items | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-001 — Query service smoke test | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-002 — Execute ad-hoc AQL query — empty db | pass | — |
| AQL execution | AqlBasic | PUT /definition/query/{name}/{version}; GET /query/{name} | ECC-QRY-003 — Execute stored AQL query — empty db | pass | — |
| AQL execution | AqlBasic | POST /ehr/{ehr_id}/composition; POST /query/aql | ECC-QRY-004 — Execute ad-hoc AQL query — loaded db | pass | — |
| AQL execution | AqlBasic | POST /ehr/{ehr_id}/composition; POST /query/aql | ECC-QRY-025 — AQL uid projection — c/uid/value returns the version id | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-005 — AQL corpus — invalid queries rejected | pass | — |
| AQL execution | AqlAdvanced | POST /query/aql | ECC-QRY-014 — AQL advanced — ORDER BY + LIMIT/OFFSET | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-006 — AQL corpus — A empty db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-007 — AQL corpus — B empty db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-008 — AQL corpus — C empty db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-009 — AQL corpus — D empty db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-010 — AQL corpus — A loaded db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-011 — AQL corpus — B loaded db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-012 — AQL corpus — C loaded db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-013 — AQL corpus — D loaded db | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-015 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-016 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-017 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-018 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-019 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-020 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-021 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-022 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-023 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| AQL execution | AqlBasic | POST /query/aql | ECC-QRY-024 — AQL corpus — dialect-adjudicated query rejected | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_any-context_any (master15 §COMPOSITION Test Cases) | ECC-VAL-001 — Validate COMPOSITION — content card any context any | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_1plus-context_any (master15 §COMPOSITION Test Cases) | ECC-VAL-002 — Validate COMPOSITION — content card 1plus context any | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_3plus-context_any (master15 §COMPOSITION Test Cases) | ECC-VAL-003 — Validate COMPOSITION — content card 3plus context any | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_opt-context_any (master15 §COMPOSITION Test Cases) | ECC-VAL-004 — Validate COMPOSITION — content card OPT context any | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_mand-context_any (master15 §COMPOSITION Test Cases) | ECC-VAL-005 — Validate COMPOSITION — content card mand context any | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_3to5-context_any (master15 §COMPOSITION Test Cases) | ECC-VAL-006 — Validate COMPOSITION — content card 3to5 context any | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_any-context_mand (master15 §COMPOSITION Test Cases) | ECC-VAL-007 — Validate COMPOSITION — content card any context mand | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_1plus-context_mand (master15 §COMPOSITION Test Cases) | ECC-VAL-008 — Validate COMPOSITION — content card 1plus context mand | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_3plus-context_mand (master15 §COMPOSITION Test Cases) | ECC-VAL-009 — Validate COMPOSITION — content card 3plus context mand | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_opt-context_mand (master15 §COMPOSITION Test Cases) | ECC-VAL-010 — Validate COMPOSITION — content card OPT context mand | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_mand-context_mand (master15 §COMPOSITION Test Cases) | ECC-VAL-011 — Validate COMPOSITION — content card mand context mand | pass | — |
| Content / archetype validation | ArchetypeValidation | CONT-COMP.content_card_3to5-context_mand (master15 §COMPOSITION Test Cases) | ECC-VAL-012 — Validate COMPOSITION — content card 3to5 context mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-OBS-state_ex_opt-protocol_ex_opt (master16 §OBSERVATION Test Cases) | ECC-VAL-013 — Validate OBSERVATION — state ex OPT protocol ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-OBS-state_ex_opt-protocol_ex_mand (master16 §OBSERVATION Test Cases) | ECC-VAL-014 — Validate OBSERVATION — state ex OPT protocol ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-OBS-state_ex_mand-protocol_ex_opt (master16 §OBSERVATION Test Cases) | ECC-VAL-015 — Validate OBSERVATION — state ex mand protocol ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-OBS-state_ex_mand-protocol_ex_mand (master16 §OBSERVATION Test Cases) | ECC-VAL-016 — Validate OBSERVATION — state ex mand protocol ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_any-summary_ex_opt (master16 §HISTORY Test Cases) | ECC-VAL-017 — Validate HISTORY — events card any summary ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_1plus-summary_ex_opt (master16 §HISTORY Test Cases) | ECC-VAL-018 — Validate HISTORY — events card 1plus summary ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_3plus-summary_ex_opt (master16 §HISTORY Test Cases) | ECC-VAL-019 — Validate HISTORY — events card 3plus summary ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_opt-summary_ex_opt (master16 §HISTORY Test Cases) | ECC-VAL-020 — Validate HISTORY — events card OPT summary ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_mand-summary_ex_opt (master16 §HISTORY Test Cases) | ECC-VAL-021 — Validate HISTORY — events card mand summary ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_3to5-summary_ex_opt (master16 §HISTORY Test Cases) | ECC-VAL-022 — Validate HISTORY — events card 3to5 summary ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_any-summary_ex_mand (master16 §HISTORY Test Cases) | ECC-VAL-023 — Validate HISTORY — events card any summary ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_1plus-summary_ex_mand (master16 §HISTORY Test Cases) | ECC-VAL-024 — Validate HISTORY — events card 1plus summary ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_3plus-summary_ex_mand (master16 §HISTORY Test Cases) | ECC-VAL-025 — Validate HISTORY — events card 3plus summary ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_opt-summary_ex_mand (master16 §HISTORY Test Cases) | ECC-VAL-026 — Validate HISTORY — events card OPT summary ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_mand-summary_ex_mand (master16 §HISTORY Test Cases) | ECC-VAL-027 — Validate HISTORY — events card mand summary ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-HIST-events_card_3to5-summary_ex_mand (master16 §HISTORY Test Cases) | ECC-VAL-028 — Validate HISTORY — events card 3to5 summary ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-EVENT-state_ex_opt (master16 §EVENT Test Cases) | ECC-VAL-029 — Validate EVENT — state ex OPT | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-EVENT-state_ex_mand (master16 §EVENT Test Cases) | ECC-VAL-030 — Validate EVENT — state ex mand | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-EVENT-type_any (master16 §EVENT Test Cases) | ECC-VAL-031 — Validate EVENT — type any | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-EVENT-type_point_event (master16 §EVENT Test Cases) | ECC-VAL-032 — Validate EVENT — type point event | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-EVENT-type_interval_event (master16 §EVENT Test Cases) | ECC-VAL-033 — Validate EVENT — type interval event | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-ITEM_STR-type_any (master16 §ITEM_STRUCTURE Test Cases) | ECC-VAL-034 — Validate ITEM_STRUCTURE — type any | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-ITEM_STR-type_item_tree (master16 §ITEM_STRUCTURE Test Cases) | ECC-VAL-035 — Validate ITEM_STRUCTURE — type item tree | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-ITEM_STR-type_item_list (master16 §ITEM_STRUCTURE Test Cases) | ECC-VAL-036 — Validate ITEM_STRUCTURE — type item list | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-ITEM_STR-type_item_table (master16 §ITEM_STRUCTURE Test Cases) | ECC-VAL-037 — Validate ITEM_STRUCTURE — type item table | pass | — |
| Content / archetype validation | ArchetypeValidation | ENTRY.CONT-ITEM_STR-type_item_single (master16 §ITEM_STRUCTURE Test Cases) | ECC-VAL-038 — Validate ITEM_STRUCTURE — type item single | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_BOOLEAN.anything_allowed (master17.1 §DV_BOOLEAN) | ECC-VAL-039 — Validate DV_BOOLEAN — anything allowed | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_BOOLEAN.only_true_allowed (master17.1 §DV_BOOLEAN) | ECC-VAL-040 — Validate DV_BOOLEAN — only true allowed | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_BOOLEAN.only_false_allowed (master17.1 §DV_BOOLEAN) | ECC-VAL-041 — Validate DV_BOOLEAN — only false allowed | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_IDENTIFIER.validate_all_pattern (master17.1 §DV_IDENTIFIER) | ECC-VAL-042 — Validate DV_IDENTIFIER — all pattern | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_IDENTIFIER.validate_all_list (master17.1 §DV_IDENTIFIER) | ECC-VAL-043 — Validate DV_IDENTIFIER — all list | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_TEXT.validate_open (master17.2 §DV_TEXT; heading duplicated, 2nd C_STRING.pattern table folded) | ECC-VAL-044 — Validate DV_TEXT — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_TEXT.validate_list (master17.2 §DV_TEXT) | ECC-VAL-045 — Validate DV_TEXT — list | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_CODED_TEXT.validate_open (master17.2 §DV_CODED_TEXT) | ECC-VAL-046 — Validate DV_CODED_TEXT — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_CODED_TEXT.validate_local_codes (master17.2 §DV_CODED_TEXT) | ECC-VAL-047 — Validate DV_CODED_TEXT — local codes | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_CODED_TEXT.validate_ext_term (master17.2 §DV_CODED_TEXT; direct C_CODE_PHRASE substitutes the CONSTRAINT_REF binding path) | ECC-VAL-048 — Validate DV_CODED_TEXT — ext term | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_ORDINAL.validate_open (master17.3 §DV_ORDINAL) | ECC-VAL-049 — Validate DV_ORDINAL — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_ORDINAL.validate_constraint (master17.3 §DV_ORDINAL) | ECC-VAL-050 — Validate DV_ORDINAL — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_SCALE.validate_open (master17.3 §DV_SCALE; RM ≥ 1.1.0) | ECC-VAL-051 — Validate DV_SCALE — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_SCALE.validate_constraint (master17.3 §DV_SCALE; RM ≥ 1.1.0, C_REAL substitute) | ECC-VAL-052 — Validate DV_SCALE — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_COUNT.validate_open (master17.3 §DV_COUNT) | ECC-VAL-053 — Validate DV_COUNT — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_COUNT.validate_range (master17.3 §DV_COUNT) | ECC-VAL-054 — Validate DV_COUNT — range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_COUNT.validate_list (master17.3 §DV_COUNT) | ECC-VAL-055 — Validate DV_COUNT — list | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_QUANTITY.validate_open (master17.3 §DV_QUANTITY) | ECC-VAL-056 — Validate DV_QUANTITY — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_QUANTITY.validate_property (master17.3 §DV_QUANTITY) | ECC-VAL-057 — Validate DV_QUANTITY — property | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_QUANTITY.validate_property_units (master17.3 §DV_QUANTITY) | ECC-VAL-058 — Validate DV_QUANTITY — property units | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_QUANTITY.validate_property_units_mag (master17.3 §DV_QUANTITY) | ECC-VAL-059 — Validate DV_QUANTITY — property units mag | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_open (master17.3 §DV_PROPORTION; 14 kind-invariant rejects untested, RM-mandatory numerator only) | ECC-VAL-060 — Validate DV_PROPORTION — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_ratio (master17.3 §DV_PROPORTION) | ECC-VAL-061 — Validate DV_PROPORTION — ratio | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_unitary (master17.3 §DV_PROPORTION) | ECC-VAL-062 — Validate DV_PROPORTION — unitary | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_percent (master17.3 §DV_PROPORTION) | ECC-VAL-063 — Validate DV_PROPORTION — percent | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_fraction (master17.3 §DV_PROPORTION) | ECC-VAL-064 — Validate DV_PROPORTION — fraction | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_integer_fraction (master17.3 §DV_PROPORTION) | ECC-VAL-065 — Validate DV_PROPORTION — integer fraction | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_any_fraction (master17.3 §DV_PROPORTION) | ECC-VAL-066 — Validate DV_PROPORTION — any fraction | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PROPORTION.validate_ratio_range (master17.3 §DV_PROPORTION; denominator C_REAL.range table not driven) | ECC-VAL-067 — Validate DV_PROPORTION — ratio range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_COUNT.validate_open (master17.3 §DV_INTERVAL<DV_COUNT>; bound C_INTEGER constraint inexpressible → RM Interval invariant triple) | ECC-VAL-068 — Validate DV_INTERVAL<DV_COUNT> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_COUNT.validate_lower_upper (master17.3; bound constraint inexpressible → RM lower ≤ upper) | ECC-VAL-069 — Validate DV_INTERVAL<DV_COUNT> — lower upper | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_COUNT.validate_lower_upper_list (master17.3; C_INTEGER.list on bounds inexpressible → RM lower ≤ upper) | ECC-VAL-070 — Validate DV_INTERVAL<DV_COUNT> — lower upper list | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_QUANTITY.validate_open (master17.3; bound C_DV_QUANTITY.list inexpressible → RM Interval invariant triple) | ECC-VAL-071 — Validate DV_INTERVAL<DV_QUANTITY> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_QUANTITY.validate_upper_lower (master17.3; bound constraint inexpressible → RM lower ≤ upper) | ECC-VAL-072 — Validate DV_INTERVAL<DV_QUANTITY> — upper lower | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DATE_TIME.validate_open (master17.3; temporal bound inexpressible → RM Interval invariant triple) | ECC-VAL-073 — Validate DV_INTERVAL<DV_DATE_TIME> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DATE_TIME.validate_lower_upper_constraint (master17.3, 68-row table; C_DATE_TIME bounds inexpressible → RM lower ≤ upper) | ECC-VAL-074 — Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DATE_TIME.validate_lower_upper_range (master17.3; C_DATE_TIME.range bounds inexpressible → RM lower ≤ upper) | ECC-VAL-075 — Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DATE.validate_open (master17.3; temporal bound inexpressible → RM Interval invariant triple) | ECC-VAL-076 — Validate DV_INTERVAL<DV_DATE> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DATE.validate_lower_upper_constraint (master17.3; C_DATE bounds inexpressible → RM lower ≤ upper) | ECC-VAL-077 — Validate DV_INTERVAL<DV_DATE> — lower upper constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DATE.validate_lower_upper_range (master17.3; C_DATE.range bounds inexpressible → RM lower ≤ upper) | ECC-VAL-078 — Validate DV_INTERVAL<DV_DATE> — lower upper range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_TIME.validate_open (master17.3; temporal bound inexpressible → RM Interval invariant triple) | ECC-VAL-079 — Validate DV_INTERVAL<DV_TIME> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_TIME.validate_lower_upper_constraint (master17.3; C_TIME bounds inexpressible → RM lower ≤ upper) | ECC-VAL-080 — Validate DV_INTERVAL<DV_TIME> — lower upper constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_TIME.validate_lower_upper_range (master17.3; C_TIME.range bounds inexpressible → RM lower ≤ upper) | ECC-VAL-081 — Validate DV_INTERVAL<DV_TIME> — lower upper range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DURATION.validate_open (master17.3; temporal bound inexpressible → RM Interval invariant triple) | ECC-VAL-082 — Validate DV_INTERVAL<DV_DURATION> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DURATION.validate_constraint (master17.3, 35-row table; C_DURATION bounds inexpressible → RM lower ≤ upper) | ECC-VAL-083 — Validate DV_INTERVAL<DV_DURATION> — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_DURATION.validate_range (master17.3; C_DURATION.range bounds inexpressible → RM lower ≤ upper) | ECC-VAL-084 — Validate DV_INTERVAL<DV_DURATION> — range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_ORDINAL.validate_open (master17.3; bound constraint inexpressible → RM Interval invariant triple) | ECC-VAL-085 — Validate DV_INTERVAL<DV_ORDINAL> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_ORDINAL.validate_constraint (master17.3; C_DV_ORDINAL bounds inexpressible → RM lower ≤ upper) | ECC-VAL-086 — Validate DV_INTERVAL<DV_ORDINAL> — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_SCALE.validate_open (master17.3; RM ≥ 1.1.0; bound inexpressible → RM Interval invariant triple) | ECC-VAL-087 — Validate DV_INTERVAL<DV_SCALE> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_SCALE.validate_constraint (master17.3; RM ≥ 1.1.0; bound inexpressible → RM lower ≤ upper) | ECC-VAL-088 — Validate DV_INTERVAL<DV_SCALE> — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_PROPORTION.validate_open (master17.3, 18-row table; bound inexpressible → RM Interval invariant triple) | ECC-VAL-089 — Validate DV_INTERVAL<DV_PROPORTION> — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_PROPORTION.validate_ratio (master17.3, 12-row table; proportion-kind bound inexpressible → RM lower ≤ upper) | ECC-VAL-090 — Validate DV_INTERVAL<DV_PROPORTION> — ratio | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_PROPORTION.validate_unitary (master17.3, 12-row table; proportion-kind bound inexpressible → RM lower ≤ upper) | ECC-VAL-091 — Validate DV_INTERVAL<DV_PROPORTION> — unitary | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_PROPORTION.validate_percentage (master17.3, 12-row table; proportion-kind bound inexpressible → RM lower ≤ upper) | ECC-VAL-092 — Validate DV_INTERVAL<DV_PROPORTION> — percentage | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_PROPORTION.validate_fraction (master17.3, 12-row table; proportion-kind bound inexpressible → RM lower ≤ upper) | ECC-VAL-093 — Validate DV_INTERVAL<DV_PROPORTION> — fraction | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_PROPORTION.validate_integer_fraction (master17.3, 12-row table; proportion-kind bound inexpressible → RM lower ≤ upper) | ECC-VAL-094 — Validate DV_INTERVAL<DV_PROPORTION> — integer fraction | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_INTERVAL_DV_PROPORTION.validate_ratio_range (master17.3, 18-row table; proportion-kind bound inexpressible → RM lower ≤ upper) | ECC-VAL-095 — Validate DV_INTERVAL<DV_PROPORTION> — ratio range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DURATION.validate_open (master17.4 §DV_DURATION) | ECC-VAL-096 — Validate DV_DURATION — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DURATION.validate_fields (master17.4 §DV_DURATION; open finding: temporal enforcement — SUT reject reported, never masked) | ECC-VAL-097 — Validate DV_DURATION — fields | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DURATION.validate_range (master17.4 §DV_DURATION; open finding: temporal enforcement) | ECC-VAL-098 — Validate DV_DURATION — range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DURATION.validate_fields_range (master17.4 §DV_DURATION; open finding: temporal enforcement) | ECC-VAL-099 — Validate DV_DURATION — fields range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_TIME.validate_open (master17.4 §DV_TIME; ISO8601-validity rows not driven, RM-mandatory value only) | ECC-VAL-100 — Validate DV_TIME — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_TIME.validate_constraint (master17.4 §DV_TIME, 70-row table; open finding: temporal enforcement) | ECC-VAL-101 — Validate DV_TIME — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_TIME.validate_range (master17.4 §DV_TIME, 200-row table — largest; open finding: temporal enforcement) | ECC-VAL-102 — Validate DV_TIME — range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DATE.validate_open (master17.4 §DV_DATE; ISO8601-validity rows not driven, RM-mandatory value only) | ECC-VAL-103 — Validate DV_DATE — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DATE.validate_constraint (master17.4 §DV_DATE; open finding: temporal enforcement) | ECC-VAL-104 — Validate DV_DATE — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DATE.validate_range (master17.4 §DV_DATE; open finding: temporal enforcement) | ECC-VAL-105 — Validate DV_DATE — range | pass | — |
| Content / archetype validation | ArchetypeValidation | POST /ehr/{ehr_id}/composition | ECC-VAL-119 — Validate DV_DATE — day disallowed by C_DATE pattern (defective vendored fixture rejected) | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DATE_TIME.validate_open (master17.4 §DV_DATE_TIME; RM-mandatory value only) | ECC-VAL-106 — Validate DV_DATE_TIME — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DATE_TIME.validate_constraint (master17.4 §DV_DATE_TIME, 176-row table; explicit open finding: SUT accepts the partial value the table rejects) | ECC-VAL-107 — Validate DV_DATE_TIME — constraint | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_DATE_TIME.validate_range (master17.4 §DV_DATE_TIME; open finding: temporal enforcement) | ECC-VAL-108 — Validate DV_DATE_TIME — range | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PARSABLE.validate_open (master17.6 §DV_PARSABLE; formalism-mandatory row not driven) | ECC-VAL-109 — Validate DV_PARSABLE — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_PARSABLE.validate_value_formalism (master17.6 §DV_PARSABLE; value C_STRING rows not driven) | ECC-VAL-110 — Validate DV_PARSABLE — value formalism | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_MULTIMEDIA.validate_open (master17.6 §DV_MULTIMEDIA; size-mandatory + media-type-codeset rows not driven) | ECC-VAL-111 — Validate DV_MULTIMEDIA — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_MULTIMEDIA.validate_media_type (master17.6 §DV_MULTIMEDIA; size C_INTEGER half of the table not driven) | ECC-VAL-112 — Validate DV_MULTIMEDIA — media type | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_URI.validate_open (master17.7 §DV_URI; headline is RFC3986 validity, ECC drives RM-mandatory value only) | ECC-VAL-113 — Validate DV_URI — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_URI.validate_pattern (master17.7 §DV_URI) | ECC-VAL-114 — Validate DV_URI — pattern | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_URI.validate_list (master17.7 §DV_URI) | ECC-VAL-115 — Validate DV_URI — list | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_EHR_URI.validate_open (master17.7 §DV_EHR_URI; headline is the ehr: scheme rule, ECC drives RM-mandatory value only) | ECC-VAL-116 — Validate DV_EHR_URI — open | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_EHR_URI.validate_pattern (master17.7 §DV_EHR_URI) | ECC-VAL-117 — Validate DV_EHR_URI — pattern | pass | — |
| Content / archetype validation | ArchetypeValidation | DV_EHR_URI.validate_list (master17.7 §DV_EHR_URI) | ECC-VAL-118 — Validate DV_EHR_URI — list | pass | — |
| Demographic service | PartyOperations | POST /demographic/person | ECC-DEM-001 — Demographic person create | pass | — |
| Demographic service | PartyOperations | POST /demographic/person | ECC-DEM-021 — Demographic create bad body | pass | — |
| Demographic service | PartyOperations | GET /demographic/person/{uid_based_id} | ECC-DEM-002 — Demographic person get | pass | — |
| Demographic service | PartyOperations | GET /demographic/person/{uid_based_id} | ECC-DEM-007 — Demographic person get absent | pass | — |
| Demographic service | PartyOperations | GET /demographic/person/{uid_based_id} | ECC-DEM-006 — Demographic person get deleted | pass | — |
| Demographic service | PartyOperations | GET /demographic/person/{version_uid} | ECC-DEM-003 — Demographic person get by version | pass | — |
| Demographic service | PartyOperations | GET /demographic/person/{uid_based_id}?version_at_time | ECC-DEM-025 — Demographic person get at time | pass | — |
| Demographic service | PartyOperations | PUT /demographic/person/{uid_based_id} | ECC-DEM-004 — Demographic person update | pass | — |
| Demographic service | PartyOperations | PUT /demographic/person/{uid_based_id} | ECC-DEM-008 — Demographic person update bad if match | pass | — |
| Demographic service | PartyOperations | DELETE /demographic/person/{uid_based_id} | ECC-DEM-005 — Demographic person delete | pass | — |
| Demographic service | PartyOperations | POST /demographic/agent | ECC-DEM-009 — Demographic agent create | pass | — |
| Demographic service | PartyOperations | GET /demographic/agent/{uid_based_id} | ECC-DEM-010 — Demographic agent get | pass | — |
| Demographic service | PartyOperations | DELETE /demographic/agent/{uid_based_id} | ECC-DEM-011 — Demographic agent delete | pass | — |
| Demographic service | PartyOperations | POST /demographic/group | ECC-DEM-012 — Demographic group create | pass | — |
| Demographic service | PartyOperations | GET /demographic/group/{uid_based_id} | ECC-DEM-013 — Demographic group get | pass | — |
| Demographic service | PartyOperations | DELETE /demographic/group/{uid_based_id} | ECC-DEM-014 — Demographic group delete | pass | — |
| Demographic service | PartyOperations | POST /demographic/organisation | ECC-DEM-015 — Demographic organisation create | pass | — |
| Demographic service | PartyOperations | GET /demographic/organisation/{uid_based_id} | ECC-DEM-016 — Demographic organisation get | pass | — |
| Demographic service | PartyOperations | DELETE /demographic/organisation/{uid_based_id} | ECC-DEM-017 — Demographic organisation delete | pass | — |
| Demographic service | PartyOperations | POST /demographic/role | ECC-DEM-018 — Demographic role create | pass | — |
| Demographic service | PartyOperations | GET /demographic/role/{uid_based_id} | ECC-DEM-019 — Demographic role get | pass | — |
| Demographic service | PartyOperations | DELETE /demographic/role/{uid_based_id} | ECC-DEM-020 — Demographic role delete | pass | — |
| Demographic service | PartyOperations | GET /demographic/versioned_party/{versioned_object_uid} | ECC-DEM-022 — Demographic versioned party get | pass | — |
| Demographic service | PartyOperations | GET /demographic/versioned_party/{versioned_object_uid}/revision_history | ECC-DEM-023 — Demographic versioned party revision history | pass | — |
| Demographic service | PartyOperations | GET /demographic/person/{uid_based_id}/tags | ECC-DEM-024 — Demographic person tags | pass | — |
| Demographic service | PartyRelationshipOperations | POST /demographic/party_relationship (ehrbase-rs extension) | ECC-DEM-026 — Demographic relationship create | pass | — |
| Demographic service | PartyRelationshipOperations | GET /demographic/party_relationship/{uid_based_id} (ehrbase-rs extension) | ECC-DEM-027 — Demographic relationship get | pass | — |
| Demographic service | PartyRelationshipOperations | GET /demographic/party_relationship/{uid_based_id}?version_at_time (ehrbase-rs extension) | ECC-DEM-028 — Demographic relationship get at time | pass | — |
| Demographic service | PartyRelationshipOperations | PUT /demographic/party_relationship/{uid_based_id} (ehrbase-rs extension) | ECC-DEM-029 — Demographic relationship update | pass | — |
| Demographic service | PartyRelationshipOperations | DELETE /demographic/party_relationship/{uid_based_id} (ehrbase-rs extension) | ECC-DEM-030 — Demographic relationship delete | pass | — |
| Demographic service | PartyRelationshipOperations | GET /demographic/versioned_party_relationship/{versioned_object_uid}/version/{version_uid} (ehrbase-rs extension) | ECC-DEM-031 — Demographic relationship get by version | pass | — |
| Admin service | AdminPhysicalDeletion | DELETE /admin/ehr/{ehr_id} | ECC-ADM-001 — Admin EHR delete | pass | — |
| Admin service | AdminPhysicalDeletion | DELETE /admin/ehr/{ehr_id} | ECC-ADM-002 — Admin EHR delete absent | pass | — |
| Admin service | AdminPhysicalDeletion | DELETE /admin/ehr/{ehr_id} | ECC-ADM-003 — Admin EHR delete idempotent | pass | — |
| Admin service | AdminPhysicalDeletion | DELETE /admin/ehr/all?ehr_id* | ECC-ADM-004 — Admin EHR delete all | pass | — |
| Admin service | AdminPhysicalDeletion | DELETE /admin/ehr/all?ehr_id* | ECC-ADM-005 — Admin EHR delete all partial | pass | — |
| Admin service | AdminPhysicalDeletion | DELETE /admin/ehr/all | ECC-ADM-006 — Admin EHR delete all (empty selector) | pass | — |
| Admin service | AdminActivityReport | native API only (I_ADMIN_SERVICE.list_contributions) | ECC-ADM-007 — Admin list contributions | n/a | — |
| Admin service | AdminActivityReport | native API only (I_ADMIN_SERVICE.contribution_count) | ECC-ADM-008 — Admin contribution count | n/a | — |
| Admin service | AdminActivityReport | native API only (I_ADMIN_SERVICE.versioned_composition_count) | ECC-ADM-009 — Admin versioned composition count | n/a | — |
| Admin service | AdminActivityReport | native API only (I_ADMIN_SERVICE.composition_version_count) | ECC-ADM-010 — Admin composition version count | n/a | — |
| Admin service | AdminEhrDumpLoad | native API only (I_ADMIN_DUMP_LOAD.export_ehrs) | ECC-ADM-011 — Admin export EHRs (dump/load) | n/a | — |
| Admin service | AdminEhrArchive | native API only (I_ADMIN_ARCHIVE.archive_ehrs) | ECC-ADM-012 — Admin archive EHRs | n/a | — |
| Admin service | AdminPhysicalDeletion | no ITS-REST binding (I_ADMIN_SERVICE.physical_party_delete) | ECC-ADM-013 — Admin physical party delete | n/a | — |
| Admin service | AdminDemographicArchive | no ITS-REST binding (I_ADMIN_ARCHIVE.archive_parties) | ECC-ADM-014 — Admin archive parties | n/a | — |
| Messaging | MessagingEhrExtract | native API only (I_EHR_EXTRACT_SERVICE.export_ehrs) | ECC-MSG-001 — EHR Extract — export whole EHR (export_ehrs) | n/a | — |
| Messaging | MessagingEhrExtract | native API only (I_EHR_EXTRACT_SERVICE.export_ehr_extracts) | ECC-MSG-002 — EHR Extract — spec-driven export (export_ehr_extracts) | n/a | — |
| Messaging | MessagingEhrExtract | native API only (I_EHR_EXTRACT_SERVICE.export_ehrs) | ECC-MSG-003 — EHR Extract — export of unknown EHR fails | n/a | — |
| Messaging | MessagingEhrExtract | native API only (I_EHR_EXTRACT_SERVICE.import_ehr) | ECC-MSG-004 — EHR Extract — import whole-EHR clone reusing source id | n/a | — |
| Messaging | MessagingEhrExtract | native API only (I_EHR_EXTRACT_SERVICE.import_ehr) | ECC-MSG-005 — EHR Extract — import whole EHR into a caller-fixed id | n/a | — |
| Messaging | MessagingEhrExtract | native API only (I_EHR_EXTRACT_SERVICE.import_ehr) | ECC-MSG-006 — EHR Extract — import into a duplicate target id fails | n/a | — |
| Messaging | MessagingEhrExtract | native API only (I_EHR_EXTRACT_SERVICE.import_ehr_extract) | ECC-MSG-007 — EHR Extract — import extract into an existing EHR | n/a | — |
| Messaging | MessagingTds | native API only (I_TDD_SERVICE.import_tdd) | ECC-MSG-008 — TDD — import a TDD as a committed COMPOSITION | n/a | — |
| Messaging | MessagingTds | native API only (I_TDD_SERVICE.import_tdd) | ECC-MSG-009 — TDD — import rejects malformed / non-TDD / unknown EHR / unknown template | n/a | — |
| Messaging | MessagingTds | native API only (I_TDD_SERVICE.import_tdds) | ECC-MSG-010 — TDD — batch import commits all, fail-fast on error | n/a | — |
| Security / authorization | Authentication | GET /ehr/{ehr_id} (no Authorization) | ECC-SEC-001 — Unauthenticated request to a protected route is refused (401) | pass | — |
| Security / authorization | Authentication | DELETE /admin/ehr/{ehr_id} (regular credential) | ECC-SEC-002 — Regular credential on an ADMIN-only route is forbidden (403) | pass | — |
| Version signing | Signing | GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid} | ECC-SIG-001 — Version signing — digest present | pass | pass |
| Version signing | Signing | GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid} | ECC-SIG-002 — Version signing — digest recomputes | pass | — |
| Version signing | Signing | PUT /ehr/{ehr_id}/ehr_status; POST /ehr/{ehr_id}/contribution; POST /ehr/{ehr_id}/directory | ECC-SIG-003 — Version signing — all kinds | pass | — |
| Version signing | Signing | POST /ehr/{ehr_id}/contribution | ECC-SIG-004 — Version signing — client verbatim | pass | — |
| Version signing | Signing | GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid} | ECC-SIG-005 — Version signing — pgp verifies | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-001 — TERMINOLOGY expand (bundle) — accepted, well-formed RESULT_SET | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-002 — TERMINOLOGY expand (bundle) — expansion constrains matches to the value set's codes | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-003 — TERMINOLOGY expand (bundle) — explicit code merged with the expansion | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-004 — TERMINOLOGY expand — unknown value set rejected (400) | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-005 — TERMINOLOGY expand — unknown service_api rejected (400) | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-006 — TERMINOLOGY expand (FHIR service_api) — accepted when a provider is configured | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-007 — TERMINOLOGY expand (FHIR) — terminology-server timeout is a server fault (500) | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-008 — TERMINOLOGY expand (FHIR) — terminology-server 5xx is a server fault (500) | pass | — |
| Terminology-server integration | Terminology | POST /query/aql | ECC-TS-009 — TERMINOLOGY expand (FHIR) — malformed terminology response is a server fault (500) | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/composition (Content-Type application/openehr.wt.flat+json) → GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept flat/json/structured) | ECC-SF-001 — FLAT commit then read-back as FLAT, canonical JSON, and STRUCTURED | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/composition (Content-Type application/openehr.wt.structured+json) → GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept structured/json/flat) | ECC-SF-002 — STRUCTURED commit then read-back as STRUCTURED, canonical JSON, and FLAT | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept with q-values) | ECC-SF-003 — Accept q-values select the highest-weight simplified format; every non-204 carries Content-Type | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /ehr/{ehr_id}/composition/{uid_based_id} + GET /definition/template/adl1.4/{template_id}/example (Accept a retired type) | ECC-SF-004 — Deprecated + legacy simplified media types are rejected on Accept (406) | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/composition + POST /ehr/{ehr_id}/contribution (Content-Type a retired type) | ECC-SF-005 — Deprecated + legacy simplified media types are rejected on write Content-Type (415) | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/composition (Content-Type flat, no openehr-template-id header) | ECC-SF-006 — FLAT commit without openehr-template-id (and no payload template id) → 422 | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/composition (Content-Type flat, unknown field id) | ECC-SF-007 — FLAT commit with an unknown field identifier → 422 | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/composition (Content-Type flat, |other + |code on one leaf) | ECC-SF-008 — FLAT commit with |other combined with |code on one coded leaf → 422 | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /definition/template/adl1.4/{template_id} (Accept application/openehr.wt+json) | ECC-SF-009 — GET a template as a Web Template document (Accept application/openehr.wt+json) | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /definition/template/adl1.4/{template_id}/example (Accept json/xml/flat/structured) | ECC-SF-010 — GET a template example in each of the four Accept forms (json, xml, flat, structured) | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /definition/template/adl1.4/{template_id}/example (Accept application/openehr.wt+json) | ECC-SF-011 — GET a template example with an unsupported Accept → 406 | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/contribution (Content-Type flat) → GET /ehr/{ehr_id}/contribution/{contribution_uid} (Accept flat) | ECC-SF-012 — CONTRIBUTION with a FLAT COMPOSITION inner payload: canonical envelope in, simplified read-back | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /ehr/{ehr_id}/ehr_status (Accept flat) + PUT /ehr/{ehr_id}/ehr_status (Content-Type flat) | ECC-SF-013 — EHR_STATUS has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /ehr/{ehr_id}/directory (Accept flat) + POST /ehr/{ehr_id}/directory (Content-Type flat) | ECC-SF-014 — DIRECTORY (FOLDER) has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | GET /demographic/person/{uid} (Accept flat) + POST /demographic/person (Content-Type flat) | ECC-SF-015 — Demographic PARTY has no Simplified-Formats mapping: Accept flat → 406, Content-Type flat → 415 | pass | — |
| Simplified Formats (FLAT / STRUCTURED / Web Template) | SimplifiedFormats | POST /ehr/{ehr_id}/composition (Content-Type flat, ctx/time set) → GET /ehr/{ehr_id}/composition/{uid_based_id} (Accept application/json) | ECC-SF-016 — FLAT ctx/time sets EVENT_CONTEXT.start_time; ctx/setting defaults to openehr::238 | pass | — |
| ADL2 template provisioning | Adl2Provisioning | POST /definition/template/adl2 (Prefer return=minimal|representation|identifier) | ECC-ADL2-001 — Upload a valid ADL2 template → 201 with Location; Prefer selects minimal/representation/identifier bodies | pass | — |
| ADL2 template provisioning | Adl2Provisioning | POST /definition/template/adl2 (same HRID twice) | ECC-ADL2-002 — Upload the same ADL2 HRID twice → the second is a 409 conflict | pass | — |
| ADL2 template provisioning | Adl2Provisioning | POST /definition/template/adl2 (unparseable source) | ECC-ADL2-003 — Upload an unparseable ADL2 source → 422 carrying syntax rule codes in validationErrors | pass | — |
| ADL2 template provisioning | Adl2Provisioning | POST /definition/template/adl2 (AOM2-invalid source) | ECC-ADL2-004 — Upload a semantically invalid ADL2 template (missing description) → 422 with the AOM2 rule code VARD | pass | — |
| ADL2 template provisioning | Adl2Provisioning | POST /definition/template/adl2 (parent) → POST /definition/template/adl2 (specialised child) | ECC-ADL2-005 — Upload a parent archetype, then a specialised child that validates against the stored parent → 201 | pass | — |
| ADL2 template provisioning | Adl2Provisioning | GET /definition/template/adl2/{template_id} (Accept text/plain | application/json | application/xml) | ECC-ADL2-006 — Get an ADL2 template as text/plain source, application/json OperationalTemplateV2, and 406 on xml-only | pass | — |
| ADL2 template provisioning | Adl2Provisioning | GET /definition/template/adl2/{template_id} | ECC-ADL2-007 — Get an unknown ADL2 template_id → 404 | pass | — |
| ADL2 template provisioning | Adl2Provisioning | GET /definition/template/adl2/{template_id}/{version} | ECC-ADL2-008 — Version get resolves an exact SEMVER and a major prefix (latest match) → 200; an unknown version → 404 | pass | — |
| ADL2 template provisioning | Adl2Provisioning | GET /definition/template/adl2/{template_id}/example (Accept json/xml/flat/structured) | ECC-ADL2-009 — Get a template example in each of the four Accept_LOCATABLE forms → 200; the JSON form is a COMPOSITION rooted at the template's archetype | pass | — |
| ADL2 template provisioning | Adl2Provisioning | GET /definition/template/adl2/{template_id}/example?type=&detail_level= | ECC-ADL2-010 — Example honours the detail_level enum (required/medium/complete) and rejects a bad type/detail_level with 400 | pass | — |
| ADL2 template provisioning | Adl2Provisioning | GET /definition/template/adl2/{template_id}/example | ECC-ADL2-011 — Example for an unknown template_id → 404; an Accept outside the four LOCATABLE forms → 406 | pass | — |
| ADL2 template provisioning | Adl2Provisioning | GET /definition/template/adl2 | ECC-ADL2-012 — List ADL2 templates → TemplateMetadata carrying template_id, concept, archetype_id, created_timestamp | pass | — |
| AQL terminology functions | AqlTerminology | POST /ehr/{ehr_id}/composition; POST /query/aql (matches TERMINOLOGY('expand', …)) | ECC-AQT-001 — TERMINOLOGY('expand') as a matches operand filters committed compositions by the value set's codes | pass | — |
| AQL terminology functions | AqlTerminology | POST /query/aql (matches TERMINOLOGY('lookup'|'map', …)) | ECC-AQT-002 — A non-expand TERMINOLOGY operation as a matches operand (lookup/map) → 400 | pass | — |
| AQL terminology functions | AqlTerminology | POST /query/aql (SELECT TERMINOLOGY('expand', …)) | ECC-AQT-003 — TERMINOLOGY() in an unsupported position (a SELECT column) → 400 | pass | — |
| AQL terminology functions | AqlTerminology | POST /query/aql (WHERE TERMINOLOGY('lookup', …) = true) | ECC-AQT-004 — A Boolean TERMINOLOGY assertion with an unsupported operation (lookup) → 400 | pass | — |

## Profile Report

### Core — PASS

| Capability | Required in profile | Result |
|---|:--:|---|
| Adl14ArchetypeProvisioning | Y | pass |
| Adl14OptProvisioning | Y | pass |
| EhrOperations | Y | pass |
| EhrStatus | Y | pass |
| CompositionOps | Y | pass |
| ChangeSets | Y | pass |
| Versioning | Y | pass |
| ArchetypeValidation | Y | pass |
| AnonymousEhrs | Y | pass |

### Standard — PASS

| Capability | Required in profile | Result |
|---|:--:|---|
| Adl14ArchetypeProvisioning | Y | pass |
| Adl14OptProvisioning | Y | pass |
| EhrOperations | Y | pass |
| EhrStatus | Y | pass |
| CompositionOps | Y | pass |
| ChangeSets | Y | pass |
| Versioning | Y | pass |
| ArchetypeValidation | Y | pass |
| AnonymousEhrs | Y | pass |
| QueryProvisioning | Y | pass |
| DirectoryOps | Y | pass |
| AqlBasic | Y | pass |
| Signing | Y | pass |

### Options — OBTAINED

| Capability | Required in profile | Result |
|---|:--:|---|
| Adl2Provisioning | OPT | pass |
| PartyOperations | OPT | pass |
| PartyRelationshipOperations | OPT | pass |
| AqlAdvanced | OPT | pass |
| AqlTerminology | OPT | pass |
| AdminActivityReport | OPT | not evidenced |
| AdminPhysicalDeletion | OPT | pass |
| AdminEhrDumpLoad | OPT | not evidenced |
| AdminBulkEhrLoad | OPT | no cases |
| AdminEhrArchive | OPT | not evidenced |
| AdminDemographicArchive | OPT | not evidenced |
| MessagingEhrExtract | OPT | not evidenced |
| MessagingTds | OPT | not evidenced |
| SimplifiedFormats | OPT | pass |

