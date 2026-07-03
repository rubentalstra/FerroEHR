//! `REVISION_HISTORY` — a history of audit items associated with versions.
//!
//! openEHR class: `REVISION_HISTORY` (concrete), package `common.generic`.
//!
//! Defines the notion of a revision history of audit items, each
//! associated with the version for which that audit was committed. The
//! list is in most-recent-first order.
//!
//! The classes `REVISION_HISTORY` and `REVISION_HISTORY_ITEM` express the
//! notion of a revision history, which consists of audit items, each
//! associated with a revision number. These classes provide an
//! interoperable definition of revision history for the `VERSIONED_OBJECT`
//! and `AUTHORED_RESOURCE` classes.
use super::revision_history_item::RevisionHistoryItem;
use openehr_foundation::serde_support::{TypeName, TypeTag};
// `ObjectVersionId::value()` is an inherent accessor, so no trait import is
// needed to read a revision item's version id string form.
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "REVISION_HISTORY";

/// `REVISION_HISTORY` declares no `Inherit` row in the spec table.
///
/// PORT NOTE: the class-level Purpose text states "The list is in
/// most-recent-first order", while the `items` attribute's own row
/// description states "The items in this history in most-recent-**last**
/// order" — a direct textual contradiction between the class overview and
/// the attribute table within the same cached spec file. The two derived
/// functions below ([`RevisionHistory::most_recent_version`] and
/// [`RevisionHistory::most_recent_version_time_committed`]) settle the
/// question unambiguously: both postconditions read from `items.last`
/// (`Result.is_equal(items.last.version_id.value)` and
/// `Result.is_equal(items.last.audits.first.time_committed.value)`), which
/// is only meaningful — "the most recent version is the *last* item" — if
/// the list is stored in most-recent-**last** order. Transcribed per the
/// attribute row and the two functions' own postconditions (most-recent-
/// last), not the class-level Purpose prose, and flagged here rather than
/// silently picking one without a trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisionHistory {
    /// Canonical `_type` discriminator (`"REVISION_HISTORY"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `items`: `List<REVISION_HISTORY_ITEM>`, cardinality `1..1`.
    ///
    /// The items in this history in most-recent-last order — see the
    /// struct-level PORT NOTE for the order-direction ambiguity this
    /// reading resolves.
    pub items: Vec<RevisionHistoryItem>,
}

impl TypeName for RevisionHistory {
    const NAME: &'static str = TYPE_NAME;
}

impl RevisionHistory {
    /// `most_recent_version(): String`, cardinality `1..1`.
    ///
    /// Post: `Result.is_equal(items.last.version_id.value)`.
    ///
    /// The version id of the most recent item, as a String.
    ///
    /// Reads `items.last.version_id.value` (the `OBJECT_VERSION_ID`'s
    /// inherent `value()` accessor). The `items` list is spec-guaranteed
    /// non-empty (cardinality `1..1`); the empty case — not reachable via a
    /// spec-valid instance — yields the empty String rather than panicking,
    /// consistent with the crate-wide no-`unwrap` rule.
    pub fn most_recent_version(&self) -> String {
        self.items
            .last()
            .map_or_else(String::new, |item| item.version_id.value().to_string())
    }

    /// `most_recent_version_time_committed(): String`, cardinality `1..1`.
    ///
    /// Post: `Result.is_equal(items.last.audits.first.time_committed.value)`.
    ///
    /// The commit date/time of the most recent item, as a String.
    ///
    /// Reads `items.last.audits.first.time_committed.value`
    /// (`DV_DATE_TIME.value`, the stored ISO 8601 string). As with
    /// [`RevisionHistory::most_recent_version`], the spec guarantees both
    /// `items` (`1..1`) and each item's `audits` (`1..1`, "always at least
    /// one commit audit") are non-empty; the not-spec-reachable empty case
    /// yields the empty String.
    pub fn most_recent_version_time_committed(&self) -> String {
        self.items
            .last()
            .and_then(|item| item.audits.first())
            .map_or_else(String::new, |audit| audit.data.time_committed.value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::change_control::versioned_object::test_support::{audit, ovid};

    /// Builds a two-item history in most-recent-**last** order (v1 then v2),
    /// each item carrying a single commit audit.
    fn history() -> RevisionHistory {
        let item1 = RevisionHistoryItem {
            type_tag: TypeTag::new(),
            version_id: ovid("87284370-2d4b-4e3d-a3f3-f303d2f4f34b::test.sys::1"),
            audits: vec![audit("2020-01-01T00:00:01")],
        };
        let item2 = RevisionHistoryItem {
            type_tag: TypeTag::new(),
            version_id: ovid("87284370-2d4b-4e3d-a3f3-f303d2f4f34b::test.sys::2"),
            audits: vec![audit("2020-01-01T00:00:05")],
        };
        RevisionHistory {
            type_tag: TypeTag::new(),
            items: vec![item1, item2],
        }
    }

    #[test]
    fn most_recent_version_reads_the_last_items_version_id() {
        // Post: Result.is_equal(items.last.version_id.value) — most-recent-last.
        assert_eq!(
            history().most_recent_version(),
            "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::test.sys::2"
        );
    }

    #[test]
    fn most_recent_version_time_committed_reads_last_items_first_audit() {
        // Post: Result.is_equal(items.last.audits.first.time_committed.value).
        assert_eq!(
            history().most_recent_version_time_committed(),
            "2020-01-01T00:00:05"
        );
    }

    #[test]
    fn empty_history_yields_empty_strings_rather_than_panicking() {
        // The empty list is not a spec-valid instance (items is 1..1); the
        // accessors degrade to "" instead of unwrapping.
        let empty = RevisionHistory {
            type_tag: TypeTag::new(),
            items: Vec::new(),
        };
        assert_eq!(empty.most_recent_version(), "");
        assert_eq!(empty.most_recent_version_time_committed(), "");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/revision_history.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Revision History / uml_classes/revision_history.adoc §REVISION_HISTORY Class
//   confidence: high
//   todos: 0
//   note: Class-level Purpose text ("most-recent-first") directly contradicts the items attribute row ("most-recent-last") within the same cached spec file; resolved in favor of most-recent-last per both derived functions' own postconditions (both read items.last), flagged loudly rather than silently picking one. Both derived functions now implemented over items.last: most_recent_version reads ObjectVersionId::value (inherent), most_recent_version_time_committed reads DvDateTime.value; empty-list (spec-impossible, items is 1..1) degrades to "" per the no-unwrap rule. Spec-derived unit tests pin both postconditions and the ordering.
// ─────────────────────────────────────────────
