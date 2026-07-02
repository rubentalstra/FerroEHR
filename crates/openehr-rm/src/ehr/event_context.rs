//! `EVENT_CONTEXT` — context information of a healthcare event.
//!
//! openEHR class: `EVENT_CONTEXT`, package `rm.ehr.composition`.
//! Inherits: `PATHABLE`.
//!
//! Documents the context information of a healthcare event involving the
//! subject of care and the health system. The context information recorded
//! here is independent of the attributes recorded in the version audit,
//! which document the "system interaction" context, i.e. the context of a
//! user interacting with the health record system. Healthcare events
//! include patient contacts, and any other business activity, such as
//! pathology investigations which take place on behalf of the patient.
//!
//! # `PATHABLE`, not `LOCATABLE` (settled hazard)
//!
//! `EVENT_CONTEXT` inherits `PATHABLE` directly
//! (`docs/research/spec-cache/RM-1.1.0/uml_classes/event_context.adoc`
//! §Inherit), **not** `LOCATABLE`. `PATHABLE`
//! (`uml_classes/pathable.adoc`) is itself abstract with zero attributes —
//! only the abstract pathing functions `parent()`, `item_at_path()`,
//! `items_at_path()`, `path_exists()`, `path_unique()`, `path_of_item()` —
//! so per ADR-001 §1 (abstract class without attributes → trait) it
//! transcribes as a trait, not an embeddable data struct. `LOCATABLE` is
//! the subtype that adds the `name`/`archetype_node_id`/`uid`/`links`/
//! `archetype_details`/`feeder_audit` attributes
//! (`uml_classes/locatable.adoc`). Consequently this struct has **no**
//! `LocatableData` embed and **no** `uid`/`name`/`archetype_node_id`
//! fields — do not add them; `.claude/rules/rm-transcription.md` names
//! this exact mistake as a settled hazard not to relitigate.
use crate::common::generic::participation::Participation; // TODO(port): forward-reference; not yet transcribed.
use crate::common::generic::party_identified::PartyIdentified; // TODO(port): forward-reference; not yet transcribed.
use crate::data_structures::item_structure::ItemStructure; // TODO(port): forward-reference; not yet transcribed. Path matches the sibling ehr_status.rs/ehr_access.rs convention (data_structures has no UML subpackage grouping, unlike data_types).
use crate::data_types::date_time::dv_date_time::DvDateTime; // TODO(port): forward-reference; not yet transcribed.
use crate::data_types::text::dv_coded_text::DvCodedText; // TODO(port): forward-reference; not yet transcribed.

/// Canonical `_type` discriminator string for this class in serialized
/// form. serde derive is deferred to P4 (ADR-001 "Refinements"); until
/// then every concrete class carries this const.
pub const TYPE_NAME: &str = "EVENT_CONTEXT";

/// `EVENT_CONTEXT` — the clinical session context of a `COMPOSITION`.
///
/// A concrete, non-abstract class (unlike `ENTRY`/`CARE_ENTRY`); embedded
/// directly (not via an enum) as `COMPOSITION.context: Option<EventContext>`
/// since `EVENT_CONTEXT` has no subtypes.
#[derive(Debug, Clone, PartialEq)]
pub struct EventContext {
    /// `start_time`: start time of the clinical session or other kind of
    /// event during which a provider performs a service of any kind for
    /// the patient.
    pub start_time: DvDateTime,

    /// `end_time`: optional end time of the clinical session.
    pub end_time: Option<DvDateTime>,

    /// `location`: the actual location where the session occurred, e.g.
    /// "microbiology lab 2", "home", "ward A3" and so on.
    ///
    /// Invariant `location_valid`: `location /= Void implies not
    /// location.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl.
    pub location: Option<String>,

    /// `setting`: the setting in which the clinical session took place.
    /// Coded using the openEHR Terminology `setting` group.
    ///
    /// Invariant `Setting_valid`: `Terminology (Terminology_id_openehr)
    /// .has_code_for_group_id (Group_id_setting, setting.defining_code)`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub setting: DvCodedText,

    /// `other_context`: other optional context which will be archetyped.
    pub other_context: Option<ItemStructure>,

    /// `health_care_facility`: the health care facility under whose care
    /// the event took place. This is the most specific workgroup or
    /// delivery unit within a care delivery enterprise that has an
    /// official identifier in the health system, and can be used to
    /// ensure medico-legal accountability.
    pub health_care_facility: Option<PartyIdentified>,

    /// `participations`: parties involved in the healthcare event. These
    /// would normally include the physician(s) and often the patient (but
    /// not the latter if the clinical session is a pathology test for
    /// example).
    ///
    /// Invariant `Participations_validity`: `participations /= Void
    /// implies not participations.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced.
    pub participations: Option<Vec<Participation>>,
}

impl EventContext {
    // TODO(port): `PATHABLE` functions (`parent()`, `item_at_path()`,
    // `items_at_path()`, `path_exists()`, `path_unique()`,
    // `path_of_item()`) are inherited via the not-yet-transcribed
    // `Pathable` trait (forward-referenced as
    // `crate::common::pathable::Pathable`); this type will `impl Pathable
    // for EventContext` once that trait lands. `parent()` must resolve to
    // `Weak<..>`/index, never an owning back-reference, per
    // `.claude/rules/rm-transcription.md`.
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.composition — docs/research/spec-cache/RM-1.1.0/uml_classes/event_context.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-composition_package.adoc §Class Descriptions / event_context.adoc §EVENT_CONTEXT Class
//   confidence: high
//   todos: 9
//   note: PATHABLE-not-LOCATABLE settled hazard applied (no LocatableData, no uid/name); Pathable trait impl deferred until common::pathable lands; three invariants and the Pathable-function forwarding left unimplemented; most of the 9 markers are forward-reference import comments.
// ─────────────────────────────────────────────
