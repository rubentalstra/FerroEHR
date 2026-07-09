//! The SM Platform Service Model interfaces, one Rust trait per interface
//! (ADR-010).
//!
//! Each trait keeps `#[async_trait]`, `Send + Sync`, and default
//! `Err(ApiError::NotImplemented)` bodies; the concrete platform component
//! (`ehrbase`) implements them and every protocol adapter (`ehrbase-rest`)
//! consumes them. The EHR mega-service is split along the SM's own interface
//! boundaries (`I_EHR_SERVICE` / `I_EHR_STATUS` / `I_EHR_COMPOSITION` /
//! `I_EHR_DIRECTORY` / `I_EHR_CONTRIBUTION`).

pub mod admin;
pub mod composition;
pub mod contribution;
pub mod definition;
pub mod demographic;
pub mod directory;
pub mod ehr;
pub mod ehr_status;
pub mod query;
pub mod system_log;
pub mod validity;
pub mod web_template;

pub use admin::AdminService;
pub use composition::EhrCompositionService;
pub use contribution::EhrContributionService;
pub use definition::{DefinitionAdl2Service, DefinitionAdl14Service, DefinitionQueryService};
pub use demographic::DemographicService;
pub use directory::EhrDirectoryService;
pub use ehr::EhrService;
pub use ehr_status::EhrStatusService;
pub use query::QueryService;
pub use system_log::SystemLog;
pub use validity::ValidityChecker;
pub use web_template::WebTemplateService;
