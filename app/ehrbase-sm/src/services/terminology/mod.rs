//! The SM Terminology service (`master12-terminology_service.adoc`):
//! `I_TERMINOLOGY_SERVICE` plus the terminology-extract model
//! (`TERMINOLOGY_DESCRIPTION`, `TERMINOLOGY_EXTRACT`, `TERM_CODE`,
//! `DEFINED_TERM`, `TERM_RELATIONSHIP`, `TERMINOLOGY_RELATION`).
//!
//! "It includes a model for terminology extracts consisting, in general, of
//! terms (either in bare code form or full definition) and relationships"
//! (master12 §Overview).

pub mod service;

pub use service::{
    DefinedTerm, TermCode, TermEntry, TermRelationship, TerminologyDescription, TerminologyExtract,
    TerminologyRelation, TerminologyRelationError, TerminologyService,
};
