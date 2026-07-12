# Validation — spec-first redesign (W-3f, area 09)

Owner directive 2026-07-12 (W-3f): redesign the `ehrbase` **platform crate**
validation surface spec-first — map the AM constraint model *onto* the code,
never the code onto the spec. This document is the read-only audit + target
design for `app/ehrbase/src/validation/` (the fresh home for the two artefact
validators today wrongly parked under `service/`). The work item is **W-3f** in
`docs/plans/WORKLIST.md`.

**Spec oracles** (read before any change):

- `docs/specs/openehr/AM/docs/AOM1.4/master04-constraint_model_package.adoc`
  (the AOM 1.4 constraint model: `C_OBJECT`/`C_ATTRIBUTE`/`C_COMPLEX_OBJECT`/
  `C_PRIMITIVE_OBJECT`/`C_DOMAIN_TYPE`, `valid_value`, node_id/paths),
  `master06-primitive_package.adoc` (the `C_PRIMITIVE` family),
  `master07-ontology_package.adoc` (ontology + bindings)
- `docs/specs/openehr/AM/docs/ADL1.4/master05-cadl.adoc` (`cADL`: existence,
  cardinality, occurrences, VCOC, primitive/temporal/duration patterns,
  slots, internal refs, placeholder constraints, "any" constraints)
- `docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc` (the **only
  formalized validity-code catalogue** — phase-1/2/3 rule codes VCORM/VCARM/
  VCAEX/VCACA/VCAM/VACMCO/VATID/VTLC/VTTBK/VTCBK/… — the oracle for artefact
  ingestion), `master04.5-constraint_model-class_definitions.adoc` (AOM2
  rule blocks + `c_conforms_to`), `master07-terminology_package.adoc` (VTVSID/
  VTVSMD/VTVSUQ, VTTBK/VTCBK, VTLC, VTSD), `master04.2-constraint_model-
  semantics.adoc`, `master10-templates.adoc`, `OPT2/master03-opt_raw.adoc`
- `docs/specs/openehr/AM/docs/ADL2/master04.6-cadl_validity_rules.adoc`
  (ADL2 `STCNT`/`STCDC`/`Sxxx` codes), `master08-adl.adoc`,
  `AM/docs/AOM2/master03-archetype_package.adoc` (VARCN/VATDF/VACDF/VATDA)
- `docs/specs/openehr/BASE/docs/foundation_types/master05-interval.adoc`
  (`Interval.has/intersects/contains`), `UML/classes/…multiplicity_interval`
  + `…cardinality` (the constraint-evaluation primitives), and
  `BASE/docs/architecture_overview/master10-archetypes.adoc` (the archetype
  chapter that frames archetype-based runtime data validation)
- Cross-reference (do not re-derive): `docs/blueprint/03-am.md` (the AM
  requirements chapter — normative rows A–I), the B2 build-step close notes
  in `docs/blueprint/00-THE-BLUEPRINT.md` §3 (ArchetypeValidation 81→0).

**Current implementation** (verified 2026-07-12). The platform crate carries
**three distinct validation surfaces**; W-3f's scope is the artefact validators
(A) — (B)/(C) are audited for the seams:

| # | Surface | Where | Lines | Concern |
|---|---|---|---|---|
| A1 | OPT 1.4 artefact validity | `service/opt_validation.rs` (+`/tests.rs`) | 1,322 (+493) | AOM2/08 catalogue on an uploaded **flattened OPT** |
| A2 | ADL2 source registration validity | `service/adl2_validation.rs` (+`/tests.rs`) | 905 (+275) | AOM2/08 header/section/terminology subset + a hand-rolled ODIN reader |
| A3 | OPT XML well-formedness | `service/template.rs::validate_opt_structure` | ~100 | alien tags / duplicated single-valued elements (CNF `upload_opt-invalid_opt`) |
| B | Commit-time **instance** `valid_value` cascade | `openehr-flat::validation` (spec crate), invoked from `service/composition.rs::validate_composition_for_commit` (L423) via `validate_rm_and_terminology` + `validate_archetype_conformance[_incomplete]` | — | RM invariants + terminology + WebTemplate walk over committed data |
| C | Per-kind RM structural validators | `ehr.rs` (`validate_ehr_status`/`validate_ehr_access`), `directory.rs` (`validate_folder`), `relationship.rs`, `demographic.rs` | — | non-COMPOSITION commit bodies (no template) |

Callers: A1 ← `template.rs:122` (POST template); A2 ← `definition.rs:275/643`,
`api/definition.rs:93` (ADL2 register); B/C ← `composition.rs:493
validate_for_commit` (direct + CONTRIBUTION paths, F-07-01 single seam).

---

## 1. The AM constraint taxonomy (the spec skeleton)

Enumerated **from the AM spec's own constraint decomposition** — the axis the
redesign files must follow. Each row: the constraint kind, its citation, and
where AOM 1.4 (OPT path) and AOM 2.4 (ADL2 path) diverge.

| T | Constraint kind | AOM 1.4 citation | AOM 2.4 divergence |
|---|---|---|---|
| T1 | **`C_COMPLEX_OBJECT` / `C_ATTRIBUTE` alternation** — an archetype is a `C_COMPLEX_OBJECT`(`rm_type_name`) → `attributes: List<C_ATTRIBUTE>` → child `C_OBJECT`s, recursively to leaves | AOM14/04 §Overview L5-7, §Complex Objects L70 | AOM2 identical shape; adds `C_COMPLEX_OBJECT_PROXY`, `C_ARCHETYPE_ROOT` as first-class (AOM2/04.5) |
| T2 | **`valid_value` conformance cascade** — recursive top-down data-conformance function ("the key function of an archetype-enabled kernel") | AOM14/04 §Valid_value L60-62 | AOM2 replaces with `c_conforms_to()` (constraint-vs-constraint) + `valid_value` (data); VSONCT/VSONCO/VSONI (AOM2/08 L96-101) |
| T3 | **Node identification & paths** — every addressable `C_OBJECT` carries a `node_id` (at-code) unique among siblings and defined in the ontology; paths are `rm_attribute_name`+`[node_id]` | UML:c_object node_id L26-29; AOM14/04 §Node_id, Paths L39-44 | AOM2 uses `id`-codes (`id1.1`), VARCN root-code form (AOM2/03) |
| T4 | **Existence** (`C_ATTRIBUTE.existence`, attribute presence) — legal `{0}`,`{0..0}`,`{0..1}`,`{1}`,`{1..1}`; default `{1..1}`; invariant `Existence_set: lower>=0 and upper<=1` | AOM14/04 L33; cADL §Existence L210; UML:c_attribute L44 | AOM2: VCAEX (conform to RM existence), VSANCE (conform to flat parent) |
| T5 | **Cardinality** (`C_MULTIPLE_ATTRIBUTE.cardinality`, container membership count + `is_ordered`/`is_unique`) — no default; absence ⇒ single-valued | cADL §Cardinality L245-273; UML:cardinality | AOM2: VCACA (conform to RM), VSANCC (conform to flat parent) |
| T5b | **Single-valued attribute w/ multiple children = alternatives** — exactly one instance appears, matching one sibling block (incl. joint/covariant block match) | cADL §Single-valued Attributes L214-237 | same |
| T6 | **Occurrences** (`C_OBJECT.occurrences`, per-block instance count) — default `{1..1}`; upper>1 only under container cardinality>1 | cADL §Occurrences L316; UML:c_object L24 | AOM2: VSONCO (conform to flat parent) |
| T7 | **VCOC / VACMCO** — Σ(child occurrences.lower)‥Σ(child occurrences.upper) must sit inside the container cardinality interval | cADL L324 (VCOC) | AOM2/08 L112 (VACMCO); AOM2/04.5 L159 |
| T8 | **`any_allowed` / `matches {*}`** — RM-permitted any value; residual check = existence + named-RM-type; invariant `Children_validity: any_allowed xor children/=Void` | cADL §"Any" L326-351; UML:c_attribute L34-38,47 | same |
| T9 | **`C_PRIMITIVE` leaves** — `C_STRING` (list + PERL `pattern`, `list_open`), `C_INTEGER`/`C_REAL` (list or interval), `C_BOOLEAN` (`true_valid`/`false_valid`) | cADL §Primitive Types L645-1006; AOM14/06 | AOM2 primitive package (04.5); STCDC duplicate-code |
| T9b | **Temporal `C_DATE`/`C_TIME`/`C_DATE_TIME`** — ISO-8601 field patterns (`yyyy-mm-ddThh:mm:ss` w/ `?`/`X`), field-ordering rule (optional→optional/disallowed; disallowed→disallowed), timezone-required suffixes; plus literal lists/intervals | cADL L858-932; UML:c_date/c_time/c_date_time `Pattern_validity` | same syntax; AOM2 formalizes as invariants |
| T9c | **`C_DURATION`** — `P[Y][M][W][D][T…]` designator patterns (openEHR deviation: `W` mixable), lists/intervals incl. negative, mixed `pattern '/' interval` | cADL L934-987 | same |
| T10 | **`C_DOMAIN_TYPE` (openEHR-profile, what OPT 1.4 carries)** — `C_DV_QUANTITY` (`property` + `C_QUANTITY_ITEM` units↔magnitude covariance), `C_DV_ORDINAL` ((symbol,value) pairs), `C_CODE_PHRASE`/`C_CODED_TEXT` (terminology + code_list), `C_DV_STATE` | UML:c_quantity/c_ordinal/c_coded_text; ADL14/master09 | AOM2 has **no** C_DOMAIN_TYPE — uses tuples (T11) + terminology value sets (T13) instead |
| T11 | **Tuple / covariant constraints** — `C_ATTRIBUTE_TUPLE`/`C_PRIMITIVE_TUPLE` for covariant leaves | **absent in AOM 1.4** (covariance only via `C_QUANTITY_ITEM` list or alt-blocks) | AOM2-only: AOM2/04.5 §Tuples; VTPNC/VTPIN (AOM2/08 L101) |
| T12 | **Assumed / default values** — leaf metadata; assumed values don't appear in data (default values do); invariant `Assumed_value_valid` (assumed ∈ constrained set) | AOM14/04 §Assumed_value L54-58; cADL L1007-1022 | AOM2 same |
| T13 | **`ARCHETYPE_SLOT`** — `include`/`exclude` PERL-regex assertion lists over archetype ids; declared RM type + occurrences still apply; VDSEV validity, VDFAI id-shape | cADL §Archetype Slots L535-601; AOM14/04 §Reference Objects L83 | AOM2: VDSEV, VARXS/VARXR/VARXID for `C_ARCHETYPE_ROOT` fillers (AOM2/08 L86-90) |
| T14 | **`ARCHETYPE_INTERNAL_REF` (`use_node`)** — reuse of a constraint by absolute path; invariant `Target_path_valid` | cADL §Internal References L618-643; AOM14/04 | AOM2: `C_COMPLEX_OBJECT_PROXY` + VSUNT/VUNP path-existence (AOM2/08 L91,111) |
| T15 | **`CONSTRAINT_REF` / ac-code placeholder** — proxy for an external value set; ontology binds `acNNNN` to query/value-set ids; VACDF (ac-code defined) | cADL §Placeholder L603-616; AOM14/04 L85-91 | AOM2: value sets in terminology (VTVSID/VTVSMD/VTVSUQ, master07) supersede CONSTRAINT_REF |
| T16 | **`C_ARCHETYPE_ROOT`** — a slot-filler root carrying its own archetype id; VARXRA/VARXTV validity | (flattened-OPT construct) | AOM2/04.5 §C_ARCHETYPE_ROOT; VARXRA/VARXTV (AOM2/08 L52) |
| T17 | **Ontology integrity + bindings** — every at/ac-code defined per language; VATID (codes in definition defined), VTLC (all codes in all langs), VTSD (code depth ≤ artefact depth), VTVSID/VTVSMD/VTVSUQ, VTTBK/VTCBK (binding keys valid), VOLT/VOTM (language present) | AOM14/07 L11-32 | AOM2/07 §Validity Rules L57-81; AOM2/08 L44-47 (identical code names) |
| T18 | **Closed-world / unmatched-content semantics** — is a present-but-unmatched instance node an error? | **AOM 1.4 silent** (only positive `valid_value`, AOM14/04 L60-62) | AOM2 formalizes: `c_conforms_to`, VSONCT/VSONCO closure (AOM2/08 L96-101) |
| T19 | **Specialisation / flattening validation** — VACSD (depth = parent+1), VDIFV/VDIFP (differential paths), VSANCE/VSANCC/VSONCT/… (child-vs-flat-parent) | (specialisation limited in ADL 1.4) | AOM2 phase-2 core: AOM2/08 §Phase 2 L64-105 |
| T20 | **Constraint-evaluation primitives** — `Interval.has/intersects/contains`, `Multiplicity_interval`, `Cardinality` (the math the above rests on) | BASE foundation_types master05-interval; UML:multiplicity_interval, :cardinality | version-independent (BASE 1.3.0) |

---

## 2. Code mapped onto the taxonomy (file:line + verdict)

**A1 — OPT 1.4 artefact validity (`opt_validation.rs`)** applies the AOM2/08
catalogue to the *flattened* OPT tree; note it does **not** run `valid_value`
(that is instance-time, surface B):

| T | Code enforces | Verdict |
|---|---|---|
| T1 | tree walk `walk_attribute`/`walk_object` over `opt14` types (L156, L758) | conformant |
| T3 | VATID `check_node_id` (L1014), at-code shape `is_at_code` (L1029) | conformant |
| T4 | `Existence_set` invariant (L183-195), VCAEX RM-existence conformance (L943) | conformant |
| T5 | VCAM container-on-single-valued (L927); VCACA numeric bound **not checked** — PORT NOTE L953 (static RM model exposes container kind, not numeric cardinality) | conformant (VCAM) / documented-gap (VCACA numeric) |
| T5b | `Members_valid` single-valued occurrences.upper≤1 (L202-216) | conformant |
| T6 | occurrences read via `co_occurrences` (L1292); used in VACMCO | conformant |
| T7 | VACMCO `check_cardinality_occurrences` (L978) — Σlower vs card.upper; deliberately omits the maxima-side (alt-block pattern) L974 | conformant |
| T9 | `check_primitive` (L453): C_BOOLEAN satisfiability, C_STRING/INT/REAL `Assumed_value_valid` | conformant |
| T9b | temporal `Pattern_validity` — `valid_date/time/date_time_pattern` + `field_chain_valid` (L629-698), `strip_tz_offset` (L702) | conformant |
| T9c | `valid_duration_pattern` + `in_order_subset` (L713-743) | conformant |
| T10 | `C_DV_QUANTITY` `Property_valid` + units↔magnitude assumed-value (L813-869); `C_DV_ORDINAL`/`C_CODE_PHRASE`/`C_CODE_REFERENCE` `Assumed_value_valid` + `check_code_list` STCDC (L575) | conformant |
| T12 | `Assumed_value_valid` on every C_* (L468-528, L604) | conformant |
| T13 | `check_slot` VDFAI id-shape via `regex_literal`/`slot_assertion_pattern` (L343-405) | conformant (id-shape only; genuine regexes deferred to runtime) |
| T14 | `check_internal_ref` `Target_path_valid` (L413) | conformant |
| T15 | `check_constraint_ref` VACDF, gated on `has_constraint_defs` (L437) | conformant (flattened-OPT tolerance PORT NOTE) |
| T16 | `C_ARCHETYPE_ROOT` VARID/VARDT `check_archetype_id` (L270,775) | conformant |
| T17 | VTTBK (L1044), VTCBK (L1092), VTLC `check_language_consistency` (L1127), code collection (L1177-1205) | conformant |
| — | VCORM `check_object_type` via `openehr_rm::model` (L887); VCARM attribute existence w/ LOCATABLE_META/LEGACY tolerance (L221-247) | conformant |
| T20 | **local reimplementation** `iv_lower`/`iv_upper`/`int_in_range`/`real_in_range` (L745,1244) over `opt14::Intervalofinteger` — does **not** consume the BASE `MultiplicityInterval::{has,intersects,contains}` / `interval_impl` primitives (now built, `crates/openehr-base/src/foundation_types/interval/`) | divergent (duplicated interval math) |

**A2 — ADL2 registration (`adl2_validation.rs`)** — a *source*-level lexical +
ODIN subset (no cADL compiler): STCNT (L88-155), VARAV/VARRV (L108-133), VARDT
(L182), VARCN (L195-222), VOLT/VOTM (L260-288), VTLC (L241-258), VDIFV (L299),
VTSD (L317), VATDF/VACDF/VATID + C_TERMINOLOGY_CODE shape (L339-366), VTVSID/
VTVSMD/VTVSUQ (L370-401), VATDA (L405-430), VTTBK (L433-444), VACSD (L459). All
**conformant** to the honestly-enforceable subset (the module doc scopes out
the compiler-bound VCxxx/VSxxx/Sxxx families, L39-45 — correct: no ADL2 compiler
exists, no CNF/ECC case exercises it). **Divergence:** L637-902 is a
hand-rolled ~270-line ODIN reader (`OdinValue`/`OdinParser`) while the project's
ODIN reader is `openehr-lang::odin` (CLAUDE.md repo map) — duplicated parsing.

**B — instance `valid_value` cascade** lives in `openehr-flat::validation`
(spec crate, out of the platform-crate rewrite scope) but is the platform's
core commit duty (T2). Invoked once from `composition.rs:433/447`. The walk is
over the *compacted WebTemplate*, not the AOM tree, and self-declares skips
(temporal ranges, precision, unresolved slots — per blueprint 03-am row 2/10).
T18 closed-world is implemented there (ADR-012). **This is a seam, not a move.**

**C — per-kind RM structural validators** (`ehr.rs`/`directory.rs`/
`relationship.rs`/`demographic.rs`) — not archetype validation; RM-invariant
"wrongness" checks on templateless commit bodies. Correctly separate; a seam.

**Code with no AM-spec taxonomy row:** `validate_opt_structure` (A3, XML
well-formedness) maps to **CNF `upload_opt-invalid_opt`**, not an AOM code — a
wire/ingestion guard, legitimately spec-silent on the AOM axis (flag: "CNF
fixture behaviour, not an AOM constraint kind"). No quarantine/delete candidates
found — every function traces to a spec code or a CNF fixture.

---

## 3. G-row register

| id | citation / flag | severity | disposition |
|---|---|---|---|
| G-09-01 | file location: two artefact validators sit under `service/` not a `validation/` module | med | **fix-in-rewrite** — move to `app/ehrbase/src/validation/` along AM boundaries |
| G-09-02 | `opt_validation.rs` = 1,322 lines, one file mixes tree-walk / AOM1.4 invariants / temporal-pattern / RM-conformance / terminology | high | **fix-in-rewrite** — split into ≤700-line files per §4 |
| G-09-03 | BASE `Interval`/`Multiplicity_interval` primitives (BASE master05-interval; `interval_impl.rs`/`multiplicity_interval_impl.rs`) exist but A1 reimplements `iv_lower`/`int_in_range` locally (opt_validation L745,1244) | med | **fix-in-rewrite** — consume `openehr_base` primitives where the `opt14` interval maps cleanly; PORT NOTE where opt14 XSD shape (`upper_unbounded` flag) does not |
| G-09-04 | `adl2_validation.rs` hand-rolls an ODIN reader (L637-902) duplicating `openehr-lang::odin` | med | **re-verify then fix-in-rewrite** — confirm `openehr-lang::odin` covers the terminology/language subset; if yes reuse, else PORT NOTE why a local lexical subset is needed at registration |
| G-09-05 | T18 closed-world semantics carried by a `// PORT NOTE:` citing **ADR-012** in `openehr-flat::validation` (violates owner rule: cite spec not ADR) | high | **fix-in-rewrite** — re-express as spec citation `AOM2/08 §Phase 2 c_conforms_to / VSONCT L96-101` + "AOM 1.4 silent (L60-62)"; ADR-012 stays decision-history only |
| G-09-06 | T5 VCACA numeric-cardinality bound unenforceable (static RM model lacks bounds; cADL L268 hedges) | low | **PORT NOTE** (already documented, opt_validation L953) — keep, re-cite cADL L268 |
| G-09-07 | T11 tuples: AOM 1.4 has no `C_*_TUPLE`; covariance only via `C_QUANTITY_ITEM`/alt-blocks (blueprint 03-am defect 10) | low | **already-correct** — record as scope boundary, not a gap (OPT 1.4 path) |
| G-09-08 | T13 slot regex: only literal id-shape decided; genuine PERL regexes deferred to runtime slot admission (opt_validation L340) | low | **PORT NOTE** — keep; the include/exclude runtime admission is surface B's job (F-07-10) |
| G-09-09 | T19 ADL2 specialisation VCxxx/VSxxx/Sxxx not enforced (no compiler) | low | **PORT NOTE** (already, adl2_validation L39-45) — keep, re-verify no CNF/ECC ADL2-compile case appears |
| G-09-10 | prior-art tolerances (LOCATABLE_META_ATTRS, LEGACY_RM_ATTRS, `property="0"`, dotted at-code bindings, empty code-list entries) tuned against the vendored corpus | med | **already-correct** — each carries an in-code PORT NOTE citing the corpus template; preserve verbatim through the move (widening, never weakening, per ADR-012 gate) |
| G-09-11 | A3 `validate_opt_structure` is XML-shape, keyed to CNF fixtures not an AOM code | low | **already-correct** — flag "CNF `upload_opt-invalid_opt`, not an AOM constraint kind"; keep in a `structure` submodule |
| G-09-12 | surface B (instance `valid_value`) lives in `openehr-flat`, invoked from `composition.rs`; the platform crate owns only the invocation seam | info | **already-correct** — record as the T2 seam; not moved by W-3f |

Counts: **fix-in-rewrite 4** (01,02,03,05; 04 also lands here after re-verify) ·
**re-verify→fix 1** (04) · **PORT NOTE 3** (06,08,09) · **already-correct 4**
(07,10,11,12).

---

## 4. Target design — `app/ehrbase/src/validation/`

Fresh module tree along **AM constraint boundaries** (the §1 taxonomy), every
file ≤ ~700 lines. `service/opt_validation.rs` (1,322) splits; `service/
adl2_validation.rs` (905) moves + sheds its ODIN reader per G-09-04.

```
app/ehrbase/src/validation/
  mod.rs              -- surface map + re-exports; the three-surface doc (A/B/C)
  opt/                -- A1: OPT 1.4 artefact validity (AOM2/08 on flat OPT)
    mod.rs            -- validate_opt_artefact entry + Ctx + tree walk (T1)
                         (walk_attribute/walk_object, code collection)  ~320
    invariants.rs     -- AOM1.4 per-node-kind invariants: Existence_set,
                         Members_valid, Target_path_valid, VARID/VARDT,
                         VACDF, VDFAI, STCDC (T4,T5b,T13,T14,T15,T16)  ~300
    rm_conformance.rs -- VCORM/VCARM/VCAEX/VCACA/VCAM + VACMCO over
                         openehr_rm::model (T5,T7 + RM checks)  ~250
    primitive.rs      -- C_PRIMITIVE + temporal/duration pattern validity +
                         C_DOMAIN_TYPE assumed-value (T9,T9b,T9c,T10,T12)  ~380
    terminology.rs    -- VATID/VTTBK/VTCBK/VTLC + code collection (T17)  ~230
    interval.rs       -- opt14-interval helpers, thin adapters onto
                         openehr_base primitives where shape permits (T20,G-03) ~80
  adl2/               -- A2: ADL2 source registration validity
    mod.rs            -- validate_adl2_source + check_specialisation_depth
                         (STCNT/VAR*/VOLT/VOTM/VTLC/VATDF/VTVS*/VATDA/VTTBK) ~420
    (ODIN parsing)    -- reuse openehr-lang::odin (G-09-04); local reader
                         deleted or, if reuse blocked, extracted here w/ PORT NOTE
  structure.rs        -- A3: validate_opt_structure (OPT XML well-formedness,
                         CNF upload_opt-invalid_opt) moved out of template.rs  ~110
  tests/              -- the existing opt/adl2 tests re-homed (no weakening)
```

**Design decisions:**

1. **Boundaries = the taxonomy, not the class list.** Files group by *kind of
   check* (structural invariant / RM conformance / primitive / terminology),
   which is the axis the AOM2/08 catalogue itself uses (Phase-1 §Basic /
   §Terminology / §Code / §Structure) — so a reviewer can trace file→catalogue
   section.
2. **A1 and A2 stay separate** (OPT 1.4 vs ADL2 source): different inputs
   (parsed `opt14` tree vs raw source text), different enforceable subsets
   (A2 cannot run cADL-semantic VCxxx). Sharing is limited to the code-shape
   helpers (`is_at_code`/`is_local_code`, version-id, id-shape) → a small
   shared `validation/codes.rs` or reuse of the existing `service/codes.rs`.
3. **Consume BASE primitives** (G-09-03): `interval.rs` is a thin adapter that
   maps `opt14::Intervalof{integer,real}` onto `openehr_base` interval math
   where the `upper_unbounded`/`lower_unbounded` XSD flags permit; a PORT NOTE
   records where the opt14 boundary shape forces a local check.
4. **Surface B is NOT moved.** The instance `valid_value` cascade stays in
   `openehr-flat` (spec crate); `composition.rs` keeps the invocation. W-3f
   only re-expresses its closed-world PORT NOTE (G-09-05) as a spec citation.
5. **No behaviour change, no test weakening.** This is a structural rehome +
   citation hygiene + primitive-reuse pass; every AOM2/08 code the current
   files enforce must still fire identically (the ECC ArchetypeValidation
   set + the vendored owned-fixture register are the gate — zero drift).

---

## 5. Seams (TODO(w3f-integrate) candidates)

- **`templates/` → WebTemplate input (surface B).** The artefact validators (A)
  validate the *uploaded* OPT; the WebTemplate builder + instance walk consume
  it later. The A→B handoff (does an A-accepted OPT always build a walkable
  WebTemplate?) is the integration seam — mark the `web_template_for` path
  (`composition.rs:443`).
- **`service/ehr` commit path (surface C).** `validate_for_commit`
  (`composition.rs:493`) dispatches per Kind to B (COMPOSITION) or C
  (EHR_STATUS/FOLDER/party). A validation `mod.rs` should own this dispatch map
  so no commit path bypasses validation (F-07-01 discipline) —
  TODO(w3f-integrate) to route C's per-kind validators through the new module.
- **Terminology binding (T17/T15).** VTTBK/VTCBK check binding *keys*; resolving
  ac-code value sets against the live terminology service (B4 `TerminologyService`)
  is unwired at ingestion — TODO(w3f-integrate) when the CONSTRAINT_REF policy
  (blueprint 03-am remaining §3) lands.
- **`openehr-lang::odin` reuse (G-09-04).** The A2 ODIN seam.

---

## 6. PORT-NOTE residue (keep / re-verify / drop)

| PORT NOTE (current) | disposition |
|---|---|
| LOCATABLE_META_ATTRS / LEGACY_RM_ATTRS tolerance (opt_validation L57-87, L221-233) | **keep** — corpus-cited; preserve verbatim |
| VACDF flattened-OPT tolerance (no constraint_definitions) L431 | **keep** — corpus-cited |
| dotted at-code binding tolerance L1049 | **keep** — blood-pressure corpus |
| `property="0"` placeholder tolerance L824 | **keep** — action-test corpus |
| empty code-list entries skipped (STCDC) L581 | **keep** — AoMRC corpus |
| VCACA numeric-bound not checkable L953 | **keep**, re-cite cADL L268 (G-09-06) |
| slot genuine-regex deferral L340 | **keep** (G-09-08) |
| ADL2 compiler-bound codes inapplicable (adl2 L39-45) | **re-verify** no CNF/ECC ADL2-compile case, then keep (G-09-09) |
| ADL2 `OdinValue::Other` tolerated (never reject on unmodelled ODIN) L654 | **keep** if the reader survives G-09-04; **drop** if `openehr-lang::odin` is reused |
| **ADR-012 closed-world citation in `openehr-flat::validation`** | **re-express** → cite `AOM2/08 c_conforms_to/VSONCT L96-101` + "AOM14/04 L60-62 silent"; drop the ADR number from code (G-09-05). ADR-012 content re-stated as: AOM 1.4 defines only positive `valid_value`; closure (reject unmatched archetyped siblings, tolerate RM-permitted metadata, tolerate unlisted archetype-rooted fillers under slotless attributes) follows AOM2 `c_conforms_to` and matches production-CDR behaviour the CNF content chapters assume |

---

*Related: `docs/blueprint/03-am.md` (AM requirements A–I); B2 close notes in
`00-THE-BLUEPRINT.md` §3; `docs/ADRs/ADR-012` (decision history only).*
