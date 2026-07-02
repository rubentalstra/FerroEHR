//! `DATA_VALUE` — abstract parent of all `DV_` data value types.
//!
//! openEHR class: `DATA_VALUE` (abstract), package `rm.data_types` (root).
//! Inherits: `OPENEHR_DEFINITIONS`.
//!
//! Abstract parent of all `DV_` data value types. Serves as a common
//! ancestor of all data value types in openEHR models.
//!
//! Per `master03-introduction.adoc` §Overview: "The data types described
//! here are named with the class prefix `DV_`, and inherit from the class
//! `DATA_VALUE`. They have two distinct uses in reference models. Firstly,
//! they may be used as 'data values' in reference model structures wherever
//! the `DATA_VALUE` class appears, for example, in the EHR Reference Model
//! via the `ELEMENT.value` attribute. Additionally, specific subtypes of
//! the data types described here can also be used as attribute types in
//! other classes in reference models, such as date/times, coded terms and
//! so on."
//!
//! # Transcription approach
//!
//! `DATA_VALUE`'s published class table declares **no `Attributes` row at
//! all** — only the `Inherit` row (`OPENEHR_DEFINITIONS`) — unlike `UID` or
//! `OBJECT_ID`, which each carry exactly one shared `value: String`
//! attribute alongside their own closed-subtype-set enum. There is no
//! per-instance state to embed here, so this class does **not** get an
//! `XxxData` struct (ADR-001 §3/Refinements is for abstract classes *with*
//! attributes to embed; `DATA_VALUE` has none). Instead, per ADR-001 §1
//! (no-attribute abstract class → trait), `DATA_VALUE`'s role as a shared
//! ancestor is modelled as the lean marker trait [`DataValueApi`] below,
//! while its role as a closed, polymorphically-used field type (`ELEMENT.
//! value: DATA_VALUE`, and every other RM attribute typed `DATA_VALUE`) is
//! modelled as the closed [`DataValue`] enum per ADR-001 §4, exactly as the
//! invoking transcription task specifies.
//!
//! `DATA_VALUE inherits OPENEHR_DEFINITIONS` is the same "inherits a
//! constants-only class" shape already settled for the `Iso8601_type`/
//! `Time_Definitions` cluster (`crates/openehr-foundation/src/time/
//! iso8601_type.rs`; see `[[project-time-types-precedent]]` item 2 in agent
//! memory) and, within this same crate's dependency, for
//! `crates::openehr_base::definitions::openehr_definitions::
//! OpenehrDefinitions` itself (a zero-field struct of associated consts).
//! `OPENEHR_DEFINITIONS` carries no per-instance state, so nothing is
//! embedded here either — no concrete `DV_*` class transcribed in this
//! cluster calls into `OpenehrDefinitions::LOCAL_TERMINOLOGY_ID` or
//! `BasicDefinitions::*` directly, but the inheritance is documented here
//! for fidelity, and a descendant that does need those constants in scope
//! reaches for `openehr_base::definitions::openehr_definitions::
//! OpenehrDefinitions::*` directly (Rust has no struct-level inheritance to
//! bring them into scope automatically).
use crate::data_types::basic::dv_boolean::DvBoolean;
use crate::data_types::basic::dv_identifier::DvIdentifier;
use crate::data_types::basic::dv_state::DvState;
use crate::data_types::text::dv_coded_text::DvCodedText;
use crate::data_types::text::dv_paragraph::DvParagraph;
use crate::data_types::text::dv_text::DvText;
use crate::data_types::uri::dv_ehr_uri::DvEhrUri;
use crate::data_types::uri::dv_uri::DvUri;

// PORT NOTE: the following are forward-reference `use` paths into sibling
// data_types subpackages (basic/quantity/date_time/encapsulated) being
// transcribed concurrently by other agents in separate worktrees, per the
// invoking task's instruction to reference them by their eventual module
// path even though the files do not yet exist in this worktree. Phase A
// (`PORT_MASTER_PLAN.md` §4.1): nothing in this crate is required to
// compile before P17. Every module segment below (`quantity`, `date_time`,
// `encapsulated`) is a directory this transcription pass does not create;
// see the file-level PORT STATUS `note` for the itemised list.
use crate::data_types::date_time::dv_date::DvDate;
use crate::data_types::date_time::dv_date_time::DvDateTime;
use crate::data_types::date_time::dv_duration::DvDuration;
use crate::data_types::date_time::dv_time::DvTime;
use crate::data_types::encapsulated::dv_multimedia::DvMultimedia;
use crate::data_types::encapsulated::dv_parsable::DvParsable;
use crate::data_types::quantity::dv_count::DvCount;
use crate::data_types::quantity::dv_interval::DvInterval;
use crate::data_types::quantity::dv_ordered::DvOrdered;
use crate::data_types::quantity::dv_ordinal::DvOrdinal;
use crate::data_types::quantity::dv_proportion::DvProportion;
use crate::data_types::quantity::dv_quantity::DvQuantity;
use crate::data_types::quantity::dv_scale::DvScale;
use crate::data_types::time_specification::dv_general_time_specification::DvGeneralTimeSpecification;
use crate::data_types::time_specification::dv_periodic_time_specification::DvPeriodicTimeSpecification;
use openehr_foundation::serde_support::TypeName;
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for the abstract `DATA_VALUE`
/// class itself. Never used as a concrete instance's own discriminator
/// (every concrete `DV_*` class carries its own `TYPE_NAME`); recorded for
/// completeness and for any future reflective/registry code that needs the
/// abstract root's name (mirroring `Any::type_of`'s "Interval<Time>"-style
/// use case).
pub const TYPE_NAME: &str = "DATA_VALUE";

/// Marker/behaviour trait for `DATA_VALUE` and every `DV_*` descendant.
///
/// `DATA_VALUE` declares no attributes and no functions of its own in the
/// published spec table — this trait exists purely so RM code that needs
/// to be generic over "is a data value" (e.g. `ELEMENT.value`'s eventual
/// accessor methods) has a shared bound to write against, and so every
/// concrete `DV_*` struct in this crate has one place that names its
/// canonical `_type` discriminator string via [`DataValueApi::type_name`].
///
/// PORT NOTE: `type_name` is not a spec-declared function — it exists
/// solely to surface each concrete type's `pub const TYPE_NAME: &str`
/// (ADR-001 Refinements: "serde derives wait until P4... each concrete
/// class carries `pub const TYPE_NAME`") through a common trait method, so
/// callers holding a `&dyn DataValueApi` or a `DataValue` enum value can
/// recover the discriminator without a full `match`. This is scaffolding
/// for the P4 serde `_type` dispatch mentioned in the class-level doc
/// comment, not an invariant or behaviour drawn from the specification
/// itself.
pub trait DataValueApi {
    /// The canonical `_type` discriminator string for this concrete data
    /// value's class, e.g. `"DV_TEXT"`, `"DV_QUANTITY"`.
    fn type_name(&self) -> &'static str;
}

/// `DATA_VALUE` is abstract in the spec and is used polymorphically
/// wherever an attribute or return type is declared `DATA_VALUE` (most
/// prominently `ELEMENT.value` in `rm.data_structures`, but also
/// `EVENT<T>.data`, `HISTORY<T>.summary`, and any other RM attribute
/// typed as the data-value root). Per ADR-001 §4 (closed subtype set →
/// enum), every concrete `DV_*` class across the `data_types` package —
/// not just the ones transcribed in this pass — is collected into this
/// closed `enum`, one variant per concrete descendant.
///
/// # Variant scope and provenance
///
/// This enum spans **all** `data_types` subpackages (basic, text, quantity,
/// date_time, time_specification, encapsulated, uri), not just the ones
/// this transcription pass owns (basic, text, uri). Sibling agents are
/// concurrently transcribing the quantity, date_time, time_specification,
/// and encapsulated clusters in separate worktrees; the variants below
/// reference their eventual types via forward-reference `use` paths that
/// may not exist yet in this worktree (Phase A — nothing in this crate is
/// required to compile before P17). Only concrete, non-abstract classes
/// become variants; abstract classes in the hierarchy (`DV_ORDERED`,
/// `DV_QUANTIFIED`, `DV_AMOUNT`, `DV_ABSOLUTE_QUANTITY`, `DV_TEMPORAL`,
/// `DV_ENCAPSULATED`) are **not** represented here directly — they surface
/// only through their concrete descendants' variants, per ADR-001 §4.
///
/// `DV_STATE` and `DV_IDENTIFIER` are `DATA_VALUE` descendants (package
/// `basic`) transcribed in this pass and are included as ordinary
/// variants, matching every other concrete leaf class in this enum — the
/// task's explicit variant list above did not name them, but they are
/// genuine, non-abstract `DATA_VALUE` subtypes per their own class tables
/// (`dv_state.adoc`, `dv_identifier.adoc`) and this transcription includes
/// them for completeness; flagged here since their omission from the
/// task's own enumerated list looks like an oversight rather than a
/// deliberate exclusion (`DV_PARAGRAPH` and `DV_URI`/`DV_EHR_URI`, also not
/// explicitly named as omitted, are likewise included; only genuinely
/// abstract classes are left out).
///
/// PORT NOTE (P4, ADR-002): `DataValue` is `#[serde(untagged)]`, never
/// `#[serde(tag = "_type")]` — the former internally-tagged form (and its
/// per-variant renames) duplicated each payload's own `_type` key. Dispatch
/// is driven by each variant payload's own `TypeTag` field: `TypeTag`'s
/// `Deserialize` fails on a mismatched `_type` string, so serde's untagged
/// variant probing selects exactly the variant whose class name matches —
/// even between structure-identical classes (`DV_DATE`/`DV_TIME`/
/// `DV_DATE_TIME` are all `{value: String}` on the wire; only the tag
/// tells them apart). Variants are ordered structurally-richer-first
/// (`CodedText` before bare `Text`, object payloads before the sparse
/// `{value}` family, `Boolean` last) so tag-less input in
/// concrete-declared slots resolves to the most specific structural match;
/// within the mutually tag-distinguished `{value: String}` family the
/// relative order only matters for tag-less input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataValue {
    /// `DV_CODED_TEXT` (package `text`).
    ///
    /// PORT NOTE: the task's variant list names `CodedText(DvCodedText)`
    /// explicitly, alongside `Text(DvText)` — both are included verbatim
    /// as instructed, even though `DvText` (below) already has a `Coded`
    /// arm that can hold a `DvCodedText`. This creates two structurally
    /// different ways to place a coded-text value into a `DataValue`
    /// (`DataValue::Text(DvText::Coded(x))` vs `DataValue::CodedText(x)`);
    /// left as specified rather than silently collapsing one path, and
    /// flagged here as a design question for the P4/P17 wiring pass to
    /// resolve (most likely by dropping the inner `DvText::Coded` arm's
    /// reachability from `DataValue` in favour of this direct variant, or
    /// vice versa). Listed before `Text` per ADR-002 richer-first ordering,
    /// so a `DV_CODED_TEXT` payload lands on this direct variant rather
    /// than the nested `Text(DvText::Coded(_))` path.
    CodedText(DvCodedText),
    /// `DV_TEXT` (package `text`), the "bare" (uncoded) form.
    ///
    /// PORT NOTE: [`DvText`] is itself the ADR-001-Refinements-style enum
    /// spanning `DV_TEXT`/`DV_CODED_TEXT` (see `text::dv_text` for the full
    /// rationale) — nested here as the outer `DataValue::Text` variant so a
    /// `DATA_VALUE`-typed field can hold either form without this outer
    /// enum needing its own separate `Coded` variant duplicating that
    /// distinction. `DataValue::Text(DvText::Coded(_))` and
    /// `DataValue::CodedText(_)` above are therefore two different paths to
    /// a coded value; see the `CodedText` variant's own note for why both
    /// exist.
    Text(DvText),
    /// `DV_PARAGRAPH` (package `text`, deprecated but legal).
    Paragraph(DvParagraph),
    /// `DV_STATE` (package `basic`).
    ///
    /// PORT NOTE: not named in the task's explicit variant enumeration;
    /// included per the class table (`DV_STATE inherits DATA_VALUE`,
    /// concrete, non-abstract) — see the enum-level doc comment.
    State(DvState),
    /// `DV_IDENTIFIER` (package `basic`).
    ///
    /// PORT NOTE: not named in the task's explicit variant enumeration;
    /// included per the class table (`DV_IDENTIFIER inherits DATA_VALUE`,
    /// concrete, non-abstract) — see the enum-level doc comment.
    Identifier(DvIdentifier),
    /// `DV_ORDINAL` (package `quantity`; forward-reference, sibling
    /// worktree).
    Ordinal(DvOrdinal),
    /// `DV_SCALE` (package `quantity`; forward-reference, sibling
    /// worktree).
    Scale(DvScale),
    /// `DV_QUANTITY` (package `quantity`; forward-reference, sibling
    /// worktree).
    Quantity(DvQuantity),
    /// `DV_COUNT` (package `quantity`; forward-reference, sibling
    /// worktree).
    Count(DvCount),
    /// `DV_PROPORTION` (package `quantity`; forward-reference, sibling
    /// worktree).
    Proportion(DvProportion),
    /// `DV_INTERVAL<T>` (package `quantity`; forward-reference, sibling
    /// worktree), constrained to `T: DvOrdered` per ADR-001 §5 — this is
    /// the generic constrained by the closed [`DvOrdered`] enum the
    /// `quantity` cluster owns, as directed by the invoking task.
    Interval(DvInterval<DvOrdered>),
    /// `DV_MULTIMEDIA` (package `encapsulated`; forward-reference, sibling
    /// worktree).
    Multimedia(DvMultimedia),
    /// `DV_PARSABLE` (package `encapsulated`; forward-reference, sibling
    /// worktree).
    Parsable(DvParsable),
    /// `DV_PERIODIC_TIME_SPECIFICATION` (package `time_specification`;
    /// forward-reference, sibling worktree).
    PeriodicTimeSpecification(DvPeriodicTimeSpecification),
    /// `DV_GENERAL_TIME_SPECIFICATION` (package `time_specification`;
    /// forward-reference, sibling worktree).
    GeneralTimeSpecification(DvGeneralTimeSpecification),
    /// `DV_DURATION` (package `date_time`; forward-reference, sibling
    /// worktree).
    Duration(DvDuration),
    /// `DV_DATE` (package `date_time`; forward-reference, sibling
    /// worktree). Structure-identical to `DV_TIME`/`DV_DATE_TIME` on the
    /// wire — only the payload's own `TypeTag` distinguishes them.
    Date(DvDate),
    /// `DV_TIME` (package `date_time`; forward-reference, sibling
    /// worktree).
    Time(DvTime),
    /// `DV_DATE_TIME` (package `date_time`; forward-reference, sibling
    /// worktree).
    DateTime(DvDateTime),
    /// `DV_URI` (package `uri`).
    Uri(DvUri),
    /// `DV_EHR_URI` (package `uri`).
    EhrUri(DvEhrUri),
    /// `DV_BOOLEAN` (package `basic`). Sparsest payload (`{value: bool}`)
    /// — listed last per ADR-002 richer-first ordering.
    Boolean(DvBoolean),
}

impl DataValueApi for DataValue {
    fn type_name(&self) -> &'static str {
        match self {
            DataValue::CodedText(v) => v.type_name(),
            DataValue::Text(v) => v.type_name(),
            DataValue::Paragraph(v) => v.type_name(),
            DataValue::State(v) => v.type_name(),
            DataValue::Identifier(v) => v.type_name(),
            // TODO(port): sibling `quantity` types do not yet implement
            // `DataValueApi` (concurrent conversion in separate worktrees);
            // their arms cannot delegate until those land and this file is
            // wired at P17. Left as `todo!()` per-arm rather than omitting
            // the arms, so the match stays exhaustive against the enum
            // defined above. The `date_time`/`time_specification`/
            // `encapsulated` arms now return the canonical name via each
            // payload's ADR-002 `TypeName` impl.
            DataValue::Ordinal(_) => todo!("DvOrdinal::type_name pending sibling transcription"),
            DataValue::Scale(_) => todo!("DvScale::type_name pending sibling transcription"),
            DataValue::Quantity(_) => todo!("DvQuantity::type_name pending sibling transcription"),
            DataValue::Count(_) => todo!("DvCount::type_name pending sibling transcription"),
            DataValue::Proportion(_) => {
                todo!("DvProportion::type_name pending sibling transcription")
            }
            DataValue::Interval(_) => todo!("DvInterval::type_name pending sibling transcription"),
            DataValue::Multimedia(_) => <DvMultimedia as TypeName>::NAME,
            DataValue::Parsable(_) => <DvParsable as TypeName>::NAME,
            DataValue::PeriodicTimeSpecification(_) => {
                <DvPeriodicTimeSpecification as TypeName>::NAME
            }
            DataValue::GeneralTimeSpecification(_) => {
                <DvGeneralTimeSpecification as TypeName>::NAME
            }
            DataValue::Duration(_) => <DvDuration as TypeName>::NAME,
            DataValue::Date(_) => <DvDate as TypeName>::NAME,
            DataValue::Time(_) => <DvTime as TypeName>::NAME,
            DataValue::DateTime(_) => <DvDateTime as TypeName>::NAME,
            DataValue::Uri(v) => v.type_name(),
            DataValue::EhrUri(v) => v.type_name(),
            DataValue::Boolean(v) => v.type_name(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR-002: untagged dispatch is tag-driven, not declaration-order
    /// driven — a `DV_TIME` payload must reach the `Time` variant even
    /// though structure-identical `{value: String}` variants (`Duration`,
    /// `Date`) are declared earlier in the enum; each earlier variant's own
    /// `TypeTag` rejects the mismatched `_type` during probing.
    #[test]
    fn untagged_dispatch_selects_dv_time_by_tag_not_declaration_order() {
        let v: DataValue =
            serde_json::from_str(r#"{"_type":"DV_TIME","value":"10:00:00"}"#).unwrap();
        assert!(matches!(v, DataValue::Time(_)));
    }

    /// A date/time-family `DataValue` round-trips: the payload's own
    /// `TypeTag` emits `_type` first on output and steers re-input back to
    /// the same variant.
    ///
    /// PORT NOTE: written against the date/time family rather than the
    /// task-suggested `DvQuantity` — the `quantity` cluster's ADR-002
    /// conversion is mid-flight in a sibling pass (no `TypeTag` fields there
    /// yet at the time of this file's conversion); extend with a
    /// `DataValue::Quantity` round-trip once that cluster lands.
    #[test]
    fn date_time_family_data_value_round_trips() {
        for (json, want_date, want_time, want_date_time) in [
            (
                r#"{"_type":"DV_DATE","value":"2026-07-02"}"#,
                true,
                false,
                false,
            ),
            (
                r#"{"_type":"DV_TIME","value":"10:00:00"}"#,
                false,
                true,
                false,
            ),
            (
                r#"{"_type":"DV_DATE_TIME","value":"2026-07-02T10:00:00"}"#,
                false,
                false,
                true,
            ),
        ] {
            let v: DataValue = serde_json::from_str(json).unwrap();
            assert_eq!(matches!(v, DataValue::Date(_)), want_date, "{json}");
            assert_eq!(matches!(v, DataValue::Time(_)), want_time, "{json}");
            assert_eq!(
                matches!(v, DataValue::DateTime(_)),
                want_date_time,
                "{json}"
            );
            let out = serde_json::to_string(&v).unwrap();
            assert_eq!(out, json, "canonical output must match canonical input");
            let back: DataValue = serde_json::from_str(&out).unwrap();
            assert_eq!(back, v);
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types (root) — docs/research/spec-cache/RM-1.1.0/uml_classes/data_value.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master03-introduction.adoc §Overview / data_value.adoc §DATA_VALUE Class
//   confidence: medium
//   todos: 1
//   note: DATA_VALUE has no attributes of its own (only an OPENEHR_DEFINITIONS constants-inherit), so it gets a lean DataValueApi marker trait per ADR-001 §1, not a Data+enum+Api triple; the six quantity-cluster arms of the DataValueApi impl remain todo!() pending that cluster's own DataValueApi/TypeName landing (6 todo!() calls under 1 TODO(port) marker), while the date_time/time_specification/encapsulated arms now return each payload's ADR-002 TypeName::NAME. DV_STATE/DV_IDENTIFIER/DV_PARAGRAPH/DV_URI/DV_EHR_URI included as variants despite not being named in the task's explicit list (flagged as likely oversight, not exclusion); Text(DvText) vs CodedText(DvCodedText) overlap (both can represent coded text) flagged as a design question for P4/P17. P4 (ADR-002): the former tagged-enum form + per-variant renames are replaced by #[serde(untagged)] — dispatch runs on each payload's own TypeTag (wrong `_type` fails that variant, so probing is tag-driven even for the structure-identical DV_DATE/DV_TIME/DV_DATE_TIME family); variants reordered richer-first (CodedText before Text, object payloads before the {value} family, Boolean last); in-file tests pin tag-driven dispatch (DV_TIME must not land on the earlier-declared Date/Duration) and date/time-family round-trips; DvQuantity round-trip deferred until the quantity cluster's sibling conversion lands (PORT-NOTEd in the test).
// ─────────────────────────────────────────────
