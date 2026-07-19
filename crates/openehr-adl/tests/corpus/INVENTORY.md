# ADL2 corpus inventory — the coverage-gate map

Per-file expected-outcome inventory of the vendored ADL2 conformance corpus and
flattener fixtures, so the 100%-coverage harness can claim every file with an
asserted outcome. Companion to `PROVENANCE.md` (which records where the assets
came from); this file records **what each file expects and which harness owns
it**.

**Corpus pins this file describes** (copy of `PROVENANCE.md`):

- `adl2-reference/` — openEHR `adl-archetypes` `ADL2-reference/`, commit
  `093c77ea003742b9540e3dd377d615e2b26f2996` (2025-06-27).
- `flattener/` — archie
  `tools/src/test/resources/com/nedap/archie/flattener/{specexamples,siblingorder}`,
  commit `e8d92f28aca33f92ea08a826ea19f9581d579720` (2026-07-08).

**Maintenance:** regenerate/update this file whenever the corpus is re-vendored
(the pins above change). File counts, the code frequency table, and the gap
lists are all derived from the tree — re-derive them, do not hand-patch.

**Spec oracle for the codes** (not any internal plan doc):

- **S-codes** (syntax validity) — `docs/specs/openehr/AM/docs/ADL2/master04.6-cadl_validity_rules.adoc`
  §Syntax Validity Rules.
- **V-/W-codes** (semantic validation, phases 1/2/3 + class invariants) —
  `docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc`; conformance
  functions in `ADL2/master04.5-cadl_primitive_types.adoc` and
  `AOM2/master04.5`; specialisation semantics in
  `ADL2/master09.02`–`master09.10`.

> **THE ORACLE IS THE `regression` TAG, NOT THE FILENAME.** Every
> `adl2-reference` archetype embeds its authoritative expected outcome in ODIN
> as `other_details["regression"] = <"CODE">` (`"PASS"`, `"FAIL"`, or a rule
> code). Filenames encode a hint that is **stale in 13 cases** (§2c). Harnesses
> MUST read the tag. Flattener fixtures carry no tag and no stored flat output.

---

## 1. Tree census

Totals: **343 files** — 319 `.adls`, 21 `.adl`, 1 `.idx` (`_repo_lib.idx`,
library manifest — not a fixture), `PROVENANCE.md`, `flattener/LICENSE`.
ADL sources = **340** (302 under `adl2-reference/`, 38 under `flattener/`).
No `.adlf` files; expected flats are not vendored (§7).

| Directory | .adls | .adl |
|---|---:|---:|
| `features/alternatives` | 1 | 0 |
| `features/aom_structures/basic` | 8 | 0 |
| `features/aom_structures/c_attribute_alternatives` | 4 | 1 |
| `features/aom_structures/primitive_types` | 3 | 0 |
| `features/aom_structures/rules` | 3 | 0 |
| `features/aom_structures/slots` | 7 | 0 |
| `features/aom_structures/tuples` | 11 | 0 |
| `features/aom_structures/use_archetype` | 1 | 0 |
| `features/aom_structures/use_node` | 3 | 0 |
| `features/description/annotations` | 4 | 0 |
| `features/description/identification` | 4 | 0 |
| `features/description/meta_data` | 7 | 0 |
| `features/description/text` | 5 | 0 |
| `features/editor` | 3 | 0 |
| `features/flattening` | 10 | 0 |
| `features/reference_model/clinical_data_types` | 7 | 0 |
| `features/reference_model/enumerated_types` | 1 | 0 |
| `features/reference_model/generic_types` | 6 | 0 |
| `features/reference_model/participations` | 1 | 0 |
| `features/spec_level_change` | 5 | 0 |
| `features/specialisation` | 19 | 0 |
| `features/specialisation/sibling_order` | 3 | 0 |
| `features/specialisation/terminology` | 10 | 0 |
| `features/terminology/term_bindings` | 5 | 0 |
| `features/terminology/value_sets` | 3 | 0 |
| `features/tool_visualisation` | 1 | 0 |
| `robustness` | 3 | 0 |
| `upgrade/upgrade_from_14` | 9 | 9 |
| `upgrade/upgrade_from_15` | 8 | 0 |
| `validity/annotations` | 2 | 0 |
| `validity/basics` | 17 | 0 |
| `validity/consistency` | 13 | 0 |
| `validity/domain_types` | 2 | 0 |
| `validity/legacy_adl_1.4` | 10 | 11 |
| `validity/paths` | 2 | 0 |
| `validity/rm_checking` | 11 | 0 |
| `validity/slots` | 7 | 0 |
| `validity/specialisation` | 32 | 0 |
| `validity/structure` | 11 | 0 |
| `validity/templates` | 7 | 0 |
| `validity/terminology` | 12 | 0 |
| `flattener/specexamples` | 25 | 0 |
| `flattener/siblingorder` | 13 | 0 |

**Flags**

- **Empty / non-UTF-8:** none.
- **UTF-8 BOM (29 files):** the lexer must strip/tolerate a leading BOM even
  though `ADL2/master03` says "no BOM" — these files must lex. List:
  `features/aom_structures/rules/…rules_formulae`;
  `features/aom_structures/slots/…{slot_include_any_exclude_empty,
  slot_include_empty_exclude_any, slot_include_empty_exclude_non_any,
  slot_include_non_any_exclude_any, slot_include_non_any_exclude_empty}`;
  `features/description/text/…unicode_farsi`;
  `features/flattening/…override_to_multiple`;
  `features/specialisation/…body_temp_test`;
  `features/specialisation/terminology/…{code_list_parent, coded_text_parent,
  dv_text_parent, ref_set_parent}`; `robustness/…empty_bindings`;
  `upgrade/upgrade_from_14/…{id_codes_as_at_codes.v1.adl,
  exclusion-adverse_reaction.v1.adl, exclusion.v1.adl,
  inherit_unchanged_node.v1.adl, inherit_unchanged_parent.v1.adl,
  adl14_meta_data.adl, upgrade_parent.v1.adl}`;
  `validity/basics/…whitespace`;
  `validity/legacy_adl_1.4/…{dimensions.v1.adl, use_node-occ_override.v1.adl,
  use_node_parent.v1.adl, lab_test-microbiology-csf.v1.adl}`;
  `validity/rm_checking/…rm_same_existence`; `validity/slots/…slot_parent`;
  `validity/specialisation/…address`.
- **Duplicate basenames (2)** — harnesses MUST key on full path, not basename:
  - `openEHR-EHR-OBSERVATION.empty_observation.v1.0.0.adls` — in
    `features/specialisation/` AND `flattener/specexamples/`.
  - `openehr-TEST_PKG-WHOLE.regular_primitive_types.v1.0.0.adls` — in
    `features/aom_structures/primitive_types/` AND `features/specialisation/`.

---

## 2. Expected-outcome per file

### 2a. Directory summary

- **`validity/**` is 100% tagged.** Files that are not `PASS`/`FAIL` carry a
  specific rule code (full list in §2b).
- **`features/**`, `robustness/**`, `upgrade/**`** = `PASS` or untagged-PASS
  (parse/validate clean; specialised `features` also flatten). A rule code
  appearing *inside* a concept name in these trees is a support archetype, not
  a failure expectation (e.g. `new_VSONCO-redef_open` is a PASS parent).
- Flattener fixtures = no tag (§7).

### 2b. Every non-PASS validity file, by authoritative `regression` tag

| Tag | File (dir · concept) |
|---|---|
| FAIL | basics · FAIL_archetype_id_empty.v1.adls |
| FAIL | basics · FAIL_archetype_id_missing.v1.adls |
| FAIL | basics · FAIL_definition_empty.v1.0.0.adls |
| FAIL | basics · FAIL_terminology_extra_end_mark.v1.0.0.adls |
| FAIL | specialisation · FAIL_missing_parent.v1.0.0.adls |
| FAIL | specialisation · FAIL_missing_parent_term.v1.0.0.adls |
| FAIL | structure · VCARM_table.v1.0.0.adls *(filename VCARM)* |
| FAIL | terminology · FAIL_terminology_empty.v1.0.0.adls |
| FAIL | terminology · FAIL_terminology_term_definitions_missing.v1.0.0.adls |
| SADF | basics · FAIL_terminology_missing.v1.0.0.adls *(filename FAIL)* |
| SADF | basics · SADF_definition_after_terminology.v1.0.0.adls |
| SCAS | basics · SCAS_attribute_empty.v1.0.0.adls |
| SCOAT | basics · SCOAT_object_empty.v1.0.0.adls |
| SDINV | legacy_adl_1.4 · FAIL_c_dv_quantity_minimal.v1.adl *(filename FAIL)* |
| SEXLU | structure · SEXLU_attribute_wrong_existence.v1.0.0.adls *(SEXLU1/2 family)* |
| STCNT | consistency · VOTM_terminology_term_definitions_empty.v1.0.0.adls *(filename VOTM)* |
| SUNK | basics · FAIL_definition_missing.v1.0.0.adls *(filename FAIL)* |
| VACDF | consistency · VACDF_ac_code_in_definition_not_in_terminology.v1.0.0.adls |
| VACMCU | structure · VACMC_occurrences_too_big.v1.0.0.adls *(filename VACMC)* |
| VACSD | specialisation · VACSD_wrong_spec_level.v1.0.0.adls |
| VACSD | specialisation · spec_test_obs-VACSD_wrong_concept_spec_level.adls |
| VACSD | specialisation · VACSD_concept_code_wrong_specialisation_level.v1.0.0.adls |
| VACSO | structure · VACSO_attribute_wrong_cardinality.v1.0.0.adls |
| VARCN | basics · VARCN_illegal_concept_code.v1.0.0.adls |
| VARDT | rm_checking · VARDT_rm_type_wrong_capitalisation.v1.0.0.adls |
| VARXID | slots · VARXID_filler_id_not_valid.v1.0.0.adls |
| VARXR | slots · VARXR_slot_id_match_but_not_found.v1.0.0.adls |
| VARXR | templates · t_non_existent_ext_ref.v1.0.0.adls *(filename no code)* |
| VARXS | slots · VARXS_slot_id_mismatch.v1.0.0.adls |
| VATDA | structure · VATDA_at_code_assumed_code_not_in_list.v1.0.0.adls |
| VATDF | consistency · VATDF_at_code_in_ordinal_not_in_terminology.v1.0.0.adls |
| VATID | consistency · VATID_concept_code_not_in_terminology.v1.0.0.adls |
| VATID | consistency · VATID_id_code_in_node_not_in_terminology.v1.0.0.adls |
| VCACA | structure · VCACA_invalid_cardinality.adls |
| VCAEX | rm_checking · VCAEX_rm_non_conformant_existence.v1.0.0.adls |
| VCARM | rm_checking · VCARM_rm_non_existent_attribute.v1.0.0.adls |
| VCOID | basics · VCOID_container_attribute_children_no_node_identifiers.v1.0.0.adls |
| VCOID | basics · VCOID_missing_ids_on_alternative_children.v1.0.0.adls |
| VCOID | basics · VCOID_objects_with_no_node_identifiers.v1.0.0.adls |
| VCOID | basics · WHOLE.VCOID_missing_root_node_id.v1.0.0.adls |
| VCOID | paths · CAR.VCOID_uncoded_interior_nodes.v1.0.0.adls |
| VCORM | rm_checking · VCORM_rm_non_existent_type.v1.0.0.adls |
| VCORMT | rm_checking · VCORMT_rm_non_conforming_type1.v1.0.0.adls |
| VCORMT | rm_checking · VCORMT_rm_non_conforming_type2.v1.0.0.adls |
| VCORMT | specialisation · VCORMT_illegal_redef_of_ac_code_node.v1.0.0.adls |
| VCORMT | specialisation · VCORMT_redefine_rm_type.v1.0.0.adls |
| VDIFP | specialisation · VDIFP_invalid_path.v1.0.0.adls |
| VDIFP | specialisation · VDIFP_path_not_in_parent.v1.0.0.adls |
| VDIFP1 | specialisation · VDIFP_non_matching_path.v1.0.0.adls *(variant of VDIFP)* |
| VDSEV | slots · VDSEV_slot_include_any_exclude_any.v1.0.0.adls |
| VDSEV | slots · VDSEV_slot_include_not_any_exclude_not_any.v1.0.0.adls |
| VDSSID | slots · VDSSID_slot_redefine_bad_id.v1.0.0.adls |
| VETDF | terminology · VETDF_wrong_property_code.v1.0.0.adls |
| VOKU | terminology · VOKU_ac_code_duplicated_in_terminology.v1.0.0.adls |
| VOKU | terminology · VOKU_at_code_duplicated_in_terminology.v1.0.0.adls |
| VOLT | consistency · VOTM_terminology_term_definitions_of_original_language_missing.v1.0.0.adls *(filename VOTM)* |
| VOTM | basics · FAIL_dadl_spurious_delimiter.v1.0.0.adls *(filename FAIL)* |
| VOTM | consistency · VOTM_terminology_term_definitions_of_other_language_missing.v1.0.0.adls |
| VPOV | specialisation · VPOV_redef_ac_code_node_to_local_codes.v1.0.0.adls |
| VPOV | terminology · VPOV_code_list_constrained.v1.0.0.adls |
| VRANP | annotations · VRANP_annotations_wrong_rm_path.v1.0.0.adls |
| VRANP | annotations · VRANP_annotations_wrong_path.v1.0.0.adls |
| VRDLA | basics · VRDLA_inconsistent_lang_codes.v1.0.0.adls *(NOT in catalogue — §3c)* |
| VSAM | rm_checking · VSAM_rm_cardinality_on_single_attr.v1.0.0.adls |
| VSAM | rm_checking · VSAM_rm_wrong_multiple_attr.v1.0.0.adls |
| VSANCC | specialisation · VSANCC_redefine_cardinality.v1.0.0.adls |
| VSANCE | specialisation · VSANCE_redefine_existence.v1.0.0.adls |
| VSONCO | specialisation · VSONCO_redefine_occurrences.v1.0.0.adls |
| VSONCOm | specialisation · new_VSONCO-redef_to_multiple_singles-FAIL.v1.0.0.adls *(variant of VSONCO)* |
| VSONIN | specialisation · VSONIN_override_obj_not_in_parent.v1.0.0.adls |
| VSSM | specialisation · address-VSSM_invalid_order_node_id.v1.0.0.adls |
| VSSM | specialisation · VSSM_added_nodes_ordered.v1.0.0.adls |
| VTLC | consistency · VTLC_ac_code_not_in_all_languages.v1.0.0.adls |
| VTLC | consistency · VTLC_at_code_in_coded_term_not_in_all_languages.v1.0.0.adls |
| VTLC | consistency · VTLC_at_code_in_ordinal_not_in_all_languages.v1.0.0.adls |
| VTLC | consistency · VTLC_missing_constraint_definitions_in_one_language.v1.0.0.adls |
| VTLC | consistency · VTLC_node_id_not_in_all_languages.v1.0.0.adls |
| VTPL | templates · template_fail_VTPL.v0.0.1.adls |
| VTSD | specialisation · VTSD_ac_code_wrong_specialisation_level.v1.0.0.adls |
| VTSD | specialisation · VTSD_at_code_wrong_specialisation_level.v1.0.0.adls |
| VTSD | terminology · VTSD_terminology_code_from_higher_level.v1.0.0.adls |
| VTSD | terminology · VTSD_terminology_code_from_lower_level.v1.0.0.adls |
| VTTBK | terminology · VOTBK_term_bindings_bad_paths.adls *(filename typo VOTBK)* |
| VTVSMD | consistency · VTVSMD_at_code_in_coded_term_not_in_terminology.v1.0.0.adls |
| VTVSUQ | domain_types · VTVSUQ_at_code_duplicated_in_ordinal.v1.0.0.adls |
| VTVSUQ | terminology · VTVSUQ_at_code_duplicated_in_internal_codes.v1.0.0.adls |
| VUNP | paths · CAR.VUNP_internal_ref_bad_path.v1.0.0.adls |
| VUNP | structure · VUNP_attribute_use_node_missing_path.v1.0.0.adls |
| VUNP | structure · VUNP_attribute_use_node_path_isnt_object.v1.0.0.adls |
| WACMCL | structure · WACMCL_container_items_out_of_bounds.v1.0.0.adls *(warning)* |
| WOUC | terminology · WOUC_ac_code_unused.v1.0.0.adls *(warning; NOT in catalogue)* |
| WOUC | terminology · WOUC_at_code_unused.v1.0.0.adls *(warning; NOT in catalogue)* |

**PASS-that-looks-like-a-code:** `validity/domain_types/…VCOV_value_duplicated_in_ordinal`
has tag `PASS` ("a duplicated value in an ordinal is currently valid"). Do NOT
expect a VCOV failure — assert clean.

### 2c. Filename-vs-tag mismatches (13) — read the tag, never the filename

`VCARM_table`→FAIL · `FAIL_terminology_missing`→SADF · `VOTM…_empty`→STCNT ·
`FAIL_definition_missing`→SUNK · `VACMC…`→VACMCU · `VOTM…_original_language_missing`→VOLT ·
`FAIL_dadl_spurious_delimiter`→VOTM · `t_non_existent_ext_ref`→VARXR ·
`VOTBK…`→VTTBK · `FAIL_c_dv_quantity_minimal`→SDINV · `VDIFP_non_matching_path`→VDIFP1 ·
`new_VSONCO-…-FAIL`→VSONCOm · `VCOV…`→PASS.

### 2d. Corpus-wide code frequency (authoritative `regression` tag)

`PASS`×192 · `FAIL`×9 · `VCOID`×5 · `VTLC`×5 · `VCORMT`×4 · `VTSD`×4 ·
`VACSD`×3 · `VUNP`×3 · (`SADF VARXR VATID VDIFP VDSEV VOKU VOTM VPOV VRANP VSAM
VSSM VTVSUQ WOUC`)×2 each · singletons: `SCAS SCOAT SDINV SEXLU STCNT SUNK
VACDF VACMCU VACSO VARCN VARDT VARXID VARXS VATDA VATDF VCACA VCAEX VCARM VCORM
VDIFP1 VDSSID VETDF VOLT VRDLA VSANCC VSANCE VSONCO VSONCOm VSONIN VTPL VTTBK
VTVSMD WACMCL`. (18 files carry no tag — §9.)

---

## 3. Catalogue gap analysis

Catalogue = the S-code set of `ADL2/master04.6` §Syntax Validity Rules and the
V-/W-code set of `AOM2/master08-validation.adoc` (phases 1/2/3 + class
invariants + specialisation rules of `ADL2/master09.02`–`master09.10`).
"Present" normalizes author variants: `VDIFP1`→VDIFP, `VSONCOm`→VSONCO,
`SEXLU`→SEXLU1/SEXLU2, `VACMC`→VACMCU.

### 3a. Catalogue codes WITH corpus coverage (49)

```
S: SADF SCAS SCOAT SDINV SEXLU1 SEXLU2 STCNT SUNK
V: VACDF VACMCU VACSD VACSO VARCN VARDT VARXID VARXR VARXS VATDA VATDF VATID
   VCACA VCAEX VCARM VCOID VCORM VCORMT VDIFP VDSEV VDSSID VETDF VOKU VOLT VOTM
   VPOV VRANP VSAM VSANCC VSANCE VSONCO VSONIN VSSM VTLC VTPL VTSD VTTBK VTVSMD
   VTVSUQ VUNP
W: WACMCL
```

`SEXLU1/2` and `VACMCU` are covered only via near-form filenames (`SEXLU`,
`VACMC`); confirm which sub-code the file triggers when writing the assertion
(the `SEXLU` file's purpose is "an attribute has existence greater than 1").

### 3b. Catalogue codes with ZERO corpus coverage — hand-written cases needed (85)

```
S-codes (37): SAAN SACO SADS SAIV SALA SALAN SAON SARID SASID SCBAV SCCOG SCDAV
  SCDPT SCDTAV SCDTPT SCDUAV SCDUPT SCIAV SCOAV SCRAV SCSAV SCSRE SCTAV SCTPT
  SDSF SEXLMG SEXLSG SEXPT SINVS SOCCF STCAC STCCP STCDC SUAID SUAIDI SUAS SUNPA
V-codes (48): VACMCO VALC VARAV VARD VARID VARRV VARXAV VARXNC VARXRA VARXTV
  VASID VATCD VATCV VCAM VCATU VCOCD VCORMEN VCORMENU VCORMENV VCOSU VDEOL VDFAI
  VDIFV VDSIV VDSSC VDSSM VDSSP VOBAV VRMVAV VRMVP VRRLP VRRLPAR VRRLPRM VSONCT
  VSONI VSONIF VSONIR VSONPI VSONPO VSONPT VSONT VSUNT VTCBK VTPIN VTPNC VTVSID
  VUNK VUNT
```

The SC*AV / SC*PT S-code family is the per-primitive value-constraint syntax
set — one malformed-primitive fixture per primitive type covers a cluster.
`VSONI`/`VSONIR` are recognise-deprecated-only.

### 3c. Corpus codes NOT in the catalogue

- **`VRDLA`** (1 case, `validity/basics`) — a language code used *inside* an
  ODIN sub-block (e.g. the inner `language = <[ISO_639-1::zh]>` of a
  `details["zh-cn"]` block) is inconsistent with the block's own key/outer
  language code. Resource-description language-code consistency; distinct from
  VTLC (terminology-language coverage). Verified from the file: the `zh-cn`
  details block declares inner `language` `zh`. → add a `VRDLA` variant or
  adjudicate against `AOM2/master08`.
- **`WOUC`** (2 cases, warning) — an at-code / ac-code defined in the
  terminology is unused anywhere in the definition (archie `ErrorType.WOUC`).
  Verified from the file (`purpose`: "at-code in the ontology is not used
  anywhere"). → add as a W-code warning alongside WACMCL.
- **`VOTBK`** — filename typo only; the tag is `VTTBK` (in catalogue). No new
  code.
- **`VCOV`** — filename only; the tag is `PASS`. No failure code.
- **`VDIFP1` / `VSONCOm`** — author sub-variant numbering of catalogue `VDIFP`
  / `VSONCO`. Decide whether the harness normalizes the trailing marker or the
  typed enum exposes sub-variants.

---

## 4. Structural feature census (whole corpus, 340 sources)

| Feature | # files | Note |
|---|---:|---|
| `specialise`/`specialize` | 114 | see §5 |
| `template` first keyword | 2 | `validity/templates/…template_{fail,pass}_VTPL` only |
| `template_overlay` block | 0 | not exercised |
| `use_archetype` | 9 | slot fill / external ref |
| `use_node` | 23 | proxy nodes |
| `allow_archetype` | 42 | slot definitions |
| tuples (`[a,b] matches {` / `[a,b] ∈`) | 33 | ordinal / quantity tuples |
| `rules` section | 4 | BEL assertions (`features/aom_structures/rules/*` + 1) |
| `annotations` section | 8 | |
| `rm_overlay` | 0 | not exercised |
| `group` cardinality constraint | 0 | not exercised |
| root node coding | id1 ×312 · at0000 ×22 | at0000 roots = the 1.4 `.adl` + legacy/upgrade forms |
| unicode operators (∈ ∧ ∃ ∼ ∉ ∗) | 34 | **all under `flattener/`**; `adl2-reference` uses text keywords only — both must lex |
| UTF-8 BOM | 29 | see §1 |

`group`, `rm_overlay`, `template_overlay` have zero corpus coverage — hand-write
fixtures if the parser is to be exercised on them.

---

## 5. Specialisation DAG

114 files declare a parent; **109 parents resolve within the corpus** (by
concept + version-family, including `org.openehr::`-namespaced refs and
hyphen↔underscore concept spellings).

**Orphan parents = 2, both intentional** (the failure IS the missing parent):
`validity/specialisation/FAIL_missing_parent.v1.0.0.adls` and
`…FAIL_missing_parent_term.v1.0.0.adls` both specialise
`openEHR-TEST_PKG-ENTRY.specialisation_parent.v1`, deliberately absent — assert
"parent not found."

Not orphans (resolved): the three `features/description/identification`
`…inherit_ns` files specialise `org.openehr::openEHR-EHR-OBSERVATION.full_id_1.v1`
→ `…full_id_1.v1.0.4.adls` (namespace-prefix + version-family match).

---

## 6. `robustness/` (3 files) — per-file assertion

All three carry `regression = "PASS"`: valid archetypes with intentionally
empty sub-structures (not broken sources). Assert **parse + phase-1..3 validate
clean** (the "never panic" floor is subsumed by PASS).

| File | Exercises |
|---|---|
| `…invariant_empty.v1.0.0.adls` | empty definition body / empty invariant |
| `…terminology_term_binding_empty.v1.0.0.adls` | `term_bindings = <["TEST"] = < >>` empty binding group |
| `…empty_bindings.v1.0.0.adls` (BOM) | empty `term_bindings`; `data` attr with no constraint |

---

## 7. Flattener fixtures (38) — expected outputs NOT vendored

No `regression` tag and no stored flat/`.adlf` golden — archie's expected flats
live in its Java test classes, which are not vendored. Per `PROVENANCE.md`, the
expected flat must be **hand-authored and spec-verified** (`AOM2/master08`
§Flattening, `ADL2/master09.02`–`master09.10`) when the flattener lands.
Harness: load parent(s) + child, flatten child against the flat parent, assert
against the hand-written expected (structural assertions or an
author-verified printed-flat snapshot).

**`specexamples` pairs (child → parent):**

```
diagnosis → problem
cardinality_specialized → cardinality_parent
occurrences_specialized → occurrences_parent
numeric_primitive_specialized → numeric_primitive_parent
tuple_specialized → tuple_parent
type_refinement_specialized → type_refinement_parent
add_tuple → type_refinement_parent            (cross-pair, not tuple_parent)
interval_value_set_specialized → internal_value_set_parent   (internal→interval intentional)
reference_redefinition_specialized → reference_redefinition_parent
reference_redefinition_no_replacement → reference_redefinition_parent
protocol_exclusion → empty_observation
protocol_mandatory → empty_observation
lipid_studies_panel → laboratory_test_panel   (declared parent HRID EVALUATION vs file CLUSTER — verify)
specialization_paths → lab_test (file lab-test.v1.0.0, hyphen→underscore HRID)
```

Root / parent-only (flatten to self, or serve as a parent):
`cardinality_parent, occurrences_parent, numeric_primitive_parent, tuple_parent,
type_refinement_parent, internal_value_set_parent, reference_redefinition_parent,
empty_observation, laboratory_test_panel, lab-test, problem`.

**`siblingorder`:** parent `order-parent.v1.0.0` has 7 children —
`redefinition_at_same_place, reorder_parent_nodes,
sibling_order_redefined_node_id{,_2,_3}, specialise_first_element,
test_anchoring, tricky_edge_case`. Plus `archetype_slot_filled →
archetype_slot_parent` and `siblingorderchild → siblingorderparent`.

---

## 8. `upgrade/upgrade_from_14` pairings (9 `.adl` → 9 `.adls`)

Convert-and-compare; pair by concept name (version suffix is NOT a uniform
`.v1`→`.v1.0.0` rule):

```
id_codes_as_at_codes.v1.adl        → .v1.0.0.adls
exclusion-adverse_reaction.v1.adl  → .v1.0.0.adls
exclusion.v1.adl                   → .v1.0.0.adls
inherit_unchanged_node.v1.adl      → .v1.0.0.adls
inherit_unchanged_parent.v1.adl    → .v1.0.0.adls
upgrade_add_use_nodes.v1.adl       → .v1.0.0.adls
upgrade_parent.v1.adl              → .v1.0.0.adls
adl14_meta_data.adl                → adl14_meta_data.v0.0.1-alpha.adls   (irregular)
test_regex.v1.adl                  → test_regex.v1.1.0.adls             (irregular: v1.1.0)
```

No unpaired files. `upgrade/upgrade_from_15` (8 `.adls`, no `.adl`) has no
conversion input vendored — parse + validate clean, not convert-compare.

---

## 9. The 18 untagged files (all PASS-expected)

`features` (10): `aom_structures/tuples/{lab_analyte-quantity,
lab_analyte-triglycerides, ACTION.medication, ACTION.medication_precise}`,
`editor/PERSON.test`, `specialisation/{empty_observation, nested_diff_paths,
protocol_diff_overlay}`, plus `validity/templates/{de_en_lang_arch,
de_lang_arch}` (support archetypes for template includes). `upgrade` (6): the
conversion `.adl`/`.adls` members that omit a tag + the 2 CIMI
`upgrade_from_15` files.

Convention: an absent tag in `features/**`/`upgrade/**` ⇒ PASS-clean. An absent
tag in `validity/**` should be a hard coverage-gate error (only the two
`validity/templates` support archetypes above are untagged in `validity`).

---

## 10. Harness-category assignment (the coverage gate)

| Directory / selector | Harness | Assertion oracle |
|---|---|---|
| `features/**/*.adls` | parse-clean + validate-PASS; if `specialise` present, also flatten | `regression` tag (PASS / absent) |
| `features/**/*.adl` (`intervention_decisions.v0.adl`) | 1.4-tolerant parse | tag |
| `validity/**/*.adls` (all tagged) | validate-expect-code: PASS→clean; FAIL→any typed error; code→exactly that code; W*→warning | `regression` tag (NOT filename) |
| `validity/legacy_adl_1.4/*.adls` | parse + validate clean | tag = PASS |
| `validity/legacy_adl_1.4/*.adl` | 1.4-tolerance parse; `FAIL_c_dv_quantity_minimal.v1.adl`→SDINV | tag |
| `robustness/**` | parse + validate → assert PASS (never-panic floor) | tag = PASS |
| `upgrade/upgrade_from_14/*.adl` | convert (adl14) → compare to paired `.adls` | the paired `.adls` |
| `upgrade/upgrade_from_14/*.adls` | parse + validate clean (also the compare target) | — |
| `upgrade/upgrade_from_15/*.adls` | parse + validate clean (no conversion input) | — |
| `flattener/specexamples/**`, `flattener/siblingorder/**` | flatten child→parent, compare to hand-authored spec-derived expected | `AOM2/master08` §Flattening; `ADL2/master09` (hand-authored) |

**Decisions needed for a zero-ambiguity 100% claim:**

1. The **31 `.adl` files** (21 legacy/upgrade + 9 upgrade_from_14 + 1 feature)
   are not in the current `.adls`-only `corpus_lex`/`corpus_outer_parse`
   harnesses — add a `.adl` walker (adl14/tolerance category) or they read as
   unclaimed.
2. Confirm `upgrade_from_15` is parse-clean (no source pair to convert).
3. Flattener fixtures cannot compare to a vendored golden — the category is
   "flatten + hand-authored assertion"; parent-only files are claimed via
   "flattens to self / used as parent."
4. Normalization rule for `VDIFP1`/`VSONCOm`/`SEXLU`/`VACMC` so
   "filename code MUST raise exactly that code" matches the tag.
5. Add `VRDLA` + `WOUC` to the catalogue (or adjudicate) so their 3 files are
   claimable.
6. Gate keys on full path (2 duplicate basenames) and the lexer strips the BOM
   (29 files).
