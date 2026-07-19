//! Stage 1 — LOAD. Parse the vendored inputs into verbatim in-memory models,
//! with no analysis or decisions: the BMM meta-model ([`bmm`]), the XSD reader
//! ([`xsd`]) and the OAS reader ([`oas`]). Every later stage consumes these
//! loaded models; none of them reach back to the raw files.

pub(crate) mod bmm;
pub(crate) mod oas;
pub(crate) mod xsd;
