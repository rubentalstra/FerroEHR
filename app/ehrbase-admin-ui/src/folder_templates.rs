//! Console-local FOLDER-template persistence: named FOLDER-tree shapes stored
//! in a small JSON file next to the query-groups store (same config dir),
//! mirroring [`crate::groups`]. No ITS-REST resource models folder templates —
//! no openEHR spec governs an admin UI; our own design/extension. The FOLDER
//! shapes the templates carry ARE spec-bound (ITS-REST
//! `specifications/schemas/ehr/Folder.yaml`; RM common
//! `master05-directory_package`), so the built-ins are generated as spec-valid
//! canonical FOLDER JSON.
//!
//! Two built-in defaults ("Episodes by year", "Clinical areas") stand in when
//! the file is absent; the first `save_folder_template` seeds them into the
//! file so they survive. The file is the whole store (read/modify/write;
//! single-instance console, low write volume).

use serde_json::Value;

use crate::error::AdminUiError;
use crate::pages::ehr_detail::directory::{
    DIRECTORY_ARCHETYPE, FOLDER_NODE_ID, FolderTemplate, folder_json,
};

/// The folder-templates store path: `admin-ui-folder-templates.json` in the
/// same directory as the query-groups store, so both console-local stores
/// share one config dir (the design's "next to the groups file").
#[must_use]
pub fn templates_path(groups_file: &str) -> String {
    std::path::Path::new(groups_file)
        .with_file_name("admin-ui-folder-templates.json")
        .to_string_lossy()
        .into_owned()
}

/// The built-in folder templates shipped when the store file is absent.
///
/// Both are spec-valid canonical FOLDER trees: the root carries the standard
/// directory archetype id (`schemas/ehr/Folder.yaml` example); child folders
/// reuse the directory archetype's internal node id and differ by `name` (RM
/// common `master05-directory_package` §Paths — uniqueness modifiers).
#[must_use]
pub fn builtin_templates() -> Vec<FolderTemplate> {
    vec![
        FolderTemplate {
            name: "Episodes by year".to_owned(),
            folder: folder_json(
                DIRECTORY_ARCHETYPE,
                "root",
                &[folder_json(
                    FOLDER_NODE_ID,
                    "episodes",
                    &[folder_json(FOLDER_NODE_ID, "2026", &[])],
                )],
            ),
        },
        FolderTemplate {
            name: "Clinical areas".to_owned(),
            folder: folder_json(
                DIRECTORY_ARCHETYPE,
                "root",
                &[
                    folder_json(FOLDER_NODE_ID, "medications", &[]),
                    folder_json(FOLDER_NODE_ID, "allergies", &[]),
                    folder_json(FOLDER_NODE_ID, "encounters", &[]),
                ],
            ),
        },
    ]
}

/// Read all folder templates; a missing file yields the built-in defaults.
///
/// # Errors
/// [`AdminUiError::Internal`] on an unreadable/corrupt file.
pub fn read_templates(path: &str) -> Result<Vec<FolderTemplate>, AdminUiError> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| AdminUiError::Internal(format!("folder-templates file `{path}`: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(builtin_templates()),
        Err(e) => Err(AdminUiError::Internal(format!(
            "folder-templates file `{path}`: {e}"
        ))),
    }
}

/// Create/replace the template named `name` with `folder`, then persist. The
/// first write seeds the built-ins (via [`read_templates`]) so they are not
/// lost once the file exists.
///
/// # Errors
/// [`AdminUiError::Internal`] on an unreadable or unwritable file.
pub fn write_template(path: &str, name: &str, folder: Value) -> Result<(), AdminUiError> {
    let mut templates = read_templates(path)?;
    templates.retain(|t| t.name != name);
    templates.push(FolderTemplate {
        name: name.to_owned(),
        folder,
    });
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    let text = serde_json::to_string_pretty(&templates)
        .map_err(|e| AdminUiError::Internal(format!("folder-templates serialize: {e}")))?;
    // Atomic replace: write a sibling temp file, then rename over the store, so
    // a crash mid-write never corrupts it (mirrors `groups::write_group`).
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, text)
        .map_err(|e| AdminUiError::Internal(format!("folder-templates file `{tmp}`: {e}")))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| AdminUiError::Internal(format!("folder-templates file `{path}`: {e}")))
}

#[cfg(test)]
mod tests {
    use crate::folder_templates::{
        builtin_templates, read_templates, templates_path, write_template,
    };

    #[test]
    fn templates_path_sits_next_to_the_groups_file() {
        assert_eq!(
            templates_path("/etc/ehrbase/admin-ui-groups.json"),
            "/etc/ehrbase/admin-ui-folder-templates.json"
        );
        // A bare filename resolves in the same (current) directory.
        assert_eq!(
            templates_path("admin-ui-groups.json"),
            "admin-ui-folder-templates.json"
        );
    }

    #[test]
    fn absent_file_yields_the_two_builtin_defaults() {
        let dir = std::env::temp_dir().join(format!("admin-ui-ft-absent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("admin-ui-folder-templates.json");
        let path = path.to_str().unwrap();

        let templates = read_templates(path).unwrap();
        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].name, "Episodes by year");
        assert_eq!(templates[1].name, "Clinical areas");
        // The built-in folders are spec-valid FOLDERs.
        assert_eq!(templates[0].folder.get("_type").unwrap(), "FOLDER");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn builtin_folders_are_well_formed() {
        for template in builtin_templates() {
            assert_eq!(template.folder.get("_type").unwrap(), "FOLDER");
            assert!(template.folder.get("archetype_node_id").is_some());
            assert!(template.folder.get("name").is_some());
            // No uid — the CDR assigns the OBJECT_VERSION_ID on create.
            assert!(template.folder.get("uid").is_none());
        }
    }

    #[test]
    fn write_seeds_builtins_then_round_trips_create_replace() {
        let dir = std::env::temp_dir().join(format!("admin-ui-ft-write-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("admin-ui-folder-templates.json");
        let path = path.to_str().unwrap();

        // First write seeds the two built-ins plus the new one.
        let folder = serde_json::json!({
            "_type": "FOLDER",
            "archetype_node_id": "openEHR-EHR-FOLDER.directory.v1",
            "name": {"_type": "DV_TEXT", "value": "root"},
            "folders": [],
            "items": []
        });
        write_template(path, "Custom", folder.clone()).unwrap();
        let templates = read_templates(path).unwrap();
        assert_eq!(templates.len(), 3);
        // Sorted by name: Clinical areas, Custom, Episodes by year.
        assert_eq!(templates[1].name, "Custom");

        // Replacing a name keeps the count stable.
        write_template(path, "Custom", folder).unwrap();
        assert_eq!(read_templates(path).unwrap().len(), 3);
        std::fs::remove_dir_all(&dir).ok();
    }
}
