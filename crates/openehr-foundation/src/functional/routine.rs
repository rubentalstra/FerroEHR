//! `ROUTINE<ARGS>` — a callable with 0 or more TUPLE-represented arguments.
//!
//! openEHR class: `ROUTINE<ARGS>`, package `base.foundation_types.functional`.
//!
//! Per the functional meta-types chapter overview: two key abstractions are
//! required — "function as a type" and "tuple" — plus a `ROUTINE` type to
//! provide the function type, and (for completeness) a `PROCEDURE` type.
//!
//! # Spec ambiguity (flagged, not silently resolved)
//!
//! `ROUTINE<ARGS>`'s own per-class table gives its Description as "Type
//! representing a function with a return type and 0 or more arguments
//! represented as a TUPLE" — but `ROUTINE`'s declared generic signature is
//! `ROUTINE<ARGS>` (one type parameter, no result type), and it is
//! `FUNCTION<ARGS,RESULT>` (`function.rs`, which `Inherit`s `ROUTINE`) that
//! actually adds a second, `RESULT`-typed parameter. `PROCEDURE<ARGS>`
//! (`procedure.rs`, which also `Inherit`s `ROUTINE`) explicitly has no
//! result type per its own description ("a procedure with 0 or more
//! arguments"), which only makes sense as a `ROUTINE` specialization if
//! `ROUTINE` itself has no `RESULT` parameter either — corroborating the
//! *signature* over the *description text*. This looks like a copy-paste
//! artifact in the published table (`FUNCTION`'s description duplicated onto
//! `ROUTINE`) rather than an intentional constraint. Per the hard rule
//! against inventing spec content, `ROUTINE<ARGS>` is transcribed from its
//! *signature* (no result type) — not from the copy-pasted description text
//! — and this discrepancy is recorded here rather than silently "corrected"
//! in either direction.
use super::tuple::Tuple;

/// `ROUTINE<ARGS>` declares no attributes and no functions of its own in its
/// per-class table (it exists to be specialized by `FUNCTION`/`PROCEDURE`),
/// so it is modelled as an empty marker trait, generic over the argument
/// tuple type `Args`, bounded by `Tuple` (`super::tuple::Tuple`) per the
/// constrained-generic transcription rule (ADR-001 §5).
pub trait Routine<Args: Tuple> {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.functional §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/routine.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master08-functional.adoc §Class Definitions / routine.adoc §ROUTINE Class
//   confidence: low
//   todos: 0
//   note: ROUTINE's per-class table Description text ("a function with a return type") appears to be copy-pasted from FUNCTION and contradicts ROUTINE's own single-parameter signature and PROCEDURE's no-result description; transcribed from the signature (no RESULT parameter on ROUTINE itself), discrepancy documented rather than silently resolved either way — a spec author/errata check would raise confidence here.
// ─────────────────────────────────────────────
