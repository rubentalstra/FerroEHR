//! RM/BASE class-invariant contract (hand-written spec behaviour).
//!
//! Hand-written; preserved across `openehr-codegen` regeneration (it is not a
//! `// @generated` file, so `write_crate` keeps it and `lib.rs` auto-declares
//! `pub mod validate;`). Concrete invariant bodies live in sibling `*_impl.rs`
//! files next to each generated type and `impl Validate for <Type>`.
//!
//! The instance-validation *algorithm* is spec-underdetermined (openEHR's AOM2
//! `validation` spec covers archetype validation, not RM-instance validation);
//! these are the openEHR **Reference Model class invariants** — the checks a
//! value must satisfy independent of any archetype — mirroring the reference
//! implementation's invariant set. See `// PORT NOTE:` markers in the impls for
//! spec-underdetermined choices.

/// An RM class-invariant violation: a human-readable message plus the RM
/// sub-path (relative to the value being checked) it applies to (empty = the
/// value itself). The composition validator prefixes the absolute RM path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    pub path: String,
    pub message: String,
}

impl InvariantViolation {
    /// A violation on the value itself (no sub-path).
    #[must_use]
    pub fn here(message: impl Into<String>) -> Self {
        Self {
            path: String::new(),
            message: message.into(),
        }
    }

    /// A violation on a named sub-attribute of the value.
    #[must_use]
    pub fn at(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// The RM class invariants of a spec type. Implemented in `*_impl.rs` siblings.
pub trait Validate {
    /// Append this value's class-invariant violations to `out`. A value with no
    /// violations appends nothing. Implementations check only their own class
    /// invariants (not archetype constraints); the caller recurses into children.
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>);

    /// Convenience: collect this value's own invariant violations.
    #[must_use]
    fn invariants(&self) -> Vec<InvariantViolation> {
        let mut out = Vec::new();
        self.validate_invariants(&mut out);
        out
    }
}
