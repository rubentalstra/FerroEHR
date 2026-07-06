//! Hand-written RM class invariants (ADR-003) for `HISTORY`.
//!
//! Mirrors archie `History` + inherited LOCATABLE:
//! - `Events_valid`: at least one event, or a summary, must be present.
//! - `Archetype_node_id_valid`: `archetype_node_id` non-empty.
//!
//! PORT NOTE: archie's `Periodic_validity` is `ignored` (never checked), so it
//! is not implemented here.

use crate::data_structures::history::history::History;
use crate::validate::{InvariantViolation, Validate, push_archetype_node_id_valid};

impl<T> Validate for History<T> {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        if self.events.is_empty() && self.summary.is_none() {
            out.push(InvariantViolation::here(
                "Invariant Events_valid failed on type HISTORY",
            ));
        }
        push_archetype_node_id_valid(out, "HISTORY", &self.archetype_node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_structures::history::event::Event;
    use crate::data_structures::history::point_event::PointEvent;
    use crate::data_types::quantity::date_time::dv_date_time::DvDateTime;
    use crate::data_types::text::dv_text::{DvText, DvTextData};

    fn text(value: &str) -> DvText {
        DvText::DvText(DvTextData {
            value: value.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: Vec::new(),
            language: None,
            encoding: None,
        })
    }

    fn origin() -> DvDateTime {
        DvDateTime {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: Vec::new(),
            magnitude_status: None,
            accuracy: None,
            value: "2021-01-01T00:00:00".to_owned(),
        }
    }

    fn history(events: Vec<Event<i32>>) -> History<i32> {
        History {
            name: text("history"),
            archetype_node_id: "at0001".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            origin: origin(),
            period: None,
            duration: None,
            summary: None,
            events,
        }
    }

    #[test]
    fn history_with_event_valid() {
        let event = Event::PointEvent(PointEvent {
            name: text("event"),
            archetype_node_id: "at0002".to_owned(),
            uid: None,
            links: Vec::new(),
            archetype_details: None,
            feeder_audit: None,
            time: origin(),
            state: None,
            data: 1,
        });
        assert!(history(vec![event]).invariants().is_empty());
    }

    #[test]
    fn empty_history_invalid() {
        let v = history(vec![]).invariants();
        assert!(
            v.iter()
                .any(|m| m.message == "Invariant Events_valid failed on type HISTORY")
        );
    }
}
