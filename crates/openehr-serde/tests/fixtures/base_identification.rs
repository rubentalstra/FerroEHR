//! Fixtures for BASE identification classes.

use openehr_base::identification::archetype_id::ArchetypeId;
use openehr_base::identification::generic_id::GenericId;
use openehr_base::identification::internet_id::InternetId;
use openehr_base::identification::iso_oid::IsoOid;
use openehr_base::identification::locatable_ref::LocatableRef;
use openehr_base::identification::object_id::ObjectIdData;
use openehr_base::identification::template_id::TemplateId;
use openehr_base::identification::terminology_id::TerminologyId;
use openehr_base::identification::uid::UidData;
use openehr_base::identification::uid_based_id::UidBasedId;
use openehr_base::identification::uuid::Uuid;
use openehr_base::identification::version_tree_id::VersionTreeId;
use openehr_foundation::serde_support::TypeTag;

use super::helpers::{hier, object_ref, object_version_id, party_ref};
use super::{Vector, vector};

pub fn fixtures() -> Vec<Vector> {
    vec![
        vector(
            "HIER_OBJECT_ID",
            &hier("8849182c-82ad-4088-a07f-48ead4180515"),
        ),
        vector(
            "OBJECT_VERSION_ID",
            &object_version_id("8849182c-82ad-4088-a07f-48ead4180515::ehrbase.example.org::1"),
        ),
        vector(
            "VERSION_TREE_ID",
            &VersionTreeId {
                type_tag: TypeTag::new(),
                value: "1.2.1".to_string(),
            },
        ),
        vector(
            "ARCHETYPE_ID",
            &ArchetypeId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: "openEHR-EHR-COMPOSITION.encounter.v1".to_string(),
                },
            },
        ),
        vector(
            "TEMPLATE_ID",
            &TemplateId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: "Vital Signs Encounter".to_string(),
                },
            },
        ),
        vector(
            "TERMINOLOGY_ID",
            &TerminologyId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: "openehr".to_string(),
                },
            },
        ),
        vector(
            "GENERIC_ID",
            &GenericId {
                type_tag: TypeTag::new(),
                object_id: ObjectIdData {
                    value: "ab1234".to_string(),
                },
                scheme: "local".to_string(),
            },
        ),
        vector(
            "ISO_OID",
            &IsoOid {
                type_tag: TypeTag::new(),
                uid: UidData {
                    value: "1.2.840.10008".to_string(),
                },
            },
        ),
        vector(
            "UUID",
            &Uuid {
                type_tag: TypeTag::new(),
                uid: UidData {
                    value: "8849182c-82ad-4088-a07f-48ead4180515".to_string(),
                },
            },
        ),
        vector(
            "INTERNET_ID",
            &InternetId {
                type_tag: TypeTag::new(),
                uid: UidData {
                    value: "ehrbase.example.org".to_string(),
                },
            },
        ),
        vector(
            "OBJECT_REF",
            &object_ref("local", "EHR", "7d44b88c-4199-4bad-97dc-d78268e01398"),
        ),
        vector(
            "PARTY_REF",
            &party_ref(
                "demographic",
                "PERSON",
                "0e04d3af-0f8a-4be3-90a0-4a34fc94b21e",
            ),
        ),
        vector(
            "LOCATABLE_REF",
            &LocatableRef {
                type_tag: TypeTag::new(),
                namespace: "local".to_string(),
                r#type: "COMPOSITION".to_string(),
                id: UidBasedId::ObjectVersionId(object_version_id(
                    "8849182c-82ad-4088-a07f-48ead4180515::ehrbase.example.org::1",
                )),
                path: None,
            },
        ),
    ]
}
