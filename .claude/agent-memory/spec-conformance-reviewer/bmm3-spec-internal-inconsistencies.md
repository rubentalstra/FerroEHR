---
name: bmm3-spec-internal-inconsistencies
description: Adjudicated — where BMM3 ch.10-12 prose disagrees with the bmm3 BMM/UML class docs, the machine-readable model governs; the nine known slips, plus the EL-vs-BEL precedence conflict
metadata:
  type: project
---

BMM3 (LANG, DEVELOPMENT generation) chapter prose repeatedly names classes and
attributes that its own machine-readable BMM + UML class docs do not have. In
every verified case the BMM json and the `org.openehr.lang.bmm3.*.adoc` class
docs AGREE with each other and the chapter prose is the outlier — so
**"follow the model" is the resolution**; the prose slip is a decision point
to register, never a code defect. Verified 2026-07-31 (ch.10–14 audit).

The nine confirmed slips:

- `EL_CONSTANT_REF` (§10.2 prose) — the class is `EL_STATIC_REF`.
- `EL_SCOPED` with attribute `scope` (§10.2) and `context`
  (`el_type_ref.adoc` `Inv_no_context`) — the real attribute is
  `EL_FEATURE_REF.scoper`. `EL_SCOPED` does not exist, and `EL_TYPE_REF`
  inherits `EL_VALUE_GENERATOR` (not `EL_FEATURE_REF`), so it has no scoping
  attribute at all → `Inv_no_context` is unimplementable as written.
- `EL_OPERATOR.definition` (§10.3 prose), `operator_def` (its invariant),
  `OPERATOR_DEF` (its attribute doc) — the attributes are
  `precedence_overridden`/`symbol`/`call`; the functions are
  `operator_definition()`/`equivalent_call()`.
- `BMM_ASSERTION` called "a tagged Boolean-returning EL_EXPRESSION" (§10.5) —
  it is a `BMM_SIMPLE_STATEMENT` that HAS an `EL_BOOLEAN_EXPRESSION`.
- Decision-table `result` "of any expression type" (§10.2) — all six decision
  classes bound `T: EL_TERMINAL` in the BMM.
- `E_TUPLE` (§10.1) → `EL_TUPLE`. Agent `eval_type` = `BMM_SIGNATURE` (§10.1,
  §11.1) vs `BMM_ROUTINE_TYPE` (class doc — the narrower truth, since
  `BMM_ROUTINE_TYPE : BMM_SIGNATURE`). `EL_LITERAL` post-condition names
  `definition.type` where the attribute is `value`.
- `open_arguments` in `is_callable()`'s post-condition → `open_args`.
- Routine body "a simple statement or a block" (§12.1) —
  `BMM_LOCAL_ROUTINE.body: BMM_STATEMENT_BLOCK`, 1..1, block only.
- `BMM_CONDITIONAL_ACTION` (§12.5 prose, :102) — no such class; the real pair
  is `BMM_ACTION_TABLE` + `BMM_ACTION_DECISION_TABLE`, and the latter is
  declared with NO ancestors and NO attributes, so §12.5's whole
  "decision table whose outputs are statements" story is unrepresentable
  upstream. `BmmActionDecisionTable {}` (an empty struct) is FAITHFUL — do not
  file it as our defect.

**Separate cross-spec conflict (register, do not "fix"):** ch.10 §10.3's
`precedence_overridden` refers to a "natural precedence" that neither ch.10
nor `BMM_OPERATOR` defines (`BMM_OPERATOR` has only `position`, `symbols`,
`name`). The only normative tables are in the DEVELOPMENT EL syntax spec
(`LANG/docs/EL/master05-expressions.adoc` §Primitive Operators + §Precedence
and Parentheses) → `NOT > AND > OR > XOR > IMPLIES`, which CONFLICTS with the
STABLE BEL grammar (`crates/openehr-lang/vendor/grammar/base_expressions.g4:58-63`)
→ `NOT > AND > XOR > OR > IMPLIES`. Our `bel/parser.rs:5-6` implements BEL's
order and is correct (STABLE + vendored grammar wins).

**How to apply:** on any BMM3 audit, read the `org.openehr.lang.bmm3.*.adoc`
class doc before trusting a chapter's prose name; report prose/model
disagreements as their own spec-silence findings, not as divergences.
