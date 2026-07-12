//! The Message service (`service/message/`) — the openEHR **Message component**
//! of the platform crate: SM `I_MESSAGE_SERVICE` / `I_EHR_EXTRACT_SERVICE` /
//! `I_TDD_SERVICE` (`docs/specs/openehr/SM/docs/openehr_platform/master09-message_service.adoc`
//! and the UML classes `i_message_service.adoc`, `i_ehr_extract_service.adoc`,
//! `i_tdd_service.adoc`). The design register is
//! `docs/design/platform/06-service-message-admin.md`.
//!
//! Layout mirrors the spec's own export / import / TDD decomposition, one file
//! per concern:
//!
//! - [`export`] — `I_EHR_EXTRACT_SERVICE.export_ehrs` / `export_ehr_extracts`:
//!   the extract-building algorithm over the stored versions (RM EHR-Extract IM
//!   `master05`/`master09`).
//! - [`import`] — `I_EHR_EXTRACT_SERVICE.import_ehr` / `import_ehr_extract`:
//!   parse a received `EXTRACT` and dispatch clone-vs-append; the
//!   `IMPORTED_VERSION` replay is [`crate::versioning`] (RM common master06
//!   §Copying).
//! - [`tdd`] — `I_TDD_SERVICE.import_tdd` / `import_tdds`: TDD XML →
//!   COMPOSITION → validated commit.
//!
//! `I_MESSAGE_SERVICE` declares no functions (`i_message_service.adoc`), so it
//! gets no code. Every file carries its domain logic; this `mod.rs` holds the
//! thin `impl <Interface>Service for EhrbaseService` adapters that delegate to
//! it (the `service/api/` pattern).
//!
//! # Integration seams — `TODO(w3f-integrate)` (reconciled at the fix pass)
//!
//! - **`crate::versioning`** — [`import`] reaches `commit_import` /
//!   `commit_demographic_import` (IMPORTED_VERSION replay); [`export`] reaches
//!   `original_version` / `versioned_object` / `revision_history` / the version
//!   reads. These are the versioning-engine surface, called directly.
//! - **`crate::templates` + `crate::validation`** — [`tdd`] resolves the OPT and
//!   commits through the validated COMPOSITION path (currently
//!   `EhrbaseService::web_template_for` / `get_template_xml` / `create_composition`).
//! - **`crate::aql`** — deferred `EXTRACT_SPEC.criteria` / `commit_time_interval`
//!   (G-M3 / G-M4), typed rejects until then.
//! - **The extension REST wire (G-M1) lives in `ehrbase-rest`**, not here
//!   (ITS-REST vends no message endpoints — a message/admin extension route is
//!   spec-silent transport, our own extension); this crate only needs its trait
//!   impls reachable.
//! - **`service/mod.rs`**: the orchestrator adds `mod message;` and removes the
//!   legacy flat `message` / `tdd` modules.

mod export;
mod import;
mod tdd;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use ehrbase_sm::{EhrExtractService, SmError, TddService};
use openehr_rm::ehr_extract::common::extract::Extract;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;

use crate::service::EhrbaseService;

#[async_trait]
impl EhrExtractService for EhrbaseService {
    async fn export_ehrs(&self, an_ehr_id: Uuid) -> Result<Vec<Value>, SmError> {
        self.export_all_ehrs(an_ehr_id).await
    }

    async fn export_ehr_extracts(&self, extract_spec: ExtractSpec) -> Result<Vec<Value>, SmError> {
        self.export_ehr_extracts_spec(extract_spec).await
    }

    async fn import_ehr(
        &self,
        an_ehr_id: Option<Uuid>,
        an_extract: Extract,
    ) -> Result<(), SmError> {
        self.import_whole_ehr(an_ehr_id, an_extract).await
    }

    async fn import_ehr_extract(
        &self,
        an_ehr_id: Uuid,
        an_extract: Extract,
    ) -> Result<(), SmError> {
        self.import_into_ehr(an_ehr_id, an_extract).await
    }
}

#[async_trait]
impl TddService for EhrbaseService {
    async fn import_tdd(&self, an_ehr_id: Uuid, tdd: String) -> Result<String, SmError> {
        self.import_one_tdd(an_ehr_id, &tdd).await
    }

    async fn import_tdds(
        &self,
        an_ehr_id: Uuid,
        tdds: Vec<String>,
    ) -> Result<Vec<String>, SmError> {
        self.import_tdds_batch(an_ehr_id, &tdds).await
    }
}
