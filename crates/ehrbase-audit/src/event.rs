//! The transport-agnostic audit input the server hands the sender.
//!
//! The REST audit layer builds an [`AuditEvent`] from the matched operation
//! (via [`crate::table`]), the authenticated principal, the client address and
//! the response status; the background drain task turns it into a DICOM
//! [`AuditMessage`](crate::message::AuditMessage) and ships it. Nothing here
//! knows about HTTP or the DB — the event carries only the resolved facts.

use jiff::Timestamp;

/// DICOM `EventActionCode` (PS3.15 §A.5.1): the CRUD/execute class of the op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventActionCode {
    /// `C` — create.
    Create,
    /// `R` — read.
    Read,
    /// `U` — update.
    Update,
    /// `D` — delete.
    Delete,
    /// `E` — execute (query).
    Execute,
}

impl EventActionCode {
    /// The single-character DICOM code.
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            EventActionCode::Create => 'C',
            EventActionCode::Read => 'R',
            EventActionCode::Update => 'U',
            EventActionCode::Delete => 'D',
            EventActionCode::Execute => 'E',
        }
    }

    /// The DICOM `ParticipantObjectDataLifeCycle` most fitting for this action
    /// (RFC 3881 §5.5.5): create→origination(1), update→amendment(3),
    /// read/execute→access-use(6), delete→logical-deletion(14).
    #[must_use]
    pub const fn data_life_cycle(self) -> &'static str {
        match self {
            EventActionCode::Create => "1",
            EventActionCode::Update => "3",
            EventActionCode::Read | EventActionCode::Execute => "6",
            EventActionCode::Delete => "14",
        }
    }
}

/// DICOM `EventOutcomeIndicator` (PS3.15 §A.5.1), derived from the HTTP status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOutcome {
    /// `0` — success.
    Success,
    /// `4` — minor failure (the action failed; e.g. 4xx incl. 401/403).
    MinorFailure,
    /// `8` — serious failure (the action was not performed; e.g. 5xx).
    SeriousFailure,
    /// `12` — major failure (the node/service is compromised/unavailable).
    MajorFailure,
}

impl EventOutcome {
    /// The numeric DICOM indicator.
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        match self {
            EventOutcome::Success => 0,
            EventOutcome::MinorFailure => 4,
            EventOutcome::SeriousFailure => 8,
            EventOutcome::MajorFailure => 12,
        }
    }

    /// Map an HTTP status code to an outcome per the binding doc §8.2:
    /// 2xx→0, 4xx→4 (incl. 401/403), 5xx→8, else 4.
    #[must_use]
    pub const fn from_http_status(status: u16) -> Self {
        match status {
            200..=299 => EventOutcome::Success,
            500..=599 => EventOutcome::SeriousFailure,
            _ => EventOutcome::MinorFailure,
        }
    }

    /// The `EventOutcomeDescription` text.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            EventOutcome::Success => "Operation performed successfully",
            _ => "Operation failed",
        }
    }
}

/// The class of resource an operation touches — determines the DICOM `EventID`
/// and the participant-object rendering (binding doc §3 field mapping).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectClass {
    /// EHR / EHR_STATUS → "Patient Record"; a Patient-Number participant.
    Ehr,
    /// Composition → "composition"; Patient-Number + object-URI participants.
    Composition,
    /// Contribution → "contribution"; Patient-Number + object-URI participants.
    Contribution,
    /// Directory (FOLDER) → "directory"; Patient-Number + object-URI participants.
    Directory,
    /// Ad-hoc / stored query execution → "query"; a Search-Criteria participant.
    Query,
    /// Login / application activity → "Application Activity"; no clinical object.
    ApplicationActivity,
}

impl ObjectClass {
    /// The DICOM EventID `(csd-code, originalText)` for this class (§3).
    #[must_use]
    pub const fn event_id(self) -> (&'static str, &'static str) {
        use crate::codes::{EVENT_APPLICATION_ACTIVITY_CODE, EVENT_PATIENT_RECORD_CODE};
        match self {
            ObjectClass::Ehr => (EVENT_PATIENT_RECORD_CODE, "Patient Record"),
            ObjectClass::Composition => (EVENT_PATIENT_RECORD_CODE, "composition"),
            ObjectClass::Contribution => (EVENT_PATIENT_RECORD_CODE, "contribution"),
            ObjectClass::Directory => (EVENT_PATIENT_RECORD_CODE, "directory"),
            ObjectClass::Query => (EVENT_PATIENT_RECORD_CODE, "query"),
            ObjectClass::ApplicationActivity => {
                (EVENT_APPLICATION_ACTIVITY_CODE, "Application Activity")
            }
        }
    }

    /// Whether this class carries a patient (Patient-Number) participant object.
    #[must_use]
    pub const fn is_patient_centric(self) -> bool {
        matches!(
            self,
            ObjectClass::Ehr
                | ObjectClass::Composition
                | ObjectClass::Contribution
                | ObjectClass::Directory
        )
    }

    /// Whether this class carries a distinct object-URI participant (in addition
    /// to the patient one) when an object id is known.
    #[must_use]
    pub const fn has_object_uri(self) -> bool {
        matches!(
            self,
            ObjectClass::Composition | ObjectClass::Contribution | ObjectClass::Directory
        )
    }

    /// Whether this class renders a Search-Criteria (query) participant.
    #[must_use]
    pub const fn is_query(self) -> bool {
        matches!(self, ObjectClass::Query)
    }
}

/// A fully-resolved audit event, ready to be rendered into a DICOM AuditMessage.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// The CRUD/execute class.
    pub action: EventActionCode,
    /// The resource class (drives EventID + participant objects).
    pub object: ObjectClass,
    /// The response outcome.
    pub outcome: EventOutcome,
    /// The requesting user (Basic username / OAuth `sub`; `UNKNOWN` when absent).
    pub user_id: String,
    /// Whether the source participant is the requestor (always true here).
    pub user_is_requestor: bool,
    /// The client network address (`X-Forwarded-First-hop`/peer), if known.
    pub client_ip: Option<String>,
    /// The owning EHR id, for optional background subject enrichment.
    pub ehr_id: Option<String>,
    /// The resolved object identifier (resource URI / version uid / query name).
    pub object_id: Option<String>,
    /// The event time.
    pub timestamp: Timestamp,
}

impl AuditEvent {
    /// Construct an event with the given class + outcome, defaulting the runtime
    /// fields (the caller fills user/ip/ids). `timestamp` is set to now.
    #[must_use]
    pub fn new(action: EventActionCode, object: ObjectClass, outcome: EventOutcome) -> Self {
        Self {
            action,
            object,
            outcome,
            user_id: String::new(),
            user_is_requestor: true,
            client_ip: None,
            ehr_id: None,
            object_id: None,
            timestamp: Timestamp::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_status_mapping() {
        assert_eq!(EventOutcome::from_http_status(201), EventOutcome::Success);
        assert_eq!(EventOutcome::from_http_status(200), EventOutcome::Success);
        assert_eq!(
            EventOutcome::from_http_status(401),
            EventOutcome::MinorFailure
        );
        assert_eq!(
            EventOutcome::from_http_status(403),
            EventOutcome::MinorFailure
        );
        assert_eq!(
            EventOutcome::from_http_status(404),
            EventOutcome::MinorFailure
        );
        assert_eq!(
            EventOutcome::from_http_status(500),
            EventOutcome::SeriousFailure
        );
        assert_eq!(
            EventOutcome::from_http_status(503),
            EventOutcome::SeriousFailure
        );
    }

    #[test]
    fn action_char_and_lifecycle() {
        assert_eq!(EventActionCode::Create.as_char(), 'C');
        assert_eq!(EventActionCode::Execute.as_char(), 'E');
        assert_eq!(EventActionCode::Create.data_life_cycle(), "1");
        assert_eq!(EventActionCode::Delete.data_life_cycle(), "14");
    }
}
