// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The Message service (`service/message/`) — the openEHR **Message component**.
//!
//! The platform crate's realization of SM `I_MESSAGE_SERVICE` /
//! `I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE`
//! (`docs/specs/openehr/SM/docs/openehr_platform/master09-message_service.adoc`
//! and the UML classes `i_message_service.adoc`, `i_ehr_extract_service.adoc`,
//! `i_tdd_service.adoc`).
//!
//! Layout mirrors the spec's own export / import / TDD decomposition, one file
//! per concern, each carrying its public `FerroEhrService` methods and the
//! machinery behind them:
//!
//! - `export` — `I_EHR_EXTRACT_SERVICE.export_ehrs` / `export_ehr_extracts`:
//!   the extract-building algorithm over the stored versions (RM EHR-Extract IM
//!   `master05`/`master09`).
//! - `import` — `I_EHR_EXTRACT_SERVICE.import_ehr` / `import_ehr_extract`:
//!   parse a received `EXTRACT` and dispatch clone-vs-append; the
//!   `IMPORTED_VERSION` replay is [`crate::versioning`] (RM common master06
//!   §Copying).
//! - `tdd` — `I_TDD_SERVICE.import_tdd` / `import_tdds`: TDD XML →
//!   COMPOSITION → validated commit.
//!
//! `I_MESSAGE_SERVICE` declares no functions (`i_message_service.adoc`), so it
//! gets no code.
//!
//! # Cross-module wiring
//!
//! - **`crate::versioning`** — `import` reaches `commit_import` /
//!   `commit_demographic_import` (`IMPORTED_VERSION` replay); `export` reaches
//!   `original_version` / `versioned_object` / `revision_history` / the version
//!   reads. These are the versioning-engine surface, called directly.
//! - **`crate::templates` + `crate::validation`** — `tdd` resolves the OPT and
//!   commits through the validated COMPOSITION path
//!   (`FerroEhrService::web_template_for` / `get_template_xml` /
//!   `create_composition`).
//! - **ATNA audit** — a completed export/import emits one EHR-Extract audit
//!   event (`FerroEhrService::emit_extract_audit`) for non-repudiation (BASE
//!   `architecture_overview/master07-security.adoc` §Non-repudiation).
//! - **`crate::aql`** — `EXTRACT_SPEC.criteria` / `commit_time_interval` remain
//!   typed rejects pending the `$ehr`-bound AQL export wave (see the NOTEs
//!   in `export`).
//! - **The extension REST wire lives in `ferroehr-rest`**, not here (ITS-REST
//!   vends no message endpoints — a message/admin extension route is
//!   spec-silent transport, our own extension).

mod export;
mod import;
mod tdd;

use crate::ids::EhrId;
use crate::service::FerroEhrService;
use crate::system_log::event::{AuditEvent, EventActionCode, EventOutcome, ObjectClass};

impl FerroEhrService {
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
    pub(super) fn emit_extract_audit(&self, ehr_id: EhrId, action: EventActionCode) {
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
