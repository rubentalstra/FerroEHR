//! `FUNCTION<ARGS,RESULT>` — a callable with a return type and 0 or more
//! TUPLE-represented arguments.
//!
//! openEHR class: `FUNCTION<ARGS,RESULT>`, package
//! `base.foundation_types.functional`.
//! Inherits: `ROUTINE`.
//!
//! Type representing a function with a return type and 0 or more arguments
//! represented as a TUPLE. See `routine.rs` for the spec-ambiguity note on
//! why `ROUTINE<ARGS>` itself is transcribed without a result type while
//! `FUNCTION` is the class that actually adds one.
use super::tuple::Tuple;

/// `FUNCTION<ARGS,RESULT>` declares no attributes or functions of its own in
/// its per-class table beyond its `ROUTINE` ancestry and the addition of the
/// `RESULT` generic parameter — per the functional meta-types chapter
/// overview, "UML does not contain native functional elements, [so] the
/// semantics here are approximated using normal class facilities," and the
/// natural Rust approximation of "a callable value with argument type `Args`
/// and result type `Result`" is a `Fn` trait-object type alias rather than a
/// zero-method marker trait.
///
/// PORT NOTE: transcribed as a documented type alias over `dyn Fn(Args) ->
/// Result` rather than a struct or a marker trait, per the task's
/// functional-types mapping guidance ("where a faithful struct shape is
/// impossible, transcribe as documented type aliases over `Fn` traits"). A
/// marker-trait shape (matching `Routine`/`Tuple`) was considered and
/// rejected here specifically because `FUNCTION` is meant to be an
/// *invocable value* (it "represents a function"), not merely a
/// classification of one — a type alias over `dyn Fn` preserves callability,
/// which a marker trait cannot express without also requiring every
/// implementor to separately declare a `call`/`apply` method the spec itself
/// never names. The `ROUTINE<ARGS>` `Inherit` relationship cannot be
/// expressed on a type alias (aliases have no distinct `impl` target); it is
/// documented here rather than encoded, and is the reason this class is not
/// also given an `impl Routine<Args> for ...` block.
pub type Function<Args, Result> = dyn Fn(Args) -> Result;

// PORT NOTE: `Args: Tuple` is not written as an explicit trait bound on the
// `Function` alias above because Rust does not permit bounds on a bare `dyn
// Fn(Args) -> Result` alias target in this position without also naming a
// concrete `Args` at every use site; the constraint is documented here
// instead, and enforced at whatever call site eventually instantiates
// `Function<SomeTuple, R>` with a concrete tuple type from `tuple1`/`tuple2`.
#[allow(dead_code)]
fn _args_bound_documentation_only<Args: Tuple>() {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.functional §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/function.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master08-functional.adoc §Class Definitions / function.adoc §FUNCTION Class
//   confidence: medium
//   todos: 0
//   note: type alias over dyn Fn(Args) -> Result rather than a marker trait, since FUNCTION represents an invocable value; the ROUTINE<ARGS> ancestry and the Args: Tuple constraint cannot be encoded on the alias itself and are documented instead of enforced by the type system at this layer.
// ─────────────────────────────────────────────
