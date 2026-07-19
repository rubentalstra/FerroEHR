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
//!
//! ## Ephemeral node identity (`_key`)
//!
//! `<For>` rows and the collapse / rename / picker UI state MUST be keyed by a
//! stable, unique, data-derived identity — never a positional path — or a
//! sibling that shifts into a deleted slot inherits the vacated row's state and
//! `<For>` view (rules §4, `view/04_iteration`). To give each FOLDER such an
//! identity, the working copy stamps every folder object with a `_key` string
//! ([`stamp_keys`]). This is a **client-only artifact of the in-memory editing
//! tree**: it is assigned after load, preserved across edits, and STRIPPED
//! ([`strip_keys`]) from every body serialized for the CDR and from the
//! advanced-JSON view — it never appears on the wire and no openEHR spec
//! governs it (our own design/extension). A folder's positional path is then
//! re-derived from its `_key` on demand ([`find_path_by_key`]) so mutations
//! keep targeting `&[usize]` paths that stay correct after indices shift.

use serde_json::{Value, json};

use crate::pages::ehr_detail::directory::{FOLDER_NODE_ID, folder_json};

/// Stamp every FOLDER in `tree` (the root included) with an ephemeral,
/// client-only `_key` identity string (`"n0"`, `"n1"`, …) drawn from
/// `counter`, inserting one only where absent — so it is idempotent and
/// preserves the keys already carried by surviving folders. The key is the
/// stable, data-derived node identity that `<For>` rows and per-folder UI
/// state key on (rules §4). It is a console-local artifact of the working
/// copy: never persisted to the CDR (see [`strip_keys`]), never rendered, and
/// no openEHR spec governs it (our own design/extension). Determinism: a plain
/// monotonic counter, no randomness or clock — hydration never observes these
/// values (they are not emitted into the DOM), but keeping them deterministic
/// costs nothing.
pub(super) fn stamp_keys(tree: &mut Value, counter: &mut u64) {
    if let Some(obj) = tree.as_object_mut() {
        if !obj.contains_key("_key") {
            obj.insert("_key".to_owned(), Value::String(format!("n{counter}")));
            *counter = counter.saturating_add(1);
        }
        if let Some(list) = obj.get_mut("folders").and_then(Value::as_array_mut) {
            for child in list {
                stamp_keys(child, counter);
            }
        }
    }
}

/// Remove the ephemeral `_key` identity from every FOLDER in `tree` (the root
/// included), recursively — the inverse of [`stamp_keys`]. Applied to the body
/// serialized for every save path and to the advanced-JSON view so the
/// console-local identity never reaches the CDR or the user.
pub(super) fn strip_keys(tree: &mut Value) {
    if let Some(obj) = tree.as_object_mut() {
        obj.remove("_key");
        if let Some(list) = obj.get_mut("folders").and_then(Value::as_array_mut) {
            for child in list {
                strip_keys(child);
            }
        }
    }
}

/// The live positional path of the folder carrying the ephemeral identity
/// `key`, searched from `root` (`Some([])` for the root, `None` if no folder
/// carries it). The `_key` is the durable identity; the path is re-derived
/// from it on every reactive read and mutation so a folder's row stays
/// correct after a sibling delete shifts indices (rules §4).
#[must_use]
pub(crate) fn find_path_by_key(root: &Value, key: &str) -> Option<Vec<usize>> {
    fn walk(node: &Value, key: &str, path: &mut Vec<usize>) -> bool {
        if node.get("_key").and_then(Value::as_str) == Some(key) {
            return true;
        }
        if let Some(list) = node.get("folders").and_then(Value::as_array) {
            for (i, child) in list.iter().enumerate() {
                path.push(i);
                if walk(child, key, path) {
                    return true;
                }
                path.pop();
            }
        }
        false
    }
    let mut path = Vec::new();
    walk(root, key, &mut path).then_some(path)
}

/// The ephemeral identity key of the folder at `path`, if stamped.
#[must_use]
pub(crate) fn node_key_at(root: &Value, path: &[usize]) -> Option<String> {
    folder_at(root, path)
        .and_then(|f| f.get("_key"))
        .and_then(Value::as_str)
        .map(str::to_owned)
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

/// The ephemeral identity keys of the child folders of `path`, in order — the
/// stable, data-derived `<For>` keys for the folder rows (rules §4). Every
/// folder in the working copy is stamped ([`stamp_keys`]), so each entry is a
/// real identity.
#[must_use]
pub(crate) fn child_node_keys(root: &Value, path: &[usize]) -> Vec<String> {
    folder_at(root, path)
        .and_then(|f| f.get("folders"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|child| {
                    child
                        .get("_key")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The number of item references in the folder at `path`.
#[must_use]
pub(crate) fn item_count(root: &Value, path: &[usize]) -> usize {
    folder_at(root, path)
        .and_then(|f| f.get("items"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
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
        // Deliberately lenient: a `datetime-local` widget only emits the two
        // lengths above; anything else is hand-typed and passed through
        // verbatim so the CDR's own `version_at_time` validation (400) is the
        // arbiter rather than a second client-side parser.
        _ => s.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        add_item, add_subfolder, child_node_keys, count_tree, delete_folder, find_path_by_key,
        item_count, item_summary, node_key_at, node_name, normalize_datetime, object_ref,
        remove_item, rename_folder, stamp_keys, strip_keys, versioned_object_id,
    };
    use crate::pages::ehr_detail::directory::{DIRECTORY_ARCHETYPE, folder_json};

    fn root() -> Value {
        folder_json(
            DIRECTORY_ARCHETYPE,
            "root",
            &[
                folder_json("at0001", "a", &[folder_json("at0001", "a1", &[])]),
                folder_json("at0001", "b", &[]),
            ],
        )
    }

    /// Collect every folder's `_key` (root first, depth-first) for assertions.
    fn collect_keys(node: &Value, out: &mut Vec<String>) {
        if let Some(k) = node.get("_key").and_then(Value::as_str) {
            out.push(k.to_owned());
        }
        if let Some(list) = node.get("folders").and_then(Value::as_array) {
            for child in list {
                collect_keys(child, out);
            }
        }
    }

    #[test]
    fn stamp_assigns_a_unique_key_to_every_folder() {
        let mut tree = root();
        let mut counter = 0u64;
        stamp_keys(&mut tree, &mut counter);
        // root + a + a1 + b = 4 folders, each stamped once.
        assert_eq!(counter, 4);
        let mut keys = Vec::new();
        collect_keys(&tree, &mut keys);
        assert_eq!(keys.len(), 4);
        let unique: std::collections::HashSet<_> = keys.iter().collect();
        assert_eq!(unique.len(), 4, "every folder _key is unique");
        for path in [&[][..], &[0], &[0, 0], &[1]] {
            assert!(node_key_at(&tree, path).is_some());
        }
    }

    #[test]
    fn strip_is_the_inverse_of_stamp() {
        let before = root();
        let mut after = before.clone();
        let mut counter = 0u64;
        stamp_keys(&mut after, &mut counter);
        strip_keys(&mut after);
        assert_eq!(after, before, "stripped tree equals the pre-stamp tree");
    }

    #[test]
    fn stamp_is_idempotent_and_keeps_existing_keys() {
        let mut tree = root();
        let mut counter = 0u64;
        stamp_keys(&mut tree, &mut counter);
        let stamped = tree.clone();
        let advanced = counter;
        // A second pass adds nothing and advances the counter for no folder.
        stamp_keys(&mut tree, &mut counter);
        assert_eq!(tree, stamped);
        assert_eq!(counter, advanced);
        // Only a newly added (unstamped) folder is stamped on the next pass.
        add_subfolder(&mut tree, &[1], "b1");
        stamp_keys(&mut tree, &mut counter);
        assert_eq!(counter, advanced + 1);
        assert!(node_key_at(&tree, &[1, 0]).is_some());
    }

    #[test]
    fn identity_key_survives_a_sibling_delete() {
        let mut tree = root();
        let mut counter = 0u64;
        stamp_keys(&mut tree, &mut counter);
        let root_key = node_key_at(&tree, &[]).unwrap();
        let b_key = node_key_at(&tree, &[1]).unwrap();
        assert_eq!(find_path_by_key(&tree, &root_key), Some(Vec::new()));
        assert_eq!(find_path_by_key(&tree, &b_key), Some(vec![1]));
        assert_eq!(find_path_by_key(&tree, "nope"), None);
        // Deleting the first child shifts `b` from index 1 to index 0 — its
        // identity key now resolves to the new path (the reviewed defect fix).
        delete_folder(&mut tree, &[0]);
        assert_eq!(find_path_by_key(&tree, &b_key), Some(vec![0]));
        assert_eq!(
            node_name(&tree, &find_path_by_key(&tree, &b_key).unwrap()),
            "b"
        );
    }

    #[test]
    fn navigation_reads_names() {
        let tree = root();
        assert_eq!(node_name(&tree, &[]), "root");
        assert_eq!(node_name(&tree, &[0]), "a");
        assert_eq!(node_name(&tree, &[0, 0]), "a1");
    }

    #[test]
    fn child_node_keys_and_item_count_read_structure() {
        let mut tree = root();
        let mut counter = 0u64;
        stamp_keys(&mut tree, &mut counter);
        assert_eq!(child_node_keys(&tree, &[]).len(), 2);
        assert_eq!(child_node_keys(&tree, &[0]).len(), 1);
        assert!(child_node_keys(&tree, &[1]).is_empty());
        // The child keys are the children's stamped identities, in order.
        let keys = child_node_keys(&tree, &[]);
        assert_eq!(keys[0], node_key_at(&tree, &[0]).unwrap());
        assert_eq!(keys[1], node_key_at(&tree, &[1]).unwrap());
        assert_eq!(item_count(&tree, &[0]), 0);
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
        assert_eq!(child_node_keys(&tree, &[]).len(), 1);
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
        assert_eq!(item_count(&tree, &[0]), 1);
        assert_eq!(
            item_summary(&tree, &[0], 0),
            ("COMPOSITION".to_owned(), "abc".to_owned())
        );
        remove_item(&mut tree, &[0], 0);
        assert_eq!(item_count(&tree, &[0]), 0);
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
