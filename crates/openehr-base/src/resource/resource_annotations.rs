//! `RESOURCE_ANNOTATIONS` — path-keyed documentary annotations on a resource.
//!
//! openEHR class: `RESOURCE_ANNOTATIONS` (concrete), package `base.resource`.
//!
//! Object representing annotations on an archetype (or other authored
//! resource). These can be of various forms, with a documentation form
//! defined so far, structured as a three-level keyed table:
//! `[ [ [String value, String key], path key], language key]`.
//!
//! # PORT NOTE: chapter-inclusion discrepancy
//!
//! The `resource` package chapter
//! (`docs/research/spec-cache/BASE-1.2.0/resource/master02-resource_package.adoc`,
//! "Class Descriptions" section) `include::`s only `authored_resource.adoc`,
//! `translation_details.adoc`, `resource_description.adoc`, and
//! `resource_description_item.adoc` — it does **not** include
//! `resource_annotations.adoc`, even though the `uml_classes/` table for
//! `RESOURCE_ANNOTATIONS` exists and is the direct, only referenced type of
//! `AUTHORED_RESOURCE.annotations` (see `authored_resource.rs`). This looks
//! like an editorial omission in the published chapter's include-list rather
//! than an intentional exclusion of the class from the package. Transcribed
//! here anyway, since the type is load-bearing for `AUTHORED_RESOURCE` and
//! its UML table is present in the same spec-cache pull; flagged rather than
//! silently decided either way.
use std::collections::HashMap;

/// `RESOURCE_ANNOTATIONS` — object representing annotations on an
/// archetype.
///
/// # Transcription approach
///
/// Concrete class with no ancestors in the spec table (no `Inherit` row).
/// The single `documentation` attribute is a triply-nested
/// `Hash<String, Hash<String, Hash<String, String>>>`
/// (`language key -> path key -> tag key -> value`), transcribed per
/// `docs/PORTING.md` §6/§14.2 (`Hash<K,V>` → `HashMap<K,V>`) applied
/// recursively. The spec's own worked example:
///
/// ```text
/// documentation = <
///     ["en"] = <
///        ["/data[id2]"] = <
///            ["ui"] = <"passthrough">
///        >
///        ["/data[id2]/items[id3]"] = <
///            ["design note"] = <"this is a design note on Statement">
///            ["requirements note"] = <"this is a requirements note on Statement">
///            ["medline ref"] = <"this is a medline ref on Statement">
///        >
///     >
/// >
/// ```
///
/// The spec notes other sub-structures might use different keys (e.g. based
/// on programming languages or UI toolkits) but does not define any beyond
/// `documentation` in this table.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct ResourceAnnotations {
    /// `documentation`: `Hash<String, Hash<String, Hash<String, String>>>`,
    /// cardinality 1..1.
    ///
    /// Documentary annotations in a multi-level keyed structure: outer key
    /// is language, middle key is archetype path, inner key is a tag (e.g.
    /// `"design note"`, `"requirements note"`, `"medline ref"`), value is
    /// the annotation text.
    pub documentation: HashMap<String, HashMap<String, HashMap<String, String>>>,
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 resource §RESOURCE_ANNOTATIONS — docs/research/spec-cache/BASE-1.2.0/uml_classes/resource_annotations.adoc (Release-1.2.0 @ 9064413)
//   source_loc: uml_classes/resource_annotations.adoc §RESOURCE_ANNOTATIONS Class (not included by master02-resource_package.adoc's own Class Descriptions list — see PORT NOTE above)
//   confidence: medium
//   todos: 0
//   note: class table exists and is referenced by AUTHORED_RESOURCE.annotations but is not in the chapter's include:: list; flagged for reviewer confirmation rather than silently included or omitted.
// ─────────────────────────────────────────────
