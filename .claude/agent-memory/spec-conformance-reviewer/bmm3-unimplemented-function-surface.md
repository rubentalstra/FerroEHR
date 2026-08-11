---
name: bmm3-unimplemented-function-surface
description: openehr-lang function-surface state — the meta-type lattice and signature surface ARE implemented now; the machine ratchet is unrealized_bmm_functions.txt, and the invariant boundary is the live gap
metadata:
  type: project
---

**Re-verified 2026-08-11.** The gaps this memory used to list are largely
CLOSED; check the ratchet before claiming any function is missing.

**The authority is now machine-enforced:**
`tools/openehr-codegen/tests/it/unrealized_bmm_functions.txt` (670 entries,
274 of them `lang/v1_0` + `lang/v1_1`) is asserted to equal the projection
EXACTLY by `unrealized_bmm_functions_match_the_ratchet`. So "a BMM function
implemented nowhere and absent from that file" is a build failure, not a
review finding — do not hand-audit completeness, read the ratchet.

**Implemented since the 2026-07-31 note** (grep-verified):
`unitary_type`, `effective_type`, `is_primitive`/`is_abstract` on types,
`type_base_name`, `is_open`/`is_closed`/`is_partially_closed`,
`effective_base_class`, `name_map`, `generic_parameter_conformance_type`,
the `type()` generators, `flat_features`, `signature`, `is_boolean`,
`arity`, `has_ancestor_class`
(`crates/openehr-lang/src/v1_1/bmm3/core/{entity/bmm_type_impl.rs,
entity/bmm_class_impl.rs,feature/bmm_feature_impl.rs}`).

**Both `type_conforms_to` divergences are FIXED**
(`…/bmm3/core/model/bmm_model_impl.rs`): base-class comparison is
`eq_ignore_ascii_case`, and open parameters are substituted with the
declared constraint via `generic_parameter_conformance_type` on BOTH sides.
What is still NOT realized is the spec's
`bmm_def_class instanceOf (BMM_GENERIC_CLASS)` guard
(`LANG/docs/bmm3/master06-core-types.adoc` §Type Conformance) — the Rust
branches purely on the STRING shape.

**The live gap is the invariants**, and its stated justification is stale:
`…/bmm3/core/feature/bmm_feature_impl.rs` module NOTE claims "the v3
generation has no materialisation source that could produce a violating
instance", while `bmm_persistence/create_bmm3_model.rs` IS one (it builds
`BmmFunction`/`BmmProcedure`/`BmmConstant`/`BmmProperty`). The conclusion
still holds — those instances satisfy the invariants by construction — but
the reason does not.

Related: [[lang-bmm3-two-schema-chimera]].
