//! Parse the official openEHR AQL worked-example corpus (vendored under
//! `vendor/examples/`, from specifications-QUERY). Every `----` listing block
//! that is standard **AQL 1.1.0** must parse.
//!
//! Some blocks are intentionally NOT standard AQL and are excluded with a
//! recorded reason — this is faithful to the grammar, not a gap:
//! - the spec doc itself states *"the ADL expressions on the right-hand side of
//!   the `matches` operator are in ADL 1.4 format"* — those `matches { TYPE
//!   matches {…} }` / `{|range|}` blocks are cADL, not the AQL `matches`
//!   value-list operand;
//! - `NOT IN (subquery)` is a SQL-style construct absent from `AqlParser.g4`.

use std::fs;
use std::path::Path;

/// Classify a block that is not standard AQL 1.1.0, with the reason. `None`
/// means it should parse as standard AQL.
fn out_of_grammar_reason(q: &str) -> Option<&'static str> {
    // ADL 1.4 cADL on the matches RHS: `{|range|}`, or `matches { TYPE matches …}`.
    let adl_range = q.contains("{|") || q.contains("|}") || q.contains("|>") || q.contains("|<");
    let nested_typed_matches = {
        // a `matches {` whose operand opens with an UPPER_CASE RM/cADL type.
        let mut hit = false;
        for (i, _) in q.match_indices("matches") {
            let rest = q[i + "matches".len()..].trim_start();
            let rest = rest.strip_prefix('{').map_or("", str::trim_start);
            if rest.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && rest.split_whitespace().nth(1) == Some("matches")
            {
                hit = true;
                break;
            }
        }
        hit
    };
    if adl_range || nested_typed_matches {
        return Some("ADL 1.4 cADL on the `matches` RHS (illustrative, not AQL)");
    }
    let lower = q.to_lowercase();
    if lower.contains("not in") || lower.contains(" in (") || lower.contains(" in(") {
        return Some("SQL-style IN/subquery — not in the AQL 1.1.0 grammar");
    }
    None
}

/// Extract the contents of `AsciiDoc` `----` listing blocks that contain SELECT.
fn select_blocks(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    let mut buf = String::new();
    for line in doc.lines() {
        if line.trim() == "----" {
            if in_block {
                if buf.to_uppercase().contains("SELECT") {
                    out.push(std::mem::take(&mut buf));
                } else {
                    buf.clear();
                }
                in_block = false;
            } else {
                in_block = true;
            }
        } else if in_block {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    out
}

#[test]
fn official_aql_corpus_parses() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/examples");
    let mut total = 0;
    let mut parsed = 0;
    let mut excluded: Vec<&'static str> = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();

    let mut files: Vec<_> = fs::read_dir(&dir)
        .expect("examples dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "adoc"))
        .collect();
    files.sort();

    for path in files {
        let doc = fs::read_to_string(&path).unwrap();
        for q in select_blocks(&doc) {
            total += 1;
            if let Some(reason) = out_of_grammar_reason(&q) {
                excluded.push(reason);
                continue;
            }
            match openehr_query::parser::parse_str(&q) {
                Ok(_) => parsed += 1,
                Err(e) => failures.push((q.clone(), e)),
            }
        }
    }

    println!(
        "AQL corpus: {total} SELECT blocks — {parsed} parsed, {} out-of-grammar excluded, {} failed",
        excluded.len(),
        failures.len()
    );
    for r in &excluded {
        println!("  excluded: {r}");
    }
    for (q, e) in &failures {
        println!("\n--- FAILED (standard AQL that did not parse) ---\n{q}\n  error: {e}");
    }
    assert!(
        failures.is_empty(),
        "{} standard-AQL example(s) failed to parse",
        failures.len()
    );
    // Every standard-AQL block parsed, and the whole official corpus is
    // accounted for (parsed + excluded == total).
    assert!(parsed > 0, "no standard-AQL examples found");
    assert_eq!(
        parsed + excluded.len(),
        total,
        "some blocks unaccounted for"
    );
}
