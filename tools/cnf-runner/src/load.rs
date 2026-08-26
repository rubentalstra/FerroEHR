// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Artifact loading: YAML → JSON value (budgeted, duplicate-rejecting) →
//! schema validation → typed model.
//!
//! Every artifact runs the same pipeline; a file that fails any stage is a
//! typed [`LoadError`] naming the file, so validator reports are uniform
//! across schema-level and model-level defects.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Loading error, one per file and stage.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Filesystem failure.
    #[error("{path}: {source}")]
    Io {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// YAML parse failure (incl. budget breaches and duplicate keys).
    #[error("{path}: YAML: {message}")]
    Yaml {
        /// The file that failed to parse.
        path: PathBuf,
        /// The parser's diagnostic.
        message: String,
    },
    /// JSON-Schema validation failure.
    #[error("{path}: schema: {message}")]
    Schema {
        /// The file that failed validation.
        path: PathBuf,
        /// The schema violation.
        message: String,
    },
    /// Typed-model parse failure (closed grammars, invariants).
    #[error("{path}: model: {message}")]
    Model {
        /// The file whose typed parse failed.
        path: PathBuf,
        /// The invariant or grammar that rejected it.
        message: String,
    },
}

impl LoadError {
    /// The stage + message without the file path (for reports that already
    /// name the artifact).
    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::Io { source, .. } => source.to_string(),
            Self::Yaml { message, .. } => format!("YAML: {message}"),
            Self::Schema { message, .. } => format!("schema: {message}"),
            Self::Model { message, .. } => format!("model: {message}"),
        }
    }

    /// The offending file.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Io { path, .. }
            | Self::Yaml { path, .. }
            | Self::Schema { path, .. }
            | Self::Model { path, .. } => path,
        }
    }
}

fn saphyr_options() -> serde_saphyr::Options {
    // Default carries the anti-bomb Budget; duplicate keys are always an
    // authoring error in schedule artifacts.
    let mut options = serde_saphyr::Options::default();
    options.duplicate_keys = serde_saphyr::DuplicateKeyPolicy::Error;
    // Only `true`/`false` are booleans: YAML 1.1 forms (`yes`/`on`) stay
    // strings, so the flow `on:` selector key and matrix cells never coerce.
    options.strict_booleans = true;
    options
}

/// Parse one YAML file to a JSON value.
///
/// # Errors
/// [`LoadError::Io`] / [`LoadError::Yaml`].
pub fn yaml_file_to_value(path: &Path) -> Result<serde_json::Value, LoadError> {
    let text = std::fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.to_owned(),
        source,
    })?;
    serde_saphyr::from_str_with_options(&text, saphyr_options()).map_err(|e| LoadError::Yaml {
        path: path.to_owned(),
        message: e.to_string(),
    })
}

/// Validate a value against a compiled schema.
///
/// # Errors
/// [`LoadError::Schema`] carrying every violation (joined).
pub fn validate_against(
    validator: &jsonschema::Validator,
    value: &serde_json::Value,
    path: &Path,
) -> Result<(), LoadError> {
    let violations: Vec<String> = validator
        .iter_errors(value)
        .map(|e| format!("{}: {e}", e.instance_path()))
        .collect();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(LoadError::Schema {
            path: path.to_owned(),
            message: violations.join("; "),
        })
    }
}

/// Full pipeline for one artifact file.
///
/// # Errors
/// Any [`LoadError`] stage.
pub fn load_artifact<T: serde::de::DeserializeOwned>(
    path: &Path,
    validator: &jsonschema::Validator,
) -> Result<T, LoadError> {
    let value = yaml_file_to_value(path)?;
    validate_against(validator, &value, path)?;
    serde_json::from_value(value).map_err(|e| LoadError::Model {
        path: path.to_owned(),
        message: e.to_string(),
    })
}

/// Compile a schema document (a defect here is a bug in
/// [`crate::schema`], surfaced as a typed error, never a panic).
///
/// # Errors
/// [`LoadError::Schema`] when the schema itself does not compile.
pub fn compile_schema(
    schema: &serde_json::Value,
    name: &str,
) -> Result<jsonschema::Validator, LoadError> {
    jsonschema::validator_for(schema).map_err(|e| LoadError::Schema {
        path: PathBuf::from(name),
        message: format!("schema does not compile: {e}"),
    })
}
