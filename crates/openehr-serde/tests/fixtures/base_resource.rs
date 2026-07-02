//! Fixtures for BASE resource classes.

use std::collections::HashMap;

use openehr_base::resource::resource_description::ResourceDescription;
use openehr_base::resource::resource_description_item::ResourceDescriptionItem;
use openehr_base::resource::translation_details::TranslationDetails;
use openehr_foundation::primitive_types::string::OpenEhrString;
use openehr_foundation::primitive_types::uri::Uri;
use openehr_foundation::serde_support::TypeTag;
use openehr_foundation::terminology_types::terminology_code::TerminologyCode;

use super::{Vector, vector};

// The pinned schema marks TERMINOLOGY_CODE.uri as required, so fixtures
// always populate it.
fn lang(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: OpenEhrString("ISO_639-1".to_string()),
        terminology_version: None,
        code_string: OpenEhrString(code.to_string()),
        uri: Some(Uri::new_unchecked(format!(
            "http://example.org/terminology/{code}"
        ))),
    }
}

fn author() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("name".to_string(), "T. Author".to_string());
    m
}

pub fn fixtures() -> Vec<Vector> {
    vec![
        {
            // schema_check off: the pinned 2021 schema predates BASE 1.2.0 (models lifecycle_state/language as string or old TERMINOLOGY_CODE-with-object-URI, requires the parent_resource back-reference); round-trip + golden vector still pin the BASE 1.2.0 shape.
            let mut v = vector(
                "RESOURCE_DESCRIPTION",
                &ResourceDescription {
                    type_tag: TypeTag::new(),
                    original_author: author(),
                    original_namespace: None,
                    original_publisher: None,
                    other_contributors: None,
                    lifecycle_state: lang("published"),
                    parent_resource: std::sync::Weak::new(),
                    custodian_namespace: None,
                    custodian_organisation: None,
                    copyright: None,
                    licence: None,
                    ip_acknowledgements: None,
                    references: None,
                    resource_package_uri: None,
                    conversion_details: None,
                    other_details: None,
                    details: None,
                },
            );
            v.schema_check = false;
            v
        },
        {
            // schema_check off: the pinned 2021 schema predates BASE 1.2.0 (models lifecycle_state/language as string or old TERMINOLOGY_CODE-with-object-URI, requires the parent_resource back-reference); round-trip + golden vector still pin the BASE 1.2.0 shape.
            let mut v = vector(
                "RESOURCE_DESCRIPTION_ITEM",
                &ResourceDescriptionItem {
                    type_tag: TypeTag::new(),
                    language: lang("en"),
                    purpose: "Recording an encounter".to_string(),
                    keywords: None,
                    use_: None,
                    misuse: None,
                    original_resource_uri: None,
                    other_details: None,
                },
            );
            v.schema_check = false;
            v
        },
        {
            // schema_check off: the pinned 2021 schema predates BASE 1.2.0 (models lifecycle_state/language as string or old TERMINOLOGY_CODE-with-object-URI, requires the parent_resource back-reference); round-trip + golden vector still pin the BASE 1.2.0 shape.
            let mut v = vector(
                "TRANSLATION_DETAILS",
                &TranslationDetails {
                    type_tag: TypeTag::new(),
                    language: lang("nl"),
                    author: author(),
                    accreditation: None,
                    other_details: None,
                    version_last_translated: None,
                    other_contributors: None,
                },
            );
            v.schema_check = false;
            v
        },
    ]
}
