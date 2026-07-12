# A1 Spec Audit — Verify + Fix — chapter `am-aom14-opt`

- **Chapter:** AM — AOM 1.4 constraint model + ADL 1.4 cADL semantics + OPT
  flattening
- **Date:** 2026-07-11
- **Scope:** all 56 requirements `am-aom14-opt-R1 … R56`
- **Result (defer-nothing pass):** the runtime validation walker (B2) already
  realized the data-conformance side; the artefact-ingestion side gained a
  full AOM 1.4 constraint-model invariant pass in
  `app/ehrbase/src/service/opt_validation.rs` — 12 new check families
  (Existence_set, Rm_attribute_name_valid, Members_valid, VARID, VARDT,
  VDFAI, Target_path_valid, VACDF, C_BOOLEAN satisfiability,
  Assumed_value_valid, temporal Pattern_validity, duration pattern syntax),
  each corpus-adjudicated (91-file upload corpus green; the wider Better/SDK
  fixture sweep drove the `DV_PROPORTION.is_integral` legacy tolerance and
  confirmed the VACDF gating). Zero deferrals.

## Enforcement contexts

1. **Artefact ingestion** (`store_template` → `validate_opt_artefact`):
   typed `opt14::from_xml` + strict top-level structure check + the AOM2/08
   catalogue (B2: VCORM/VCARM/VCAEX/VCAM/VACMCO/VATID/VTTBK/VTCBK/VTLC) +
   the NEW AOM 1.4 invariant pass (this chapter).
2. **Runtime composition validation** (openehr-flat walker over the
   WebTemplate built from the flattened OPT): occurrences/cardinality/
   existence, alternatives, subtype conformance, slot admission, leaf
   constraints (B2).

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1 | fixed-in-this-pass | `Existence_set` (lower ≥ 0, upper ≤ 1) enforced in `walk_attribute`; test `existence_set_upper_above_one` |
| R2 | fixed-in-this-pass | `Rm_attribute_name_valid` (non-empty) enforced |
| R3, R11, R12, R13, R20, R46 | verified-model | `any_allowed` is not serialized in OPT 1.4 XML — it is *derived* from children/attributes/item/includes+excludes absence (Template.xsd), so the xor invariants are definitional in the wire form; `Item_valid`'s "item absent ⇒ any allowed" is the typed `Option` |
| R4, R7 | fixed-in-this-pass | `Members_valid`: child occurrences upper ≤ 1 under `C_SINGLE_ATTRIBUTE`; test `members_valid_occurrences_above_one_under_single_attribute`; corpus-safe (91/91) |
| R5, R6 | verified-model + PORT-NOTEd | `cardinality`/`interval`/`is_ordered`/`is_unique` mandatory in the typed model (deserialize fails when absent); the RM-cardinality-contradiction half (VCACA) stays PORT-NOTEd — the static RM model exposes container kind, not numeric bounds |
| R8 | verified | `node_id` mandatory (typed `String`); at-code definedness = VATID (B2) |
| R9, R10 | fixed-in-this-pass | `Assumed_value_valid` for every leaf/domain kind expressible in OPT 1.4 XML: C_STRING closed list, C_INTEGER/C_REAL list+range, C_BOOLEAN vs true/false_valid, C_CODE_PHRASE/C_CODE_REFERENCE code list, C_DV_ORDINAL value pairs, C_DV_QUANTITY units+magnitude; tests `assumed_value_outside_closed_list` (+ boolean case). Assumed values are leaf-only by construction (only leaf/domain types carry the field in the XSD) |
| R14 | verified-model | absent ≡ empty list in the XML wire — a present-but-empty includes is unrepresentable after deserialize |
| R15, R16 | verified | runtime slot admission `slot_admits` (openehr-flat validation/mod.rs): includes any-match, excludes veto (with the closed-slot `.*` idiom), Perl-regex via `matches_pattern` (regex + fancy-regex fail-closed) |
| R17, R24, R31 | fixed-in-this-pass | `Target_path_valid` (non-empty absolute path) on `ARCHETYPE_INTERNAL_REF`; corpus evidence: 0/229 vendored OPTs carry internal refs (flattening expands them), so full target resolution has no live surface — the syntactic VDFPT check guards malformed artefacts |
| R18, R19, R51 | verified-inapplicable | internal-ref occurrence defaulting + VUNT + slot/ref expansion are duties of the *flattener* (template tooling); the CDR ingests already-flattened OPTs (0/229 corpus presence). Unfilled slots build no WebTemplate node; fillers appear as `C_ARCHETYPE_ROOT`s |
| R21, R32 | fixed-in-this-pass | VACDF: `CONSTRAINT_REF.reference` ∈ `constraint_definitions` — gated on the artefact declaring a constraint vocabulary at all (PORT NOTE: Ocean/Better flattened exports carry CONSTRAINT_REF with ZERO constraint_definitions — 32 corpus files, all with 0 cdefs) |
| R22 | verified-inapplicable + enforced-part | OPT 1.4 `concept` is the concept *name* string (not an at-code); non-empty enforced in `store_template` (CNF removed_concept_value); the at-code VARCN invariant applies to raw ARCHETYPE artefacts, which the adl1.4 surface does not ingest |
| R23 | verified | VATID (B2) |
| R25, R26, R49 | verified-inapplicable | `version`, `parent_archetype_id`, specialisation depth are not serialized in OPT 1.4 XML (Template.xsd) — structurally unrepresentable at this surface |
| R27 | verified | typed deserialize: `definition`, `template_id` mandatory fields; `concept` emptiness checked in `store_template`; root is typed `CArchetypeRoot` (a C_COMPLEX_OBJECT) |
| R28 | fixed-in-this-pass | VARDT: root + every nested `C_ARCHETYPE_ROOT` rm_type_name vs the id's type slot, case-insensitive (BASE `base_types` master05 §Composite Identifiers and Case); test `vardt_root_type_mismatch` |
| R29 | fixed-in-this-pass | VARID on root + nested roots. Tolerances (PORT-NOTEd, corpus-adjudicated): multi-part numeric versions (`v1.0.0`, vendored Request_for_Pancreas template) and parenthesized tooling concept names (LANIT/Better corpus); test `varid_tolerates_multipart_version_and_tooling_names` |
| R30 | fixed-in-this-pass | VDFAI: literal (regex-metachar-free) slot include/exclude alternatives validated as archetype ids; genuine regexes left to runtime admission |
| R33, R34 | verified | existence/occurrences always explicit in OPT 1.4 XML (mandatory XSD elements → typed non-Option); `{0..0}` honoured by the walker (prohibited node) |
| R35 | verified | VACMCO/VCOC (B2) — mandatory-occurrence sums vs cardinality (the Better/SDK sweep confirmed `sdk/section_cardinality.opt` genuinely violates it and must reject) |
| R36, R52 | verified-partial + PORT-NOTEd | the RM-widening axis is enforced (VCAEX existence, VCAM multiplicity, VCORM types); parent-archetype subset conformance (`is_subset_of`) is inapplicable to a flat artefact — no differential parent is present in an OPT |
| R37 | verified | alternatives: the walker admits an instance node matching *any* sibling constraint (validation/mod.rs admission) |
| R38 | verified | subtype conformance (openehr-flat validation/subtype.rs) — foreign `_type` rejected |
| R39, R40 | verified | C_STRING case-sensitive list/pattern, fail-closed regex (B2; leaf.rs `check_string_constraints`/`matches_pattern`) |
| R41 | fixed-in-this-pass | C_BOOLEAN true_valid/false_valid both false → reject (`C_BOOLEAN_validity`); test `c_boolean_unsatisfiable` |
| R42 | verified | numeric list/range at runtime (leaf.rs `check_numeric_lists`, ranges); artefact-side assumed-value coverage added (R9) |
| R43, R44 | fixed-in-this-pass | `Pattern_validity` at ingestion: legal pattern forms + the optional→optional/disallowed, disallowed→disallowed monotonic chain; C_DATE_TIME validates one Month→Day→Hour→Minute→Second chain (`hour_validity` exists on C_DATE_TIME but not C_TIME — the CNF RIPPLE template's `yyyy-??-??T??:??:??` adjudicated this); tests `pattern_validity_rejects_nonmonotonic_temporal_pattern`, `temporal_pattern_validity_forms` |
| R45 | verified | timezone can be required, never prohibited: pattern suffixes accepted (`Z`/`±hh[:mm]`); runtime `check_timezone_validity` |
| R47 | fixed-in-this-pass (artefact) + verified (runtime) | duration pattern syntax `P[Y][M][W][D][T[H][M][S]]` with openEHR's W-mixing; all 15 corpus forms green; runtime designator enforcement was B2 (leaf.rs `check_duration`) |
| R48, R50 | verified | validation runs against the WebTemplate built from the flattened OPT; terminology resolution consumes `ontology` + `component_ontologies` + inline root `term_definitions` (builder.rs) |
| R53 | verified | C_DOMAIN_TYPE leaves validated natively (leaf.rs quantity/ordinal/code-phrase checks) — `standard_equivalent` reduction unnecessary when the domain semantics are implemented directly |
| R54 | verified-inapplicable | archetype FOL `invariants` are NOT serialized in OPT 1.4 XML (Template.xsd carries no `invariants` element — only Archetype.xsd does; 0/229 corpus occurrences): structurally unrepresentable at this ingestion surface |
| R55, R56 | verified | the artefact walk descends the whole constraint tree (first violation reported); runtime `valid_value` is the top-down walker cascade |

## Fixes applied

- `app/ehrbase/src/service/opt_validation.rs` — the AOM 1.4
  constraint-model invariant pass (see table); `DV_PROPORTION.is_integral`
  added to the legacy attribute tolerances (computed function in RM 1.2.0,
  RM 1.0.x tooling emits it as constrainable).
- `app/ehrbase/src/service/opt_validation/tests.rs` — 10 new
  negative/positive tests; corpus guard documents why the Better/SDK
  serialization fixtures are excluded (several genuinely violate VCOC and
  MUST reject at upload).
- Clippy-clean sweep across the audit-touched service files (vobject,
  message, ehr, directory, demographic, dump_load, aql/sql, aql/analyze) —
  incl. scrubbing one residual ADR citation in vobject.rs.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
