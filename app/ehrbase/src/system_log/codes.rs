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

/// DICOM `EventID` `csd-code` for "Patient Record".
pub const EVENT_PATIENT_RECORD_CODE: &str = "110110";
/// DICOM `EventID` `csd-code` for "Application Activity".
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

// ── ATNA rendering of the SM event enums ─────────────────────────────────────
// The SM `System Log` event model (`ehrbase_sm::{EventActionCode, EventOutcome,
// ObjectClass}`) is a pure, transport-agnostic model with no methods. The DICOM
// / RFC-3881 renderings live here, in the ATNA layer, as an extension trait so
// the `message` serializer's call sites (`event.action.as_char()`,
// `event.object.event_id()`, …) resolve through the trait.

use ehrbase_sm::{EventActionCode, EventOutcome, ObjectClass};

/// The ATNA (DICOM / RFC-3881) rendering of the SM System Log event enums.
pub(crate) trait AtnaCodes {
    /// The single-character DICOM `EventActionCode`.
    fn as_char(&self) -> char {
        unreachable!()
    }
    /// The DICOM `ParticipantObjectDataLifeCycle` most fitting for the action.
    fn data_life_cycle(&self) -> &'static str {
        unreachable!()
    }
    /// The numeric DICOM `EventOutcomeIndicator`.
    fn as_i32(&self) -> i32 {
        unreachable!()
    }
    /// The `EventOutcomeDescription` text.
    fn description(&self) -> &'static str {
        unreachable!()
    }
    /// The DICOM `EventID` `(csd-code, originalText)` for this object class.
    fn event_id(&self) -> (&'static str, &'static str) {
        unreachable!()
    }
    /// Whether this class carries a patient (Patient-Number) participant object.
    fn is_patient_centric(&self) -> bool {
        unreachable!()
    }
    /// Whether this class carries an object-URI participant.
    fn has_object_uri(&self) -> bool {
        unreachable!()
    }
    /// Whether this class renders a Search-Criteria (query) participant.
    fn is_query(&self) -> bool {
        unreachable!()
    }
}

impl AtnaCodes for EventActionCode {
    /// `C`/`R`/`U`/`D`/`E` (DICOM PS3.15 §A.5.1).
    fn as_char(&self) -> char {
        match self {
            EventActionCode::Create => 'C',
            EventActionCode::Read => 'R',
            EventActionCode::Update => 'U',
            EventActionCode::Delete => 'D',
            EventActionCode::Execute => 'E',
        }
    }

    /// The DICOM `ParticipantObjectDataLifeCycle` (RFC 3881 §5.5.5):
    /// create→origination(1), update→amendment(3), read/execute→access-use(6),
    /// delete→logical-deletion(14).
    fn data_life_cycle(&self) -> &'static str {
        match self {
            EventActionCode::Create => "1",
            EventActionCode::Update => "3",
            EventActionCode::Read | EventActionCode::Execute => "6",
            EventActionCode::Delete => "14",
        }
    }
}

impl AtnaCodes for EventOutcome {
    /// The numeric DICOM indicator (`0`/`4`/`8`/`12`).
    fn as_i32(&self) -> i32 {
        match self {
            EventOutcome::Success => 0,
            EventOutcome::MinorFailure => 4,
            EventOutcome::SeriousFailure => 8,
            EventOutcome::MajorFailure => 12,
        }
    }

    /// The `EventOutcomeDescription` text.
    fn description(&self) -> &'static str {
        match self {
            EventOutcome::Success => "Operation performed successfully",
            _ => "Operation failed",
        }
    }
}

impl AtnaCodes for ObjectClass {
    /// The DICOM `EventID` `(csd-code, originalText)` for this class (§3).
    fn event_id(&self) -> (&'static str, &'static str) {
        match self {
            ObjectClass::Ehr => (EVENT_PATIENT_RECORD_CODE, "Patient Record"),
            ObjectClass::Composition => (EVENT_PATIENT_RECORD_CODE, "composition"),
            ObjectClass::Contribution => (EVENT_PATIENT_RECORD_CODE, "contribution"),
            ObjectClass::Directory => (EVENT_PATIENT_RECORD_CODE, "directory"),
            ObjectClass::Query => (EVENT_PATIENT_RECORD_CODE, "query"),
            // PORT NOTE: the binding doc's §3 EventID table does not define
            // template provisioning (its data-op family varies `originalText`
            // under one DCM csd-code). Templates are not patient data, so they
            // use the Application-Activity code (110100) with
            // `originalText="template"` — same varied-text pattern.
            ObjectClass::Template => (EVENT_APPLICATION_ACTIVITY_CODE, "template"),
            // PORT NOTE: demographic parties are person-identifiable, so they
            // use the Patient-Record code (110110) with
            // `originalText="demographic"` (doc §3 pattern; not in its table).
            ObjectClass::Demographic => (EVENT_PATIENT_RECORD_CODE, "demographic"),
            ObjectClass::ApplicationActivity => {
                (EVENT_APPLICATION_ACTIVITY_CODE, "Application Activity")
            }
        }
    }

    /// Whether this class carries a patient (Patient-Number) participant object.
    fn is_patient_centric(&self) -> bool {
        matches!(
            self,
            ObjectClass::Ehr
                | ObjectClass::Composition
                | ObjectClass::Contribution
                | ObjectClass::Directory
        )
    }

    /// Whether this class carries an object-URI participant (in addition to the
    /// patient one, for the patient-centric classes) when an object id is known.
    fn has_object_uri(&self) -> bool {
        matches!(
            self,
            ObjectClass::Composition
                | ObjectClass::Contribution
                | ObjectClass::Directory
                | ObjectClass::Template
                | ObjectClass::Demographic
        )
    }

    /// Whether this class renders a Search-Criteria (query) participant.
    fn is_query(&self) -> bool {
        matches!(self, ObjectClass::Query)
    }
}
