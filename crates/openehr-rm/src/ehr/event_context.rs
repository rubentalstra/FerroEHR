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
use crate::common::generic::participation::Participation;
use crate::common::generic::party_identified::PartyIdentified;
use crate::data_structures::item_structure::ItemStructure;
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::text::dv_coded_text::DvCodedText;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use openehr_term::{
    OpenehrTerminologyGroupIdentifiers, TerminologyAccess, TerminologyCode, TerminologyService,
};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
///
/// Being `PATHABLE`-not-`LOCATABLE` changes this class's *fields* (no
/// `LocatableData` embed), not its `_type`: the pinned ITS-JSON schema
/// defines `EVENT_CONTEXT` as a concrete class with its own `_type` const,
/// so it self-tags like every other concrete class.
pub const TYPE_NAME: &str = "EVENT_CONTEXT";

/// `EVENT_CONTEXT` — the clinical session context of a `COMPOSITION`.
///
/// A concrete, non-abstract class (unlike `ENTRY`/`CARE_ENTRY`); embedded
/// directly (not via an enum) as `COMPOSITION.context: Option<EventContext>`
/// since `EVENT_CONTEXT` has no subtypes. No `LocatableData` embed (settled
/// `PATHABLE`-not-`LOCATABLE` hazard, see module doc comment), so this is a
/// plain struct with no `#[serde(flatten)]` fields of its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventContext {
    /// Canonical `_type` discriminator (`"EVENT_CONTEXT"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `start_time`: start time of the clinical session or other kind of
    /// event during which a provider performs a service of any kind for
    /// the patient.
    pub start_time: DvDateTime,

    /// `end_time`: optional end time of the clinical session.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub end_time: Option<DvDateTime>,

    /// `location`: the actual location where the session occurred, e.g.
    /// "microbiology lab 2", "home", "ward A3" and so on.
    ///
    /// Invariant `location_valid`: `location /= Void implies not
    /// location.is_empty` — see [`EventContext::invariant_location_valid`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub location: Option<String>,

    /// `setting`: the setting in which the clinical session took place.
    /// Coded using the openEHR Terminology `setting` group.
    ///
    /// Invariant `Setting_valid`: `Terminology (Terminology_id_openehr)
    /// .has_code_for_group_id (Group_id_setting, setting.defining_code)` —
    /// see [`EventContext::invariant_setting_valid`].
    pub setting: DvCodedText,

    /// `other_context`: other optional context which will be archetyped.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub other_context: Option<ItemStructure>,

    /// `health_care_facility`: the health care facility under whose care
    /// the event took place. This is the most specific workgroup or
    /// delivery unit within a care delivery enterprise that has an
    /// official identifier in the health system, and can be used to
    /// ensure medico-legal accountability.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub health_care_facility: Option<PartyIdentified>,

    /// `participations`: parties involved in the healthcare event. These
    /// would normally include the physician(s) and often the patient (but
    /// not the latter if the clinical session is a pathology test for
    /// example).
    ///
    /// Invariant `Participations_validity`: `participations /= Void
    /// implies not participations.is_empty` — see
    /// [`EventContext::invariant_participations_validity`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub participations: Option<Vec<Participation>>,
}

impl TypeName for EventContext {
    const NAME: &'static str = TYPE_NAME;
}

impl EventContext {
    /// Invariant `Setting_valid`: `Terminology (Terminology_id_openehr)
    /// .has_code_for_group_id (Group_id_setting, setting.defining_code)`.
    ///
    /// Terminology-bound invariant (ADR-003 §8): checks `setting.defining_code`
    /// against the openEHR `setting` group.
    #[must_use]
    pub fn invariant_setting_valid(&self, terminology: &TerminologyService) -> bool {
        terminology
            .terminology(OpenehrTerminologyGroupIdentifiers::TERMINOLOGY_ID_OPENEHR)
            .is_some_and(|access| {
                access.has_code_for_group_id(
                    OpenehrTerminologyGroupIdentifiers::GROUP_ID_SETTING,
                    &TerminologyCode::new(
                        self.setting.defining_code.terminology_id.value(),
                        self.setting.defining_code.code_string.clone(),
                    ),
                )
            })
    }

    /// Invariant `Participations_validity`: `participations /= Void implies
    /// not participations.is_empty`.
    #[must_use]
    pub fn invariant_participations_validity(&self) -> bool {
        self.participations.as_ref().is_none_or(|p| !p.is_empty())
    }

    /// Invariant `location_valid`: `location /= Void implies not
    /// location.is_empty`.
    #[must_use]
    pub fn invariant_location_valid(&self) -> bool {
        self.location.as_ref().is_none_or(|l| !l.is_empty())
    }

    // TODO(port): `PATHABLE` functions (`parent()`, `item_at_path()`,
    // `items_at_path()`, `path_exists()`, `path_unique()`,
    // `path_of_item()`) are inherited via the not-yet-transcribed
    // `Pathable` trait (forward-referenced as
    // `crate::common::pathable::Pathable`); this type will `impl Pathable
    // for EventContext` once that trait lands. `parent()` must resolve to
    // `Weak<..>`/index, never an owning back-reference, per
    // `.claude/rules/rm-transcription.md`.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_text::DvTextData;
    use openehr_base::identification::object_id::ObjectIdData;
    use openehr_base::identification::terminology_id::TerminologyId;

    fn date_time(value: &str) -> DvDateTime {
        serde_json::from_value(serde_json::json!({ "value": value }))
            .expect("test DV_DATE_TIME literal deserializes")
    }

    fn setting(code: &str) -> DvCodedText {
        DvCodedText {
            type_tag: TypeTag::new(),
            text: DvTextData {
                value: "setting".to_string(),
                hyperlink: None,
                formatting: None,
                mappings: None,
                language: None,
                encoding: None,
            },
            defining_code: CodePhrase {
                type_tag: TypeTag::new(),
                terminology_id: TerminologyId {
                    type_tag: TypeTag::new(),
                    object_id: ObjectIdData {
                        value: "openehr".to_string(),
                    },
                },
                code_string: code.to_string(),
                preferred_term: None,
            },
        }
    }

    fn event_context(setting_code: &str) -> EventContext {
        EventContext {
            type_tag: TypeTag::new(),
            start_time: date_time("2020-01-01T09:00:00"),
            end_time: None,
            location: None,
            setting: setting(setting_code),
            other_context: None,
            health_care_facility: None,
            participations: None,
        }
    }

    #[test]
    fn setting_valid_checks_the_setting_group() {
        let terminology = TerminologyService::bundled().expect("bundled terminology parses");
        // 225 = "home" in the openEHR "setting" group.
        assert!(event_context("225").invariant_setting_valid(terminology));
        assert!(!event_context("999999").invariant_setting_valid(terminology));
    }

    #[test]
    fn location_and_participations_invariants() {
        let mut ctx = event_context("225");
        assert!(ctx.invariant_location_valid()); // None: valid
        assert!(ctx.invariant_participations_validity()); // None: valid

        ctx.location = Some(String::new()); // present-but-empty: invalid
        assert!(!ctx.invariant_location_valid());
        ctx.location = Some("ward A3".to_string());
        assert!(ctx.invariant_location_valid());

        ctx.participations = Some(Vec::new()); // present-but-empty: invalid
        assert!(!ctx.invariant_participations_validity());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.composition — docs/research/spec-cache/RM-1.1.0/uml_classes/event_context.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master05-composition_package.adoc §Class Descriptions / event_context.adoc §EVENT_CONTEXT Class
//   confidence: high
//   todos: 1
//   note: PATHABLE-not-LOCATABLE settled hazard applied (no LocatableData, no uid/name). P5/ADR-003 §8: Setting_valid (terminology-bound, &TerminologyService), Participations_validity and location_valid (structural) all implemented, pinned by unit tests. The one remaining TODO(port) is the PATHABLE `Pathable`-trait function forwarding (parent()/item_at_path/…), which awaits the common::pathable trait — a legitimate cited deferral. P4/ADR-002: self-tagging TypeTag<Self> + TypeName; Option fields skip-if-none.
// ─────────────────────────────────────────────
