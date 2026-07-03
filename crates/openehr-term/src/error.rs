//! Error type for terminology bundle parsing and lookup.

/// Errors raised while parsing the bundled openEHR terminology XML assets.
#[derive(Debug, thiserror::Error)]
pub enum TerminologyError {
    /// The underlying XML reader failed.
    #[error("XML error in {source_name}: {source}")]
    Xml {
        /// Which bundled asset was being parsed.
        source_name: &'static str,
        #[source]
        source: quick_xml::Error,
    },

    /// An attribute could not be read or decoded.
    #[error("attribute error in {source_name}: {source}")]
    Attribute {
        /// Which bundled asset was being parsed.
        source_name: &'static str,
        #[source]
        source: quick_xml::events::attributes::AttrError,
    },

    /// A required attribute is absent from an element.
    #[error("missing required attribute '{attribute}' on <{element}> in {source_name}")]
    MissingAttribute {
        /// Which bundled asset was being parsed.
        source_name: &'static str,
        /// Element the attribute was expected on.
        element: &'static str,
        /// The absent attribute.
        attribute: &'static str,
    },

    /// The document structure deviates from the `openehr_term.xsd` shape.
    #[error("unexpected structure in {source_name}: {detail}")]
    UnexpectedStructure {
        /// Which bundled asset was being parsed.
        source_name: &'static str,
        /// Human-readable description of the deviation.
        detail: String,
    },
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: infrastructure for TERM Release-3.0.0 bundle parsing (no spec class)
//   source_loc: n/a
//   confidence: high
//   todos: 0
//   note: error surface only; grows if later phases add external terminology loading
// ─────────────────────────────────────────────
