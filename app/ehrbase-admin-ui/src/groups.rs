//! Console-local query-group persistence: a small JSON file next to the
//! console (path configurable). No ITS-REST resource models query groups —
//! no openEHR spec governs this; our own design/extension. The file is the
//! whole store (read/modify/write; single-instance console, low write
//! volume).

use crate::error::AdminUiError;
use crate::queries_api::QueryGroup;

/// Read all groups; a missing file is an empty store.
///
/// # Errors
/// [`AdminUiError::Internal`] on an unreadable/corrupt file.
pub fn read_groups(path: &str) -> Result<Vec<QueryGroup>, AdminUiError> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| AdminUiError::Internal(format!("groups file `{path}`: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(AdminUiError::Internal(format!("groups file `{path}`: {e}"))),
    }
}

/// Create/replace `name` (empty `members` deletes it), then persist.
///
/// # Errors
/// [`AdminUiError::Internal`] on an unreadable or unwritable file.
pub fn write_group(path: &str, name: &str, members: Vec<String>) -> Result<(), AdminUiError> {
    let mut groups = read_groups(path)?;
    groups.retain(|g| g.name != name);
    if !members.is_empty() {
        groups.push(QueryGroup {
            name: name.to_owned(),
            members,
        });
        groups.sort_by(|a, b| a.name.cmp(&b.name));
    }
    let text = serde_json::to_string_pretty(&groups)
        .map_err(|e| AdminUiError::Internal(format!("groups serialize: {e}")))?;
    // Atomic replace: write a sibling temp file, then rename over the store,
    // so a crash mid-write never corrupts it (reliability rule: fail loud,
    // never leave a half-written clinical-adjacent store).
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, text)
        .map_err(|e| AdminUiError::Internal(format!("groups file `{tmp}`: {e}")))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| AdminUiError::Internal(format!("groups file `{path}`: {e}")))
}

#[cfg(test)]
mod tests {
    use crate::groups::{read_groups, write_group};

    #[test]
    fn groups_round_trip_create_replace_delete() {
        let dir = std::env::temp_dir().join(format!("admin-ui-groups-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("groups.json");
        let path = path.to_str().unwrap();

        assert!(read_groups(path).unwrap().is_empty());
        write_group(path, "chronic", vec!["a::q@1.0".to_owned()]).unwrap();
        write_group(path, "acute", vec!["b::q@1.0".to_owned()]).unwrap();
        let groups = read_groups(path).unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "acute"); // sorted

        // Replace, then delete via empty members.
        write_group(path, "acute", vec!["c::q@2.0".to_owned()]).unwrap();
        assert_eq!(read_groups(path).unwrap()[0].members, vec!["c::q@2.0"]);
        write_group(path, "acute", vec![]).unwrap();
        assert_eq!(read_groups(path).unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
