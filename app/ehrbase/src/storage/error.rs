/// Errors produced by the node-storage codec.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The root value is not a decomposable versioned-object tree.
    #[error("root object has no structure _type (found {0:?})")]
    NotAStructureRoot(Option<String>),

    /// Arrays must be uniformly structure or non-structure values.
    #[error("array {attribute:?} mixes structure and non-structure elements")]
    MixedArray { attribute: String },

    /// Reassembly received rows that do not form one tree.
    #[error("invalid node rows: {0}")]
    InvalidRows(String),
}
