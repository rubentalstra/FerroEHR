//! RM-level validation glue (ADR-003 spec behaviour; hand-written, preserved
//! across `openehr-codegen` regeneration — it is not a `// @generated` file, so
//! the generator's `declare_hand_written_modules` keeps it and `lib.rs`
//! auto-declares `pub mod validate;`).
//!
//! Two things live here:
//!
//! 1. **The `_type`→[`Validate`] dispatcher** ([`validate_rm_value`]) the
//!    composition validator (P15) calls on a canonical-JSON node: it reads the
//!    node's `_type`, deserializes into the matching concrete `openehr-rm` /
//!    `openehr-base` type, and runs that type's RM **class invariants**.
//! 2. **Shared invariant helpers** used by the sibling `*_impl.rs` behaviour
//!    files (the DV_AMOUNT / DV_QUANTIFIED accuracy + magnitude-status rules,
//!    the LOCATABLE `Archetype_node_id_valid` rule, ISO-8601 value checks).
//!
//! # Fidelity to the reference implementation (archie)
//!
//! The RM class invariants mirror openEHR's reference implementation
//! **archie** (`com.nedap.archie.rmobjectvalidator`). Archie runs each
//! `@Invariant`-annotated boolean method and, on failure, emits one uniform
//! message: `Invariant <Name> failed on type <RM_TYPE>`. We reproduce that
//! message verbatim (see [`invariant_failed`]) so a violation is identifiable
//! by archie's own invariant name.
//!
//! What we deliberately do **not** implement here (`// PORT NOTE:`):
//! - **Terminology-bound invariants** (archie's `Language_valid`,
//!   `Encoding_valid`, `Category_validity`, `Setting_valid`, `Change_type_valid`,
//!   `Normal_status_validity`, `Media_type_valid`, `Current_state_valid`, …).
//!   `openehr-rm` has no `openehr-term` dependency; these belong to the
//!   composition validator + terminology binding (P15 PR-C), which resolves
//!   codes against the openEHR terminology bundle.
//! - **archie's `ignored = true` invariants** (never executed by archie —
//!   implementing them would over-reject relative to the reference).
//! - **Cross-child recursion**: each `Validate` impl checks only its own class
//!   invariants; the composition validator recurses into children (and prefixes
//!   the absolute RM path onto each [`InvariantViolation`]).

use serde::de::DeserializeOwned;
use serde_json::Value;

pub use openehr_base::validate::{InvariantViolation, Validate};

/// Build an archie-style class-invariant violation:
/// `"Invariant <name> failed on type <RM_TYPE>"` — the exact message the
/// reference implementation's `RMObjectValidator` emits for every invariant
/// failure. The path is left empty (the value itself); the composition
/// validator prefixes the absolute RM path.
#[must_use]
pub(crate) fn invariant_failed(name: &str, rm_type: &str) -> InvariantViolation {
    InvariantViolation::here(format!("Invariant {name} failed on type {rm_type}"))
}

/// `true` when a floating value denotes a whole number (archie `isInteger`).
#[must_use]
#[allow(clippy::float_cmp)] // exact-integrality test, mirrors archie's `x.floor() == x`
pub(crate) fn is_integral(v: f64) -> bool {
    v.is_finite() && v.floor() == v
}

/// DV_QUANTIFIED `Magnitude_status_valid`: if present, `magnitude_status` must
/// be one of `= < > <= >= ~` (archie `DvQuantified.VALID_MAGNITUDE_STATUS_CODES`).
pub(crate) fn push_magnitude_status_valid(
    out: &mut Vec<InvariantViolation>,
    rm_type: &str,
    magnitude_status: Option<&str>,
) {
    if let Some(s) = magnitude_status
        && !matches!(s, "=" | "<" | ">" | "<=" | ">=" | "~")
    {
        out.push(invariant_failed("Magnitude_status_valid", rm_type));
    }
}

/// The DV_AMOUNT invariants (`Accuracy_is_percent_validity`, `Accuracy_valid`)
/// plus the inherited DV_QUANTIFIED `Magnitude_status_valid`. Shared by every
/// concrete DV_AMOUNT descendant (DV_QUANTITY, DV_COUNT, DV_DURATION,
/// DV_PROPORTION) — mirrors archie `DvAmount` / `DvQuantified`.
#[allow(clippy::float_cmp)] // exact accuracy == 0 test, mirrors archie's `accuracy == 0.0`
pub(crate) fn push_dv_amount_invariants(
    out: &mut Vec<InvariantViolation>,
    rm_type: &str,
    accuracy: Option<f64>,
    accuracy_is_percent: Option<bool>,
    magnitude_status: Option<&str>,
) {
    // Accuracy_is_percent_validity: accuracy = 0 implies not recorded as percent.
    if accuracy == Some(0.0) && accuracy_is_percent == Some(true) {
        out.push(invariant_failed("Accuracy_is_percent_validity", rm_type));
    }
    // Accuracy_valid: recorded as percent implies 0 <= accuracy <= 100.
    if accuracy_is_percent == Some(true)
        && let Some(a) = accuracy
        && !(0.0..=100.0).contains(&a)
    {
        out.push(invariant_failed("Accuracy_valid", rm_type));
    }
    push_magnitude_status_valid(out, rm_type, magnitude_status);
}

/// LOCATABLE `Archetype_node_id_valid`: `archetype_node_id` must be non-empty
/// (archie `Locatable`, `nullOrNotEmpty`). Applied by every concrete LOCATABLE
/// impl.
pub(crate) fn push_archetype_node_id_valid(
    out: &mut Vec<InvariantViolation>,
    rm_type: &str,
    archetype_node_id: &str,
) {
    if archetype_node_id.is_empty() {
        out.push(invariant_failed("Archetype_node_id_valid", rm_type));
    }
}

// ── ISO-8601 value validation ────────────────────────────────────────────────
//
// PORT NOTE: archie has no `@Invariant` for DV_DATE/DV_TIME/DV_DATE_TIME/
// DV_DURATION value well-formedness — it enforces it structurally by parsing
// `value` into a typed `java.time` object at construction. In our model the
// value is a `String`, so we express the same guarantee as an explicit RM class
// invariant (`Value_valid`). The forms accepted are the openEHR ISO-8601 subset
// (partial precision permitted; DV_DURATION permits a leading sign and a `W`
// designator mixed with others, per the openEHR deviation). Kept intentionally
// lenient: it rejects clearly-malformed values, not valid partial ones.

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn digits_n(s: &str, n: usize) -> bool {
    s.len() == n && all_digits(s)
}

fn in_range(s: &str, lo: u32, hi: u32) -> bool {
    s.len() == 2 && all_digits(s) && s.parse::<u32>().is_ok_and(|v| (lo..=hi).contains(&v))
}

/// A valid openEHR ISO-8601 date: `YYYY`, `YYYY-MM`, `YYYY-MM-DD`, or the
/// compact `YYYYMM` / `YYYYMMDD` forms.
#[must_use]
pub(crate) fn is_valid_iso_date(s: &str) -> bool {
    if s.contains('-') {
        match s.split('-').collect::<Vec<_>>().as_slice() {
            [y] => digits_n(y, 4),
            [y, m] => digits_n(y, 4) && in_range(m, 1, 12),
            [y, m, d] => digits_n(y, 4) && in_range(m, 1, 12) && in_range(d, 1, 31),
            _ => false,
        }
    } else {
        match s.len() {
            4 => all_digits(s),
            6 => all_digits(s) && in_range(&s[4..6], 1, 12),
            8 => all_digits(s) && in_range(&s[4..6], 1, 12) && in_range(&s[6..8], 1, 31),
            _ => false,
        }
    }
}

fn is_valid_tz(tz: &str) -> bool {
    if tz.is_empty() || tz == "Z" {
        return true;
    }
    let Some(rest) = tz.strip_prefix(['+', '-']) else {
        return false;
    };
    if rest.contains(':') {
        matches!(rest.split(':').collect::<Vec<_>>().as_slice(),
            [h, m] if in_range(h, 0, 14) && in_range(m, 0, 59))
    } else {
        match rest.len() {
            2 => in_range(rest, 0, 14),
            4 => in_range(&rest[0..2], 0, 14) && in_range(&rest[2..4], 0, 59),
            _ => false,
        }
    }
}

fn is_valid_time_core(s: &str) -> bool {
    // optional fractional seconds after '.' or ','
    let (base, frac) = match s.split_once(['.', ',']) {
        Some((b, f)) => (b, Some(f)),
        None => (s, None),
    };
    if let Some(f) = frac
        && !all_digits(f)
    {
        return false;
    }
    if base.contains(':') {
        match base.split(':').collect::<Vec<_>>().as_slice() {
            [h] => in_range(h, 0, 23),
            [h, m] => in_range(h, 0, 23) && in_range(m, 0, 59),
            [h, m, sec] => in_range(h, 0, 23) && in_range(m, 0, 59) && in_range(sec, 0, 60),
            _ => false,
        }
    } else {
        match base.len() {
            2 => in_range(base, 0, 23),
            4 => in_range(&base[0..2], 0, 23) && in_range(&base[2..4], 0, 59),
            6 => {
                in_range(&base[0..2], 0, 23)
                    && in_range(&base[2..4], 0, 59)
                    && in_range(&base[4..6], 0, 60)
            }
            _ => false,
        }
    }
}

/// A valid openEHR ISO-8601 time: `HH`, `HH:MM`, `HH:MM:SS[.fff]` (and the
/// compact `HHMM` / `HHMMSS` forms), with an optional `Z` / `±HH[:MM]` timezone.
#[must_use]
pub(crate) fn is_valid_iso_time(s: &str) -> bool {
    // Split off a trailing timezone (`Z`, or a `+`/`-` offset that is not the
    // fractional separator). Scan from the end for `Z`/`+`/`-`.
    if let Some(stripped) = s.strip_suffix('Z') {
        return is_valid_time_core(stripped);
    }
    if let Some(pos) = s.rfind(['+', '-']) {
        return is_valid_time_core(&s[..pos]) && is_valid_tz(&s[pos..]);
    }
    is_valid_time_core(s)
}

/// A valid openEHR ISO-8601 date-time: a date, then (if a time component is
/// present) `T` and a time. A `T`-less value is accepted as a date-only partial.
#[must_use]
pub(crate) fn is_valid_iso_date_time(s: &str) -> bool {
    match s.split_once('T') {
        Some((date, time)) => is_valid_iso_date(date) && is_valid_iso_time(time),
        None => is_valid_iso_date(s),
    }
}

fn parse_duration_components(s: &str, allowed: &[u8], any: &mut bool) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b',') {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i == start || i >= bytes.len() {
            return false; // no number, or number without a designator
        }
        if !allowed.contains(&bytes[i]) {
            return false;
        }
        i += 1;
        *any = true;
    }
    true
}

/// A valid openEHR ISO-8601 duration: optional leading sign, `P`, then one or
/// more `nY nM nW nD` components and an optional `T nH nM nS` part (openEHR
/// permits the sign and a `W` designator mixed with the others).
#[must_use]
pub(crate) fn is_valid_iso_duration(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let Some(rest) = s.strip_prefix('P') else {
        return false;
    };
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (rest, None),
    };
    let mut any = false;
    if !parse_duration_components(date_part, b"YMWD", &mut any) {
        return false;
    }
    if let Some(t) = time_part
        && (t.is_empty() || !parse_duration_components(t, b"HMS", &mut any))
    {
        return false;
    }
    any
}

// ── the _type → Validate dispatcher ──────────────────────────────────────────

fn run<T: DeserializeOwned + Validate>(value: &Value, out: &mut Vec<InvariantViolation>) {
    // A node that fails to deserialize into its declared concrete type is a
    // structural error caught by the codec/schema layer, not an invariant
    // failure — so we simply run no invariants for it here.
    if let Ok(v) = serde_json::from_value::<T>(value.clone()) {
        v.validate_invariants(out);
    }
}

/// Run the RM class invariants for a single canonical-JSON node, dispatching on
/// its `_type`. A node with no (or an unrecognised) `_type` runs no invariants
/// (returns without appending). The composition validator (P15) calls this per
/// node and prefixes the absolute RM path onto each [`InvariantViolation`].
///
/// Coverage is the set of concrete `openehr-rm` / `openehr-base` types that
/// carry a non-terminology class invariant (the ones with a `*_impl.rs`
/// sibling). `DV_INTERVAL` is dispatched with a `DvOrdered` element type so
/// the `Limits_consistent` ordering invariant is reached (F-12-04/10), falling
/// back to `serde_json::Value` (own boundary-flag invariants only) when the
/// limits do not deserialize as typed `DV_ORDERED` values. The other generic
/// containers (`HISTORY`, `POINT_EVENT`, `INTERVAL_EVENT`) are checked with
/// `serde_json::Value` as the element type — enough for their own (non-child)
/// invariants.
#[allow(clippy::too_many_lines)] // a flat _type → run::<T> dispatch table
pub fn validate_rm_value(value: &Value, out: &mut Vec<InvariantViolation>) {
    use openehr_base::base_types::identification::internet_id::InternetId;
    use openehr_base::base_types::identification::iso_oid::IsoOid;
    use openehr_base::base_types::identification::object_ref::ObjectRefData;
    use openehr_base::base_types::identification::object_version_id::ObjectVersionId;
    use openehr_base::base_types::identification::party_ref::PartyRef;
    use openehr_base::base_types::identification::version_tree_id::VersionTreeId;

    use crate::common::archetyped::archetyped::Archetyped;
    use crate::common::archetyped::feeder_audit_details::FeederAuditDetails;
    use crate::common::directory::folder::Folder;
    use crate::common::generic::attestation::Attestation;
    use crate::common::generic::audit_details::AuditDetailsData;
    use crate::common::generic::party_identified::PartyIdentifiedData;
    use crate::common::generic::party_related::PartyRelated;
    use crate::common::tags::item_tag::ItemTag;
    use crate::composition::composition::Composition;
    use crate::composition::content::entry::action::Action;
    use crate::composition::content::entry::activity::Activity;
    use crate::composition::content::entry::admin_entry::AdminEntry;
    use crate::composition::content::entry::evaluation::Evaluation;
    use crate::composition::content::entry::instruction::Instruction;
    use crate::composition::content::entry::instruction_details::InstructionDetails;
    use crate::composition::content::entry::observation::Observation;
    use crate::composition::content::navigation::section::Section;
    use crate::composition::event_context::EventContext;
    use crate::data_structures::history::history::History;
    use crate::data_structures::history::interval_event::IntervalEvent;
    use crate::data_structures::history::point_event::PointEvent;
    use crate::data_structures::item_structure::item_table::ItemTable;
    use crate::data_structures::representation::cluster::Cluster;
    use crate::data_structures::representation::element::Element;
    use crate::data_types::basic::dv_identifier::DvIdentifier;
    use crate::data_types::encapsulated::dv_multimedia::DvMultimedia;
    use crate::data_types::encapsulated::dv_parsable::DvParsable;
    use crate::data_types::quantity::date_time::dv_date::DvDate;
    use crate::data_types::quantity::date_time::dv_date_time::DvDateTime;
    use crate::data_types::quantity::date_time::dv_duration::DvDuration;
    use crate::data_types::quantity::date_time::dv_time::DvTime;
    use crate::data_types::quantity::dv_count::DvCount;
    use crate::data_types::quantity::dv_interval::DvInterval;
    use crate::data_types::quantity::dv_ordered::DvOrdered;
    use crate::data_types::quantity::dv_ordinal::DvOrdinal;
    use crate::data_types::quantity::dv_proportion::DvProportion;
    use crate::data_types::quantity::dv_quantity::DvQuantity;
    use crate::data_types::quantity::dv_scale::DvScale;
    use crate::data_types::quantity::reference_range::ReferenceRange;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::term_mapping::TermMapping;
    use crate::data_types::uri::dv_ehr_uri::DvEhrUri;
    use crate::data_types::uri::dv_uri::DvUriData;

    let Some(ty) = value.get("_type").and_then(Value::as_str) else {
        return;
    };
    match ty {
        // data_types
        "CODE_PHRASE" => run::<CodePhrase>(value, out),
        "DV_URI" => run::<DvUriData>(value, out),
        "DV_EHR_URI" => run::<DvEhrUri>(value, out),
        "DV_IDENTIFIER" => run::<DvIdentifier>(value, out),
        "TERM_MAPPING" => run::<TermMapping>(value, out),
        "DV_MULTIMEDIA" => run::<DvMultimedia>(value, out),
        "DV_PROPORTION" => run::<DvProportion>(value, out),
        "DV_QUANTITY" => run::<DvQuantity>(value, out),
        "DV_COUNT" => run::<DvCount>(value, out),
        "DV_DURATION" => run::<DvDuration>(value, out),
        "DV_DATE" => run::<DvDate>(value, out),
        "DV_TIME" => run::<DvTime>(value, out),
        "DV_DATE_TIME" => run::<DvDateTime>(value, out),
        "DV_ORDINAL" => run::<DvOrdinal>(value, out),
        "DV_SCALE" => run::<DvScale>(value, out),
        "DV_PARSABLE" => run::<DvParsable>(value, out),
        "REFERENCE_RANGE" => run::<ReferenceRange>(value, out),
        // DV_INTERVAL: prefer the DV_ORDERED-typed element so the
        // Limits_consistent ordering invariant runs (F-12-04); fall back to
        // Value elements (boundary flags only) for non-DV_ORDERED payloads.
        "DV_INTERVAL" => {
            if let Ok(v) = serde_json::from_value::<DvInterval<DvOrdered>>(value.clone()) {
                v.validate_invariants(out);
            } else {
                run::<DvInterval<Value>>(value, out);
            }
        }
        // data_structures
        "ELEMENT" => run::<Element>(value, out),
        "CLUSTER" => run::<Cluster>(value, out),
        "HISTORY" => run::<History<Value>>(value, out),
        "POINT_EVENT" => run::<PointEvent<Value>>(value, out),
        "INTERVAL_EVENT" => run::<IntervalEvent<Value>>(value, out),
        "ITEM_TABLE" => run::<ItemTable>(value, out),
        // common
        "PARTY_IDENTIFIED" => run::<PartyIdentifiedData>(value, out),
        "PARTY_RELATED" => run::<PartyRelated>(value, out),
        "AUDIT_DETAILS" => run::<AuditDetailsData>(value, out),
        "ATTESTATION" => run::<Attestation>(value, out),
        "FEEDER_AUDIT_DETAILS" => run::<FeederAuditDetails>(value, out),
        "ARCHETYPED" => run::<Archetyped>(value, out),
        // ehr / composition
        "COMPOSITION" => run::<Composition>(value, out),
        "EVENT_CONTEXT" => run::<EventContext>(value, out),
        "ACTIVITY" => run::<Activity>(value, out),
        "INSTRUCTION_DETAILS" => run::<InstructionDetails>(value, out),
        "OBSERVATION" => run::<Observation>(value, out),
        "INSTRUCTION" => run::<Instruction>(value, out),
        "ACTION" => run::<Action>(value, out),
        "EVALUATION" => run::<Evaluation>(value, out),
        "ADMIN_ENTRY" => run::<AdminEntry>(value, out),
        "SECTION" => run::<Section>(value, out),
        "FOLDER" => run::<Folder>(value, out),
        "ITEM_TAG" => run::<ItemTag>(value, out),
        // base identification
        "OBJECT_REF" => run::<ObjectRefData>(value, out),
        "PARTY_REF" => run::<PartyRef>(value, out),
        "VERSION_TREE_ID" => run::<VersionTreeId>(value, out),
        "OBJECT_VERSION_ID" => run::<ObjectVersionId>(value, out),
        "ISO_OID" => run::<IsoOid>(value, out),
        "INTERNET_ID" => run::<InternetId>(value, out),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn iso_date_forms() {
        assert!(is_valid_iso_date("2021"));
        assert!(is_valid_iso_date("2021-05"));
        assert!(is_valid_iso_date("2021-05-17"));
        assert!(is_valid_iso_date("20210517"));
        assert!(!is_valid_iso_date("2021-13"));
        assert!(!is_valid_iso_date("2021-05-32"));
        assert!(!is_valid_iso_date("not-a-date"));
        assert!(!is_valid_iso_date(""));
    }

    #[test]
    fn iso_time_forms() {
        assert!(is_valid_iso_time("10"));
        assert!(is_valid_iso_time("10:30"));
        assert!(is_valid_iso_time("10:30:59"));
        assert!(is_valid_iso_time("10:30:59.250"));
        assert!(is_valid_iso_time("10:30:59Z"));
        assert!(is_valid_iso_time("10:30:59+01:00"));
        assert!(!is_valid_iso_time("25:00"));
        assert!(!is_valid_iso_time("10:61"));
        assert!(!is_valid_iso_time("abc"));
    }

    #[test]
    fn iso_date_time_forms() {
        assert!(is_valid_iso_date_time("2021-05-17T10:30:00"));
        assert!(is_valid_iso_date_time("2021-05-17T10:30:00+02:00"));
        assert!(is_valid_iso_date_time("2021-05-17"));
        assert!(!is_valid_iso_date_time("2021-05-17T99:00"));
        assert!(!is_valid_iso_date_time("nope"));
    }

    #[test]
    fn iso_duration_forms() {
        assert!(is_valid_iso_duration("P1Y"));
        assert!(is_valid_iso_duration("P1Y2M10D"));
        assert!(is_valid_iso_duration("PT2H30M"));
        assert!(is_valid_iso_duration("P1Y2M10DT2H30M"));
        assert!(is_valid_iso_duration("P2W"));
        assert!(is_valid_iso_duration("-P1D"));
        assert!(is_valid_iso_duration("PT0.5S"));
        assert!(!is_valid_iso_duration("P"));
        assert!(!is_valid_iso_duration("1Y"));
        assert!(!is_valid_iso_duration("P1X"));
        assert!(!is_valid_iso_duration("PT"));
    }

    #[test]
    fn dispatch_unknown_or_untyped_is_noop() {
        let mut out = Vec::new();
        validate_rm_value(&json!({"value": "x"}), &mut out);
        validate_rm_value(&json!({"_type": "NOT_A_REAL_TYPE"}), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn dispatch_code_phrase_invalid() {
        // CODE_PHRASE with an empty code_string violates Code_string_valid.
        let node = json!({
            "_type": "CODE_PHRASE",
            "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
            "code_string": ""
        });
        let mut out = Vec::new();
        validate_rm_value(&node, &mut out);
        assert!(
            out.iter()
                .any(|v| v.message == "Invariant Code_string_valid failed on type CODE_PHRASE"),
            "got {out:?}"
        );
    }

    #[test]
    fn dispatch_dv_proportion_valid_and_invalid() {
        let valid = json!({
            "_type": "DV_PROPORTION", "numerator": 1.0, "denominator": 100.0, "type": 2
        });
        let mut out = Vec::new();
        validate_rm_value(&valid, &mut out);
        assert!(
            out.is_empty(),
            "expected valid percent proportion, got {out:?}"
        );

        // percent kind (2) requires denominator == 100.
        let invalid = json!({
            "_type": "DV_PROPORTION", "numerator": 1.0, "denominator": 3.0, "type": 2
        });
        let mut out = Vec::new();
        validate_rm_value(&invalid, &mut out);
        assert!(
            out.iter()
                .any(|v| v.message == "Invariant Percent_validity failed on type DV_PROPORTION"),
            "got {out:?}"
        );
    }
}
