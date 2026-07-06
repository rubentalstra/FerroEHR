//! DICOM (DCM) / RFC-3881 code constants used in the DICOM Audit Message.
//!
//! Values are the well-known DICOM PS3.15 §A.5 and IETF RFC 3881 audit code
//! sets. Only the codes the CDR emits are listed; each carries its code system
//! and display text so the serializer never hard-codes a string twice.

/// A coded value: `csd-code` + `codeSystemName` + `originalText`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Code {
    /// The `csd-code` attribute (the numeric code within the system).
    pub csd_code: &'static str,
    /// The `codeSystemName` attribute (`DCM` or `RFC-3881`).
    pub code_system: &'static str,
    /// The `originalText` attribute (human-readable display).
    pub original_text: &'static str,
}

/// DICOM Controlled Terminology code system name.
pub const DCM: &str = "DCM";
/// IETF RFC 3881 code system name (the older audit code set DICOM inherited).
pub const RFC_3881: &str = "RFC-3881";

// ── EventID (DICOM PS3.15 §A.5.1, `DCM`) ─────────────────────────────────────
// EHR / clinical-data ops use "Patient Record" (110110); the CDR varies the
// `originalText` per resource (composition/contribution/directory/query) while
// keeping the DICOM `csd-code` 110110 — the mapping the binding doc §3 pins.
// PORT NOTE: query execution could use the distinct DICOM code 110112 ("Query");
// the binding doc groups it under the data-op "Patient Record" family with
// originalText="query", which we follow (docs/enterprise/atna-audit.md §3).

/// DICOM EventID `csd-code` for "Patient Record".
pub const EVENT_PATIENT_RECORD_CODE: &str = "110110";
/// DICOM EventID `csd-code` for "Application Activity".
pub const EVENT_APPLICATION_ACTIVITY_CODE: &str = "110100";

// ── RoleIDCode (DICOM PS3.15 §A.5.2, `DCM`) ──────────────────────────────────

/// The requesting-node "Source Role ID".
pub const ROLE_SOURCE: Code = Code {
    csd_code: "110153",
    code_system: DCM,
    original_text: "Source Role ID",
};
/// The serving-node "Destination Role ID".
pub const ROLE_DESTINATION: Code = Code {
    csd_code: "110152",
    code_system: DCM,
    original_text: "Destination Role ID",
};

// ── AuditSourceTypeCode (DICOM PS3.15 §A.5.3) ────────────────────────────────

/// "Application Server Process or Thread" (code 4).
pub const SOURCE_TYPE_APPLICATION_SERVER: Code = Code {
    csd_code: "4",
    code_system: DCM,
    original_text: "Application Server Process or Thread",
};

// ── ParticipantObjectIDTypeCode (RFC 3881 §5.5.1) ────────────────────────────

/// Patient Number (RFC-3881 code 2) — the EHR subject id participant.
pub const OBJ_ID_PATIENT_NUMBER: Code = Code {
    csd_code: "2",
    code_system: RFC_3881,
    original_text: "Patient Number",
};
/// Search Criteria (RFC-3881 code 10) — the AQL query participant.
pub const OBJ_ID_SEARCH_CRITERIA: Code = Code {
    csd_code: "10",
    code_system: RFC_3881,
    original_text: "Search Criteria",
};
/// URI (RFC-3881 code 12) — a clinical object's resource URI participant.
pub const OBJ_ID_URI: Code = Code {
    csd_code: "12",
    code_system: RFC_3881,
    original_text: "URI",
};

// ── NetworkAccessPointTypeCode (RFC 3881 §5.3) ───────────────────────────────

/// IP Address network-access-point type.
pub const NETWORK_ACCESS_POINT_IP: &str = "2";

// ── ParticipantObjectTypeCode / *Role (RFC 3881 §5.5) ────────────────────────

/// Participant object type: person (`1`).
pub const OBJECT_TYPE_PERSON: &str = "1";
/// Participant object type: system object (`2`).
pub const OBJECT_TYPE_SYSTEM: &str = "2";
/// Participant object type-code role: patient (`1`).
pub const OBJECT_ROLE_PATIENT: &str = "1";
/// Participant object type-code role: query (`24`).
pub const OBJECT_ROLE_QUERY: &str = "24";
