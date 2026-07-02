//! `VERSIONED_FOLDER` — a version-controlled hierarchy of `FOLDER`s.
//!
//! openEHR class: `VERSIONED_FOLDER`, package `common.directory`.
//! Inherits: `VERSIONED_OBJECT<T>` (bound to `T = FOLDER`).
//!
//! A version-controlled hierarchy of `FOLDER`s giving the effect of a
//! directory. The `VERSIONED_FOLDER` class is the binding of
//! `VERSIONED_OBJECT<T>` to the class `FOLDER`, i.e. it is a
//! `VERSIONED_OBJECT<FOLDER>`. This means that each of its versions is a
//! Folder structure rather than a single Folder. It provides a means of
//! versioning `FOLDER` structures over time, which is useful in the EHR,
//! Demographics service or anywhere else where Folders are used to group
//! things.
use crate::common::change_control::versioned_object::VersionedObject;
use crate::common::directory::folder::Folder;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 (Refinements), `serde` derives wait until P4.
pub const TYPE_NAME: &str = "VERSIONED_FOLDER";

/// `VERSIONED_FOLDER` — `VERSIONED_OBJECT<FOLDER>`.
///
/// PORT NOTE: the spec's own per-class table declares no additional
/// attributes or functions beyond the `Inherit: VERSIONED_OBJECT` row, so
/// this binding is transcribed as a type alias rather than a wrapping
/// struct — there is no independent state or behaviour to add. This
/// mirrors how a Java/Eiffel-style "bind the generic parameter, add
/// nothing" subtype is representable directly in Rust's own generic
/// system, without composition. If a future RM revision adds attributes
/// specific to `VERSIONED_FOLDER`, this should become a newtype struct
/// wrapping `VersionedObject<Folder>` (per ADR-001 §3) instead of a bare
/// alias.
pub type VersionedFolder = VersionedObject<Folder>;

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.directory §VERSIONED_FOLDER — docs/research/spec-cache/RM-1.1.0/uml_classes/versioned_folder.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-directory_package.adoc §Class Descriptions / versioned_folder.adoc §VERSIONED_FOLDER Class
//   confidence: high
//   todos: 0
//   note: transcribed as a generic type alias (VersionedObject<Folder>) since the spec table declares no attributes/functions of its own beyond the Inherit row; upgrade to a wrapping struct if a later RM revision adds VERSIONED_FOLDER-specific state.
// ─────────────────────────────────────────────
