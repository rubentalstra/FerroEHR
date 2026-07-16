//! EHR-service types (SM `I_EHR_SERVICE` / `I_EHR_CONTRIBUTION`,
//! `docs/specs/openehr/SM/docs/openehr_platform/master05-ehr_service.adoc`).
//!
//! The former generic `I_EHR` accessor facade died with the trait catalog
//! (W-14 B+C): the concrete service exposes the calls directly.

use serde_json::Value;

// --- EHR summary (SM I_EHR_SERVICE) ---
#[derive(Debug, Clone)]
pub struct EhrSummary {
    /// `ehr_id: UUID` — "EHR identifier of this EHR."
    pub ehr_id: String,
    /// `system_id: String` — "Copy of `EHR.system_id`."
    pub system_id: String,
    /// `ehr_status: EHR_STATUS` — "Copy of `EHR.ehr_status`" (canonical JSON).
    pub ehr_status: Value,
    /// `time_created: Iso8601_date_time` — "Copy of `EHR.time_created`."
    pub time_created: String,
    /// `contribution_count: Integer` — "Number of Contributions in this EHR."
    pub contribution_count: i64,
    /// `composition_count: Integer` — "Number of (versioned) Compositions in
    /// this EHR."
    pub composition_count: i64,
}

// --- contribution list time range (SM I_EHR_CONTRIBUTION) ---
/// The optional inclusive `(lower, upper)` ISO-8601 bounds of an SM
/// `Interval<Iso8601_date_time>` — either side open (`None`) means unbounded.
pub type TimeRange = Option<(Option<String>, Option<String>)>;
