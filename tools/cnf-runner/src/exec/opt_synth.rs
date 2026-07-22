//! Per-row OPT 1.4 XML synthesis for the *value* and *interval* content
//! families (issue #228). A content decision table whose
//! `constraint_context.constraint_columns` name constraint-axis columns needs
//! one OPT per row (the ELEMENT.value domain constraint varies per row). This
//! module builds those OPTs from the row's constraint cells; the structural
//! families live in [`crate::exec::content_synth`], which dispatches value +
//! interval rows here.
//!
//! The constraint XML shapes are grounded in AM AOM1.4
//! (`docs/specs/openehr/AM/docs/`): `C_PRIMITIVE_OBJECT`/`C_INTEGER`/`C_REAL`/
//! `C_STRING`/`C_DATE`/`C_TIME`/`C_DATE_TIME`/`C_DURATION`, `C_CODE_PHRASE`,
//! `CONSTRAINT_REF`, `C_DV_ORDINAL`, and the temporal `<pattern>` grammar of
//! `AM ADL1.4 master05-cadl §Patterns` + `§Duration Constraints`. The carrier
//! skeleton mirrors the committed Python reference
//! `corpus/templates/generate_content_opts.py` (built on the vendored CNF Robot
//! `minimal_observation.opt`).
//!
//! NOTE: no openEHR spec governs the corpus template packaging — our own
//! corpus-authoring design; the constraint SHAPES are the AOM1.4 ones cited.
//! NOTE: the `timezone_validity` columns are emitted as the Archetype.xsd
//! `<timezone_validity>` element (`VALIDITY_KIND` 1001/1003 — the ITS-XML
//! 1.0.2 wire serializes exactly this one validity axis); the
//! `millisecond_validity` columns and the `C_DURATION` seconds-vs-fractional
//! distinction have NO wire form (no XSD element; the ADL1.4 pattern ends at
//! the seconds slot) — rows whose expected rejection rests solely on those
//! axes are gated per-row N/A upstream (AMB-42,
//! [`crate::exec::content_synth::unrealizable_row`]).

use serde_json::Value;

use core::fmt::Write as _;

use crate::model::case::MatrixCell;

/// A synthesis defect (the row's constraint columns had a shape this module
/// does not cover) — an interpreter defect, never a conformance outcome.
#[derive(Debug, thiserror::Error)]
pub enum SynthError {
    /// The `rm_class` / column shape is not covered by this module.
    #[error("opt_synth: {0}")]
    Unsupported(String),
}

/// One decision-table row bound to its columns.
struct Row<'a> {
    columns: &'a [String],
    cells: &'a [MatrixCell],
}

impl Row<'_> {
    fn cell(&self, name: &str) -> Option<&MatrixCell> {
        self.columns
            .iter()
            .position(|c| c == name)
            .and_then(|i| self.cells.get(i))
    }

    /// The literal text of a column; `None` for null/absent/provided.
    fn text(&self, name: &str) -> Option<&str> {
        match self.cell(name) {
            Some(MatrixCell::Literal(Value::String(s))) => Some(s.as_str()),
            _ => None,
        }
    }

    /// The literal JSON of a column; `None` for null/absent/provided.
    fn literal(&self, name: &str) -> Option<&Value> {
        match self.cell(name) {
            Some(MatrixCell::Literal(v)) => Some(v),
            _ => None,
        }
    }

    fn has(&self, name: &str) -> bool {
        self.columns.iter().any(|c| c == name)
    }
}

/// Synthesize the OPT 1.4 XML for one value/interval content row.
///
/// # Errors
/// [`SynthError::Unsupported`] when the `rm_class` / column shape is not covered.
pub fn synthesize_value_opt(
    case_id: &str,
    rm_class: &str,
    template_id: &str,
    columns: &[String],
    cells: &[MatrixCell],
) -> Result<String, SynthError> {
    let row = Row { columns, cells };
    let (value_children, extra_terms) = match rm_class {
        "DV_DATE" => (build_date(&row), Vec::new()),
        "DV_TIME" => (build_time(&row), Vec::new()),
        "DV_DATE_TIME" => (build_date_time(&row), Vec::new()),
        "DV_DURATION" => (build_duration(&row), Vec::new()),
        "DV_TEXT" | "DV_URI" | "DV_EHR_URI" => (build_string(rm_class, &row, "value"), Vec::new()),
        "DV_CODED_TEXT" => (build_coded_text(&row), Vec::new()),
        "DV_IDENTIFIER" => (build_identifier(&row), Vec::new()),
        "DV_PARSABLE" => (build_parsable(&row), Vec::new()),
        "DV_MULTIMEDIA" => (build_multimedia(&row), Vec::new()),
        "DV_INTERVAL" => build_interval(case_id, &row),
        other => {
            return Err(SynthError::Unsupported(format!(
                "no value synthesizer for rm_class {other}"
            )));
        }
    };
    Ok(value_template(template_id, &value_children, &extra_terms))
}

// ---------------------------------------------------------------------------
// Low-level cADL/OPT XML builders (faithful port of generate_content_opts.py).
// ---------------------------------------------------------------------------

fn xesc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn det_uid(template_id: &str) -> String {
    const NS: uuid::Uuid = uuid::Uuid::from_bytes([
        0x6f, 0x96, 0x19, 0xff, 0x8b, 0x86, 0xd0, 0x11, 0xb4, 0x2d, 0x00, 0xcf, 0x4f, 0xc9, 0x64,
        0xff,
    ]);
    uuid::Uuid::new_v5(&NS, template_id.as_bytes()).to_string()
}

/// `IntervalOfInteger` body (`None` bound => unbounded that side).
fn int_interval(lower: Option<i64>, upper: Option<i64>) -> String {
    let lu = if lower.is_none() { "true" } else { "false" };
    let uu = if upper.is_none() { "true" } else { "false" };
    let mut s = format!(
        "<lower_included>true</lower_included><upper_included>true</upper_included>\
         <lower_unbounded>{lu}</lower_unbounded><upper_unbounded>{uu}</upper_unbounded>"
    );
    if let Some(l) = lower {
        let _ = write!(s, "<lower>{l}</lower>");
    }
    if let Some(u) = upper {
        let _ = write!(s, "<upper>{u}</upper>");
    }
    s
}

fn occ(lower: i64, upper: Option<i64>) -> String {
    format!(
        "<occurrences>{}</occurrences>",
        int_interval(Some(lower), upper)
    )
}

fn existence(lower: i64, upper: i64) -> String {
    format!(
        "<existence>{}</existence>",
        int_interval(Some(lower), Some(upper))
    )
}

fn c_single_attr(name: &str, child: &str, exist: (i64, i64)) -> String {
    format!(
        "<attributes xsi:type=\"C_SINGLE_ATTRIBUTE\"><rm_attribute_name>{name}</rm_attribute_name>{}{child}</attributes>",
        existence(exist.0, exist.1)
    )
}

fn c_multiple_attr(name: &str, children: &str, card: &str, exist: (i64, i64)) -> String {
    format!(
        "<attributes xsi:type=\"C_MULTIPLE_ATTRIBUTE\"><rm_attribute_name>{name}</rm_attribute_name>{}{children}{card}</attributes>",
        existence(exist.0, exist.1)
    )
}

fn cardinality_any() -> String {
    "<cardinality><is_ordered>false</is_ordered><is_unique>false</is_unique><interval>\
     <lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>\
     <upper_unbounded>true</upper_unbounded><lower>0</lower></interval></cardinality>"
        .to_owned()
}

fn c_complex(rm_type: &str, attrs: &str) -> String {
    format!(
        "<children xsi:type=\"C_COMPLEX_OBJECT\"><rm_type_name>{}</rm_type_name>{}<node_id />{attrs}</children>",
        xesc(rm_type),
        occ(1, Some(1))
    )
}

fn c_primitive(prim_rm_type: &str, item: &str) -> String {
    format!(
        "<children xsi:type=\"C_PRIMITIVE_OBJECT\"><rm_type_name>{prim_rm_type}</rm_type_name>{}<node_id />{item}</children>",
        occ(1, Some(1))
    )
}

fn item_c_string_pattern(pattern: &str) -> String {
    format!(
        "<item xsi:type=\"C_STRING\"><pattern>{}</pattern></item>",
        xesc(pattern)
    )
}

fn item_c_string_list(vals: &[String]) -> String {
    let mut s = String::from("<item xsi:type=\"C_STRING\">");
    for v in vals {
        let _ = write!(s, "<list>{}</list>", xesc(v));
    }
    s.push_str("<list_open>false</list_open></item>");
    s
}

fn item_c_integer_list(vals: &[i64]) -> String {
    let mut s = String::from("<item xsi:type=\"C_INTEGER\">");
    for v in vals {
        let _ = write!(s, "<list>{v}</list>");
    }
    s.push_str("</item>");
    s
}

fn item_c_integer_range(lo: i64, hi: i64) -> String {
    format!(
        "<item xsi:type=\"C_INTEGER\"><range>{}</range></item>",
        int_interval(Some(lo), Some(hi))
    )
}

fn item_c_date(pattern: &str) -> String {
    format!("<item xsi:type=\"C_DATE\"><pattern>{pattern}</pattern></item>")
}

/// `mandatory|optional|prohibited` -> the ITS-XML `VALIDITY_KIND` code
/// (Archetype.xsd: 1001 mandatory, 1002 optional, 1003 disallowed); `None`
/// when the column is absent/optional (the XSD element is 0..1 and absence
/// means optional).
fn validity_kind_code(token: Option<&str>) -> Option<&'static str> {
    match token {
        Some("mandatory") => Some("1001"),
        Some("prohibited") => Some("1003"),
        _ => None,
    }
}

/// `<timezone_validity>` element (the ONE validity the ITS-XML 1.0.2
/// `Archetype.xsd` serializes on `C_TIME`/`C_DATE_TIME` — see AMB-42 for the
/// unserializable millisecond axis).
fn timezone_validity_elem(token: Option<&str>) -> String {
    validity_kind_code(token)
        .map(|code| format!("<timezone_validity>{code}</timezone_validity>"))
        .unwrap_or_default()
}

fn item_c_time_tz(pattern: &str, tz: Option<&str>) -> String {
    format!(
        "<item xsi:type=\"C_TIME\"><pattern>{pattern}</pattern>{}</item>",
        timezone_validity_elem(tz)
    )
}

fn item_c_date_time_tz(pattern: &str, tz: Option<&str>) -> String {
    format!(
        "<item xsi:type=\"C_DATE_TIME\"><pattern>{pattern}</pattern>{}</item>",
        timezone_validity_elem(tz)
    )
}

fn item_c_duration(pattern: &str) -> String {
    format!("<item xsi:type=\"C_DURATION\"><pattern>{pattern}</pattern></item>")
}

/// Interval of a temporal/duration literal (string bounds), both included.
fn literal_interval(lo: &str, hi: &str) -> String {
    literal_interval_opt(Some(lo), Some(hi))
}

/// Interval of a temporal/duration literal with optionally unbounded sides
/// (`None` bound => that side unbounded, included flag omitted per the
/// ITS-XML base-types interval shape).
fn literal_interval_opt(lo: Option<&str>, hi: Option<&str>) -> String {
    let mut s = String::new();
    if lo.is_some() {
        s.push_str("<lower_included>true</lower_included>");
    }
    if hi.is_some() {
        s.push_str("<upper_included>true</upper_included>");
    }
    let _ = write!(
        s,
        "<lower_unbounded>{}</lower_unbounded><upper_unbounded>{}</upper_unbounded>",
        lo.is_none(),
        hi.is_none()
    );
    if let Some(lo) = lo {
        let _ = write!(s, "<lower>{}</lower>", xesc(lo));
    }
    if let Some(hi) = hi {
        let _ = write!(s, "<upper>{}</upper>", xesc(hi));
    }
    s
}

fn item_c_date_range(lo: Option<&str>, hi: Option<&str>) -> String {
    format!(
        "<item xsi:type=\"C_DATE\"><range>{}</range></item>",
        literal_interval_opt(lo, hi)
    )
}

fn item_c_time_range(lo: Option<&str>, hi: Option<&str>) -> String {
    format!(
        "<item xsi:type=\"C_TIME\"><range>{}</range></item>",
        literal_interval_opt(lo, hi)
    )
}

fn item_c_date_time_range(lo: Option<&str>, hi: Option<&str>) -> String {
    format!(
        "<item xsi:type=\"C_DATE_TIME\"><range>{}</range></item>",
        literal_interval_opt(lo, hi)
    )
}

fn real_interval(lo: f64, hi: f64) -> String {
    format!(
        "<lower_included>true</lower_included><upper_included>true</upper_included>\
         <lower_unbounded>false</lower_unbounded><upper_unbounded>false</upper_unbounded>\
         <lower>{lo}</lower><upper>{hi}</upper>"
    )
}

fn c_code_phrase(term: &str, codes: &[String]) -> String {
    let mut cl = String::new();
    for c in codes {
        let _ = write!(cl, "<code_list>{}</code_list>", xesc(c));
    }
    format!(
        "<children xsi:type=\"C_CODE_PHRASE\"><rm_type_name>CODE_PHRASE</rm_type_name>{}<node_id /><terminology_id><value>{}</value></terminology_id>{cl}</children>",
        occ(1, Some(1)),
        xesc(term)
    )
}

fn constraint_ref(reference: &str) -> String {
    format!(
        "<children xsi:type=\"CONSTRAINT_REF\"><rm_type_name>CODE_PHRASE</rm_type_name>{}<node_id /><reference>{}</reference></children>",
        occ(1, Some(1)),
        xesc(reference)
    )
}

// ---------------------------------------------------------------------------
// Value builders (each returns the ELEMENT.value <children> block).
// ---------------------------------------------------------------------------

fn dv_leaf(rm_type: &str, field: &str, prim: &str, item: Option<&str>) -> String {
    match item {
        None => c_complex(rm_type, ""),
        Some(item) => c_complex(
            rm_type,
            &c_single_attr(field, &c_primitive(prim, item), (1, 1)),
        ),
    }
}

/// `mandatory|optional|prohibited` -> the pattern component token.
fn date_component(token: Option<&str>, mandatory: &str) -> String {
    match token {
        Some("mandatory") => mandatory.to_owned(),
        Some("prohibited") => "X".repeat(mandatory.len()),
        // optional / null / unknown -> optional
        _ => "?".repeat(mandatory.len()),
    }
}

fn build_date(row: &Row<'_>) -> String {
    if row.has("range.lower") || row.has("range.upper") {
        let lo = row.text("range.lower");
        let hi = row.text("range.upper");
        // A half-open range is a real constraint: the absent side is
        // unbounded, never "no constraint".
        return dv_leaf("DV_DATE", "value", "DATE", Some(&item_c_date_range(lo, hi)));
    }
    let month = date_component(row.text("month_validity"), "mm");
    let day = date_component(row.text("day_validity"), "dd");
    let pattern = format!("yyyy-{month}-{day}");
    dv_leaf("DV_DATE", "value", "DATE", Some(&item_c_date(&pattern)))
}

fn build_time(row: &Row<'_>) -> String {
    if row.has("range.lower") || row.has("range.upper") {
        let lo = row.text("range.lower");
        let hi = row.text("range.upper");
        return dv_leaf("DV_TIME", "value", "TIME", Some(&item_c_time_range(lo, hi)));
    }
    // AOM1.4 pattern base is HH; minute/second from validity. timezone is
    // the Archetype.xsd `timezone_validity` element; millisecond is
    // unserializable on this wire (AMB-42) and gated per-row upstream.
    let min = date_component(row.text("minute_validity"), "MM");
    let sec = date_component(row.text("second_validity"), "SS");
    let pattern = format!("HH:{min}:{sec}");
    dv_leaf(
        "DV_TIME",
        "value",
        "TIME",
        Some(&item_c_time_tz(&pattern, row.text("timezone_validity"))),
    )
}

fn build_date_time(row: &Row<'_>) -> String {
    if row.has("range.lower") || row.has("range.upper") {
        let lo = row.text("range.lower");
        let hi = row.text("range.upper");
        return dv_leaf(
            "DV_DATE_TIME",
            "value",
            "DATE_TIME",
            Some(&item_c_date_time_range(lo, hi)),
        );
    }
    let month = date_component(row.text("month_validity"), "mm");
    let day = date_component(row.text("day_validity"), "dd");
    let hour = date_component(row.text("hour_validity"), "HH");
    let min = date_component(row.text("minute_validity"), "MM");
    let sec = date_component(row.text("second_validity"), "SS");
    let pattern = format!("yyyy-{month}-{day}T{hour}:{min}:{sec}");
    dv_leaf(
        "DV_DATE_TIME",
        "value",
        "DATE_TIME",
        Some(&item_c_date_time_tz(
            &pattern,
            row.text("timezone_validity"),
        )),
    )
}

/// Duration pattern letters for the allowed slots (AM ADL1.4 §Duration
/// Constraints): P Y M W D then T H M S; T only when a time slot is allowed.
fn duration_pattern(row: &Row<'_>, suffix: &str) -> String {
    let allowed = |field: &str| -> bool {
        matches!(
            row.literal(&format!("{field}{suffix}")),
            Some(Value::Bool(true))
        ) || matches!(row.text(&format!("{field}{suffix}")), Some("true"))
    };
    let mut p = String::from("P");
    if allowed("years_allowed") {
        p.push('Y');
    }
    if allowed("months_allowed") {
        p.push('M');
    }
    if allowed("weeks_allowed") {
        p.push('W');
    }
    if allowed("days_allowed") {
        p.push('D');
    }
    let time = allowed("hours_allowed")
        || allowed("minutes_allowed")
        || allowed("seconds_allowed")
        || allowed("fractional_seconds_allowed");
    if time {
        p.push('T');
    }
    if allowed("hours_allowed") {
        p.push('H');
    }
    if allowed("minutes_allowed") {
        p.push('M');
    }
    if allowed("seconds_allowed") || allowed("fractional_seconds_allowed") {
        p.push('S');
    }
    p
}

fn build_duration(row: &Row<'_>) -> String {
    let has_fields = row.has("years_allowed");
    let has_range = row.has("range.lower") || row.has("range.upper");
    // Pattern from the field-allowed flags (default: all slots allowed).
    let pattern = if has_fields {
        duration_pattern(row, "")
    } else {
        "PYMWDTHMS".to_owned()
    };
    let mut item = format!("<item xsi:type=\"C_DURATION\"><pattern>{pattern}</pattern>");
    if has_range && let (Some(lo), Some(hi)) = (row.text("range.lower"), row.text("range.upper")) {
        let _ = write!(item, "<range>{}</range>", literal_interval(lo, hi));
    }
    item.push_str("</item>");
    dv_leaf("DV_DURATION", "value", "DURATION", Some(&item))
}

/// Parse a list cell authored as `[a, b, c]` or a JSON array or a single value.
fn parse_list(cell: Option<&Value>) -> Vec<String> {
    match cell {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(ToOwned::to_owned))
            .collect(),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            let inner = trimmed
                .strip_prefix('[')
                .and_then(|t| t.strip_suffix(']'))
                .unwrap_or(trimmed);
            inner
                .split(',')
                .map(|p| p.trim().to_owned())
                .filter(|p| !p.is_empty())
                .collect()
        }
        _ => Vec::new(),
    }
}

fn build_string(rm_class: &str, row: &Row<'_>, field: &str) -> String {
    if let Some(pattern) = row.text("C_STRING.pattern") {
        return dv_leaf(
            rm_class,
            field,
            "STRING",
            Some(&item_c_string_pattern(pattern)),
        );
    }
    let list = parse_list(row.literal("C_STRING.list"));
    if !list.is_empty() {
        return dv_leaf(rm_class, field, "STRING", Some(&item_c_string_list(&list)));
    }
    // Both null -> open (RM invariants only).
    dv_leaf(rm_class, field, "STRING", None)
}

fn build_coded_text(row: &Row<'_>) -> String {
    if let Some(reference) = row.text("CONSTRAINT_REF.reference") {
        return c_complex(
            "DV_CODED_TEXT",
            &c_single_attr("defining_code", &constraint_ref(reference), (1, 1)),
        );
    }
    let codes = parse_list(row.literal("C_CODE_PHRASE.code_list"));
    if !codes.is_empty() {
        let term = row.text("C_CODE_PHRASE.terminology_id").unwrap_or("local");
        return c_complex(
            "DV_CODED_TEXT",
            &c_single_attr("defining_code", &c_code_phrase(term, &codes), (1, 1)),
        );
    }
    // Open coded text (RM mandatory-attribute checks only).
    c_complex("DV_CODED_TEXT", "")
}

fn build_identifier(row: &Row<'_>) -> String {
    // The `attribute` column selects which DV_IDENTIFIER field the C_STRING
    // binds to (issue #228). Default `id` (the mandatory field).
    let field = row.text("attribute").unwrap_or("id");
    if let Some(pattern) = row.text("C_STRING.pattern") {
        return c_complex(
            "DV_IDENTIFIER",
            &c_single_attr(
                field,
                &c_primitive("STRING", &item_c_string_pattern(pattern)),
                (1, 1),
            ),
        );
    }
    let list = parse_list(row.literal("C_STRING.list"));
    if !list.is_empty() {
        return c_complex(
            "DV_IDENTIFIER",
            &c_single_attr(
                field,
                &c_primitive("STRING", &item_c_string_list(&list)),
                (1, 1),
            ),
        );
    }
    c_complex("DV_IDENTIFIER", "")
}

fn build_parsable(row: &Row<'_>) -> String {
    let mut attrs = String::new();
    let field_constraint = |pattern_col: &str, list_col: &str| -> Option<String> {
        if let Some(p) = row.text(pattern_col) {
            Some(item_c_string_pattern(p))
        } else {
            let list = parse_list(row.literal(list_col));
            (!list.is_empty()).then(|| item_c_string_list(&list))
        }
    };
    if let Some(item) = field_constraint("C_STRING.pattern (value)", "C_STRING.list (value)") {
        attrs.push_str(&c_single_attr(
            "value",
            &c_primitive("STRING", &item),
            (1, 1),
        ));
    }
    if let Some(item) =
        field_constraint("C_STRING.pattern (formalism)", "C_STRING.list (formalism)")
    {
        attrs.push_str(&c_single_attr(
            "formalism",
            &c_primitive("STRING", &item),
            (1, 1),
        ));
    }
    c_complex("DV_PARSABLE", &attrs)
}

fn build_multimedia(row: &Row<'_>) -> String {
    let mut attrs = String::new();
    let media = parse_list(row.literal("C_CODE_PHRASE"));
    if !media.is_empty() {
        attrs.push_str(&c_single_attr(
            "media_type",
            &c_code_phrase("IANA_media-types", &media),
            (1, 1),
        ));
    }
    if let Some(range) = row.text("C_INTEGER.range")
        && let Some((lo, hi)) = parse_int_range(range)
    {
        // DV_MULTIMEDIA.size is RM-mandatory (existence 1..1 in the reference
        // model — RM data_types §DV_MULTIMEDIA); the archetype must not relax it.
        attrs.push_str(&c_single_attr(
            "size",
            &c_primitive("INTEGER", &item_c_integer_range(lo, hi)),
            (1, 1),
        ));
    } else {
        let ints = parse_int_list(row.literal("C_INTEGER.list"));
        if !ints.is_empty() {
            attrs.push_str(&c_single_attr(
                "size",
                &c_primitive("INTEGER", &item_c_integer_list(&ints)),
                (1, 1),
            ));
        }
    }
    c_complex("DV_MULTIMEDIA", &attrs)
}

fn parse_int_list(cell: Option<&Value>) -> Vec<i64> {
    parse_list(cell)
        .into_iter()
        .filter_map(|s| s.parse::<i64>().ok())
        .collect()
}

/// Parse a range cell authored as `lo..hi` (or `[lo, hi]`) into integer bounds.
fn parse_int_range(cell: &str) -> Option<(i64, i64)> {
    let (lo, hi) = split_range(cell)?;
    Some((lo.parse().ok()?, hi.parse().ok()?))
}

/// Split a `lo..hi` (or `[lo, hi]`) range cell into its two string bounds.
fn split_range(cell: &str) -> Option<(String, String)> {
    let trimmed = cell.trim().trim_start_matches('[').trim_end_matches(']');
    if let Some((lo, hi)) = trimmed.split_once("..") {
        return Some((lo.trim().to_owned(), hi.trim().to_owned()));
    }
    if let Some((lo, hi)) = trimmed.split_once(',') {
        return Some((lo.trim().to_owned(), hi.trim().to_owned()));
    }
    None
}

// ---------------------------------------------------------------------------
// DV_INTERVAL<T> — inner limit constraints per inner type.
// ---------------------------------------------------------------------------

/// The inner RM type of a `DV_INTERVAL<T>` case, from its id.
fn interval_inner(case_id: &str) -> &'static str {
    let id = case_id.to_ascii_lowercase();
    if id.contains("date_time") {
        "DV_DATE_TIME"
    } else if id.contains("_date") {
        "DV_DATE"
    } else if id.contains("time") {
        "DV_TIME"
    } else if id.contains("duration") {
        "DV_DURATION"
    } else if id.contains("quantity") {
        "DV_QUANTITY"
    } else if id.contains("ordinal") {
        "DV_ORDINAL"
    } else if id.contains("scale") {
        "DV_SCALE"
    } else if id.contains("proportion") {
        "DV_PROPORTION"
    } else {
        "DV_COUNT"
    }
}

/// A `DV_INTERVAL`<inner> value `C_COMPLEX_OBJECT` with optional lower/upper limit
/// object constraints (`None` => that side unconstrained).
fn dv_interval(inner: &str, lower: Option<&str>, upper: Option<&str>) -> String {
    let mut attrs = String::new();
    if let Some(lower) = lower {
        attrs.push_str(&c_single_attr("lower", lower, (0, 1)));
    }
    if let Some(upper) = upper {
        attrs.push_str(&c_single_attr("upper", upper, (0, 1)));
    }
    c_complex(&format!("DV_INTERVAL<{inner}>"), &attrs)
}

/// The `DV_ORDINAL` / `DV_SCALE` list limit object (mirrors Python
/// `dv_ordinal_list` — a two-item mild/severe list) and its term extras.
/// Parse an ordinal/scale list cell — `"[1|[local::at0005], 2.4|[local::at0006]]"`
/// — into (value, terminology, code) triples. Falls back to the fixed
/// mild/severe pair when the cell is absent (the fixed-list corpus shape).
fn parse_ordinal_list_cell(cell: Option<&str>) -> Vec<(String, String, String)> {
    let Some(text) = cell else {
        return vec![
            ("1".to_owned(), "local".to_owned(), "at0005".to_owned()),
            ("2".to_owned(), "local".to_owned(), "at0006".to_owned()),
        ];
    };
    let inner = text
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .unwrap_or(text);
    inner
        .split("], ")
        .filter_map(|entry| {
            let (value, rest) = entry.split_once("|[")?;
            let rest = rest.trim_end_matches(']');
            let (term, code) = rest.split_once("::")?;
            Some((
                value.trim().to_owned(),
                term.trim().to_owned(),
                code.trim().to_owned(),
            ))
        })
        .collect()
}

fn ordinal_list_children(
    inner: &str,
    cell: Option<&str>,
) -> (String, Vec<(String, String, String)>) {
    let entries = parse_ordinal_list_cell(cell);
    if inner == "DV_SCALE" {
        // AOM1.4 has no C_DV_SCALE constrainer (AM masterAppA domain
        // extension defines integer-valued C_ORDINAL only) — express the
        // row's value set generically: a C_REAL list on DV_SCALE.value plus
        // a C_CODE_PHRASE code_list on symbol.defining_code.
        let mut item = String::from("<item xsi:type=\"C_REAL\">");
        for (value, _, _) in &entries {
            let _ = write!(item, "<list>{}</list>", xesc(value));
        }
        item.push_str("</item>");
        let mut attrs = c_single_attr("value", &c_primitive("REAL", &item), (1, 1));
        let terminology = entries
            .first()
            .map_or("local", |(_, term, _)| term.as_str())
            .to_owned();
        let codes: Vec<String> = entries.iter().map(|(_, _, code)| code.clone()).collect();
        attrs.push_str(&c_single_attr(
            "symbol",
            &c_complex(
                "DV_CODED_TEXT",
                &c_single_attr(
                    "defining_code",
                    &c_code_phrase(&terminology, &codes),
                    (1, 1),
                ),
            ),
            // DV_SCALE.symbol is 1..1 in the RM — an OPT existence may
            // narrow but never relax RM existence (VCAEX).
            (1, 1),
        ));
        let terms = entries
            .iter()
            .map(|(value, _, code)| {
                (
                    code.clone(),
                    format!("scale {value}"),
                    format!("scale {value}"),
                )
            })
            .collect();
        return (c_complex("DV_SCALE", &attrs), terms);
    }
    let mut body = String::new();
    for (value, term, code) in &entries {
        let _ = write!(
            body,
            "<list xsi:type=\"DV_ORDINAL\"><value>{}</value><symbol><value>ord {}</value>\
             <defining_code><terminology_id><value>{}</value></terminology_id><code_string>{}</code_string></defining_code></symbol></list>",
            xesc(value),
            xesc(code),
            xesc(term),
            xesc(code)
        );
    }
    let children = format!(
        "<children xsi:type=\"C_DV_ORDINAL\"><rm_type_name>DV_ORDINAL</rm_type_name>{}<node_id />{body}</children>",
        occ(1, Some(1))
    );
    let terms = entries
        .iter()
        .map(|(value, _, code)| (code.clone(), format!("ord {value}"), format!("ord {value}")))
        .collect();
    (children, terms)
}

/// A `DV_PROPORTION` limit with numerator/denominator `C_REAL` ranges (`ratio_range`).
fn proportion_range_limit(num_range: Option<&str>, den_range: Option<&str>) -> String {
    let mut attrs = String::new();
    attrs.push_str(&c_single_attr(
        "type",
        &c_primitive("INTEGER", &item_c_integer_list(&[0])),
        (1, 1),
    ));
    if let Some(nr) = num_range.and_then(split_range)
        && let (Ok(lo), Ok(hi)) = (nr.0.parse::<f64>(), nr.1.parse::<f64>())
    {
        attrs.push_str(&c_single_attr(
            "numerator",
            &c_primitive(
                "REAL",
                &format!(
                    "<item xsi:type=\"C_REAL\"><range>{}</range></item>",
                    real_interval(lo, hi)
                ),
            ),
            (1, 1),
        ));
    }
    if let Some(dr) = den_range.and_then(split_range)
        && let (Ok(lo), Ok(hi)) = (dr.0.parse::<f64>(), dr.1.parse::<f64>())
    {
        attrs.push_str(&c_single_attr(
            "denominator",
            &c_primitive(
                "REAL",
                &format!(
                    "<item xsi:type=\"C_REAL\"><range>{}</range></item>",
                    real_interval(lo, hi)
                ),
            ),
            (1, 1),
        ));
    }
    c_complex("DV_PROPORTION", &attrs)
}

#[allow(clippy::too_many_lines)] // one match arm per DV_INTERVAL inner type
fn build_interval(case_id: &str, row: &Row<'_>) -> (String, Vec<(String, String, String)>) {
    let inner = interval_inner(case_id);
    let mut terms = Vec::new();
    let (lower, upper): (Option<String>, Option<String>) = match inner {
        "DV_DATE" => {
            if row.has("c_date_range_lower") {
                let l = row
                    .text("c_date_range_lower")
                    .and_then(split_range)
                    .map(|(lo, hi)| {
                        dv_leaf(
                            "DV_DATE",
                            "value",
                            "DATE",
                            Some(&item_c_date_range(Some(&lo), Some(&hi))),
                        )
                    });
                let u = row
                    .text("c_date_range_upper")
                    .and_then(split_range)
                    .map(|(lo, hi)| {
                        dv_leaf(
                            "DV_DATE",
                            "value",
                            "DATE",
                            Some(&item_c_date_range(Some(&lo), Some(&hi))),
                        )
                    });
                (l, u)
            } else {
                let l = date_pattern_limit("DV_DATE", "DATE", row, "_lower");
                let u = date_pattern_limit("DV_DATE", "DATE", row, "_upper");
                (l, u)
            }
        }
        "DV_DATE_TIME" => {
            if row.has("c_date_time_range_lower") {
                let l = row
                    .text("c_date_time_range_lower")
                    .and_then(split_range)
                    .map(|(lo, hi)| {
                        dv_leaf(
                            "DV_DATE_TIME",
                            "value",
                            "DATE_TIME",
                            Some(&item_c_date_time_range(Some(&lo), Some(&hi))),
                        )
                    });
                let u = row
                    .text("c_date_time_range_upper")
                    .and_then(split_range)
                    .map(|(lo, hi)| {
                        dv_leaf(
                            "DV_DATE_TIME",
                            "value",
                            "DATE_TIME",
                            Some(&item_c_date_time_range(Some(&lo), Some(&hi))),
                        )
                    });
                (l, u)
            } else {
                let l = date_time_pattern_limit(row, "_lower");
                let u = date_time_pattern_limit(row, "_upper");
                (l, u)
            }
        }
        "DV_TIME" => {
            if row.has("c_time_range_lower") {
                let l = row
                    .text("c_time_range_lower")
                    .and_then(split_range)
                    .map(|(lo, hi)| {
                        dv_leaf(
                            "DV_TIME",
                            "value",
                            "TIME",
                            Some(&item_c_time_range(Some(&lo), Some(&hi))),
                        )
                    });
                let u = row
                    .text("c_time_range_upper")
                    .and_then(split_range)
                    .map(|(lo, hi)| {
                        dv_leaf(
                            "DV_TIME",
                            "value",
                            "TIME",
                            Some(&item_c_time_range(Some(&lo), Some(&hi))),
                        )
                    });
                (l, u)
            } else {
                let l = time_pattern_limit(row, "_lower");
                let u = time_pattern_limit(row, "_upper");
                (l, u)
            }
        }
        "DV_DURATION" => {
            if row.has("range_lower_for_lower") {
                let l = duration_range_limit(row, "range_lower_for_lower", "range_upper_for_lower");
                let u = duration_range_limit(row, "range_lower_for_upper", "range_upper_for_upper");
                (l, u)
            } else if row.has("years_allowed_lower") {
                let l = Some(dv_leaf(
                    "DV_DURATION",
                    "value",
                    "DURATION",
                    Some(&item_c_duration(&duration_pattern(row, "_lower"))),
                ));
                let u = Some(dv_leaf(
                    "DV_DURATION",
                    "value",
                    "DURATION",
                    Some(&item_c_duration(&duration_pattern(row, "_upper"))),
                ));
                (l, u)
            } else {
                (None, None)
            }
        }
        "DV_ORDINAL" | "DV_SCALE" if row.has("lower_c_dv_ordinal_list") => {
            // Each bound carries ITS OWN row-cell value set — never a
            // shared fixed list (the bounds' lists differ per row).
            let lower_cell = row.text("lower_c_dv_ordinal_list");
            let upper_cell = row.text("upper_c_dv_ordinal_list");
            let l = lower_cell.map(|cell| {
                let (children, lt) = ordinal_list_children(inner, Some(cell));
                terms.extend(lt);
                children
            });
            let u = upper_cell.map(|cell| {
                let (children, ut) = ordinal_list_children(inner, Some(cell));
                terms.extend(ut);
                children
            });
            terms.sort();
            terms.dedup();
            (l, u)
        }
        "DV_PROPORTION" if row.has("lower_num_range") => {
            let l =
                proportion_range_limit(row.text("lower_num_range"), row.text("lower_den_range"));
            let u =
                proportion_range_limit(row.text("upper_num_range"), row.text("upper_den_range"));
            (Some(l), Some(u))
        }
        // Open interval (validate_open and any all-null inner constraint set).
        _ => (None, None),
    };
    (
        dv_interval(inner, lower.as_deref(), upper.as_deref()),
        terms,
    )
}

fn date_pattern_limit(rm: &str, prim: &str, row: &Row<'_>, suffix: &str) -> Option<String> {
    if !row.has(&format!("month_validity{suffix}")) {
        return None;
    }
    let month = date_component(row.text(&format!("month_validity{suffix}")), "mm");
    let day = date_component(row.text(&format!("day_validity{suffix}")), "dd");
    Some(dv_leaf(
        rm,
        "value",
        prim,
        Some(&item_c_date(&format!("yyyy-{month}-{day}"))),
    ))
}

fn date_time_pattern_limit(row: &Row<'_>, suffix: &str) -> Option<String> {
    if !row.has(&format!("month_validity{suffix}")) {
        return None;
    }
    let month = date_component(row.text(&format!("month_validity{suffix}")), "mm");
    let day = date_component(row.text(&format!("day_validity{suffix}")), "dd");
    let hour = date_component(row.text(&format!("hour_validity{suffix}")), "HH");
    let min = date_component(row.text(&format!("minute_validity{suffix}")), "MM");
    let sec = date_component(row.text(&format!("second_validity{suffix}")), "SS");
    let pattern = format!("yyyy-{month}-{day}T{hour}:{min}:{sec}");
    Some(dv_leaf(
        "DV_DATE_TIME",
        "value",
        "DATE_TIME",
        Some(&item_c_date_time_tz(
            &pattern,
            row.text(&format!("timezone_validity{suffix}")),
        )),
    ))
}

fn time_pattern_limit(row: &Row<'_>, suffix: &str) -> Option<String> {
    if !row.has(&format!("minute_validity{suffix}")) {
        return None;
    }
    let min = date_component(row.text(&format!("minute_validity{suffix}")), "MM");
    let sec = date_component(row.text(&format!("second_validity{suffix}")), "SS");
    Some(dv_leaf(
        "DV_TIME",
        "value",
        "TIME",
        Some(&item_c_time_tz(
            &format!("HH:{min}:{sec}"),
            row.text(&format!("timezone_validity{suffix}")),
        )),
    ))
}

fn duration_range_limit(row: &Row<'_>, lo_col: &str, hi_col: &str) -> Option<String> {
    let lo = row.text(lo_col)?;
    let hi = row.text(hi_col)?;
    let item = format!(
        "<item xsi:type=\"C_DURATION\"><pattern>PYMWDTHMS</pattern><range>{}</range></item>",
        literal_interval(lo, hi)
    );
    Some(dv_leaf("DV_DURATION", "value", "DURATION", Some(&item)))
}

// ---------------------------------------------------------------------------
// The OBSERVATION carrier assembler (mirrors the Python skeleton exactly).
// ---------------------------------------------------------------------------

fn obs_term_defs(extra: &[(String, String, String)]) -> String {
    let mut base: Vec<(&str, &str, &str)> = vec![
        ("at0000", "Minimal", "unknown"),
        ("at0001", "Event Series", "@ internal @"),
        ("at0002", "Any event", "*"),
        ("at0003", "Tree", "@ internal @"),
        ("at0004", "value", "*"),
    ];
    base.extend(
        extra
            .iter()
            .map(|(code, text, desc)| (code.as_str(), text.as_str(), desc.as_str())),
    );
    let mut out = String::new();
    for (code, text, desc) in base {
        let _ = write!(
            out,
            "<term_definitions code=\"{code}\"><items id=\"description\">{}</items><items id=\"text\">{}</items></term_definitions>",
            xesc(desc),
            xesc(text)
        );
    }
    out
}

fn observation_root(value_children: &str, extra_terms: &[(String, String, String)]) -> String {
    let value_attr = c_single_attr("value", value_children, (0, 1));
    let element = format!(
        "<children xsi:type=\"C_COMPLEX_OBJECT\"><rm_type_name>ELEMENT</rm_type_name>{}<node_id>at0004</node_id>{value_attr}</children>",
        occ(0, Some(1))
    );
    let items = c_multiple_attr("items", &element, &cardinality_any(), (0, 1));
    let item_tree = format!(
        "<children xsi:type=\"C_COMPLEX_OBJECT\"><rm_type_name>ITEM_TREE</rm_type_name>{}<node_id>at0003</node_id>{items}</children>",
        occ(1, Some(1))
    );
    let data_attr = c_single_attr("data", &item_tree, (1, 1));
    let event = format!(
        "<children xsi:type=\"C_COMPLEX_OBJECT\"><rm_type_name>EVENT</rm_type_name>{}<node_id>at0002</node_id>{data_attr}</children>",
        occ(0, Some(1))
    );
    let events = c_multiple_attr("events", &event, &cardinality_min1(), (1, 1));
    let history = format!(
        "<children xsi:type=\"C_COMPLEX_OBJECT\"><rm_type_name>HISTORY</rm_type_name>{}<node_id>at0001</node_id>{events}</children>",
        occ(1, Some(1))
    );
    let hist_data = c_single_attr("data", &history, (1, 1));
    format!(
        "<children xsi:type=\"C_ARCHETYPE_ROOT\"><rm_type_name>OBSERVATION</rm_type_name>\
<occurrences><lower_included>true</lower_included><lower_unbounded>false</lower_unbounded><upper_unbounded>true</upper_unbounded><lower>0</lower></occurrences>\
<node_id>at0000</node_id>{hist_data}<archetype_id><value>openEHR-EHR-OBSERVATION.minimal.v1</value></archetype_id>{}</children>",
        obs_term_defs(extra_terms)
    )
}

fn cardinality_min1() -> String {
    "<cardinality><is_ordered>false</is_ordered><is_unique>false</is_unique><interval>\
     <lower_included>true</lower_included><lower_unbounded>false</lower_unbounded>\
     <upper_unbounded>true</upper_unbounded><lower>1</lower></interval></cardinality>"
        .to_owned()
}

fn value_template(
    template_id: &str,
    value_children: &str,
    extra_terms: &[(String, String, String)],
) -> String {
    let uid = det_uid(template_id);
    let obs = observation_root(value_children, extra_terms);
    let content_attr = c_multiple_attr("content", &obs, &cardinality_any(), (0, 1));
    let category = c_single_attr(
        "category",
        &c_complex(
            "DV_CODED_TEXT",
            &c_single_attr(
                "defining_code",
                &c_code_phrase("openehr", &["433".to_owned()]),
                (1, 1),
            ),
        ),
        (1, 1),
    );
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<template xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" xmlns=\"http://schemas.openehr.org/v1\">\n\
  <language><terminology_id><value>ISO_639-1</value></terminology_id><code_string>en</code_string></language>\n\
  <description><original_author id=\"Original Author\">CNF corpus</original_author><lifecycle_state>Initial</lifecycle_state><details><language><terminology_id><value>ISO_639-1</value></terminology_id><code_string>en</code_string></language><purpose>CNF content constraint template</purpose></details></description>\n\
  <uid><value>{uid}</value></uid>\n\
  <template_id><value>{tid}</value></template_id>\n\
  <concept>CNF content constraint</concept>\n\
  <definition>\n\
    <rm_type_name>COMPOSITION</rm_type_name>\n\
    {occ}\n\
    <node_id>at0000</node_id>\n\
    {category}\n\
    {content_attr}\n\
    <archetype_id><value>openEHR-EHR-COMPOSITION.minimal.v1</value></archetype_id>\n\
    <template_id><value>{tid}</value></template_id>\n\
    <term_definitions code=\"at0000\"><items id=\"description\">unknown</items><items id=\"text\">Minimal</items></term_definitions>\n\
  </definition>\n\
</template>\n",
        tid = xesc(template_id),
        occ = occ(1, Some(1)),
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::needless_pass_by_value
)] // test fixtures
mod tests {
    use super::*;
    use serde_json::json;

    fn cols(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    fn lit(s: &str) -> MatrixCell {
        MatrixCell::Literal(json!(s))
    }

    fn synth(case: &str, rm: &str, c: &[&str], cells: Vec<MatrixCell>) -> String {
        synthesize_value_opt(case, rm, "cnf.tpl.x.r0", &cols(c), &cells).unwrap()
    }

    #[test]
    fn date_constraint_pattern_from_validity() {
        // month mandatory, day prohibited -> yyyy-mm-XX
        let xml = synth(
            "CONT-DV_DATE-validate_constraint",
            "DV_DATE",
            &[
                "value",
                "month_validity",
                "day_validity",
                "expected",
                "violates",
            ],
            vec![
                lit("2021-10"),
                lit("mandatory"),
                lit("prohibited"),
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(xml.contains("<pattern>yyyy-mm-XX</pattern>"), "{xml}");
    }

    #[test]
    fn date_range_emits_bounds() {
        let xml = synth(
            "CONT-DV_DATE-validate_range",
            "DV_DATE",
            &[
                "value",
                "range.lower",
                "range.upper",
                "expected",
                "violates",
            ],
            vec![
                lit("2021"),
                lit("1900"),
                lit("2030"),
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(xml.contains("<lower>1900</lower>"), "{xml}");
        assert!(xml.contains("<upper>2030</upper>"), "{xml}");
        assert!(!xml.contains("<pattern>"), "{xml}");
    }

    #[test]
    fn duration_fields_pattern() {
        // Y,M,W,D allowed; no time -> PYMWD
        let xml = synth(
            "CONT-DV_DURATION-validate_fields",
            "DV_DURATION",
            &[
                "value",
                "years_allowed",
                "months_allowed",
                "weeks_allowed",
                "days_allowed",
                "hours_allowed",
                "minutes_allowed",
                "seconds_allowed",
                "fractional_seconds_allowed",
                "expected",
                "violates",
            ],
            vec![
                lit("P1Y"),
                MatrixCell::Literal(json!(true)),
                MatrixCell::Literal(json!(true)),
                MatrixCell::Literal(json!(true)),
                MatrixCell::Literal(json!(true)),
                MatrixCell::Literal(json!(false)),
                MatrixCell::Literal(json!(false)),
                MatrixCell::Literal(json!(false)),
                MatrixCell::Literal(json!(false)),
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(xml.contains("<pattern>PYMWD</pattern>"), "{xml}");
    }

    #[test]
    fn string_pattern_and_list() {
        let xml = synth(
            "CONT-DV_TEXT-validate_open",
            "DV_TEXT",
            &[
                "value",
                "C_STRING.pattern",
                "C_STRING.list",
                "expected",
                "violates",
            ],
            vec![
                lit("XYZ"),
                lit("XYZ"),
                MatrixCell::Null,
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(xml.contains("<pattern>XYZ</pattern>"), "{xml}");
        assert!(xml.contains("DV_TEXT"), "{xml}");
    }

    #[test]
    fn coded_text_open_when_null() {
        let xml = synth(
            "CONT-DV_CODED_TEXT-validate_open",
            "DV_CODED_TEXT",
            &[
                "code_string",
                "terminology_id",
                "C_CODE_PHRASE.code_list",
                "C_CODE_PHRASE.terminology_id",
                "expected",
                "violates",
            ],
            vec![
                lit("ABC"),
                lit("local"),
                MatrixCell::Null,
                MatrixCell::Null,
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(
            xml.contains("<rm_type_name>DV_CODED_TEXT</rm_type_name>"),
            "{xml}"
        );
        // The ELEMENT.value is an OPEN DV_CODED_TEXT (no defining_code
        // constraint). The only C_CODE_PHRASE in the OPT is the COMPOSITION
        // category's mandatory defining_code — so exactly one, not two.
        assert_eq!(xml.matches("C_CODE_PHRASE").count(), 1, "{xml}");
    }

    #[test]
    fn identifier_binds_named_field() {
        let xml = synth(
            "CONT-DV_IDENTIFIER-validate_all_pattern",
            "DV_IDENTIFIER",
            &[
                "attribute",
                "value",
                "C_STRING.pattern",
                "C_STRING.list",
                "expected",
                "violates",
            ],
            vec![
                lit("issuer"),
                lit("x"),
                lit("[A-Z]+"),
                MatrixCell::Null,
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(
            xml.contains("<rm_attribute_name>issuer</rm_attribute_name>"),
            "{xml}"
        );
        assert!(xml.contains("<pattern>[A-Z]+</pattern>"), "{xml}");
    }

    #[test]
    fn interval_date_range_nested_limits() {
        let xml = synth(
            "CONT-DV_INTERVAL_DV_DATE-validate_lower_upper_range",
            "DV_INTERVAL",
            &[
                "lower",
                "upper",
                "c_date_range_lower",
                "c_date_range_upper",
                "expected",
                "violates",
            ],
            vec![
                lit("2021"),
                lit("2022"),
                lit("1900..2030"),
                lit("1900..2030"),
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(xml.contains("DV_INTERVAL&lt;DV_DATE&gt;"), "{xml}");
        assert!(
            xml.contains("<rm_attribute_name>lower</rm_attribute_name>"),
            "{xml}"
        );
        assert!(xml.contains("<lower>1900</lower>"), "{xml}");
    }

    #[test]
    fn interval_open_has_no_limit_constraints() {
        let xml = synth(
            "CONT-DV_INTERVAL_DV_DATE-validate_open",
            "DV_INTERVAL",
            &[
                "lower",
                "upper",
                "lower_unbounded",
                "upper_unbounded",
                "lower_included",
                "upper_included",
                "expected",
                "violates",
            ],
            vec![
                lit("2021"),
                lit("2022"),
                MatrixCell::Literal(json!(false)),
                MatrixCell::Literal(json!(false)),
                MatrixCell::Literal(json!(true)),
                MatrixCell::Literal(json!(true)),
                lit("accepted"),
                lit("[]"),
            ],
        );
        assert!(xml.contains("DV_INTERVAL&lt;DV_DATE&gt;"), "{xml}");
        assert!(
            !xml.contains("<rm_attribute_name>lower</rm_attribute_name>"),
            "{xml}"
        );
    }

    #[test]
    fn synthesis_is_deterministic_and_uid_stable() {
        let c = [
            "value",
            "month_validity",
            "day_validity",
            "expected",
            "violates",
        ];
        let cells = vec![
            lit("2021"),
            lit("optional"),
            lit("optional"),
            lit("accepted"),
            lit("[]"),
        ];
        let a = synth(
            "CONT-DV_DATE-validate_constraint",
            "DV_DATE",
            &c,
            cells.clone(),
        );
        let b = synth("CONT-DV_DATE-validate_constraint", "DV_DATE", &c, cells);
        assert_eq!(a, b);
        assert_eq!(det_uid("cnf.tpl.x.r0"), det_uid("cnf.tpl.x.r0"));
        assert_ne!(det_uid("a"), det_uid("b"));
    }
}
