# Catalogue audit — DEFINITION_ADL14 chapter

Issue #231 · audited 2026-07-24 · 17 cases · verdicts: 14 ok / 3 DEFECT (all three fixed in this audit) / 0 ambiguities

Chapter context: CNF `master04-func_tc_definition_adl.adoc` carries 16 real
official case tables (all present in the catalogue with verbatim ids) plus the
catalogue's one addition (`list_archetypes-unrealized`, AMB-41 — the SM/Profiles
archetype-provisioning surface ITS-REST 1.1.0 does not expose). The chapter's
name divergences are register-covered: AMB-13 (get_opts vs SM list_opts),
AMB-16 (validate_opt vs SM valid_opt; no validation endpoint — realized via
upload per the master04 NOTE), AMB-4 (duplicate template_id: option_select
siblings conflict/versioned), AMB-17 (delete_opt has no ITS-REST 1.1.0 wire —
the four delete cases are active, SM-anchored, N/A-with-citation on this ITS).

| case | verdict | evidence | resolution |
|---|---|---|---|
| validate_opt-valid_opt | ok | AMB-16 tagged; official valid-OPT data-set realization; reset_per_row matches the master04 §data sets run semantics (read: the per-X pre/post reset block) | phantom pointer → AMB-16 (fixed) |
| validate_opt-invalid_opt | ok | The four official invalid-OPT defects (empty file, empty template_id, removed mandatory, multiple elements — verified against the master04 §upload_opt data-set list) with state-unchanged postcondition | phantom pointer → AMB-16 (fixed) |
| upload_opt-valid_opt | ok | Official ground; retrievability postconditions wired to the read cases | none |
| upload_opt-invalid_opt | ok | Same official invalid data-set family; unchanged-server postcondition | none |
| upload_opt-valid_opt_twice_conflict | ok | AMB-4 conflict sibling (`option: adl14-duplicate-conflict`); already_exists on the duplicate | none |
| upload_opt-valid_opt_twice_no_conflict | ok | AMB-4 versioned sibling; distinct v3/v4 fixtures avoid collision with the read-side versioned pair | none |
| get_opt-retrieve_single | ok | "exactly the same as the uploaded one" → `assert: equivalent`; template_id `obs_act.en.v1` matches the manifest | none |
| get_opt-retrieve_fail | ok | Random id → not_found with message exemplar | none |
| get_opt-retrieve_latest_version | ok | Partial id `test_versioned` → latest (v2) per the ITS-REST template_id resolution; option-gated on the versioned branch | none |
| get_opt-retrieve_specific_version | ok | Full id `test_versioned.en.v1` → that version | none |
| get_opts-retrieve_all | ok | AMB-13 tagged (SM list_opts); both loaded ids asserted | none |
| get_opts-retrieve_all_no_opts | ok | `server: exclusive` correctly guards the global-emptiness ground; empty set, no failure | none |
| delete_opt-delete_existing | DEFECT — fixed | Flow used the stub-era `cnf.minimal_event` template_id, which matches no manifest entry (the fixture's id is `minimal_evaluation.en.v1`) — masked because AMB-17 leaves the case never-driven | ids corrected to the manifest template_id |
| delete_opt-delete_latest_version | DEFECT — fixed | Two defects: stub-era ids (`cnf.versioned.*`), and a GROUND misalignment — the official cell's flow deletes with NO version parameter and its NOTE says "all the versions of the OPT will be deleted", ending in retrieve-none; the case instead encoded delete-v2-prior-remains, a ground the official cell does not state | re-encoded to the version-less all-versions delete with the NOTE cited |
| delete_opt-delete_specific_version | DEFECT — fixed | Stub-era ids; the ground itself (delete non-latest → latest remains retrievable) matches the official flow | ids corrected to `test_versioned.en.v1/.v2` |
| delete_opt-delete_non_existing | ok | Non-existent id → not_found, unchanged server | none |
| list_archetypes-unrealized | ok | Catalogue addition; AMB-41 covers the SM-vs-wire archetype-provisioning gap; Profiles §Functional (Definitions) names the capability | none |

Checks common to the chapter:
- **Ground (dim 1):** all 16 official master04 ids present verbatim; the delete_latest_version ground realigned to the official NOTE (above).
- **Expectations (dim 2):** recomputed from the official tables + SM `i_definition_adl14.adoc`; the official invalid-OPT defect list matched one-to-one.
- **Fixtures (dim 4):** every `cnf.opt.*` key mapped to its manifest `template_id` (the audit table above used those mappings); the stale-id class the #231 contract predicted was found exactly here, masked by AMB-17's unrealized status.
- **Captures (dim 5):** single capture (`template_id`) in the conflict sibling; consistent.
- **Ambiguity tags (dim 6):** AMB-4/13/16/17/41 all read; each covers its tagged divergence.

Post-fix machine floor: `cnf-runner validate` — 395 cases, 88 bindings, 0 findings.
