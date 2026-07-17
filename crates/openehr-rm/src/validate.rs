//! RM-level validation glue (hand-written spec behaviour; preserved
//! across `openehr-codegen` regeneration — the generator does not emit or overwrite it, so
//! the generator's `declare_hand_written_modules` keeps it and `lib.rs`
//! auto-declares `pub mod validate;`).
//!
//! Two things live here:
//!
//! 1. **The `_type`→[`Validate`] dispatcher** ([`validate_rm_value`]) the
//! composition validator calls on a canonical-JSON node: it reads the
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
//! What we deliberately do **not** implement here (`// NOTE:`):
//! - **Terminology-bound invariants** (archie's `Language_valid`,
//!   `Encoding_valid`, `Category_validity`, `Setting_valid`, `Change_type_valid`,
//!   `Normal_status_validity`, `Media_type_valid`, `Current_state_valid`, …).
//!   `openehr-rm` has no `openehr-term` dependency; these belong to the
//! composition validator + terminology binding (P15 PR-C), which resolves
//!   codes against the openEHR terminology bundle.
//! - **archie's `ignored = true` invariants** (never executed by archie —
//!   implementing them would over-reject relative to the reference).
//! - **Cross-child recursion**: each `Validate` impl checks only its own class
//!   invariants; the composition validator recurses into children (and prefixes
//!   the absolute RM path onto each [`InvariantViolation`]).

use serde::de::DeserializeOwned;
use serde_json::Value;

pub use openehr_base::validate::{InvariantViolation, Validate};

mod fast;

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

/// The shared ENTRY-root invariants (archie `Is_archetypeRoot` on every
/// concrete ENTRY subtype + inherited LOCATABLE `Archetype_node_id_valid`):
/// an ENTRY is an archetype root, so `archetype_details` must be present.
/// One core for the typed `Validate` impls and the value-level fast path.
pub(crate) fn push_entry_root_invariants(
    out: &mut Vec<InvariantViolation>,
    rm_type: &str,
    has_archetype_details: bool,
    archetype_node_id: &str,
) {
    if !has_archetype_details {
        out.push(invariant_failed("Is_archetypeRoot", rm_type));
    }
    push_archetype_node_id_valid(out, rm_type, archetype_node_id);
}

/// The temporal `Value_valid` invariant (see the ISO-8601 module notes above:
/// archie enforces well-formedness structurally; our string-valued model
/// expresses it as an explicit class invariant). One core for the typed
/// impls and the value-level fast path.
pub(crate) fn push_temporal_value_valid(
    out: &mut Vec<InvariantViolation>,
    rm_type: &str,
    valid: bool,
) {
    if !valid {
        out.push(invariant_failed("Value_valid", rm_type));
    }
}

// ── ISO-8601 value validation ────────────────────────────────────────────────
//
// NOTE: archie has no `@Invariant` for DV_DATE/DV_TIME/DV_DATE_TIME/
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

/// `true` for a Gregorian leap year: divisible by 4, except centuries not
/// divisible by 400 (BASE `Time_definitions`; the calendar `days_in_month`
/// depends on it).
fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Calendar days in a given month of a given year — the `days_in_month (m, y)`
/// the BASE `Time_definitions.valid_day` postcondition dispatches through
/// (`docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.time_definitions.adoc`
/// lines 95–103). Returns `0` for a month outside `1..=12` (caller has already
/// range-checked the month, so that branch is defensive).
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A calendar-valid two-digit day for the 4-digit `y` / 2-digit `m` strings:
/// `d` is `01`..`days_in_month(m, y)` — the BASE `Iso8601_date` invariant
/// `Day_valid: not day_unknown implies valid_day (year, month, day)` with
/// `valid_day (y, m, d) = (d >= 1 and d <= days_in_month (m, y))`
/// (`org.openehr.base.foundation_types.iso8601_date.adoc` line 107;
/// `time_definitions.adoc` line 102). This is calendar-exact — it rejects
/// `2021-02-31`, `2021-04-31`, and `2021-02-29` (non-leap) while accepting
/// `2020-02-29` (leap).
fn valid_day(y: &str, m: &str, d: &str) -> bool {
    if d.len() != 2 || !all_digits(d) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (y.parse::<u32>(), m.parse::<u32>(), d.parse::<u32>())
    else {
        return false;
    };
    (1..=days_in_month(year, month)).contains(&day)
}

/// A valid openEHR ISO-8601 date: `YYYY`, `YYYY-MM`, `YYYY-MM-DD`, or the
/// compact `YYYYMM` / `YYYYMMDD` forms. Day validity is **calendar-exact**
/// (month lengths + leap years) per BASE `Iso8601_date.Day_valid`, not a bare
/// `1..=31` range.
#[must_use]
pub(crate) fn is_valid_iso_date(s: &str) -> bool {
    if s.contains('-') {
        match s.split('-').collect::<Vec<_>>().as_slice() {
            [y] => digits_n(y, 4),
            [y, m] => digits_n(y, 4) && in_range(m, 1, 12),
            [y, m, d] => digits_n(y, 4) && in_range(m, 1, 12) && valid_day(y, m, d),
            _ => false,
        }
    } else {
        match s.len() {
            4 => all_digits(s),
            6 => all_digits(s) && in_range(&s[4..6], 1, 12),
            8 => {
                all_digits(s)
                    && in_range(&s[4..6], 1, 12)
                    && valid_day(&s[0..4], &s[4..6], &s[6..8])
            }
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
    // BASE `Iso8601_timezone` bounds are ASYMMETRIC (`iso8601_timezone.adoc`
    // Max_hour_valid / Min_hour_valid; `time_definitions.adoc`
    // Max_timezone_hour = 14, Min_timezone_hour = 12): `+` offsets go to
    // +14:00, `-` offsets only to -12:00 (reject `-13:00`).
    //
    // NOTE (corpus adjudication): the invariants literally require
    // `hour > 0` when signed, but the canonical corpus + CNF data sets carry
    // `+00:00`/`-00:00` UTC forms in 42 files — the corpus outranks the prose
    // reading, so hour 0 is accepted with either sign (≡ `Z`).
    let max_hour = if tz.starts_with('+') { 14 } else { 12 };
    if rest.contains(':') {
        matches!(rest.split(':').collect::<Vec<_>>().as_slice(),
            [h, m] if in_range(h, 0, max_hour) && in_range(m, 0, 59))
    } else {
        match rest.len() {
            2 => in_range(rest, 0, max_hour),
            4 => in_range(&rest[0..2], 0, max_hour) && in_range(&rest[2..4], 0, 59),
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

/// Record a typed-deserialize failure as a validation violation.
///
/// A node that does not deserialize into its declared concrete RM type is NOT
/// "caught by the codec/schema layer" on the commit path: the node codec stores
/// the raw canonical-JSON fragment verbatim (no openEHR spec governs the storage
/// mechanics — our own storage design) and the ITS-JSON schema is not enforced
/// at commit, so a missing mandatory attribute (e.g. `COMPOSITION.composer [1]`)
/// or a wrong nested type (e.g. an `EHR_STATUS.subject` that is not `PARTY_SELF`)
/// reaches here and nowhere else. Per ITS-REST `422_COMPOSITION.yaml` ("converts,
/// but does not validate") this is a validation failure — surface it. (The valid
/// corpus deserializes cleanly at every node, so this never rejects a valid
/// input; if it ever did, that would expose a codegen field-optionality bug to
/// fix in the emitter — see docs/plans/s2-phase-04-cnf-hardening.md.)
fn record_type_mismatch(value: &Value, err: &serde_json::Error, out: &mut Vec<InvariantViolation>) {
    let ty = value
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");
    out.push(InvariantViolation::here(format!(
        "does not conform to RM type {ty}: {err}"
    )));
}

fn run<T: DeserializeOwned + Validate>(value: &Value, out: &mut Vec<InvariantViolation>) {
    match T::deserialize(value) {
        Ok(v) => v.validate_invariants(out),
        Err(e) => record_type_mismatch(value, &e, out),
    }
}

/// Like [`run`], but deserialize `T` from a copy of `value` whose nested
/// RM-node child collections have been emptied ([`prune_child_nodes`]).
///
/// TODO(perf): the RM-invariant pass ([`validate_rm_value`]) is called once per
/// `_type` node while the composition validator recurses the live JSON tree, so
/// deserializing each node's *whole* subtree (as `T::deserialize` does for a
/// concrete container type) re-parses every descendant once per ancestor —
/// O(Σ subtree sizes) for overlapping subtrees (measured ~47 ms of pure CPU for
/// a populated International Patient Summary, `crates/openehr-flat/src/validation/
/// tests.rs::measure_ips_validation_walk_cost`). This shallow variant is used for
/// the LOCATABLE structural containers whose own class invariants inspect only
/// scalar / single-object attributes (never a child collection): with the child
/// arrays emptied, each node deserializes only its own immediate shape, so the
/// pass is O(total nodes) instead of O(Σ subtree sizes). The node's own
/// single-valued attributes are KEPT (only collections are emptied), so its
/// mandatory-attribute presence and single-object type conformance are still
/// enforced on deserialize — the missing-mandatory-attribute rejection
/// (`422_COMPOSITION`, e.g. a dropped `COMPOSITION.composer [1]`) and every class
/// invariant result are unchanged (the valid corpus + the openehr-flat
/// validation suite verify this). Types whose own invariants DO read a child
/// collection (`HISTORY.events`, `ITEM_TABLE.rows`) keep the full [`run`]
/// deserialize.
///
/// NOTE: emptying a child *collection* here means a malformation *inside*
/// an array element is no longer reported at this ancestor's path — it is
/// reported at that element's own recursion step instead (each collection member
/// is a separate `_type` node the composition validator visits and dispatches).
/// For array-element types the dispatcher does not cover (embedded non-LOCATABLE
/// helpers such as `LINK` / `PARTICIPATION`, which carry no class invariant), a
/// structural malformation that the full ancestor deserialize used to surface is
/// no longer surfaced. This narrows only the redundant ancestor-cascade reporting
/// on already-invalid input; the valid path and every test-pinned rejection are
/// byte-identical, and the ITS-JSON schema gate remains the exhaustive
/// structural oracle where one is required (this pass is the RM class-invariant
/// check, not a schema validator — `422_COMPOSITION.yaml`).
fn run_shallow<T: DeserializeOwned + Validate>(value: &Value, out: &mut Vec<InvariantViolation>) {
    match T::deserialize(&prune_child_nodes(value)) {
        Ok(v) => v.validate_invariants(out),
        Err(e) => record_type_mismatch(value, &e, out),
    }
}

/// A shallow copy of an RM node with every nested RM-node **collection** emptied,
/// recursing through single-valued nested nodes (which are kept, so the node's
/// own mandatory single attributes stay enforced on deserialize). Scalar arrays
/// (e.g. `DV_MULTIMEDIA.data` octets) are kept as-is. See [`run_shallow`] for why
/// this is sound for the structural container types.
fn prune_child_nodes(value: &Value) -> Value {
    let Value::Object(map) = value else {
        return value.clone();
    };
    let mut out = serde_json::Map::with_capacity(map.len());
    for (key, child) in map {
        let pruned = match child {
            // An array of RM nodes: the children are recursed into (and fully
            // validated) individually by the composition validator, so this
            // node's own invariants never need them — drop them.
            Value::Array(items) if items.iter().any(Value::is_object) => Value::Array(Vec::new()),
            // A single nested node: keep it (its presence is a structural
            // constraint this node's deserialize must still enforce), but recurse
            // to empty ITS child collections.
            Value::Object(_) => prune_child_nodes(child),
            // Scalars and scalar arrays: keep verbatim.
            other => other.clone(),
        };
        out.insert(key.clone(), pruned);
    }
    Value::Object(out)
}

/// Run the RM class invariants for a single canonical-JSON node, dispatching on
/// its `_type`. A node with no (or an unrecognised) `_type` runs no invariants
/// (returns without appending). The composition validator calls this per
/// node and prefixes the absolute RM path onto each [`InvariantViolation`].
///
/// Two tiers (PERF: the RM-invariant pass visits every `_type` node of a
/// commit, ~1.5k for a populated composition, so the per-node cost is
/// load-bearing — measured via `openehr-flat`'s
/// `measure_ips_validation_walk_cost` harness):
///
/// 1. the **fast path** ([`fast`]) verifies structural conformance directly
///    against the live JSON node using the generated static RM model and runs
///    the class invariants through the same `pub(crate)` cores the typed
///    impls call — no deserialization, no allocation, byte-identical output;
/// 2. anything the fast path cannot vouch for falls back to the authoritative
///    **typed dispatch** below ([`validate_rm_value_typed`]), which
///    deserializes into the concrete RM type (surfacing `does not conform to
///    RM type …` for a structural mismatch) and runs the typed invariants.
pub fn validate_rm_value(value: &Value, out: &mut Vec<InvariantViolation>) {
    let Some(ty) = value.get("_type").and_then(Value::as_str) else {
        return;
    };
    if fast::try_validate(ty, value, out) {
        return;
    }
    validate_rm_value_typed(ty, value, out);
}

/// The typed dispatch tier of [`validate_rm_value`]: deserialize the node into
/// its concrete RM type and run that type's `Validate` impl. Authoritative for
/// every node (the fast path may only *skip* it when its result is provably
/// identical); also the oracle the fast-path equivalence tests compare against.
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
pub(crate) fn validate_rm_value_typed(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    use openehr_base::base_types::identification::archetype_id::ArchetypeId;
    use openehr_base::base_types::identification::internet_id::InternetId;
    use openehr_base::base_types::identification::iso_oid::IsoOid;
    use openehr_base::base_types::identification::object_ref::ObjectRefData;
    use openehr_base::base_types::identification::object_version_id::ObjectVersionId;
    use openehr_base::base_types::identification::party_ref::PartyRef;
    use openehr_base::base_types::identification::terminology_id::TerminologyId;
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
    use crate::data_types::text::dv_text::DvText;
    use crate::data_types::text::term_mapping::TermMapping;
    use crate::data_types::time_specification::dv_periodic_time_specification::DvPeriodicTimeSpecification;
    use crate::data_types::uri::dv_ehr_uri::DvEhrUri;
    use crate::data_types::uri::dv_uri::DvUriData;
    use crate::integration::generic_entry::GenericEntry;

    match ty {
        // data_types
        "CODE_PHRASE" => run::<CodePhrase>(value, out),
        // DV_TEXT + DV_CODED_TEXT share the DvText enum (Valid_value /
        // Formatting_valid, dv_text.adoc; DV_CODED_TEXT adds the structural
        // defining_code 1..1 at deserialize).
        "DV_TEXT" | "DV_CODED_TEXT" => run::<DvText>(value, out),
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
        "DV_PERIODIC_TIME_SPECIFICATION" => run::<DvPeriodicTimeSpecification>(value, out),
        "REFERENCE_RANGE" => run::<ReferenceRange>(value, out),
        // DV_INTERVAL: prefer the DV_ORDERED-typed element so the
        // Limits_consistent ordering invariant runs; fall back to
        // Value elements (boundary flags only) for non-DV_ORDERED payloads.
        "DV_INTERVAL" => {
            if let Ok(v) = serde_json::from_value::<DvInterval<DvOrdered>>(value.clone()) {
                v.validate_invariants(out);
            } else {
                run::<DvInterval<Value>>(value, out);
            }
        }
        // data_structures. HISTORY and ITEM_TABLE keep the full deserialize —
        // their own invariants read a child collection (`events` / `rows`); the
        // rest are structural containers with scalar-only invariants, so they
        // deserialize shallowly (see `run_shallow`).
        "ELEMENT" => run::<Element>(value, out),
        "CLUSTER" => run_shallow::<Cluster>(value, out),
        "HISTORY" => run::<History<Value>>(value, out),
        "POINT_EVENT" => run_shallow::<PointEvent<Value>>(value, out),
        "INTERVAL_EVENT" => run_shallow::<IntervalEvent<Value>>(value, out),
        "ITEM_TABLE" => run::<ItemTable>(value, out),
        // common
        "PARTY_IDENTIFIED" => run::<PartyIdentifiedData>(value, out),
        "PARTY_RELATED" => run::<PartyRelated>(value, out),
        "AUDIT_DETAILS" => run::<AuditDetailsData>(value, out),
        "ATTESTATION" => run::<Attestation>(value, out),
        "FEEDER_AUDIT_DETAILS" => run::<FeederAuditDetails>(value, out),
        "ARCHETYPED" => run::<Archetyped>(value, out),
        // ehr / composition — structural containers (scalar-only invariants),
        // deserialized shallowly (see `run_shallow`). GENERIC_ENTRY's `data:
        // ITEM [1..1]` is a single-valued node, so `run_shallow` keeps it and
        // still enforces its presence.
        "COMPOSITION" => run_shallow::<Composition>(value, out),
        "EVENT_CONTEXT" => run_shallow::<EventContext>(value, out),
        "ACTIVITY" => run_shallow::<Activity>(value, out),
        "INSTRUCTION_DETAILS" => run::<InstructionDetails>(value, out),
        "OBSERVATION" => run_shallow::<Observation>(value, out),
        "INSTRUCTION" => run_shallow::<Instruction>(value, out),
        "ACTION" => run_shallow::<Action>(value, out),
        "EVALUATION" => run_shallow::<Evaluation>(value, out),
        "ADMIN_ENTRY" => run_shallow::<AdminEntry>(value, out),
        "GENERIC_ENTRY" => run_shallow::<GenericEntry>(value, out),
        "SECTION" => run_shallow::<Section>(value, out),
        "FOLDER" => run_shallow::<Folder>(value, out),
        "ITEM_TAG" => run::<ItemTag>(value, out),
        // base identification
        "OBJECT_REF" => run::<ObjectRefData>(value, out),
        "PARTY_REF" => run::<PartyRef>(value, out),
        "VERSION_TREE_ID" => run::<VersionTreeId>(value, out),
        "OBJECT_VERSION_ID" => run::<ObjectVersionId>(value, out),
        "ISO_OID" => run::<IsoOid>(value, out),
        "ARCHETYPE_ID" => run::<ArchetypeId>(value, out),
        "TERMINOLOGY_ID" => run::<TerminologyId>(value, out),
        "INTERNET_ID" => run::<InternetId>(value, out),
        _ => {}
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
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

    /// BASE `Iso8601_date.Day_valid` (`valid_day = d <= days_in_month(m, y)`,
    /// `iso8601_date.adoc` line 107). Calendar-exact month lengths, both the
    /// extended (`YYYY-MM-DD`) and compact (`YYYYMMDD`) forms.
    #[test]
    fn iso_date_day_is_calendar_exact() {
        // 31-day months accept 31; 30-day months reject it.
        assert!(is_valid_iso_date("2021-01-31"));
        assert!(is_valid_iso_date("2021-12-31"));
        assert!(!is_valid_iso_date("2021-04-31")); // April has 30 days
        assert!(!is_valid_iso_date("2021-06-31"));
        assert!(!is_valid_iso_date("2021-09-31"));
        assert!(!is_valid_iso_date("2021-11-31"));
        assert!(is_valid_iso_date("2021-04-30"));

        // February: 28 in a common year, 29 in a leap year, never 30/31.
        assert!(!is_valid_iso_date("2021-02-31"));
        assert!(!is_valid_iso_date("2021-02-30"));
        assert!(!is_valid_iso_date("2021-02-29")); // 2021 is not a leap year
        assert!(is_valid_iso_date("2021-02-28"));
        assert!(is_valid_iso_date("2020-02-29")); // 2020 divisible by 4
        assert!(is_valid_iso_date("2000-02-29")); // 2000 divisible by 400
        assert!(!is_valid_iso_date("1900-02-29")); // 1900 century, not /400

        // Day 00 is never valid.
        assert!(!is_valid_iso_date("2021-05-00"));

        // Compact form is held to the same calendar rule.
        assert!(!is_valid_iso_date("20210431"));
        assert!(!is_valid_iso_date("20210229"));
        assert!(is_valid_iso_date("20200229"));
        assert!(is_valid_iso_date("20210131"));
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

    /// BASE `Iso8601_timezone`: `+` offsets reach +14:00, `-` offsets stop at
    /// -12:00; ±00:00 accepted per the corpus (see `is_valid_tz`).
    #[test]
    fn timezone_bounds_are_asymmetric() {
        assert!(is_valid_iso_time("10:00:00+14:00"));
        assert!(is_valid_iso_time("10:00:00-12:00"));
        assert!(is_valid_iso_time("10:00:00+00:00"));
        assert!(is_valid_iso_time("10:00:00-00:00"));
        assert!(
            !is_valid_iso_time("10:00:00+15:00"),
            "+15 exceeds Max_timezone_hour"
        );
        assert!(
            !is_valid_iso_time("10:00:00-13:00"),
            "-13 exceeds Min_timezone_hour"
        );
    }
}
