//! The ODIN bridge — one home for reading an `openehr_lang::odin` value tree
//! into AOM/BASE types, plus the `master03` lexical decoding both ODIN and cADL
//! share.
//!
//! ODIN is a LANG-component specification
//! (`docs/specs/openehr/LANG/docs/odin/`), parsed by `openehr_lang::odin`; this
//! module is the *reading* layer over that tree: type-cast peeling
//! (`untyped`), scalar/list/map extraction, term-code and UUID conversion, and
//! the canonical-JSON encoding a `C_DEFINED_OBJECT.default_value` needs.
//!
//! Two lexical concerns sit here too, because they are shared by the ODIN
//! sections and the cADL definition rather than owned by either: the
//! string/character literal decoding of `ADL2/master03-file_encoding.adoc`
//! §File Encoding + §Special Character Sequences
//! (`decode_string`/`decode_character` — delimiter stripping over
//! [`openehr_lang::escape`], which owns the escape
//! semantics for ODIN, BEL and cADL alike) and the delimited-regex handling of
//! `AOM2/master04.5` §Class Definitions (`C_STRING`).
//!
//! NOTE: a delimited regex (`/…/`, `^…^`) NEVER passes through the escape
//! decoder — `ADL2/master03-file_encoding.adoc` §Special Character Sequences,
//! final paragraph: backslash patterns in a regular expression "should not be
//! treated as anything other than literal strings, since they are processed by
//! a regular expression parser". Only the optional `;"assumed"` suffix beside a
//! regex is decoded.

use std::collections::BTreeMap;

use openehr_base::base_types::definitions::definitions_impl::LOCAL_TERMINOLOGY_ID;
use openehr_base::prelude::{TerminologyCode, Uuid};
use openehr_lang::odin::{OdinKey, OdinValue};

// ── ODIN tree reading ─────────────────────────────────────────────────────

/// Peel any `(TYPE)` casts off an ODIN value.
///
/// The `(TYPE)` prefix of `LANG/docs/odin/master05-content` §Adding Type
/// Information is dynamic-binding information for the reader ("if the type of
/// an object is not inferrable from the data, it must be indicated in an ODIN
/// document"), never part of the datum. Every accessor below therefore reads
/// straight through it: a section written
/// `details = (Hash<RESOURCE_DESCRIPTION_ITEM,String>) <…>` assembles exactly
/// like the uncast form, instead of silently yielding nothing. The cast itself
/// stays on the tree in [`OdinValue::Typed`] for any caller that wants it.
pub(crate) fn untyped(v: &OdinValue) -> &OdinValue {
    let mut cur = v;
    while let OdinValue::Typed { value, .. } = cur {
        cur = value;
    }
    cur
}

/// The object map an ODIN value carries, reading through any type cast.
pub(crate) fn as_object(v: &OdinValue) -> Option<&indexmap::IndexMap<String, OdinValue>> {
    match untyped(v) {
        OdinValue::Object(m) => Some(m),
        _ => None,
    }
}

/// The keyed-list entries an ODIN value carries, reading through any type cast.
pub(crate) fn as_keyed(v: &OdinValue) -> Option<&[(OdinKey, OdinValue)]> {
    match untyped(v) {
        OdinValue::KeyedList(items) => Some(items),
        _ => None,
    }
}

/// An ODIN keyed-list key as a plain [`String`].
pub(crate) fn key_str(k: &OdinKey) -> String {
    match k {
        OdinKey::String(s) | OdinKey::Date(s) | OdinKey::Time(s) | OdinKey::DateTime(s) => {
            s.clone()
        }
        OdinKey::Integer(i) => i.to_string(),
    }
}

/// A one-word name for an ODIN value's kind, for defect messages.
pub(crate) fn odin_kind(v: &OdinValue) -> &'static str {
    match v {
        OdinValue::Object(_) => "an object",
        OdinValue::KeyedList(_) => "a keyed list",
        OdinValue::List(_) => "a list",
        OdinValue::Typed { .. } => "a typed block",
        OdinValue::PathList(_) => "a path list",
        OdinValue::Empty => "an empty block",
        _ => "a leaf value",
    }
}

/// The scalar string an ODIN leaf carries (or a single-element list's leaf).
pub(crate) fn string_of(v: Option<&OdinValue>) -> Option<String> {
    match untyped(v?) {
        OdinValue::String(s)
        | OdinValue::Date(s)
        | OdinValue::Time(s)
        | OdinValue::DateTime(s)
        | OdinValue::Duration(s)
        | OdinValue::Path(s) => Some(s.clone()),
        OdinValue::Integer(i) => Some(i.to_string()),
        OdinValue::Real(r) => Some(r.to_string()),
        OdinValue::Boolean(b) => Some(b.to_string()),
        OdinValue::Character(c) => Some(c.to_string()),
        OdinValue::Uri(u) => Some(strip_uri_delims(u)),
        OdinValue::List(items) if items.len() == 1 => string_of(items.first()),
        _ => None,
    }
}

/// A list of strings from an ODIN `List` (or a single scalar as a one-element
/// list), dropping the trailing open-list marker.
pub(crate) fn string_list(v: &OdinValue) -> Vec<String> {
    match untyped(v) {
        OdinValue::List(items) => items
            .iter()
            .filter(|x| !matches!(x, OdinValue::ListContinue))
            .filter_map(|x| string_of(Some(x)))
            .collect(),
        other => string_of(Some(other)).into_iter().collect(),
    }
}

/// A `key → String` map from an ODIN keyed list or object of string leaves.
pub(crate) fn string_map(v: &OdinValue) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    match untyped(v) {
        OdinValue::KeyedList(items) => {
            for (k, val) in items {
                if let Some(s) = string_of(Some(val)) {
                    out.insert(key_str(k), s);
                }
            }
        }
        OdinValue::Object(map) => {
            for (k, val) in map {
                if let Some(s) = string_of(Some(val)) {
                    out.insert(k.clone(), s);
                }
            }
        }
        _ => {}
    }
    out
}

/// The `key → String` map at `field` of `obj` (empty if absent).
pub(crate) fn string_map_of(
    obj: &indexmap::IndexMap<String, OdinValue>,
    field: &str,
) -> BTreeMap<String, String> {
    obj.get(field).map(string_map).unwrap_or_default()
}

/// Every string leaf of an object as `key → String` (used to gather an
/// `ARCHETYPE_TERM`'s `other_items` before removing `text`/`description`).
pub(crate) fn term_other_items(
    obj: &indexmap::IndexMap<String, OdinValue>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (k, v) in obj {
        if let Some(s) = string_of(Some(v)) {
            out.insert(k.clone(), s);
        }
    }
    out
}

/// Unwrap the deprecated `items = <…>` wrapper around a term/binding block
/// (`master07.13` §Deprecated Terminology Section Features); returns the inner
/// value, or `v` unchanged if there is no wrapper.
pub(crate) fn unwrap_items(v: &OdinValue) -> &OdinValue {
    let v = untyped(v);
    if let OdinValue::Object(map) = v
        && map.len() == 1
        && let Some(inner) = map.get("items")
    {
        return untyped(inner);
    }
    v
}

/// The URI string a binding value carries, stripped of ODIN `<>` delimiters.
pub(crate) fn uri_string(v: &OdinValue) -> String {
    match untyped(v) {
        OdinValue::Uri(u) => strip_uri_delims(u),
        OdinValue::TermCode(t) => t.clone(),
        OdinValue::PathList(ps) => ps.first().cloned().unwrap_or_default(),
        other => string_of(Some(other)).unwrap_or_default(),
    }
}

/// Strip the ODIN `<…>` URI delimiters.
pub(crate) fn strip_uri_delims(u: &str) -> String {
    u.trim_start_matches('<').trim_end_matches('>').to_owned()
}

/// A `TerminologyCode` from an ODIN term-code leaf (`[ISO_639-1::en]`).
pub(crate) fn term_code_of(v: &OdinValue) -> TerminologyCode {
    match untyped(v) {
        OdinValue::TermCode(code) => parse_term_code(code),
        other => string_of(Some(other)).map_or_else(
            || term_code(LOCAL_TERMINOLOGY_ID, ""),
            |s| parse_term_code(&s),
        ),
    }
}

/// Parse `[terminology::code]` (or `[terminology(version)::code]`) into a
/// [`TerminologyCode`].
pub(crate) fn parse_term_code(raw: &str) -> TerminologyCode {
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    let (terminology_part, code_string) = match inner.split_once("::") {
        Some((t, c)) => (t.to_owned(), c.to_owned()),
        None => (LOCAL_TERMINOLOGY_ID.to_owned(), inner.to_owned()),
    };
    // Optional `(version)` suffix on the terminology id.
    let (terminology_id, terminology_version) = match terminology_part.split_once('(') {
        Some((id, ver)) => (id.to_owned(), Some(ver.trim_end_matches(')').to_owned())),
        None => (terminology_part, None),
    };
    TerminologyCode {
        terminology_id,
        terminology_version,
        code_string,
        uri: None,
    }
}

/// A [`TerminologyCode`] naming `terminology_id` and `code`.
pub(crate) fn term_code(terminology_id: &str, code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: terminology_id.to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// Parse a textual UUID, or `None` if it is not one.
pub(crate) fn parse_uuid(s: &str) -> Option<Uuid> {
    uuid::Uuid::parse_str(s.trim())
        .ok()
        .map(|value| Uuid { value })
}

/// The nil UUID (`00000000-0000-0000-0000-000000000000`).
pub(crate) fn nil_uuid() -> Uuid {
    Uuid {
        value: uuid::Uuid::nil(),
    }
}

/// Whether an ODIN value tree contains an interval anywhere.
///
/// An interval has no canonical-JSON encoding here (see [`odin_to_json`]), so
/// a `_default` carrying one is refused outright rather than silently reduced
/// to `null` — the loss would turn "this default is an interval" into "this
/// node has a null default", which no reader can tell from a real absence.
pub(crate) fn odin_contains_interval(v: &OdinValue) -> bool {
    match v {
        OdinValue::Interval(_) => true,
        OdinValue::List(items) => items.iter().any(odin_contains_interval),
        OdinValue::Object(map) => map.values().any(odin_contains_interval),
        OdinValue::KeyedList(items) => items.iter().any(|(_, val)| odin_contains_interval(val)),
        OdinValue::Typed { value, .. } => odin_contains_interval(value),
        _ => false,
    }
}

/// Convert an [`OdinValue`] to canonical JSON for a
/// `C_DEFINED_OBJECT.default_value`.
///
/// NOTE: `AOM2/master04` types `C_DEFINED_OBJECT.default_value` as an instance
/// of the constrained RM type, and no openEHR spec mandates an intermediate
/// JSON shape for it — the canonical-JSON encoding used here is our own
/// design/extension. An `<>` / `<...>` empty block is a genuine "no value" and
/// maps to `null`.
///
// TODO: encode ODIN interval values (`|0..5|`) as a typed default instead of
// `null` — [`odin_contains_interval`] refuses them at the parse for now, so a
// `_default = <|0..5|>` is an error rather than a silent null.
pub(crate) fn odin_to_json(v: &OdinValue) -> serde_json::Value {
    match v {
        OdinValue::String(s)
        | OdinValue::Date(s)
        | OdinValue::Time(s)
        | OdinValue::DateTime(s)
        | OdinValue::Duration(s)
        | OdinValue::TermCode(s)
        | OdinValue::Uri(s)
        | OdinValue::Path(s) => serde_json::Value::String(s.clone()),
        OdinValue::Integer(i) => serde_json::Value::from(*i),
        OdinValue::Real(r) => serde_json::Value::from(*r),
        OdinValue::Boolean(b) => serde_json::Value::from(*b),
        OdinValue::Character(c) => serde_json::Value::String(c.to_string()),
        OdinValue::Empty | OdinValue::Interval(_) => serde_json::Value::Null,
        OdinValue::ListContinue => serde_json::Value::String("...".to_owned()),
        OdinValue::List(items) => {
            serde_json::Value::Array(items.iter().map(odin_to_json).collect())
        }
        OdinValue::PathList(ps) => serde_json::Value::Array(
            ps.iter()
                .map(|p| serde_json::Value::String(p.clone()))
                .collect(),
        ),
        OdinValue::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), odin_to_json(v)))
                .collect(),
        ),
        OdinValue::KeyedList(items) => serde_json::Value::Object(
            items
                .iter()
                .map(|(k, v)| (key_str(k), odin_to_json(v)))
                .collect(),
        ),
        OdinValue::Typed { rm_type, value } => {
            let mut inner = odin_to_json(value);
            if let serde_json::Value::Object(m) = &mut inner {
                m.insert(
                    "_type".to_owned(),
                    serde_json::Value::String(class_name(rm_type).to_owned()),
                );
            }
            inner
        }
    }
}

/// The class name a (possibly namespaced) ODIN type cast denotes.
///
/// `LANG/docs/odin/master05-content` §Adding Type Information (verbatim in
/// `AM/docs/ADL1.4/master04-dadl` §Adding Type Information) builds a qualified
/// type identifier "by prepending package names, separated by the '.'
/// character" to the type name, so the dotted prefix says which package the
/// class comes from and the class itself is the terminal segment:
/// `org.openehr.rm.ehr.content.ENTRY` and a bare `ENTRY` name the same type.
/// The canonical-JSON `_type` tag carries the class name, so the package path
/// is dropped on the way into JSON; the cast keeps its fully-qualified
/// spelling, as authored, on the ODIN tree.
///
/// A generic head is qualified independently of its parameters
/// (`org.openehr.base.Interval<Quantity>`), so only the text before the first
/// `<` is unqualified.
fn class_name(rm_type: &str) -> &str {
    let head_end = rm_type.find('<').unwrap_or(rm_type.len());
    let Some(head) = rm_type.get(..head_end) else {
        return rm_type;
    };
    match head.rfind('.') {
        Some(dot) => rm_type.get(dot + 1..).unwrap_or(rm_type),
        None => rm_type,
    }
}

// ── `master03` lexical decoding ───────────────────────────────────────────

/// Decode a double-quoted `master03` string literal (delimiters included).
///
/// The escape semantics themselves live in
/// [`openehr_lang::escape`] — one home for ODIN, BEL and
/// cADL, since `ADL2/master03-file_encoding.adoc` §File Encoding + §Special
/// Character Sequences and their verbatim ODIN twin
/// (`LANG/docs/odin/master03-basics.adoc`) define one escape set.
///
/// # Errors
/// [`openehr_lang::escape::EscapeError`] for a `\u` escape that denotes no
/// character. The cADL lexer's own escape check is STRUCTURAL only (4 or 8 hex
/// digits), so this is where such a defect is caught, with the offending
/// literal's span.
pub(crate) fn decode_string(raw: &str) -> Result<String, openehr_lang::escape::EscapeError> {
    openehr_lang::escape::decode_string_literal(raw)
}

/// Decode a single-quoted `CHARACTER` literal (delimiters included) into the
/// one-character string that carries it (`base_lexer.g4` `CHARACTER`).
///
/// # Errors
/// As [`decode_string`]. The lexer admits only the six quoted forms inside a
/// character literal, so no `\u` escape reaches here in practice.
pub(crate) fn decode_character(raw: &str) -> Result<String, openehr_lang::escape::EscapeError> {
    openehr_lang::escape::decode_character_literal(raw)
}

// ── delimited regex (`AOM2/master04.5` §`C_STRING`) ───────────────────────

/// The inner regex of a `/re/` or `^re^` delimited pattern.
pub(crate) fn regex_inner(delimited: &str) -> &str {
    let d = delimited.trim();
    for delimiter in ['/', '^'] {
        if let Some(inner) = d
            .strip_prefix(delimiter)
            .and_then(|rest| rest.strip_suffix(delimiter))
        {
            return inner;
        }
    }
    d
}

/// Backslash-escape every UNESCAPED `/` in a regex body.
///
/// `AOM2/master04.5` §`C_STRING` types `constraint` as "a list of literal
/// strings and / or regular expression strings **delimited by the '/'
/// character**", so the AOM carrier is always the `/…/` form — which makes the
/// `^…^` delimiter a purely lexical alternative that has to be normalised on
/// the way in. The chapter states the two forms' equivalence with its own
/// worked pair (`ADL1.4/master05-cadl.adoc` §Regular Expression L696-702:
/// "If the delimiter character is required in the pattern, it must be quoted
/// with the backslash ('\\') character, or else alternative delimiters can be
/// used … The following two patterns are equivalent: `{/km\\/h|mi\\/h/}` …
/// `{^km/h|mi/h^}`"), so escaping on normalisation is the spec's own mapping
/// and keeps parse → print → parse lossless. An already-escaped `\/` (a
/// slash-delimited source) is left alone, so the transform is idempotent.
pub(crate) fn escape_regex_delimiter(inner: &str) -> String {
    let mut out = String::with_capacity(inner.len());
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => {
                escaped = true;
                out.push(ch);
            }
            '/' => out.push_str("\\/"),
            _ => out.push(ch),
        }
    }
    out
}

/// The single delimited regex a `C_STRING` constraint carries, if any.
pub(crate) fn regex_of(constraint: &[String]) -> Option<&str> {
    match constraint {
        [one] if is_delimited_regex(one) => Some(one),
        _ => None,
    }
}

/// True if a `C_STRING` constraint entry is a single delimited regex (`/re/` or
/// `^re^`) — the `cadl2.g4` `CONTAINED_REGEXP` form the parser stores verbatim.
///
/// A plain string *value* that merely starts with `/` (e.g. a unit `"/min"`) is
/// not a regex — the delimiter must close, so both ends are required. Compared
/// VERBATIM: a leading or trailing space defeats the test (the printer's
/// round-trip depends on that strictness). The whitespace-tolerant sibling is
/// [`is_delimited_regex_trimmed`].
pub(crate) fn is_delimited_regex(s: &str) -> bool {
    (s.len() >= 2 && s.starts_with('/') && s.ends_with('/'))
        || (s.len() >= 2 && s.starts_with('^') && s.ends_with('^'))
}

/// True if a `C_STRING` constraint entry is a delimited regex (`/re/` or `^re^`)
/// rather than a literal string (`master04.5` §`C_STRING`), IGNORING surrounding
/// whitespace.
///
/// The whitespace-strict sibling is [`is_delimited_regex`].
pub(crate) fn is_delimited_regex_trimmed(s: &str) -> bool {
    let t = s.trim();
    t.len() >= 2
        && ((t.starts_with('/') && t.ends_with('/')) || (t.starts_with('^') && t.ends_with('^')))
}
#[cfg(test)]
mod tests {
    use super::{class_name, odin_to_json};

    /// A namespaced ODIN cast names the same class as its bare spelling
    /// (`LANG/docs/odin/master05-content` §Adding Type Information), so the
    /// package path is dropped on the way into the canonical-JSON `_type`
    /// tag while a generic head is unqualified independently of its
    /// parameters.
    #[test]
    fn a_namespaced_cast_reduces_to_its_class_name() {
        assert_eq!(class_name("ENTRY"), "ENTRY");
        assert_eq!(class_name("org.openehr.rm.ehr.content.ENTRY"), "ENTRY");
        assert_eq!(
            class_name("Core.Abstractions.Relationships.Relationship"),
            "Relationship"
        );
        assert_eq!(class_name("Interval<Quantity>"), "Interval<Quantity>");
        assert_eq!(
            class_name("org.openehr.base.Interval<org.openehr.base.Quantity>"),
            "Interval<org.openehr.base.Quantity>"
        );
    }

    /// The `_default` value block a namespaced cast heads carries the class
    /// name as its `_type`, so a qualified spelling in the source does not
    /// produce a dotted tag no RM reader recognises.
    #[test]
    fn a_namespaced_cast_tags_a_default_value_with_the_class_name() {
        let value = openehr_lang::odin::parse(
            "a = (org.openehr.rm.data_types.text.DV_TEXT) <value = <\"x\">>",
        )
        .expect("the namespaced cast should parse");
        let json = odin_to_json(&value);
        assert_eq!(json["a"]["_type"], serde_json::json!("DV_TEXT"));
        assert_eq!(json["a"]["value"], serde_json::json!("x"));
    }
}
