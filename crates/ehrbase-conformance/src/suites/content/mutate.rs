//! Typed fixture mutators (design §2.2a): the upstream
//! `composition_validation_lib` catalogue re-expressed as Rust over
//! `serde_json::Value`, so content-chapter variants are generated from the
//! vendored base fixtures instead of hand-maintaining hundreds of static files.
//!
//! Two mutators: set a high-level field to `Exist`/`NotExist`/`Invalid`, and
//! pad/trim a JSON array to a target count.

use serde_json::Value;

/// How to mutate a field (the upstream `exist`/`not_exist`/`invalid` states).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldState {
    /// Ensure the field is present (a valid placeholder value if absent).
    Exist,
    /// Remove the field.
    NotExist,
    /// Corrupt the field so validation must reject it (here: break its `_type`).
    Invalid,
}

/// Apply `state` to `object[field]`.
pub fn mutate_field(object: &mut Value, field: &str, state: FieldState) {
    let Value::Object(map) = object else { return };
    match state {
        FieldState::NotExist => {
            map.remove(field);
        }
        FieldState::Exist => {
            map.entry(field)
                .or_insert_with(|| Value::String("placeholder".to_owned()));
        }
        FieldState::Invalid => {
            if let Some(Value::Object(inner)) = map.get_mut(field) {
                inner.insert("_type".to_owned(), Value::String("__INVALID__".to_owned()));
            } else {
                map.insert(field.to_owned(), Value::String("__INVALID__".to_owned()));
            }
        }
    }
}

/// Pad or trim `object[array_field]` to exactly `count` items (padding by cloning
/// the last element, trimming from the end). No-op if the field is not an array.
pub fn set_array_count(object: &mut Value, array_field: &str, count: usize) {
    let Some(Value::Array(items)) = object.get_mut(array_field) else {
        return;
    };
    if items.len() > count {
        items.truncate(count);
    } else {
        while items.len() < count {
            match items.last().cloned() {
                Some(last) => items.push(last),
                None => break,
            }
        }
    }
}

/// Set `COMPOSITION.category` to a coded value (openEHR terminology group 13):
/// `433` = `event`, `431` = `persistent` (the `Category_validity` RM invariant
/// keys context presence off this). Both `value` and `defining_code.code_string`
/// are updated so the wire object is internally consistent.
pub fn set_category(comp: &mut Value, code: &str, value: &str) {
    if let Value::Object(map) = comp {
        map.insert(
            "category".to_owned(),
            serde_json::json!({
                "_type": "DV_CODED_TEXT",
                "value": value,
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": code,
                }
            }),
        );
    }
}

/// Remove `COMPOSITION.context` (the "no context" data set).
pub fn remove_context(comp: &mut Value) {
    if let Value::Object(map) = comp {
        map.remove("context");
    }
}

/// Whether `COMPOSITION.context` is present.
#[must_use]
pub fn has_context(comp: &Value) -> bool {
    comp.get("context").is_some_and(|c| !c.is_null())
}

/// A depth-first search for the first node (object) whose `_type` equals `ty`,
/// returning a mutable reference. Arrays and object values are traversed in
/// document order; the outermost matching node wins.
pub fn first_node_mut<'a>(value: &'a mut Value, ty: &str) -> Option<&'a mut Value> {
    // Check the node itself first (pre-order), then descend.
    if value.get("_type").and_then(Value::as_str) == Some(ty) {
        return Some(value);
    }
    match value {
        Value::Object(map) => {
            for (_k, v) in map.iter_mut() {
                if let Some(found) = first_node_mut(v, ty) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => {
            for v in items.iter_mut() {
                if let Some(found) = first_node_mut(v, ty) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// Whether a node of `_type == ty` exists anywhere in `value`.
#[must_use]
pub fn contains_node(value: &Value, ty: &str) -> bool {
    if value.get("_type").and_then(Value::as_str) == Some(ty) {
        return true;
    }
    match value {
        Value::Object(map) => map.values().any(|v| contains_node(v, ty)),
        Value::Array(items) => items.iter().any(|v| contains_node(v, ty)),
        _ => false,
    }
}

/// Set a scalar field on an object node (creating/overwriting it).
pub fn set_field(node: &mut Value, field: &str, val: Value) {
    if let Value::Object(map) = node {
        map.insert(field.to_owned(), val);
    }
}

/// Remove a field from an object node.
pub fn remove_field(node: &mut Value, field: &str) {
    if let Value::Object(map) = node {
        map.remove(field);
    }
}

/// Set the value at an RFC-6901 JSON Pointer (`/content/0/…/value/units`),
/// returning whether the slot resolved. Points a constraint-violating value at
/// exactly the leaf an OPT constrains (design §2.2a: fixture-cited mutation of a
/// vendored composition, not a hand-built one).
pub fn set_pointer(root: &mut Value, pointer: &str, val: Value) -> bool {
    match root.pointer_mut(pointer) {
        Some(slot) => {
            *slot = val;
            true
        }
        None => false,
    }
}

/// Remove the leaf a JSON Pointer addresses (its parent's last segment),
/// returning whether it was present. `~1`/`~0` escapes are decoded per RFC 6901.
pub fn remove_pointer(root: &mut Value, pointer: &str) -> bool {
    let Some(idx) = pointer.rfind('/') else {
        return false;
    };
    let parent = &pointer[..idx];
    let key = pointer[idx + 1..].replace("~1", "/").replace("~0", "~");
    match root.pointer_mut(parent) {
        Some(Value::Object(map)) => map.remove(&key).is_some(),
        Some(Value::Array(items)) => match key.parse::<usize>() {
            Ok(i) if i < items.len() => {
                items.remove(i);
                true
            }
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn field_states() {
        let mut v = json!({ "language": { "_type": "CODE_PHRASE" }, "keep": 1 });
        mutate_field(&mut v, "language", FieldState::NotExist);
        assert!(v.get("language").is_none());

        let mut v = json!({ "keep": 1 });
        mutate_field(&mut v, "category", FieldState::Exist);
        assert!(v.get("category").is_some());

        let mut v = json!({ "composer": { "_type": "PARTY_IDENTIFIED" } });
        mutate_field(&mut v, "composer", FieldState::Invalid);
        assert_eq!(v["composer"]["_type"], "__INVALID__");
    }

    #[test]
    fn array_pad_and_trim() {
        let mut v = json!({ "content": [{ "n": 1 }] });
        set_array_count(&mut v, "content", 3);
        assert_eq!(v["content"].as_array().unwrap().len(), 3);
        set_array_count(&mut v, "content", 1);
        assert_eq!(v["content"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn category_and_context() {
        let mut v = json!({ "context": { "_type": "EVENT_CONTEXT" } });
        set_category(&mut v, "431", "persistent");
        assert_eq!(v["category"]["defining_code"]["code_string"], "431");
        assert!(has_context(&v));
        remove_context(&mut v);
        assert!(!has_context(&v));
    }

    #[test]
    fn node_finder() {
        let mut v = json!({
            "_type": "COMPOSITION",
            "name": { "_type": "DV_TEXT", "value": "x" },
            "content": [
                { "_type": "SECTION", "items": [
                    { "_type": "OBSERVATION", "data": { "_type": "HISTORY",
                        "events": [ { "_type": "POINT_EVENT",
                            "data": { "_type": "ITEM_TREE", "items": [
                                { "_type": "ELEMENT", "value": { "_type": "DV_QUANTITY", "magnitude": 1.0 } }
                            ] } } ] } }
                ] }
            ]
        });
        assert!(contains_node(&v, "OBSERVATION"));
        assert!(contains_node(&v, "DV_QUANTITY"));
        assert!(!contains_node(&v, "DV_URI"));

        // The outermost (pre-order) DV_TEXT is the composition name; mutating it
        // survives an OBSERVATION.data removal.
        let dv = first_node_mut(&mut v, "DV_TEXT").unwrap();
        set_field(dv, "value", json!("y"));
        assert_eq!(v["name"]["value"], "y");

        let obs = first_node_mut(&mut v, "OBSERVATION").unwrap();
        remove_field(obs, "data");
        assert!(v["content"][0]["items"][0].get("data").is_none());
    }
}
