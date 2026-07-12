//! OPT 1.4 ingestion: canonical XML → [`OperationalTemplate`] parse plus the
//! top-level structural well-formedness gate (S-05).
//!
//! # Spec basis
//!
//! An operational template is the **compiled, inheritance-flattened, standalone
//! top-level artefact** (`docs/specs/openehr/AM/docs/OPT2/master02-overview.adoc`
//! §Types of OPT; `master03-opt_raw.adoc` §Flattening) and, being a descendant
//! of `AUTHORED_RESOURCE`
//! (`docs/specs/openehr/BASE/docs/resource/master02-resource_package.adoc`),
//! carries the S-01/S-02/S-03 meta-data (original language + translations,
//! `RESOURCE_DESCRIPTION`, revision history).
//!
//! PORT NOTE (G-T11 — OPT 1.4 has no prose master): there is **no normative
//! prose chapter** for the OPT 1.4 wire structure (the OPT2 masters describe the
//! ADL2 successor; blueprint `docs/blueprint/03-am.md` §Spec defects). The OPT
//! 1.4 canonical XML this module ingests is governed by the **ITS-XML v1
//! Template XSD** plus AOM 1.4 — cite those, never the OPT2 masters, for
//! structure conformance. The tolerant [`openehr_its::opt14`] codec decodes it;
//! [`validate_opt_structure`] closes the leniency the codec would otherwise
//! accept.
//!
//! PORT NOTE (G-T12 — meta-data parsed, not surfaced): the S-01/S-02/S-03
//! meta-data (`language` / `description` / `translations` / `revision_history`)
//! is parsed by the codec but we index only `template_id` / `concept` / root
//! archetype for lookup and listing (see [`crate::templates::store`]); the spec
//! permits an optional `_description_`
//! (BASE resource master02 §Meta-data). Surfacing/querying the full
//! `AUTHORED_RESOURCE` meta-data is not required by the provisioning surface.

use std::collections::HashMap;

use openehr_its::opt14::OperationalTemplate;

use crate::service::ServiceError;

/// Parse OPT 1.4 canonical XML into an [`OperationalTemplate`]. A codec failure
/// is a semantic error on the artefact (→ ITS-REST `422`), not a transport
/// error: the XML negotiated fine at the REST edge but does not decode as a
/// well-formed OPT.
pub(crate) fn parse_opt(xml: &str) -> Result<OperationalTemplate, ServiceError> {
    openehr_its::opt14::from_xml(xml)
        .map_err(|e| ServiceError::Unprocessable(format!("invalid OPT 1.4 XML: {e}")))
}

/// The known top-level child elements of an OPT 1.4 `<template>`
/// (`OPERATIONAL_TEMPLATE`) — the wire names of [`OperationalTemplate`]'s
/// attributes (ITS-XML v1 Template XSD). Any other top-level element is a
/// foreign / "alien" tag.
const OPT_TOP_LEVEL: &[&str] = &[
    "language",
    "is_controlled",
    "description",
    "revision_history",
    "uid",
    "template_id",
    "concept",
    "definition",
    "ontology",
    "component_ontologies",
    "annotations",
    "constraints",
    "view",
];

/// The top-level OPT attributes that legitimately repeat (`Vec`-valued):
/// `component_ontologies` and `annotations`. Every other attribute is
/// single-valued, so a repeated element is malformed.
const OPT_TOP_LEVEL_MULTIPLE: &[&str] = &["component_ontologies", "annotations"];

/// Reject an OPT the tolerant [`openehr_its::opt14`] codec would silently
/// accept: a **foreign** top-level element (CNF `invalid_templates/alien_tags`)
/// or a **duplicated single-valued** top-level element (CNF
/// `invalid_templates/multiple_elements` — `concept`/`definition`/`template_id`
/// twice). Only the direct children of the root `<template>` are inspected, so a
/// legitimate nested reuse of a name (e.g. `<template_id>` inside a term
/// binding) is unaffected. The codec already catches missing mandatory
/// elements; this closes the leniency the CNF `upload_opt-invalid_opt` case
/// exercises.
///
/// Spec: the structure is XSD-defined (G-T11) — the check enforces the ITS-XML
/// v1 Template XSD content model at the top level; a violation is `422`
/// (`Unprocessable`).
pub(crate) fn validate_opt_structure(xml: &str) -> Result<(), ServiceError> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    let mut depth: i32 = 0;
    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut check = |raw: &[u8]| -> Result<(), ServiceError> {
        let name = String::from_utf8_lossy(raw).into_owned();
        if !OPT_TOP_LEVEL.contains(&name.as_str()) {
            return Err(ServiceError::Unprocessable(format!(
                "operational template has an unexpected top-level element <{name}> \
                 (not an OPERATIONAL_TEMPLATE attribute)"
            )));
        }
        let count = seen.entry(name.clone()).or_insert(0);
        *count += 1;
        if *count > 1 && !OPT_TOP_LEVEL_MULTIPLE.contains(&name.as_str()) {
            return Err(ServiceError::Unprocessable(format!(
                "operational template has a duplicate single-valued <{name}> element"
            )));
        }
        Ok(())
    };

    loop {
        match reader.read_event() {
            // A direct child's Start is seen at depth 2 (root <template> is 1).
            Ok(Event::Start(e)) => {
                depth += 1;
                if depth == 2 {
                    check(e.local_name().as_ref())?;
                }
            }
            // A self-closing direct child is seen while the root (depth 1) is open.
            Ok(Event::Empty(e)) => {
                if depth == 1 {
                    check(e.local_name().as_ref())?;
                }
            }
            Ok(Event::End(_)) => depth -= 1,
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ServiceError::Unprocessable(format!(
                    "operational template XML is malformed: {e}"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_opt_structure;

    fn tmpl(children: &str) -> String {
        format!(
            "<?xml version=\"1.0\"?>\n<template xmlns=\"http://schemas.openehr.org/v1\">{children}</template>"
        )
    }

    #[test]
    fn accepts_a_well_formed_top_level() {
        // Single-valued attributes once each + a repeated multi-valued one.
        let xml = tmpl(
            "<language/><template_id><value>t.v1</value></template_id>\
             <concept>C</concept><definition/>\
             <component_ontologies/><component_ontologies/>",
        );
        assert!(validate_opt_structure(&xml).is_ok());
    }

    #[test]
    fn rejects_a_foreign_top_level_element() {
        // CNF invalid_templates/alien_tags — a <bullfrog> tag at the top level.
        let xml = tmpl("<concept>C</concept><bullfrog>x</bullfrog>");
        let err = validate_opt_structure(&xml).unwrap_err().to_string();
        assert!(err.contains("bullfrog"), "{err}");
    }

    #[test]
    fn rejects_duplicate_single_valued_top_level() {
        // CNF invalid_templates/multiple_elements — concept/definition/template_id twice.
        for name in ["concept", "definition", "template_id"] {
            let xml = tmpl(&format!("<{name}>a</{name}><{name}>b</{name}>"));
            let err = validate_opt_structure(&xml).unwrap_err().to_string();
            assert!(err.contains("duplicate"), "{name}: {err}");
        }
    }

    #[test]
    fn ignores_nested_reuse_of_a_name() {
        // A nested <template_id> deeper in the tree must not count as a duplicate
        // of the single top-level one (valid OPTs carry nested references).
        let xml = tmpl(
            "<template_id><value>t.v1</value></template_id>\
             <definition><attributes><template_id>ref</template_id></attributes></definition>",
        );
        assert!(validate_opt_structure(&xml).is_ok());
    }
}
