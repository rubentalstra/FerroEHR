//! OPT 1.4 corpus gate (ADR-005): every vendored `.opt` operational template
//! must parse into the generated `opt14::OperationalTemplate` model without
//! error. The corpus lives with the `ehrbase` app tests; this crate reads it by
//! a workspace-relative path.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The OPT corpus dir (`crates/ehrbase/tests/resources/service`), resolved from
/// this crate's manifest dir.
fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../ehrbase/tests/resources/service")
}

/// Recursively collect every `*.opt` file under `dir`.
fn opt_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = match std::fs::read_dir(&d) {
            Ok(entries) => entries,
            Err(e) => panic!("read corpus dir {}: {e}", d.display()),
        };
        for path in entries.flatten().map(|e| e.path()) {
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "opt") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_opt_template_parses() {
    let files = opt_files(&corpus_dir());
    assert!(
        files.len() >= 90,
        "expected the full OPT corpus (~91 files), found {}",
        files.len()
    );

    let mut failures = Vec::new();
    let mut parsed = 0usize;
    for path in &files {
        let xml = std::fs::read_to_string(path).expect("read opt file");
        match openehr_its::opt14::from_xml(&xml) {
            Ok(_) => parsed += 1,
            Err(e) => failures.push((path.clone(), e.to_string())),
        }
    }

    if !failures.is_empty() {
        let mut msg = format!(
            "{}/{} OPT files parsed; {} failed:\n",
            parsed,
            files.len(),
            failures.len()
        );
        for (p, e) in &failures {
            let _ = writeln!(msg, "  - {}: {}", p.display(), e);
        }
        panic!("{msg}");
    }
    assert_eq!(parsed, files.len());
}

/// Spot-check that key envelope fields are actually populated (not merely that
/// parsing returns `Ok`), on a representative subset of the corpus.
#[test]
fn key_fields_populated() {
    let dir = corpus_dir();
    for rel in [
        "knowledge/IDCR Allergies List.v0.opt",
        "knowledge/non_unique_aql_paths.opt", // the ns2:-prefixed export
    ] {
        let path = dir.join(rel);
        let xml = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {rel}"));
        let opt = openehr_its::opt14::from_xml(&xml).unwrap_or_else(|e| panic!("parse {rel}: {e}"));
        assert!(
            !opt.template_id.value.is_empty(),
            "{rel}: template_id.value empty"
        );
        assert!(!opt.concept.is_empty(), "{rel}: concept empty");
        assert_eq!(
            opt.definition.rm_type_name, "COMPOSITION",
            "{rel}: definition.rm_type_name"
        );
        assert!(
            !opt.definition.attributes.is_empty(),
            "{rel}: definition has no attributes"
        );
    }
}
