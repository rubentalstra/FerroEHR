//! Leaf domain-constraint validation (the `C_DV`_* / `C_PRIMITIVE` checks archie
//! runs on a `DATA_VALUE` against its archetype constraint), approximated from the
//! `WebTemplate` `inputs`.
//!
//! For each input we validate the corresponding datum of the instance value,
//! keyed by the input `suffix` / `type`: coded-value membership (unless the list
//! is open or the code is from an external terminology), numeric range (honoring
//! `minOp`/`maxOp`), and string patterns. Temporal ranges and decimal precision
//! are intentionally not checked here (`// PORT NOTE:` — RM well-formedness of
//! date/time/duration values is covered by the RM-invariant pass; precise
//! temporal-range and precision semantics are deferred).

use serde_json::Value;

use super::{ValidationKind, Validator};
use crate::webtemplate::{
    WebTemplateInput, WebTemplateInputType, WebTemplateNode, WebTemplateRange,
};

pub(super) fn check_inputs(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let rm = wt.rm_type.split('<').next().unwrap_or(&wt.rm_type);
    match rm {
        "DV_CODED_TEXT" | "DV_STATE" => check_coded_text(v, instance, wt),
        "CODE_PHRASE" => check_code_phrase(v, instance, wt),
        "DV_ORDINAL" | "DV_SCALE" => check_ordinal(v, instance, wt),
        "DV_QUANTITY" => check_quantity(v, instance, wt),
        "DV_COUNT" => check_count(v, instance, wt),
        "DV_PROPORTION" => check_proportion(v, instance, wt),
        "DV_TEXT" | "DV_URI" | "DV_EHR_URI" => check_text(v, instance, wt),
        "DV_BOOLEAN" => check_boolean(v, instance, wt),
        "DV_IDENTIFIER" => check_identifier(v, instance, wt),
        _ => {}
    }
}

// ── coded text / code phrase / ordinal ─────────────────────────────────────────

fn check_coded_text(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let Some(code_input) = input_with_suffix(wt, "code") else {
        return;
    };
    let code = instance
        .get("defining_code")
        .and_then(|c| c.get("code_string"))
        .and_then(Value::as_str);
    let terminology = instance
        .get("defining_code")
        .and_then(|c| c.get("terminology_id"))
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str);
    check_code_membership(v, wt, code_input, code, terminology);
}

fn check_code_phrase(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let Some(code_input) = input_with_suffix(wt, "code").or_else(|| wt.inputs.first()) else {
        return;
    };
    let code = instance.get("code_string").and_then(Value::as_str);
    let terminology = instance
        .get("terminology_id")
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str);
    check_code_membership(v, wt, code_input, code, terminology);
}

fn check_ordinal(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let Some(input) = wt.inputs.first() else {
        return;
    };
    // DV_ORDINAL/DV_SCALE encode their coded symbol under `symbol/defining_code`.
    let code = instance
        .get("symbol")
        .and_then(|s| s.get("defining_code"))
        .and_then(|c| c.get("code_string"))
        .and_then(Value::as_str);
    let terminology = instance
        .get("symbol")
        .and_then(|s| s.get("defining_code"))
        .and_then(|c| c.get("terminology_id"))
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str);
    check_code_membership(v, wt, input, code, terminology);
}

/// Report a coded value that is not among the constrained options. Only checked
/// for an internal (`local`/empty) terminology and a non-open, non-empty list —
/// external-terminology codes cannot be validated against the archetype list.
fn check_code_membership(
    v: &mut Validator,
    wt: &WebTemplateNode,
    input: &WebTemplateInput,
    code: Option<&str>,
    terminology: Option<&str>,
) {
    if input.list.is_empty() || input.list_open == Some(true) {
        return;
    }
    let is_internal = matches!(terminology, None | Some("local" | ""));
    if !is_internal {
        return;
    }
    let Some(code) = code else { return };
    if !input.list.iter().any(|cv| cv.value == code) {
        v.push(
            &wt.aql_path,
            format!("coded value '{code}' is not in the constrained value set"),
            ValidationKind::CodedValue,
        );
    }
}

// ── quantity / count / proportion ──────────────────────────────────────────────

fn check_quantity(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let unit = instance.get("units").and_then(Value::as_str);
    let unit_input = input_with_suffix(wt, "unit");

    // Unit membership.
    if let (Some(ui), Some(u)) = (unit_input, unit)
        && !ui.list.is_empty()
        && ui.list_open != Some(true)
        && !ui.list.iter().any(|cv| cv.value == u)
    {
        v.push(
            &wt.aql_path,
            format!("unit '{u}' is not among the constrained units"),
            ValidationKind::CodedValue,
        );
    }

    // Magnitude range: prefer the magnitude input's own range, else the range on
    // the coded value for the instance's unit.
    if let Some(mag) = instance.get("magnitude").and_then(as_f64) {
        let range = input_with_suffix(wt, "magnitude")
            .and_then(|i| i.validation.as_ref())
            .and_then(|val| val.range.as_ref())
            .or_else(|| unit_scoped_range(unit_input, unit));
        if let Some(range) = range
            && !in_range(mag, range)
        {
            v.push(
                &wt.aql_path,
                format!("magnitude {mag} is outside the constrained range"),
                ValidationKind::RangeError,
            );
        }
    }
}

fn unit_scoped_range<'a>(
    unit_input: Option<&'a WebTemplateInput>,
    unit: Option<&str>,
) -> Option<&'a WebTemplateRange> {
    let (ui, u) = (unit_input?, unit?);
    ui.list
        .iter()
        .find(|cv| cv.value == u)
        .and_then(|cv| cv.validation.as_ref())
        .and_then(|val| val.range.as_ref())
}

fn check_count(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let Some(mag) = instance.get("magnitude").and_then(as_f64) else {
        return;
    };
    if let Some(range) = wt
        .inputs
        .first()
        .and_then(|i| i.validation.as_ref())
        .and_then(|val| val.range.as_ref())
        && !in_range(mag, range)
    {
        v.push(
            &wt.aql_path,
            format!("count {mag} is outside the constrained range"),
            ValidationKind::RangeError,
        );
    }
}

fn check_proportion(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    for part in ["numerator", "denominator"] {
        let Some(value) = instance.get(part).and_then(as_f64) else {
            continue;
        };
        if let Some(range) = input_with_suffix(wt, part)
            .and_then(|i| i.validation.as_ref())
            .and_then(|val| val.range.as_ref())
            && !in_range(value, range)
        {
            v.push(
                &wt.aql_path,
                format!("{part} {value} is outside the constrained range"),
                ValidationKind::RangeError,
            );
        }
    }
}

// ── text / boolean / identifier ────────────────────────────────────────────────

fn check_text(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let Some(input) = wt.inputs.first() else {
        return;
    };
    let Some(value) = instance.get("value").and_then(Value::as_str) else {
        return;
    };
    check_string_constraints(v, wt, input, value);
}

fn check_boolean(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    let Some(input) = wt.inputs.first() else {
        return;
    };
    let Some(value) = instance.get("value").and_then(Value::as_bool) else {
        return;
    };
    // A list restricts booleans to a single literal ("true" / "false").
    if !input.list.is_empty() && !input.list.iter().any(|cv| cv.value == value.to_string()) {
        v.push(
            &wt.aql_path,
            format!("boolean value {value} is not permitted"),
            ValidationKind::CodedValue,
        );
    }
}

fn check_identifier(v: &mut Validator, instance: &Value, wt: &WebTemplateNode) {
    for part in ["id", "type", "issuer", "assigner"] {
        let Some(input) = input_with_suffix(wt, part) else {
            continue;
        };
        let Some(value) = instance.get(part).and_then(Value::as_str) else {
            continue;
        };
        check_string_constraints(v, wt, input, value);
    }
}

/// A string datum against an enumerated value list and/or a regex pattern.
fn check_string_constraints(
    v: &mut Validator,
    wt: &WebTemplateNode,
    input: &WebTemplateInput,
    value: &str,
) {
    if input.input_type == WebTemplateInputType::Text
        && !input.list.is_empty()
        && input.list_open != Some(true)
        && !input.list.iter().any(|cv| cv.value == value)
    {
        v.push(
            &wt.aql_path,
            format!("value '{value}' is not in the constrained value set"),
            ValidationKind::CodedValue,
        );
    }
    if let Some(pattern) = input
        .validation
        .as_ref()
        .and_then(|val| val.pattern.as_ref())
        && !matches_pattern(pattern, value)
    {
        v.push(
            &wt.aql_path,
            format!("value '{value}' does not match pattern /{pattern}/"),
            ValidationKind::PatternError,
        );
    }
}

/// Whether `value` matches an archetype `C_STRING` regex. ADL delimits regexes
/// with `/`…`/`; the match is full-string (anchored). A pattern that does not
/// compile is skipped (returns `true`) rather than reported, since we cannot be
/// sure of its intended semantics.
fn matches_pattern(pattern: &str, value: &str) -> bool {
    let body = pattern
        .strip_prefix('/')
        .and_then(|p| p.strip_suffix('/'))
        .unwrap_or(pattern);
    let anchored = format!("^(?:{body})$");
    match regex::Regex::new(&anchored) {
        Ok(re) => re.is_match(value),
        Err(_) => true,
    }
}

// ── numeric helpers ─────────────────────────────────────────────────────────────

fn input_with_suffix<'a>(wt: &'a WebTemplateNode, suffix: &str) -> Option<&'a WebTemplateInput> {
    wt.inputs
        .iter()
        .find(|i| i.suffix.as_deref() == Some(suffix))
}

fn as_f64(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// Whether `value` satisfies a `WebTemplate` numeric range (honoring the
/// inclusive/exclusive `minOp`/`maxOp`; missing bounds are unbounded).
fn in_range(value: f64, range: &WebTemplateRange) -> bool {
    if let Some(min) = range.min.as_ref().and_then(as_f64) {
        let ok = match range.min_op.as_deref() {
            Some(">") => value > min,
            _ => value >= min,
        };
        if !ok {
            return false;
        }
    }
    if let Some(max) = range.max.as_ref().and_then(as_f64) {
        let ok = match range.max_op.as_deref() {
            Some("<") => value < max,
            _ => value <= max,
        };
        if !ok {
            return false;
        }
    }
    true
}
