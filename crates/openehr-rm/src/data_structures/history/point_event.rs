//! `POINT_EVENT<T>` — a single point event in a series.
//!
//! openEHR class: `POINT_EVENT<T>`, package `rm.data_structures.history`.
//!
//! Defines a single point event in a series. Declares no attributes or
//! functions of its own beyond what it inherits from `EVENT<T>`.

use super::event::{EventApi, EventData};

/// `POINT_EVENT<T>` class.
///
/// Embeds the shared `EVENT<T>` state (per ADR-001 §3, combined with §5 for
/// the generic parameter).
#[derive(Debug, Clone, PartialEq)]
pub struct PointEvent<T> {
    /// Inherited `EVENT<T>` (and transitively `LOCATABLE`) state.
    pub event: EventData<T>,
}

impl<T> EventApi<T> for PointEvent<T> {
    fn event_data(&self) -> &EventData<T> {
        &self.event
    }
}

pub const TYPE_NAME: &str = "POINT_EVENT";

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_structures.history §POINT_EVENT — docs/research/spec-cache/RM-1.1.0/uml_classes/point_event.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-history_package.adoc §Class Descriptions / point_event.adoc §POINT_EVENT Class
//   confidence: high
//   todos: 0
//   note: pure embedding, no additional attributes/functions/invariants declared in the spec table.
// ─────────────────────────────────────────────
