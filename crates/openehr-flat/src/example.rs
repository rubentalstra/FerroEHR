//! Example-COMPOSITION generation from a [`WebTemplate`] (the `adl1.4/{id}/example`
//! endpoint).
//!
//! [`example_composition`] walks the web-template tree and produces a canonical
//! openEHR JSON COMPOSITION populated with **deterministic** placeholder data
//! (no randomness, no current-time), at one of three [`DetailLevel`]s:
//!
//! * [`Required`](DetailLevel::Required) — only the mandatory skeleton (nodes on
//!   a fully-mandatory chain, plus whatever a min-cardinality forces). Intended
//!   to be committable as-is.
//! * [`Medium`](DetailLevel::Medium) — the fully-populated single-instance
//!   document: every optional branch descended to its leaves, one occurrence of
//!   each node, the first alternative of any choice. Intended to be committable
//!   as-is. (A former cut at "one level of optional elements" produced *empty*
//!   `content` for the common template shape whose whole content chain is
//!   optional — a populated example is the level's entire point.)
//! * [`Complete`](DetailLevel::Complete) — everything `medium` emits, plus a
//!   second occurrence of each repeating node (demonstrating repetition); not
//!   necessarily committable.
//!
//! The set of populated leaves is monotonic across the levels
//! (`required ⊆ medium ⊆ complete`).
//!
//! # How it works
//!
//! Rather than re-implement the RM housekeeping ([`crate::from_flat`] already
//! materialises the compacted RM structure and fills every RM-mandatory field
//! FLAT never surfaces — language / territory / category / composer / context /
//! ENTRY-mandatory fields / event & history scaffolding), the generator walks the
//! tree to emit a **FLAT map** of deterministic example values and reuses
//! [`from_flat`](crate::from_flat) to assemble the canonical COMPOSITION. The
//! result therefore round-trips cleanly through [`to_flat`](crate::to_flat) and
//! deserialises as an `openehr-rm` `Composition`.
//!
//! # PORT NOTE (non-normative)
//!
//! The endpoint spec is a **post-1.0.3 dev-OAS** addition (absent from the pinned
//! ITS-REST 1.0.3 contract) and states the example-generation algorithm is
//! explicitly non-normative ("vendors may produce different results"). The value
//! choices here (fixed instants, first coded value, range-clamped magnitudes) are
//! ours; only the mandatory-skeleton-is-committable contract of the `required`
//! level is load-bearing (verified by the P15 validator in the crate tests).
//! Reachable-in-content `PARTY_*` value leaves are skipped rather than fabricated
//! (they carry no FLAT round-trip shape); they are almost always optional.

use std::collections::HashSet;

use serde_json::{Map, Value, json};

use crate::from_flat;
use crate::webtemplate::{
    WebTemplate, WebTemplateInput, WebTemplateInputType, WebTemplateNode, WebTemplateRange,
};

/// Fixed example instants used for the RM temporal leaves (deterministic).
const EXAMPLE_DATE_TIME: &str = "2022-02-03T04:05:06Z";
const EXAMPLE_DURATION: &str = "PT1H";

/// The level of detail for a generated example (`detail_level` query parameter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    /// Mandatory skeleton only; intended to be committable without adjustment.
    Required,
    /// Fully populated, one occurrence of every node (first alternative of any
    /// choice); intended to be committable without adjustment.
    Medium,
    /// `Medium` plus a second occurrence of each repeating node; not expected
    /// to be committable.
    Complete,
}

impl DetailLevel {
    /// Parse the `detail_level` query value (default [`Required`](Self::Required)).
    ///
    /// # Errors
    /// A message (→ ITS-REST `400`) for a value outside `required|medium|complete`.
    pub fn from_query(value: Option<&str>) -> Result<Self, String> {
        match value.map(str::trim) {
            None | Some("" | "required") => Ok(Self::Required),
            Some("medium") => Ok(Self::Medium),
            Some("complete") => Ok(Self::Complete),
            Some(other) => Err(format!(
                "unsupported detail_level '{other}' (expected one of required, medium, complete)"
            )),
        }
    }
}

/// The intended use of a generated example (`type` query parameter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExampleType {
    /// As submitted to the repository (committable; no `uid`).
    Input,
    /// As retrieved from the repository (carries a populated `uid`).
    Output,
}

impl ExampleType {
    /// Parse the `type` query value (default [`Input`](Self::Input)).
    ///
    /// # Errors
    /// A message (→ ITS-REST `400`) for a value outside `input|output`.
    pub fn from_query(value: Option<&str>) -> Result<Self, String> {
        match value.map(str::trim) {
            None | Some("" | "input") => Ok(Self::Input),
            Some("output") => Ok(Self::Output),
            Some(other) => Err(format!(
                "unsupported type '{other}' (expected one of input, output)"
            )),
        }
    }
}

/// Generate an example (input-form) COMPOSITION from `wt` at `level`.
///
/// The result is canonical openEHR JSON that deserialises as an `openehr-rm`
/// `Composition`. For the `output` form, additionally call
/// [`apply_output_uid`].
#[must_use]
pub fn example_composition(wt: &WebTemplate, level: DetailLevel) -> Value {
    let mut flat: Map<String, Value> = Map::new();

    // Composition-level housekeeping the tree does not carry as data leaves.
    // `from_flat`/`apply_ctx` turn these into the RM-mandatory language /
    // territory / composer / context (the tree's `default_language` drives the
    // language; territory has no template constraint, so a fixed valid default).
    flat.insert("ctx/language".to_owned(), json!(wt.default_language));
    flat.insert("ctx/territory".to_owned(), json!("US"));
    flat.insert("ctx/composer_name".to_owned(), json!("Example composer"));
    flat.insert("ctx/time".to_owned(), json!(EXAMPLE_DATE_TIME));

    // Walk the tree from the root (COMPOSITION); the root is never forced empty —
    // an empty-content composition is valid RM, and a real content cardinality is
    // honoured by the per-container cardinality pass in `walk`.
    walk(&wt.tree, &wt.tree.id, 0, level, false, &mut flat);

    match from_flat(&flat, wt) {
        Ok(value) => value,
        // `from_flat` does not fail for a well-formed tree; keep a total function.
        Err(_) => Value::Object(Map::new()),
    }
}

/// Populate a deterministic `uid` (an `OBJECT_VERSION_ID`) on an example
/// composition — the `type=output` form (as retrieved from the repository).
///
/// The version-object UUID is derived deterministically from `template_id` (no
/// randomness), so repeated calls yield an identical example.
pub fn apply_output_uid(composition: &mut Value, template_id: &str) {
    if let Value::Object(map) = composition {
        map.insert(
            "uid".to_owned(),
            json!({
                "_type": "OBJECT_VERSION_ID",
                "value": format!("{}::example.server::1", deterministic_uuid(template_id)),
            }),
        );
    }
}

// ── the tree walk ──────────────────────────────────────────────────────────────

/// Walk a container node, emitting the flat entries for its included leaves.
/// Returns whether anything was emitted under this node.
///
/// `opt_depth` is the number of optional (`min < 1`) nodes on the path to this
/// node inclusive; it drives which optional branches each level includes.
/// `force` requests that a (mandatory) container not be left empty.
fn walk(
    node: &WebTemplateNode,
    prefix: &str,
    opt_depth: usize,
    level: DetailLevel,
    force: bool,
    out: &mut Map<String, Value>,
) -> bool {
    if node.has_input() {
        return emit_leaf(node, prefix, out);
    }

    let groups = child_groups(node);
    let mut emitted = false;
    let mut included: Vec<&str> = Vec::new();
    // Materialised instances per container cardinality (`max != -1`), so the
    // populated levels never overrun a container's upper bound — an *optional*
    // child is skipped once its cardinality is full (a mandatory one still
    // materialises; a template whose mandatory children alone exceed the bound
    // is contradictory and the validator's job to report).
    let mut card_counts = vec![0usize; node.cardinalities.len()];
    let card_idx = |child: &WebTemplateNode| {
        node.cardinalities
            .iter()
            .position(|c| c.max != -1 && child.aql_path.starts_with(c.path.as_str()))
    };

    for child in &groups {
        let child_opt = opt_depth + usize::from(is_optional(child));
        let include = match level {
            DetailLevel::Required => child_opt == 0,
            DetailLevel::Medium | DetailLevel::Complete => true,
        };
        if include {
            // An OPTIONAL node whose coded-name constraint is display/rubric-
            // incoherent (see `CodedName::incoherent`) is omitted: no instance
            // form of it is accepted by every conforming consumer, and an
            // example exists to be committable everywhere. A MANDATORY one is
            // still emitted in our spec-faithful form.
            if is_optional(child) && child.name_coded.as_ref().is_some_and(|cn| cn.incoherent) {
                continue;
            }
            let ci = card_idx(child);
            let card_full = |counts: &[usize]| {
                ci.is_some_and(|i| {
                    counts[i] >= usize::try_from(node.cardinalities[i].max).unwrap_or(usize::MAX)
                })
            };
            if is_optional(child) && card_full(&card_counts) {
                continue;
            }
            included.push(child.aql_path.as_str());
            let child_prefix = format!("{prefix}/{}", seg_for(child));
            // A mandatory child must materialise even when all of *its* children
            // are optional (else the mandatory node would go missing).
            let child_force = !is_optional(child);
            let child_emitted = walk(child, &child_prefix, child_opt, level, child_force, out);
            emitted |= child_emitted;
            if child_emitted && let Some(i) = ci {
                card_counts[i] += 1;
            }
            // `Complete` demonstrates repetition: a second occurrence of any
            // repeating node that materialised (cardinality permitting).
            if level == DetailLevel::Complete
                && child_emitted
                && is_repeating(child)
                && !card_full(&card_counts)
            {
                let second_prefix = format!("{prefix}/{}:1", child.id);
                let second_emitted = walk(child, &second_prefix, child_opt, level, false, out);
                emitted |= second_emitted;
                if second_emitted && let Some(i) = ci {
                    card_counts[i] += 1;
                }
            }
        }
    }

    // Cardinality satisfaction: for every container attribute constrained to
    // `min >= 1`, ensure at least one child under it is materialised (committable
    // skeleton), even if the level would otherwise skip it.
    for card in &node.cardinalities {
        if card.min.unwrap_or(0) < 1 {
            continue;
        }
        let satisfied = included.iter().any(|p| p.starts_with(card.path.as_str()));
        if !satisfied
            && let Some(child) = groups.iter().find(|c| c.aql_path.starts_with(&card.path))
        {
            let child_prefix = format!("{prefix}/{}", seg_for(child));
            emitted |= walk(child, &child_prefix, opt_depth, level, true, out);
        }
    }

    // A forced (mandatory) container with nothing yet: drill its first child to a
    // leaf so the node is not emitted empty.
    if force
        && !emitted
        && let Some(child) = groups.first()
    {
        let child_prefix = format!("{prefix}/{}", seg_for(child));
        emitted |= walk(child, &child_prefix, opt_depth, level, true, out);
    }

    emitted
}

/// The children to consider under a container: one per distinct `aql_path` (the
/// first alternative of any polymorphic choice), excluding the `EVENT_CONTEXT`
/// subtree (rebuilt wholesale from the `ctx/…` context keys by `from_flat`).
fn child_groups(node: &WebTemplateNode) -> Vec<&WebTemplateNode> {
    let mut seen: HashSet<&str> = HashSet::new();
    node.children
        .iter()
        .filter(|c| c.rm_type != "EVENT_CONTEXT")
        .filter(|c| seen.insert(c.aql_path.as_str()))
        .collect()
}

/// Whether a node is optional (occurrences lower bound `< 1`).
fn is_optional(node: &WebTemplateNode) -> bool {
    node.min.unwrap_or(0) < 1
}

/// Whether a node may occur more than once (Better's `isRepeating`).
fn is_repeating(node: &WebTemplateNode) -> bool {
    node.max == -1 || node.max > 1
}

/// The flat path segment for a node: `id:0` for a repeating node (Better's
/// `isRepeating`: `max == -1 || max > 1`), else the bare `id` — matching
/// [`to_flat`](crate::to_flat) so the example round-trips.
fn seg_for(node: &WebTemplateNode) -> String {
    if is_repeating(node) {
        format!("{}:0", node.id)
    } else {
        node.id.clone()
    }
}

// ── leaf value emission ─────────────────────────────────────────────────────────

/// Emit the flat `path|suffix` entries for one leaf value at `base`. Returns
/// whether anything was emitted.
fn emit_leaf(node: &WebTemplateNode, base: &str, out: &mut Map<String, Value>) -> bool {
    let rm = node.rm_type.split('<').next().unwrap_or(&node.rm_type);
    match rm {
        "DV_TEXT" | "DV_PARAGRAPH" => {
            put(out, base, "", json!(example_text(node)));
        }
        "DV_URI" | "DV_EHR_URI" => {
            let value = list_value(first_input(node)).unwrap_or_else(|| {
                if rm == "DV_EHR_URI" {
                    "ehr://example.org/composition".to_owned()
                } else {
                    "http://example.org/resource".to_owned()
                }
            });
            put(out, base, "", json!(value));
        }
        "DV_CODED_TEXT" | "DV_STATE" => emit_coded_text(node, base, out),
        "CODE_PHRASE" => emit_code_phrase(node, base, out),
        "DV_QUANTITY" => emit_quantity(node, base, out),
        "DV_COUNT" => {
            let range = first_input(node).and_then(input_range);
            put(out, base, "", json!(pick_count(range)));
        }
        "DV_PROPORTION" => emit_proportion(node, base, out),
        "DV_ORDINAL" => emit_ordinal(node, base, out, false),
        "DV_SCALE" => emit_ordinal(node, base, out, true),
        "DV_BOOLEAN" => put(out, base, "", json!(example_boolean(node))),
        "DV_DATE_TIME" | "DV_DATE" | "DV_TIME" => {
            put(out, base, "", json!(example_temporal(node, rm)));
        }
        "DV_DURATION" => put(out, base, "", json!(example_duration(node))),
        "DV_IDENTIFIER" => put(out, base, "id", json!("example-id")),
        "DV_MULTIMEDIA" => {
            put(out, base, "", json!("http://example.org/media"));
            // Honour a `C_CODE_PHRASE` list on `media_type` (captured in
            // `code_lists` — the constraint the validator enforces).
            let media = node
                .code_lists
                .iter()
                .find(|cl| cl.attr == "media_type")
                .and_then(|cl| cl.codes.first())
                .map_or("text/plain", String::as_str);
            put(out, base, "mediatype", json!(media));
            // A plausible non-zero size. The RM invariant is `size >= 0`
            // (RM data_types §DV_MULTIMEDIA `Size_valid`), so 0 is valid —
            // but a referenced resource of zero bytes is unreal, and the
            // reference implementation's known `size > 0` quirk rejects it
            // (the DV_MULTIMEDIA PORT NOTE); a realistic example avoids both.
            put(out, base, "size", json!(1024));
        }
        "DV_PARSABLE" => {
            put(out, base, "", json!("example"));
            put(out, base, "formalism", json!("text"));
        }
        // PARTY_PROXY / PARTY_IDENTIFIED value leaves carry no FLAT round-trip
        // shape (they are rebuilt from `ctx/…`, not tree data); skip rather than
        // fabricate an incomplete party. See the module PORT NOTE.
        _ => return false,
    }
    true
}

/// Insert a flat entry at `base` (bare) or `base|suffix`.
fn put(out: &mut Map<String, Value>, base: &str, suffix: &str, value: Value) {
    let key = if suffix.is_empty() {
        base.to_owned()
    } else {
        format!("{base}|{suffix}")
    };
    out.insert(key, value);
}

/// A deterministic text value: the first enumerated value (a non-open string
/// list), else the node's rubric name, else a generic placeholder.
fn example_text(node: &WebTemplateNode) -> String {
    if let Some(input) = first_input(node)
        && input.input_type == WebTemplateInputType::Text
        && input.list_open != Some(true)
        && let Some(cv) = input.list.first()
    {
        return cv.value.clone();
    }
    node.name
        .clone()
        .unwrap_or_else(|| "Example text".to_owned())
}

fn emit_coded_text(node: &WebTemplateNode, base: &str, out: &mut Map<String, Value>) {
    let input = input_with_suffix(node, "code").or_else(|| first_input(node));
    let (code, value, terminology) = coded_example(input);
    put(out, base, "code", json!(code));
    put(out, base, "value", json!(value));
    put(out, base, "terminology", json!(terminology));
}

fn emit_code_phrase(node: &WebTemplateNode, base: &str, out: &mut Map<String, Value>) {
    let input = input_with_suffix(node, "code").or_else(|| first_input(node));
    let (code, _value, terminology) = coded_example(input);
    put(out, base, "code", json!(code));
    put(out, base, "terminology", json!(terminology));
}

/// The `(code, value, terminology)` for an example coded value: the first entry
/// of the input's coded list, else a placeholder local code. The list labels
/// already carry the display text — term definitions, then the TERM 3.1.0
/// rubric for `openehr` codes (`webtemplate::inputs::coded_value`).
fn coded_example(input: Option<&WebTemplateInput>) -> (String, String, String) {
    match input {
        Some(i) if !i.list.is_empty() => {
            let cv = &i.list[0];
            let terminology = i.terminology.clone().unwrap_or_else(|| "local".to_owned());
            let value = cv.label.clone().unwrap_or_else(|| cv.value.clone());
            (cv.value.clone(), value, terminology)
        }
        Some(i) => (
            "at0000".to_owned(),
            "Example".to_owned(),
            i.terminology.clone().unwrap_or_else(|| "local".to_owned()),
        ),
        None => (
            "at0000".to_owned(),
            "Example".to_owned(),
            "local".to_owned(),
        ),
    }
}

fn emit_quantity(node: &WebTemplateNode, base: &str, out: &mut Map<String, Value>) {
    let unit_input = input_with_suffix(node, "unit");
    let unit = unit_input
        .and_then(|i| i.list.first())
        .map_or_else(|| "1".to_owned(), |cv| cv.value.clone());
    // Prefer the magnitude input's own range, else the range scoped to the unit.
    let range = input_with_suffix(node, "magnitude")
        .and_then(input_range)
        .or_else(|| {
            unit_input
                .and_then(|i| i.list.first())
                .and_then(|cv| cv.validation.as_ref())
                .and_then(|v| v.range.as_ref())
        });
    put(out, base, "magnitude", json!(pick_decimal(range)));
    put(out, base, "unit", json!(unit));
}

fn emit_proportion(node: &WebTemplateNode, base: &str, out: &mut Map<String, Value>) {
    // Choose values satisfying the DV_PROPORTION invariants for the first allowed
    // kind (unitary → denominator 1; percent → 100; fraction/integer_fraction →
    // integral). `type` codes: ratio 0, unitary 1, percent 2, fraction 3,
    // integer_fraction 4.
    let (numerator, denominator, type_code) =
        match node.proportion_types.first().map(String::as_str) {
            Some("unitary") => (1.0, 1.0, 1),
            Some("percent") => (50.0, 100.0, 2),
            Some("fraction") => (1.0, 2.0, 3),
            Some("integer_fraction") => (1.0, 2.0, 4),
            _ => (1.0, 1.0, 0),
        };
    put(out, base, "numerator", json!(numerator));
    put(out, base, "denominator", json!(denominator));
    put(out, base, "type", json!(type_code));
}

fn emit_ordinal(node: &WebTemplateNode, base: &str, out: &mut Map<String, Value>, scale: bool) {
    let numeric_suffix = if scale { "scale" } else { "ordinal" };
    if let Some(cv) = first_input(node).and_then(|i| i.list.first()) {
        let numeric = if scale {
            json!(cv.scale.unwrap_or(0.0))
        } else {
            json!(cv.ordinal.unwrap_or(0))
        };
        put(out, base, numeric_suffix, numeric);
        put(out, base, "code", json!(cv.value));
        put(
            out,
            base,
            "value",
            json!(cv.label.clone().unwrap_or_else(|| cv.value.clone())),
        );
    } else {
        put(
            out,
            base,
            numeric_suffix,
            if scale { json!(0.0) } else { json!(0) },
        );
        put(out, base, "code", json!("at0000"));
        put(out, base, "value", json!("Example"));
    }
}

/// A deterministic temporal example honouring the leaf's AOM 1.4 temporal
/// pattern (`yyyy-??-XX` etc.: literal = mandatory, `??` = optional/kept,
/// `XX` = prohibited/omitted — mirroring the validator's presence-only
/// judgement) and the `timezone_validity` constraint (`1003` = the designator
/// must be absent; the fixed example instants already carry `Z` for the rest).
fn example_temporal(node: &WebTemplateNode, rm: &str) -> String {
    let pattern = node
        .inputs
        .first()
        .and_then(|i| i.validation.as_ref())
        .and_then(|v| v.pattern.as_deref());
    let (pat_date, pat_time) = match pattern {
        Some(p) if p.contains('T') => {
            let (d, t) = p.split_once('T').unwrap_or((p, ""));
            (d, t)
        }
        Some(p) if p.contains(':') => ("", p),
        Some(p) => (p, ""),
        None => ("", ""),
    };
    // Date segments: year always; month/day kept unless the pattern prohibits
    // them (an omitted month forces the day off — ISO 8601 partial precision
    // truncates right-to-left).
    let date_full = ["2022", "02", "03"];
    let date = |pat: &str| -> String {
        let segs: Vec<&str> = if pat.is_empty() {
            Vec::new()
        } else {
            pat.splitn(3, '-').collect()
        };
        let mut out = String::from(date_full[0]);
        for (i, part) in date_full.iter().enumerate().skip(1) {
            if segs.get(i).is_some_and(|s| *s == "XX") {
                break;
            }
            out.push('-');
            out.push_str(part);
        }
        out
    };
    let time_full = ["04", "05", "06"];
    let time = |pat: &str| -> String {
        let segs: Vec<&str> = if pat.is_empty() {
            Vec::new()
        } else {
            pat.splitn(3, ':').collect()
        };
        let mut out = String::from(time_full[0]);
        for (i, part) in time_full.iter().enumerate().skip(1) {
            if segs.get(i).is_some_and(|s| *s == "XX") {
                break;
            }
            out.push(':');
            out.push_str(part);
        }
        out
    };
    let tz = if node.tz_validity == Some(1003) {
        ""
    } else {
        "Z"
    };
    match rm {
        "DV_DATE" => date(pat_date),
        "DV_TIME" => format!("{}{tz}", time(pat_time)),
        // DV_DATE_TIME: a time part prohibited outright (pattern `…TXX:…`)
        // truncates to the date.
        _ => {
            if pat_time.starts_with("XX") {
                date(pat_date)
            } else {
                format!("{}T{}{tz}", date(pat_date), time(pat_time))
            }
        }
    }
}

/// A deterministic duration example honouring the `C_DURATION` allowed-fields
/// pattern (encoded by the builder as which per-field inputs exist — mirroring
/// the validator) and the ISO duration range: the range minimum when one is
/// declared, else one unit of the first allowed field, else `PT1H`.
fn example_duration(node: &WebTemplateNode) -> String {
    if let Some(range) = node.duration_range.as_ref()
        && let Some(min) = range.min.as_ref().and_then(Value::as_str)
    {
        // An INCLUSIVE minimum is itself a valid pick; an exclusive one
        // (`> PT0S` — BASE Interval lower_included = false) falls through to
        // the one-unit-of-first-allowed-field pick, which is strictly above
        // a zero minimum (the only exclusive-minimum shape in the corpus;
        // a non-zero exclusive minimum would need min+1 — extend when one
        // appears rather than guessing).
        if range.min_op.as_deref() != Some(">") {
            return min.to_owned();
        }
    }
    let iso = [
        ("year", "P1Y"),
        ("month", "P1M"),
        ("week", "P1W"),
        ("day", "P1D"),
        ("hour", "PT1H"),
        ("minute", "PT1M"),
        ("second", "PT1S"),
    ];
    let allowed: Vec<&str> = node
        .inputs
        .iter()
        .filter_map(|i| i.suffix.as_deref())
        .collect();
    if allowed.is_empty() || allowed.len() >= 7 {
        return EXAMPLE_DURATION.to_owned();
    }
    iso.iter()
        .find(|(field, _)| allowed.contains(field))
        .map_or_else(|| EXAMPLE_DURATION.to_owned(), |(_, v)| (*v).to_owned())
}

/// A deterministic boolean example: `false` when the archetype allows only
/// `false`, else `true`.
fn example_boolean(node: &WebTemplateNode) -> bool {
    let only_false = first_input(node).is_some_and(|i| {
        i.list.iter().any(|c| c.value == "false") && !i.list.iter().any(|c| c.value == "true")
    });
    !only_false
}

// ── input helpers ───────────────────────────────────────────────────────────────

fn first_input(node: &WebTemplateNode) -> Option<&WebTemplateInput> {
    node.inputs.first()
}

fn input_with_suffix<'a>(node: &'a WebTemplateNode, suffix: &str) -> Option<&'a WebTemplateInput> {
    node.inputs
        .iter()
        .find(|i| i.suffix.as_deref() == Some(suffix))
}

/// The first enumerated value of a (non-open) text input, if any.
fn list_value(input: Option<&WebTemplateInput>) -> Option<String> {
    let input = input?;
    if input.list_open == Some(true) {
        return None;
    }
    input.list.first().map(|cv| cv.value.clone())
}

fn input_range(input: &WebTemplateInput) -> Option<&WebTemplateRange> {
    input.validation.as_ref().and_then(|v| v.range.as_ref())
}

// ── numeric picking (deterministic, range-clamped) ──────────────────────────────

/// A plausible, non-zero decimal example value within `range`.
///
/// An unconstrained magnitude gets a deterministic non-zero default
/// ([`DEFAULT_DECIMAL`]); a range-constrained one is placed at the range
/// **midpoint** when both bounds exist (honouring the open-bound `>`/`<`
/// operators), and clamped toward the single stated bound otherwise. The result
/// never lands on `0.0` when the range admits a non-zero value: a zero example
/// magnitude is clinically unreal and defeats the multiplicative payload jitter
/// the example skeletons feed. Deterministic (no randomness): the same range
/// always yields the same value. The FLAT/STRUCTURED formats have no versioned
/// openEHR spec of their own — this pick policy is our own design.
fn pick_decimal(range: Option<&WebTemplateRange>) -> f64 {
    /// The default when no numeric bound is stated (a plausible non-zero value).
    const DEFAULT_DECIMAL: f64 = 10.0;
    /// The step taken across an open (`>` / `<`) bound.
    const STEP: f64 = 1.0;

    let Some(r) = range else {
        return DEFAULT_DECIMAL;
    };
    // Effective bounds, moved just inside an open (`>` / `<`) constraint.
    let lo = r.min.as_ref().and_then(Value::as_f64).map(|min| {
        if r.min_op.as_deref() == Some(">") {
            min + STEP
        } else {
            min
        }
    });
    let hi = r.max.as_ref().and_then(Value::as_f64).map(|max| {
        if r.max_op.as_deref() == Some("<") {
            max - STEP
        } else {
            max
        }
    });
    match (lo, hi) {
        (Some(lo), Some(hi)) => {
            let mid = f64::midpoint(lo, hi);
            // A midpoint of 0.0 on a range symmetric about zero (e.g. [-5, 5])
            // would land on the forbidden zero even though a non-zero value is
            // in range — bias to the positive half instead.
            if mid == 0.0 && hi > 0.0 {
                hi / 2.0
            } else {
                mid
            }
        }
        // Only a lower bound: sit at the floor, or the default when that is not
        // above zero (so the pick stays non-zero and in range).
        (Some(lo), None) => lo.max(DEFAULT_DECIMAL),
        // Only an upper bound: sit at the ceiling, or the default when that is
        // not below zero; a non-positive ceiling admits only a value below it.
        (None, Some(hi)) => {
            let v = hi.min(DEFAULT_DECIMAL);
            if v == 0.0 { hi - STEP } else { v }
        }
        (None, None) => DEFAULT_DECIMAL,
    }
}

/// An integer example value clamped into `range` (defaulting to `0`).
fn pick_count(range: Option<&WebTemplateRange>) -> i64 {
    let Some(r) = range else { return 0 };
    let mut v: i64 = 0;
    if let Some(min) = r.min.as_ref().and_then(Value::as_i64) {
        let lo = if r.min_op.as_deref() == Some(">") {
            min + 1
        } else {
            min
        };
        if v < lo {
            v = lo;
        }
    }
    if let Some(max) = r.max.as_ref().and_then(Value::as_i64) {
        let hi = if r.max_op.as_deref() == Some("<") {
            max - 1
        } else {
            max
        };
        if v > hi {
            v = hi;
        }
    }
    v
}

// ── deterministic UUID (for the output-form uid) ────────────────────────────────

/// A deterministic, RFC-4122-shaped UUID derived from `seed` (FNV-1a; no
/// randomness). Not a cryptographic v5, but syntactically a valid UUID and stable
/// for a given seed — sufficient for a pedagogical example's `OBJECT_VERSION_ID`.
fn deterministic_uuid(seed: &str) -> String {
    let h1 = fnv1a64(seed.as_bytes(), 0xcbf2_9ce4_8422_2325);
    let h2 = fnv1a64(seed.as_bytes(), 0x8422_2325_cbf2_9ce4);
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&h1.to_be_bytes());
    b[8..].copy_from_slice(&h2.to_be_bytes());
    b[6] = (b[6] & 0x0f) | 0x50; // version 5 nibble
    b[8] = (b[8] & 0x3f) | 0x80; // RFC-4122 variant
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15],
    )
}

fn fnv1a64(data: &[u8], basis: u64) -> u64 {
    let mut hash = basis;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_web_template;

    fn web_template(opt_rel: &str) -> WebTemplate {
        let path = format!("{}/{opt_rel}", env!("CARGO_MANIFEST_DIR"));
        let xml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let opt = openehr_its::opt14::from_xml(&xml).expect("parse OPT");
        build_web_template(&opt).expect("build web template")
    }

    /// The Demo Vitals template (OBSERVATION with a history of events) vendored
    /// for the FLAT tests.
    fn demo_vitals() -> WebTemplate {
        web_template("tests/fixtures/better/Demo Vitals.opt")
    }

    fn leaf_count(comp: &Value) -> usize {
        // Count populated DATA_VALUE leaves as a coarse coverage metric.
        fn rec(v: &Value, n: &mut usize) {
            match v {
                Value::Object(m) => {
                    if m.get("_type")
                        .and_then(Value::as_str)
                        .is_some_and(|t| t.starts_with("DV_"))
                    {
                        *n += 1;
                    }
                    for val in m.values() {
                        rec(val, n);
                    }
                }
                Value::Array(a) => {
                    for e in a {
                        rec(e, n);
                    }
                }
                _ => {}
            }
        }
        let mut n = 0;
        rec(comp, &mut n);
        n
    }

    #[test]
    fn parses_detail_level() {
        assert_eq!(DetailLevel::from_query(None), Ok(DetailLevel::Required));
        assert_eq!(
            DetailLevel::from_query(Some("required")),
            Ok(DetailLevel::Required)
        );
        assert_eq!(
            DetailLevel::from_query(Some("medium")),
            Ok(DetailLevel::Medium)
        );
        assert_eq!(
            DetailLevel::from_query(Some("complete")),
            Ok(DetailLevel::Complete)
        );
        assert!(DetailLevel::from_query(Some("full")).is_err());
    }

    #[test]
    fn parses_example_type() {
        assert_eq!(ExampleType::from_query(None), Ok(ExampleType::Input));
        assert_eq!(
            ExampleType::from_query(Some("output")),
            Ok(ExampleType::Output)
        );
        assert!(ExampleType::from_query(Some("bogus")).is_err());
    }

    #[test]
    fn is_deterministic() {
        let wt = demo_vitals();
        let a = example_composition(&wt, DetailLevel::Medium);
        let b = example_composition(&wt, DetailLevel::Medium);
        assert_eq!(a, b, "two calls produce identical JSON");
    }

    #[test]
    fn is_a_composition_with_housekeeping() {
        let wt = demo_vitals();
        let comp = example_composition(&wt, DetailLevel::Required);
        assert_eq!(
            comp.get("_type").and_then(Value::as_str),
            Some("COMPOSITION")
        );
        assert!(comp.get("language").is_some(), "mandatory language present");
        assert!(
            comp.get("territory").is_some(),
            "mandatory territory present"
        );
        assert!(comp.get("composer").is_some(), "mandatory composer present");
        assert!(comp.get("category").is_some(), "mandatory category present");
        assert_eq!(
            comp.pointer("/composer/name").and_then(Value::as_str),
            Some("Example composer")
        );
        assert_eq!(
            comp.pointer("/archetype_details/template_id/value")
                .and_then(Value::as_str),
            Some(wt.template_id.as_str()),
            "self-describing template id"
        );
    }

    #[test]
    fn medium_populates_optional_content() {
        // The level's whole point: a template whose content chain is entirely
        // optional still yields a populated document at `medium` (the empty
        // `required` skeleton is the committable *minimum*, not the example).
        let wt = demo_vitals();
        let comp = example_composition(&wt, DetailLevel::Medium);
        let content = comp.get("content").and_then(Value::as_array);
        assert!(
            content.is_some_and(|c| !c.is_empty()),
            "medium example carries content entries"
        );
        assert!(
            leaf_count(&comp) > leaf_count(&example_composition(&wt, DetailLevel::Required)),
            "medium populates strictly more leaves than required"
        );
    }

    #[test]
    fn complete_demonstrates_repetition() {
        let wt = demo_vitals();
        let medium = example_composition(&wt, DetailLevel::Medium);
        let complete = example_composition(&wt, DetailLevel::Complete);
        assert!(
            leaf_count(&complete) > leaf_count(&medium),
            "complete adds second occurrences of repeating nodes"
        );
    }

    #[test]
    fn levels_are_monotonic() {
        let wt = demo_vitals();
        let required = leaf_count(&example_composition(&wt, DetailLevel::Required));
        let medium = leaf_count(&example_composition(&wt, DetailLevel::Medium));
        let complete = leaf_count(&example_composition(&wt, DetailLevel::Complete));
        assert!(
            required <= medium && medium <= complete,
            "coverage grows with detail: required={required}, medium={medium}, complete={complete}"
        );
        assert!(
            complete > required,
            "a rich template exposes more leaves at complete than required \
             (required={required}, complete={complete})"
        );
    }

    #[test]
    fn output_populates_a_deterministic_uid() {
        let wt = demo_vitals();
        let mut input = example_composition(&wt, DetailLevel::Required);
        assert!(input.get("uid").is_none(), "input form has no uid");

        let mut output = input.clone();
        apply_output_uid(&mut output, &wt.template_id);
        let uid = output
            .pointer("/uid/value")
            .and_then(Value::as_str)
            .expect("output uid");
        assert!(
            uid.ends_with("::example.server::1"),
            "OBJECT_VERSION_ID form: {uid}"
        );
        assert_eq!(
            output.pointer("/uid/_type").and_then(Value::as_str),
            Some("OBJECT_VERSION_ID")
        );

        // Deterministic across calls.
        apply_output_uid(&mut input, &wt.template_id);
        assert_eq!(input.get("uid"), output.get("uid"));
    }

    fn range(
        min: Option<f64>,
        min_op: Option<&str>,
        max: Option<f64>,
        max_op: Option<&str>,
    ) -> WebTemplateRange {
        WebTemplateRange {
            min_op: min_op.map(str::to_owned),
            min: min.map(|m| json!(m)),
            max_op: max_op.map(str::to_owned),
            max: max.map(|m| json!(m)),
        }
    }

    #[test]
    #[allow(clippy::float_cmp)] // the picker returns exactly representable values
    fn pick_decimal_is_non_zero_and_in_range() {
        // Unconstrained: a plausible non-zero default, not 0.0.
        assert_eq!(pick_decimal(None), 10.0);
        // Both bounds present: the midpoint (never the min, never 0.0).
        assert_eq!(
            pick_decimal(Some(&range(Some(0.0), None, Some(10.0), None))),
            5.0
        );
        assert_eq!(
            pick_decimal(Some(&range(Some(60.0), None, Some(80.0), None))),
            70.0
        );
        // A range symmetric about zero must not yield the forbidden 0.0.
        let v = pick_decimal(Some(&range(Some(-5.0), None, Some(5.0), None)));
        assert!(
            v != 0.0 && (-5.0..=5.0).contains(&v),
            "in range, non-zero: {v}"
        );
        // Only a lower bound: at the floor when it is above zero, else the default.
        assert_eq!(
            pick_decimal(Some(&range(Some(50.0), None, None, None))),
            50.0
        );
        assert_eq!(
            pick_decimal(Some(&range(Some(0.0), None, None, None))),
            10.0
        );
        // Only an upper bound: below the ceiling, non-zero.
        assert_eq!(
            pick_decimal(Some(&range(None, None, Some(100.0), None))),
            10.0
        );
        assert_eq!(pick_decimal(Some(&range(None, None, Some(4.0), None))), 4.0);
        // Open bounds are honoured (`>` / `<` exclude the stated value).
        assert_eq!(
            pick_decimal(Some(&range(Some(0.0), Some(">"), None, None))),
            10.0
        );
        let hi = pick_decimal(Some(&range(None, None, Some(1.0), Some("<"))));
        assert!(hi < 1.0, "strictly below an open upper bound: {hi}");
    }

    #[test]
    #[allow(clippy::float_cmp)] // a magnitude of exactly 0.0 is the value under test
    fn pick_decimal_populated_magnitudes_are_non_zero() {
        fn collect(v: &Value, out: &mut Vec<f64>) {
            match v {
                Value::Object(m) => {
                    if m.get("_type").and_then(Value::as_str) == Some("DV_QUANTITY")
                        && let Some(x) = m.get("magnitude").and_then(Value::as_f64)
                    {
                        out.push(x);
                    }
                    m.values().for_each(|c| collect(c, out));
                }
                Value::Array(a) => a.iter().for_each(|e| collect(e, out)),
                _ => {}
            }
        }
        // A populated example must not seed a zero-valued quantity magnitude
        // (clinically unreal; defeats multiplicative jitter).
        let wt = demo_vitals();
        let comp = example_composition(&wt, DetailLevel::Medium);
        let mut mags = Vec::new();
        collect(&comp, &mut mags);
        assert!(!mags.is_empty(), "Demo Vitals exposes DV_QUANTITY leaves");
        assert!(
            mags.iter().all(|m| *m != 0.0),
            "no example magnitude is zero: {mags:?}"
        );
    }
}
