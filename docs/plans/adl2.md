# ADL2 — full implementation, spec-exact (the ADL2 worklist row)

Owner directive (2026-07-19): full ADL2 support — ADL2/cADL2/ODIN source
parser, the complete AOM2 semantic-validation catalogue, specialisation
flattening, OPT2, template semantics, and an in-CDR ADL 1.4 → ADL 2
migration story. **The vendored specs are the oracle**
(`docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/`); openEHR archie is prior
art only, never a parity target. This plan embeds the full requirements
extraction (researched first-hand from the vendored text, 2026-07-19) so
every coding session works from the same catalogue. Per the doc lifecycle,
this file is DELETED in the PR that closes the row; the durable record is
`docs/PROGRESS.md` + the living reference docs.

Branch: `feat/adl2` (renamed from `claude/adl2`, 2026-07-19 branch-naming
hard rule).

---

## Part I — Design

### The crate: `crates/openehr-adl` (hand-written, spec-versioned 2.4.0)

Precedent: `openehr-query` (hand-written AQL front end, logos + chumsky).
`openehr-am` stays generated data-types-only; ALL ADL2 behaviour lives in a
new hand-written spec crate:

- **`openehr-adl`** — the ADL2 text + semantics engine:
  - `lexer` + `parser` — ADL2 outer syntax, cADL definition section,
    rules/slot assertions (BEL subset).
  - **BEL lives in `openehr-lang` too (owner ruling 2026-07-19):** BEL is
    a LANG spec (`docs/specs/openehr/LANG/docs/BEL/`; the generated
    `openehr_lang::beom` model already realizes it). The core expression
    parser (statements, assertions, assignments, operators, literals →
    beom types) is a hand-written `openehr_lang::bel` module beside
    `odin`. The three AOM-specific expression leaves are AM classes
    (AOM2 master05: EXPR_ARCHETYPE_REF, EXPR_CONSTRAINT wrapping a cADL
    C_PRIMITIVE_OBJECT, EXPR_ARCHETYPE_ID_CONSTRAINT), so `openehr-adl`'s
    rules/slot-assertion parser COMPOSES: core grammar from
    `openehr_lang::bel` + the AOM leaf productions on top (this mirrors
    upstream: base_expressions.g4 sits in the adl grammar set because it
    imports cadl2_primitives — it stays vendored in openehr-adl).
    **EL is NOT needed** (owner-settled 2026-07-19): ADL2 rules depend on
    BEL (STABLE); EL is DEVELOPMENT-status and targets Task Planning /
    guidelines. Where the rules chapter defers operator semantics to the
    EL text (`master07.11-adl_rulesNEW`), cite the vendored
    `docs/specs/openehr/LANG/docs/EL/` — a documentation dependency only.
    If a future roadmap item needs EL, it lands as `openehr_lang::el`.
  - **ODIN lives in `openehr-lang` (owner ruling 2026-07-19):** ODIN is a
    LANG-component spec (`docs/specs/openehr/LANG/docs/odin/`), so the
    full ODIN value tree + self-contained lexer/parser is the new
    `openehr_lang::odin` module — completing that crate's mirror of the
    official LANG component set (ODIN + BMM + BMM3 + P_BMM + BEL). The
    ADL outer parser captures each ODIN section body as a span and calls
    `openehr_lang::odin::parse` (the ADL2 grammar likewise imports the
    ODIN grammar from LANG). ODIN parse failures map to SDINV in
    openehr-adl. The odin module is off the codegen path (bmm/beom
    modules untouched).
  - `ast`→AOM build — constructs `openehr_am::am24::aom2` values directly
    (the generated model is 1:1 complete vs the spec — verified).
  - `codes` — node-id/at/ac code math (depth, `codes_conformant`,
    specialisation-level parsing) per AOM2 master02/master07.
  - `paths` — ADL path grammar (master05) + differential/specialisation
    paths.
  - `validation` — the complete V-code catalogue (phases 1–3, Part III).
  - `conformance` — the master04.5 conformance functions
    (`c_conforms_to`, `occurrences_conforms_to`,
    `collective_occurrences_of`, `effective_occurrences`,
    `c_value_conforms_to` per primitive, …).
  - `flatten` — specialisation flattening + template overlay application.
  - `opt2` — OPT2 creation (raw + profiled) + `component_terminologies`.
  - `printer` — ADL2 serializer (differential + flat + OPT output; the
    REST `text/plain` bodies and round-trip tests need it).
  - `adl14` — ADL 1.4 → ADL 2 conversion (feeding from our parsed
    `am14` OPT/archetype structures; reproducible via a conversion log).
- Dependencies (downward only): `openehr-am`, `openehr-base`,
  `openehr-lang` (ODIN + BEL/beom expression classes), `openehr-term`
  where terminology validation needs it. App crates consume `openehr-adl`.
- RM checking (VCORM/VCARM/VCAEX/VCACA/VCAM/VCORMT) runs against the
  **BMM-generated RM attribute model** (`openehr-codegen -- emit-rm-model`
  — the same model the AQL engine types against), never reflection.

### Vendored assets to add (durable, with PROVENANCE.md)

1. **`crates/openehr-adl/vendor/grammar/`** — the normative ANTLR4
   grammars from `openEHR/adl-antlr` (Apache-2.0). The vendored spec text's
   syntax appendix (`ADL2/masterAppB-syntax_spec.adoc`) is only `include::`
   pointers at these files — they are the normative token
   regexes/productions and are currently NOT in the repo. Mirror the
   `openehr-query/vendor/grammar/` pattern (reference input for the
   hand-written parser, not a build input).
2. **`crates/openehr-adl/tests/corpus/adl2-tests/`** — the openEHR ADL2
   conformance corpus (~270 `.adls`; the ADL Workbench reference library).
   File names encode the expected rule code
   (`…VPOV_code_list_constrained….adls`, `FAIL_…`, `WOUC_…`) — harvest as
   a validator-conformance suite keyed by rule code. Vendor from the
   openEHR upstream for clean provenance (archie carries a copy; clinical
   CKM content is CC-BY — check per-file headers, code license does not
   cover model content).
3. Flattener behaviour fixtures: the AOM2-spec flattening examples +
   sibling-order cases (archie `tools/src/test/resources/com/nedap/archie/
   flattener/{specexamples,siblingorder}/` — these encode the spec's own
   worked examples; verify each against the spec text before pinning).

### Spec-silence register (each is a `// NOTE:` in code, per spec-adherence)

- The normative grammar files are external (`adl-antlr`) — vendored under
  item 1 above; the vendored prose gives semantics + examples only.
- Top-level section ORDER is example-derived (no vendored production).
- 14 V-codes have only a master08 one-liner, no full vendored text:
  STCNT, VOLT, VATCV, VATID, VARXRA, VCORMENV, VCORMENU, VCORMEN, VPOV,
  VUNK, VTPNC, VTPIN, VRRLPRM, VRRLPAR.
- `VARXRA` is not defined in master04.5 — treat as the C_ARCHETYPE_ROOT
  validity set (VARXNC/VARXAV/VARXR).
- `VSONPT` naming mismatch: master08 glosses it as differential-path
  existence; master04.5 L382 defines prohibited-node AOM-type validity.
  The class text is normative.
- `VSONI`/`VSONIR` are Deprecated — recognise, do not enforce. `VACMI`
  (referenced by VSONIF) is undefined — dangling cross-ref.
- OPT2 `master05-file_formats.adoc` is an empty stub — file naming is
  spec-silent (only master02's `.opt/.optx/.optj` extension list).
- Profiled-OPT node-level terminology substitution: explicit spec TBD.
- Description-inheritance `<<precursor>>`: explicit spec TBD.
- ADL 1.4 → ADL 2 / OPT migration: NO vendored-spec basis anywhere — the
  whole `adl14` module is "no openEHR spec governs this — our own
  design/extension", with archie's converter as prior art.
- `operational_template` vs `operational_archetype` keyword inconsistency
  in master07.04 — `operational_template` canonical, accept both.
- OPEN ADJUDICATION (A3a, 2026-07-19): corpus
  `features/aom_structures/tuples/openehr-ehr-ACTION.medication_precise…`
  (PASS-expected) uses attribute-tuple members that are COMPLEX objects;
  the pinned `cadl2.g4` (`c_primitive_tuple_item`) and AOM2
  `C_PRIMITIVE_TUPLE` (master04.3/04.5) admit only primitives. Resolve
  before the coverage gate closes: newer grammar form vs defective
  fixture — check upstream adl-antlr HEAD + AOM2 text, then either extend
  with citation or adjudicate the fixture.

### Prior-art takeaways adopted from archie (design only, no porting)

- Everything parameterised over the meta-model — matches our generated RM
  model; no reflection anywhere.
- Validation phases strictly gated (phase N+1 only if N passed); parent +
  overlay validation memoised in a repository; **phase 3 runs on the FLAT
  form after flattening**; OPT-creation failure is a warning, not an
  archetype error.
- Flattener: single-use, recursion-first-on-parent (flatten the parent
  fully, then overlay the differential child); sibling-order insertion via
  a moving anchor (`after` advances to last-inserted, `before` stays);
  cloning per the VSONCO single/multiple rule.
- 1.4 converter: conversion log (created codes/value sets) makes
  re-conversion idempotent; new codes allocated OUTSIDE the existing code
  number range; `at0000`→root/+1 segment shift; external single codes
  become synthesized at-codes + term-binding URIs (configurable templates);
  multi at-code lists become synthesized ac-code value sets; specialised
  1.4 archetypes convert flat then re-differentialise (a differ is
  required). Archie has NO 1.4 OPT ingestion — our `am14` OPT store needs
  its own front end feeding the same conversion core (our own design).
- Known archie gaps to NOT replicate: VCORMENV/VCORMENU/VCORMEN
  unimplemented, VETDF/VOKU/VDFAI missing, sibling-id collisions break
  path-keyed lookups. We implement the full catalogue.
- Tuples live outside the normal attribute tree (own list on
  C_COMPLEX_OBJECT — the generated model already has `attribute_tuples`);
  the representation leaks into converter/flattener/validators — settle
  helpers first.

---

## Part II — Syntax requirements (parser)

Oracle: `docs/specs/openehr/AM/docs/ADL2/` + `LANG/docs/odin/`. Complete
extraction; cite these sections in code.

### Lexical (master03, master04.2, master07.04)
- UTF-8 only; no BOM. Non-ASCII only in strings/regex/char literals.
  Escapes: `\r \n \t \\ \" \'` (anything else illegal); `\uHHHH` +
  `\uHHHHHHHH` optional; regex `\d`-style escapes pass through unparsed.
- Comments: `--` to EOL only. Template-overlay separator `----…` lines are
  comments.
- Identifiers: type = upper-initial [A-Za-z0-9_]*; generic types
  (`Interval<Quantity>`) per UML, case-insensitive + whitespace-ignored
  matching; attribute = lower-initial.
- Symbol/text equivalences (accept both): `matches`/`is_in`/`∈`,
  `not`/`~`/`∼` (+ `~matches`, `∉`), `*`/`∗`; assertions use `∧ ∃ ∼ ->`.
- Braces block structure; indentation cosmetic.

### Artefact level (master07.04–07.14, master02, master10)
- First keyword: `archetype` | `template` | `template_overlay` |
  `operational_template` (accept `operational_archetype`); flat forms
  prefixed `flat`.
- Keywords: + `specialise`/`specialize`, `language`, `description`,
  `definition`, `rules`, `terminology`, `annotations`. Deprecated:
  `invariant`→`rules`, `ontology`→`terminology`, `concept` obsolete.
  Keywords legal as identifiers inside definition/terminology.
- Section order (by example — see spec-silence): identification →
  specialise? → language → description → definition → rules? → symbols?
  (symbol_definitions/symbol_bindings) → terminology → annotations? →
  rm_overlay?; `component_terminologies` (OPT only, mandatory there).
- template_overlay: NO language/description (inherits); id line +
  specialize + definition + terminology; many per file after the root
  template.
- Identification (master07.05): `keyword ( meta; … )` + HRID line.
  Meta items: `adl_version=` (mandatory), `rm_release=` (mandatory),
  `uid=`, `build_uid=`, `provenance_id=`, `generated`,
  `controlled`/`uncontrolled`.
- HRID: `[ns::]publisher-package-class.concept.vN.M.P[-rc.N|-alpha.N]`;
  1.4 `v1` accepted → `v1.0.0`.
- Codes: at-code system (MANDATORY for openEHR RM: node ids `[atNNNN]`,
  root `at0000`) AND id-code system (`[idN]`, root `id1`) — ADL 2.4 dual
  support. `[atN]` term codes, `[acN]` value-set codes. Depth = number of
  dots (`at0001.1`, `at0.1` extension, `at0004.0.1`). OPT binding form
  `[acN@terminology]`. Primitive nodes get the AOM Primitive_node_id.

### cADL definition (master04.1–04.5)
- Object node: `TYPE[node_id] [occurrences ∈ {…}] ∈ { attrs }`; node id
  required on every object node. Attribute node:
  `name [existence ∈ {…}] [cardinality ∈ {…; ordered|unordered;
  unique|non-unique}] ∈ { objects }`.
- Existence values: {0},{0..0},{0..1},{1},{1..1} only. Occurrences: any
  interval incl. {0} (exclusion). Single-valued attr alternatives; "any"
  = bare `TYPE[id]` (accept deprecated `matches {*}`).
- `use_node TYPE[id] [occ] /abs/path` (proxy); `use_archetype
  TYPE[id, archetype-id] [occ]` (external ref / slot fill);
  `allow_archetype TYPE[id] [occ] ∈ { include/exclude assertions,
  closed }` (slot; empty = open; archetype-id regex meta-pattern
  `^.+-.+-.+\..*\..+$`).
- Primitives (04.5, brief + regular forms): Boolean {True,False;assumed};
  String "…" lists + `/re/` or `^re^` regex (PERL subset); Character
  single-quoted lists + classes; Integer/Real lists + ODIN intervals
  (`|a..b| |>a..<b| |a+/-b| |>=a|` …; Real requires decimal point);
  Date/Time/DateTime value lists/intervals + constraint patterns
  (`yyyy-mm-??`, `hh:??:XX`, TZ patterns; `??`→ only `??`/`XX` rightward,
  `XX`→ only `XX`); Duration patterns
  `P[Y|y][M|m][W|w][D|d][T[H|h][M|m][S|s]]` + mixed `pattern/|interval|`
  (`PWD/|P0W..P50W|`); Terminology `{[ac1]}` `{[at0004]}`
  `{[ac2; at0022]}` + strengths `required|extensible|preferred|example`
  + `[ac1@snomed_ct]`; assumed values `;`-suffixed everywhere; enums via
  underlying primitive.
- Second-order (04.4): tuples `[a, b] ∈ { [{c1},{c2}], … }` (replaces 1.4
  C_DV_QUANTITY/C_DV_ORDINAL; ordinal = `[value,symbol]` tuples); tuple
  paths use 1-based child index `value[2]`; group constraints
  `group cardinality ∈ {…} occurrences ∈ {…} ∈ { … }` (non-structural,
  nestable, both multiplicities mandatory).

### ODIN sections (LANG/odin master07-leaf_data + ADL 07.07/08/12/13/14)
- Structure `< attr = <v> … >`, keyed lists `["k"] = <…>`, typed cast
  `(TYPE) <…>`, embedded JSON `(json) <# … #>`.
- Leaves: 'c', "str" (multi-line, entity quoting), ints, reals (period
  only), True/False, ISO-8601 extended-form only (+ `??` partials),
  durations, intervals `|…|`, bare URIs, `[terminology::code]` (+ version
  `[snomed_ct(3.1)::…]`), comma lists (single-element trailing `, ...`).
- language: `original_language` (exactly 1) + `translations`; tolerate
  missing section (upgrade from old terminology headers).
- terminology: `term_definitions` (mandatory; at-coded ⇒ all at+ac codes;
  id-coded ⇒ container/alternative id-codes + all at+ac), `value_sets`
  (`id` + `members`), `term_bindings` (per-terminology URI maps; keys =
  at/id/ac code or code-terminated path). Deprecated accepted:
  `terminologies_available`, split constraint_definitions/bindings
  (merge), `items=<…>` wrapper, 0-padding differences.
- rm_overlay: `rm_visibility` map (path → visibility show|hide +
  alias at-code). annotations: `documentation` lang→path→tag map.
  symbols: `symbol_definitions` lang→sym→text, `symbol_bindings`
  sym→path.

### Rules section (master07.11-adl_rulesNEW; grammar = BEL, LANG/docs/BEL)
- `check <expr>` assertions + assignments; arithmetic, `implies`,
  `defined()`, `matches {…}` sub-constraints (EXPR_CONSTRAINT wrapping a
  C_PRIMITIVE_OBJECT), path leaves = EXPR_ARCHETYPE_REF; symbols bound via
  symbol_bindings. Slot include/exclude uses the same expression grammar.

### Paths (master05, master04.3/04.4, master09.02)
- `('/'? segment ('/' segment)+)`, segment `attr ['[' id ']']`; `//`
  movable; predicates: node-id, meaning name, position int, tuple index;
  display `[at0003|label|]`. External prefix `[archetype-id]/…`.
  Specialisation paths: down to first constrained-child attribute, every
  segment carries a node-id predicate, at-code overrides allowed inline.
  Runtime paths drop single-valued predicates.

### Syntax error catalogue (master04.6 — pin as the AM 2.4.0 set)
SUNK SARID SASID SACO SALA SALAN SADS SADF SAIV SAON SAAN SDSF SDINV
SCCOG SUAID SUAIDI SOCCF SUNPA SCOAT SUAS SCAS SINVS SEXPT SEXLSG SEXLU1
SEXLU2 SEXLMG SCIAV SCRAV SCDAV SCTAV SCDTAV SCDUAV SCSAV SCBAV SCOAV
SCDPT SCTPT SCDTPT SCDUPT SCSRE STCCP STCDC STCAC STCNT.
(meanings in `ADL2/master04.6-cadl_validity_rules.adoc` §Syntax Validity
Rules — a typed `SyntaxError` enum mirrors this list 1:1.)

### 1.4-tolerance set (accept + normalise, all spec-cited)
Single-number versions; `-`-specialised concept ids; `matches {*}`;
`invariant`/`ontology`/`concept` keywords; terminology deprecations above;
missing language section.

---

## Part III — AOM2 validation catalogue (the acceptance checklist)

Oracle: `AOM2/master08-validation.adoc` (phases) + full texts in
`master03`/`master04.5`/`master06`/`master07`. Every code below ships as a
typed variant + at least one corpus/unit case. Phases gate: 1 → 2 → flatten
→ 3.

**Phase 1 — basic integrity (standalone):**
VARDT VARCN STCNT VACSD · VOLT VARAV VARRV VOTM · VDIFV VDIFP ·
VATCV VTSD VTLC VTTBK VTCBK VETDF VTVSID VTVSMD VTVSUQ ·
VDSEV(+VDSIV co-algorithm) VARXRA→(VARXNC/VARXAV/VARXR) VARXTV ·
VATID VATCD VATDF VACDF VATDA · VRANP.
Plus class invariants not phase-listed: VOKU VARID VDEOL VARD VASID VALC
VTPL VRRLP VCOCD VCOID VCOSU VCATU VDFAI VDSIV VOBAV VRMVP VRMVAV
VACSO VACMCU WACMCL(warning) VSONIF.
Plus corpus-adjudicated additions (2026-07-19; no full vendored text —
NOTE-flagged, archie ErrorType parity): VRDLA (resource-description
language-code consistency) · WOUC (warning: defined terminology code
unused in the definition).

**Phase 2 — vs RM + vs flat parent (specialised only for the latter):**
RM: VCORM VCARM VCORMT VCAEX VCACA VCAM (+ VCORMENV VCORMENU VCORMEN —
spec-silent full text, implement from the master08 gloss).
Specialised: VSONPT VSONIN VSSM VSANCE VSANCC VSAM · VARXS VARXR VARXID
VDSSID VDSSM VDSSP VDSSC VARXNC VARXAV VARXTV · VSUNT VUNT · VSONT VSONCT
VSONCO(collective occurrences — see below) VSONPI VSONPO · VPOV VUNK
VTPNC VTPIN · VRRLPRM VRRLPAR.
Deprecated, recognise only: VSONI VSONIR.

**Phase 3 — flat form:** VUNP VACMCO (+VACMCU on containers).

**Conformance machinery (master04.5, normative Eiffel — implement 1:1):**
`c_conforms_to` / `c_congruent_to` / `existence_conforms_to` /
`cardinality_conforms_to` / `occurrences_conforms_to` /
`collective_occurrences_of` / `effective_occurrences` /
`c_value_conforms_to` per primitive. C_TERMINOLOGY_CODE conformance:
constraint_status order required(0)<extensible(1)<preferred(2)<example(3),
child ≤ parent, value-set-expansion subset; any strength ≠ required is
formally equivalent to NO constraint (master09.05 §Terminology Constraint
Redefinition).

**VSONCO — the load-bearing occurrences rule (master04.5 L359-379):**
single-occurrence parent (upper = 1): each child interval wholly contained
(children = runtime alternatives). Multiple-occurrence parent: the
*collective occurrences* of the specialised node set must **intersect** the
parent's flattened occurrences; collective lower = Σ member lowers,
collective upper = min(Σ member uppers [∞ if any unbounded], flattened
cardinality upper of the owning attribute); members without local override
inherit the parent node's occurrences.

---

## Part IV — Specialisation + flattening + OPT2 semantics

Oracle: `ADL2/master09.01–09.10`, `master10`, `AOM2/master08` §Flattening,
`OPT2/master02–05`. Key requirements (full verbatim quotes researched
2026-07-19; re-read the cited section when implementing):

- Covariance: every instance conforming to the child conforms to all
  parents (master09.01). Single inheritance. Differential authoring;
  child differences are vs the **flat parent** (master09.02).
- Depth = dot count in node ids; path congruence = strip one
  specialisation level from child-level ids to find the parent node
  (master09.02).
- Attribute redefinition (master09.04): multiplicity type immutable
  (VSAM); existence narrows (VSANCE; `{0}` = remove attribute+substructure);
  cardinality narrows (VSANCC, respect VACMCU/VACMCO/VCACA);
  sibling order `before`/`after` only in ordered containers, anchors from
  flat parent or local redefinitions (VSSM); default placement: redefined
  nodes in place, extensions at end; `before` defaults first / `after`
  defaults LAST available conforming sibling on anchor loss.
- Object redefinition (master09.05): id redefinition mandatory when
  splitting into multiple children; `atX.{0.}*N` redefined /
  `at0.{0.}*N` new-node forms (VSONIN); occurrences: mandation/exclusion
  for single-occurrence, collective set for multiple (VSONCO); prohibition
  `{0}` needs same AOM type (VSONPT), exact parent node_id (VSONPI), never
  on new nodes (VSONPO); cloning iff parent max effective_occurrences > 1
  and child is not sole max-1 replacement; exhaustive replacement =
  exclusion node LAST; RM subtype refinement (VSONCT) + meta-type rules
  (VSONT: terminal CCO→any non-primitive AOM type, proxy→CCO,
  slot→C_ARCHETYPE_ROOT); use_node target vs reference redefinition
  (inline expansion; re-targeting is NOT ADL); external refs narrow to
  descendant archetypes (VARXAV); slot fill (`use_archetype`, filler id =
  specialisation of slot id VARXID) / close (`closed`, VDSSP parent open,
  VDSSC close XOR narrow, VDSSM proper subset); terminology constraints
  only narrow, strengths only tighten toward required.
- Flattening (master08 §Flattening; sequence: parents top-down, suppliers
  recursive, parse → P1 → P2 → flatten → P3): overlay differential onto
  flat parent; handle differential paths (incl. overridden codes),
  meta-type overrides, sibling markers, cloning, deletions
  (`existence {0}` / `occurrences {0}`), proxy inline expansion on
  override. Section semantics: terminology term_definitions ACCUMULATE,
  value_sets REPLACE; description REPLACES; languages = intersection
  (new-only languages discarded); rules append with "and then" semantics;
  bindings override only toward specialised/subset targets.
- OPT2 (master02/03): OPT = compiled artefact; raw vs profiled (same
  formalism); the OPT checklist — all refs full-version-resolved, no
  specialise section, no sibling markers, no use_node (expanded), all
  fillers/external refs inlined, closed slots removed,
  `existence {0}` attributes + `occurrences {0}` objects removed, all
  overlays applied, `component_terminologies` = flat terminology of every
  constituent except the root (root keeps `terminology`). Grammar:
  `operational_template (…) HRID` + language description definition
  rules? terminology annotations? component_terminologies.
  Profiled processing (master04): annotations removal, language filtering
  (≥1 left), binding filtering, terminology substitution (node-level TBD
  in spec).
- Templates (master10): template = specialised archetype with fillers;
  overlays = local specialised archetypes (no language/description);
  the `template` keyword has no formal semantics.

---

## Part V — ADL 1.4 → ADL 2 conversion (our own design; archie prior art)

No openEHR spec governs 1.4→2 conversion — flag on the module. We store
ADL 1.4 OPTs (`template_store`, parsed via `openehr-am::am14`); the
converter consumes our parsed 1.4 structures (archie converts 1.4 *source
text* only — the OPT front end is ours).

Adopted strategy (validated against archie's `ADL14NodeIDConverter` +
`ADL14TermConstraintConverter` + `Differentiator`):
1. Version/meta: adl_version stamp, rm_release from config, `.v1` →
   `.v1.0.0`; OID uids → description other_details.
2. Node-id conversion: 1.4 at-codes → 2 codes with the first segment
   shifted (+1; `at0000`→root), leading-zero normalisation, path rewrites
   everywhere (proxies, bindings, annotations).
3. Term constraints: local at-lists → synthesized ac-code value sets;
   external single/multi codes → synthesized at-codes + term-binding URIs
   (configurable per-terminology URI templates; flag fabricated fallback
   URIs); 1.4 C_DV_QUANTITY/C_DV_ORDINAL → attribute/primitive tuples.
4. Missing node ids synthesized (match flat parent by path + RM type when
   specialised; `(synthesised)` term rubric).
5. Specialised 1.4 artefacts: convert against the converted+flattened
   parent, then re-differentialise (requires a differ).
6. Reproducibility: a conversion log (created codes/value sets) makes
   re-conversion idempotent; fresh codes allocated outside the existing
   number range.
7. CLUSTER.items-style cardinality corrections where 1.4 data is invalid
   ADL2.

---

## Part VI — REST/service integration (the seam this row completes)

Current state: `adl2_artefact` store + registration-subset validator
(`app/ehrbase/src/validation/adl2/`); `template_adl2.rs` serves text/plain;
`_example_get`/`_version_get` = 501; JSON/XML OPT projections = 406.

Target (ITS-REST `definition/template/adl2` group, dev-OAS):
- Upload: full parse + phase-1/2/3 validation (422 with rule codes on
  failure; real VACSD replaces the string-probe check); store parsed
  metadata (concept, archetype_id for TemplateMetadata in lists).
- Get: text/plain source (existing) + `application/json`
  OperationalTemplateV2 projection (serialize the flat/OPT AOM — replaces
  the 406) + XML where the OAS declares it.
- `_version_get`: implement (deprecated op, but spec-declared — serve it).
- `_example_get`: OPT2-driven example composition generation (shares
  design with issue #94's generator walk — keep the seam compatible).
- The registration validator in `app/ehrbase/src/validation/adl2/` is
  REPLACED by `openehr-adl` (delete the hand-rolled probe; service calls
  the real parser/validator). **Audit finding (2026-07-19, the
  generator-first hard-rule sweep):** `validation/adl2/odin.rs` is a
  confirmed shadow of `openehr_lang::odin` — its justifying comment
  ("no `openehr_lang::odin` item exists") went stale on this branch, it
  forks ODIN acceptance on the upload wire (422/409 path), and it carries
  a banned tracker ID (G-09-04). Its deletion is a named exit criterion
  of this phase, not a nice-to-have. (The only other audit finding — the
  cadl.rs Assertion placeholder — is being fixed in A3b; the rest of the
  consumer scope audited CLEAN: AQL uses the generated RM model, OPT
  validation uses opt14 DTOs, FLAT/ITS runtimes/REST contract all consume
  the generated model directly.)
- Admin/console + book chapter + changelog in the same PRs as the wire
  changes.

---

## Part VII — Phases (compiling, tested increments; each gates green)

- [ ] **A1 — scaffold + vendoring**: `crates/openehr-adl` (crate-scaffold
  skill), vendor adl-antlr grammars + adl2-tests corpus + flattener
  fixtures with PROVENANCE; wire `codegen-drift`-style provenance notes;
  crate CLAUDE.md.
- [ ] **A2 — lexer + ODIN + outer parser**: logos lexer (dual symbol/text
  tokens, ISO-8601/duration/pattern tokens); ADL outer sections; ODIN
  section parsing (reuse/extend `openehr-lang::odin`); identification +
  HRID; S-code syntax errors; corpus: every adl2-tests file LEXES.
- [ ] **A3 — cADL parser → AOM build**: full definition-section grammar
  (primitives, tuples, groups, slots, use_node/use_archetype, occurrences/
  existence/cardinality) building `am24::aom2`; rules/assertion parsing
  via BEL; printer (round-trip: parse→print→parse = equal); corpus: every
  well-formed adl2-tests file parses; property tests.
- [ ] **A4 — codes + paths + phase-1 validation**: code math, path
  grammar, the full phase-1 catalogue + class invariants; corpus cases
  keyed by rule code assert the exact code.
- [x] **A5 — RM validation**: VCORM/VCARM/VCORMT/VCAEX/VCACA/VCAM against
  the pluggable RmModel seam (production = generated openehr-rm model;
  corpus = BMM-loaded openEHR test models, full 48-schema archie
  referencemodels set vendored); VACSO/interior-VATID un-deferred;
  phase gating per master08.
- [ ] **A5b — emit-rm-model completeness, EMITTER ONLY (owner split
  2026-07-19)**: the generated `openehr_rm::model` lacks (1) generic type
  parameters, (2) RM container cardinality, (3) enumeration literals.
  Extend the `emit-rm-model` emitter to expose all three from the BMM +
  regenerate — model-surface change only, zero consumer changes, all
  existing suites stay green.
- [ ] **ODIN escape-set alignment** (found by the vendor-fixture battery,
  no fixture drives it): the ODIN lexer rejects `\a \b \f \v \?` (allowed
  by base_lexer.g4 ESCAPE_SEQ) and accepts 8-hex `\uHHHHHHHH` (grammar
  allows 4-hex only) — align to the grammar + add the fixture cases.
- [ ] **A6b — consume the A5b model data**: un-defer VCORMT
  generic-parameter substitution, VCACA's tight lower-bound half on the
  production model, VCORMENV/VCORMENU/VCORMEN; re-claim the adjudicated
  `VCORMT_rm_non_conforming_type1` fixture; new tests per code.
- [ ] **A6 — conformance functions + phase-2 specialisation validation**:
  master04.5 machinery incl. collective occurrences; the full phase-2
  catalogue; specialisation corpus green.
- [ ] **A7 — flattener + phase-3**: overlay algorithm (cloning, sibling
  anchors, deletions, proxies, differential paths), section-level merge
  semantics, phase-3 checks; spec-example + sibling-order fixtures green.
- [ ] **A8 — templates + OPT2**: overlay application, slot
  fill/close/inline, component_terminologies, raw + profiled OPT,
  OPT printer; master10 worked example reproduced end-to-end.
- [ ] **A9 — ADL 1.4 → 2 conversion**: converter core + our OPT14 front
  end + conversion log; corpus: converted 1.4 fixtures validate clean
  under the phase-1..3 catalogue.
- [ ] **A10 — REST/service integration**: replace the registration
  validator, wire upload/get/example/version + JSON/XML projections,
  utoipa declarations, admin-console touchpoints, book chapter,
  changelog; ECC zero-drift run.
- [ ] **A11 — close**: full-workspace gates, ECC, PROGRESS/worklist close,
  DELETE this plan file.

### Corpus coverage — HARD REQUIREMENT (owner, 2026-07-19)

**Every file under `crates/openehr-adl/tests/corpus/` is exercised with an
asserted expected outcome — 100%, no dead fixtures.** This is the same
discipline that gives archie its validation depth; the corpus is the
use-case library. Enforcement:

- **The oracle is the in-file `regression` tag, never the filename**
  (inventory finding 2026-07-19: every adl2-reference file embeds
  `other_details["regression"] = <"CODE"|"PASS"|"FAIL">`; 13 filenames
  contradict their tag — full list in
  `crates/openehr-adl/tests/corpus/INVENTORY.md`). Harnesses read the tag.
  Normalisations: `VDIFP1`→VDIFP, `VSONCOm`→VSONCO, `SEXLU`→SEXLU1/2
  family, `VACMC`→VACMCU; `VCOV` file is tagged PASS (assert clean).
- **Catalogue additions from the corpus** (adjudicated 2026-07-19):
  `VRDLA` (resource-description language-code consistency — corpus
  `validity/basics`; archie ErrorType parity; no full vendored AOM2 text —
  NOTE-flagged) and `WOUC` (warning: terminology code unused in
  definition). Both get typed variants + corpus claims.
- **85 catalogue codes have ZERO corpus cases** (list in INVENTORY.md §3b
  — most of the SC*AV/SC*PT primitive families and the specialisation
  VSON*/VDSS*/VARX* families): each gets a HAND-WRITTEN case in the phase
  that implements its check — "every code has a test" stands regardless
  of corpus coverage.
- A **coverage gate test** walks the whole corpus tree and fails if any
  file is not claimed by exactly one harness category — new or unclaimed
  files break CI. Keys on FULL PATH (2 duplicate basenames); must include
  a `.adl` walker (31 ADL-1.4 files: legacy_adl_1.4, upgrade_from_14, one
  features file — currently unexercised by the `.adls`-only A2 tests).
  Untagged files in `features/**`/`upgrade/**` = PASS-expected; an
  untagged file under `validity/**` fails the gate.
- Per-category harnesses:
  - `adl2-reference/validity/**` — parse + validate; tag carrying a rule
    code MUST raise exactly that code (W* as warnings); PASS-tagged files
    must pass clean; FAIL-tagged = any typed error.
  - `adl2-reference/features/**` — parse + validate clean; specialised
    cases also flatten successfully.
  - `adl2-reference/robustness/**` — never panic; typed errors only.
  - `adl2-reference/upgrade/upgrade_from_14/**` — each `.adl` (1.4)
    converts via the `adl14` module and the result is checked against its
    paired expected `.adls` (the converter oracle).
  - `adl2-reference/validity/legacy_adl_1.4/**` — the documented 1.4
    tolerance behaviours.
  - `flattener/specexamples/**` + `flattener/siblingorder/**` — flatten
    child→parent and assert against HAND-AUTHORED spec-derived
    expectations (no goldens are vendored — archie keeps them in Java
    test code; each authored assertion cites AOM2 master08/master09).
    Parent-only fixtures are claimed as flatten-inputs. Pair map in
    INVENTORY.md §7. NOTE: these 38 files are the ONLY corpus files using
    the unicode operator forms (∈ etc.) — they also regression-pin the
    dual-form lexer.
  - `upgrade/upgrade_from_15/**` — parse + validate clean (no 1.5 source
    inputs vendored, so no conversion comparison).
- Skips only via a documented adjudication entry (file + reason + spec
  citation) — never a silent exclusion; the adjudication list shrinks to
  zero by row close unless a fixture is proven defective against the spec
  text.

Exit criteria: every S-code and V-code has a typed variant + test; **the
full corpus coverage gate above is green (100% of files exercised)**;
round-trip parse/print stability; the master10 template → OPT worked
example reproduces; 1.4→2 conversion of our stored OPT corpus validates;
REST group serves all spec-declared representations; ECC zero drift.
