//! FLAT-key → nested [`Entry`] parsing (Better `WebTemplatePathSegment` +
//! `FlatToStructuredConverter.convertEntry`).
//!
//! A flat key `a/b:0/c|magnitude` is split on `/`; each segment is
//! `key[:index][|attribute[:attributeIndex]]`. The `|attribute` folds into an
//! extra child level named `"|attribute"`, so the deepest child of the chain
//! carries the leaf value.

use serde_json::Value;

/// One node of a parsed flat key: a name, its `:index` order, and a child chain.
#[derive(Clone)]
pub(super) struct Entry {
    /// The segment key (or `"|suffix"` for a folded attribute; `""` for a bare
    /// value wrapped alongside suffixed siblings).
    pub name: String,
    /// The `:index` of the segment (0 when absent).
    pub order: usize,
    /// Whether the segment carried an explicit `:index` (distinguishes a `ctx`
    /// scalar from a single-element indexed list).
    pub indexed: bool,
    /// The next segment down, if any.
    pub child: Option<Box<Entry>>,
    /// The leaf value (present only on the deepest segment).
    pub value: Option<Value>,
}

impl Entry {
    /// Parse a flat key + value into a nested [`Entry`] chain.
    pub(super) fn parse(key: &str, value: Value) -> Entry {
        let segs: Vec<&str> = key.split('/').collect();
        convert_recursive(&segs, 0, Some(value)).unwrap_or_else(|| Entry {
            name: key.to_owned(),
            order: 0,
            indexed: false,
            child: None,
            value: None,
        })
    }
}

fn convert_recursive(segs: &[&str], i: usize, value: Option<Value>) -> Option<Entry> {
    if i >= segs.len() {
        return None;
    }
    let child = convert_recursive(segs, i + 1, value.clone());
    let v = if child.is_none() { value } else { None };
    Some(convert_key_segment(segs[i], child, v))
}

fn convert_key_segment(seg: &str, child: Option<Entry>, value: Option<Value>) -> Entry {
    let (key_part, attr_part) = match seg.split_once('|') {
        Some((k, a)) => (k, Some(a)),
        None => (seg, None),
    };
    let (key, index, indexed) = match key_part.split_once(':') {
        Some((k, idx)) => (k.to_owned(), idx.parse().unwrap_or(0), true),
        None => (key_part.to_owned(), 0, false),
    };
    match attr_part {
        None => Entry {
            name: key,
            order: index,
            indexed,
            child: child.map(Box::new),
            value,
        },
        Some(attr) => {
            let (attr_name, attr_index) = match attr.split_once(':') {
                Some((a, ai)) => (a, ai.parse().unwrap_or(0)),
                None => (attr, 0usize),
            };
            let inner = Entry {
                name: format!("|{attr_name}"),
                order: attr_index,
                indexed: false,
                child: child.map(Box::new),
                value: value.clone(),
            };
            Entry {
                name: key,
                order: index,
                indexed,
                child: Some(Box::new(inner)),
                value,
            }
        }
    }
}
