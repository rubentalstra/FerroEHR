# A1 Spec Audit — Verify + Fix — chapter `rm-support`

- **Chapter:** RM 1.2.0 support + BASE identification (the identifier machinery,
  terminology/measurement service duties)
- **Date:** 2026-07-11
- **Scope:** all 40 requirements `rm-support-R1 … R40`
- **Result (defer-nothing pass):** 4 gaps fixed — ARCHETYPE_ID and
  TERMINOLOGY_ID had NO lexical validation (accessors only, never dispatched);
  the version-tree `creating_system_id` comparison was case-sensitive; UCUM
  syntax validation (`is_valid_units_string`) did not exist. One CNF
  adjudication (spaces in terminology names) and one corpus adjudication
  (no commit-time UCUM rejection) recorded.

## Verdict table

| id | classification | evidence / fix |
|---|---|---|
| R1 | verified | `uuid::Uuid` parse (strict RFC form) wherever UUIDs enter; `Uid` dispatch |
| R2/R3 | verified | `iso_oid_impl.rs` / `internet_id_impl.rs` lexical checks + dispatch arms |
| R4 | verified | `Uid` resolution is structure-driven (UUID → ISO_OID → INTERNET_ID patterns are disjoint) |
| R5 | verified-derived | `has_extension()` computed from `extension()` — the XOR is definitional |
| R6 | verified | `uid_based_id_impl.rs` root/extension split on the first `::` |
| R7/R8 | verified | `ObjectVersionId::from_str` strict 3-part + UID-typed parts (chapter-1 `version_id.rs` uses it everywhere) |
| R9–R16 | verified | `version_tree_id_impl.rs` `is_valid_version_tree` (trunk ≥ 1, 1-or-3-part, numeric); 2-part rejected; branch semantics first-class since chapter 1 |
| R17–R20 | fixed-in-this-pass | ARCHETYPE_ID lexical form (`rm_originator-rm_name-rm_entity.domain_concept.vN`, numeric version — the `.v1draft` WARNING enforced) — new `is_valid_archetype_id` + `Validate` + dispatch arm; corpus-scanned safe (0 violations) |
| R21 | fixed-in-this-pass | TERMINOLOGY_ID lexical form — new `is_valid_terminology_id` + `Validate` + dispatch; PORT NOTE: interior spaces accepted (the CNF's own data carries `"SNOMED CT"` — CNF outranks the strict `name-str` production) |
| R22 | verified | `value: String` non-optional on every OBJECT_ID subtype |
| R23 | verified | `object_ref_impl.rs` `namespace_valid` regex |
| R24/R25 | verified | `type`/`id` non-optional |
| R26 | verified | `party_ref_impl.rs` `Type_validity` (7-member set) |
| R27 | verified | `LocatableRef.id: UidBasedId` — narrowed type, fail-closed |
| R28/R29 | verified | `path: Option<String>`; `as_uri` in `locatable_ref_impl.rs` |
| R30 | fixed-in-this-pass | case-insensitive composite-id equality existed at the base layer (`is_equal`, `eq_ignore_ascii_case`) but the version-tree fork decision compared `creating_system_id` case-SENSITIVELY — a csid differing only by case forked a spurious branch; now folds case (`vobject.rs::next_version`) |
| R31 | verified | storage preserves identifier case byte-for-byte (values stored verbatim; only comparisons fold) |
| R32 | verified | the lexical productions (R17–R21, R2/R3) are ASCII-restricted |
| R33/R34/R37 | verified | `bundle::code_set()` returns `Option` (unknown id → `None`); the seven internal code-set ids resolvable |
| R35/R36 | verified | the walker terminology pass (chapters 3/5/6) realizes the group/code-set duties |
| R38 | verified | all mandated group ids present in the TERM bundle accessors |
| R39 | fixed-in-this-pass | `MEASUREMENT_SERVICE.is_valid_units_string` did not exist — new UCUM SYNTAX validator (`openehr-term/src/measurement.rs`, grammar-level: terms, `.`/`/`, exponents, factors, `[...]`, `{...}`, parens). PORT NOTE: commit-time rejection of non-UCUM `DV_QUANTITY.units` is corpus-ADJUDICATED OFF — the CNF's own valid data carries `°C`, `mmHg`, `pH`, and the RM declares no `Units_valid` invariant; template-declared unit constraints are enforced by the walker |
| R40 | verified-behavioural | `units_equivalent` (dimensional analysis) needs the UCUM atom table — out of the syntax validator's scope; no RM invariant or CNF case consumes it; the walker's template unit constraints cover the conformance surface. Flagged: no openEHR spec defines the atom table — UCUM itself does; revisit if an ECC case ever exercises it |

## Fixes applied

- **R17–R21** — `archetype_id_impl.rs`/`terminology_id_impl.rs`: lexical
  validity + `Validate` impls + `validate_rm_value` dispatch arms (direct
  names, no import aliases); tests `archetype_id_lexical_form`,
  `terminology_id_lexical_form`.
- **R30** — `vobject.rs::next_version`: csid comparison folds ASCII case
  (BASE master05 §Composite Identifiers and Case).
- **R39** — `openehr-term/src/measurement.rs::is_valid_units_string`
  (UCUM syntax); test `ucum_syntax`.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
