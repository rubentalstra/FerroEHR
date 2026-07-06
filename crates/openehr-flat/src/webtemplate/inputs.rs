//! RM-type → `inputs` mapping (Better `builder/input/*`).
//!
//! Each leaf `DATA_VALUE` / PARTY node is given its `inputs` (and, for
//! `DV_PROPORTION`, the node's `proportionTypes`). The dispatch and per-builder
//! shape follow `WebTemplateInputBuilderDelegator` and the individual
//! `*WebTemplateInputBuilder`s: suffixes, input types, coded lists, and
//! validation ranges are transcribed verbatim (`|unit` is singular, per Better).
//!
//! Not modelled here (scope boundaries for this PR, recorded as `TODO(port)`):
//! `defaultValue` from assumed/RM-default values, external `otherTerminologies`,
//! and the openEHR-terminology rubric lookup (labels fall back to the code when
//! the archetype rubric is unknown, as for non-`local` terminologies).

use indexmap::IndexMap;
use openehr_its::opt14::{CAttribute, CObject, CPrimitive, Intervalofinteger, Intervalofreal};

use super::model::{
    WebTemplateBindingCodedValue, WebTemplateCodedValue, WebTemplateInput, WebTemplateInputType,
    WebTemplateRange, WebTemplateValidation,
};

/// Resolves rubric text for a `(terminology, code)` pair (the archetype
/// ontology, in the builder). Non-`local` terminologies and unknown codes yield
/// `None`, and the label falls back to the code string.
pub(crate) trait Labels {
    fn text(&self, terminology: &str, code: &str) -> Option<String>;
    fn localized(&self, terminology: &str, code: &str) -> IndexMap<String, String>;
    /// External terminology bindings for `code` (the archetype ontology's
    /// `term_bindings`), keyed by terminology — Better `findTermBindings` +
    /// `getBindingCodedValue`, populating each coded value's `termBindings`.
    fn term_bindings(&self, code: &str) -> IndexMap<String, WebTemplateBindingCodedValue>;
}

/// Build the `inputs` (and `proportion_types`) for a leaf node.
pub(crate) fn build_inputs(
    rm_type: &str,
    co: &CObject,
    labels: &dyn Labels,
) -> (Vec<WebTemplateInput>, Vec<String>) {
    // Strip any generic argument (`DV_INTERVAL<DV_QUANTITY>` → `DV_INTERVAL`).
    let base = rm_type.split('<').next().unwrap_or(rm_type);
    let mut proportion_types = Vec::new();
    let inputs = match base {
        "DV_TEXT" | "DV_MULTIMEDIA" | "DV_URI" | "DV_EHR_URI" => {
            vec![text_input(primitive_under(co, "value"), None)]
        }
        "DV_CODED_TEXT" | "DV_STATE" | "CODE_PHRASE" => coded_text_inputs(co, labels),
        "DV_QUANTITY" => quantity_inputs(co, labels),
        "DV_COUNT" => vec![count_input(co)],
        "DV_PROPORTION" => proportion_inputs(co, &mut proportion_types),
        "DV_ORDINAL" => vec![ordinal_input(co, labels, false)],
        "DV_SCALE" => vec![ordinal_input(co, labels, true)],
        "DV_BOOLEAN" => vec![boolean_input(co)],
        "DV_DATE" => vec![temporal_input(co, WebTemplateInputType::Date)],
        "DV_DATE_TIME" => vec![temporal_input(co, WebTemplateInputType::Datetime)],
        "DV_TIME" => vec![temporal_input(co, WebTemplateInputType::Time)],
        "DV_DURATION" => duration_inputs(co),
        "DV_IDENTIFIER" => ["id", "type", "issuer", "assigner"]
            .into_iter()
            .map(|s| text_input(primitive_under(co, s), Some(s)))
            .collect(),
        "DV_PARSABLE" => ["value", "formalism"]
            .into_iter()
            .map(|s| text_input(primitive_under(co, s), Some(s)))
            .collect(),
        "PARTY_PROXY" | "PARTY_IDENTIFIED" => ["id", "id_scheme", "id_namespace", "name"]
            .into_iter()
            .map(|s| text_input(primitive_under(co, s), Some(s)))
            .collect(),
        _ => Vec::new(),
    };
    (inputs, proportion_types)
}

// ── DV_TEXT / string ─────────────────────────────────────────────────────────

fn text_input(cstring: Option<&CPrimitive>, suffix: Option<&str>) -> WebTemplateInput {
    let mut input = WebTemplateInput::new(WebTemplateInputType::Text, suffix);
    if let Some(CPrimitive::CString(cs)) = cstring {
        if let Some(pattern) = &cs.pattern {
            input.validation = Some(WebTemplateValidation {
                pattern: Some(pattern.clone()),
                ..Default::default()
            });
        }
        for item in &cs.list {
            input
                .list
                .push(WebTemplateCodedValue::new(item, Some(item.clone())));
        }
        input.list_open = Some(cs.list_open == Some(true));
    }
    input
}

// ── DV_CODED_TEXT ────────────────────────────────────────────────────────────

fn coded_text_inputs(co: &CObject, labels: &dyn Labels) -> Vec<WebTemplateInput> {
    // The node's own CObject may be a CODE_PHRASE, else look under `defining_code`.
    let code_phrases: Vec<&CObject> = match co {
        CObject::CCodePhrase(_) | CObject::CCodeReference(_) => vec![co],
        _ => code_phrases_under(co, "defining_code"),
    };

    let mut inputs = Vec::new();
    match code_phrases.first() {
        Some(CObject::CCodeReference(r)) => {
            let uri = reference_set_uri(&r.referenceSetUri);
            inputs.extend(external_terminology_inputs(uri.as_deref()));
        }
        Some(CObject::CCodePhrase(cp)) => {
            let terminology = cp.terminology_id.as_ref().map(|t| t.value.clone());
            let codes = coded_values(terminology.as_deref(), &cp.code_list, labels);
            if codes.is_empty() {
                inputs.extend(external_terminology_inputs(terminology.as_deref()));
            } else {
                let mut input =
                    WebTemplateInput::new(WebTemplateInputType::CodedText, Some("code"));
                input.list = codes;
                if let Some(term) = &terminology
                    && !term.is_empty()
                    && term != "local"
                {
                    input.terminology = Some(term.clone());
                }
                inputs.push(input);
            }
        }
        // No `defining_code` constraint (or some other object): free-text coded
        // input pair (Better `addExternalTerminologyInputs(null)`).
        None | Some(_) => inputs.extend(external_terminology_inputs(None)),
    }

    // A `value` C_STRING with listOpen opens every input's list.
    if let Some(CPrimitive::CString(cs)) = primitive_under(co, "value")
        && cs.list_open == Some(true)
    {
        for input in &mut inputs {
            input.list_open = Some(true);
        }
    }
    inputs
}

fn external_terminology_inputs(terminology: Option<&str>) -> Vec<WebTemplateInput> {
    let mut code = WebTemplateInput::new(WebTemplateInputType::Text, Some("code"));
    code.terminology = terminology.map(str::to_owned);
    let mut value = WebTemplateInput::new(WebTemplateInputType::Text, Some("value"));
    value.terminology = terminology.map(str::to_owned);
    vec![code, value]
}

fn reference_set_uri(uri: &str) -> Option<String> {
    if uri.is_empty() {
        None
    } else if let Some(rest) = uri.strip_prefix("terminology:") {
        Some(rest.to_owned())
    } else {
        Some(uri.to_owned())
    }
}

fn coded_values(
    terminology: Option<&str>,
    codes: &[String],
    labels: &dyn Labels,
) -> Vec<WebTemplateCodedValue> {
    let term = terminology.unwrap_or("local");
    codes
        .iter()
        .map(|code| {
            let mut cv = coded_value(term, code, labels);
            // Per-coded-value external term bindings (Better
            // `CodePhraseWebTemplateInputBuilder.ConvertToWebTemplateCodedValueFunction`):
            // the coded-text path adds them; ordinals/scales do not.
            cv.term_bindings = labels.term_bindings(code);
            cv
        })
        .collect()
}

fn coded_value(terminology: &str, code: &str, labels: &dyn Labels) -> WebTemplateCodedValue {
    let label = labels
        .text(terminology, code)
        .unwrap_or_else(|| code.to_owned());
    let mut cv = WebTemplateCodedValue::new(code, Some(label));
    cv.localized_labels = labels.localized(terminology, code);
    cv
}

// ── DV_QUANTITY ──────────────────────────────────────────────────────────────

fn quantity_inputs(co: &CObject, labels: &dyn Labels) -> Vec<WebTemplateInput> {
    let mut magnitude = WebTemplateInput::new(WebTemplateInputType::Decimal, Some("magnitude"));
    if let CObject::CDvQuantity(q) = co {
        let mut units = WebTemplateInput::new(WebTemplateInputType::CodedText, Some("unit"));
        for item in &q.list {
            let mut value = WebTemplateCodedValue::new(&item.units, Some(item.units.clone()));
            let mut validation = WebTemplateValidation::default();
            if let Some(p) = item.precision.as_ref().and_then(int_range) {
                validation.precision = Some(p);
            }
            if let Some(r) = item.magnitude.as_ref().and_then(decimal_range) {
                validation.range = Some(r);
            }
            if !validation.is_empty() {
                value.validation = Some(validation);
            }
            value.localized_labels = labels.localized("local", &item.units);
            units.list.push(value);
        }
        // A single allowed unit promotes its range/precision onto the magnitude.
        if units.list.len() == 1 {
            magnitude.validation.clone_from(&units.list[0].validation);
        }
        vec![magnitude, units]
    } else {
        let units = WebTemplateInput::new(WebTemplateInputType::Text, Some("unit"));
        vec![magnitude, units]
    }
}

// ── DV_COUNT ─────────────────────────────────────────────────────────────────

fn count_input(co: &CObject) -> WebTemplateInput {
    let mut input = WebTemplateInput::new(WebTemplateInputType::Integer, None);
    if let Some(CPrimitive::CInteger(ci)) = primitive_under(co, "magnitude")
        && let Some(range) = ci.range.as_ref().and_then(int_range)
    {
        input.validation = Some(WebTemplateValidation {
            range: Some(range),
            ..Default::default()
        });
    }
    input
}

// ── DV_ORDINAL / DV_SCALE ────────────────────────────────────────────────────

fn ordinal_input(co: &CObject, labels: &dyn Labels, scale: bool) -> WebTemplateInput {
    let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, None);
    if let CObject::CDvOrdinal(ord) = co {
        for entry in &ord.list {
            let dc = &entry.symbol.defining_code;
            let term = &dc.terminology_id.value;
            let mut cv = coded_value(term, &dc.code_string, labels);
            // The label should be the ordinal symbol rubric; prefer the symbol value.
            if !entry.symbol.value.is_empty() {
                cv.label = Some(entry.symbol.value.clone());
            }
            if scale {
                cv.scale = Some(f64::from(entry.value));
            } else {
                cv.ordinal = Some(entry.value);
            }
            input.list.push(cv);
        }
    }
    input
}

// ── DV_BOOLEAN ───────────────────────────────────────────────────────────────

fn boolean_input(co: &CObject) -> WebTemplateInput {
    let mut input = WebTemplateInput::new(WebTemplateInputType::Boolean, None);
    if let Some(CPrimitive::CBoolean(cb)) = primitive_under(co, "value") {
        if cb.false_valid && !cb.true_valid {
            input.list.push(WebTemplateCodedValue::new(
                "false",
                Some("false".to_owned()),
            ));
        } else if cb.true_valid && !cb.false_valid {
            input
                .list
                .push(WebTemplateCodedValue::new("true", Some("true".to_owned())));
        }
    }
    input
}

// ── temporal (DATE / DATETIME / TIME) ────────────────────────────────────────

fn temporal_input(co: &CObject, ty: WebTemplateInputType) -> WebTemplateInput {
    let mut input = WebTemplateInput::new(ty, None);
    let (pattern, range) = match primitive_under(co, "value") {
        Some(CPrimitive::CDate(c)) => (
            c.pattern.clone(),
            temporal_range(c.range.as_ref().map(iv_bounds_date)),
        ),
        Some(CPrimitive::CDateTime(c)) => (
            c.pattern.clone(),
            temporal_range(c.range.as_ref().map(iv_bounds_datetime)),
        ),
        Some(CPrimitive::CTime(c)) => (
            c.pattern.clone(),
            temporal_range(c.range.as_ref().map(iv_bounds_time)),
        ),
        _ => (None, None),
    };
    if pattern.is_some() || range.is_some() {
        input.validation = Some(WebTemplateValidation {
            pattern,
            range,
            ..Default::default()
        });
    }
    input
}

// ── DV_DURATION ──────────────────────────────────────────────────────────────

/// Better `FULL_DURATION` order (note DAY precedes WEEK).
const DURATION_FIELDS: [(&str, char); 7] = [
    ("year", 'Y'),
    ("month", 'M'),
    ("day", 'D'),
    ("week", 'W'),
    ("hour", 'H'),
    ("minute", 'm'),
    ("second", 'S'),
];

fn duration_inputs(co: &CObject) -> Vec<WebTemplateInput> {
    let pattern = match primitive_under(co, "value") {
        Some(CPrimitive::CDuration(d)) => d.pattern.clone(),
        _ => None,
    };
    let allowed = duration_allowed_fields(pattern.as_deref());
    DURATION_FIELDS
        .iter()
        .filter(|(name, _)| allowed.contains(name))
        .map(|(name, _)| WebTemplateInput::new(WebTemplateInputType::Integer, Some(name)))
        .collect()
}

/// The duration fields allowed by an ISO-8601 duration pattern (a `P…` string
/// with `?`/`{n}` occurrence markers); all fields when there is no pattern.
fn duration_allowed_fields(pattern: Option<&str>) -> Vec<&'static str> {
    let Some(pattern) = pattern else {
        return DURATION_FIELDS.iter().map(|(n, _)| *n).collect();
    };
    // Time-part letters follow `T`; `M` before `T` is month, after is minute.
    let mut in_time = false;
    let mut allowed = Vec::new();
    for ch in pattern.chars() {
        match ch {
            'T' => in_time = true,
            'Y' => allowed.push("year"),
            'M' => allowed.push(if in_time { "minute" } else { "month" }),
            'W' => allowed.push("week"),
            'D' => allowed.push("day"),
            'H' => allowed.push("hour"),
            'S' => allowed.push("second"),
            _ => {}
        }
    }
    if allowed.is_empty() {
        DURATION_FIELDS.iter().map(|(n, _)| *n).collect()
    } else {
        allowed
    }
}

// ── DV_PROPORTION ────────────────────────────────────────────────────────────

const PROPORTION_KINDS: [&str; 5] = [
    "ratio",
    "unitary",
    "percent",
    "fraction",
    "integer_fraction",
];

fn proportion_inputs(co: &CObject, proportion_types: &mut Vec<String>) -> Vec<WebTemplateInput> {
    let type_codes: Vec<i32> = match primitive_under(co, "type") {
        Some(CPrimitive::CInteger(ci)) => ci.list.clone(),
        _ => Vec::new(),
    };
    let is_integral = matches!(primitive_under(co, "is_integral"),
        Some(CPrimitive::CBoolean(b)) if b.true_valid && !b.false_valid);

    *proportion_types = if type_codes.is_empty() {
        PROPORTION_KINDS.iter().map(|s| (*s).to_owned()).collect()
    } else {
        type_codes
            .iter()
            .filter_map(|c| PROPORTION_KINDS.get(usize::try_from(*c).unwrap_or(usize::MAX)))
            .map(|s| (*s).to_owned())
            .collect()
    };

    ["numerator", "denominator"]
        .into_iter()
        .map(|suffix| proportion_part(co, suffix, is_integral))
        .collect()
}

fn proportion_part(co: &CObject, suffix: &str, is_integral: bool) -> WebTemplateInput {
    let ty = if is_integral {
        WebTemplateInputType::Integer
    } else {
        WebTemplateInputType::Decimal
    };
    let mut input = WebTemplateInput::new(ty, Some(suffix));
    if let Some(CPrimitive::CReal(cr)) = primitive_under(co, suffix)
        && let Some(range) = cr.range.as_ref().and_then(decimal_range)
    {
        input.validation = Some(WebTemplateValidation {
            range: Some(range),
            ..Default::default()
        });
    }
    input
}

// ── opt14 CObject navigation ─────────────────────────────────────────────────

pub(crate) fn attributes(co: &CObject) -> &[CAttribute] {
    match co {
        CObject::CComplexObject(c) => &c.attributes,
        CObject::CArchetypeRoot(c) => &c.attributes,
        CObject::TComplexObject(c) => &c.attributes,
        _ => &[],
    }
}

pub(crate) fn attr_children<'a>(co: &'a CObject, name: &str) -> impl Iterator<Item = &'a CObject> {
    attributes(co)
        .iter()
        .filter(move |a| attribute_name(a) == name)
        .flat_map(attribute_children)
}

pub(crate) fn attribute_name(a: &CAttribute) -> &str {
    match a {
        CAttribute::CSingleAttribute(s) => &s.rm_attribute_name,
        CAttribute::CMultipleAttribute(m) => &m.rm_attribute_name,
    }
}

pub(crate) fn attribute_children(a: &CAttribute) -> &[CObject] {
    match a {
        CAttribute::CSingleAttribute(s) => &s.children,
        CAttribute::CMultipleAttribute(m) => &m.children,
    }
}

/// The first `C_PRIMITIVE` constrained under the named attribute.
pub(crate) fn primitive_under<'a>(co: &'a CObject, name: &str) -> Option<&'a CPrimitive> {
    attr_children(co, name).find_map(|child| match child {
        CObject::CPrimitiveObject(p) => p.item.as_deref(),
        _ => None,
    })
}

/// The `C_CODE_PHRASE`/`C_CODE_REFERENCE` children under the named attribute.
fn code_phrases_under<'a>(co: &'a CObject, name: &str) -> Vec<&'a CObject> {
    attr_children(co, name)
        .filter(|c| matches!(c, CObject::CCodePhrase(_) | CObject::CCodeReference(_)))
        .collect()
}

// ── range helpers ────────────────────────────────────────────────────────────

fn json_i32(v: i32) -> serde_json::Value {
    serde_json::Value::from(v)
}

fn json_f64(v: f64) -> serde_json::Value {
    serde_json::Number::from_f64(v).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

/// `WebTemplateValidationIntegerRange`: min via lower, max via upper (minus one
/// when the upper is excluded); ops fixed to `>=`/`<=`.
fn int_range(iv: &Intervalofinteger) -> Option<WebTemplateRange> {
    let min = if iv.lower_unbounded { None } else { iv.lower };
    let max = if iv.upper_unbounded {
        None
    } else {
        iv.upper.map(|u| {
            if iv.upper_included == Some(false) {
                u - 1
            } else {
                u
            }
        })
    };
    if min.is_none() && max.is_none() {
        return None;
    }
    Some(WebTemplateRange {
        min_op: min.map(|_| ">=".to_owned()),
        min: min.map(json_i32),
        max_op: max.map(|_| "<=".to_owned()),
        max: max.map(json_i32),
    })
}

/// `WebTemplateDecimalRange`: bounds when not unbounded; ops from inclusivity.
fn decimal_range(iv: &Intervalofreal) -> Option<WebTemplateRange> {
    let min = if iv.lower_unbounded { None } else { iv.lower };
    let max = if iv.upper_unbounded { None } else { iv.upper };
    if min.is_none() && max.is_none() {
        return None;
    }
    let min_op = if iv.lower_included == Some(false) {
        ">"
    } else {
        ">="
    };
    let max_op = if iv.upper_included == Some(false) {
        "<"
    } else {
        "<="
    };
    Some(WebTemplateRange {
        min_op: min.map(|_| min_op.to_owned()),
        min: min.map(json_f64),
        max_op: max.map(|_| max_op.to_owned()),
        max: max.map(json_f64),
    })
}

/// Bounds of a temporal interval as `(min, minOp, max, maxOp)` ISO strings.
type TemporalBounds = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn temporal_range(bounds: Option<TemporalBounds>) -> Option<WebTemplateRange> {
    let (min, min_op, max, max_op) = bounds?;
    if min.is_none() && max.is_none() {
        return None;
    }
    Some(WebTemplateRange {
        min_op,
        min: min.map(serde_json::Value::String),
        max_op,
        max: max.map(serde_json::Value::String),
    })
}

macro_rules! iv_bounds {
    ($fn_name:ident, $ty:ty) => {
        fn $fn_name(iv: &$ty) -> TemporalBounds {
            let min = if iv.lower_unbounded {
                None
            } else {
                iv.lower.clone()
            };
            let max = if iv.upper_unbounded {
                None
            } else {
                iv.upper.clone()
            };
            let min_op = min.as_ref().map(|_| {
                if iv.lower_included == Some(false) {
                    ">"
                } else {
                    ">="
                }
                .to_owned()
            });
            let max_op = max.as_ref().map(|_| {
                if iv.upper_included == Some(false) {
                    "<"
                } else {
                    "<="
                }
                .to_owned()
            });
            (min, min_op, max, max_op)
        }
    };
}

iv_bounds!(iv_bounds_date, openehr_its::opt14::Intervalofdate);
iv_bounds!(iv_bounds_datetime, openehr_its::opt14::Intervalofdatetime);
iv_bounds!(iv_bounds_time, openehr_its::opt14::Intervaloftime);
