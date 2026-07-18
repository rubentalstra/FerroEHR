//! Pure, component-free helpers for the structured directory tree editor:
//! navigation and mutation of a canonical `FOLDER` tree held as
//! `serde_json::Value`, `OBJECT_REF` construction, tree statistics, and the
//! `version_at_time` datetime normalization. Kept out of the view code so the
//! editing logic is unit-tested directly (rules §10 — business logic lives in
//! plain types).
//!
//! The FOLDER shape these operate on IS spec-bound (ITS-REST
//! `specifications/schemas/ehr/Folder.yaml`; RM common
//! `master05-directory_package`): a FOLDER carries `folders` (child FOLDERs)
//! and `items` (`OBJECT_REF`s). Node paths address a folder by the chain of
//! child indices into successive `folders` arrays from the root
//! (`[]` = the root FOLDER, `[0]` = its first child, `[0,1]` = that child's
//! second child).

use serde_json::{Value, json};

use crate::pages::ehr_detail::directory::{FOLDER_NODE_ID, folder_json};

/// The stable rendering key for a folder path: the child indices joined with
/// `/` (`[]` → `""`, `[0,1]` → `"0/1"`). Data-derived, so `<For>` keys stay
/// meaningful across edits (rules §4).
#[must_use]
pub(crate) fn key_of(path: &[usize]) -> String {
    path.iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("/")
}

/// Parse a folder path key back into its child-index chain
/// (`"0/1"` → `[0, 1]`, `""` → `[]`). Non-numeric segments are dropped.
#[must_use]
pub(crate) fn parse_key(key: &str) -> Vec<usize> {
    if key.is_empty() {
        return Vec::new();
    }
    key.split('/').filter_map(|s| s.parse().ok()).collect()
}

/// The rendering key for the `idx`-th item of the folder at `path`
/// (`(path "0", idx 1)` → `"0#1"`).
#[must_use]
pub(crate) fn item_key(path: &[usize], idx: usize) -> String {
    format!("{}#{idx}", key_of(path))
}

/// Split an item key into its owning folder path and item index
/// (`"0#1"` → `([0], 1)`).
#[must_use]
pub(crate) fn parse_item_key(key: &str) -> (Vec<usize>, usize) {
    match key.split_once('#') {
        Some((folder, idx)) => (parse_key(folder), idx.parse().unwrap_or(0)),
        None => (Vec::new(), 0),
    }
}

/// Borrow the folder node at `path`, following `folders` arrays from `root`.
#[must_use]
pub(crate) fn folder_at<'a>(root: &'a Value, path: &[usize]) -> Option<&'a Value> {
    let mut node = root;
    for &idx in path {
        node = node.get("folders")?.get(idx)?;
    }
    Some(node)
}

/// Mutably borrow the folder node at `path`.
fn folder_at_mut<'a>(root: &'a mut Value, path: &[usize]) -> Option<&'a mut Value> {
    let mut node = root;
    for &idx in path {
        node = node.get_mut("folders")?.get_mut(idx)?;
    }
    Some(node)
}

/// The display name (`name/value`) of the folder at `path`, or a placeholder.
#[must_use]
pub(crate) fn node_name(root: &Value, path: &[usize]) -> String {
    folder_at(root, path)
        .and_then(|f| f.get("name"))
        .and_then(|n| n.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("(folder)")
        .to_owned()
}

/// The child-folder rendering keys for the folder at `path`, in order.
#[must_use]
pub(crate) fn child_keys(root: &Value, path: &[usize]) -> Vec<String> {
    let count = folder_at(root, path)
        .and_then(|f| f.get("folders"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    (0..count)
        .map(|i| {
            let mut child = path.to_vec();
            child.push(i);
            key_of(&child)
        })
        .collect()
}

/// The item rendering keys for the folder at `path`, in order.
#[must_use]
pub(crate) fn item_keys(root: &Value, path: &[usize]) -> Vec<String> {
    let count = folder_at(root, path)
        .and_then(|f| f.get("items"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    (0..count).map(|i| item_key(path, i)).collect()
}

/// The `(type, id-value)` summary of the `idx`-th item of the folder at
/// `path`, for row display.
#[must_use]
pub(crate) fn item_summary(root: &Value, path: &[usize], idx: usize) -> (String, String) {
    let item = folder_at(root, path)
        .and_then(|f| f.get("items"))
        .and_then(Value::as_array)
        .and_then(|items| items.get(idx));
    let ref_type = item
        .and_then(|i| i.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("OBJECT")
        .to_owned();
    let id = item
        .and_then(|i| i.get("id"))
        .and_then(|i| i.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("(ref)")
        .to_owned();
    (ref_type, id)
}

/// Append a new empty child folder named `name` to the folder at `path`.
pub(crate) fn add_subfolder(root: &mut Value, path: &[usize], name: &str) {
    if let Some(node) = folder_at_mut(root, path).and_then(Value::as_object_mut) {
        let folders = node
            .entry("folders")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(list) = folders.as_array_mut() {
            list.push(folder_json(FOLDER_NODE_ID, name, &[]));
        }
    }
}

/// Rename the folder at `path` (sets `name/value`, keeping it a `DV_TEXT`).
pub(crate) fn rename_folder(root: &mut Value, path: &[usize], name: &str) {
    if let Some(node) = folder_at_mut(root, path).and_then(Value::as_object_mut) {
        let name_obj = node
            .entry("name")
            .or_insert_with(|| json!({ "_type": "DV_TEXT", "value": "" }));
        if let Some(obj) = name_obj.as_object_mut() {
            obj.insert("value".to_owned(), Value::String(name.to_owned()));
            obj.entry("_type")
                .or_insert_with(|| Value::String("DV_TEXT".to_owned()));
        }
    }
}

/// Remove the folder at `path` from its parent. A no-op for the root
/// (`path` empty) — the root FOLDER is never deleted from within the tree.
pub(crate) fn delete_folder(root: &mut Value, path: &[usize]) {
    if let Some((&idx, parent_path)) = path.split_last()
        && let Some(list) = folder_at_mut(root, parent_path)
            .and_then(|p| p.get_mut("folders"))
            .and_then(Value::as_array_mut)
        && idx < list.len()
    {
        list.remove(idx);
    }
}

/// Append an item reference to the folder at `path`.
pub(crate) fn add_item(root: &mut Value, path: &[usize], item: Value) {
    if let Some(node) = folder_at_mut(root, path).and_then(Value::as_object_mut) {
        let items = node
            .entry("items")
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(list) = items.as_array_mut() {
            list.push(item);
        }
    }
}

/// Remove the `idx`-th item from the folder at `path`.
pub(crate) fn remove_item(root: &mut Value, path: &[usize], idx: usize) {
    if let Some(list) = folder_at_mut(root, path)
        .and_then(|f| f.get_mut("items"))
        .and_then(Value::as_array_mut)
        && idx < list.len()
    {
        list.remove(idx);
    }
}

/// Build a canonical `OBJECT_REF` from its parts, per RM common
/// `master03-support` (`OBJECT_REF` = `namespace` + `type` +
/// `id: OBJECT_ID`). `id_type` is the concrete `OBJECT_ID` subtype
/// (`HIER_OBJECT_ID`, `OBJECT_VERSION_ID`, …).
#[must_use]
pub(crate) fn object_ref(namespace: &str, ref_type: &str, id_type: &str, id_value: &str) -> Value {
    json!({
        "_type": "OBJECT_REF",
        "namespace": namespace,
        "type": ref_type,
        "id": { "_type": id_type, "value": id_value },
    })
}

/// The versioned-object id of an `OBJECT_VERSION_ID` value (everything before
/// the first `::`), used as the `HIER_OBJECT_ID` value when referencing a
/// composition by its versioned object rather than a specific version.
#[must_use]
pub(crate) fn versioned_object_id(uid: &str) -> &str {
    uid.split_once("::").map_or(uid, |(head, _)| head)
}

/// The total descendant folder count (excluding the root) and total item
/// count across the whole tree, for the version-history summary.
#[must_use]
pub(crate) fn count_tree(root: &Value) -> (i32, i32) {
    fn walk(node: &Value, folders: &mut i32, items: &mut i32) {
        if let Some(list) = node.get("items").and_then(Value::as_array) {
            *items = items.saturating_add(i32::try_from(list.len()).unwrap_or(i32::MAX));
        }
        if let Some(subs) = node.get("folders").and_then(Value::as_array) {
            for sub in subs {
                *folders = folders.saturating_add(1);
                walk(sub, folders, items);
            }
        }
    }
    let (mut folders, mut items) = (0, 0);
    walk(root, &mut folders, &mut items);
    (folders, items)
}

/// Normalize a browser `datetime-local` value (`YYYY-MM-DDTHH:MM[:SS]`, no
/// zone) into an ISO 8601 datetime for the `version_at_time` query parameter
/// (ITS-REST `parameters/query/version_at_time.yaml`).
///
/// NOTE: the browser's timezone offset is not available without JavaScript
/// (the console is JS-free), so a zone-less input is interpreted as UTC and
/// stamped with `Z`. An input that already carries a zone (`Z` or an offset)
/// passes through unchanged.
#[must_use]
pub(crate) fn normalize_datetime(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return String::new();
    }
    let has_zone = s
        .get(10..)
        .is_some_and(|tail| tail.contains('Z') || tail.contains('+') || tail.contains('-'));
    if has_zone {
        return s.to_owned();
    }
    match s.len() {
        16 => format!("{s}:00Z"),
        19 => format!("{s}Z"),
        _ => s.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        add_item, add_subfolder, child_keys, count_tree, delete_folder, item_keys, item_summary,
        key_of, node_name, normalize_datetime, object_ref, parse_item_key, parse_key, remove_item,
        rename_folder, versioned_object_id,
    };
    use crate::pages::ehr_detail::directory::{DIRECTORY_ARCHETYPE, folder_json};

    fn root() -> serde_json::Value {
        folder_json(
            DIRECTORY_ARCHETYPE,
            "root",
            &[
                folder_json("at0001", "a", &[folder_json("at0001", "a1", &[])]),
                folder_json("at0001", "b", &[]),
            ],
        )
    }

    #[test]
    fn key_round_trips_through_parse() {
        assert_eq!(key_of(&[]), "");
        assert_eq!(key_of(&[0, 1]), "0/1");
        assert_eq!(parse_key(""), Vec::<usize>::new());
        assert_eq!(parse_key("0/1"), vec![0, 1]);
    }

    #[test]
    fn item_key_round_trips() {
        assert_eq!(super::item_key(&[0], 2), "0#2");
        assert_eq!(parse_item_key("0#2"), (vec![0], 2));
        assert_eq!(parse_item_key("#0"), (Vec::<usize>::new(), 0));
    }

    #[test]
    fn navigation_reads_names_and_children() {
        let tree = root();
        assert_eq!(node_name(&tree, &[]), "root");
        assert_eq!(node_name(&tree, &[0]), "a");
        assert_eq!(node_name(&tree, &[0, 0]), "a1");
        assert_eq!(child_keys(&tree, &[]), vec!["0", "1"]);
        assert_eq!(child_keys(&tree, &[0]), vec!["0/0"]);
        assert!(child_keys(&tree, &[1]).is_empty());
    }

    #[test]
    fn add_and_rename_and_delete_folders() {
        let mut tree = root();
        add_subfolder(&mut tree, &[1], "b1");
        assert_eq!(node_name(&tree, &[1, 0]), "b1");
        rename_folder(&mut tree, &[1, 0], "renamed");
        assert_eq!(node_name(&tree, &[1, 0]), "renamed");
        // Deleting the first child shifts the second into index 0.
        delete_folder(&mut tree, &[0]);
        assert_eq!(child_keys(&tree, &[]), vec!["0"]);
        assert_eq!(node_name(&tree, &[0]), "b");
        // Deleting the root is a no-op.
        delete_folder(&mut tree, &[]);
        assert_eq!(node_name(&tree, &[]), "root");
    }

    #[test]
    fn items_add_summarize_and_remove() {
        let mut tree = root();
        add_item(
            &mut tree,
            &[0],
            object_ref("local", "COMPOSITION", "HIER_OBJECT_ID", "abc"),
        );
        assert_eq!(item_keys(&tree, &[0]), vec!["0#0"]);
        assert_eq!(
            item_summary(&tree, &[0], 0),
            ("COMPOSITION".to_owned(), "abc".to_owned())
        );
        remove_item(&mut tree, &[0], 0);
        assert!(item_keys(&tree, &[0]).is_empty());
    }

    #[test]
    fn object_ref_is_spec_shaped() {
        let r = object_ref("local", "COMPOSITION", "HIER_OBJECT_ID", "abc::sys::1");
        assert_eq!(r["_type"], "OBJECT_REF");
        assert_eq!(r["namespace"], "local");
        assert_eq!(r["type"], "COMPOSITION");
        assert_eq!(r["id"]["_type"], "HIER_OBJECT_ID");
        assert_eq!(r["id"]["value"], "abc::sys::1");
    }

    #[test]
    fn versioned_object_id_strips_the_version() {
        assert_eq!(versioned_object_id("abc::sys::1"), "abc");
        assert_eq!(versioned_object_id("abc"), "abc");
    }

    #[test]
    fn count_tree_counts_descendants_and_items() {
        let mut tree = root();
        add_item(
            &mut tree,
            &[0],
            object_ref("local", "COMPOSITION", "HIER_OBJECT_ID", "x"),
        );
        // Folders a, a1, b = 3; one item.
        assert_eq!(count_tree(&tree), (3, 1));
    }

    #[test]
    fn datetime_normalization_stamps_utc_and_respects_zones() {
        assert_eq!(
            normalize_datetime("2026-07-18T14:30"),
            "2026-07-18T14:30:00Z"
        );
        assert_eq!(
            normalize_datetime("2026-07-18T14:30:15"),
            "2026-07-18T14:30:15Z"
        );
        assert_eq!(
            normalize_datetime("2026-07-18T14:30:00Z"),
            "2026-07-18T14:30:00Z"
        );
        assert_eq!(
            normalize_datetime("2026-07-18T14:30:00+02:00"),
            "2026-07-18T14:30:00+02:00"
        );
        assert_eq!(normalize_datetime("  "), "");
    }
}
