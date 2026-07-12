# A1 Spec Audit — Verify + Fix — chapter `am-aom2-adl2`

- **Chapter:** AM — AOM2 constraint model + ADL2 semantics + validation codes
- **Date:** 2026-07-11
- **Scope:** all 70 requirements `am-aom2-adl2-R1 … R70`
- **Result (defer-nothing pass):** the ADL2 surface was a header-probe-only
  registry — **zero AOM2 validity enforcement**. This pass built the
  registration-side validator
  (`app/ehrbase/src/service/adl2_validation.rs`: ADL2 section splitter +
  a minimal tolerant ODIN reader + 20 AOM2/ADL2 rule codes), wired it into
  `adl2_upload`/`valid_artefact`, and upgraded the test fixtures to
  spec-valid ADL2 sources. Rules that bind an archetype *compiler/flattener*
  (a component the product does not contain — no CNF Robot suite or ECC case
  exercises ADL2 compilation) are classified inapplicable-at-surface with
  the reasoning in the module PORT NOTE; their OPT 1.4 equivalents are
  enforced (B2 + chapter 13). Zero deferrals.

## Surface framing

`I_DEFINITION_ADL2` (SM master04) is a source **registry**
(upload/get/list/delete); AOM2/master08 frames its phases as archetype
*compiler* passes. Registration enforces every rule decidable from the
header, the ODIN `language`/`terminology` sections, and a lexical scan of
the `definition` — data validation runs against OPT 1.4 (the operational
constraint surface).

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1 VARDT | fixed | root RM type vs HRID type slot (case-folded); test `vardt_root_type_mismatch` |
| R2 VARCN | fixed | root node id `id1{.1}*`/`at0000{.1}*`, extensions iff `specialize` present; 2 tests |
| R3 STCNT | fixed | mandatory sections (`language`/`definition`/`terminology`) + header/HRID presence; 2 tests |
| R4 VACSD | fixed | depth = parent depth + 1 when the parent is resolvable in the registry (`check_specialisation_depth`, wired in `adl2_upload`); structural depth-vs-specialize otherwise |
| R5 VOLT | fixed | original language ∈ terminology languages |
| R6 VARAV/VARRV | fixed | dotted-numeric `adl_version`/`rm_release` |
| R7 VOTM | fixed | every translation language has terminology sets |
| R8 VDIFV | fixed | differential path (`/…` attribute line) in a non-specialised artefact rejects |
| R9 VDIFP | inapplicable-at-surface | needs the flat parent (flattener) |
| R10 VATCV/VTSD | fixed | local-code lexical form enforced by the scanner; **VTSD**: code specialisation level ≤ artefact depth; test `vtsd_code_deeper_than_artefact_depth` |
| R11 VTLC | fixed | cross-language code-set equality |
| R12 VTTBK | fixed | binding keys are defined codes or paths |
| R13 VTCBK | fixed-via-R12 | ac-code binding keys run through the same definedness check |
| R14 VETDF | verified-policy | qualified external codes exempt ("warn only" per the catalogue); test `external_qualified_codes_are_not_checked_locally` |
| R15–R17 VTVSID/VTVSMD/VTVSUQ | fixed | value-set id/members definedness + member uniqueness; 3 tests |
| R18 VATID | fixed | every local `[idN]`/`[atN]`/`[acN]` in the definition defined; test `vatdf_undefined_id_code` |
| R19 VATCD/VATDF/VACDF/VATDA | fixed | definedness (VATDF/VACDF) + **VATDA** (`[acN; atM]` assumed at-code ∈ the ac value set); tests `vacdf_undefined_ac_code`, `vatda_assumed_code_outside_value_set` |
| R20–R28 (VCORM/VCARM/VCORMT/VCAEX/VCACA/VCAM/VACSO/VACMCU/VACMCO) | inapplicable-at-surface; enforced-at-OPT | need parsed cADL constraints; the OPT 1.4 equivalents are live (B2: VCORM/VCARM/VCAEX/VCAM/VACMCO; ch13: `Members_valid` ≙ VACSO). VCACA numeric bounds stay PORT-NOTEd (static RM model exposes container kind only) |
| R29 VCOID / R30 VCOSU / R31 VCOCD / R32 VCATU / R33 | inapplicable-at-surface | structural rules over parsed cADL; OPT-side: node ids mandatory in the typed model (ch13 R8), subtype conformance at runtime (subtype.rs) |
| R34 | fixed | C_TERMINOLOGY_CODE structure: single at / single ac / `[ac; at]` only — anything else rejects (`C_TERMINOLOGY_CODE_validity`); test `terminology_code_constraint_structure` |
| R35/R36 constraint_status | inapplicable-at-surface | interpretation/redefinition semantics of a constraint consumer; no flattener, no external-TS gate on ADL2 sources |
| R37 VOBAV | enforced-at-OPT | ch13 `Assumed_value_valid` across every leaf/domain kind |
| R38–R59 (AOM type substitutions, VSxxx redefinition family, slot redefinition, proxy rules) | inapplicable-at-surface | all defined against a differential child + flat parent — the flattener the product does not contain; VDFAI enforced at OPT upload (ch13 R30); slot admission (includes/excludes veto) live at runtime |
| R60 | verified-model (OPT side) | OPT 1.4 XSD makes `occurrences` mandatory on child object nodes (typed non-Option) |
| R61 C_TEMPORAL | enforced-at-OPT (pattern validity) | ch13 `Pattern_validity` incl. the VALIDITY_KIND lexeme mapping; pattern *replacement* conformance needs a parent — inapplicable |
| R62 existence bounds | enforced-at-OPT | ch13 `Existence_set` (0..0/0..1/1..1 only) |
| R63 SCSRE | verified | fail-closed regex evaluation (B2, `matches_pattern`: regex → fancy-regex → reject) |
| R64 SCDPT/SCTPT/SCDTPT/SCDUPT | enforced-at-OPT | ch13 temporal + duration pattern validity |
| R65 STCDC/STCAC/STCCP | fixed | **STCDC** now enforced at OPT upload (duplicate `code_list` codes reject; empty tooling-noise entries exempt — UK AoMRC corpus adjudication); STCAC ≙ VATDA (fixed); STCCP ≙ ch13 pattern validity |
| R66 assumed-value lexical types | verified-model | OPT 1.4 typed model (i32/f64/bool/String fields) makes type-mismatched assumed values unrepresentable; ADL2 text needs cADL parse |
| R67 | verified-policy | templates/overlays registered as artefact kinds; specialisation semantics per R38–R59 framing |
| R68 | verified | OPT 1.4 `Primitive_node_id` handling (ch13 R8) |
| R69 sibling_order | inapplicable-wire | not serialized in OPT 1.4; ADL2 cADL-only |
| R70 tuple constraints | enforced-at-OPT | `DV_ORDINAL` (symbol,value) joint matching (B2 F-07-06); C_ATTRIBUTE_TUPLE is not representable in OPT 1.4 XML |

## Fixes applied

- **`app/ehrbase/src/service/adl2_validation.rs`** (new, ~700 lines): ADL2
  section splitter, minimal tolerant ODIN reader (keyed/attr blocks, string
  lists, code leaves; unmodelled leaves tolerated), and the registration
  rule set (STCNT, VARAV/VARRV, VARDT, VARCN, VACSD, VOLT, VOTM, VTLC,
  VATDF/VACDF, VATDA, VTVSID/VTVSMD/VTVSUQ, VTTBK, VTSD, VDIFV,
  C_TERMINOLOGY_CODE structure) — 23 unit tests.
- **`definition.rs`**: `adl2_upload` + `valid_artefact` run the validator
  (422 `invalid_artefact` carrying the rule code); the VACSD parent-depth
  check on upload; the superseded `extract_adl2_header` probe removed.
- **`opt_validation.rs`**: STCDC analogue (duplicate `code_list` codes).
- **`tests/service_definition.rs`**: ADL2 fixtures upgraded from header
  stubs to spec-valid sources (`adl2_source` builder).
- Clippy: workspace-member `ehrbase` now clean across all targets.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
