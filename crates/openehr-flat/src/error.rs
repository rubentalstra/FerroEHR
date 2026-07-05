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
}
