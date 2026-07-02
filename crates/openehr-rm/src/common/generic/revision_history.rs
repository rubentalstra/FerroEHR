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
    /// TODO(port): `ObjectVersionId` (the type of
    /// `RevisionHistoryItem::version_id`) does not yet expose a plain
    /// `.value` string accessor call site here — its `UidBasedId`/
    /// `ObjectId` ancestry (`openehr_base::identification`) provides
    /// `value()` via the `UidBasedIdApi`/`ObjectIdApi` traits (see
    /// `uid_based_id.rs`), so this is directly wireable once the trait is
    /// in scope; left `todo!()` for now to avoid pulling in the trait
    /// import ahead of confirming the exact accessor path other RM
    /// classes settle on for reading an `ObjectVersionId`'s string form.
    pub fn most_recent_version(&self) -> String {
        todo!(
            "RevisionHistory::most_recent_version: items.last().version_id needs a UidBasedIdApi::value() call once that trait import path is confirmed for RM call sites"
        )
    }

    /// `most_recent_version_time_committed(): String`, cardinality `1..1`.
    ///
    /// Post: `Result.is_equal(items.last.audits.first.time_committed.value)`.
    ///
    /// The commit date/time of the most recent item, as a String.
    ///
    /// TODO(port): needs `DvDateTime` (forward-referenced, `data_types`
    /// package, not yet landed in this worktree) to expose a `.value`
    /// string accessor; left `todo!()`.
    pub fn most_recent_version_time_committed(&self) -> String {
        todo!(
            "RevisionHistory::most_recent_version_time_committed: needs DvDateTime's string-value accessor, not yet landed (data_types package)"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.generic — docs/research/spec-cache/RM-1.1.0/uml_classes/revision_history.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master04-generic_package.adoc §Revision History / uml_classes/revision_history.adoc §REVISION_HISTORY Class
//   confidence: medium
//   todos: 2
//   note: Class-level Purpose text ("most-recent-first") directly contradicts the items attribute row ("most-recent-last") within the same cached spec file; resolved in favor of most-recent-last per both derived functions' own postconditions (both read items.last), flagged loudly rather than silently picking one. Both derived functions left todo!()-bodied pending a settled ObjectVersionId/DvDateTime string-value accessor call-site convention.
// ─────────────────────────────────────────────
