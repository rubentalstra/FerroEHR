//! `CONTENT_ITEM` — abstract ancestor of all concrete content types.
//!
//! openEHR class: `CONTENT_ITEM` (abstract), package `rm.ehr.composition`.
//! Inherits: `LOCATABLE`.
//!
//! Abstract ancestor of all concrete content types.
//!
//! # Enum boundary (ADR-001 §4)
//!
//! `CONTENT_ITEM` is abstract with **zero** attributes of its own beyond
//! the embedded `LOCATABLE` state (the published class table declares no
//! `Attributes`, `Functions`, or `Invariants` sections). It is used
//! polymorphically as the element type of `SECTION.items` and
//! `COMPOSITION.content` (both `List<CONTENT_ITEM>`). Per ADR-001 §4
//! (closed subtype set → enum) and this transcription's explicit scope,
//! [`ContentItem`] closes over the seven concrete/leaf-usable descendants:
//!
//! * [`super::section::Section`] (direct `CONTENT_ITEM` subtype);
//! * the four concrete `CARE_ENTRY` subtypes reachable through the
//!   abstract `ENTRY`/`CARE_ENTRY` chain — [`super::admin_entry::AdminEntry`]
//!   (via `ENTRY` directly), [`super::observation::Observation`],
//!   [`super::evaluation::Evaluation`], [`super::instruction::Instruction`],
//!   [`super::action::Action`] (via `CARE_ENTRY`);
//! * `GENERIC_ENTRY`, forward-referenced as
//!   `crate::integration::generic_entry::GenericEntry` — RM 1.1.0's
//!   `rm.integration` package (PORT_MASTER_PLAN.md §7.1) declares
//!   `GENERIC_ENTRY` as a `CONTENT_ITEM` descendant used to wrap
//!   non-RM-typed content; it is a sibling cluster being transcribed
//!   separately in this same phase and is not yet on disk.
//!
//! `ENTRY` and `CARE_ENTRY` are themselves **abstract** (per
//! `docs/research/spec-cache/RM-1.1.0/uml_classes/entry.adoc` and
//! `care_entry.adoc`, both titled `__ENTRY (abstract)__` /
//! `__CARE_ENTRY (abstract)__`) and therefore are **not** given their own
//! enum variants here — only their concrete leaf descendants are, matching
//! the "one enum variant per concrete class" rule (ADR-001 §4) rather than
//! one variant per every ancestor in the chain.
use crate::common::archetyped::locatable::LocatableData; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention.

/// Embedded attribute state of the abstract `CONTENT_ITEM` class.
///
/// Per ADR-001 §3, concrete `CONTENT_ITEM` descendants embed this struct by
/// composition. `CONTENT_ITEM` itself declares no attribute beyond the
/// inherited `LOCATABLE` state, so this struct is a thin, single-field
/// wrapper rather than adding fields of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentItemData {
    /// Embedded `LOCATABLE` state.
    pub locatable: LocatableData,
}

/// `CONTENT_ITEM` — the closed set of concrete content types that may
/// appear in `SECTION.items` / `COMPOSITION.content`.
///
/// See the module-level doc comment for the enum-boundary rationale
/// (ADR-001 §4) and why `ENTRY`/`CARE_ENTRY` do not get their own variants.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentItem {
    /// `SECTION`.
    Section(super::section::Section),
    /// `ADMIN_ENTRY`.
    AdminEntry(super::admin_entry::AdminEntry),
    /// `OBSERVATION`.
    Observation(super::observation::Observation),
    /// `EVALUATION`.
    Evaluation(super::evaluation::Evaluation),
    /// `INSTRUCTION`.
    Instruction(super::instruction::Instruction),
    /// `ACTION`.
    Action(super::action::Action),
    /// `GENERIC_ENTRY` — RM 1.1.0 `rm.integration` package.
    ///
    /// TODO(port): forward-reference; `rm.integration.GENERIC_ENTRY` is a
    /// sibling cluster being transcribed separately in this phase and is
    /// not yet on disk at `crate::integration::generic_entry`.
    GenericEntry(crate::integration::generic_entry::GenericEntry),
}

/// Marker/accessor trait shared by every `CONTENT_ITEM` variant, exposing
/// the abstract class's embedded `LOCATABLE` state uniformly.
pub trait ContentItemApi {
    /// Access to the embedded [`ContentItemData`].
    fn content_item_data(&self) -> &ContentItemData;
}

impl ContentItem {
    /// Access to the embedded [`ContentItemData`] shared by every variant.
    ///
    /// TODO(port): each concrete variant's own `ContentItemApi` impl (via
    /// its `LocatableData`-embedding chain) is not yet wired, since several
    /// variants (`Observation`, `Evaluation`, `Instruction`, `Action`) only
    /// carry `LOCATABLE` state indirectly through `CARE_ENTRY`/`ENTRY`,
    /// whose `EntryData`/`CareEntryData` structs do not yet expose a
    /// direct `&ContentItemData` accessor. Left as `todo!()` rather than
    /// guessing the accessor chain ahead of `common::archetyped::locatable`
    /// landing.
    pub fn content_item_data(&self) -> &ContentItemData {
        todo!(
            "port: ContentItem::content_item_data needs the LocatableData accessor chain through EntryData/CareEntryData, pending common::archetyped::locatable"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.composition — docs/research/spec-cache/RM-1.1.0/uml_classes/content_item.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-content_package.adoc §Class Descriptions / content_item.adoc §CONTENT_ITEM Class
//   confidence: medium
//   todos: 3
//   note: closed enum per ADR-001 §4 covering Section + the four concrete CARE_ENTRY leaves + AdminEntry + the forward-referenced GenericEntry (rm.integration sibling cluster, not yet on disk); ENTRY/CARE_ENTRY deliberately excluded as variants since both are abstract; content_item_data() accessor chain stubbed pending LOCATABLE.
// ─────────────────────────────────────────────
