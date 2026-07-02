//! `VERSION<T>` — abstract model of one Version within a Version container.
//!
//! openEHR class: `VERSION<T>` (abstract), package
//! `common.change_control`.
//!
//! Abstract model of one Version within a Version container, containing
//! data, commit audit trail, and the identifier of its Contribution. Has
//! exactly two concrete subtypes, `ORIGINAL_VERSION<T>` and
//! `IMPORTED_VERSION<T>`.
use openehr_base::identification::object_ref::ObjectRef;
use openehr_base::identification::object_version_id::ObjectVersionId;

use crate::common::change_control::imported_version::ImportedVersion;
use crate::common::change_control::original_version::OriginalVersion;
use crate::common::generic::audit_details::AuditDetails;
use crate::data_types::text::dv_coded_text::DvCodedText;
use serde::{Deserialize, Serialize};

/// Shared attribute state of `VERSION<T>` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct +
/// marker trait), both concrete `VERSION<T>` subtypes embed this struct.
/// Only the three declared attributes live here; the abstract functions
/// (`uid`, `preceding_version_uid`, `data`, `lifecycle_state`,
/// `canonical_form`, `owner_id`, `is_branch`) are not attributes and are
/// instead exposed via [`VersionApi`], since `ORIGINAL_VERSION` stores its
/// answers directly while `IMPORTED_VERSION` computes them by delegating
/// to its wrapped `item`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionData {
    /// `contribution`: Contribution in which this version was added.
    pub contribution: ObjectRef,

    /// `signature`: OpenPGP digital signature or digest of content
    /// committed in this Version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    /// `commit_audit`: audit trail corresponding to the committal of this
    /// version to the `VERSIONED_OBJECT`.
    pub commit_audit: AuditDetails,
}

/// `VERSION<T>` is abstract and used polymorphically wherever an attribute
/// or return type is declared of that type (e.g.
/// `VERSIONED_OBJECT.all_versions()`, `.version_with_id()`). Per ADR-001
/// §4 (closed subtype set → enum) — explicitly named for this class by the
/// invoking transcription task — the two concrete subtypes
/// `ORIGINAL_VERSION<T>` and `IMPORTED_VERSION<T>` are collected into this
/// closed `enum`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// PORT NOTE: untagged per ADR-002 — dispatch is driven by each concrete
// payload's own TypeTag, which rejects a mismatched `_type`.
#[serde(untagged)]
pub enum Version<T> {
    /// `ORIGINAL_VERSION<T>`.
    Original(OriginalVersion<T>),
    /// `IMPORTED_VERSION<T>`.
    Imported(ImportedVersion<T>),
}

/// Behaviour trait for `VERSION<T>` and its descendants, providing the
/// abstract class's declared functions uniformly whether the caller holds
/// a concrete type or a `Version<T>` enum value.
///
/// `data(): T` is declared here as `fn data(&self) -> &T` (borrowing)
/// rather than by value, since the spec's `T` is an unconstrained generic
/// parameter with no `Clone`/`Copy` bound and every declared use
/// (`VERSIONED_OBJECT.commit_original_version(..., a_data: T, ...)`
/// aside, which is itself a fresh value, not an extraction) is read-only
/// access to already-stored data.
pub trait VersionApi<T> {
    /// `uid(): OBJECT_VERSION_ID` (abstract).
    ///
    /// Unique identifier of this `VERSION`, in the form of an `{object_id,
    /// a version_tree_id, creating_system_id}` triple, where the
    /// `object_id` has the same value as the containing `VERSIONED_OBJECT
    /// uid`.
    fn uid(&self) -> &ObjectVersionId;

    /// `preceding_version_uid(): OBJECT_VERSION_ID` (abstract).
    ///
    /// Unique identifier of the version of which this version is a
    /// modification; Void if this is the first version.
    fn preceding_version_uid(&self) -> Option<&ObjectVersionId>;

    /// `data(): T` (abstract).
    ///
    /// The data of this Version. Original content of this Version.
    ///
    /// PORT NOTE: borrows rather than returning by value — see the trait
    /// doc comment.
    fn data(&self) -> Option<&T>;

    /// `lifecycle_state(): DV_CODED_TEXT` (abstract).
    ///
    /// Lifecycle state of this version; coded by openEHR vocabulary
    /// "version lifecycle state".
    fn lifecycle_state(&self) -> &DvCodedText;

    /// `canonical_form(): String`.
    ///
    /// A canonical serial form of this Version, suitable for generating
    /// reliable hashes and signatures. Canonical form of Version object,
    /// created by serialising all attributes except signature.
    ///
    /// TODO(port): the exact serialisation is stated by the spec prose
    /// (`common.change_control` §Digital Signature) to be "not yet defined
    /// by openEHR" ("[.tbd] To Be Determined"); left unimplemented pending
    /// that determination and the P4/P5 canonical-serialization phases.
    fn canonical_form(&self) -> String {
        todo!(
            "VersionApi::canonical_form: exact serialisation form is spec-TBD (common.change_control, Digital Signature)"
        )
    }

    /// `owner_id(): HIER_OBJECT_ID`.
    ///
    /// Copy of the owning `VERSIONED_OBJECT.uid` value; extracted from the
    /// local `uid` property's `object_id`.
    ///
    /// Post: `Result.value.is_equal(uid.object_id.value)`.
    ///
    /// TODO(port): depends on `ObjectVersionId::object_id()`, itself
    /// deferred pending the format-sniffing UID sub-parser noted in
    /// `openehr-base::identification::uid_based_id`.
    fn owner_id(&self) -> openehr_base::identification::hier_object_id::HierObjectId {
        todo!("VersionApi::owner_id: depends on ObjectVersionId::object_id()")
    }

    /// `is_branch(): Boolean`.
    ///
    /// True if this Version represents a branch. Derived from `uid`
    /// attribute.
    ///
    /// Delegates to `OBJECT_VERSION_ID.is_branch()`, itself delegating to
    /// `VERSION_TREE_ID.is_branch()`.
    fn is_branch(&self) -> bool {
        self.uid().is_branch()
    }
}

impl<T> VersionApi<T> for Version<T> {
    fn uid(&self) -> &ObjectVersionId {
        match self {
            Version::Original(v) => VersionApi::<T>::uid(v),
            Version::Imported(v) => VersionApi::<T>::uid(v),
        }
    }

    fn preceding_version_uid(&self) -> Option<&ObjectVersionId> {
        match self {
            Version::Original(v) => VersionApi::<T>::preceding_version_uid(v),
            Version::Imported(v) => VersionApi::<T>::preceding_version_uid(v),
        }
    }

    fn data(&self) -> Option<&T> {
        match self {
            Version::Original(v) => VersionApi::<T>::data(v),
            Version::Imported(v) => VersionApi::<T>::data(v),
        }
    }

    fn lifecycle_state(&self) -> &DvCodedText {
        match self {
            Version::Original(v) => VersionApi::<T>::lifecycle_state(v),
            Version::Imported(v) => VersionApi::<T>::lifecycle_state(v),
        }
    }
}

// Invariants (spec `Invariants` table, not yet enforced by a
// constructor/`Validate` impl — see `.claude/rules/rm-transcription.md`
// "Invariants"):
//   Preceding_version_uid_validity:
//     uid.version_tree_id.is_first xor preceding_version_uid /= Void
//     TODO(port): needs VERSION_TREE_ID.is_first(), not yet transcribed on
//     `openehr_base::identification::version_tree_id::VersionTreeId`
//     (out of scope for this change_control/directory pass — flagged for
//     the identification-package owner).
//   Lifecycle_state_valid:
//     lifecycle_state /= Void and then terminology(Term_id_openehr)
//       .has_code_for_group_id(Group_id_version_lifecycle_state,
//       lifecycle_state.defining_code)
//     TODO(port): requires the terminology service binding
//     (`openehr-terminology`) wired through the RM invariant framework.

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.change_control §VERSION — docs/research/spec-cache/RM-1.1.0/uml_classes/version.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-change_control_package.adoc §Class Descriptions / version.adoc §VERSION Class
//   confidence: high
//   todos: 4
//   note: ADR-001 §4-named worked example (VersionData + closed Version<T> enum + VersionApi<T> trait); data() borrows (&T) rather than by-value since T carries no Clone bound in the spec.
// ─────────────────────────────────────────────
