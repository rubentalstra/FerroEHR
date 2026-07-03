//! `TERMINOLOGY_SERVICE` — proxy access to the terminology service, backed
//! by the bundled TERM Release-3.0.0 assets.
//!
//! openEHR class: `TERMINOLOGY_SERVICE` (RM 1.1.0,
//! `rm.support.terminology`). Spec `Inherit`:
//! `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS`, `OPENEHR_CODE_SET_IDENTIFIERS` —
//! both are constants-only classes, realised as direct calls per the P1
//! precedent rather than supertraits.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::assets;
use crate::bundle::{CodeSet, Terminology, parse_terminology};
use crate::code_set_access::BundledCodeSetAccess;
use crate::error::TerminologyError;
use crate::openehr_code_set_identifiers::OpenehrCodeSetIdentifiers;
use crate::property_unit_data::{PropertyUnitData, parse_property_unit_data};
use crate::terminology_access::BundledTerminologyAccess;

/// The terminology service over the compiled-in assets: one openEHR
/// terminology bundle per built-in language, the external ISO/IANA code
/// sets, and the property/unit data.
#[derive(Debug)]
pub struct TerminologyService {
    /// Language bundles in [`assets::bundled_language_xml`] order (English
    /// first).
    terminologies: Vec<Terminology>,
    /// The external ISO/IANA code sets (countries, character sets,
    /// languages, media types).
    external: Terminology,
    /// Property/unit data for `DV_QUANTITY` validation (P11).
    property_unit_data: PropertyUnitData,
}

static BUNDLED: LazyLock<Result<TerminologyService, TerminologyError>> =
    LazyLock::new(TerminologyService::from_bundled_assets);

impl TerminologyService {
    /// Parses every compiled-in asset into a service instance.
    ///
    /// # Errors
    ///
    /// [`TerminologyError`] if any vendored asset fails to parse — which the
    /// crate's own tests rule out for shipped assets.
    pub fn from_bundled_assets() -> Result<Self, TerminologyError> {
        let mut terminologies = Vec::new();
        for (language, xml) in assets::bundled_language_xml() {
            let terminology = parse_terminology(xml, "openehr_term.xml")?;
            debug_assert_eq!(terminology.language, language);
            terminologies.push(terminology);
        }
        Ok(Self {
            terminologies,
            external: parse_terminology(
                assets::OPENEHR_EXTERNAL_TERMINOLOGIES,
                "openehr_external_terminologies.xml",
            )?,
            property_unit_data: parse_property_unit_data(assets::PROPERTY_UNIT_DATA)?,
        })
    }

    /// The process-wide service over the bundled assets, parsed on first
    /// use.
    ///
    /// # Errors
    ///
    /// Sticky parse failure of a vendored asset: the same
    /// [`TerminologyError`] is reported on every call (and is caught by this
    /// crate's tests long before shipping).
    pub fn bundled() -> Result<&'static Self, &'static TerminologyError> {
        BUNDLED.as_ref()
    }

    /// Spec `terminology(name): TERMINOLOGY_ACCESS` — an interface to the
    /// terminology named `name` (only `openehr` is bundled), in English.
    /// `None` where the spec precondition `has_terminology(name)` fails
    /// (PORT NOTE: Option instead of a contract violation).
    #[must_use]
    pub fn terminology(&self, name: &str) -> Option<BundledTerminologyAccess<'_>> {
        self.terminology_in_language(name, "en")
    }

    /// PORT NOTE: convenience beyond the spec signature — the same access in
    /// another bundled language (`lang-*` features).
    #[must_use]
    pub fn terminology_in_language(
        &self,
        name: &str,
        language: &str,
    ) -> Option<BundledTerminologyAccess<'_>> {
        self.terminologies
            .iter()
            .find(|t| t.name == name && t.language == language)
            .map(BundledTerminologyAccess::new)
    }

    /// Spec `code_set(name): CODE_SET_ACCESS` — an interface to the code set
    /// identified by the *external* identifier `name` (e.g. `ISO_639-1`).
    /// `None` where the spec precondition `has_code_set(name)` fails.
    #[must_use]
    pub fn code_set(&self, name: &str) -> Option<BundledCodeSetAccess<'_>> {
        self.find_code_set(|cs| cs.external_id == name)
            .map(BundledCodeSetAccess::new)
    }

    /// Spec `code_set_for_id(id): CODE_SET_ACCESS` — an interface to the
    /// code set identified *internally in openEHR* by `id`, one of
    /// [`OpenehrCodeSetIdentifiers`]'s space-form values (e.g. `languages`).
    /// `None` where the spec precondition `valid_code_set_id(id)` fails or
    /// no bundle carries the set.
    #[must_use]
    pub fn code_set_for_id(&self, id: &str) -> Option<BundledCodeSetAccess<'_>> {
        if !OpenehrCodeSetIdentifiers::valid_code_set_id(id) {
            return None;
        }
        self.find_code_set(|cs| cs.name == id)
            .map(BundledCodeSetAccess::new)
    }

    /// Spec `has_terminology(name): Boolean`.
    #[must_use]
    pub fn has_terminology(&self, name: &str) -> bool {
        self.terminologies.iter().any(|t| t.name == name)
    }

    /// Spec `has_code_set(name): Boolean` — true if a code set linked to the
    /// *internal* name (e.g. `languages`) is available.
    #[must_use]
    pub fn has_code_set(&self, name: &str) -> bool {
        self.find_code_set(|cs| cs.name == name).is_some()
    }

    /// Spec `terminology_identifiers(): List<String>`.
    #[must_use]
    pub fn terminology_identifiers(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.terminologies.iter().map(|t| t.name.clone()).collect();
        ids.dedup();
        ids
    }

    /// Spec `openehr_code_sets(): Hash<String, String>` — external
    /// identifiers keyed by internal openEHR name, for every bundled code
    /// set.
    #[must_use]
    pub fn openehr_code_sets(&self) -> HashMap<String, String> {
        self.all_code_sets()
            .map(|cs| (cs.name.clone(), cs.external_id.clone()))
            .collect()
    }

    /// Spec `code_set_identifiers(): List<String>` — the internal openEHR
    /// names of all available code sets.
    #[must_use]
    pub fn code_set_identifiers(&self) -> Vec<String> {
        self.all_code_sets().map(|cs| cs.name.clone()).collect()
    }

    /// PORT NOTE: convenience beyond the spec — the property/unit table
    /// (consumed by `DV_QUANTITY` validation at P11).
    #[must_use]
    pub fn property_unit_data(&self) -> &PropertyUnitData {
        &self.property_unit_data
    }

    /// Code sets from the English bundle plus the external ISO/IANA file.
    /// (Code sets are language-invariant; the `en` bundle is authoritative.)
    fn all_code_sets(&self) -> impl Iterator<Item = &CodeSet> {
        self.terminologies
            .first()
            .into_iter()
            .flat_map(|t| t.code_sets.iter())
            .chain(self.external.code_sets.iter())
    }

    fn find_code_set(&self, mut pred: impl FnMut(&&CodeSet) -> bool) -> Option<&CodeSet> {
        self.all_code_sets().find(|cs| pred(cs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_set_access::CodeSetAccess;
    use crate::openehr_terminology_group_identifiers::OpenehrTerminologyGroupIdentifiers as Groups;
    use crate::terminology_access::TerminologyAccess;
    use crate::terminology_code::TerminologyCode;

    fn service() -> &'static TerminologyService {
        TerminologyService::bundled().expect("bundled assets must parse")
    }

    #[test]
    fn resolves_a_real_rubric_from_the_bundled_xml() {
        let openehr = service()
            .terminology("openehr")
            .expect("openehr terminology");
        // audit change type: 249 = creation (TERM Release-3.0.0).
        assert_eq!(openehr.rubric_for_code("249").as_deref(), Some("creation"));
        assert_eq!(openehr.rubric_for_code("no-such-code"), None);
    }

    #[test]
    fn group_lookups_work_with_both_id_forms() {
        let openehr = service()
            .terminology("openehr")
            .expect("openehr terminology");
        let by_spec_id = openehr.codes_for_group_id(Groups::GROUP_ID_AUDIT_CHANGE_TYPE);
        let by_xml_id = openehr.codes_for_group_id("audit_change_type");
        assert_eq!(by_spec_id.len(), 9);
        assert_eq!(by_spec_id, by_xml_id);
        let creation = TerminologyCode::new(Groups::TERMINOLOGY_ID_OPENEHR, "249");
        assert!(openehr.has_code_for_group_id(Groups::GROUP_ID_AUDIT_CHANGE_TYPE, &creation));
    }

    #[test]
    fn id_532_first_match_is_the_version_lifecycle_rubric() {
        // SPECPR-51 regression: both rubrics exist; document-order scan
        // resolves 532 to the 'version lifecycle state' spelling.
        let openehr = service()
            .terminology("openehr")
            .expect("openehr terminology");
        assert_eq!(openehr.rubric_for_code("532").as_deref(), Some("complete"));
        let states = openehr.codes_for_group_id(Groups::GROUP_ID_INSTRUCTION_STATES);
        assert!(states.iter().any(|c| c.code_string == "532"));
    }

    #[test]
    fn all_seven_spec_code_set_identifiers_resolve() {
        let s = service();
        for id in [
            "character sets",
            "compression algorithms",
            "countries",
            "integrity check algorithms",
            "languages",
            "media types",
            "normal statuses",
        ] {
            assert!(s.has_code_set(id), "missing code set: {id}");
            assert!(s.code_set_for_id(id).is_some(), "no access for: {id}");
        }
        let languages = s.code_set("ISO_639-1").expect("ISO_639-1 by external id");
        assert!(languages.has_code("en"));
        assert!(languages.has_lang("de"));
        assert_eq!(s.openehr_code_sets().len(), 7);
    }

    #[test]
    fn spec_preconditions_reported_as_none() {
        let s = service();
        assert!(s.terminology("centc251").is_none());
        assert!(s.code_set_for_id("ISO_639-1").is_none()); // external id is not an internal id
        assert!(!s.has_terminology("umls"));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 support §terminology_package — docs/research/spec-cache/RM-1.1.0/support/uml_classes/terminology_service.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: terminology_service.adoc (8 functions) + TERM Release-3.0.0 assets
//   confidence: medium
//   todos: 0
//   note: preconditions surface as Option; constants-class inheritance realised as direct calls; language-selection convenience beyond spec is PORT-NOTEd
// ─────────────────────────────────────────────
