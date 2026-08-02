//! Hand-written spec functions of `AUTHORED_RESOURCE`.
//!
//! Spec: `BASE/docs/UML/classes/org.openehr.base.resource.authored_resource.adoc`
//! §Functions, and `BASE/docs/resource/master02-resource_package.adoc`
//! §Natural Languages and Translation ("The languages_available function
//! provides a complete list of languages in the resource").

use crate::resource::authored_resource::AuthoredResource;

impl AuthoredResource {
    /// `AUTHORED_RESOURCE.languages_available`: the total list of languages
    /// available in this resource, derived from `original_language` and
    /// `translations` (class doc §Functions). The original language is always
    /// a member (invariant `Languages_available_valid`:
    /// `languages_available.has (original_language)`), followed by the
    /// translation language codes in their stored (key) order.
    #[must_use]
    pub fn languages_available(&self) -> Vec<&str> {
        let mut out = vec![self.original_language.code_string.as_str()];
        if let Some(translations) = &self.translations {
            out.extend(translations.keys().map(String::as_str));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use crate::foundation_types::terminology::terminology_code::TerminologyCode;
    use crate::resource::authored_resource::AuthoredResource;
    use crate::resource::translation_details::TranslationDetails;

    fn resource(translations: &[&str]) -> AuthoredResource {
        AuthoredResource {
            uid: None,
            original_language: TerminologyCode {
                terminology_id: "ISO_639-1".to_owned(),
                terminology_version: None,
                code_string: "en".to_owned(),
                uri: None,
            },
            description: None,
            is_controlled: None,
            annotations: None,
            translations: if translations.is_empty() {
                None
            } else {
                Some(
                    translations
                        .iter()
                        .map(|lang| {
                            (
                                (*lang).to_owned(),
                                TranslationDetails {
                                    language: TerminologyCode {
                                        terminology_id: "ISO_639-1".to_owned(),
                                        terminology_version: None,
                                        code_string: (*lang).to_owned(),
                                        uri: None,
                                    },
                                    author: std::collections::BTreeMap::new(),
                                    accreditation: None,
                                    other_details: None,
                                    version_last_translated: None,
                                    other_contributors: None,
                                },
                            )
                        })
                        .collect(),
                )
            },
        }
    }

    #[test]
    fn original_language_only() {
        // Languages_available_valid: the original language is always a member.
        assert_eq!(resource(&[]).languages_available(), ["en"]);
    }

    #[test]
    fn original_plus_translations() {
        assert_eq!(
            resource(&["de", "pt"]).languages_available(),
            ["en", "de", "pt"]
        );
    }
}
