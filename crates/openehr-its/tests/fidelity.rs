#![allow(clippy::doc_markdown)] // prose with spec/crate proper nouns
//! Interop fidelity gate — deserialize the real EHRbase / openEHR_SDK canonical
//! JSON corpus (`tests/vendor/`) into our **generated** `openehr-rm` types.
//!
//! This is the acceptance test that our generated RM actually reads real
//! openEHR data. Each corpus file is dispatched by its top-level `_type` to the
//! matching generated type; `rawdb_*` / array / `_type`-less fragments are not
//! canonical single-RM-object roots and are skipped with a recorded reason.
//!
//! NOTE: the corpus is RM 1.1.0-era while the generated types are RM 1.2.0, and
//! `OpenEhrType` deserialization is lenient (ignores unknown fields), so this
//! gate proves *readability*; a stricter lossless re-serialize round-trip is a
//! follow-up once the 1.1↔1.2 field drift is characterized.

use openehr_rm::prelude::{Composition, Contribution, EhrStatus, Folder, ItemTree};
use std::fs;
use std::path::{Path, PathBuf};

fn corpus_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vendor");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "json") {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

/// Deserialize `json` into the generated type named by `ty`, then re-serialize
/// (proving the value is well-formed on the way back out too).
fn deserialize_as(ty: &str, json: &str) -> Result<(), String> {
    macro_rules! roundtrip {
        ($T:ty) => {{
            let v: $T = serde_json::from_str(json).map_err(|e| e.to_string())?;
            serde_json::to_string(&v).map_err(|e| e.to_string())?;
            Ok(())
        }};
    }
    match ty {
        "COMPOSITION" => roundtrip!(Composition),
        "FOLDER" => roundtrip!(Folder),
        "EHR_STATUS" => roundtrip!(EhrStatus),
        "CONTRIBUTION" => roundtrip!(Contribution),
        "ITEM_TREE" => roundtrip!(ItemTree),
        other => Err(format!("no dispatch for top-level _type {other:?}")),
    }
}

#[test]
fn generated_rm_reads_the_openehr_sdk_corpus() {
    let mut ok = 0;
    let mut skipped = 0;
    let mut failures: Vec<(String, String)> = Vec::new();

    for path in corpus_files() {
        let txt = fs::read_to_string(&path).unwrap();
        let name = path
            .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vendor"))
            .unwrap_or(&path)
            .display()
            .to_string();
        // Only canonical single-RM-object roots (a top-level `_type`).
        let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&txt)
        else {
            skipped += 1; // arrays / non-objects (rawdb helpers)
            continue;
        };
        let Some(ty) = map.get("_type").and_then(|v| v.as_str()) else {
            skipped += 1; // _type-less fragments (rawdb / wrapped)
            continue;
        };
        match deserialize_as(ty, &txt) {
            Ok(()) => ok += 1,
            Err(e) => failures.push((name, e)),
        }
    }

    println!(
        "openEHR_SDK corpus: {ok} read OK, {skipped} skipped (non-canonical-root), {} failed",
        failures.len()
    );
    for (f, e) in failures.iter().take(30) {
        println!("\n--- FAILED: {f}\n  {e}");
    }
    assert!(ok > 0, "no corpus files were read");
    assert!(
        failures.is_empty(),
        "{} corpus file(s) failed to deserialize into the generated RM types",
        failures.len()
    );
}
