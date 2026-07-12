//! XML round-trip fidelity gate: for every composition in the openEHR
//! corpus, RM → XML → RM → XML must be stable, proving the generated `ToXml` and
//! `FromXml` impls are mutually consistent on real data.
use openehr_its::xml::{from_xml, to_canonical_xml};
use openehr_rm::prelude::Composition;
use std::path::Path;

fn corpus_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vendor/openehr_sdk/composition/canonical_json")
}

#[test]
fn composition_xml_round_trips() {
    // Fixtures that are deliberately invalid or raw-DB/flat shapes (excluded from
    // the JSON gate too — they don't deserialize as a canonical COMPOSITION).
    let exclude = [
        "invalid",
        "ips_invalid",
        "rawdb_composition",
        "rawdb_composition_history",
        "rawdb_composition_observation_event",
        "rawdb_composition_observation_event_item",
        "flat_",
    ];
    let (mut ok, mut skipped) = (0, 0);
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(corpus_dir()).expect("corpus dir") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
        if exclude.iter().any(|e| stem.contains(e)) {
            skipped += 1;
            continue;
        }
        let json = std::fs::read_to_string(&path).unwrap();
        let Ok(compo) = serde_json::from_str::<Composition>(&json) else {
            skipped += 1; // not a canonical composition
            continue;
        };
        let xml1 = to_canonical_xml(&compo, "composition").expect("serialize 1");
        match from_xml::<Composition>(&xml1) {
            Ok(compo2) => {
                let xml2 = to_canonical_xml(&compo2, "composition").expect("serialize 2");
                if xml1 == xml2 {
                    ok += 1;
                } else {
                    failures.push(format!("{stem}: round-trip not stable"));
                }
            }
            Err(e) => failures.push(format!("{stem}: parse failed: {e}")),
        }
    }
    eprintln!(
        "xml round-trip: {ok} ok, {skipped} skipped, {} failed",
        failures.len()
    );
    assert!(failures.is_empty(), "failures:\n{}", failures.join("\n"));
    assert!(
        ok > 10,
        "expected many compositions to round-trip, got {ok}"
    );
}
