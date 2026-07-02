//! `IMPORTED_VERSION<T>` — a Version wrapping an `ORIGINAL_VERSION<T>`
//! copied from elsewhere.
//!
//! openEHR class: `IMPORTED_VERSION<T>`, package
//! `common.change_control`.
//! Inherits: `VERSION<T>`.
//!
//! Versions whose content is an `ORIGINAL_VERSION` copied from another
//! location; this class inherits `commit_audit` and `contribution` from
//! `VERSION<T>`, providing imported versions with their own audit trail
//! and Contribution, distinct from those of the imported
//! `ORIGINAL_VERSION`. Acts as a wrapper: its own `uid`,
//! `preceding_version_uid`, `lifecycle_state`, and `data` are all
//! "computed" (per the spec's "(effected)" function-row annotation) by
//! delegating to the wrapped `item`, rather than storing independent
//! values — an `IMPORTED_VERSION` does not have its own version identifier
//! distinct from the version it is wrapping.
use crate::common::change_control::original_version::OriginalVersion;
use crate::common::change_control::version::{VersionApi, VersionData};
use openehr_base::identification::object_version_id::ObjectVersionId;

use crate::data_types::text::dv_coded_text::DvCodedText;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 (Refinements), `serde` derives wait until P4.
pub const TYPE_NAME: &str = "IMPORTED_VERSION";

/// `IMPORTED_VERSION<T>` — wraps a copied `ORIGINAL_VERSION<T>`.
///
/// PORT NOTE: the field is named `item` per the spec's own attribute name
/// (`item: ORIGINAL_VERSION`), and holds an [`OriginalVersion<T>`]
/// directly (not the generic `Version<T>` enum), matching the spec's
/// declared type exactly — `IMPORTED_VERSION` cannot wrap another
/// `IMPORTED_VERSION`.
#[derive(Debug, Clone)]
pub struct ImportedVersion<T> {
    /// Embedded `VERSION<T>` state (`contribution`, `signature`,
    /// `commit_audit`) per ADR-001 §3. Per the class description, these
    /// are the *local* act-of-committal contribution and audit, distinct
    /// from the ones embedded inside the wrapped `item`.
    pub version: VersionData,

    /// `item`: the `ORIGINAL_VERSION` object that was imported.
    pub item: Box<OriginalVersion<T>>,
}

impl<T> VersionApi<T> for ImportedVersion<T> {
    /// `uid(): OBJECT_VERSION_ID` (effected).
    ///
    /// Computed version of inheritance precursor, derived as `item.uid`.
    ///
    /// Post: `Result = item.uid`.
    fn uid(&self) -> &ObjectVersionId {
        VersionApi::<T>::uid(self.item.as_ref())
    }

    /// `preceding_version_uid(): OBJECT_VERSION_ID` (effected).
    ///
    /// Computed version of inheritance precursor, derived as
    /// `item.preceding_version_uid`.
    ///
    /// Post: `Result = item.preceding_version_uid`.
    fn preceding_version_uid(&self) -> Option<&ObjectVersionId> {
        VersionApi::<T>::preceding_version_uid(self.item.as_ref())
    }

    /// `data(): T` (effected).
    ///
    /// Original content of this Version.
    fn data(&self) -> Option<&T> {
        VersionApi::<T>::data(self.item.as_ref())
    }

    /// `lifecycle_state(): DV_CODED_TEXT` (effected).
    ///
    /// Lifecycle state of the content item in wrapped `ORIGINAL_VERSION`,
    /// derived as `item.lifecycle_state`; coded by openEHR vocabulary
    /// "version lifecycle state".
    fn lifecycle_state(&self) -> &DvCodedText {
        VersionApi::<T>::lifecycle_state(self.item.as_ref())
    }
}

// This class declares no Invariants table of its own (its "Functions" row
// annotations of "(effected)" carry each function's Post-condition inline
// via the "= item.<x>" wording, transcribed as this file's method bodies
// above rather than as a separate Invariants block).

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.change_control §IMPORTED_VERSION — docs/research/spec-cache/RM-1.1.0/uml_classes/imported_version.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-change_control_package.adoc §Class Descriptions / imported_version.adoc §IMPORTED_VERSION Class
//   confidence: high
//   todos: 0
//   note: item boxed for a modest sizeof win (OriginalVersion<T> carries several Option<Vec<..>> fields) — not a recursion hazard per se, since ImportedVersion cannot wrap another ImportedVersion, but IMPORTED_VERSION and ORIGINAL_VERSION are mutually referenced via Version<T> which does recurse structurally through VersionedObject.
// ─────────────────────────────────────────────
