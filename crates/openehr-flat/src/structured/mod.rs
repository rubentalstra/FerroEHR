//! STRUCTURED (structSDT) `RM ⇄ STRUCTURED` conversion.
//!
//! Better's `converter/structured/FlatToStructuredConverter` shows that
//! **flat ⇄ structured is a pure, WebTemplate-independent nesting transform**:
//! a flat map of `path[:index][|suffix]` keys deterministically nests into the
//! structured tree and back. This module implements that transform
//! ([`flat_to_structured`] / [`structured_to_flat`]) and composes it with the
//! FLAT converter ([`super::flat`]):
//!
//! * [`to_structured`] = `flat_to_structured(to_flat(composition, wt))`
//! * [`from_structured`] = `from_flat(structured_to_flat(structured), wt)`
//!
//! ## The structured shape (matching Better `RawToStructuredConverter`)
//!
//! The root maps the template json-id to an **object** (and `ctx` to an object);
//! every level below is an **array of objects** (`"vitals": [ { … } ]`), a
//! node's datum parts live as `"|suffix"` keys (a bare value uses the `""` key),
//! and a bare-only leaf is an array of scalars (`"note": ["hi"]`). Container
//! array position encodes the FLAT `:index`.
//!
//! ## Inversion
//!
//! `structured_to_flat` re-emits `:index` for an array element **only when the
//! array has more than one element** — a single-instance node needs no index
//! (the FLAT `from_flat` groups by index, so this is RM-lossless). Consequently
//! `flat_to_structured(structured_to_flat(s)) == s` is exact (the structured
//! round-trip); `structured_to_flat(flat_to_structured(f))` equals `f` up to
//! that single-occurrence `:0` normalisation.

use serde_json::{Map, Value};

use crate::FlatError;
use crate::webtemplate::WebTemplate;

mod entry;

use entry::Entry;

/// Convert a canonical-JSON composition to STRUCTURED, driven by `wt`.
///
/// # Errors
/// Propagates [`FlatError`] from the underlying FLAT conversion.
pub fn to_structured(composition: &Value, wt: &WebTemplate) -> Result<Value, FlatError> {
    let flat = super::flat::to_flat(composition, wt)?;
    let map: Map<String, Value> = flat.into_iter().collect();
    Ok(flat_to_structured(&map))
}

/// Convert a STRUCTURED composition to canonical-JSON, driven by `wt`.
///
/// # Errors
/// Propagates [`FlatError`] from the underlying FLAT conversion.
pub fn from_structured(structured: &Value, wt: &WebTemplate) -> Result<Value, FlatError> {
    let flat = structured_to_flat(structured);
    super::flat::from_flat(&flat, wt)
}

// ── flat → structured ──────────────────────────────────────────────────────

/// The `ctx` namespace key.
const CTX: &str = "ctx";

/// Nest a flat `path[:index][|suffix]` map into the STRUCTURED tree.
///
/// Pure and WebTemplate-independent (Better `FlatToStructuredConverter`).
#[must_use]
pub fn flat_to_structured(flat: &Map<String, Value>) -> Value {
    let entries: Vec<Entry> = flat
        .iter()
        .filter(|(_, v)| is_not_empty(v))
        .map(|(k, v)| Entry::parse(k, v.clone()))
        .collect();
    Value::Object(build_object(&entries, 0))
}

fn is_not_empty(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

/// Group entries by name, preserving first-seen order.
fn group_by_name(entries: &[Entry]) -> Vec<(String, Vec<Entry>)> {
    let mut out: Vec<(String, Vec<Entry>)> = Vec::new();
    for e in entries {
        if let Some(slot) = out.iter_mut().find(|(n, _)| n == &e.name) {
            slot.1.push(e.clone());
        } else {
            out.push((e.name.clone(), vec![e.clone()]));
        }
    }
    out
}

/// The child entries of a group (Better `mapNotNull { it.child }`).
fn children_of(group: &[Entry]) -> Vec<Entry> {
    group
        .iter()
        .filter_map(|e| e.child.as_deref().cloned())
        .collect()
}

fn build_object(entries: &[Entry], depth: usize) -> Map<String, Value> {
    let mut node = Map::new();
    for (name, group) in group_by_name(entries) {
        if name == CTX {
            node.insert(CTX.to_owned(), build_ctx(&children_of(&group)));
        } else if group.iter().all(|e| e.child.is_none()) {
            // Leaf group.
            if group.len() == 1 && (name.starts_with('|') || name.is_empty()) {
                node.insert(name, group[0].value.clone().unwrap_or(Value::Null));
            } else {
                let mut g = group.clone();
                g.sort_by_key(|e| e.order);
                node.insert(
                    name,
                    Value::Array(
                        g.iter()
                            .map(|e| e.value.clone().unwrap_or(Value::Null))
                            .collect(),
                    ),
                );
            }
        } else if depth == 0 {
            // The root node maps to a single object.
            node.insert(
                name,
                Value::Object(build_object(&children_of(&group), depth + 1)),
            );
        } else {
            // A repeating container: one array element per :index (order).
            let mut orders: Vec<usize> = group.iter().map(|e| e.order).collect();
            orders.sort_unstable();
            orders.dedup();
            let arr: Vec<Value> = orders
                .into_iter()
                .map(|ord| {
                    let og: Vec<Entry> = group.iter().filter(|e| e.order == ord).cloned().collect();
                    let wrapped = convert_entries(&og);
                    Value::Object(build_object(&children_of(&wrapped), depth + 1))
                })
                .collect();
            node.insert(name, Value::Array(arr));
        }
    }
    node
}

/// Wrap bare (childless) entries in a `""`-named child so a value passed without
/// a suffix nests alongside its suffixed siblings (Better `convertEntries`).
fn convert_entries(entries: &[Entry]) -> Vec<Entry> {
    entries
        .iter()
        .map(|e| {
            if e.child.is_none() {
                Entry {
                    name: e.name.clone(),
                    order: e.order,
                    indexed: e.indexed,
                    child: Some(Box::new(Entry {
                        name: String::new(),
                        order: 0,
                        indexed: false,
                        child: None,
                        value: e.value.clone(),
                    })),
                    value: None,
                }
            } else {
                e.clone()
            }
        })
        .collect()
}

/// Build the `ctx` object. Our `ctx/…` keys are shallow — `ctx/name` (scalar),
/// `ctx/name|suffix` (object of `|suffix` parts), or `ctx/name:index` (array) —
/// so the transform is a direct, lossless nesting (Better collapses single
/// `:index` arrays to scalars; we keep them arrays so the round-trip is exact).
fn build_ctx(children: &[Entry]) -> Value {
    let mut node = Map::new();
    for (name, group) in group_by_name(children) {
        if group.iter().any(|e| e.child.is_some()) {
            // Suffixed: `ctx/name|suffix` → object of `|suffix` keys.
            let mut obj = Map::new();
            for e in &group {
                if let Some(child) = &e.child {
                    obj.insert(
                        child.name.clone(),
                        child.value.clone().unwrap_or(Value::Null),
                    );
                }
            }
            node.insert(name, Value::Object(obj));
        } else if group.len() == 1 && group[0].order == 0 && !group[0].indexed {
            // `ctx/name` → scalar.
            node.insert(name, group[0].value.clone().unwrap_or(Value::Null));
        } else {
            // `ctx/name:index` → array of scalars.
            let mut g = group.clone();
            g.sort_by_key(|e| e.order);
            node.insert(
                name,
                Value::Array(
                    g.iter()
                        .map(|e| e.value.clone().unwrap_or(Value::Null))
                        .collect(),
                ),
            );
        }
    }
    Value::Object(node)
}

// ── structured → flat ──────────────────────────────────────────────────────

/// Flatten a STRUCTURED tree back into a flat `path[:index][|suffix]` map.
///
/// The exact inverse of [`flat_to_structured`] up to single-occurrence index
/// normalisation (see the module docs).
#[must_use]
pub fn structured_to_flat(structured: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    if let Value::Object(root) = structured {
        for (key, val) in root {
            if key == CTX {
                flatten_ctx(val, &mut out);
            } else if let Value::Object(_) = val {
                // The template root node maps to an object; recurse its content.
                flatten_node(val, key, &mut out);
            }
        }
    }
    out
}

/// Flatten a structured node's inner object at `prefix`.
fn flatten_node(node: &Value, prefix: &str, out: &mut Map<String, Value>) {
    let Value::Object(m) = node else { return };
    for (key, val) in m {
        if key.starts_with('|') {
            // A datum suffix part.
            out.insert(format!("{prefix}{key}"), val.clone());
        } else if key.is_empty() {
            // The bare value part.
            out.insert(prefix.to_owned(), val.clone());
        } else if let Value::Array(arr) = val {
            let multi = arr.len() > 1;
            for (i, el) in arr.iter().enumerate() {
                let child_prefix = if multi {
                    format!("{prefix}/{key}:{i}")
                } else {
                    format!("{prefix}/{key}")
                };
                if let Value::Object(_) = el {
                    flatten_node(el, &child_prefix, out);
                } else {
                    // A bare-only leaf (array of scalars).
                    out.insert(child_prefix, el.clone());
                }
            }
        }
    }
}

/// Flatten the `ctx` object back into `ctx/…` keys (inverse of [`build_ctx`]).
fn flatten_ctx(ctx: &Value, out: &mut Map<String, Value>) {
    let Value::Object(m) = ctx else { return };
    for (name, val) in m {
        match val {
            Value::Object(obj) => {
                // `ctx/name|suffix` parts.
                for (suffix, v) in obj {
                    out.insert(format!("{CTX}/{name}{suffix}"), v.clone());
                }
            }
            Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    out.insert(format!("{CTX}/{name}:{i}"), v.clone());
                }
            }
            other => {
                out.insert(format!("{CTX}/{name}"), other.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn nests_and_flattens_a_leaf() {
        let mut flat = Map::new();
        flat.insert("t/vitals/systolic|magnitude".into(), json!(120));
        flat.insert("t/vitals/systolic|unit".into(), json!("mmHg"));
        flat.insert("t/note".into(), json!("hi"));
        let s = flat_to_structured(&flat);
        // structured: { t: { vitals: [ { systolic: [ {|magnitude,|unit} ] } ], note: ["hi"] } }
        assert_eq!(
            s.pointer("/t/vitals/0/systolic/0/|magnitude"),
            Some(&json!(120))
        );
        assert_eq!(s.pointer("/t/note/0"), Some(&json!("hi")));
        // round-trip back to flat is exact.
        let back = structured_to_flat(&s);
        assert_eq!(back, flat);
    }

    #[test]
    fn indexes_repeating_siblings() {
        let mut flat = Map::new();
        flat.insert("t/dx:0/name".into(), json!("a"));
        flat.insert("t/dx:1/name".into(), json!("b"));
        let s = flat_to_structured(&flat);
        assert_eq!(s.pointer("/t/dx/0/name/0"), Some(&json!("a")));
        assert_eq!(s.pointer("/t/dx/1/name/0"), Some(&json!("b")));
        let back = structured_to_flat(&s);
        assert_eq!(back, flat);
    }

    #[test]
    fn ctx_round_trips() {
        let mut flat = Map::new();
        flat.insert("ctx/language".into(), json!("en"));
        flat.insert("ctx/setting|code".into(), json!("238"));
        flat.insert("ctx/setting|value".into(), json!("other care"));
        flat.insert("ctx/participation_name:0".into(), json!("Dr X"));
        let s = flat_to_structured(&flat);
        assert_eq!(s.pointer("/ctx/language"), Some(&json!("en")));
        assert_eq!(s.pointer("/ctx/setting/|code"), Some(&json!("238")));
        let back = structured_to_flat(&s);
        assert_eq!(back, flat);
    }
}
