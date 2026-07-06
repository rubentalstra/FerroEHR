//! Errors for the FLAT / `WebTemplate` layer.

/// An error building a [`crate::WebTemplate`] from an operational template.
#[derive(Debug, thiserror::Error)]
pub enum FlatError {
    /// The operational template lacked something the builder requires.
    #[error("invalid operational template: {0}")]
    InvalidTemplate(String),

    /// An OPT 1.4 XML document failed to parse.
    #[error("failed to parse OPT 1.4 XML: {0}")]
    OptParse(String),

    /// The resulting `WebTemplate` could not be serialized to JSON.
    #[error("failed to serialize WebTemplate to JSON: {0}")]
    Serialize(#[from] serde_json::Error),

    /// A FLAT composition was not the shape the converter expects (e.g. the RM
    /// value at a web-template path was not the RM type the template declares).
    #[error("FLAT conversion error: {0}")]
    Conversion(String),

    /// A FLAT key could not be resolved to a web-template node.
    #[error("unknown FLAT path: {0}")]
    UnknownPath(String),
}
