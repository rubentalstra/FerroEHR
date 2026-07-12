//! Web-template `id` (json-id) derivation and de-duplication.
//!
//! Mirrors Better `builder/id/WebTemplateIdBuilder.kt` +
//! `NumericSuffixIdDeduplicator.kt` + `WebTemplateConversionUtils
//! .getWebTemplatePathSegmentForName`:
//!
//! * base id = the node `name` (rubric), else `"value"` (when the parent is an
//!   ELEMENT and the node is not a `DV_INTERVAL`), else the last aql-path
//!   segment;
//! * sanitize: every char not in `[\p{Alphabetic}0-9_.-]` → `_`, lowercase,
//!   collapse repeated `_`, trim leading/trailing `_`; empty → `"id"`; a leading
//!   digit → prefixed with `"a"`;
//! * de-duplicate within the parent scope with a numeric suffix (`_2` is spelt
//!   as the bare `2`, then `3`, …);
//! * a polymorphic choice ELEMENT's `DV_*` children get a typed id
//!   (`quantity_value`, `coded_text_value`, `interval_of_quantity_value`) plus an
//!   `alt_json_id` (`value`/`value2`), coded-text ordered before text.

use std::collections::{HashMap, HashSet};

use super::model::WebTemplateNode;

/// Numeric-suffix de-duplicator, scoped per parent id (Better
/// `AbstractSuffixIdDeduplicator`, `MAX_SUFFIX = 100`).
#[derive(Default)]
struct Deduplicator {
    used: HashMap<String, HashSet<String>>,
}

/// Duplicate-suffix spelling. The STABLE Simplified Formats spec's worked example
/// (master02/master04 §"Node ID Generation Rules") maps a duplicate "Blood
/// Pressure" to `blood_pressure_1` — underscore separator, counting from `1`.
/// Better's `NumericSuffixIdDeduplicator` spells it `blood_pressure2` (no
/// separator, from `2`).
///
/// PORT NOTE (master02/master04 §"Node ID Generation Rules"): we keep Better's
/// `blood_pressure2` spelling as the default because a WebTemplate json-id is a
/// shared contract with existing Better/EHRbase clients and stored form
/// definitions — the spec's `_1` form is an *illustrative* example and interop
/// tooling universally emits the Better form (SPECITS-94 did not touch the dedup
/// form). Selecting the spec spelling by default flips the constants below.
// TODO(w3e-formats): G-9 remaining — adopt the spec `_1` spelling as the default
// once the `crates/openehr-flat/tests/` webtemplate snapshots are regenerated
// (`cargo insta`); the mechanism (separator + start) is parameterised here.
const DUP_SEP: &str = "";
const DUP_START: usize = 2;

impl Deduplicator {
    fn unique(&mut self, parent_id: &str, base: &str) -> String {
        let set = self.used.entry(parent_id.to_owned()).or_default();
        if set.contains(base) {
            let mut i = DUP_START;
            while i < 100 && set.contains(&format!("{base}{DUP_SEP}{i}")) {
                i += 1;
            }
            let candidate = format!("{base}{DUP_SEP}{i}");
            set.insert(candidate.clone());
            candidate
        } else {
            set.insert(base.to_owned());
            base.to_owned()
        }
    }
}

/// `getWebTemplatePathSegmentForName`: keep letters/digits/`_`/`.`/`-`, lowercase,
/// collapse `_`, trim `_`.
pub(crate) fn sanitize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_alphabetic() || ch.is_ascii_digit() || ch == '_' || ch == '.' || ch == '-' {
            out.extend(ch.to_lowercase());
        } else {
            out.push('_');
        }
    }
    // Collapse runs of '_' and trim leading/trailing '_'.
    let mut collapsed = String::with_capacity(out.len());
    let mut prev_underscore = false;
    for ch in out.chars() {
        if ch == '_' {
            if !prev_underscore {
                collapsed.push('_');
            }
            prev_underscore = true;
        } else {
            collapsed.push(ch);
            prev_underscore = false;
        }
    }
    collapsed.trim_matches('_').to_owned()
}

/// The polymorphic typed id for a `DV_*` choice child.
fn typed_id(rm_type: &str) -> String {
    if let Some(rest) = rm_type
        .strip_prefix("DV_INTERVAL<DV_")
        .and_then(|s| s.strip_suffix('>'))
    {
        format!("interval_of_{}_value", rest.to_lowercase())
    } else if let Some(rest) = rm_type.strip_prefix("DV_") {
        format!("{}_value", rest.to_lowercase())
    } else {
        format!("{}_value", rm_type.to_lowercase())
    }
}

/// Base id before de-duplication/normalization.
fn base_id(node: &WebTemplateNode, parent_rm_type: Option<&str>) -> String {
    let name = match &node.name {
        Some(n) if !n.trim().is_empty() => n.clone(),
        _ => {
            if parent_rm_type == Some("ELEMENT") && !node.rm_type.starts_with("DV_INTERVAL") {
                "value".to_owned()
            } else {
                last_path_segment(&node.aql_path)
            }
        }
    };
    sanitize(&name)
}

fn last_path_segment(path: &str) -> String {
    match path.rfind('/') {
        Some(i) => path[i + 1..].to_owned(),
        None => path.to_owned(),
    }
}

fn normalize_base(base: &str) -> String {
    if base.is_empty() {
        "id".to_owned()
    } else if base.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("a{base}")
    } else {
        base.to_owned()
    }
}

/// Reorder a choice ELEMENT's children so `DV_CODED_TEXT` precedes `DV_TEXT`
/// (Better `fixPolymorphicOrder`).
fn fix_polymorphic_order(children: &mut Vec<WebTemplateNode>) {
    let coded = children.iter().position(|c| c.rm_type == "DV_CODED_TEXT");
    let text = children.iter().position(|c| c.rm_type == "DV_TEXT");
    if let (Some(coded), Some(text)) = (coded, text)
        && coded > text
    {
        let node = children.remove(coded);
        children.insert(text, node);
    }
}

/// Assign json-ids across the whole tree.
pub(crate) fn build_ids(root: &mut WebTemplateNode) {
    let mut dedup = Deduplicator::default();
    assign(root, "", &mut dedup, None, None);
}

fn assign(
    node: &mut WebTemplateNode,
    parent_prefix: &str,
    dedup: &mut Deduplicator,
    forced_base: Option<String>,
    parent_rm_type: Option<&str>,
) {
    let raw_base = forced_base.unwrap_or_else(|| base_id(node, parent_rm_type));
    let base = normalize_base(&raw_base);
    let id = dedup.unique(parent_prefix, &base);
    node.full_id = format!("{parent_prefix}{id}");
    node.id = id;

    let is_choice = node.rm_type == "ELEMENT" && node.children.len() > 1;
    if is_choice {
        fix_polymorphic_order(&mut node.children);
    }
    let child_prefix = format!("{}/", node.full_id);
    let rm_type = node.rm_type.clone();
    for (index, child) in node.children.iter_mut().enumerate() {
        let fb = if is_choice && child.rm_type.starts_with("DV_") {
            child.alt_json_id = Some(if index > 0 {
                format!("value{}", index + 1)
            } else {
                "value".to_owned()
            });
            Some(typed_id(&child.rm_type))
        } else {
            None
        };
        assign(child, &child_prefix, dedup, fb, Some(&rm_type));
    }

    resolve_cardinalities(node);
    resolve_depends_on(node);
}

/// Fill each cardinality's `ids` from the child json-ids whose aql path is under
/// the cardinality attribute path; drop cardinalities that match nothing.
fn resolve_cardinalities(node: &mut WebTemplateNode) {
    if node.cardinalities.is_empty() {
        return;
    }
    let child_ids: Vec<(String, String)> = node
        .children
        .iter()
        .map(|c| (c.aql_path.clone(), c.id.clone()))
        .collect();
    node.cardinalities.retain_mut(|card| {
        let ids: Vec<String> = child_ids
            .iter()
            .filter(|(path, _)| path.starts_with(&card.path))
            .map(|(_, id)| id.clone())
            .collect();
        if ids.is_empty() {
            false
        } else {
            card.ids = Some(ids);
            true
        }
    });
}

/// Map each child's `depends_on` RM paths to sibling json-ids (Better
/// `updateDependsOn`): a dependency resolves to the ids of non-`inContext`
/// siblings whose aql path starts with the dependency path.
fn resolve_depends_on(node: &mut WebTemplateNode) {
    let siblings: Vec<(String, String, bool)> = node
        .children
        .iter()
        .map(|c| (c.aql_path.clone(), c.id.clone(), c.in_context == Some(true)))
        .collect();
    for child in &mut node.children {
        let Some(paths) = child.depends_on.take() else {
            continue;
        };
        let mut ids: Vec<String> = Vec::new();
        for path in &paths {
            for (spath, sid, in_context) in &siblings {
                if !in_context && spath.starts_with(path) && !ids.contains(sid) {
                    ids.push(sid.clone());
                }
            }
        }
        child.depends_on = if ids.is_empty() { None } else { Some(ids) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize("Systolic"), "systolic");
        assert_eq!(sanitize("Blood pressure"), "blood_pressure");
        assert_eq!(sanitize("  a / b  "), "a_b");
        assert_eq!(sanitize("keeps.dot-dash"), "keeps.dot-dash");
        assert_eq!(sanitize("__weird__name__"), "weird_name");
    }

    #[test]
    fn normalizes_edge_cases() {
        assert_eq!(normalize_base(""), "id");
        assert_eq!(normalize_base("2nd"), "a2nd");
        assert_eq!(normalize_base("ok"), "ok");
    }

    #[test]
    fn dedups_with_numeric_suffix() {
        let mut d = Deduplicator::default();
        assert_eq!(d.unique("root", "value"), "value");
        assert_eq!(d.unique("root", "value"), "value2");
        assert_eq!(d.unique("root", "value"), "value3");
        // Different parent scope is independent.
        assert_eq!(d.unique("other", "value"), "value");
    }

    #[test]
    fn typed_ids_for_polymorphic_values() {
        assert_eq!(typed_id("DV_QUANTITY"), "quantity_value");
        assert_eq!(typed_id("DV_CODED_TEXT"), "coded_text_value");
        assert_eq!(
            typed_id("DV_INTERVAL<DV_QUANTITY>"),
            "interval_of_quantity_value"
        );
    }
}
