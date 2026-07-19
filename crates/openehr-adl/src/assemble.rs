//! AOM artefact assembly (phase A3c): fold the parsed ODIN sections, the cADL
//! `definition`, and the `rules` of a [`crate::source::SourceArtefact`] into the
//! **generated** `openehr_am::am24::aom2` object model, producing a complete
//! [`Archetype`].
//!
//! The lower-level entry points ([`crate::source::parse_source`],
//! [`crate::cadl::parse_definition_body`], [`crate::rules::parse_rules_body`])
//! stay available; this module composes them and maps each ODIN section
//! (`language`/`description`/`terminology`/`annotations`/`rm_overlay`/
//! `component_terminologies`) into its typed model class.
//!
//! Spec oracle: `docs/specs/openehr/AM/docs/ADL2/{master07.07,master07.08,
//! master07.12,master07.13,master07.14}` (the section formats), `AOM2/master03`
//! (archetype package), and `docs/specs/openehr/BASE/docs/resource/` (the
//! resource-description model). Type errors in a section raise
//! [`SyntaxErrorCode::Sdinv`] (or the section-specific `SA**` code) with the
//! section's position where recoverable.
//!
//! NOTE: the owner/parent back-references (`ARCHETYPE_TERMINOLOGY.owner_archetype`,
//! `RESOURCE_DESCRIPTION.parent_resource`) are omitted from the generated model
//! by the emitter (they are non-data navigational associations that make the
//! type non-constructible if owned — see the codegen `back_reference` rule), so
//! assembly needs no back-pointer wiring.

use std::collections::BTreeMap;

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::archetype::authored_archetype::{
    AuthoredArchetype, AuthoredArchetypeData,
};
use openehr_am::am24::aom2::archetype::operational_template::OperationalTemplate;
use openehr_am::am24::aom2::archetype::template::Template;
use openehr_am::am24::aom2::archetype::template_overlay::TemplateOverlay;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::rm_overlay::rm_attribute_visibility::RmAttributeVisibility;
use openehr_am::am24::aom2::rm_overlay::rm_overlay::RmOverlay;
use openehr_am::am24::aom2::rm_overlay::visibility_type::VisibilityType;
use openehr_am::am24::aom2::terminology::archetype_term::ArchetypeTerm;
use openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_am::am24::aom2::terminology::value_set::ValueSet;
use openehr_am::am24::beom::core::statement_set::StatementSet;
use openehr_am::am24::resource::resource_description::ResourceDescription;
use openehr_base::prelude::{
    ResourceAnnotations, ResourceDescriptionItem, TerminologyCode, TranslationDetails, Uuid,
};
use openehr_lang::odin::{OdinKey, OdinValue};

use crate::cadl::parse_definition_body;
use crate::error::{SyntaxError, SyntaxErrorCode};
use crate::rules::parse_artefact_rules;
use crate::source::{ArtefactKind, ArtefactMeta, SourceArtefact, parse_source};

/// Parse an ADL2 source into a fully-assembled [`Archetype`].
///
/// This is the high-level entry point of the ADL2 front end: it outer-parses
/// (`crate::source`), cADL-parses the `definition` (`crate::cadl`), parses the
/// `rules` (`crate::rules`), and maps every ODIN section into the generated
/// `openehr_am::am24::aom2` model, returning the artefact-kind-appropriate
/// `Archetype` enum variant.
///
/// # Errors
/// Returns every [`SyntaxError`] found across the outer parse, the cADL
/// definition, the rules, and the ODIN-section-to-model mapping.
pub fn parse_artefact(src: &str) -> Result<Archetype, Vec<SyntaxError>> {
    let art = parse_source(src)?;
    assemble(&art, src)
}

/// Assemble an already-outer-parsed [`SourceArtefact`] into an [`Archetype`],
/// given the whole source text (needed to re-lex the `definition`/`rules`
/// spans).
///
/// # Errors
/// Returns every [`SyntaxError`] from the cADL/rules parse and the section
/// mapping.
pub fn assemble(art: &SourceArtefact, src: &str) -> Result<Archetype, Vec<SyntaxError>> {
    let mut errors: Vec<SyntaxError> = Vec::new();

    let definition = assemble_definition(art, src, &mut errors);
    let rules = match parse_artefact_rules(art, src) {
        Ok(set) => set.into_iter().collect::<Vec<StatementSet>>(),
        Err(errs) => {
            errors.extend(errs);
            Vec::new()
        }
    };

    let original_language = assemble_original_language(art, &mut errors);
    let translations = art
        .language
        .as_ref()
        .and_then(assemble_translations)
        .filter(|t| !t.is_empty());
    let description = art
        .description
        .as_ref()
        .map(|d| Box::new(assemble_description(d)));
    let annotations = art.annotations.as_ref().map(assemble_annotations);
    let rm_overlay = art.rm_overlay.as_ref().map(assemble_rm_overlay);

    let concept_code = definition.as_ref().map(root_node_id).unwrap_or_default();
    let terminology = assemble_terminology(
        art,
        &concept_code,
        original_language.code_string.clone(),
        &mut errors,
    );

    // A definition failure is fatal for assembly (no root object to hang the
    // model on); report all collected errors together.
    let Some(definition) = definition else {
        if errors.is_empty() {
            errors.push(SyntaxError::at(
                SyntaxErrorCode::Sadf,
                "missing definition section",
                0..0,
                src,
            ));
        }
        return Err(errors);
    };
    // A `template` may carry `template_overlay` blocks; overlays store
    // whole-file byte spans, so they re-assemble against the same `src`.
    let mut overlays = Vec::new();
    for ov in &art.overlays {
        match assemble(ov, src) {
            Ok(Archetype::TemplateOverlay(b)) => overlays.push(*b),
            Ok(_) => {}
            Err(errs) => errors.extend(errs),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(build_archetype(BuildInputs {
        art,
        definition,
        terminology,
        rules,
        original_language,
        description,
        translations,
        annotations,
        rm_overlay,
        overlays,
    }))
}

/// cADL-parse the root `definition` span, span-offsetting errors to the whole
/// file. Returns `None` (and records the errors) on failure or absence.
fn assemble_definition(
    art: &SourceArtefact,
    src: &str,
    errors: &mut Vec<SyntaxError>,
) -> Option<CComplexObject> {
    let def = art.definition.as_ref()?;
    let body = src.get(def.bytes.clone()).unwrap_or_default();
    let offset = def.bytes.start;
    match parse_definition_body(body) {
        Ok(cco) => Some(cco),
        Err(errs) => {
            for e in errs {
                errors.push(SyntaxError::at(
                    e.code,
                    e.message,
                    (e.span.start + offset)..(e.span.end + offset),
                    src,
                ));
            }
            None
        }
    }
}

/// The root node id (`concept_code`) of a definition tree.
fn root_node_id(def: &CComplexObject) -> String {
    match def {
        CComplexObject::CComplexObject(d) => d.node_id.clone(),
        CComplexObject::CArchetypeRoot(r) => r.node_id.clone(),
    }
}

// ── language (master07.07) ────────────────────────────────────────────────

/// `original_language = <[ISO_639-1::en]>` → a [`TerminologyCode`]. A missing
/// `language` section is tolerated (`master07.07` legacy upgrade): the language
/// falls back to `ISO_639-1::en` so the model stays constructible; the absence
/// is not itself a hard error here (the outer parser already raised `SALAN`
/// where required).
fn assemble_original_language(
    art: &SourceArtefact,
    errors: &mut Vec<SyntaxError>,
) -> TerminologyCode {
    let Some(OdinValue::Object(map)) = art.language.as_ref() else {
        return term_code("ISO_639-1", "en");
    };
    match map.get("original_language") {
        Some(OdinValue::TermCode(code)) => parse_term_code(code),
        Some(other) => {
            errors.push(SyntaxError::at(
                SyntaxErrorCode::Sala,
                format!("original_language must be a terminology code, found {other:?}"),
                0..0,
                "",
            ));
            term_code("ISO_639-1", "en")
        }
        None => term_code("ISO_639-1", "en"),
    }
}

/// `translations = <["de"] = < language=<…> author=<…> … >>` →
/// `lang → TRANSLATION_DETAILS` (`master07.07`).
fn assemble_translations(language: &OdinValue) -> Option<BTreeMap<String, TranslationDetails>> {
    let OdinValue::Object(map) = language else {
        return None;
    };
    let entries = as_keyed(map.get("translations")?)?;
    let mut out = BTreeMap::new();
    for (key, value) in entries {
        let lang = key_str(key);
        let obj = as_object(value);
        let language_code = obj
            .and_then(|o| o.get("language"))
            .map_or_else(|| term_code("ISO_639-1", &lang), term_code_of);
        out.insert(
            lang,
            TranslationDetails {
                language: language_code,
                author: obj.map(|o| string_map_of(o, "author")).unwrap_or_default(),
                accreditation: obj.and_then(|o| string_of(o.get("accreditation"))),
                other_details: obj.and_then(|o| o.get("other_details")).map(string_map),
                version_last_translated: obj
                    .and_then(|o| string_of(o.get("version_last_translated"))),
                other_contributors: obj
                    .and_then(|o| o.get("other_contributors"))
                    .map(string_list)
                    .unwrap_or_default(),
            },
        );
    }
    Some(out)
}

// ── description (master07.08) ─────────────────────────────────────────────

/// The `description` ODIN block → [`ResourceDescription`] (`master07.08`;
/// resource model `docs/specs/openehr/BASE/docs/resource/`). The `regression`
/// tag the corpus uses lands in `other_details`.
fn assemble_description(desc: &OdinValue) -> ResourceDescription {
    let empty = indexmap::IndexMap::new();
    let map = as_object(desc).unwrap_or(&empty);
    ResourceDescription {
        title: string_of(map.get("title")),
        original_author: string_map_of(map, "original_author"),
        original_namespace: string_of(map.get("original_namespace")),
        original_publisher: string_of(map.get("original_publisher")),
        other_contributors: map
            .get("other_contributors")
            .map(string_list)
            .unwrap_or_default(),
        lifecycle_state: string_of(map.get("lifecycle_state")).unwrap_or_default(),
        custodian_namespace: string_of(map.get("custodian_namespace")),
        custodian_organisation: string_of(map.get("custodian_organisation")),
        copyright: string_of(map.get("copyright")),
        licence: string_of(map.get("licence")),
        ip_acknowledgements: map.get("ip_acknowledgements").map(string_map),
        references: map.get("references").map(string_map),
        resource_package_uri: string_of(map.get("resource_package_uri")),
        conversion_details: map.get("conversion_details").map(string_map),
        details: map.get("details").and_then(assemble_details),
        other_details: map.get("other_details").map(string_map),
    }
}

/// `details = <["en"] = < purpose=<…> keywords=<…> … >>` →
/// `lang → RESOURCE_DESCRIPTION_ITEM`.
fn assemble_details(details: &OdinValue) -> Option<BTreeMap<String, ResourceDescriptionItem>> {
    let entries = as_keyed(details)?;
    let mut out = BTreeMap::new();
    for (key, value) in entries {
        let lang = key_str(key);
        let obj = as_object(value);
        let language = obj
            .and_then(|o| o.get("language"))
            .map_or_else(|| term_code("ISO_639-1", &lang), term_code_of);
        out.insert(
            lang,
            ResourceDescriptionItem {
                language,
                purpose: obj
                    .and_then(|o| string_of(o.get("purpose")))
                    .unwrap_or_default(),
                keywords: obj
                    .and_then(|o| o.get("keywords"))
                    .map(string_list)
                    .unwrap_or_default(),
                use_: obj.and_then(|o| string_of(o.get("use"))),
                misuse: obj.and_then(|o| string_of(o.get("misuse"))),
                original_resource_uri: obj
                    .and_then(|o| o.get("original_resource_uri"))
                    .map(string_map),
                other_details: obj.and_then(|o| o.get("other_details")).map(string_map),
            },
        );
    }
    Some(out)
}

// ── terminology (master07.13) ─────────────────────────────────────────────

/// The `terminology` ODIN block → [`ArchetypeTerminology`] (`master07.13`).
/// Handles the deprecated 1.4 forms per §Deprecated Terminology Section
/// Features: `constraint_definitions`/`constraint_bindings` are merged into
/// `term_definitions`/`term_bindings`, the `items=<>` wrapper is unwrapped, and
/// `terminologies_available` is ignored.
fn assemble_terminology(
    art: &SourceArtefact,
    concept_code: &str,
    original_language: String,
    errors: &mut Vec<SyntaxError>,
) -> Box<ArchetypeTerminology> {
    let map = match art.terminology.as_ref() {
        Some(OdinValue::Object(m)) => Some(m),
        Some(other) => {
            errors.push(SyntaxError::at(
                SyntaxErrorCode::Saon,
                format!("terminology section must be an ODIN object, found {other:?}"),
                0..0,
                "",
            ));
            None
        }
        None => None,
    };

    let mut term_definitions: BTreeMap<String, BTreeMap<String, ArchetypeTerm>> = BTreeMap::new();
    let mut term_bindings: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut value_sets: BTreeMap<String, ValueSet> = BTreeMap::new();

    if let Some(map) = map {
        // term_definitions (+ deprecated constraint_definitions, merged).
        for key in ["term_definitions", "constraint_definitions"] {
            if let Some(v) = map.get(key) {
                merge_term_definitions(v, &mut term_definitions);
            }
        }
        // term_bindings (+ deprecated constraint_bindings, merged).
        for key in ["term_bindings", "constraint_bindings"] {
            if let Some(v) = map.get(key) {
                merge_term_bindings(v, &mut term_bindings);
            }
        }
        if let Some(v) = map.get("value_sets") {
            merge_value_sets(v, &mut value_sets);
        }
        // `terminologies_available` is intentionally ignored (deprecated).
    }

    Box::new(ArchetypeTerminology {
        // A specialised (differential-authored) artefact carries only
        // differential terms (`master07.13`); a top-level source archetype's
        // terminology is complete.
        is_differential: art.parent_ref.is_some(),
        original_language,
        concept_code: concept_code.to_owned(),
        term_definitions,
        term_bindings: (!term_bindings.is_empty()).then_some(term_bindings),
        value_sets: (!value_sets.is_empty()).then_some(value_sets),
        terminology_extracts: None,
    })
}

/// Merge a `term_definitions`/`constraint_definitions` block
/// (`["lang"] = <["code"] = < text=<…> description=<…> … >>`) — the `items=<…>`
/// wrapper is unwrapped where present (deprecated 1.4 form).
fn merge_term_definitions(
    v: &OdinValue,
    out: &mut BTreeMap<String, BTreeMap<String, ArchetypeTerm>>,
) {
    let Some(langs) = as_keyed(v) else { return };
    for (lang_key, lang_val) in langs {
        let lang = key_str(lang_key);
        let codes = unwrap_items(lang_val);
        let Some(code_entries) = as_keyed(codes) else {
            continue;
        };
        let bucket = out.entry(lang).or_default();
        for (code_key, code_val) in code_entries {
            let code = key_str(code_key);
            let obj = as_object(code_val);
            let mut other_items = obj.map(term_other_items).unwrap_or_default();
            let text = obj
                .and_then(|o| string_of(o.get("text")))
                .unwrap_or_default();
            let description = obj
                .and_then(|o| string_of(o.get("description")))
                .unwrap_or_default();
            other_items.remove("text");
            other_items.remove("description");
            bucket.insert(
                code.clone(),
                ArchetypeTerm {
                    code,
                    text,
                    description,
                    other_items: (!other_items.is_empty()).then_some(other_items),
                },
            );
        }
    }
}

/// Merge a `term_bindings`/`constraint_bindings` block
/// (`["terminology"] = <["key"] = <uri>>`).
fn merge_term_bindings(v: &OdinValue, out: &mut BTreeMap<String, BTreeMap<String, String>>) {
    let Some(terms) = as_keyed(v) else { return };
    for (term_key, term_val) in terms {
        let terminology = key_str(term_key);
        let inner = unwrap_items(term_val);
        let Some(bindings) = as_keyed(inner) else {
            continue;
        };
        let bucket = out.entry(terminology).or_default();
        for (bkey, bval) in bindings {
            bucket.insert(key_str(bkey), uri_string(bval));
        }
    }
}

/// Merge a `value_sets` block (`["ac1"] = < id=<"ac1"> members=<"at1", …> >`).
fn merge_value_sets(v: &OdinValue, out: &mut BTreeMap<String, ValueSet>) {
    let Some(entries) = as_keyed(v) else { return };
    for (key, value) in entries {
        let id_key = key_str(key);
        let obj = as_object(value);
        let id = obj
            .and_then(|o| string_of(o.get("id")))
            .unwrap_or_else(|| id_key.clone());
        let members = obj
            .and_then(|o| o.get("members"))
            .map(string_list)
            .unwrap_or_default();
        out.insert(id_key, ValueSet { id, members });
    }
}

// ── annotations (master07.14) + rm_overlay (master07.12) ──────────────────

/// `annotations` → [`ResourceAnnotations`] (`documentation` lang→path→tag map;
/// `master07.14`). The deprecated `items = <…>` wrapper at each level is
/// unwrapped (the same 1.4 tolerance as the terminology section, `master07.13`),
/// and the whole block may itself be keyed `items` instead of `documentation`.
fn assemble_annotations(annotations: &OdinValue) -> ResourceAnnotations {
    let mut documentation: BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>> =
        BTreeMap::new();
    if let OdinValue::Object(map) = annotations {
        let top = map
            .get("documentation")
            .or_else(|| map.get("items"))
            .map(unwrap_items);
        if let Some(langs) = top.and_then(as_keyed) {
            for (lang_key, lang_val) in langs {
                let lang = key_str(lang_key);
                let mut paths: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
                if let Some(path_entries) = as_keyed(unwrap_items(lang_val)) {
                    for (path_key, path_val) in path_entries {
                        paths.insert(key_str(path_key), string_map(unwrap_items(path_val)));
                    }
                }
                documentation.insert(lang, paths);
            }
        }
    }
    ResourceAnnotations { documentation }
}

/// `rm_overlay` → [`RmOverlay`] (`rm_visibility` path → visibility + alias;
/// `master07.12`).
fn assemble_rm_overlay(rm_overlay: &OdinValue) -> RmOverlay {
    let mut rm_visibility: BTreeMap<String, RmAttributeVisibility> = BTreeMap::new();
    if let OdinValue::Object(map) = rm_overlay
        && let Some(entries) = map.get("rm_visibility").and_then(as_keyed)
    {
        for (path_key, path_val) in entries {
            let obj = as_object(path_val);
            rm_visibility.insert(
                key_str(path_key),
                RmAttributeVisibility {
                    visibility: obj
                        .and_then(|o| string_of(o.get("visibility")))
                        .map(VisibilityType),
                    alias: obj.and_then(|o| o.get("alias")).map(term_code_of),
                },
            );
        }
    }
    RmOverlay {
        rm_visibility: (!rm_visibility.is_empty()).then_some(rm_visibility),
    }
}

// ── component_terminologies (OPT; master10) ───────────────────────────────

/// `component_terminologies = <["archetype-id"] = < … terminology … >>` →
/// `archetype-id → ARCHETYPE_TERMINOLOGY` for an OPT (`master10`).
fn assemble_component_terminologies(section: &OdinValue) -> BTreeMap<String, ArchetypeTerminology> {
    let mut out = BTreeMap::new();
    let Some(entries) = as_keyed(section) else {
        return out;
    };
    for (key, value) in entries {
        let archetype_id = key_str(key);
        let mut term_definitions: BTreeMap<String, BTreeMap<String, ArchetypeTerm>> =
            BTreeMap::new();
        let mut term_bindings: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut value_sets: BTreeMap<String, ValueSet> = BTreeMap::new();
        let mut original_language = String::from("en");
        if let Some(obj) = as_object(value) {
            if let Some(v) = obj.get("term_definitions") {
                merge_term_definitions(v, &mut term_definitions);
            }
            if let Some(v) = obj.get("term_bindings") {
                merge_term_bindings(v, &mut term_bindings);
            }
            if let Some(v) = obj.get("value_sets") {
                merge_value_sets(v, &mut value_sets);
            }
            if let Some(lang) = string_of(obj.get("original_language")) {
                original_language = lang;
            }
        }
        out.insert(
            archetype_id,
            ArchetypeTerminology {
                is_differential: false,
                original_language,
                concept_code: String::new(),
                term_definitions,
                term_bindings: (!term_bindings.is_empty()).then_some(term_bindings),
                value_sets: (!value_sets.is_empty()).then_some(value_sets),
                terminology_extracts: None,
            },
        );
    }
    out
}

// ── archetype construction ────────────────────────────────────────────────

/// The assembled parts fed to [`build_archetype`].
struct BuildInputs<'a> {
    art: &'a SourceArtefact,
    definition: CComplexObject,
    terminology: Box<ArchetypeTerminology>,
    rules: Vec<StatementSet>,
    original_language: TerminologyCode,
    description: Option<Box<ResourceDescription>>,
    translations: Option<BTreeMap<String, TranslationDetails>>,
    annotations: Option<ResourceAnnotations>,
    rm_overlay: Option<RmOverlay>,
    overlays: Vec<TemplateOverlay>,
}

/// Build the artefact-kind-appropriate [`Archetype`] enum variant from the
/// assembled parts and the identification meta (`master07.05`).
fn build_archetype(i: BuildInputs<'_>) -> Archetype {
    let meta = Meta::from(&i.art.meta);
    let parent_archetype_id = i
        .art
        .parent_ref
        .as_ref()
        .map(crate::printer::hrid_to_string);

    match i.art.kind {
        ArtefactKind::TemplateOverlay => Archetype::TemplateOverlay(Box::new(TemplateOverlay {
            parent_archetype_id,
            archetype_id: i.art.hrid.clone(),
            is_differential: true,
            definition: i.definition,
            terminology: i.terminology,
            rules: i.rules,
            rm_overlay: i.rm_overlay,
        })),
        ArtefactKind::OperationalTemplate => {
            let component_terminologies = i
                .art
                .component_terminologies
                .as_ref()
                .map(assemble_component_terminologies);
            Archetype::AuthoredArchetype(Box::new(AuthoredArchetype::OperationalTemplate(
                Box::new(OperationalTemplate {
                    parent_archetype_id,
                    archetype_id: i.art.hrid.clone(),
                    is_differential: false,
                    definition: i.definition,
                    terminology: *i.terminology,
                    rules: i.rules,
                    rm_overlay: i.rm_overlay,
                    uid: meta.uid,
                    original_language: i.original_language,
                    description: i.description.map(|b| *b),
                    is_controlled: meta.is_controlled,
                    annotations: i.annotations,
                    translations: i.translations,
                    adl_version: meta.adl_version,
                    build_uid: meta.build_uid,
                    rm_release: meta.rm_release,
                    is_generated: meta.is_generated,
                    other_meta_data: meta.other,
                    component_terminologies,
                    terminology_extracts: None,
                }),
            )))
        }
        ArtefactKind::Template => Archetype::AuthoredArchetype(Box::new(
            AuthoredArchetype::Template(Box::new(Template {
                parent_archetype_id,
                archetype_id: i.art.hrid.clone(),
                is_differential: true,
                definition: i.definition,
                terminology: *i.terminology,
                rules: i.rules,
                rm_overlay: i.rm_overlay,
                uid: meta.uid,
                original_language: i.original_language,
                description: i.description.map(|b| *b),
                is_controlled: meta.is_controlled,
                annotations: i.annotations,
                translations: i.translations,
                adl_version: meta.adl_version,
                build_uid: meta.build_uid,
                rm_release: meta.rm_release,
                is_generated: meta.is_generated,
                other_meta_data: meta.other,
                overlays: i.overlays,
            })),
        )),
        ArtefactKind::Archetype => Archetype::AuthoredArchetype(Box::new(
            AuthoredArchetype::AuthoredArchetype(AuthoredArchetypeData {
                parent_archetype_id,
                archetype_id: i.art.hrid.clone(),
                is_differential: true,
                definition: i.definition,
                terminology: i.terminology,
                rules: i.rules,
                rm_overlay: i.rm_overlay,
                uid: meta.uid,
                original_language: i.original_language,
                description: i.description,
                is_controlled: meta.is_controlled,
                annotations: i.annotations,
                translations: i.translations,
                adl_version: meta.adl_version,
                build_uid: meta.build_uid,
                rm_release: meta.rm_release,
                is_generated: meta.is_generated,
                other_meta_data: meta.other,
            }),
        )),
    }
}

/// The identification meta, mapped from [`ArtefactMeta`] into the typed fields.
struct Meta {
    adl_version: Option<String>,
    rm_release: String,
    uid: Option<Uuid>,
    build_uid: Uuid,
    is_controlled: Option<bool>,
    is_generated: bool,
    other: BTreeMap<String, String>,
}

impl From<&ArtefactMeta> for Meta {
    fn from(m: &ArtefactMeta) -> Self {
        let mut other: BTreeMap<String, String> = BTreeMap::new();
        // A UUID `uid` populates the typed field; any other id form (e.g. an ISO
        // OID) is preserved verbatim in `other_meta_data` (`master07.05`).
        let uid = m.uid.as_ref().and_then(|s| parse_uuid(s));
        if uid.is_none()
            && let Some(s) = &m.uid
        {
            other.insert("uid".to_owned(), s.clone());
        }
        if let Some(p) = &m.provenance_id {
            other.insert("provenance_id".to_owned(), p.clone());
        }
        for (k, v) in &m.other {
            other.insert(k.clone(), v.clone().unwrap_or_default());
        }
        Self {
            adl_version: m.adl_version.clone(),
            rm_release: m.rm_release.clone().unwrap_or_default(),
            uid,
            build_uid: m
                .build_uid
                .as_ref()
                .and_then(|s| parse_uuid(s))
                .unwrap_or_else(nil_uuid),
            is_controlled: m.controlled,
            is_generated: m.generated,
            other,
        }
    }
}

// ── ODIN accessors ────────────────────────────────────────────────────────

fn as_object(v: &OdinValue) -> Option<&indexmap::IndexMap<String, OdinValue>> {
    match v {
        OdinValue::Object(m) => Some(m),
        _ => None,
    }
}

fn as_keyed(v: &OdinValue) -> Option<&[(OdinKey, OdinValue)]> {
    match v {
        OdinValue::KeyedList(items) => Some(items),
        _ => None,
    }
}

fn key_str(k: &OdinKey) -> String {
    match k {
        OdinKey::String(s) | OdinKey::Date(s) | OdinKey::Time(s) | OdinKey::DateTime(s) => {
            s.clone()
        }
        OdinKey::Integer(i) => i.to_string(),
    }
}

/// The scalar string an ODIN leaf carries (or a single-element list's leaf).
fn string_of(v: Option<&OdinValue>) -> Option<String> {
    match v? {
        OdinValue::String(s)
        | OdinValue::Date(s)
        | OdinValue::Time(s)
        | OdinValue::DateTime(s)
        | OdinValue::Duration(s)
        | OdinValue::Path(s) => Some(s.clone()),
        OdinValue::Integer(i) => Some(i.to_string()),
        OdinValue::Real(r) => Some(r.to_string()),
        OdinValue::Boolean(b) => Some(b.to_string()),
        OdinValue::Character(c) => Some(c.to_string()),
        OdinValue::Uri(u) => Some(strip_uri_delims(u)),
        OdinValue::List(items) if items.len() == 1 => string_of(items.first()),
        _ => None,
    }
}

/// A list of strings from an ODIN `List` (or a single scalar as a one-element
/// list), dropping the trailing open-list marker.
fn string_list(v: &OdinValue) -> Vec<String> {
    match v {
        OdinValue::List(items) => items
            .iter()
            .filter(|x| !matches!(x, OdinValue::ListContinue))
            .filter_map(|x| string_of(Some(x)))
            .collect(),
        other => string_of(Some(other)).into_iter().collect(),
    }
}

/// A `key → String` map from an ODIN keyed list or object of string leaves.
fn string_map(v: &OdinValue) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    match v {
        OdinValue::KeyedList(items) => {
            for (k, val) in items {
                if let Some(s) = string_of(Some(val)) {
                    out.insert(key_str(k), s);
                }
            }
        }
        OdinValue::Object(map) => {
            for (k, val) in map {
                if let Some(s) = string_of(Some(val)) {
                    out.insert(k.clone(), s);
                }
            }
        }
        _ => {}
    }
    out
}

/// The `key → String` map at `field` of `obj` (empty if absent).
fn string_map_of(
    obj: &indexmap::IndexMap<String, OdinValue>,
    field: &str,
) -> BTreeMap<String, String> {
    obj.get(field).map(string_map).unwrap_or_default()
}

/// Every string leaf of an object as `key → String` (used to gather an
/// `ARCHETYPE_TERM`'s `other_items` before removing `text`/`description`).
fn term_other_items(obj: &indexmap::IndexMap<String, OdinValue>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (k, v) in obj {
        if let Some(s) = string_of(Some(v)) {
            out.insert(k.clone(), s);
        }
    }
    out
}

/// Unwrap the deprecated `items = <…>` wrapper around a term/binding block
/// (`master07.13` §Deprecated Terminology Section Features); returns the inner
/// value, or `v` unchanged if there is no wrapper.
fn unwrap_items(v: &OdinValue) -> &OdinValue {
    if let OdinValue::Object(map) = v
        && map.len() == 1
        && let Some(inner) = map.get("items")
    {
        return inner;
    }
    v
}

/// The URI string a binding value carries, stripped of ODIN `<>` delimiters.
fn uri_string(v: &OdinValue) -> String {
    match v {
        OdinValue::Uri(u) => strip_uri_delims(u),
        OdinValue::TermCode(t) => t.clone(),
        OdinValue::PathList(ps) => ps.first().cloned().unwrap_or_default(),
        other => string_of(Some(other)).unwrap_or_default(),
    }
}

fn strip_uri_delims(u: &str) -> String {
    u.trim_start_matches('<').trim_end_matches('>').to_owned()
}

/// A `TerminologyCode` from an ODIN term-code leaf (`[ISO_639-1::en]`).
fn term_code_of(v: &OdinValue) -> TerminologyCode {
    match v {
        OdinValue::TermCode(code) => parse_term_code(code),
        other => {
            string_of(Some(other)).map_or_else(|| term_code("local", ""), |s| parse_term_code(&s))
        }
    }
}

/// Parse `[terminology::code]` (or `[terminology(version)::code]`) into a
/// [`TerminologyCode`].
fn parse_term_code(raw: &str) -> TerminologyCode {
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    let (terminology_part, code_string) = match inner.split_once("::") {
        Some((t, c)) => (t.to_owned(), c.to_owned()),
        None => ("local".to_owned(), inner.to_owned()),
    };
    // Optional `(version)` suffix on the terminology id.
    let (terminology_id, terminology_version) = match terminology_part.split_once('(') {
        Some((id, ver)) => (id.to_owned(), Some(ver.trim_end_matches(')').to_owned())),
        None => (terminology_part, None),
    };
    TerminologyCode {
        terminology_id,
        terminology_version,
        code_string,
        uri: None,
    }
}

fn term_code(terminology_id: &str, code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: terminology_id.to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

fn parse_uuid(s: &str) -> Option<Uuid> {
    uuid::Uuid::parse_str(s.trim())
        .ok()
        .map(|value| Uuid { value })
}

fn nil_uuid() -> Uuid {
    Uuid {
        value: uuid::Uuid::nil(),
    }
}

/// The `regression` tag of an assembled [`Archetype`], read from the resource
/// description's `other_details["regression"]`. The corpus uses this tag as the
/// authoritative expected-outcome oracle (INVENTORY.md); validation-phase
/// harnesses read it through this helper.
#[must_use]
pub fn regression_tag(archetype: &Archetype) -> Option<String> {
    description_of(archetype)?
        .other_details
        .as_ref()?
        .get("regression")
        .cloned()
}

/// The [`ResourceDescription`] of an archetype, if it carries one (a
/// `TEMPLATE_OVERLAY` inherits its owner's description and has none).
fn description_of(archetype: &Archetype) -> Option<&ResourceDescription> {
    match archetype {
        Archetype::AuthoredArchetype(a) => match a.as_ref() {
            AuthoredArchetype::AuthoredArchetype(d) => d.description.as_deref(),
            AuthoredArchetype::Template(t) => t.description.as_ref(),
            AuthoredArchetype::OperationalTemplate(o) => o.description.as_ref(),
        },
        Archetype::TemplateOverlay(_) => None,
    }
}
