---
name: adl2-object-redefinition-09.05-location
description: Section→line map of ADL2 master09.05 (object redefinition), which AOM2 V-codes own each rule, and its confirmed released-text defects + grammar gaps
metadata:
  type: reference
---

# ADL2 object redefinition — `AM/docs/ADL2/master09.05-spec_object_redef.adoc` (1601 lines)

Section → line map: intro L1-3 · **Node Identifiers L5-118** (when redefinition
is required L9-13; the `atNNNN{.0}*.N` / `idN{.0}*.N` syntax L91-94; multi-level
examples L98-118) · **Adding Nodes L120-135** (extension ids start `0.`;
"level introduced = number of dots") · Occurrences Redefinition L137-151
(single- vs multiple-occurrence node distinction L145-149; VSONCO delegation
L151) · Mandation L153-219 (partial-redefinition rule L219) · Exclusion
L221-310 (alternatives removal L256-283; container removal L285-308; whole
attribute → existence {0} L310) · **Cloning rule L312-323** (the `clone not
needed` predicate L318-321) · Exhaustive/Non-Exhaustive L325-410 (default
non-exhaustive + "exclusion node MUST COME LAST" L333; 'frozen' node L410) ·
RM Type Refinement L412-653 (subtype narrowing L414-452; inherited parent
constraints L454; multi-subtype alternatives L512-549; supertype-with-exceptions
+ "subtypes matched first" L551-596) · Internal Reference (use_node) L655-787
(2 redefinition modes L659-660; manual-copy caveat L662; occurrences {0} on the
target propagates L783; re-targeting explicitly NOT part of ADL L785-787) ·
External Reference Redefinition L789-795 (3 allowed ways) · **Slot Filling and
Redefinition L797-950** (slot = definition + filler list L799; fill / narrow /
close / remove L801-806; `use_archetype` filling L849-878; recursive filling
L880-919; **`closed` syntax L921-948**; narrowing = replacement
`allow_archetype` L950) · Unconstrained Attributes L952-1022 (`use_archetype`
under an unarchetyped RM attribute; "no slot … validity-checked against the RM"
L956) · **Primitive Object Redefinition L1024-1031** (3 forms) · Numeric
L1033-1091 · Terminology Constraint Redefinition L1093-1095 (the "golden rule")
· Constrain-previously-unconstrained L1097-1143 · Internal value set L1145-1272 ·
External subset L1274-1405 (unvalidatable-without-terminology L1405) ·
**Constraint Strength Redefinition L1407-1536** (order example→preferred→
extensible→required L1409; `required` not redefinable L1452; non-required ≡ no
constraint ⇒ any value set L1495) · Tuple Redefinition L1538-1600.

**Only ONE V-code is named in the whole chapter: VSONCO (L151).** Every other
rule must be mapped by hand to `AOM2/master04.5-constraint_model-class_definitions.adoc`
(VSONT L341, VSONCT L344, VSONIR/VSONI **Deprecated** L348/L351, VSONIN L353,
VSONIF L356 — its cross-ref *VACMI is defined nowhere in the vendored AM text*,
VSONCO L359-379, VSONPT L381, VSONPI L384, VSONPO L387, VSSM L390, VARXNC/AV/TV/
R/S/ID L409-427, VDSSID L461, VDSSM L464, VDSSP L467, VDSSC L470, VSUNT L487).
`effective_occurrences` algorithm = master04.5 L177+ (`_occurrences_inferencing_rules`).
Constraint-strength model = `AOM2/master04.2` §Constraint Strengths (`[#constraint_strengths]`
L191) + `UML/classes/org.openehr.am.aom2.c_terminology_code.adoc` (`constraint_status`,
`effective_constraint_status()`, Void⇒required); syntax = `ADL2/master04.5` §Soft
Terminology Constraint L600+.

## Confirmed released-text defects / tensions (master09.05)
1. **Slot-filler node id**: L799 ("identifier is the specialisation of a slot node")
   + AOM2 VARXID (filler id must be a specialisation of the slot's) vs L849 +
   every `use_archetype` example (L859-861, L872-874, L892-894, L931, L943)
   which reuse the SLOT'S OWN id unchanged, and use the SAME id for 3 sibling
   fillers. Self-contradiction inside one section.
2. **Exclusion example uses wrong node ids** (violates VSONPI "exactly the same
   node_id"): parent L233-237/L246-250 declares `DV_INTERVAL<DV_QUANTITY>[at9001]`
   (`[id5]`) but the child L266/L278 excludes `[at9000]` (`[id4]`).
3. L149 xref `<<Redefinition for Specialisation>>` says the example is "provided
   above" — it is in a different file (master09.03 L137).
4. `<<Node Identifiers>>` (L7) is a duplicate heading title (this file L5 and
   master04.3 L195) — ambiguous xref.
5. L1409 typo `exensible`; L1398 id-coded value-set example has misplaced quotes
   (`members = "<at22", "at23">`).
6. Constraint-strength narrowing (L1409) contradicts L1495 in effect: strength
   must narrow, yet a non-required node's value set may be replaced by a
   NON-conforming one — two different conformance regimes in one subsection.
7. L783 use_node occurrences-{0} propagation is stated with a probabilistic
   hedge ("chance … vanishingly small"), not as a rule.

## Grammar gaps vs `crates/openehr-adl/vendor/grammar/`
- `closed` **HAS** a token/production: `adl_keywords.g4` L39 `SYM_CLOSED`;
  `cadl2.g4` L41 `archetype_slot : allow_archetype rm_type_id '[' ID_CODE ']'
  ((c_occurrences? (matches '{' includes? excludes? '}')?) | SYM_CLOSED)` — the
  alternation encodes VDSSC (closed XOR narrowed) syntactically.
- **NO production**: (a) `use_archetype TYPE[archetype_ref]` without an id-code
  (L888, L929) — `c_archetype_root` (cadl2.g4 L35) REQUIRES `'[' ID_CODE ','
  archetype_ref ']'`; (b) a `use_archetype … ∈ { … }` BODY (L888-898, L929-934)
  — `c_archetype_root` has no `matches` block; (c) an archetype-root definition
  ROOT — `adl2.g4` L69 `definition_section : SYM_DEFINITION c_complex_object`;
  (d) constraint-strength keywords (`required|extensible|preferred|example`) —
  absent from `adl_keywords.g4`, and `cadl2_primitives.g4` L54
  `c_terminology_code : '[' (AC_CODE (';' AT_CODE)? | AT_CODE) ']'` has no slot
  for them; (e) the entire at-coded flavour (zero-padded `atNNNN`) — see
  [[adl2-parser-spec-location]].
