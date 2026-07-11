# A1 Spec Audit — Verify + Fix — chapter `term`

- **Chapter:** TERM 3.1.0 support terminology + RM support terminology package
- **Date:** 2026-07-11
- **Scope:** all 53 requirements `term-R1 … R53`
- **Result (defer-nothing pass):** 3 gaps fixed — the RM identifier-constant
  classes (`OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS` /
  `OPENEHR_CODE_SET_IDENTIFIERS`) had no `*_impl.rs` (constants +
  `valid_terminology_group_id`/`valid_code_set_id` did not exist), and
  `C_DV_QUANTITY.property` was unvalidated at OPT upload. Every group and
  code-set membership was re-verified byte-exactly against the vendored
  TERM 3.1.0 assets. Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1–R5 | verified | the two binding families in `openehr-flat/src/validation/terminology.rs` realize `has_code_for_group_id` (openehr-guarded) and `code_set(id).has_code` (unguarded, per the RM invariant form); membership predicates are exact code-value matches via the bundle |
| R6, R7 | fixed-in-this-pass | new `openehr_terminology_group_identifiers_impl.rs` + `openehr_code_set_identifiers_impl.rs`: the complete constant sets (14 group ids, 7 code-set ids) + `valid_terminology_group_id`/`valid_code_set_id`; regenerated mod registration; 2 tests |
| R8 | fixed-in-this-pass | `TERMINOLOGY_ID_OPENEHR = "openehr"` constant (same impl) |
| R9, R10 | fixed-in-this-pass | the enumerated sets are the `ALL_GROUP_IDS`/`ALL_CODE_SET_IDS` constants; the runtime validator consults every one of them (walker slots + service paths) |
| R11, R12 | verified | `AUDIT_DETAILS.change_type` via walker slot + service/codes.rs wire validation + the `ck_audit_change_type` DB CHECK; membership re-verified: exactly {249,250,251,252,253,523,666,816,817} in the bundle (incl. the RM-current 816/817) |
| R13, R14 | verified | lifecycle via `codes::lifecycle_state_code` → `is_valid_version_lifecycle_state` (five-state commit path); membership exactly {532,553,523,800,801} |
| R15, R16 | verified | `Reason_valid` at attestation completion (`contribution::complete_attestation`, any coded reason checked — the literal invariant carries no terminology guard) + the walker ATTESTATION.reason slot; membership exactly {240,648} |
| R17, R18 | verified | COMPOSITION.category walker slot; membership exactly {431,451,433,815} |
| R19, R20 | verified | `null_flavour` checked on any node (walker); membership exactly {271,253,272,273} |
| R21, R22 | verified | EVENT_CONTEXT.setting; membership exactly the 14 codes (incl. 802) |
| R23, R24 | verified | PARTICIPATION.function/mode walker slots |
| R25–R27 | verified | ISM_TRANSITION.current_state/transition; instruction-states membership exact |
| R28, R29 | verified | EVENT/POINT_EVENT/INTERVAL_EVENT.math_function; membership exactly the 11 codes |
| R30 | verified | PARTY_RELATED.relationship walker slot + the STRICT commit-path check (`validate_party_related_relationship`, ch1) |
| R31, R32 | verified | TERM_MAPPING.purpose; membership exactly {669,670,671} |
| R33 | fixed-in-this-pass | `C_DV_QUANTITY.property` at OPT upload: openehr-coded property must be in the `property` group (`Property_valid`); corpus-adjudicated (PORT NOTE: Ocean's placeholder code `0` = unconstrained, the vendored `action test` template) |
| R34–R36, R38, R39, R41, R43 | verified | languages/countries/charsets/media types/compression/integrity/normal statuses slots (COMPOSITION, ENTRY family, DV_TEXT family, DV_MULTIMEDIA, DV_PARSABLE, DV_ORDERED) — chapters 5/6 completed the encapsulated slots |
| R37, R40, R42, R44 | verified | closed-set memberships re-verified byte-exactly against the vendored assets (14 charsets, 5 compression, 7 integrity, 7 normal statuses) |
| R45 | verified-policy | code-set resolution is by RM slot (the invariant passes only the code to `has_code`; external ids like `ISO_639-1` are configuration metadata on the code set, not a data-rejection duty on `terminology_id` spelling) |
| R46 | verified-policy | terminology identifiers are accepted in both UMLS forms (no rejection surface — the lexical TERMINOLOGY_ID check is ch7's) |
| R47, R48 | verified | bundle accessors return `Option` (precondition-guarded lookup); `has_terminology`-style existence via `group`/`code_set` |
| R49–R51 | verified | membership matching keys on the code value / concept id, never the rubric (the codes-not-rubrics fix, blueprint row 10); code sets are language-independent single artefacts |
| R52 | verified | membership sets are read from the bundle enumeration (`concepts_in_group` / `CodeSet.codes`), never hardcoded |
| R53 | verified | vendored bundle pinned (TERM 3.1.0, byte-identical assets, `version="3.1.0"` in each artefact) |

## Fixes applied

- `crates/openehr-rm/src/support/terminology/openehr_terminology_group_identifiers_impl.rs`
  + `openehr_code_set_identifiers_impl.rs` (new): the RM constant classes +
  validity functions; mod registration regenerated (`openehr-codegen -- emit`
  auto-declares `*_impl` siblings — the run also normalized three earlier
  hand-added `pub mod *_impl;` lines into the canonical generated section).
- `app/ehrbase/src/service/opt_validation.rs`: `Property_valid` on
  `C_DV_QUANTITY.property` at OPT upload.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
