//! `WebTemplate` builder tests.
//!
//! * **smoke gate** — every `.opt` in the 91-file service corpus AND the full
//!   vendored Better `web-template-tests` set (63 templates) is exercised: each
//!   that the `openehr_its::opt14` parser can read MUST build a `WebTemplate`
//!   without panicking, and the tree root's `rmType` must round-trip the OPT's
//!   own root type. Templates that `opt14` cannot yet parse are reported as
//!   pre-existing P13 parser gaps (not a WebTemplate-builder failure).
//! * **insta goldens** — deterministic snapshots for representative templates.
//! * **targeted assertions** — ports of Better `BuilderTest`/`CodedTextTest`/
//!   `OrdinalWebTemplateInputBuilder`-style checks (coded-text `|code` + list,
//!   quantity `|magnitude`+`|unit`, ordinal integers, min/max, snake-cased ids,
//!   archetype/`atNNNN` predicates, sibling-id de-duplication).

use std::path::{Path, PathBuf};

use openehr_flat::build_web_template;
use openehr_flat::webtemplate::{WebTemplateInputType, WebTemplateNode};
use openehr_its::opt14;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corpus_dir() -> PathBuf {
    manifest_dir().join("../ehrbase/tests/resources/service")
}

fn better_fixtures_dir() -> PathBuf {
    manifest_dir().join("tests/fixtures/better")
}

/// Recursively collect every `*.opt` under `dir`.
fn opt_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(opt_files(&path));
        } else if path.extension().is_some_and(|e| e == "opt") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Parse an `OPT` and build a `WebTemplate`, returning any error as a string.
fn build_from_file(path: &Path) -> Result<openehr_flat::WebTemplate, String> {
    let xml = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    let opt = opt14::from_xml(&xml).map_err(|e| format!("opt14 parse: {e}"))?;
    build_web_template(&opt).map_err(|e| format!("build: {e}"))
}

fn find_node<'a>(
    node: &'a WebTemplateNode,
    pred: &dyn Fn(&WebTemplateNode) -> bool,
) -> Option<&'a WebTemplateNode> {
    if pred(node) {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node(child, pred) {
            return Some(found);
        }
    }
    None
}

fn for_each_node(node: &WebTemplateNode, f: &mut dyn FnMut(&WebTemplateNode)) {
    f(node);
    for child in &node.children {
        for_each_node(child, f);
    }
}

/// Assert that every parent's child ids are unique (Better numeric-suffix dedup).
fn assert_unique_sibling_ids(node: &WebTemplateNode) {
    let mut seen = std::collections::HashSet::new();
    for child in &node.children {
        assert!(
            seen.insert(child.id.clone()),
            "duplicate sibling id {:?} under {:?}",
            child.id,
            node.id
        );
        assert_unique_sibling_ids(child);
    }
}

// ── smoke gate ───────────────────────────────────────────────────────────────

#[test]
fn every_opt_builds_a_web_template() {
    let corpus = opt_files(&corpus_dir());
    let vendored = opt_files(&better_fixtures_dir());
    let corpus_count = corpus.len();
    let vendored_count = vendored.len();

    assert!(
        corpus_count >= 90,
        "expected the ~91-file service corpus, found {corpus_count} (dir: {})",
        corpus_dir().display()
    );
    assert!(
        vendored_count >= 60,
        "expected the full vendored Better set (~63), found {vendored_count}"
    );

    let mut build_failures = Vec::new();
    // opt14 (P13) parser gaps — reported, not a WebTemplate-builder failure.
    let mut parse_skips = Vec::new();
    let mut built = 0usize;
    let mut composition_roots = 0usize;

    for path in corpus.iter().chain(vendored.iter()) {
        let xml = match std::fs::read_to_string(path) {
            Ok(x) => x,
            Err(e) => {
                build_failures.push(format!("{}: read: {e}", path.display()));
                continue;
            }
        };
        let opt = match opt14::from_xml(&xml) {
            Ok(o) => o,
            Err(e) => {
                parse_skips.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let expected_root = opt.definition.rm_type_name.clone();
        match build_web_template(&opt) {
            Ok(wt) => {
                built += 1;
                if wt.tree.rm_type != expected_root {
                    build_failures.push(format!(
                        "{}: root rmType = {} (expected {}, the OPT root type)",
                        path.display(),
                        wt.tree.rm_type,
                        expected_root
                    ));
                }
                if wt.tree.rm_type == "COMPOSITION" {
                    composition_roots += 1;
                }
            }
            Err(e) => build_failures.push(format!("{}: {e}", path.display())),
        }
    }

    eprintln!(
        "web-template smoke: {corpus_count} corpus + {vendored_count} vendored | \
         built OK = {built} ({composition_roots} COMPOSITION-rooted), \
         opt14 parse-skipped = {}",
        parse_skips.len()
    );
    if !parse_skips.is_empty() {
        eprintln!(
            "opt14 (P13) could not parse {} template(s):\n  {}",
            parse_skips.len(),
            parse_skips.join("\n  ")
        );
    }

    // The gate: the WebTemplate builder must succeed on every parseable OPT.
    assert!(
        build_failures.is_empty(),
        "{} template(s) failed the WebTemplate builder:\n{}",
        build_failures.len(),
        build_failures.join("\n")
    );
    assert!(
        built >= 140,
        "expected most templates to build, only {built} did"
    );
    assert!(
        composition_roots >= 130,
        "expected most templates to be COMPOSITION-rooted, got {composition_roots}"
    );
}

// ── insta goldens ────────────────────────────────────────────────────────────

fn golden(name: &str) {
    let path = better_fixtures_dir().join(name);
    let wt = build_from_file(&path).unwrap_or_else(|e| panic!("build {name}: {e}"));
    // WebTemplate output is deterministic — no volatile fields to redact.
    insta::assert_json_snapshot!(name.replace(['.', ' '], "_"), wt);
}

#[test]
fn golden_demo_vitals() {
    golden("Demo Vitals.opt");
}

#[test]
fn golden_diagnosis() {
    golden("Diagnosis.opt");
}

#[test]
fn golden_medication_list() {
    golden("medication_list.opt");
}

// ── targeted assertions ──────────────────────────────────────────────────────

#[test]
fn demo_vitals_shapes() {
    let wt =
        build_from_file(&better_fixtures_dir().join("Demo Vitals.opt")).expect("build Demo Vitals");

    // Root is a COMPOSITION with the format version and a template id.
    assert_eq!(wt.tree.rm_type, "COMPOSITION");
    assert_eq!(wt.version, "2.3");
    assert!(!wt.template_id.is_empty());
    assert!(!wt.default_language.is_empty());
    assert!(wt.languages.contains(&wt.default_language));

    // Every node has a non-empty snake-cased id; occurrences are consistent
    // (max == -1 for unbounded, else >= min).
    let mut node_count = 0usize;
    for_each_node(&wt.tree, &mut |n| {
        node_count += 1;
        assert!(!n.id.is_empty(), "empty id on {}", n.aql_path);
        assert!(
            n.id.chars().all(|c| c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(c, '_' | '.' | '-')),
            "non-sanitized id {:?}",
            n.id
        );
        if n.max != -1 {
            let min = n.min.unwrap_or(0);
            assert!(n.max >= min, "max {} < min {min} at {}", n.max, n.aql_path);
        }
    });
    assert!(
        node_count > 5,
        "expected a populated tree, got {node_count} nodes"
    );

    // A quantity leaf carries |magnitude (DECIMAL) + |unit inputs (|unit singular).
    let quantity = find_node(&wt.tree, &|n| {
        n.inputs
            .iter()
            .any(|i| i.suffix.as_deref() == Some("magnitude"))
    })
    .expect("a DV_QUANTITY node with a |magnitude input");
    let suffixes: Vec<&str> = quantity
        .inputs
        .iter()
        .filter_map(|i| i.suffix.as_deref())
        .collect();
    assert!(
        suffixes.contains(&"magnitude"),
        "quantity inputs: {suffixes:?}"
    );
    assert!(
        suffixes.contains(&"unit"),
        "quantity inputs (|unit singular): {suffixes:?}"
    );

    // aql paths carry archetype/atNNNN predicates.
    assert!(
        find_node(&wt.tree, &|n| n.aql_path.contains("[at")).is_some(),
        "expected an aqlPath with an [atNNNN] predicate"
    );
    assert!(
        find_node(&wt.tree, &|n| n.aql_path.contains("[openEHR-EHR-")).is_some(),
        "expected an aqlPath with an [openEHR-EHR-...] archetype predicate"
    );
}

#[test]
fn coded_text_has_code_input_and_list() {
    // A CODED_TEXT leaf (with a local coded list) has a single CODED_TEXT input
    // suffixed `code` carrying a non-empty coded list (Better `CodedTextTest`).
    let wt =
        build_from_file(&better_fixtures_dir().join("Diagnosis.opt")).expect("build Diagnosis");
    let coded = find_node(&wt.tree, &|n| {
        n.inputs.iter().any(|i| {
            i.suffix.as_deref() == Some("code")
                && matches!(i.input_type, WebTemplateInputType::CodedText)
                && !i.list.is_empty()
        })
    })
    .expect("a coded-text node with a |code input + coded list");

    let code_input = coded
        .inputs
        .iter()
        .find(|i| i.suffix.as_deref() == Some("code"))
        .expect("code input");
    assert!(
        !code_input.list.is_empty(),
        "coded list should be populated"
    );
    for cv in &code_input.list {
        assert!(!cv.value.is_empty(), "coded value has a code");
    }
}

#[test]
fn ordinal_values_carry_ordinal_integers() {
    // DV_ORDINAL → a single CODED_TEXT input whose list entries carry `ordinal`
    // integers (Better `OrdinalWebTemplateInputBuilder`).
    let wt = build_from_file(&better_fixtures_dir().join("Testing Template N.opt"))
        .expect("build Testing Template N");
    let ordinal = find_node(&wt.tree, &|n| {
        n.inputs
            .iter()
            .any(|i| i.list.iter().any(|cv| cv.ordinal.is_some()))
    });
    assert!(
        ordinal.is_some(),
        "expected at least one DV_ORDINAL node with ordinal-tagged coded values"
    );
}

#[test]
fn ids_are_deduplicated_within_a_parent() {
    let wt = build_from_file(&better_fixtures_dir().join("Testing Template N.opt"))
        .expect("build Testing Template N");
    assert_unique_sibling_ids(&wt.tree);
}
