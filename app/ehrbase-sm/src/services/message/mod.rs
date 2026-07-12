//! The SM Message service (`master09-message_service.adoc`):
//! `I_MESSAGE_SERVICE` / `I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE`.
//!
//! `I_MESSAGE_SERVICE` itself declares **no functions** in the vendored spec
//! (`i_message_service.adoc` has an empty function table) — the component is
//! realized by its two concrete interfaces: [`extract`] (EHR Extract
//! export/import over the RM `ehr_extract` model) and [`tdd`] (Template Data
//! Document import).

pub mod extract;
pub mod tdd;

pub use extract::EhrExtractService;
pub use tdd::TddService;
