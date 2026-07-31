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
  give full normative text for most codes. It is a PROSE bullet list under
  `====` sub-headings (Basic checks / AUTHORED_ARCHETYPE meta-data checks /
  Definition Structure Validation / Basic Terminology Validation / Various
  Structure Validation / Code Validation / Validate Annotations), NOT a table.
  CRITICAL: master08 carries NO severity column/markings — Error vs Warning is
  NOT stated here (inferable only from the V/W name prefix). W-codes WACMCL,
  WOUC, WOSU, WOTC are ABSENT from master08 entirely; so is VSONIF (its full
  text + the phase-2 sibling-identity role live in master04.5 line 356-357,
  where it forward-references VACMI which is itself undefined tree-wide — an
  archie/spec-draft dangling ref). VRDLA + "resource description language"
  phrase: absent tree-wide (archie-only).
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

PHASE-ARCHITECTURE facts (master08, verified 2026-07-31) — needed whenever a
question asks "is skipping phase 3 legal?":
- master08 L3 self-describes as "a guide for validation, flattening and diffing,
  **based on the ADL workbench reference compiler**"; §Validation L18 says
  "Validation is **best implemented** in a multi-pass fashion". The 3-phase split is
  therefore ADVISORY ORDERING, not a conformance profile menu.
- The L5-14 processing sequence nests BOTH phase 2 and phase 3 under
  "if passed, and **A is specialised**:" — a literal reading exempts top-level
  archetypes from phase 3. That reading is defeated by
  `docs/specs/openehr/AM/docs/ADL2/master09.02-spec_concepts.adoc` L7:
  "**For a top-level archetype, the flat-form is the same as its differential form**"
  — so the flat form always exists and phase-3 rules are always evaluable.
- §Phase 3 - Validation of Flat Form (L107-112) contains exactly TWO codes: VUNP + VACMCO.
- The V-codes themselves carry NO phase/conditionality qualifier: VUNP + VUNT + VSUNT
  live under master04.5 §"Validity Rules: C_COMPLEX_OBJECT_PROXY" (L475+);
  VACMCU + VACMCO under master04.5 §"Validity Rules: C_ATTRIBUTE" (L121+, in the
  "container attributes / is_multiple = True" sub-paragraph). master03 §Validity Rules
  (L205) opens "The following validity rules apply to **all varieties of ARCHETYPE
  object**".
- Codes whose OWN text is scoped to the flat form: VATDF (master03 L220 "...terminology
  of the flattened form of the current archetype") and VTVSMD (master07 L65, same
  phrase). VACDF by contrast says "of the current archetype" (NOT flattened) — the
  VATDF/VACDF asymmetry is in the released text.
- **SPEC SILENCE:** grep for "partial validation" / "staged validation" /
  "validation profile" / "level of validation" across the whole AM tree returns ZERO
  hits. No reduced/partial validation profile is defined or permitted anywhere.

Generated am24 tree `crates/openehr-am/src/am24/aom2/` covers the class model
1:1 (archetype/ constraint_model/ primitive/ terminology/ rules/ rm_overlay/
profile/ definitions/ persistence). DIFFERENTIAL_ARCHETYPE / FLAT_ARCHETYPE are
prose-only variants in master03, NOT BMM classes → no generated type (not a
gap). BEL rules base classes (Assertion, Expression, ExprLeaf, ExprOperator,
ExprBinaryOperator, ExprValue) live in `openehr-lang::beom`, not openehr-am.
