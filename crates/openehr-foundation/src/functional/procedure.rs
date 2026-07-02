//! `PROCEDURE<ARGS>` — a callable with 0 or more TUPLE-represented arguments
//! and no return value.
//!
//! openEHR class: `PROCEDURE<ARGS>`, package
//! `base.foundation_types.functional`.
//! Inherits: `ROUTINE`.
//!
//! Type representing a procedure with 0 or more arguments represented as a
//! TUPLE. Declares no attributes or functions of its own beyond its
//! `ROUTINE` ancestry. Per the functional meta-types chapter overview,
//! `PROCEDURE` exists "for completeness" alongside `FUNCTION`.
use super::tuple::Tuple;

/// Same treatment as `Function<Args, Result>` (`super::function`): a
/// documented type alias over a `Fn` trait object, here `dyn Fn(Args)`
/// (returning unit `()`) since a procedure has no declared result type.
///
/// PORT NOTE: the spec does not state whether a `PROCEDURE` may mutate
/// captured state (Eiffel agents do not carry Rust's `Fn`/`FnMut`/`FnOnce`
/// distinction at all); `dyn Fn(Args)` is chosen for consistency with
/// `Function`'s `dyn Fn(Args) -> Result` shape rather than assuming
/// `FnMut`/`FnOnce`, which the spec gives no basis to prefer. As with
/// `Function`, the `ROUTINE<ARGS>` ancestry and the `Args: Tuple` constraint
/// cannot be encoded on a bare type alias and are documented rather than
/// enforced at this layer.
pub type Procedure<Args> = dyn Fn(Args);

// PORT NOTE: see `function.rs`'s equivalent documentation-only function for
// why `Args: Tuple` is recorded here rather than expressed as an alias bound.
#[allow(dead_code)]
fn _args_bound_documentation_only<Args: Tuple>() {}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 foundation_types.functional §Class Definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/procedure.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master08-functional.adoc §Class Definitions / procedure.adoc §PROCEDURE Class
//   confidence: medium
//   todos: 0
//   note: type alias over dyn Fn(Args), matching Function's treatment with no result type; Fn (not FnMut/FnOnce) chosen for consistency with Function since the spec gives no basis to distinguish mutability. ROUTINE<ARGS> ancestry and Args: Tuple constraint documented, not encoded, same as function.rs.
// ─────────────────────────────────────────────
