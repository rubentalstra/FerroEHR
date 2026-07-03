//! openEHR TERM 3.x — terminology bundle and terminology service API.
//!
//! Spec crate: bundles the vendored TERM Release-3.0.0 XML assets
//! (`openehr_term.xml` per language, the external ISO/IANA code sets,
//! `PropertyUnitData.xml`) and implements the `rm.support.terminology`
//! service surface (`TERMINOLOGY_SERVICE`, `TERMINOLOGY_ACCESS`,
//! `CODE_SET_ACCESS`, and the two identifier constants classes).
//!
//! Unlike the Phase-A crates this one is wired and compiles (P2 is a
//! dependency leaf): `cargo test -p openehr-terminology` exercises the
//! bundled assets, including the SPECPR-51 `id=532` dual-rubric quirk, which
//! is preserved verbatim.

pub mod assets;
pub mod bundle;
pub mod code_set_access;
pub mod error;
pub mod openehr_code_set_identifiers;
pub mod openehr_terminology_group_identifiers;
pub mod property_unit_data;
pub mod terminology_access;
pub mod terminology_code;
pub mod terminology_service;

pub use bundle::{Code, CodeSet, Concept, ConceptGroup, Terminology};
pub use code_set_access::{BundledCodeSetAccess, CodeSetAccess};
pub use error::TerminologyError;
pub use openehr_code_set_identifiers::OpenehrCodeSetIdentifiers;
pub use openehr_terminology_group_identifiers::OpenehrTerminologyGroupIdentifiers;
pub use property_unit_data::{Property, PropertyUnitData, Unit};
pub use terminology_access::{BundledTerminologyAccess, TerminologyAccess};
pub use terminology_code::TerminologyCode;
pub use terminology_service::TerminologyService;
