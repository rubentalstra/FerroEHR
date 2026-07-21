//! The loaded artifact set — one root directory laid out per the artifact
//! families:
//!
//! ```text
//! <root>/schedule/**/*.yaml        case cores
//! <root>/bindings/<its>/*.yaml     operation bindings
//! <root>/vocab/outcomes.yaml       outcome vocabulary
//! <root>/vocab/selectors.yaml      selector vocabulary + ignore-sets
//! <root>/vocab/capability_matrix.yaml
//! <root>/corpus/MANIFEST.yaml      corpus manifest
//! <root>/registers/ambiguities.yaml
//! ```
//!
//! Loading never fails fast: every file error becomes a finding, so one
//! validation run reports the whole tree.

use std::path::{Path, PathBuf};

use crate::load::{LoadError, compile_schema, load_artifact};
use crate::model::binding::OperationBinding;
use crate::model::capability::CapabilityMatrix;
use crate::model::case::CaseCore;
use crate::model::corpus::CorpusManifest;
use crate::model::register::AmbiguityRegister;
use crate::model::vocab_files::{OutcomesVocab, SelectorsVocab};
use crate::schema;

/// The fully-typed artifact set (files that failed to load are absent; their
/// errors travel alongside).
#[derive(Debug, Default)]
pub struct ArtifactSet {
    pub cases: Vec<(PathBuf, CaseCore)>,
    /// `kind: performance` cases (their own schema family; measured, not
    /// asserted).
    pub performance: Vec<(PathBuf, crate::perf::PerformanceCase)>,
    pub bindings: Vec<(PathBuf, OperationBinding)>,
    pub outcomes: Option<(PathBuf, OutcomesVocab)>,
    pub selectors: Option<(PathBuf, SelectorsVocab)>,
    pub matrix: Option<(PathBuf, CapabilityMatrix)>,
    pub corpus: Option<(PathBuf, CorpusManifest)>,
    pub register: Option<(PathBuf, AmbiguityRegister)>,
    /// The corpus manifest's directory (source paths resolve against it).
    pub corpus_dir: Option<PathBuf>,
}

/// A load pass over one artifact root.
#[derive(Debug, Default)]
pub struct Loaded {
    pub set: ArtifactSet,
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
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "yaml" || e == "yml") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
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

/// Load every artifact under `root`.
///
/// # Errors
/// Only on a schema-compilation defect in [`crate::schema`] itself — a bug
/// in this crate, not in the artifact tree. Tree problems come back as
/// [`Loaded::errors`].
pub fn load_root(root: &Path) -> Result<Loaded, LoadError> {
    let case_schema = compile_schema(&schema::case_core_schema(), "case-core.schema.json")?;
    let binding_schema = compile_schema(
        &schema::operation_binding_schema(),
        "operation-binding.schema.json",
    )?;
    let outcomes_schema = compile_schema(&schema::outcomes_schema(), "outcomes.schema.json")?;
    let selectors_schema = compile_schema(&schema::selectors_schema(), "selectors.schema.json")?;
    let matrix_schema = compile_schema(
        &schema::capability_matrix_schema(),
        "capability-matrix.schema.json",
    )?;
    let corpus_schema = compile_schema(
        &schema::corpus_manifest_schema(),
        "corpus-manifest.schema.json",
    )?;
    let register_schema = compile_schema(
        &schema::ambiguity_register_schema(),
        "ambiguity-register.schema.json",
    )?;

    let mut loaded = Loaded::default();

    let performance_dir = root.join("schedule/performance");
    for path in yaml_files_under(&root.join("schedule")) {
        if path.starts_with(&performance_dir) {
            match load_performance_case(&path) {
                Ok(case) => loaded.set.performance.push((path, case)),
                Err(e) => loaded.errors.push(e),
            }
            continue;
        }
        match load_artifact::<CaseCore>(&path, &case_schema) {
            Ok(case) => loaded.set.cases.push((path, case)),
            Err(e) => loaded.errors.push(e),
        }
    }
    for path in yaml_files_under(&root.join("bindings")) {
        match load_artifact::<OperationBinding>(&path, &binding_schema) {
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
        &mut |path, p| match load_artifact::<OutcomesVocab>(p, &outcomes_schema) {
            Ok(v) => {
                loaded.set.outcomes = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "vocab/selectors.yaml",
        &mut |path, p| match load_artifact::<SelectorsVocab>(p, &selectors_schema) {
            Ok(v) => {
                loaded.set.selectors = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "vocab/capability_matrix.yaml",
        &mut |path, p| match load_artifact::<CapabilityMatrix>(p, &matrix_schema) {
            Ok(v) => {
                loaded.set.matrix = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );
    singleton(
        "corpus/MANIFEST.yaml",
        &mut |path, p| match load_artifact::<CorpusManifest>(p, &corpus_schema) {
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
        &mut |path, p| match load_artifact::<AmbiguityRegister>(p, &register_schema) {
            Ok(v) => {
                loaded.set.register = Some((path, v));
                None
            }
            Err(e) => Some(e),
        },
    );

    Ok(loaded)
}
