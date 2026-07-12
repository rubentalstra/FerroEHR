//! The SM Demographic service (`master06-demographic_service.adoc`):
//! `I_DEMOGRAPHIC_SERVICE` / `I_PARTY` / `I_PARTY_RELATIONSHIP`, with the
//! `UV_PARTY` / `UV_PARTY_RELATIONSHIP` commit envelopes as instantiations of
//! the common [`UpdateVersion`](crate::common::UpdateVersion) (master03
//! §Version Update Semantics).
//!
//! Module map: [`service`] (`I_DEMOGRAPHIC_SERVICE` + `I_PARTY`, the
//! wire-seam realization), [`relationship`] (`I_PARTY_RELATIONSHIP`).
//!
//! PORT NOTE (wire): ITS-REST 1.0.3 defines **no** demographic wire contract
//! (the CNF demographic schedule is TBD; demographic is OPTIONS-profile
//! only). The wire-shaped methods on these traits are therefore our own
//! extension **by analogy with the EHR group** — parties are versioned
//! objects on the same machinery with no EHR scope, and the status codes /
//! `ETag` / `Location` / `Prefer` / `If-Match` behaviour mirrors the EHR
//! group. A demographic `ResourceMeta` carries an empty `ehr_id`.

pub mod relationship;
pub mod service;

pub use relationship::PartyRelationshipService;
pub use service::{DemographicService, PartyKind};
