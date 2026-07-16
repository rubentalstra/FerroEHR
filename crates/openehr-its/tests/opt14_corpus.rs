//! OPT 1.4 corpus gate: every vendored `.opt` operational template
//! must parse into the generated `opt14::OperationalTemplate` model without
//! error. The corpus lives with the `ehrbase` app tests; this crate reads it by
//! a workspace-relative path.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The OPT corpus dir (`app/ehrbase/tests/resources/service`), resolved from
/// this crate's manifest dir.
fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/ehrbase/tests/resources/service")
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

/// Model-level losslessness gate: parse → `ToXml` → re-parse must be
/// structurally stable over the whole corpus. This is what actually exercises
/// the generated `ToXml` impls (storage/GET serve verbatim XML, so nothing else
/// does) and asserts the typed model loses nothing it captured on the way back
/// out — `xsi:type` dispatch, interval defaults, dictionary groups, and the
/// `T_CONSTRAINT` overlay all round-trip.
#[test]
fn every_opt_template_round_trips() {
    let files = opt_files(&corpus_dir());
    assert!(files.len() >= 90, "expected the full OPT corpus");

    let mut failures = Vec::new();
    for path in &files {
        let xml = std::fs::read_to_string(path).expect("read opt file");
        let opt = match openehr_its::opt14::from_xml(&xml) {
            Ok(o) => o,
            Err(e) => {
                failures.push((path.clone(), format!("first parse: {e}")));
                continue;
            }
        };
        let re_xml = match openehr_its::opt14::to_xml(&opt) {
            Ok(x) => x,
            Err(e) => {
                failures.push((path.clone(), format!("to_xml: {e}")));
                continue;
            }
        };
        match openehr_its::opt14::from_xml(&re_xml) {
            Ok(re_opt) => {
                if re_opt != opt {
                    failures.push((path.clone(), "re-parse != first parse".to_string()));
                }
            }
            Err(e) => failures.push((path.clone(), format!("re-parse: {e}"))),
        }
    }

    if !failures.is_empty() {
        let mut msg = format!("{} OPT files failed the round-trip gate:\n", failures.len());
        for (p, e) in &failures {
            let _ = writeln!(msg, "  - {}: {}", p.display(), e);
        }
        panic!("{msg}");
    }
}

/// `StringDictionaryItem` groups are XSD ordered sequences; the model must
/// preserve document order (`IndexMap`, F-09-05). `IndexMap`'s `PartialEq` is
/// order-insensitive, so the round-trip gate above cannot see reordering —
/// this asserts key *order* survives serialize → re-parse explicitly, on a
/// fixture whose document order is **non-alphabetical** (`text` before
/// `description`), which an alphabetically-sorted container would invert.
#[test]
fn string_dictionary_order_preserved() {
    use openehr_its::opt14::{ArchetypeTerm, CAttribute, CObject};

    /// Collect every inline `C_ARCHETYPE_ROOT` term definition in the tree.
    fn collect_terms<'a>(obj: &'a CObject, out: &mut Vec<&'a ArchetypeTerm>) {
        let attrs: &[CAttribute] = match obj {
            CObject::CArchetypeRoot(r) => {
                out.extend(&r.term_definitions);
                &r.attributes
            }
            CObject::CComplexObject(c) => &c.attributes,
            CObject::TComplexObject(t) => &t.attributes,
            _ => return,
        };
        for a in attrs {
            let children = match a {
                CAttribute::CSingleAttribute(s) => &s.children,
                CAttribute::CMultipleAttribute(m) => &m.children,
            };
            for c in children {
                collect_terms(c, out);
            }
        }
    }

    fn text_first_orders(
        opt: &openehr_its::opt14::OperationalTemplate,
    ) -> Vec<(String, Vec<String>)> {
        let mut terms: Vec<&ArchetypeTerm> = opt.definition.term_definitions.iter().collect();
        for a in &opt.definition.attributes {
            let children = match a {
                CAttribute::CSingleAttribute(s) => &s.children,
                CAttribute::CMultipleAttribute(m) => &m.children,
            };
            for c in children {
                collect_terms(c, &mut terms);
            }
        }
        terms
            .iter()
            .filter(|t| t.items.keys().collect::<Vec<_>>() == ["text", "description"])
            .map(|t| (t.code.clone(), t.items.keys().cloned().collect()))
            .collect()
    }

    let dir = corpus_dir();
    let path = dir.join("knowledge/operational_templates/Generic Laboratory Test Report.v0.opt");
    let xml = std::fs::read_to_string(&path).expect("read opt");
    let opt = openehr_its::opt14::from_xml(&xml).expect("parse");

    // The fixture carries terms whose document order is `text` then
    // `description`; a sorted container would yield none of these.
    let orders = text_first_orders(&opt);
    assert!(
        !orders.is_empty(),
        "expected at least one term in document order (text, description) — \
         a sorted map container would have inverted them all"
    );

    let re_xml = openehr_its::opt14::to_xml(&opt).expect("to_xml");
    let re_opt = openehr_its::opt14::from_xml(&re_xml).expect("re-parse");
    assert_eq!(
        text_first_orders(&re_opt),
        orders,
        "dictionary key order changed across the round-trip"
    );
}

/// `T_CONSTRAINT` (`<constraints>`) carries node `default_value` overlays
///; assert they are parsed into the typed model, not skipped, for a
/// corpus file known to carry one.
#[test]
fn t_constraint_default_values_parsed() {
    let dir = corpus_dir();
    let path = dir.join("knowledge/IDCR - Adverse Reaction List.v1.opt");
    let xml = std::fs::read_to_string(&path).expect("read opt");
    let opt = openehr_its::opt14::from_xml(&xml).expect("parse");

    let tc = opt
        .constraints
        .as_ref()
        .expect("template has a <constraints> (T_CONSTRAINT) block");
    assert!(!tc.attributes.is_empty(), "T_CONSTRAINT.attributes empty");
    let attr = &tc.attributes[0];
    assert_eq!(attr.rm_attribute_name, "value");
    assert!(
        attr.differential_path
            .starts_with("[openEHR-EHR-COMPOSITION.adverse_reaction_list.v1]"),
        "differential_path not parsed: {:?}",
        attr.differential_path
    );
    let child = attr.children.first().expect("T_ATTRIBUTE.children");
    assert_eq!(child.rm_type_name, "DV_TEXT");
    let dv = child.default_value.as_ref().expect("default_value parsed");
    // The overlay's DV_TEXT default must carry its value through
    // (`DV_TEXT` is a polymorphic slot: DataValue::DvText(DvText::DvText(data))).
    let openehr_rm::prelude::DataValue::DvText(openehr_rm::prelude::DvText::DvText(text)) = dv
    else {
        panic!("default_value should be a plain DV_TEXT, got {dv:?}");
    };
    assert_eq!(text.value, "No history of allergies / adverse reactions");
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
