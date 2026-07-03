//! `VERSIONED_OBJECT<T>` — version control abstraction for one versioned
//! item.
//!
//! openEHR class: `VERSIONED_OBJECT<T>`, package
//! `common.change_control`.
//!
//! Version control abstraction, defining semantics for versioning one
//! complex object. An instance provides the versioning facilities for one
//! versioned item and is often referred to as a "version container". The
//! generic parameter `T` is the type of the versioned data (e.g.
//! `COMPOSITION`, `FOLDER`, `PARTY`), ensuring all versions in a given
//! container are of the same type — see ADR-001 §5 (constrained generic →
//! generic with trait bound), though the spec places no explicit
//! constraint on `T` beyond "the type of the data", so no trait bound is
//! applied here (compare `HISTORY<T: ITEM_STRUCTURE>`, which the spec does
//! constrain).
//!
//! `VERSIONED_FOLDER` (`common.directory` package,
//! `crate::common::directory::versioned_folder`) is the binding
//! `VERSIONED_OBJECT<FOLDER>`.
use openehr_base::identification::hier_object_id::HierObjectId;
use openehr_base::identification::object_ref::ObjectRef;
use openehr_base::identification::object_version_id::ObjectVersionId;

use openehr_base::identification::uid_based_id::UidBasedIdApi;

use crate::common::change_control::imported_version::ImportedVersion;
use crate::common::change_control::original_version::OriginalVersion;
use crate::common::change_control::version::{Version, VersionApi, VersionData};
use crate::common::generic::attestation::Attestation;
use crate::common::generic::audit_details::AuditDetails;
use crate::common::generic::revision_history::RevisionHistory;
use crate::common::generic::revision_history_item::RevisionHistoryItem;
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::text::dv_coded_text::DvCodedText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form (ITS-JSON/ITS-XML). Per ADR-001 (Refinements), `serde` derives and
/// `#[serde(rename = ...)]` wait until P4; this `const` records the
/// discriminator value in the meantime.
pub const TYPE_NAME: &str = "VERSIONED_OBJECT";

/// `VERSIONED_OBJECT<T>` — a version container for one logical item.
///
/// The spec describes the data of a `VERSIONED_OBJECT` as "a collection of
/// instances of the two `VERSION<T>` subtypes, ... available only via the
/// functional interface of `VERSIONED_OBJECT`", explicitly leaving the
/// internal representation unspecified ("How the representation of this
/// collection is implemented inside the `VERSIONED_OBJECT` is not defined
/// by this specification"). Transcribed here as an owned `Vec<Version<T>>`
/// — the simplest representation consistent with the spec's own
/// "simple... versions stored as full copies in a list" example — with
/// every declared function implemented against that field. A future phase
/// may replace this with a lazily-loaded/compressed backing store without
/// changing the public function battery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionedObject<T> {
    /// Canonical `_type` discriminator (`"VERSIONED_OBJECT"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<VersionedObject<T>>,

    /// `uid`: unique identifier of this version container in the form of a
    /// UID with no extension. This id will be the same in all instances of
    /// the same container in a distributed environment, meaning that it
    /// can be understood as the uid of the virtual version tree.
    ///
    /// Invariant `Uid_validity`: `extension.is_empty` — see
    /// [`VersionedObject::uid`] doc and the `TODO(port)` on invariant
    /// enforcement below.
    pub uid: HierObjectId,

    /// `owner_id`: reference to object to which this version container
    /// belongs, e.g. the id of the containing EHR or other relevant
    /// owning entity.
    pub owner_id: ObjectRef,

    /// `time_created`: time of initial creation of this versioned object.
    pub time_created: DvDateTime,

    /// PORT NOTE: not a declared spec attribute. Holds the versions this
    /// `VERSIONED_OBJECT` contains, per the spec's own "simple... stored
    /// as full copies in a list" example representation (see the struct
    /// doc comment above for the full quotation and rationale). Every
    /// declared spec function is implemented against this field.
    /// PORT NOTE (ADR-002/P4): `versions` is this transcription's chosen
    /// internal representation (the spec leaves storage undefined) and is NOT
    /// part of the ITS-JSON `VERSIONED_OBJECT` wire shape (uid, owner_id,
    /// time_created only, additionalProperties: false) — skipped when empty
    /// so canonical output validates; a populated store still round-trips.
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub versions: Vec<Version<T>>,
}

impl<T> TypeName for VersionedObject<T> {
    const NAME: &'static str = TYPE_NAME;
}

/// Errors raised when a `VERSIONED_OBJECT` commit function's spec
/// precondition (or a cheaply-enforceable `ORIGINAL_VERSION` invariant)
/// fails.
///
/// PORT NOTE: the spec expresses these as Eiffel `Pre` clauses (contract
/// violations); per `docs/PORTING.md` §5 (checked/unchecked exception →
/// `Result<T, E>` with `thiserror`) and ADR-003 decision 8 (constructors
/// that can enforce an invariant cheaply do so), the mutating commit
/// functions return `Result` instead of panicking on a violated
/// precondition.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionedObjectCommitError {
    /// Pre: `all_version_ids.has(a_preceding_version_uid) or else
    /// version_count = 0`.
    #[error("preceding version id {preceding_version_id:?} not found in this version container")]
    PrecedingVersionNotFound {
        /// The `a_preceding_version_id` value that matched no stored
        /// version uid.
        preceding_version_id: String,
    },
    /// Pre (`commit_attestation`): `has_version_id(a_ver_id)`.
    #[error("no version with id {version_id:?} in this version container")]
    VersionNotFound {
        /// The `a_ver_id` value that matched no stored version uid.
        version_id: String,
    },
    /// Pre (`commit_attestation`): `is_original_version(a_ver_id)` —
    /// attestations can only be added to Original versions.
    #[error("version {version_id:?} is not an ORIGINAL_VERSION")]
    NotAnOriginalVersion {
        /// The `a_ver_id` value that named an `IMPORTED_VERSION`.
        version_id: String,
    },
    /// `ORIGINAL_VERSION` invariant `Other_input_version_uids_valid`:
    /// `other_input_version_uids /= Void implies not
    /// other_input_version_uids.is_empty` — a merged commit must name at
    /// least one other input version.
    #[error("an_other_input_uids must not be empty for a merged version")]
    EmptyOtherInputUids,
}

/// The commit time of a version: `commit_audit.time_committed`. For an
/// `IMPORTED_VERSION` this is the *local* act-of-committal audit (the
/// `VERSION<T>` state), not the wrapped item's original audit, matching
/// the class description ("providing imported versions with their own
/// audit trail").
fn commit_time<T>(version: &Version<T>) -> &DvDateTime {
    match version {
        Version::Original(v) => &v.version.commit_audit.data.time_committed,
        Version::Imported(v) => &v.version.commit_audit.data.time_committed,
    }
}

impl<T> VersionedObject<T> {
    /// `version_count(): Integer`.
    ///
    /// Return the total number of versions in this object.
    pub fn version_count(&self) -> i32 {
        self.versions.len() as i32
    }

    /// `all_version_ids(): List<OBJECT_VERSION_ID>`.
    ///
    /// Return a list of ids of all versions in this object.
    pub fn all_version_ids(&self) -> Vec<ObjectVersionId> {
        self.versions.iter().map(|v| v.uid().clone()).collect()
    }

    /// `all_versions(): List<VERSION>`.
    ///
    /// Return a list of all versions in this object.
    pub fn all_versions(&self) -> &[Version<T>] {
        &self.versions
    }

    /// `has_version_at_time(a_time: DV_DATE_TIME): Boolean`.
    ///
    /// True if a version for time `a_time` exists.
    ///
    /// PORT NOTE: the spec does not define "a version for time `a_time`"
    /// beyond this one line. Transcribed with the lookup-at-a-time reading
    /// used by EHRbase's version-at-time REST semantics: a version exists
    /// for `a_time` iff at least one version's
    /// `commit_audit.time_committed` is at or before `a_time` (the
    /// container had a current version at that instant) — see
    /// [`VersionedObject::version_at_time`].
    pub fn has_version_at_time(&self, a_time: &DvDateTime) -> bool {
        self.version_at_time(a_time).is_some()
    }

    /// `has_version_id(a_version_uid: OBJECT_VERSION_ID): Boolean`.
    ///
    /// True if a version with `a_version_uid` exists.
    ///
    /// Ids are compared by their `value` string (the full
    /// `object_id '::' creating_system_id '::' version_tree_id` form).
    pub fn has_version_id(&self, a_version_uid: &ObjectVersionId) -> bool {
        self.versions
            .iter()
            .any(|v| v.uid().value() == a_version_uid.value())
    }

    /// `version_with_id(a_version_uid: OBJECT_VERSION_ID): VERSION`.
    ///
    /// Return the version with `uid` = `a_version_uid`.
    ///
    /// Pre: `has_version_id(a_ver_id)`.
    ///
    /// PORT NOTE: the spec precondition is surfaced as `Option` (`None`
    /// where `has_version_id` fails) rather than a contract violation,
    /// matching the crate-wide precedent set by
    /// `openehr_terminology::TerminologyService::terminology`.
    pub fn version_with_id(&self, a_version_uid: &ObjectVersionId) -> Option<&Version<T>> {
        self.versions
            .iter()
            .find(|v| v.uid().value() == a_version_uid.value())
    }

    /// `is_original_version(a_version_uid: OBJECT_VERSION_ID): Boolean`.
    ///
    /// True if version with `a_version_uid` is an `ORIGINAL_VERSION`.
    ///
    /// Pre: `has_version_id(a_ver_id)` — false is returned where the
    /// precondition fails (no such version), per the same Option/false
    /// convention as [`VersionedObject::version_with_id`].
    pub fn is_original_version(&self, a_version_uid: &ObjectVersionId) -> bool {
        matches!(
            self.version_with_id(a_version_uid),
            Some(Version::Original(_))
        )
    }

    /// `version_at_time(a_time: DV_DATE_TIME): VERSION`.
    ///
    /// Return the version for time `a_time`.
    ///
    /// Pre: `has_version_at_time(a_time)`.
    ///
    /// PORT NOTE: interpretation as for
    /// [`VersionedObject::has_version_at_time`] — the version current at
    /// `a_time`, i.e. the one with the greatest
    /// `commit_audit.time_committed` at or before `a_time` (ties broken
    /// toward the most recently added). Comparison uses
    /// `DV_DATE_TIME.magnitude()` (seconds since origin); the precondition
    /// is surfaced as `Option`.
    pub fn version_at_time(&self, a_time: &DvDateTime) -> Option<&Version<T>> {
        let cutoff = a_time.magnitude();
        self.versions
            .iter()
            .filter(|v| commit_time(v).magnitude() <= cutoff)
            .max_by(|a, b| {
                commit_time(a)
                    .magnitude()
                    .total_cmp(&commit_time(b).magnitude())
            })
    }

    /// `revision_history(): REVISION_HISTORY`.
    ///
    /// History of all audits and attestations in this versioned
    /// repository.
    ///
    /// Derived per the `REVISION_HISTORY`/`REVISION_HISTORY_ITEM` class
    /// descriptions: one item per version (in most-recent-last order —
    /// commits append, so list order is commit order), each carrying the
    /// version's commit audit first, then any attestations of the
    /// (wrapped, for imported versions) `ORIGINAL_VERSION`.
    ///
    /// PORT NOTE: `REVISION_HISTORY_ITEM.audits` is spec-typed
    /// `List<AUDIT_DETAILS>`, so each `ATTESTATION` is up-cast to its
    /// embedded `AUDIT_DETAILS` state, losing the attestation-specific
    /// fields — the representational limitation already flagged on
    /// `RevisionHistoryItem::audits`.
    pub fn revision_history(&self) -> RevisionHistory {
        let items =
            self.versions
                .iter()
                .map(|version| {
                    let (version_data, attestations): (&VersionData, Option<&[Attestation]>) =
                        match version {
                            Version::Original(v) => (&v.version, v.attestations.as_deref()),
                            Version::Imported(v) => (&v.version, v.item.attestations.as_deref()),
                        };
                    let mut audits = vec![version_data.commit_audit.clone()];
                    audits.extend(attestations.into_iter().flatten().map(|attestation| {
                        AuditDetails {
                            type_tag: TypeTag::new(),
                            data: attestation.audit_details.clone(),
                        }
                    }));
                    RevisionHistoryItem {
                        type_tag: TypeTag::new(),
                        version_id: version.uid().clone(),
                        audits,
                    }
                })
                .collect();
        RevisionHistory {
            type_tag: TypeTag::new(),
            items,
        }
    }

    /// `latest_version(): VERSION`.
    ///
    /// Return the most recently added version (i.e. on trunk or any
    /// branch).
    ///
    /// PORT NOTE: with the `Vec<Version<T>>` internal store, "most
    /// recently added" is the last element (the commit functions append).
    /// Returns `Option` — the spec invariant `Latest_version_valid` only
    /// guarantees a non-Void result when `version_count > 0`.
    pub fn latest_version(&self) -> Option<&Version<T>> {
        self.versions.last()
    }

    /// `latest_trunk_version(): VERSION`.
    ///
    /// Return the most recently added trunk version (i.e. skipping branch
    /// versions, whose `uid.version_tree_id` carries branch parts).
    ///
    /// PORT NOTE: `Option` for the empty/branches-only container, same
    /// rationale as [`VersionedObject::latest_version`].
    pub fn latest_trunk_version(&self) -> Option<&Version<T>> {
        self.versions
            .iter()
            .rev()
            .find(|v| !VersionApi::<T>::is_branch(*v))
    }

    /// `trunk_lifecycle_state(): DV_CODED_TEXT`.
    ///
    /// Return the lifecycle state from the latest trunk version. Useful
    /// for determining if the version container is logically deleted.
    ///
    /// Post: `Result = latest_trunk_version.lifecycle_state`.
    ///
    /// PORT NOTE: `Option`, propagated from
    /// [`VersionedObject::latest_trunk_version`]; borrows rather than
    /// copying (`DvCodedText` is owned by the version).
    pub fn trunk_lifecycle_state(&self) -> Option<&DvCodedText> {
        self.latest_trunk_version()
            .map(super::version::VersionApi::lifecycle_state)
    }

    /// Invariant `Uid_validity`: `extension.is_empty` — this container's
    /// `uid` is "a UID with no extension".
    ///
    /// Working method per ADR-003 decision 8; delegates to
    /// `UID_BASED_ID.has_extension()`.
    pub fn is_uid_valid(&self) -> bool {
        !self.uid.has_extension()
    }

    /// `commit_original_version(a_contribution: OBJECT_REF,
    /// a_new_version_uid: OBJECT_VERSION_ID, a_preceding_version_id:
    /// OBJECT_VERSION_ID, an_audit: AUDIT_DETAILS, a_lifecycle_state:
    /// DV_CODED_TEXT, a_data: T, signing_key: String)`.
    ///
    /// Add a new original version.
    ///
    /// Pre: `all_version_ids.has(a_preceding_version_uid) or else
    /// version_count = 0`.
    ///
    /// PORT NOTE: the `void` return becomes
    /// `Result<(), VersionedObjectCommitError>` so the spec precondition is
    /// enforced rather than panicking (see the error enum's PORT NOTE).
    /// When `version_count = 0` (first commit), `a_preceding_version_id`
    /// is discarded and the stored `preceding_version_uid` is Void, per
    /// the `VERSION` invariant `Preceding_version_uid_validity`
    /// (`is_first xor preceding_version_uid /= Void`).
    ///
    /// TODO(port): `signing_key` is accepted but unused — generating
    /// `signature` requires `VERSION.canonical_form()`, whose serial form
    /// the spec marks "[.tbd] To Be Determined" (`common.change_control`
    /// §Digital Signature); `signature` stays Void until openEHR defines
    /// it.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_original_version(
        &mut self,
        a_contribution: ObjectRef,
        a_new_version_uid: ObjectVersionId,
        a_preceding_version_id: ObjectVersionId,
        an_audit: AuditDetails,
        a_lifecycle_state: DvCodedText,
        a_data: T,
        signing_key: String,
    ) -> Result<(), VersionedObjectCommitError> {
        let preceding_version_uid = self.checked_preceding_version_uid(a_preceding_version_id)?;
        let _ = signing_key;
        self.versions.push(Version::Original(OriginalVersion {
            type_tag: TypeTag::new(),
            version: VersionData {
                contribution: a_contribution,
                signature: None,
                commit_audit: an_audit,
            },
            uid: a_new_version_uid,
            preceding_version_uid,
            other_input_version_uids: None,
            lifecycle_state: a_lifecycle_state,
            attestations: None,
            data: Some(a_data),
        }));
        Ok(())
    }

    /// `commit_original_merged_version(a_contribution: OBJECT_REF,
    /// a_new_version_uid: OBJECT_VERSION_ID, a_preceding_version_id:
    /// OBJECT_VERSION_ID, an_audit: AUDIT_DETAILS, a_lifecycle_state:
    /// DV_CODED_TEXT, a_data: T, an_other_input_uids:
    /// List<OBJECT_VERSION_ID>, signing_key: String)`.
    ///
    /// Add a new original merged version. This commit function adds a
    /// parameter containing the ids of other versions merged into the
    /// current one.
    ///
    /// Pre: `all_version_ids.has(a_preceding_version_uid) or else
    /// version_count = 0`.
    ///
    /// PORT NOTE: same `Result` reshape, first-commit handling, and
    /// `signing_key` deferral as
    /// [`VersionedObject::commit_original_version`]. Additionally enforces
    /// the `ORIGINAL_VERSION` invariant `Other_input_version_uids_valid`
    /// cheaply (ADR-003 decision 8): an empty `an_other_input_uids` list is
    /// rejected, since a merged version by definition has other inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_original_merged_version(
        &mut self,
        a_contribution: ObjectRef,
        a_new_version_uid: ObjectVersionId,
        a_preceding_version_id: ObjectVersionId,
        an_audit: AuditDetails,
        a_lifecycle_state: DvCodedText,
        a_data: T,
        an_other_input_uids: Vec<ObjectVersionId>,
        signing_key: String,
    ) -> Result<(), VersionedObjectCommitError> {
        if an_other_input_uids.is_empty() {
            return Err(VersionedObjectCommitError::EmptyOtherInputUids);
        }
        let preceding_version_uid = self.checked_preceding_version_uid(a_preceding_version_id)?;
        let _ = signing_key;
        self.versions.push(Version::Original(OriginalVersion {
            type_tag: TypeTag::new(),
            version: VersionData {
                contribution: a_contribution,
                signature: None,
                commit_audit: an_audit,
            },
            uid: a_new_version_uid,
            preceding_version_uid,
            other_input_version_uids: Some(an_other_input_uids),
            lifecycle_state: a_lifecycle_state,
            attestations: None,
            data: Some(a_data),
        }));
        Ok(())
    }

    /// `commit_imported_version(a_contribution: OBJECT_REF, an_audit:
    /// AUDIT_DETAILS, a_version: ORIGINAL_VERSION)`.
    ///
    /// Add a new imported version. Details of version id etc come from the
    /// `ORIGINAL_VERSION` being committed. The given contribution and
    /// audit become the imported version's *local* act-of-committal state,
    /// distinct from those inside the wrapped `ORIGINAL_VERSION` (per the
    /// `IMPORTED_VERSION` class description). No precondition is declared,
    /// so no `Result` is needed.
    pub fn commit_imported_version(
        &mut self,
        a_contribution: ObjectRef,
        an_audit: AuditDetails,
        a_version: OriginalVersion<T>,
    ) {
        self.versions.push(Version::Imported(ImportedVersion {
            type_tag: TypeTag::new(),
            version: VersionData {
                contribution: a_contribution,
                signature: None,
                commit_audit: an_audit,
            },
            item: Box::new(a_version),
        }));
    }

    /// `commit_attestation(an_attestation: ATTESTATION, a_ver_id:
    /// OBJECT_VERSION_ID, signing_key: String)`.
    ///
    /// Add a new attestation to a specified original version. Attestations
    /// can only be added to Original versions.
    ///
    /// Pre: `has_version_id(a_ver_id) and is_original_version(a_ver_id)`.
    ///
    /// PORT NOTE: `Result` reshape for the two preconditions, as on the
    /// other commit functions.
    ///
    /// TODO(port): `signing_key` unused pending the spec-TBD
    /// `canonical_form()` — see
    /// [`VersionedObject::commit_original_version`].
    pub fn commit_attestation(
        &mut self,
        an_attestation: Attestation,
        a_ver_id: ObjectVersionId,
        signing_key: String,
    ) -> Result<(), VersionedObjectCommitError> {
        let _ = signing_key;
        let target = self
            .versions
            .iter_mut()
            .find(|v| v.uid().value() == a_ver_id.value())
            .ok_or_else(|| VersionedObjectCommitError::VersionNotFound {
                version_id: a_ver_id.value().to_string(),
            })?;
        match target {
            Version::Original(original) => {
                original
                    .attestations
                    .get_or_insert_with(Vec::new)
                    .push(an_attestation);
                Ok(())
            }
            Version::Imported(_) => Err(VersionedObjectCommitError::NotAnOriginalVersion {
                version_id: a_ver_id.value().to_string(),
            }),
        }
    }

    /// Shared precondition check for the two original-version commit
    /// functions: `all_version_ids.has(a_preceding_version_uid) or else
    /// version_count = 0`. Returns the `preceding_version_uid` to store —
    /// Void for the first commit (see the PORT NOTE on
    /// [`VersionedObject::commit_original_version`]).
    fn checked_preceding_version_uid(
        &self,
        a_preceding_version_id: ObjectVersionId,
    ) -> Result<Option<ObjectVersionId>, VersionedObjectCommitError> {
        if self.versions.is_empty() {
            Ok(None)
        } else if self.has_version_id(&a_preceding_version_id) {
            Ok(Some(a_preceding_version_id))
        } else {
            Err(VersionedObjectCommitError::PrecedingVersionNotFound {
                preceding_version_id: a_preceding_version_id.value().to_string(),
            })
        }
    }
}

// Invariants (spec `Invariants` table):
//   Version_count_valid: version_count >= 0
//   All_version_ids_valid: all_version_ids.count = version_count
//   All_versions_valid: all_versions.count = version_count
//   Latest_version_valid: version_count > 0 implies latest_version /= Void
//     (all four are structurally true for the Vec-backed store and its
//     directly-derived functions; documented, not separately enforced.)
//   Uid_validity: extension.is_empty
//     Implemented as the working method `is_uid_valid()` per ADR-003
//     decision 8; Validate-framework wiring remains a P11 deliverable.

/// Shared `#[cfg(test)]` fixture builders for the `common.change_control`
/// test modules (this file, `version.rs`, `original_version.rs`,
/// `imported_version.rs`). Housed here rather than in a separate
/// `test_support.rs` to keep the module list of `mod.rs` untouched.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::common::generic::audit_details::AuditDetailsData;
    use crate::common::generic::party_proxy::PartyProxy;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_text::DvTextData;
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;
    use openehr_base::identification::uid_based_id::UidBasedIdData;

    /// A `DV_CODED_TEXT` with an `openehr` defining code.
    pub(crate) fn coded(code: &str, rubric: &str) -> DvCodedText {
        DvCodedText {
            type_tag: TypeTag::new(),
            text: DvTextData {
                value: rubric.to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
            defining_code: CodePhrase {
                type_tag: TypeTag::new(),
                terminology_id: TerminologyId {
                    type_tag: TypeTag::new(),
                    object_id: ObjectIdData {
                        value: "openehr".to_string(),
                    },
                },
                code_string: code.to_string(),
                preferred_term: None,
            },
        }
    }

    /// A `DV_DATE_TIME` from an ISO 8601 string.
    pub(crate) fn date_time(value: &str) -> DvDateTime {
        serde_json::from_value(serde_json::json!({ "value": value }))
            .expect("test DV_DATE_TIME literal deserializes")
    }

    /// A `PARTY_SELF` committer.
    pub(crate) fn party_self() -> PartyProxy {
        serde_json::from_value(serde_json::json!({ "_type": "PARTY_SELF" }))
            .expect("test PARTY_SELF literal deserializes")
    }

    /// An `AUDIT_DETAILS` with change type 249 ("creation") committed at
    /// `time`.
    pub(crate) fn audit(time: &str) -> AuditDetails {
        AuditDetails {
            type_tag: TypeTag::new(),
            data: AuditDetailsData {
                system_id: "test.ehr.system".to_string(),
                time_committed: date_time(time),
                change_type: coded("249", "creation"),
                description: None,
                committer: party_self(),
            },
        }
    }

    /// An `OBJECT_VERSION_ID` from its raw
    /// `object_id::creating_system_id::version_tree_id` string.
    pub(crate) fn ovid(value: &str) -> ObjectVersionId {
        ObjectVersionId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: value.to_string(),
            },
        }
    }

    /// A `HIER_OBJECT_ID` from its raw string.
    pub(crate) fn hier(value: &str) -> HierObjectId {
        HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: value.to_string(),
            },
        }
    }

    /// An `OBJECT_REF` pointing at `id` in the local namespace.
    pub(crate) fn object_ref(r#type: &str, id: &str) -> ObjectRef {
        ObjectRef {
            type_tag: TypeTag::new(),
            namespace: "local".to_string(),
            r#type: r#type.to_string(),
            id: hier(id).into(),
        }
    }

    /// A standalone `ORIGINAL_VERSION<String>` (for `version.rs` /
    /// `original_version.rs` tests that need a version outside a
    /// container).
    pub(crate) fn original_version(
        uid: &str,
        preceding: Option<&str>,
        time: &str,
        lifecycle_code: &str,
    ) -> OriginalVersion<String> {
        OriginalVersion {
            type_tag: TypeTag::new(),
            version: VersionData {
                contribution: object_ref("CONTRIBUTION", "b0e6c17c-7b78-4de1-a1a4-fd2988c8d3a1"),
                signature: None,
                commit_audit: audit(time),
            },
            uid: ovid(uid),
            preceding_version_uid: preceding.map(ovid),
            other_input_version_uids: None,
            lifecycle_state: coded(lifecycle_code, "complete"),
            attestations: None,
            data: Some("data".to_string()),
        }
    }

    /// An empty `VERSIONED_OBJECT<String>` container.
    pub(crate) fn container(uid: &str) -> VersionedObject<String> {
        VersionedObject {
            type_tag: TypeTag::new(),
            uid: hier(uid),
            owner_id: object_ref("EHR", "b5a56f4c-4574-4759-9bd5-b09be2f0e532"),
            time_created: date_time("2020-01-01T00:00:00"),
            versions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::common::generic::attestation::Attestation;
    use crate::data_types::text::dv_text::DvText;

    const CONTAINER_UID: &str = "87284370-2d4b-4e3d-a3f3-f303d2f4f34b";

    fn vid(tree: &str) -> String {
        format!("{CONTAINER_UID}::test.sys::{tree}")
    }

    /// An attestation wrapping the standard test audit.
    fn attestation(time: &str) -> Attestation {
        Attestation {
            type_tag: TypeTag::new(),
            audit_details: audit(time).data,
            attested_view: None,
            proof: None,
            items: None,
            reason: DvText::Coded(coded("240", "signed")),
            is_pending: false,
        }
    }

    /// Commits versions 1 and 2 (trunk) into a fresh container.
    fn committed_container() -> VersionedObject<String> {
        let mut container = container(CONTAINER_UID);
        container
            .commit_original_version(
                object_ref("CONTRIBUTION", "10c19ae2-64f2-43dc-a710-bd52012bb0a3"),
                ovid(&vid("1")),
                ovid(&vid("0")), // discarded: first commit
                audit("2020-01-01T00:00:01"),
                coded("532", "complete"),
                "v1 data".to_string(),
                String::new(),
            )
            .expect("first commit succeeds");
        container
            .commit_original_version(
                object_ref("CONTRIBUTION", "2b1a3cbe-33b2-4be5-b06c-2c6c443c04ae"),
                ovid(&vid("2")),
                ovid(&vid("1")),
                audit("2020-01-01T00:00:03"),
                coded("532", "complete"),
                "v2 data".to_string(),
                String::new(),
            )
            .expect("second commit succeeds");
        container
    }

    #[test]
    fn commit_sequence_produces_correct_ids_and_counts() {
        let container = committed_container();

        assert_eq!(container.version_count(), 2);
        assert_eq!(container.all_versions().len(), 2);
        let ids: Vec<String> = container
            .all_version_ids()
            .iter()
            .map(|id| id.value().to_string())
            .collect();
        assert_eq!(ids, vec![vid("1"), vid("2")]);

        assert!(container.has_version_id(&ovid(&vid("1"))));
        assert!(!container.has_version_id(&ovid(&vid("9"))));
        assert!(container.is_original_version(&ovid(&vid("1"))));
        assert!(container.version_with_id(&ovid(&vid("9"))).is_none());

        // First commit stored a Void preceding_version_uid despite the
        // dummy parameter; second stored Some(v1).
        let v1 = container
            .version_with_id(&ovid(&vid("1")))
            .expect("v1 exists");
        assert!(v1.preceding_version_uid().is_none());
        let v2 = container
            .version_with_id(&ovid(&vid("2")))
            .expect("v2 exists");
        assert_eq!(
            v2.preceding_version_uid().map(ObjectVersionId::value),
            Some(vid("1").as_str())
        );

        assert!(container.is_uid_valid());
    }

    #[test]
    fn commit_rejects_an_unknown_preceding_version() {
        let mut container = committed_container();
        let err = container
            .commit_original_version(
                object_ref("CONTRIBUTION", "5e2b1c6a-8c9d-4a3b-9f00-15b0c26e9a01"),
                ovid(&vid("3")),
                ovid(&vid("42")),
                audit("2020-01-01T00:00:05"),
                coded("532", "complete"),
                "v3 data".to_string(),
                String::new(),
            )
            .expect_err("unknown preceding id must be rejected");
        assert_eq!(
            err,
            VersionedObjectCommitError::PrecedingVersionNotFound {
                preceding_version_id: vid("42"),
            }
        );
        assert_eq!(container.version_count(), 2);
    }

    #[test]
    fn latest_version_and_latest_trunk_version_diverge_on_branches() {
        let mut container = committed_container();
        container
            .commit_original_version(
                object_ref("CONTRIBUTION", "6f1d2e3c-4b5a-6978-8a9b-0c1d2e3f4a5b"),
                ovid(&vid("2.1.1")),
                ovid(&vid("2")),
                audit("2020-01-01T00:00:05"),
                coded("553", "incomplete"),
                "branch data".to_string(),
                String::new(),
            )
            .expect("branch commit succeeds");

        let latest = container.latest_version().expect("non-empty container");
        assert_eq!(latest.uid().value(), vid("2.1.1"));
        assert!(VersionApi::<String>::is_branch(latest));

        let trunk = container
            .latest_trunk_version()
            .expect("trunk version exists");
        assert_eq!(trunk.uid().value(), vid("2"));

        // Post: Result = latest_trunk_version.lifecycle_state.
        assert_eq!(
            container
                .trunk_lifecycle_state()
                .map(|state| state.defining_code.code_string.as_str()),
            Some("532")
        );
    }

    #[test]
    fn version_at_time_boundaries() {
        // Commit times: v1 @ :01, v2 @ :03.
        let container = committed_container();

        // Before the first commit: nothing existed.
        assert!(!container.has_version_at_time(&date_time("2020-01-01T00:00:00")));
        assert!(
            container
                .version_at_time(&date_time("2020-01-01T00:00:00"))
                .is_none()
        );

        // Exactly at the first commit time (inclusive boundary).
        let at_v1 = container
            .version_at_time(&date_time("2020-01-01T00:00:01"))
            .expect("v1 is current at its own commit time");
        assert_eq!(at_v1.uid().value(), vid("1"));

        // Between the two commits: v1 is still current.
        let between = container
            .version_at_time(&date_time("2020-01-01T00:00:02"))
            .expect("v1 is current between commits");
        assert_eq!(between.uid().value(), vid("1"));

        // At and after the second commit: v2.
        for probe in ["2020-01-01T00:00:03", "2021-06-30T12:00:00"] {
            let current = container
                .version_at_time(&date_time(probe))
                .expect("v2 is current");
            assert_eq!(current.uid().value(), vid("2"));
            assert!(container.has_version_at_time(&date_time(probe)));
        }
    }

    #[test]
    fn commit_original_merged_version_stores_other_inputs_and_rejects_empty() {
        let mut container = committed_container();

        let err = container
            .commit_original_merged_version(
                object_ref("CONTRIBUTION", "7a8b9c0d-1e2f-4a3b-8c5d-6e7f8a9b0c1d"),
                ovid(&vid("3")),
                ovid(&vid("2")),
                audit("2020-01-01T00:00:07"),
                coded("532", "complete"),
                "merged data".to_string(),
                Vec::new(),
                String::new(),
            )
            .expect_err("empty other-input list violates Other_input_version_uids_valid");
        assert_eq!(err, VersionedObjectCommitError::EmptyOtherInputUids);

        container
            .commit_original_merged_version(
                object_ref("CONTRIBUTION", "7a8b9c0d-1e2f-4a3b-8c5d-6e7f8a9b0c1d"),
                ovid(&vid("3")),
                ovid(&vid("2")),
                audit("2020-01-01T00:00:07"),
                coded("532", "complete"),
                "merged data".to_string(),
                vec![ovid(&vid("1"))],
                String::new(),
            )
            .expect("merged commit succeeds");

        let merged = container
            .version_with_id(&ovid(&vid("3")))
            .expect("merged version exists");
        match merged {
            Version::Original(original) => {
                assert!(original.is_merged());
                assert_eq!(
                    original
                        .other_input_version_uids
                        .as_ref()
                        .map(Vec::as_slice)
                        .map(|uids| uids.len()),
                    Some(1)
                );
            }
            Version::Imported(_) => panic!("merged commit must produce an ORIGINAL_VERSION"),
        }
    }

    #[test]
    fn commit_attestation_targets_only_original_versions() {
        let mut container = committed_container();

        // Import a foreign original version, so an IMPORTED_VERSION exists.
        let foreign = original_version(
            "0d9358f8-73a4-4225-9df8-cf85a1a45ac5::remote.sys::1",
            None,
            "2020-01-01T00:00:04",
            "532",
        );
        container.commit_imported_version(
            object_ref("CONTRIBUTION", "8c0d1e2f-3a4b-4c5d-8e7f-9a0b1c2d3e4f"),
            audit("2020-01-01T00:00:05"),
            foreign,
        );
        assert_eq!(container.version_count(), 3);
        let imported_id = ovid("0d9358f8-73a4-4225-9df8-cf85a1a45ac5::remote.sys::1");
        assert!(container.has_version_id(&imported_id));
        assert!(!container.is_original_version(&imported_id));

        // Attestation on an original version succeeds.
        container
            .commit_attestation(
                attestation("2020-01-01T00:00:06"),
                ovid(&vid("2")),
                String::new(),
            )
            .expect("attestation on an original version succeeds");

        // On a missing version: VersionNotFound.
        assert_eq!(
            container.commit_attestation(
                attestation("2020-01-01T00:00:06"),
                ovid(&vid("9")),
                String::new(),
            ),
            Err(VersionedObjectCommitError::VersionNotFound {
                version_id: vid("9"),
            })
        );

        // On an imported version: NotAnOriginalVersion.
        assert_eq!(
            container.commit_attestation(
                attestation("2020-01-01T00:00:06"),
                imported_id.clone(),
                String::new(),
            ),
            Err(VersionedObjectCommitError::NotAnOriginalVersion {
                version_id: imported_id.value().to_string(),
            })
        );
    }

    #[test]
    fn revision_history_is_most_recent_last_and_carries_attestations() {
        let mut container = committed_container();
        container
            .commit_attestation(
                attestation("2020-01-01T00:00:06"),
                ovid(&vid("2")),
                String::new(),
            )
            .expect("attestation succeeds");

        let history = container.revision_history();
        assert_eq!(history.items.len(), 2);

        // Most-recent-LAST ordering (settled ROSETTA reading).
        assert_eq!(history.items[0].version_id.value(), vid("1"));
        assert_eq!(history.items[1].version_id.value(), vid("2"));
        // most_recent_version(): String — Post: Result.is_equal(items.last.version_id.value).
        assert_eq!(history.most_recent_version(), vid("2"));
        // most_recent_version_time_committed(): String — Post:
        // Result.is_equal(items.last.audits.first.time_committed.value); v2's
        // first audit committed at :03.
        assert_eq!(
            history.most_recent_version_time_committed(),
            "2020-01-01T00:00:03"
        );

        // v1: just its commit audit; v2: commit audit + up-cast attestation.
        assert_eq!(history.items[0].audits.len(), 1);
        assert_eq!(history.items[1].audits.len(), 2);
        assert_eq!(
            history.items[1].audits[1].data.time_committed.value,
            "2020-01-01T00:00:06"
        );
    }
}
