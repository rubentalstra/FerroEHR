---
name: folder-directory-model-location
description: Where the openEHR FOLDER / directory / VERSIONED_FOLDER model + invariants live in the vendored RM/BASE spec, and their generated Rust bindings
metadata:
  type: reference
---

FOLDER / directory model spec navigation (RM 1.2.0 / BASE 1.3.0):

- Directory package overview + §Paths (name-uniqueness modifier convention, NOT an invariant): `RM/docs/common/master05-directory_package.adoc`. It `include::`s the class tables from `RM/docs/UML/classes/org.openehr.rm.common.folder.adoc` and `...versioned_folder.adoc`.
- **FOLDER class table** (`org.openehr.rm.common.folder.adoc`): attrs `items 0..1 List<OBJECT_REF>`, `folders 0..1 List<FOLDER>`, `details 0..1 ITEM_STRUCTURE`. **FOLDER declares NO own invariants** (empty invariant section). name/archetype_node_id/uid/links inherited from LOCATABLE (`org.openehr.rm.common.locatable.adoc` — invariants Links_valid, Archetyped_valid, Archetype_node_id_valid; no sibling-name-uniqueness rule anywhere).
- **VERSIONED_FOLDER** = `VERSIONED_OBJECT<FOLDER>` (`org.openehr.rm.common.versioned_folder.adoc`, no own attrs). VERSIONED_OBJECT semantics: `org.openehr.rm.common.versioned_object.adoc`.
- **EHR.directory vs EHR.folders** (RM 1.1.0+ added folders via SPECRM-55): EHR class table `org.openehr.rm.ehr.ehr.adoc` — `directory 0..1 OBJECT_REF` + `folders 0..1 List<OBJECT_REF>`; invariants Directory_valid, Folders_valid, Directory_in_folders (directory = folders.item(1)). Prose: `RM/docs/ehr/master04-ehr_package.adoc §Folders` (L102-131).
- **Deletion / lifecycle**: `RM/docs/common/master06-change_control_package.adoc §Logical Deletion` (L190+) + L58-76 — logical delete = new ORIGINAL_VERSION with data=Void, change_type 523|deleted|.
- OBJECT_REF / OBJECT_VERSION_ID: `BASE/docs/UML/classes/org.openehr.base.base_types.object_ref.adoc` (namespace/type/id) + `...object_version_id.adoc`.
- Generated Rust: `crates/openehr-rm/src/common/directory/folder.rs` (struct Folder), `crates/openehr-base/src/base_types/identification/object_ref.rs` (ObjectRef untagged enum + ObjectRefData), `.../object_version_id.rs`.
- Canonical JSON fixtures: `crates/openehr-its/tests/vendor/openehr_sdk/folder/canonical_json/*.json` (incl. `duplicate_folder_names.json` — confirms sibling dup names round-trip; RM does not forbid).
