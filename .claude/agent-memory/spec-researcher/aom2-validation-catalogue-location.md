---
name: aom2-validation-catalogue-location
description: Where the AOM2 semantic model + the full V-code validation catalogue live in the vendored AM specs, and which codes are spec-silent
metadata:
  type: reference
---

AOM2 spec text: `docs/specs/openehr/AM/docs/AOM2/master*.adoc`.

Validation catalogue split (the V-codes):
- `master08-validation.adoc` = the ORCHESTRATION chapter: lists every code by
  the 3 phases (Phase 1 basic integrity / Phase 2 vs flat parent / Phase 3 flat
  form) with a one-line meaning each, plus flattening/diffing prose. It does NOT
  give full normative text for most codes.
- Full normative text of each code is scattered in the CLASS-DEFINITION files:
  - `master03-archetype_package.adoc` §Validity Rules → ARCHETYPE-level codes
    (VARAV VARRV VARCN VATDF VACDF VATDA VETDF VOTM VOKU VARDT VRANP VRRLP VARID
    VDEOL VARD VASID VALC VACSD VATCD VTPL).
  - `master04.5-constraint_model-class_definitions.adoc` → C_ATTRIBUTE /
    C_OBJECT / C_COMPLEX_OBJECT / C_ARCHETYPE_ROOT / ARCHETYPE_SLOT /
    C_COMPLEX_OBJECT_PROXY / C_PRIMITIVE_OBJECT codes (VCARM VCAEX VCAM VDIFV
    VDIFP VSANCE VSAM VACSO VACMCU VACMCO VCACA WACMCL VSANCC VCORM VCORMT VCOCD
    VCOID VCOSU VSONT VSONCT VSONIN VSONIF VSONCO VSONPT VSONPI VSONPO VSSM VCATU
    VARXNC VARXAV VARXTV VARXR VARXS VARXID VDFAI VDSIV VDSEV VDSSID VDSSM VDSSP
    VDSSC VUNT VUNP VSUNT VOBAV; VSONIR + VSONI are marked _Deprecated_) — also
    the c_conforms_to / c_congruent_to Eiffel conformance functions per type.
  - `master07-terminology_package.adoc` → VTVSID VTVSMD VTVSUQ VTSD VTLC VTTBK VTCBK.
  - `master06-rm_overlay.adoc` → VRMVP VRMVAV.

SPEC-SILENCE: several codes referenced in master08 have NO full text anywhere in
the vendored AM tree (only the master08 one-liner) — the authoritative full text
lives in the EXTERNAL file github.com/openEHR/adl-resources
messages/ADL/adl_syntax_errors.txt (NOT vendored). These: STCNT, VOLT, VATCV,
VATID, VARXRA, VCORMENV, VCORMENU, VCORMEN, VPOV, VUNK, VTPNC, VTPIN, VRRLPRM,
VRRLPAR. ADL2 dir does NOT define V-codes with the `*Vxxx*:` bold form (grep
empty) — it only has SOCCF-style cADL *syntax* error codes in
`ADL2/master04.6-cadl_validity_rules.adoc`.

Generated am24 tree `crates/openehr-am/src/am24/aom2/` covers the class model
1:1 (archetype/ constraint_model/ primitive/ terminology/ rules/ rm_overlay/
profile/ definitions/ persistence). DIFFERENTIAL_ARCHETYPE / FLAT_ARCHETYPE are
prose-only variants in master03, NOT BMM classes → no generated type (not a
gap). BEL rules base classes (Assertion, Expression, ExprLeaf, ExprOperator,
ExprBinaryOperator, ExprValue) live in `openehr-lang::beom`, not openehr-am.
