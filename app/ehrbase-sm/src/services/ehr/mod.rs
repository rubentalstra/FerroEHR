//! The SM EHR service (`master05-ehr_service.adoc`): `I_EHR_SERVICE` plus the
//! per-EHR interfaces `I_EHR` / `I_EHR_STATUS` / `I_EHR_COMPOSITION` /
//! `I_EHR_DIRECTORY` / `I_EHR_CONTRIBUTION`, and `EHR_SUMMARY`.
//!
//! One Rust trait per SM interface, split along the SM's own interface
//! boundaries; the per-EHR `I_EHR` accessor is realized as the generic
//! [`IEhr`] handle. Every mutating call is an implicit-CONTRIBUTION commit
//! ("with implicit Contribution creation") realized through the common
//! version-commit envelope
//! ([`UpdateVersion`](crate::common::UpdateVersion), master03 §Version
//! Update Semantics) — the `UV_FOLDER`/`UV_COMPOSITION` derivations of the
//! chapter are its instantiations.

pub mod composition;
pub mod contribution;
pub mod directory;
pub mod handle;
pub mod service;
pub mod status;

pub use composition::EhrCompositionService;
pub use contribution::{EhrContributionService, TimeRange};
pub use directory::EhrDirectoryService;
pub use handle::{
    EhrCompositionHandle, EhrContributionHandle, EhrDirectoryHandle, EhrStatusHandle, IEhr,
};
pub use service::{EhrService, EhrSummary};
pub use status::EhrStatusService;
