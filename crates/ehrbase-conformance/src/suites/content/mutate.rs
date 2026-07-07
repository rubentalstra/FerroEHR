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
}
