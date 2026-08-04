//! FerroEHR's optional-integration extensions — FHIR, events, multimedia —
//! carved out of the platform library behind one additive cargo feature per
//! integration (tracker #1890; no openEHR spec governs these surfaces — our
//! own design/extensions).

#[cfg(feature = "multimedia")]
pub mod multimedia;
