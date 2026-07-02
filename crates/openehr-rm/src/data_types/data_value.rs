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
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    /// `DV_BOOLEAN` (package `basic`).
    Boolean(DvBoolean),
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
    /// `DV_TEXT` (package `text`), the "bare" (uncoded) form.
    ///
    /// PORT NOTE: [`DvText`] is itself the ADR-001-Refinements-style enum
    /// spanning `DV_TEXT`/`DV_CODED_TEXT` (see `text::dv_text` for the full
    /// rationale) — nested here as the outer `DataValue::Text` variant so a
    /// `DATA_VALUE`-typed field can hold either form without this outer
    /// enum needing its own separate `Coded` variant duplicating that
    /// distinction. `DataValue::Text(DvText::Coded(_))` and
    /// `DataValue::CodedText(_)` below are therefore two different paths to
    /// a coded value; see the `CodedText` variant's own note for why both
    /// exist.
    Text(DvText),
    /// `DV_CODED_TEXT` (package `text`).
    ///
    /// PORT NOTE: the task's variant list names `CodedText(DvCodedText)`
    /// explicitly, alongside `Text(DvText)` — both are included verbatim
    /// as instructed, even though `DvText` (above) already has a `Coded`
    /// arm that can hold a `DvCodedText`. This creates two structurally
    /// different ways to place a coded-text value into a `DataValue`
    /// (`DataValue::Text(DvText::Coded(x))` vs `DataValue::CodedText(x)`);
    /// left as specified rather than silently collapsing one path, and
    /// flagged here as a design question for the P4/P17 wiring pass to
    /// resolve (most likely by dropping the inner `DvText::Coded` arm's
    /// reachability from `DataValue` in favour of this direct variant, or
    /// vice versa).
    CodedText(DvCodedText),
    /// `DV_PARAGRAPH` (package `text`, deprecated but legal).
    Paragraph(DvParagraph),
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
    /// `DV_DATE` (package `date_time`; forward-reference, sibling
    /// worktree).
    Date(DvDate),
    /// `DV_TIME` (package `date_time`; forward-reference, sibling
    /// worktree).
    Time(DvTime),
    /// `DV_DATE_TIME` (package `date_time`; forward-reference, sibling
    /// worktree).
    DateTime(DvDateTime),
    /// `DV_DURATION` (package `date_time`; forward-reference, sibling
    /// worktree).
    Duration(DvDuration),
    /// `DV_PERIODIC_TIME_SPECIFICATION` (package `time_specification`;
    /// forward-reference, sibling worktree).
    PeriodicTimeSpecification(DvPeriodicTimeSpecification),
    /// `DV_GENERAL_TIME_SPECIFICATION` (package `time_specification`;
    /// forward-reference, sibling worktree).
    GeneralTimeSpecification(DvGeneralTimeSpecification),
    /// `DV_MULTIMEDIA` (package `encapsulated`; forward-reference, sibling
    /// worktree).
    Multimedia(DvMultimedia),
    /// `DV_PARSABLE` (package `encapsulated`; forward-reference, sibling
    /// worktree).
    Parsable(DvParsable),
    /// `DV_URI` (package `uri`).
    Uri(DvUri),
    /// `DV_EHR_URI` (package `uri`).
    EhrUri(DvEhrUri),
}

impl DataValueApi for DataValue {
    fn type_name(&self) -> &'static str {
        match self {
            DataValue::Boolean(v) => v.type_name(),
            DataValue::State(v) => v.type_name(),
            DataValue::Identifier(v) => v.type_name(),
            DataValue::Text(v) => v.type_name(),
            DataValue::CodedText(v) => v.type_name(),
            DataValue::Paragraph(v) => v.type_name(),
            // TODO(port): sibling `quantity`/`date_time`/`time_specification`/
            // `encapsulated` types do not yet exist in this worktree
            // (concurrent transcription in separate worktrees); their
            // `DataValueApi` impls cannot be called until those land and
            // this file is wired at P17. Left as `todo!()` per-arm rather
            // than omitting the arms, so the match stays exhaustive against
            // the enum defined above.
            DataValue::Ordinal(_) => todo!("DvOrdinal::type_name pending sibling transcription"),
            DataValue::Scale(_) => todo!("DvScale::type_name pending sibling transcription"),
            DataValue::Quantity(_) => todo!("DvQuantity::type_name pending sibling transcription"),
            DataValue::Count(_) => todo!("DvCount::type_name pending sibling transcription"),
            DataValue::Proportion(_) => {
                todo!("DvProportion::type_name pending sibling transcription")
            }
            DataValue::Interval(_) => todo!("DvInterval::type_name pending sibling transcription"),
            DataValue::Date(_) => todo!("DvDate::type_name pending sibling transcription"),
            DataValue::Time(_) => todo!("DvTime::type_name pending sibling transcription"),
            DataValue::DateTime(_) => todo!("DvDateTime::type_name pending sibling transcription"),
            DataValue::Duration(_) => todo!("DvDuration::type_name pending sibling transcription"),
            DataValue::PeriodicTimeSpecification(_) => {
                todo!("DvPeriodicTimeSpecification::type_name pending sibling transcription")
            }
            DataValue::GeneralTimeSpecification(_) => {
                todo!("DvGeneralTimeSpecification::type_name pending sibling transcription")
            }
            DataValue::Multimedia(_) => {
                todo!("DvMultimedia::type_name pending sibling transcription")
            }
            DataValue::Parsable(_) => todo!("DvParsable::type_name pending sibling transcription"),
            DataValue::Uri(v) => v.type_name(),
            DataValue::EhrUri(v) => v.type_name(),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 data_types (root) — docs/research/spec-cache/RM-1.1.0/uml_classes/data_value.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master03-introduction.adoc §Overview / data_value.adoc §DATA_VALUE Class
//   confidence: medium
//   todos: 1
//   note: DATA_VALUE has no attributes of its own (only an OPENEHR_DEFINITIONS constants-inherit), so it gets a lean DataValueApi marker trait per ADR-001 §1, not a Data+enum+Api triple; the DataValue enum's 14 non-basic/text/uri variants forward-reference sibling-worktree modules (quantity, date_time, time_specification, encapsulated) that do not exist in this worktree yet — each is a distinct todo!() arm (14 todo!() calls, a separate count from the one literal TODO(port) marker above them) in the DataValueApi impl, kept to preserve match exhaustiveness. DV_STATE/DV_IDENTIFIER/DV_PARAGRAPH/DV_URI/DV_EHR_URI included as variants despite not being named in the task's explicit list (flagged as likely oversight, not exclusion); Text(DvText) vs CodedText(DvCodedText) overlap (both can represent coded text) flagged as a design question for P4/P17. serde `_type` dispatch is out of scope until P4 per the class-level doc comment.
// ─────────────────────────────────────────────
