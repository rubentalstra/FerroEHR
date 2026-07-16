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
//! # Cross-module wiring
//!
//! - **`crate::versioning`** — [`import`] reaches `commit_import` /
//!   `commit_demographic_import` (`IMPORTED_VERSION` replay); [`export`] reaches
//!   `original_version` / `versioned_object` / `revision_history` / the version
//!   reads. These are the versioning-engine surface, called directly.
//! - **`crate::templates` + `crate::validation`** — [`tdd`] resolves the OPT and
//!   commits through the validated COMPOSITION path
//!   (`EhrbaseService::web_template_for` / `get_template_xml` /
//!   `create_composition`).
//! - **ATNA audit** — a completed export/import emits one EHR-Extract audit
//!   event ([`EhrbaseService::emit_extract_audit`]) for non-repudiation (BASE
//!   `architecture_overview/master07-security.adoc` §Non-repudiation).
//! - **`crate::aql`** — `EXTRACT_SPEC.criteria` / `commit_time_interval` remain
//!   typed rejects pending the `$ehr`-bound AQL export wave (see the PORT NOTEs
//!   in [`export`]; register 06 G-M3 / G-M4).
//! - **The extension REST wire (G-M1) lives in `ehrbase-rest`**, not here
//!   (ITS-REST vends no message endpoints — a message/admin extension route is
//!   spec-silent transport, our own extension); this crate only needs its trait
//!   impls reachable.

mod export;
mod import;
mod tdd;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::service::SmError;
use crate::system_log::event::{AuditEvent, EventActionCode, EventOutcome, ObjectClass};
use openehr_rm::ehr_extract::common::extract::Extract;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;

use crate::service::EhrbaseService;

impl EhrbaseService {
    /// Emit one IHE-ATNA EHR-Extract audit event for a completed export or
    /// import of `ehr_id` (fire-and-forget, like every other emission — an
    /// audit-delivery failure never fails the operation).
    ///
    /// EHR-Extract communication carries patient-identifiable clinical data
    /// across systems and is audited for **non-repudiation** — the security
    /// chapter requires that "logging of communication of Extracts … can be
    /// used to guarantee non-repudiation of information passed between systems"
    /// (BASE `architecture_overview/master07-security.adoc` §Non-repudiation).
    /// The event carries [`ObjectClass::Extract`], which renders the
    /// Patient-Record `EventID` family (DICOM PS3.15 §A.5.1, code 110110) with
    /// `originalText="extract"` and an ehr-id-scoped patient participant
    /// (`crate::system_log::codes`); the export-vs-import direction is carried
    /// by the [`EventActionCode`] (`Read` out / `Create` in). The native
    /// service layer has no HTTP principal, so `user_id` stays empty and the
    /// ATNA renderer supplies `UNKNOWN`.
    pub(super) fn emit_extract_audit(&self, ehr_id: Uuid, action: EventActionCode) {
        if !self.audit_enabled() {
            return;
        }
        let mut event = AuditEvent::new(action, ObjectClass::Extract, EventOutcome::Success);
        let id = ehr_id.to_string();
        event.ehr_id = Some(id.clone());
        event.object_id = Some(id);
        let _ = self.emit(event);
    }
}

impl EhrbaseService {
    pub async fn extract_ehrs(&self, an_ehr_id: Uuid) -> Result<Vec<Value>, SmError> {
        self.export_all_ehrs(an_ehr_id).await
    }

    pub async fn export_ehr_extracts(&self, extract_spec: ExtractSpec) -> Result<Vec<Value>, SmError> {
        self.export_ehr_extracts_spec(extract_spec).await
    }

    pub async fn import_ehr(
        &self,
        an_ehr_id: Option<Uuid>,
        an_extract: Extract,
    ) -> Result<(), SmError> {
        self.import_whole_ehr(an_ehr_id, an_extract).await
    }

    pub async fn import_ehr_extract(
        &self,
        an_ehr_id: Uuid,
        an_extract: Extract,
    ) -> Result<(), SmError> {
        self.import_into_ehr(an_ehr_id, an_extract).await
    }
}

impl EhrbaseService {
    pub async fn import_tdd(&self, an_ehr_id: Uuid, tdd: String) -> Result<String, SmError> {
        self.import_one_tdd(an_ehr_id, &tdd).await
    }

    pub async fn import_tdds(
        &self,
        an_ehr_id: Uuid,
        tdds: Vec<String>,
    ) -> Result<Vec<String>, SmError> {
        self.import_tdds_batch(an_ehr_id, &tdds).await
    }
}
