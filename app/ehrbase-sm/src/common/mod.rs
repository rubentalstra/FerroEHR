//! The SM `platform.common` package (`master03-common_package.adoc`) plus the
//! master02 global conventions every interface shares.
//!
//! Contents (master03 §Overview): "`I_STATUS` / `CALL_STATUS`: a
//! representation of the status result of any call execution;
//! `UPDATE_VERSION`: an information structure suitable for committing data to
//! a versioned store …; `PLATFORM_SERVICE`: an enumeration of the available
//! services". Plus `I_VALIDITY_CHECKER` (included by the same chapter) and
//! the master02 §List Handling cursor ([`Page`]).
//!
//! Module map: [`status`] (call status + the error model), [`version_update`]
//! (the version-commit envelope), [`platform_service`], [`validity`],
//! [`list`].

pub mod list;
pub mod platform_service;
pub mod status;
pub mod validity;
pub mod version_update;

pub use list::Page;
pub use platform_service::PlatformService;
pub use status::{CallStatus, CallStatusType, SmError};
pub use validity::ValidityChecker;
pub use version_update::{UpdateAttestation, UpdateAudit, UpdateVersion};
