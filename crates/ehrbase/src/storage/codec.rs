//! Canonical JSON ⇄ node rows (ADR-008; spike-validated in
//! `tests/storage_spike.rs`).

use serde_json::{Map, Value};

use crate::storage::StorageError;

/// RM structure types: every one of these gets its own `node` row (fine
/// granularity — the P10 spike decision). LOCATABLE content types plus
/// `EVENT_CONTEXT` and `FEEDER_AUDIT`, which AQL can address although the RM
/// does not make them LOCATABLE.
const STRUCTURE_TYPES: &[&str] = &[
    "COMPOSITION",
    "EHR_STATUS",
    "FOLDER",
    "EVENT_CONTEXT",
    "SECTION",
    "GENERIC_ENTRY",
    "ADMIN_ENTRY",
    "OBSERVATION",
    "EVALUATION",
    "INSTRUCTION",
    "ACTION",
    "ACTIVITY",
    "HISTORY",
    "POINT_EVENT",
    "INTERVAL_EVENT",
    "ITEM_TREE",
    "ITEM_LIST",
    "ITEM_SINGLE",
    "ITEM_TABLE",
    "CLUSTER",
    "ELEMENT",
    "FEEDER_AUDIT",
];

/// Whether an RM `_type` gets its own `node` row.
#[must_use]
pub fn is_structure_type(rm_type: &str) -> bool {
    STRUCTURE_TYPES.contains(&rm_type)
}

/// One decomposed `node` row (content columns only — storage context like
/// `vo_id`/`sys_version`/`ehr_id` is added by the repository).
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRow {
    /// Pre-order number within the versioned object (root = 0).
    pub num: i32,
    /// Max `num` in this row's subtree: the subtree is `num..=num_cap`.
    pub num_cap: i32,
    /// `num` of the parent structure node (root points at itself/0).
    pub parent_num: i32,
    /// `num` of the nearest ancestor carrying an archetype id.
    pub citem_num: Option<i32>,
    /// The RM `_type`, verbatim (e.g. `OBSERVATION`).
    pub rm_type: String,
    /// `archetype_node_id`, verbatim.
    pub archetype: Option<String>,
    /// `name/value`.
    pub name: Option<String>,
    /// Materialized path from the root: full attribute names, array index
    /// appended, `.`-terminated steps (`content0.data.events1.`) so byte
    /// order under `COLLATE "C"` equals tree order.
    pub path: String,
    /// The node's canonical JSON fragment, structure children pruned.
    pub data: Value,
}

/// Decomposes a versioned object's canonical JSON into node rows.
///
/// # Errors
///
/// Fails when the root has no structure `_type`, or an array mixes
/// structure and non-structure elements (canonical RM JSON never does).
pub fn decompose(root: Value) -> Result<Vec<NodeRow>, StorageError> {
    let root_type = root.get("_type").and_then(Value::as_str);
    if !root_type.is_some_and(is_structure_type) {
        return Err(StorageError::NotAStructureRoot(
            root_type.map(str::to_owned),
        ));
    }

    let mut rows = Vec::new();
    walk(root, "", -1, None, &mut rows)?;

    // num_cap: children always follow their parents — one reverse pass
    let mut caps: Vec<i32> = rows.iter().map(|r| r.num).collect();
    for i in (1..rows.len()).rev() {
        let parent = usize::try_from(rows[i].parent_num).unwrap_or_default();
        if caps[i] > caps[parent] {
            caps[parent] = caps[i];
        }
        rows[i].num_cap = caps[i];
    }
    if let (Some(row), Some(cap)) = (rows.first_mut(), caps.first()) {
        row.num_cap = *cap;
    }
    Ok(rows)
}

fn walk(
    mut json: Value,
    path: &str,
    parent: i32,
    citem: Option<i32>,
    rows: &mut Vec<NodeRow>,
) -> Result<(), StorageError> {
    let index = rows.len();
    let num = i32::try_from(index).unwrap_or(i32::MAX);
    let rm_type = json
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let archetype = json
        .get("archetype_node_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let name = json
        .get("name")
        .and_then(|n| n.get("value"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    // the archetype ancestor for at-code scoping (this node if it carries an
    // archetype id itself, else inherited)
    let child_citem = if archetype
        .as_deref()
        .is_some_and(|a| a.starts_with("openEHR-"))
    {
        Some(num)
    } else {
        citem
    };
    rows.push(NodeRow {
        num,
        num_cap: num,
        parent_num: parent.max(0),
        citem_num: citem,
        rm_type,
        archetype,
        name,
        path: path.to_owned(),
        data: Value::Null,
    });

    if let Value::Object(map) = &mut json {
        prune_children(map, path, num, child_citem, rows)?;
    }
    rows[index].data = json;
    Ok(())
}

/// Prunes structure children out of `map`, recursing in document order.
fn prune_children(
    map: &mut Map<String, Value>,
    path: &str,
    num: i32,
    citem: Option<i32>,
    rows: &mut Vec<NodeRow>,
) -> Result<(), StorageError> {
    let attributes: Vec<String> = map.keys().cloned().collect();
    for attribute in attributes {
        match map.get(&attribute) {
            Some(child @ Value::Object(_)) if is_structure(child) => {
                let owned = map.shift_remove(&attribute).unwrap_or(Value::Null);
                walk(owned, &format!("{path}{attribute}."), num, citem, rows)?;
            }
            Some(Value::Array(items)) if !items.is_empty() => {
                let structure_count = items.iter().filter(|c| is_structure(c)).count();
                if structure_count == 0 {
                    continue;
                }
                if structure_count != items.len() {
                    return Err(StorageError::MixedArray { attribute });
                }
                let Some(Value::Array(items)) = map.shift_remove(&attribute) else {
                    continue;
                };
                for (i, item) in items.into_iter().enumerate() {
                    walk(item, &format!("{path}{attribute}{i}."), num, citem, rows)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn is_structure(v: &Value) -> bool {
    v.get("_type")
        .and_then(Value::as_str)
        .is_some_and(is_structure_type)
}

/// Reassembles the canonical JSON from node rows (sorted or not — rows are
/// ordered by `num` internally). Lossless inverse of [`decompose`].
///
/// # Errors
///
/// Fails when the rows do not form a single tree rooted at `num = 0`.
pub fn reassemble(rows: &[NodeRow]) -> Result<Value, StorageError> {
    let mut ordered: Vec<&NodeRow> = rows.iter().collect();
    ordered.sort_by_key(|r| r.num);
    let Some(root_row) = ordered.first() else {
        return Err(StorageError::InvalidRows("no rows".into()));
    };
    if root_row.num != 0 || !root_row.path.is_empty() {
        return Err(StorageError::InvalidRows(format!(
            "root row must have num=0 and empty path (got num={}, path={:?})",
            root_row.num, root_row.path
        )));
    }
    let mut root = root_row.data.clone();
    for row in &ordered[1..] {
        attach(&mut root, &row.path, row.data.clone())?;
    }
    Ok(root)
}

/// Re-attaches one pruned fragment at its materialized path. Parents come
/// before children (`num` order), so every ancestor is already in place.
fn attach(root: &mut Value, path: &str, fragment: Value) -> Result<(), StorageError> {
    let missing =
        |step: &str| StorageError::InvalidRows(format!("missing ancestor {step:?} ({path})"));
    let steps: Vec<&str> = path.trim_end_matches('.').split('.').collect();
    let mut current = root;
    for (i, step) in steps.iter().enumerate() {
        let is_leaf = i == steps.len() - 1;
        let (attribute, index) = split_step(step);
        let Value::Object(map) = current else {
            return Err(missing(step));
        };
        match index {
            None => {
                if is_leaf {
                    map.insert(attribute.to_owned(), fragment);
                    return Ok(());
                }
                current = map.get_mut(attribute).ok_or_else(|| missing(step))?;
            }
            Some(index) => {
                let slot = map
                    .entry(attribute.to_owned())
                    .or_insert_with(|| Value::Array(Vec::new()));
                let Value::Array(array) = slot else {
                    return Err(missing(step));
                };
                if is_leaf {
                    if array.len() <= index {
                        array.resize(index + 1, Value::Null);
                    }
                    array[index] = fragment;
                    return Ok(());
                }
                current = array.get_mut(index).ok_or_else(|| missing(step))?;
            }
        }
    }
    Err(StorageError::InvalidRows(format!("empty path ({path})")))
}

/// Splits a path step into (attribute, array index): `content0` → `content`
/// + 0; `context` → no index.
fn split_step(step: &str) -> (&str, Option<usize>) {
    let digits = step.chars().rev().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        (step, None)
    } else {
        let (attribute, number) = step.split_at(step.len() - digits);
        (attribute, number.parse().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "_type": "COMPOSITION",
            "archetype_node_id": "openEHR-EHR-COMPOSITION.report.v1",
            "name": {"_type": "DV_TEXT", "value": "Report"},
            "context": {
                "_type": "EVENT_CONTEXT",
                "setting": {"_type": "DV_CODED_TEXT", "value": "other care"}
            },
            "content": [
                {
                    "_type": "OBSERVATION",
                    "archetype_node_id": "openEHR-EHR-OBSERVATION.bp.v2",
                    "name": {"_type": "DV_TEXT", "value": "Blood pressure"},
                    "data": {
                        "_type": "HISTORY",
                        "archetype_node_id": "at0001",
                        "name": {"_type": "DV_TEXT", "value": "history"},
                        "events": [{
                            "_type": "POINT_EVENT",
                            "archetype_node_id": "at0006",
                            "name": {"_type": "DV_TEXT", "value": "any event"}
                        }]
                    }
                }
            ]
        })
    }

    type BriefRow<'a> = (&'a str, &'a str, i32, i32, i32, Option<i32>);

    #[test]
    fn decomposes_with_nested_set_numbers() {
        let rows = decompose(sample()).unwrap();
        let brief: Vec<BriefRow> = rows
            .iter()
            .map(|r| {
                (
                    r.rm_type.as_str(),
                    r.path.as_str(),
                    r.num,
                    r.num_cap,
                    r.parent_num,
                    r.citem_num,
                )
            })
            .collect();
        assert_eq!(
            brief,
            vec![
                ("COMPOSITION", "", 0, 4, 0, None),
                ("EVENT_CONTEXT", "context.", 1, 1, 0, Some(0)),
                ("OBSERVATION", "content0.", 2, 4, 0, Some(0)),
                ("HISTORY", "content0.data.", 3, 4, 2, Some(2)),
                ("POINT_EVENT", "content0.data.events0.", 4, 4, 3, Some(2)),
            ]
        );
        // structure children are pruned out of the parent fragments
        assert!(rows[0].data.get("context").is_none());
        assert!(rows[0].data.get("content").is_none());
        assert!(rows[2].data.get("data").is_none());
        // non-structure content stays verbatim
        assert_eq!(rows[0].data["name"]["value"], "Report");
        assert_eq!(rows[1].data["setting"]["value"], "other care");
    }

    #[test]
    fn round_trips_losslessly() {
        let original = sample();
        let rows = decompose(original.clone()).unwrap();
        assert_eq!(reassemble(&rows).unwrap(), original);
    }

    #[test]
    fn rejects_non_structure_roots() {
        assert!(matches!(
            decompose(json!({"_type": "DV_TEXT", "value": "x"})),
            Err(StorageError::NotAStructureRoot(Some(t))) if t == "DV_TEXT"
        ));
    }

    #[test]
    fn rejects_mixed_arrays() {
        let v = json!({
            "_type": "COMPOSITION",
            "content": [
                {"_type": "OBSERVATION"},
                {"_type": "DV_TEXT", "value": "not a structure"}
            ]
        });
        assert!(matches!(
            decompose(v),
            Err(StorageError::MixedArray { attribute }) if attribute == "content"
        ));
    }

    #[test]
    fn reassembles_out_of_order_rows() {
        let original = sample();
        let mut rows = decompose(original.clone()).unwrap();
        rows.reverse();
        assert_eq!(reassemble(&rows).unwrap(), original);
    }
}
