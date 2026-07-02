//! Parsed model of `openehr_terminology.xml` (and the structurally identical
//! `openehr_external_terminologies.xml`), plus the quick-xml pull parser.
//!
//! The document shape (per `assets/schema/openehr_terminology.xsd`):
//!
//! ```xml
//! <terminology name="openehr" language="en" version="3.0.0" date="...">
//!   <codeset issuer=".." openehr_id=".." name=".." external_id="..">
//!     <code value=".." [description=".."]/>
//!   </codeset>
//!   <group openehr_id=".." name="..">
//!     <concept id=".." rubric=".."/>
//!   </group>
//! </terminology>
//! ```
//!
//! Concepts are kept **verbatim in document order, duplicates included** —
//! the `id="532"` concept deliberately appears in both the
//! `version lifecycle state` group (rubric `complete`) and the
//! `instruction states` group (rubric `completed`); see SPECPR-51. Do not
//! dedupe by id.

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::error::TerminologyError;

/// One `<terminology>` document: metadata plus its code sets and concept
/// groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminology {
    /// `name` attribute (e.g. `openehr`).
    pub name: String,
    /// `language` attribute (e.g. `en`).
    pub language: String,
    /// `version` attribute (e.g. `3.0.0`).
    pub version: String,
    /// `date` attribute.
    pub date: String,
    /// `<codeset>` children, in document order.
    pub code_sets: Vec<CodeSet>,
    /// `<group>` children, in document order.
    pub groups: Vec<ConceptGroup>,
}

/// One `<codeset>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSet {
    /// `issuer` attribute (e.g. `openehr`, `ISO`, `IANA`).
    pub issuer: String,
    /// `openehr_id` attribute — the underscore form of the internal openEHR
    /// code-set name (e.g. `normal_statuses`).
    pub openehr_id: String,
    /// `name` attribute — the space form (e.g. `normal statuses`).
    pub name: String,
    /// `external_id` attribute (e.g. `openehr_normal_statuses`, `ISO_639-1`).
    pub external_id: String,
    /// `<code>` children, in document order.
    pub codes: Vec<Code>,
}

/// One `<code>` element inside a code set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Code {
    /// `value` attribute.
    pub value: String,
    /// Optional `description` attribute (used by the external ISO/IANA
    /// code sets, absent from the openEHR-issued ones).
    pub description: Option<String>,
}

/// One `<group>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptGroup {
    /// `openehr_id` attribute — underscore form (e.g. `audit_change_type`).
    pub openehr_id: String,
    /// `name` attribute — space form, language-dependent
    /// (e.g. `audit change type`).
    pub name: String,
    /// `<concept>` children, verbatim, in document order.
    pub concepts: Vec<Concept>,
}

/// One `<concept>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Concept {
    /// `id` attribute. Kept as a string: ids are numerals in practice but the
    /// schema does not require it.
    pub id: String,
    /// `rubric` attribute.
    pub rubric: String,
}

/// Pulls one attribute by name from an element, decoded and unescaped.
fn attribute(
    element: &BytesStart<'_>,
    name: &'static str,
    source_name: &'static str,
) -> Result<Option<String>, TerminologyError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|source| TerminologyError::Attribute {
            source_name,
            source,
        })?;
        if attr.key.as_ref() == name.as_bytes() {
            // The terminology bundles carry no XML declaration → XML 1.0
            // attribute-value normalization is assumed per the XML spec.
            let value = attr
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|source| TerminologyError::Xml {
                    source_name,
                    source,
                })?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

/// Like [`attribute`], but the attribute is required by the schema.
fn required_attribute(
    element: &BytesStart<'_>,
    name: &'static str,
    element_name: &'static str,
    source_name: &'static str,
) -> Result<String, TerminologyError> {
    attribute(element, name, source_name)?.ok_or(TerminologyError::MissingAttribute {
        source_name,
        element: element_name,
        attribute: name,
    })
}

/// Parses one terminology XML document.
///
/// `source_name` identifies the bundled asset in error messages.
///
/// # Errors
///
/// [`TerminologyError`] when the XML is malformed, a schema-required
/// attribute is missing, or the element nesting deviates from the XSD shape.
pub fn parse_terminology(
    xml: &str,
    source_name: &'static str,
) -> Result<Terminology, TerminologyError> {
    let mut reader = Reader::from_str(xml);
    let mut terminology: Option<Terminology> = None;
    let mut open_code_set: Option<CodeSet> = None;
    let mut open_group: Option<ConceptGroup> = None;

    loop {
        let event = reader
            .read_event()
            .map_err(|source| TerminologyError::Xml {
                source_name,
                source,
            })?;
        match event {
            Event::Start(e) | Event::Empty(e) => match e.name().as_ref() {
                b"terminology" => {
                    terminology = Some(Terminology {
                        name: required_attribute(&e, "name", "terminology", source_name)?,
                        language: required_attribute(&e, "language", "terminology", source_name)?,
                        version: required_attribute(&e, "version", "terminology", source_name)?,
                        date: required_attribute(&e, "date", "terminology", source_name)?,
                        code_sets: Vec::new(),
                        groups: Vec::new(),
                    });
                }
                b"codeset" => {
                    open_code_set = Some(CodeSet {
                        issuer: required_attribute(&e, "issuer", "codeset", source_name)?,
                        openehr_id: required_attribute(&e, "openehr_id", "codeset", source_name)?,
                        name: required_attribute(&e, "name", "codeset", source_name)?,
                        external_id: required_attribute(&e, "external_id", "codeset", source_name)?,
                        codes: Vec::new(),
                    });
                }
                b"code" => {
                    let code = Code {
                        value: required_attribute(&e, "value", "code", source_name)?,
                        description: attribute(&e, "description", source_name)?,
                    };
                    match open_code_set.as_mut() {
                        Some(cs) => cs.codes.push(code),
                        None => {
                            return Err(TerminologyError::UnexpectedStructure {
                                source_name,
                                detail: "<code> outside <codeset>".into(),
                            });
                        }
                    }
                }
                b"group" => {
                    open_group = Some(ConceptGroup {
                        openehr_id: required_attribute(&e, "openehr_id", "group", source_name)?,
                        name: required_attribute(&e, "name", "group", source_name)?,
                        concepts: Vec::new(),
                    });
                }
                b"concept" => {
                    let concept = Concept {
                        id: required_attribute(&e, "id", "concept", source_name)?,
                        rubric: required_attribute(&e, "rubric", "concept", source_name)?,
                    };
                    match open_group.as_mut() {
                        Some(g) => g.concepts.push(concept),
                        None => {
                            return Err(TerminologyError::UnexpectedStructure {
                                source_name,
                                detail: "<concept> outside <group>".into(),
                            });
                        }
                    }
                }
                _ => {}
            },
            Event::End(e) => match e.name().as_ref() {
                b"codeset" => {
                    if let (Some(t), Some(cs)) = (terminology.as_mut(), open_code_set.take()) {
                        t.code_sets.push(cs);
                    }
                }
                b"group" => {
                    if let (Some(t), Some(g)) = (terminology.as_mut(), open_group.take()) {
                        t.groups.push(g);
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            // Comments (incl. the SPECPR-51 warnings), text whitespace, and
            // the XML declaration carry no model content.
            _ => {}
        }
    }

    terminology.ok_or(TerminologyError::UnexpectedStructure {
        source_name,
        detail: "no <terminology> root element".into(),
    })
}

impl Terminology {
    /// Find a concept group by either its `openehr_id` (underscore form) or
    /// its `name` (space form).
    #[must_use]
    pub fn group(&self, id_or_name: &str) -> Option<&ConceptGroup> {
        self.groups
            .iter()
            .find(|g| g.openehr_id == id_or_name || g.name == id_or_name)
    }

    /// Find a code set by `openehr_id`, `name`, or `external_id`.
    #[must_use]
    pub fn code_set(&self, id_or_name: &str) -> Option<&CodeSet> {
        self.code_sets.iter().find(|cs| {
            cs.openehr_id == id_or_name || cs.name == id_or_name || cs.external_id == id_or_name
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets;

    #[test]
    fn parses_the_bundled_en_terminology() {
        let t = parse_terminology(assets::OPENEHR_TERMINOLOGY_EN, "en/openehr_terminology.xml")
            .expect("bundled en terminology must parse");
        assert_eq!(t.name, "openehr");
        assert_eq!(t.language, "en");
        assert_eq!(t.version, "3.0.0");
        // Pinned against TERM Release-3.0.0 exactly (regression guard for
        // accidental asset edits).
        assert_eq!(t.code_sets.len(), 3);
        assert_eq!(t.groups.len(), 17);
        let concept_count: usize = t.groups.iter().map(|g| g.concepts.len()).sum();
        assert_eq!(concept_count, 249);
    }

    #[test]
    fn parses_the_bundled_external_terminologies() {
        let t = parse_terminology(
            assets::OPENEHR_EXTERNAL_TERMINOLOGIES,
            "openehr_external_terminologies.xml",
        )
        .expect("bundled external terminologies must parse");
        let ids: Vec<&str> = t
            .code_sets
            .iter()
            .map(|cs| cs.openehr_id.as_str())
            .collect();
        assert_eq!(
            ids,
            ["countries", "character_sets", "languages", "media_types"]
        );
        let languages = t.code_set("languages").expect("languages code set");
        assert_eq!(languages.external_id, "ISO_639-1");
        assert!(languages.codes.iter().any(|c| c.value == "en"));
    }

    #[test]
    fn preserves_the_id_532_dual_rubric_quirk_verbatim() {
        // SPECPR-51: concept id 532 has rubric "complete" in
        // 'version lifecycle state' but "completed" in 'instruction states'.
        // The bundle keeps both, untouched, in document order.
        let t = parse_terminology(assets::OPENEHR_TERMINOLOGY_EN, "en/openehr_terminology.xml")
            .expect("bundled en terminology must parse");

        let version_lifecycle = t
            .group("version lifecycle state")
            .expect("version lifecycle state group");
        assert!(
            version_lifecycle
                .concepts
                .iter()
                .any(|c| c.id == "532" && c.rubric == "complete")
        );

        let instruction_states = t
            .group("instruction states")
            .expect("instruction states group");
        assert!(
            instruction_states
                .concepts
                .iter()
                .any(|c| c.id == "532" && c.rubric == "completed")
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: TERM Release-3.0.0 computable/XML (openehr_terminology.xsd shape) — crates/openehr-terminology/assets/ (Release-3.0.0 @ d45ef3e)
//   source_loc: assets/schema/openehr_terminology.xsd
//   confidence: high
//   todos: 0
//   note: duplicates kept verbatim (SPECPR-51 id=532); parser is shared by the language bundles and the external terminologies file
// ─────────────────────────────────────────────
