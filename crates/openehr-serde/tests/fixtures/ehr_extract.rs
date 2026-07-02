//! Fixtures for rm.ehr_extract classes.

use openehr_base::identification::uid_based_id::UidBasedId;
use openehr_foundation::serde_support::TypeTag;
use openehr_rm::common::archetyped::locatable::LocatableData;
use openehr_rm::integration::ehr_extract::{
    AddressedMessage, Extract, ExtractActionRequest, ExtractChapter, ExtractEntityChapter,
    ExtractEntityManifest, ExtractFolder, ExtractManifest, ExtractParticipation, ExtractRequest,
    ExtractSpec, ExtractUpdateSpec, ExtractVersionSpec, GenericContentItem, Message,
    MessageContent, OpenehrContentItem, SyncExtract, SyncExtractRequest, SyncExtractSpec,
    XContribution, XVersionedComposition, XVersionedEhrAccess, XVersionedEhrStatus,
    XVersionedFolder, XVersionedObject, XVersionedObjectData, XVersionedParty,
};

use super::helpers::{audit, code_phrase, coded, date_time, hier, locatable, object_ref, text};
use super::{Vector, vector};

fn locatable_with_uid(name: &str, node_id: &str, uid: &str) -> LocatableData {
    let mut value = locatable(name, node_id);
    value.uid = Some(UidBasedId::HierObjectId(hier(uid)));
    value
}

fn manifest() -> ExtractManifest {
    ExtractManifest {
        type_tag: TypeTag::new(),
        entities: None,
    }
}

fn extract_spec() -> ExtractSpec {
    ExtractSpec {
        type_tag: TypeTag::new(),
        extract_type: coded("full", "openehr", "full"),
        include_multimedia: false,
        priority: 0,
        link_depth: 1,
        criteria: None,
        manifest: manifest(),
        version_spec: None,
        other_details: None,
    }
}

fn sync_spec() -> SyncExtractSpec {
    SyncExtractSpec {
        type_tag: TypeTag::new(),
        includes_versions: true,
        contribution_list: None,
        contributions_since: None,
        all_contributions: None,
    }
}

fn sync_extract() -> SyncExtract {
    SyncExtract {
        type_tag: TypeTag::new(),
        locatable: locatable("sync extract", "openEHR-EHR-SYNC_EXTRACT.sample.v1"),
        specification: sync_spec(),
        items: None,
    }
}

fn sync_extract_request() -> SyncExtractRequest {
    SyncExtractRequest {
        type_tag: TypeTag::new(),
        locatable: locatable(
            "sync extract request",
            "openEHR-EHR-SYNC_EXTRACT_REQUEST.sample.v1",
        ),
        specification: sync_spec(),
    }
}

fn x_versioned_data(uid: &str) -> XVersionedObjectData {
    XVersionedObjectData {
        uid: hier(uid),
        owner_id: object_ref("local", "EHR", "c7a4f272-bf28-4145-9da5-a2e7e6c7d9fe"),
        time_created: date_time("2026-07-02T10:00:00Z"),
        total_version_count: 1,
        extract_version_count: 1,
        revision_history: None,
        versions: None,
    }
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        vector(
            "ADDRESSED_MESSAGE",
            &AddressedMessage {
                type_tag: TypeTag::new(),
                sender: "ehrbase.example.org".to_string(),
                sender_reference: "sender-1".to_string(),
                addressees: None,
                urgency: None,
                message: None,
            },
        ),
        vector(
            "MESSAGE",
            &Message {
                type_tag: TypeTag::new(),
                author: super::helpers::committer(),
                audit: audit(),
                content: MessageContent::SyncExtractRequest(sync_extract_request()),
                signature: None,
            },
        ),
        vector("SYNC_EXTRACT_SPEC", &sync_spec()),
        vector("SYNC_EXTRACT", &sync_extract()),
        vector("SYNC_EXTRACT_REQUEST", &sync_extract_request()),
        vector("EXTRACT_MANIFEST", &manifest()),
        vector(
            "EXTRACT_ENTITY_MANIFEST",
            &ExtractEntityManifest {
                type_tag: TypeTag::new(),
                extract_id_key: "entity-1".to_string(),
                ehr_id: None,
                subject_id: None,
                other_ids: None,
                item_list: None,
            },
        ),
        vector(
            "EXTRACT_VERSION_SPEC",
            &ExtractVersionSpec {
                type_tag: TypeTag::new(),
                include_all_versions: true,
                commit_time_interval: None,
                include_revision_history: false,
                include_data: true,
            },
        ),
        vector(
            "EXTRACT_UPDATE_SPEC",
            &ExtractUpdateSpec {
                type_tag: TypeTag::new(),
                persist_in_server: false,
                trigger_events: None,
                repeat_period: None,
                update_method: code_phrase("openehr", "sync"),
            },
        ),
        vector("EXTRACT_SPEC", &extract_spec()),
        vector(
            "EXTRACT_PARTICIPATION",
            &ExtractParticipation {
                type_tag: TypeTag::new(),
                performer: "clinician-1".to_string(),
                function: text("author"),
                mode: None,
                time: None,
            },
        ),
        vector(
            "EXTRACT",
            &Extract {
                type_tag: TypeTag::new(),
                locatable: locatable("extract", "openEHR-EHR-EXTRACT.sample.v1"),
                request_id: None,
                time_created: date_time("2026-07-02T10:00:00Z"),
                system_id: hier("2a1a9172-e6ab-44b8-a18f-40e3417a950a"),
                sequence_nr: 1,
                specification: None,
                chapters: None,
                participations: None,
            },
        ),
        vector(
            "EXTRACT_REQUEST",
            &ExtractRequest {
                type_tag: TypeTag::new(),
                locatable: locatable_with_uid(
                    "extract request",
                    "openEHR-EHR-EXTRACT_REQUEST.sample.v1",
                    "e820e09c-c4b3-4200-80bd-c012048cfac2",
                ),
                extract_spec: extract_spec(),
                update_spec: None,
            },
        ),
        vector(
            "EXTRACT_ACTION_REQUEST",
            &ExtractActionRequest {
                type_tag: TypeTag::new(),
                locatable: locatable_with_uid(
                    "extract action",
                    "openEHR-EHR-EXTRACT_ACTION_REQUEST.sample.v1",
                    "28f3f09b-c810-4aec-b778-3b9d83918d01",
                ),
                request_id: object_ref("local", "EXTRACT_REQUEST", "request-1"),
                action: coded("cancel", "openehr", "cancel"),
            },
        ),
        vector(
            "EXTRACT_CHAPTER",
            &ExtractChapter {
                type_tag: TypeTag::new(),
                locatable: locatable("chapter", "openEHR-EHR-EXTRACT_CHAPTER.sample.v1"),
                items: None,
            },
        ),
        vector(
            "EXTRACT_ENTITY_CHAPTER",
            &ExtractEntityChapter {
                type_tag: TypeTag::new(),
                locatable: locatable(
                    "entity chapter",
                    "openEHR-EHR-EXTRACT_ENTITY_CHAPTER.sample.v1",
                ),
                items: None,
                extract_id_key: "entity-1".to_string(),
            },
        ),
        vector(
            "EXTRACT_FOLDER",
            &ExtractFolder {
                type_tag: TypeTag::new(),
                locatable: locatable("folder", "openEHR-EHR-EXTRACT_FOLDER.sample.v1"),
                items: None,
            },
        ),
        vector(
            "GENERIC_CONTENT_ITEM",
            &GenericContentItem {
                type_tag: TypeTag::new(),
                locatable: locatable(
                    "generic content",
                    "openEHR-EHR-GENERIC_CONTENT_ITEM.sample.v1",
                ),
                is_primary: true,
                is_changed: None,
                is_masked: None,
                item: None,
                item_type: None,
                item_type_version: None,
                author: None,
                creation_time: None,
                authoriser: None,
                authorisation_time: None,
                item_status: None,
                version_id: None,
                version_set_id: None,
                system_id: None,
                other_details: None,
            },
        ),
        vector(
            "OPENEHR_CONTENT_ITEM",
            &OpenehrContentItem {
                type_tag: TypeTag::new(),
                locatable: locatable(
                    "openehr content",
                    "openEHR-EHR-OPENEHR_CONTENT_ITEM.sample.v1",
                ),
                is_primary: true,
                is_changed: None,
                is_masked: None,
                item: None,
            },
        ),
        vector(
            "X_CONTRIBUTION",
            &XContribution {
                type_tag: TypeTag::new(),
                uid: hier("bc996ae4-14e5-4586-b646-c5313e46de55"),
                audit: audit(),
                versions: None,
            },
        ),
        vector(
            "X_VERSIONED_OBJECT",
            &XVersionedObject {
                type_tag: TypeTag::new(),
                data: x_versioned_data("4ef9d72e-5abb-4aa1-983c-c47c0e943ccf"),
            },
        ),
        vector(
            "X_VERSIONED_COMPOSITION",
            &XVersionedComposition {
                type_tag: TypeTag::new(),
                data: x_versioned_data("7a8f2d05-69a4-4ce4-8d58-69d330b6f605"),
            },
        ),
        vector(
            "X_VERSIONED_EHR_ACCESS",
            &XVersionedEhrAccess {
                type_tag: TypeTag::new(),
                data: x_versioned_data("ab202b42-90d8-4e06-bbfe-4829db30d5a9"),
            },
        ),
        vector(
            "X_VERSIONED_EHR_STATUS",
            &XVersionedEhrStatus {
                type_tag: TypeTag::new(),
                data: x_versioned_data("e83f586b-6889-42b2-85ac-e9d89607dff9"),
            },
        ),
        vector(
            "X_VERSIONED_FOLDER",
            &XVersionedFolder {
                type_tag: TypeTag::new(),
                data: x_versioned_data("7b7eac1b-5a2e-4a09-8745-39422db558bc"),
            },
        ),
        vector(
            "X_VERSIONED_PARTY",
            &XVersionedParty {
                type_tag: TypeTag::new(),
                data: x_versioned_data("61cd2e5f-5994-458a-a10f-5d7d364bf43c"),
            },
        ),
    ]
}
