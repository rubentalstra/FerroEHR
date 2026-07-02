//! `rm.ehr_extract` — extract and synchronisation message classes.
//!
//! openEHR package: `rm.ehr_extract`.
//!
//! The main RM transcription deferred this package behind the
//! `ehr-extract` feature. Phase 04's full ITS-JSON coverage requires the
//! concrete wire classes to exist, so this module captures the schema
//! shape faithfully while leaving service behaviour to later phases.
use crate::common::archetyped::locatable::LocatableData;
use crate::common::change_control::original_version::OriginalVersion;
use crate::common::generic::audit_details::AuditDetails;
use crate::common::generic::party_proxy::PartyProxy;
use crate::common::generic::revision_history::RevisionHistory;
use crate::data_structures::item_structure::item_structure::ItemStructure;
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::encapsulated::dv_parsable::DvParsable;
use crate::data_types::quantity::dv_interval::DvInterval;
use crate::data_types::text::code_phrase::CodePhrase;
use crate::data_types::text::dv_coded_text::DvCodedText;
use crate::data_types::text::dv_text::DvText;
use openehr_base::identification::hier_object_id::HierObjectId;
use openehr_base::identification::object_ref::ObjectRef;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

macro_rules! impl_type_name {
    ($ty:ty, $name:literal) => {
        impl TypeName for $ty {
            const NAME: &'static str = $name;
        }
    };
}

/// `ADDRESSED_MESSAGE` — transport envelope around a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddressedMessage {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub sender: String,
    pub sender_reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addressees: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
}

impl_type_name!(AddressedMessage, "ADDRESSED_MESSAGE");

/// `MESSAGE` — author, audit, and synchronisation content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub author: PartyProxy,
    pub audit: AuditDetails,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl_type_name!(Message, "MESSAGE");

/// Closed content set for `MESSAGE.content`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    SyncExtract(SyncExtract),
    SyncExtractRequest(SyncExtractRequest),
}

/// `SYNC_EXTRACT_SPEC` — criteria for synchronous extract contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncExtractSpec {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub includes_versions: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contribution_list: Option<Vec<HierObjectId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributions_since: Option<DvDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_contributions: Option<bool>,
}

impl_type_name!(SyncExtractSpec, "SYNC_EXTRACT_SPEC");

/// `SYNC_EXTRACT` — synchronisation extract payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncExtract {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    pub specification: SyncExtractSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<XContribution>>,
}

impl_type_name!(SyncExtract, "SYNC_EXTRACT");

/// `SYNC_EXTRACT_REQUEST` — request for a synchronisation extract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncExtractRequest {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    pub specification: SyncExtractSpec,
}

impl_type_name!(SyncExtractRequest, "SYNC_EXTRACT_REQUEST");

/// `EXTRACT_MANIFEST` — manifest of extract entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractManifest {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<ExtractEntityManifest>>,
}

impl_type_name!(ExtractManifest, "EXTRACT_MANIFEST");

/// `EXTRACT_ENTITY_MANIFEST` — identifiers for one extract entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractEntityManifest {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub extract_id_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ehr_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_list: Option<Vec<ObjectRef>>,
}

impl_type_name!(ExtractEntityManifest, "EXTRACT_ENTITY_MANIFEST");

/// `EXTRACT_VERSION_SPEC` — version-selection flags for an extract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractVersionSpec {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub include_all_versions: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_time_interval: Option<DvInterval<DvDateTime>>,
    pub include_revision_history: bool,
    pub include_data: bool,
}

impl_type_name!(ExtractVersionSpec, "EXTRACT_VERSION_SPEC");

/// `EXTRACT_UPDATE_SPEC` — update method and trigger settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractUpdateSpec {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub persist_in_server: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_events: Option<Vec<DvCodedText>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_period: Option<DvDuration>,
    pub update_method: CodePhrase,
}

impl_type_name!(ExtractUpdateSpec, "EXTRACT_UPDATE_SPEC");

/// `EXTRACT_SPEC` — extract request criteria.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractSpec {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub extract_type: DvCodedText,
    pub include_multimedia: bool,
    pub priority: i32,
    pub link_depth: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Vec<DvParsable>>,
    pub manifest: ExtractManifest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_spec: Option<ExtractVersionSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_details: Option<ItemStructure>,
}

impl_type_name!(ExtractSpec, "EXTRACT_SPEC");

/// `EXTRACT_PARTICIPATION` — participation metadata in an extract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractParticipation {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub performer: String,
    pub function: DvText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<DvCodedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<DvInterval<DvDateTime>>,
}

impl_type_name!(ExtractParticipation, "EXTRACT_PARTICIPATION");

/// `EXTRACT` — asynchronous extract response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extract {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<HierObjectId>,
    pub time_created: DvDateTime,
    pub system_id: HierObjectId,
    pub sequence_nr: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specification: Option<ExtractSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapters: Option<Vec<ExtractChapterVariant>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participations: Option<Vec<ExtractParticipation>>,
}

impl_type_name!(Extract, "EXTRACT");

/// `EXTRACT_REQUEST` — locatable request for an extract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractRequest {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    pub extract_spec: ExtractSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_spec: Option<ExtractUpdateSpec>,
}

impl_type_name!(ExtractRequest, "EXTRACT_REQUEST");

/// `EXTRACT_ACTION_REQUEST` — request to perform an extract action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractActionRequest {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    pub request_id: ObjectRef,
    pub action: DvCodedText,
}

impl_type_name!(ExtractActionRequest, "EXTRACT_ACTION_REQUEST");

/// Closed chapter set for `EXTRACT.chapters`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtractChapterVariant {
    Entity(Box<ExtractEntityChapter>),
    Chapter(ExtractChapter),
}

/// `EXTRACT_CHAPTER` — chapter containing extract items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractChapter {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ExtractItem>>,
}

impl_type_name!(ExtractChapter, "EXTRACT_CHAPTER");

/// `EXTRACT_ENTITY_CHAPTER` — chapter for a single extract entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractEntityChapter {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ExtractItem>>,
    pub extract_id_key: String,
}

impl_type_name!(ExtractEntityChapter, "EXTRACT_ENTITY_CHAPTER");

/// Closed item set for extract chapters and folders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtractItem {
    Folder(Box<ExtractFolder>),
    GenericContentItem(Box<GenericContentItem>),
    OpenehrContentItem(Box<OpenehrContentItem>),
}

/// `EXTRACT_FOLDER` — recursive folder inside an extract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractFolder {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<ExtractItem>>,
}

impl_type_name!(ExtractFolder, "EXTRACT_FOLDER");

/// `GENERIC_CONTENT_ITEM` — non-openEHR content in an extract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericContentItem {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    pub is_primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_masked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<ExtractItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<DvCodedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authoriser: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorisation_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_status: Option<DvCodedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_set_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_details: Option<Vec<String>>,
}

impl_type_name!(GenericContentItem, "GENERIC_CONTENT_ITEM");

/// `OPENEHR_CONTENT_ITEM` — openEHR versioned content in an extract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenehrContentItem {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    #[serde(flatten)]
    pub locatable: LocatableData,
    pub is_primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_masked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<XVersionedObjectVariant>,
}

impl_type_name!(OpenehrContentItem, "OPENEHR_CONTENT_ITEM");

/// `X_CONTRIBUTION` — extracted contribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XContribution {
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,
    pub uid: HierObjectId,
    pub audit: AuditDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<Vec<OriginalVersion<DvText>>>,
}

impl_type_name!(XContribution, "X_CONTRIBUTION");

/// Shared state for `X_VERSIONED_*` classes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XVersionedObjectData {
    pub uid: HierObjectId,
    pub owner_id: ObjectRef,
    pub time_created: DvDateTime,
    pub total_version_count: i32,
    pub extract_version_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_history: Option<RevisionHistory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub versions: Option<Vec<OriginalVersion<DvText>>>,
}

macro_rules! x_versioned_type {
    ($ty:ident, $name:literal) => {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct $ty {
            #[serde(rename = "_type", default = "TypeTag::new")]
            pub type_tag: TypeTag<Self>,
            #[serde(flatten)]
            pub data: XVersionedObjectData,
        }

        impl_type_name!($ty, $name);
    };
}

x_versioned_type!(XVersionedObject, "X_VERSIONED_OBJECT");
x_versioned_type!(XVersionedComposition, "X_VERSIONED_COMPOSITION");
x_versioned_type!(XVersionedEhrAccess, "X_VERSIONED_EHR_ACCESS");
x_versioned_type!(XVersionedEhrStatus, "X_VERSIONED_EHR_STATUS");
x_versioned_type!(XVersionedFolder, "X_VERSIONED_FOLDER");
x_versioned_type!(XVersionedParty, "X_VERSIONED_PARTY");

/// Closed set for `OPENEHR_CONTENT_ITEM.item`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum XVersionedObjectVariant {
    Party(XVersionedParty),
    Composition(XVersionedComposition),
    Folder(XVersionedFolder),
    EhrAccess(XVersionedEhrAccess),
    EhrStatus(XVersionedEhrStatus),
    Object(XVersionedObject),
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr_extract package — docs/research/spec-cache/RM-1.1.0/uml_classes/{addressed_message,extract*,message,sync_extract*}.adoc + ITS-JSON pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6
//   source_loc: docs/research/spec-cache/RM-1.1.0/uml_classes/; openehr_rm_1.1.0_all.json#/definitions/{ADDRESSED_MESSAGE,MESSAGE,SYNC_EXTRACT*,EXTRACT*,GENERIC_CONTENT_ITEM,OPENEHR_CONTENT_ITEM,X_*}
//   confidence: medium
//   todos: 0
//   note: feature-gated P4 schema-coverage transcription of rm.ehr_extract concrete classes; behaviour and service orchestration remain deferred, but structural serde uses existing RM/base nested types and boxed enums for recursive extract folders/items.
// ─────────────────────────────────────────────
