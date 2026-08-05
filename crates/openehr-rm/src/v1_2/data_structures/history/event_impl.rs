//! Hand-written RM spec functions for the abstract `EVENT<T>` enum.
//!
//! Spec: RM 1.2.0
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_structures.event.adoc`.
//! `EVENT.offset() = time.diff(parent.origin)` — the parent back-reference is
//! not stored (repo convention: no owning back-refs), so the offset is
//! computed from an explicitly supplied `HISTORY.origin`
//! ([`Event::offset_from`]); the `HISTORY` invariant checks call it with their
//! own origin.

use crate::v1_2::data_structures::history::event::Event;
use crate::v1_2::data_types::quantity::date_time::dv_date_time::DvDateTime;
use crate::v1_2::data_types::quantity::date_time::dv_duration::DvDuration;

/// Build the `DV_DURATION` for a number of seconds (`PT<n>S`, negative
/// durations use the openEHR leading-sign deviation).
fn duration_from_seconds(secs: f64) -> DvDuration {
    let magnitude = secs.abs();
    // Emit an integral second count without a fractional part.
    #[expect(
        clippy::float_cmp,
        reason = "an exact-integrality test is precisely a bit-equality question (`x.floor() == x`), not a tolerance comparison"
    )]
    let body = if magnitude.floor() == magnitude {
        format!("PT{magnitude:.0}S")
    } else {
        format!("PT{magnitude}S")
    };
    DvDuration {
        normal_status: None,
        normal_range: None,
        other_reference_ranges: None,
        magnitude_status: None,
        accuracy: None,
        accuracy_is_percent: None,
        value: if secs < 0.0 { format!("-{body}") } else { body },
    }
}

/// Offset in seconds of an event `time` from a history `origin`, when both
/// magnitudes are available.
pub(crate) fn offset_seconds(time: &DvDateTime, origin: &DvDateTime) -> Option<f64> {
    Some(time.magnitude()? - origin.magnitude()?)
}

impl<T> Event<T> {
    /// The event `time` (common to both `POINT_EVENT` and `INTERVAL_EVENT`).
    #[must_use]
    pub fn time(&self) -> &DvDateTime {
        match self {
            Self::IntervalEvent(e) => &e.time,
            Self::PointEvent(e) => &e.time,
        }
    }

    /// The event `data`.
    #[must_use]
    pub fn data(&self) -> &T {
        match self {
            Self::IntervalEvent(e) => &e.data,
            Self::PointEvent(e) => &e.data,
        }
    }

    /// RM `EVENT.offset()`: offset of this event from the history origin,
    /// `time.diff(origin)`, as a `DV_DURATION` in seconds. `None` when either
    /// temporal magnitude is unavailable (malformed value). The `parent`
    /// back-reference of the spec signature is supplied explicitly (no owning
    /// back-refs; see module doc).
    #[must_use]
    pub fn offset_from(&self, origin: &DvDateTime) -> Option<DvDuration> {
        offset_seconds(self.time(), origin).map(duration_from_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_2::data_structures::history::point_event::PointEvent;
    use crate::v1_2::data_types::text::dv_text::{DvText, DvTextData};

    fn text(value: &str) -> DvText {
        DvText::DvText(DvTextData {
            value: value.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: openehr_base::containers::present_nonempty(Vec::new()),
            language: None,
            encoding: None,
        })
    }

    fn date_time(value: &str) -> DvDateTime {
        DvDateTime {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: None,
            magnitude_status: None,
            accuracy: None,
            value: value.to_owned(),
        }
    }

    fn event(time: &str) -> Event<i32> {
        Event::PointEvent(PointEvent {
            name: text("event"),
            archetype_node_id: "at0002".to_owned(),
            uid: None,
            links: openehr_base::containers::present_nonempty(Vec::new()),
            archetype_details: None,
            feeder_audit: None,
            time: date_time(time),
            state: None,
            data: 1,
        })
    }

    #[test]
    fn offset_is_time_minus_origin() {
        let origin = date_time("2021-01-01T00:00:00");
        let e = event("2021-01-01T01:00:00");
        assert_eq!(
            e.offset_from(&origin).map(|d| d.value),
            Some("PT3600S".to_owned())
        );
        // Event before origin → negative duration.
        let early = event("2020-12-31T23:59:00");
        assert_eq!(
            early.offset_from(&origin).map(|d| d.value),
            Some("-PT60S".to_owned())
        );
        // Offsets round-trip through DV_DURATION.magnitude().
        assert_eq!(
            e.offset_from(&origin).and_then(|d| d.magnitude()),
            Some(3600.0)
        );
    }

    #[test]
    fn offset_unavailable_for_malformed_time() {
        let origin = date_time("not-a-time");
        assert!(event("2021-01-01T01:00:00").offset_from(&origin).is_none());
    }
}
