// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The loaded artifact set — one root directory laid out per the artifact
//! families:
//!
//! ```text
//! <root>/schedule/**/*.yaml        case cores
//! <root>/bindings/<its>/*.yaml     operation bindings
//! <root>/vocab/outcomes.yaml       outcome vocabulary
//! <root>/vocab/selectors.yaml      selector vocabulary + ignore-sets
//! <root>/vocab/capability_matrix.yaml
//! <root>/vocab/journey_catalogue.yaml   the hospital-simulation journeys
//! <root>/corpus/MANIFEST.yaml      corpus manifest
//! <root>/registers/ambiguities.yaml
//! ```
//!
//! The committed PARTY statements are swept alongside, from the sibling
//! `party/` directory of the artifact root (`<root>/../party/*/statement.json`
//! — the repo layout is `tools/cnf-runner/{artifacts,party}`). They are not
//! schedule artifacts, but the claim-completeness gate is a relation between
//! a claim and the catalogue, so validate cannot judge one without the other.
//! The sweep is best-effort by design: a bare artifact tree with no sibling
//! `party/` directory validates exactly as before.
//!
//! Loading never fails fast: every file error becomes a finding, so one
//! validation run reports the whole tree.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::load::{LoadError, compile_schema, load_artifact};
use crate::model::binding::OperationBinding;
use crate::model::capability::CapabilityMatrix;
use crate::model::case::CaseCore;
use crate::model::corpus::CorpusManifest;
use crate::model::register::AmbiguityRegister;
use crate::model::vocab_files::{OutcomesVocab, SelectorsVocab};
use crate::model::wire_surface::WireSurface;
use crate::schema;

/// The fully-typed artifact set (files that failed to load are absent; their
/// errors travel alongside).
#[derive(Debug, Default)]
pub struct ArtifactSet {
    /// The loaded functional/content case cores, with their source paths.
    pub cases: Vec<(PathBuf, CaseCore)>,
    /// `kind: performance` cases (their own schema family; measured, not
    /// asserted).
    pub performance: Vec<(PathBuf, crate::perf::PerformanceCase)>,
    /// The wire realizations, one file per SM operation (plus variants).
    pub bindings: Vec<(PathBuf, OperationBinding)>,
    /// The outcome-kind vocabulary.
    pub outcomes: Option<(PathBuf, OutcomesVocab)>,
    /// The selector vocabulary (server-assigned sets, ctx defaults).
    pub selectors: Option<(PathBuf, SelectorsVocab)>,
    /// The capability matrix: the certificate's rating dimensions.
    pub matrix: Option<(PathBuf, CapabilityMatrix)>,
    /// The clinical journey catalogue the performance workloads decompose
    /// into (`vocab/journey_catalogue.yaml`).
    pub journeys: Option<(PathBuf, crate::perf::JourneyCatalogue)>,
    /// The corpus manifest: every fixture, its format and its adjudication.
    pub corpus: Option<(PathBuf, CorpusManifest)>,
    /// The ambiguity register: spec silences with a typed disposition.
    pub register: Option<(PathBuf, AmbiguityRegister)>,
    /// The wire-surface coverage register (`vocab/wire_surface.yaml`) — the
    /// authored, spec-cited exceptions + cross-cutting elements the
    /// `surface-coverage` gate measures the catalogue against.
    pub wire_surface: Option<(PathBuf, WireSurface)>,
    /// The corpus manifest's directory (source paths resolve against it).
    pub corpus_dir: Option<PathBuf>,
    /// The committed party statements (`<root>/../party/*/statement.json`),
    /// in path order — the ICS side of the claim-completeness gate. Empty
    /// when no sibling `party/` directory exists.
    pub parties: Vec<(PathBuf, crate::party::Statement)>,
}

/// A load pass over one artifact root.
#[derive(Debug, Default)]
pub struct Loaded {
    /// Everything that loaded and typed successfully.
    pub set: ArtifactSet,
    /// One error per file that did not, in discovery order.
    pub errors: Vec<LoadError>,
}

fn yaml_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_owned()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            // `entry.file_type()` reads the kind the `read_dir` iterator
            // already carries; `Path::is_dir` would re-`stat` every entry.
            // It reports the ENTRY's own kind and does NOT follow symlinks
            // (<https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.file_type>),
            // so a symlink still costs the following stat — the seeded-defect
            // harness overlays the catalogue with symlinked directories, and
            // treating one as a leaf would silently truncate the walk.
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            let is_dir = if kind.is_symlink() {
                path.is_dir()
            } else {
                kind.is_dir()
            };
            if is_dir {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Every `<dir>/*/statement.json` under the party directory, path-sorted.
fn statement_files_under(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path().join("statement.json"))
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    files
}

/// Load one party statement (JSON, schema-validated like every artifact).
fn load_statement(
    path: &Path,
    validator: &jsonschema::Validator,
) -> Result<crate::party::Statement, LoadError> {
    let text = std::fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.to_owned(),
        source,
    })?;
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| LoadError::Model {
        path: path.to_owned(),
        message: format!("JSON: {e}"),
    })?;
    crate::load::validate_against(validator, &value, path)?;
    serde_json::from_value(value).map_err(|e| LoadError::Model {
        path: path.to_owned(),
        message: e.to_string(),
    })
}

/// Load one `kind: performance` case (its own schema family; the typed
/// model + invariants are the validation).
fn load_performance_case(path: &Path) -> Result<crate::perf::PerformanceCase, LoadError> {
    let value = crate::load::yaml_file_to_value(path)?;
    let case: crate::perf::PerformanceCase =
        serde_json::from_value(value).map_err(|e| LoadError::Model {
            path: path.to_owned(),
            message: e.to_string(),
        })?;
    case.check_invariants()
        .map_err(|message| LoadError::Model {
            path: path.to_owned(),
            message,
        })?;
    Ok(case)
}

/// The ten artifact-family JSON Schemas, compiled once per process.
///
/// The schemas are compile-time constants built by [`crate::schema`], so
/// compiling them on every [`load_root`] call was pure repeated work — and
/// `load_root` is the dominant per-invocation cost of the seeded-defect
/// battery, which calls it once per defect.
struct Schemas {
    /// `case-core.schema.json`.
    case: &'static jsonschema::Validator,
    /// `operation-binding.schema.json`.
    binding: &'static jsonschema::Validator,
    /// `outcomes.schema.json`.
    outcomes: &'static jsonschema::Validator,
    /// `selectors.schema.json`.
    selectors: &'static jsonschema::Validator,
    /// `capability-matrix.schema.json`.
    matrix: &'static jsonschema::Validator,
    /// `corpus-manifest.schema.json`.
    corpus: &'static jsonschema::Validator,
    /// `ambiguity-register.schema.json`.
    register: &'static jsonschema::Validator,
    /// `journey-catalogue.schema.json`.
    journeys: &'static jsonschema::Validator,
    /// `wire-surface.schema.json`.
    wire_surface: &'static jsonschema::Validator,
    /// `statement.schema.json`.
    statement: &'static jsonschema::Validator,
}

/// The process-wide compiled schema set.
///
/// A compilation failure is a defect in [`crate::schema`] itself, so it is
/// stored as the (name, message) pair [`LoadError::Schema`] is rebuilt from —
/// `LoadError` is not `Clone` (it carries `std::io::Error`), and the error is
/// deterministic, so re-raising it per call is exactly equivalent to the
/// previous per-call compile.
static SCHEMAS: LazyLock<Result<Schemas, (PathBuf, String)>> = LazyLock::new(|| {
    fn one(
        schema: &serde_json::Value,
        name: &'static str,
    ) -> Result<&'static jsonschema::Validator, (PathBuf, String)> {
        match compile_schema(schema, name) {
            Ok(v) => Ok(Box::leak(Box::new(v))),
            Err(LoadError::Schema { path, message }) => Err((path, message)),
            Err(other) => Err((PathBuf::from(name), other.to_string())),
        }
    }
    Ok(Schemas {
        case: one(&schema::case_core_schema(), "case-core.schema.json")?,
        binding: one(
            &schema::operation_binding_schema(),
            "operation-binding.schema.json",
        )?,
        outcomes: one(&schema::outcomes_schema(), "outcomes.schema.json")?,
        selectors: one(&schema::selectors_schema(), "selectors.schema.json")?,
        matrix: one(
            &schema::capability_matrix_schema(),
            "capability-matrix.schema.json",
        )?,
        corpus: one(
            &schema::corpus_manifest_schema(),
            "corpus-manifest.schema.json",
        )?,
        register: one(
            &schema::ambiguity_register_schema(),
            "ambiguity-register.schema.json",
        )?,
        journeys: one(
            &schema::journey_catalogue_schema(),
            "journey-catalogue.schema.json",
        )?,
        wire_surface: one(&schema::wire_surface_schema(), "wire-surface.schema.json")?,
        statement: one(&schema::statement_schema(), "statement.schema.json")?,
    })
});

/// Borrow the process-wide compiled schemas, re-raising a compilation defect.
fn compiled_schemas() -> Result<&'static Schemas, LoadError> {
    SCHEMAS
        .as_ref()
        .map_err(|(path, message)| LoadError::Schema {
            path: path.clone(),
            message: message.clone(),
        })
}

/// Load every artifact under `root`.
///
/// # Errors
/// Only on a schema-compilation defect in [`crate::schema`] itself — a bug
/// in this crate, not in the artifact tree. Tree problems come back as
/// [`Loaded::errors`].
#[expect(
    clippy::too_many_lines,
    reason = "one singleton-loading block per artifact family"
)]
pub fn load_root(root: &Path) -> Result<Loaded, LoadError> {
    let schemas = compiled_schemas()?;
    let Schemas {
        case: case_schema,
        binding: binding_schema,
        outcomes: outcomes_schema,
        selectors: selectors_schema,
        matrix: matrix_schema,
        corpus: corpus_schema,
        register: register_schema,
        journeys: journeys_schema,
        wire_surface: wire_surface_schema,
        statement: statement_schema,
    } = schemas;

    let mut loaded = Loaded::default();

    // The party statements live beside the artifact root, not inside it: a
    // claim is a submission document, while `root` is the published
    // catalogue. Swept anyway, because "a claim without cases" is a relation
    // between the two and no gate can see it from one side alone.
    if let Some(party_dir) = root.parent().map(|p| p.join("party")) {
        for path in statement_files_under(&party_dir) {
            match load_statement(&path, statement_schema) {
                Ok(statement) => loaded.set.parties.push((path, statement)),
                Err(e) => loaded.errors.push(e),
            }
        }
    }

    let performance_dir = root.join("schedule/performance");
    for path in yaml_files_under(&root.join("schedule")) {
        if path.starts_with(&performance_dir) {
            match load_performance_case(&path) {
                Ok(case) => loaded.set.performance.push((path, case)),
                Err(e) => loaded.errors.push(e),
            }
            continue;
        }
        match load_artifact::<CaseCore>(&path, case_schema) {
            Ok(case) => loaded.set.cases.push((path, case)),
            Err(e) => loaded.errors.push(e),
        }
    }
    for path in yaml_files_under(&root.join("bindings")) {
        match load_artifact::<OperationBinding>(&path, binding_schema) {
            Ok(binding) => loaded.set.bindings.push((path, binding)),
            Err(e) => loaded.errors.push(e),
        }
    }

    let mut singleton = |rel: &str, out: &mut dyn FnMut(PathBuf, &Path) -> Option<LoadError>| {
        let path = root.join(rel);
        if path.exists()
            && let Some(e) = out(path.clone(), &path)
        {
            loaded.errors.push(e);
        }
    };

    singleton(
        "vocab/outcomes.yaml",
        &mut |path, p| match load_artifact::<OutcomesVocab>(p, outcomes_schema) {
            Ok(v) => {
                loaded.set.outcomes = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "vocab/selectors.yaml",
        &mut |path, p| match load_artifact::<SelectorsVocab>(p, selectors_schema) {
            Ok(v) => {
                loaded.set.selectors = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "vocab/capability_matrix.yaml",
        &mut |path, p| match load_artifact::<CapabilityMatrix>(p, matrix_schema) {
            Ok(v) => {
                loaded.set.matrix = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "vocab/journey_catalogue.yaml",
        &mut |path, p| match load_artifact::<crate::perf::JourneyCatalogue>(p, journeys_schema) {
            Ok(v) => {
                loaded.set.journeys = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "corpus/MANIFEST.yaml",
        &mut |path, p| match load_artifact::<CorpusManifest>(p, corpus_schema) {
            Ok(v) => {
                loaded.set.corpus_dir = path.parent().map(Path::to_owned);
                loaded.set.corpus = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "registers/ambiguities.yaml",
        &mut |path, p| match load_artifact::<AmbiguityRegister>(p, register_schema) {
            Ok(v) => {
                loaded.set.register = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "vocab/wire_surface.yaml",
        &mut |path, p| match load_artifact::<WireSurface>(p, wire_surface_schema) {
            Ok(v) => {
                loaded.set.wire_surface = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );

    Ok(loaded)
}
