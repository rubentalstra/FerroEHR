// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Rebuilds a generated `*Params` struct from the three HTTP sources.
//!
//! The ITS-REST contract combines an operation's path, query and header
//! parameters into one generated `*Params` struct (`openehr_its::rest::generated`)
//! while axum extracts the three separately. This module merges them into a
//! multi-map keyed by parameter name — headers under the canonical HTTP name the
//! generator's `#[serde(rename = "…")]` expects — and deserializes the struct
//! from it.
//!
//! The deserializer is type-directed rather than a `serde_json::Value` because
//! query and header values all arrive as strings while the generated params mix
//! `String`, `i64`, `Option` and `Vec` fields: a JSON map cannot represent
//! `"5"`-the-string and `5`-the-integer without knowing the target type, so
//! serde drives the coercion instead.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 9): the wire boundary — one byte-to-JSON \
              step per route, consumed by the typed decode"
)]

use http::{HeaderMap, HeaderValue};
use indexmap::IndexMap;
use serde::de::value::Error;
use serde::de::{self, DeserializeOwned, Deserializer, IntoDeserializer, MapAccess, Visitor};

use openehr_its::rest::runtime::ApiError;
use openehr_rm::prelude::ItemTag;

/// Build the generated params struct `P` for an operation from its request
/// sources.
///
/// `path` are the matched axum path parameters, `query` is the raw query string
/// (the part after `?`, if any), and `headers` is the full header map. Header
/// values are exposed under their canonical HTTP name so the generated
/// `#[serde(rename = "Accept")]` (etc.) fields resolve.
///
/// # Errors
///
/// Returns [`ApiError::BadRequest`] when a supplied value cannot be coerced to
/// the type the target field requires (e.g. a non-numeric `offset`).
pub(crate) fn build<P: DeserializeOwned>(
    path: &IndexMap<String, String>,
    query: Option<&str>,
    headers: &HeaderMap,
) -> Result<P, ApiError> {
    let mut values: IndexMap<String, Vec<String>> = IndexMap::new();

    for (k, v) in path {
        values.entry(k.clone()).or_default().push(v.clone());
    }
    if let Some(q) = query {
        for (k, v) in form_urlencoded_pairs(q) {
            values.entry(k).or_default().push(v);
        }
    }
    for name in headers.keys() {
        // Deserialization is case-sensitive on the generated rename, so the
        // lower-cased wire name is exposed under the contract's spelling too.
        // NOTE: no openEHR spec governs undecodable header bytes — our own
        // design: refuse, because dropping the value would deserialize the
        // request as if the header had never been sent.
        let entry: Vec<String> = headers
            .get_all(name)
            .iter()
            .map(|v| {
                v.to_str().map(str::to_owned).map_err(|e| {
                    tracing::debug!(header = %name, error = %e, "undecodable header value → 400");
                    ApiError::BadRequest(format!(
                        "header {name} carries a value that is not decodable as text"
                    ))
                })
            })
            .collect::<Result<_, _>>()?;
        if entry.is_empty() {
            continue;
        }
        values.insert(canonical_header_name(name.as_str()), entry);
    }

    P::deserialize(RequestValuesDeserializer { values })
        .map_err(|e| ApiError::BadRequest(format!("invalid request parameters: {e}")))
}

/// Maps a lower-cased header name to the canonical spelling the ITS-REST
/// contract's `#[serde(rename)]` expects.
///
/// An unknown header passes through unchanged and matches no field.
fn canonical_header_name(lower: &str) -> String {
    match lower {
        "accept" => "Accept".to_owned(),
        "content-type" => "Content-Type".to_owned(),
        "prefer" => "Prefer".to_owned(),
        "if-match" => "If-Match".to_owned(),
        "if-none-match" => "If-None-Match".to_owned(),
        other => other.to_owned(),
    }
}

/// Splits a query string into decoded `application/x-www-form-urlencoded` pairs.
///
/// `form_urlencoded::parse` does the whole job — the pair split, the `+`-to-space
/// rule and percent-decoding — to the WHATWG URL standard.
fn form_urlencoded_pairs(query: &str) -> Vec<(String, String)> {
    form_urlencoded::parse(query.as_bytes())
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

/// Returns the named query parameters of a query-execution `GET` as AQL binds
/// (ITS-REST `docs/query/Request.md` §"Query parameters").
///
/// Every query-string key that is not a reserved request control becomes a
/// bind; a `$` prefix is tolerated and stripped (parameter names "SHOULD NOT be
/// prefixed with `$`"). Values are read as JSON first and fall back to strings,
/// and repeats are last-wins. Binds from the literal `query_parameters=<JSON
/// object>` form arrive in `base`, and a name collision resolves to the named
/// form.
pub(crate) fn named_query_parameters(
    query: Option<&str>,
    base: std::collections::BTreeMap<String, serde_json::Value>,
    reserved: &[&str],
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut parameters = base;
    let Some(query) = query else {
        return parameters;
    };
    for (key, raw) in form_urlencoded_pairs(query) {
        let name = key.strip_prefix('$').unwrap_or(&key);
        if name.is_empty() || reserved.contains(&name) {
            continue;
        }
        let value = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .filter(|v| !v.is_array() && !v.is_object())
            .unwrap_or_else(|| serde_json::Value::String(raw.clone()));
        parameters.insert(name.to_owned(), value);
    }
    parameters
}

/// The reserved query-string keys of the query-execution `GET`s: the request
/// controls the contract itself defines.
pub(crate) const QUERY_RESERVED_KEYS: &[&str] =
    &["ehr_id", "offset", "fetch", "q", "query_parameters"];

/// Looks up a single percent-decoded query-string parameter by key.
pub(crate) fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    let query = query?;
    form_urlencoded_pairs(query)
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

// The `openehr-item-tag` / `openehr-version-item-tag` wrappers over the
// dedicated ITEM_TAG operations (overview §"openehr-item-tag and
// openehr-version-item-tag"): a `;`-separated list of entries, each a
// comma-separated `key`/`value`/`target_path` set, targeting a
// VERSIONED_OBJECT or one VERSION respectively.
// NOTE: this module owns only header parse/validate/emit; the EHR group
// validates before the content commit and writes after it, so a defective tag
// refuses the request and a tag never re-versions its content.

/// The canonical HTTP header names for the two `ITEM_TAG` wrapper headers.
pub(crate) const H_ITEM_TAG: &str = "openehr-item-tag";
pub(crate) const H_VERSION_ITEM_TAG: &str = "openehr-version-item-tag";

/// A single `openehr-item-tag` / `openehr-version-item-tag` entry: a `key`, its
/// optional `value`, and an optional `target_path`. Multiple `ITEM_TAGs` may
/// target one resource, uniquely identified by their `key`+`target_path` pair
/// (overview §"openehr-item-tag and openehr-version-item-tag").
///
/// `value` is `Option` because `ITEM_TAG.value` is `0..1` and
/// `Inv_value_valid` forbids a set-but-empty one: a header entry spelling
/// `value=""` normalizes to absent on the way in, and the echo renders no
/// `value` token on the way out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemTagHeaderEntry {
    /// The tag key.
    pub(crate) key: String,
    /// The tag value, absent for a bare marker tag.
    pub(crate) value: Option<String>,
    /// Optional RM path the tag is anchored to within the target.
    pub(crate) target_path: Option<String>,
}

/// Splits a wrapper-header value into its `;`-separated entries, quote-aware.
///
/// A `target_path` is a quoted token that may legitimately contain a `;` (an AQL
/// path predicate such as `[at0001, 'a;b']`), so this scanner breaks only on a
/// `;` outside a double-quoted run, exactly as [`key_value_pairs`] treats a
/// quoted value as opaque at the `,` level.
///
/// The release gives the header no ABNF — the grammar is one worked example
/// (`Requests_and_responses.md` §openehr-item-tag and openehr-version-item-tag)
/// showing quoted values and both separators but no escaping rules. Treating a
/// quoted run as opaque is our own reading, and the only one under which the
/// section's own `target_path="/composition/start_time/value"` token stays
/// meaningful when a path contains a separator.
fn split_item_tag_entries(input: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut in_quotes = false;
    let mut start = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ';' if !in_quotes => {
                if let Some(segment) = input.get(start..idx) {
                    out.push(segment);
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    if let Some(tail) = input.get(start..) {
        out.push(tail);
    }
    out
}

/// Parses an `ITEM_TAG` wrapper header ([`H_ITEM_TAG`] or
/// [`H_VERSION_ITEM_TAG`]) into its entries, merging repeated occurrences.
///
/// Returns `None` when the header is absent and `Some(empty)` when it is present
/// but empty — the spec's "remove all `ITEM_TAGs`" signal.
///
/// # Errors
/// [`ApiError::BadRequest`] when a non-blank entry carries no `key`: the
/// released schema makes `key` the one required member of an `UPDATE_ITEM_TAG`
/// (`schemas/common/UpdateItemTag.yaml`), so the wrapper cannot admit what the
/// operation refuses. A blank segment carries no entry at all and is not an
/// error.
pub(crate) fn parse_item_tag_header(
    headers: &HeaderMap,
    name: &str,
) -> Result<Option<Vec<ItemTagHeaderEntry>>, ApiError> {
    // An undecodable value is refused, never skipped: dropping it would remove
    // a tag the client believes it set.
    let raws: Vec<String> = headers
        .get_all(name)
        .iter()
        .map(|v| {
            v.to_str().map(str::to_owned).map_err(|e| {
                tracing::debug!(header = name, error = %e, "undecodable header value → 400");
                ApiError::BadRequest(format!(
                    "header {name} carries a value that is not decodable as text"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    if raws.is_empty() {
        return Ok(None);
    }
    let joined = raws.join(";");
    // An empty value "will effectively remove all ITEM_TAGs" for the target.
    if joined.trim().is_empty() {
        return Ok(Some(Vec::new()));
    }
    let mut out = Vec::new();
    for segment in split_item_tag_entries(&joined) {
        if segment.trim().is_empty() {
            continue;
        }
        let pairs = key_value_pairs(segment);
        let Some(key) = tag_value(&pairs, "key") else {
            return Err(ApiError::BadRequest(format!(
                "the {name} header entry {segment:?} carries no `key`; every ITEM_TAG \
                 entry must name one"
            )));
        };
        out.push(ItemTagHeaderEntry {
            key,
            // `value=""` is the absent value, never a stored empty string (RM
            // `ITEM_TAG.Inv_value_valid`: a set value may not be empty).
            value: tag_value(&pairs, "value").filter(|v| !v.is_empty()),
            target_path: tag_value(&pairs, "target_path"),
        });
    }
    Ok(Some(out))
}

/// Judges parsed wrapper-header entries against the RM `ITEM_TAG` invariants,
/// before the request's content is committed.
///
/// The invariants are evaluated through `openehr_rm`'s own predicates
/// ([`ItemTag::key_valid`] / [`ItemTag::value_valid`]), which the `Validate` impl
/// for `ItemTag` is written in terms of, so this check and the service seam that
/// writes the tags cannot disagree. They are reached as predicates rather than
/// through `Validate` because the version the tag's `target` names is not minted
/// yet, so no `ITEM_TAG` instance exists.
///
/// # Errors
/// [`ApiError::Unprocessable`] naming the offending entry and the invariant it
/// breaks (`Inv_key_valid` / `Inv_value_valid`,
/// `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc`).
pub(crate) fn validate_item_tag_entries(
    entries: &[ItemTagHeaderEntry],
    name: &str,
) -> Result<(), ApiError> {
    for entry in entries {
        if !ItemTag::key_valid(&entry.key) {
            return Err(ApiError::Unprocessable(format!(
                "the {name} header entry key {:?} breaks ITEM_TAG.Inv_key_valid \
                 (a key may not be empty or carry leading or trailing whitespace)",
                entry.key
            )));
        }
        if !ItemTag::value_valid(entry.value.as_deref()) {
            return Err(ApiError::Unprocessable(format!(
                "the {name} header entry {:?} breaks ITEM_TAG.Inv_value_valid \
                 (a value, if set, may not be empty)",
                entry.key
            )));
        }
    }
    Ok(())
}

/// Renders `ITEM_TAG` entries as a wrapper-header value (`;`-separated
/// `key="…"[,value="…"][,target_path="…"]` pairs), for echoing stored tags on a
/// response (overview §"Usage in Responses", a MAY).
///
/// Returns `None` when the list cannot be rendered as a header value — a key or
/// value carrying a byte HTTP forbids in a field value (RFC 9110 §5.5), which
/// nothing in the RM bars from an `ITEM_TAG.key`. The caller must then omit the
/// header entirely and never fall back to an empty one: an empty value is the
/// release's instruction that "providing an empty value for this header will
/// effectively remove all `ITEM_TAGs` associated with the given target"
/// (§Usage in Requests), so an echo of it would hand the client a destructive
/// form as state. A valueless tag renders without a `value` token for the same
/// reason: a `value=""` echo describes a tag violating `Inv_value_valid`.
pub(crate) fn emit_item_tag_header(entries: &[ItemTagHeaderEntry]) -> Option<HeaderValue> {
    if entries.is_empty() {
        return None;
    }
    let rendered = entries
        .iter()
        .map(|e| {
            let mut parts = vec![format!("key=\"{}\"", e.key)];
            if let Some(value) = &e.value {
                parts.push(format!("value=\"{value}\""));
            }
            if let Some(tp) = &e.target_path {
                parts.push(format!("target_path=\"{tp}\""));
            }
            parts.join(",")
        })
        .collect::<Vec<_>>()
        .join("; ");
    HeaderValue::from_str(&rendered).ok()
}

/// Projects one RM [`ItemTag`] onto the [`ItemTagHeaderEntry`]
/// [`emit_item_tag_header`] renders: the three members the header grammar
/// carries, read off the typed instance.
pub(crate) fn item_tag_to_header_entry(tag: &ItemTag) -> ItemTagHeaderEntry {
    ItemTagHeaderEntry {
        key: tag.key().to_owned(),
        value: tag.value().map(str::to_owned),
        target_path: tag.target_path().map(str::to_owned),
    }
}

/// Returns the value of a parsed `key` in a tag-pair segment.
fn tag_value(pairs: &[(String, String)], key: &str) -> Option<String> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Parses a tolerant comma-separated list of `key="value"` or bare `key=value`
/// pairs.
///
/// The one scanner behind both the tag-pair segments here and the committal
/// attribute headers ([`crate::overview::committal`]): a double-quoted value is
/// read opaquely, a bare value runs to the next top-level comma, and whitespace
/// around separators and keys is trimmed. Both grammars are example-only in the
/// ITS-REST overview, hence the shared tolerant reader.
pub(crate) fn key_value_pairs(input: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = input;
    loop {
        // Skip leading separators/whitespace.
        rest = rest.trim_start_matches(|c: char| c == ',' || c.is_ascii_whitespace());
        // Read the key up to '='. A segment whose next delimiter is a comma
        // carries no '=' — not a pair; leave the comma for the skip above.
        let Some((key, after_key)) = rest.find(['=', ',']).and_then(|d| rest.split_at_checked(d))
        else {
            break;
        };
        let Some(after_eq) = after_key.strip_prefix('=') else {
            rest = after_key;
            continue;
        };
        // Read the value: quoted (opaque) or bare (to next comma).
        let (value, tail) = if let Some(quoted) = after_eq.strip_prefix('"') {
            match quoted.find('"').and_then(|q| quoted.split_at_checked(q)) {
                // Consume the closing quote; anything before the next comma is
                // then skipped as a keyless segment.
                Some((v, after_v)) => (v.to_owned(), after_v.get(1..).unwrap_or_default()),
                None => (quoted.to_owned(), ""),
            }
        } else {
            match after_eq
                .find(',')
                .and_then(|c| after_eq.split_at_checked(c))
            {
                Some((v, after_v)) => (v.trim().to_owned(), after_v),
                None => (after_eq.trim().to_owned(), ""),
            }
        };
        let key = key.trim();
        if !key.is_empty() {
            out.push((key.to_owned(), value));
        }
        rest = tail;
    }
    out
}

/// Deserializer over the merged `name` to `[values]` multi-map.
///
/// Only map and struct shapes are meaningful at the top level; everything
/// routes through the map accessor.
struct RequestValuesDeserializer {
    values: IndexMap<String, Vec<String>>,
}

impl<'de> Deserializer<'de> for RequestValuesDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(RequestMapAccess {
            entries: self.values.into_iter().collect(),
            cursor: 0,
            value: None,
        })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

/// Walks the multi-map entries, handing each value to [`ScalarDeserializer`].
struct RequestMapAccess {
    entries: Vec<(String, Vec<String>)>,
    cursor: usize,
    value: Option<Vec<String>>,
}

impl<'de> MapAccess<'de> for RequestMapAccess {
    type Error = Error;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        let Some((key, val)) = self.entries.get(self.cursor).cloned() else {
            return Ok(None);
        };
        self.cursor += 1;
        self.value = Some(val);
        seed.deserialize(key.into_deserializer()).map(Some)
    }

    fn next_value_seed<S: de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        let values = self
            .value
            .take()
            .ok_or_else(|| de::Error::custom("value requested before key"))?;
        seed.deserialize(ScalarDeserializer { values })
    }
}

/// Deserializes one parameter's value(s), coercing the raw string(s) to the
/// type the target field asks for.
struct ScalarDeserializer {
    values: Vec<String>,
}

impl ScalarDeserializer {
    fn first(&self) -> Result<&str, Error> {
        self.values
            .first()
            .map(String::as_str)
            .ok_or_else(|| de::Error::custom("empty parameter value"))
    }
}

macro_rules! deserialize_parsed {
    ($method:ident, $visit:ident, $ty:ty) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            let raw = self.first()?;
            let parsed: $ty = raw.parse().map_err(|_| {
                de::Error::custom(format!("expected {}, got {raw:?}", stringify!($ty)))
            })?;
            visitor.$visit(parsed)
        }
    };
}

impl<'de> Deserializer<'de> for ScalarDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // Untyped targets (e.g. `serde_json::Value`) receive the raw string.
        visitor.visit_string(self.first()?.to_owned())
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_str(self.first()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.first()?.to_owned())
    }

    deserialize_parsed!(deserialize_bool, visit_bool, bool);
    deserialize_parsed!(deserialize_i8, visit_i8, i8);
    deserialize_parsed!(deserialize_i16, visit_i16, i16);
    deserialize_parsed!(deserialize_i32, visit_i32, i32);
    deserialize_parsed!(deserialize_i64, visit_i64, i64);
    deserialize_parsed!(deserialize_u8, visit_u8, u8);
    deserialize_parsed!(deserialize_u16, visit_u16, u16);
    deserialize_parsed!(deserialize_u32, visit_u32, u32);
    deserialize_parsed!(deserialize_u64, visit_u64, u64);
    deserialize_parsed!(deserialize_f32, visit_f32, f32);
    deserialize_parsed!(deserialize_f64, visit_f64, f64);

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // A present key always deserializes to `Some`; missing keys never reach
        // here (serde yields `None` for absent `Option` fields).
        visitor.visit_some(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let elems = self
            .values
            .into_iter()
            .map(|v| ScalarDeserializer { values: vec![v] });
        visitor.visit_seq(de::value::SeqDeserializer::new(elems))
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // The only map-typed contract field is `query_parameters`
        // (`BTreeMap<String, Value>`), rarely sent via the query string; when it
        // is, the value is a JSON object literal.
        let raw = self.first()?;
        let json: serde_json::Value = serde_json::from_str(raw)
            .map_err(|e| de::Error::custom(format!("expected JSON object: {e}")))?;
        json.deserialize_map(visitor)
            .map_err(|e: serde_json::Error| de::Error::custom(e))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        i128 u128 char bytes byte_buf unit unit_struct newtype_struct tuple
        tuple_struct struct enum identifier
    }
}

impl IntoDeserializer<'_, Error> for ScalarDeserializer {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    // ── named query parameters (Request.md §Query parameters) ────────────────

    /// The documented GET form: arbitrary NAMED keys become AQL binds
    /// (worked example `?temperature_from=36&temperature_unit=Cel`), JSON-first
    /// typing with string fallback, `$` prefix tolerated-and-stripped,
    /// reserved request controls excluded, and the JSON-object
    /// `query_parameters` superset merged with named-wins collisions.
    #[test]
    fn named_query_parameters_bind_per_the_docs_text() {
        use serde_json::{Value, json};
        let base: std::collections::BTreeMap<String, Value> = [
            ("from_object".to_owned(), json!("x")),
            ("shared".to_owned(), json!("object-form")),
        ]
        .into_iter()
        .collect();
        let got = named_query_parameters(
            Some(
                "temperature_from=36&temperature_unit=Cel&$flagged=true\
                 &uid=90910cf0-66a0-4382-b1f8-c0f27e81b42d::openEHRSys.example.com::1\
                 &offset=10&fetch=5&ehr_id=abc&q=SELECT&query_parameters=%7B%7D\
                 &shared=named-form&name=a+b",
            ),
            base,
            QUERY_RESERVED_KEYS,
        );
        assert_eq!(got["temperature_from"], json!(36));
        assert_eq!(got["temperature_unit"], json!("Cel"));
        assert_eq!(got["flagged"], json!(true), "$ prefix stripped, JSON-typed");
        assert_eq!(
            got["uid"],
            json!("90910cf0-66a0-4382-b1f8-c0f27e81b42d::openEHRSys.example.com::1"),
            "a version uid stays text"
        );
        assert_eq!(got["from_object"], json!("x"), "object-form binds survive");
        assert_eq!(
            got["shared"],
            json!("named-form"),
            "named form wins a collision"
        );
        assert_eq!(got["name"], json!("a b"), "form decoding applies");
        for reserved in QUERY_RESERVED_KEYS {
            assert!(
                !got.contains_key(*reserved),
                "reserved control {reserved:?} must not bind"
            );
        }
    }

    /// Structured JSON literals stay strings on the named form (an array or
    /// object as a bare query value is not a documented bind shape), and an
    /// absent query string passes the base through untouched.
    #[test]
    fn named_query_parameters_edges() {
        use serde_json::{Value, json};
        let got = named_query_parameters(
            Some("list=%5B1%2C2%5D"),
            std::collections::BTreeMap::new(),
            QUERY_RESERVED_KEYS,
        );
        assert_eq!(got["list"], json!("[1,2]"), "structured literals stay text");
        let base: std::collections::BTreeMap<String, Value> =
            [("kept".to_owned(), json!(1))].into_iter().collect();
        assert_eq!(
            named_query_parameters(None, base.clone(), QUERY_RESERVED_KEYS),
            base
        );
    }

    use super::*;
    use http::HeaderValue;
    use serde::Deserialize;

    fn path(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Sample {
        ehr_id: String,
        offset: Option<i64>,
        fetch: Option<i64>,
        #[serde(rename = "Accept")]
        accept: Option<String>,
    }

    #[test]
    fn path_and_typed_query_and_header() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("application/json"));
        let got: Sample = build(
            &path(&[("ehr_id", "abc-123")]),
            Some("offset=5&fetch=20"),
            &headers,
        )
        .expect("params");
        assert_eq!(
            got,
            Sample {
                ehr_id: "abc-123".to_owned(),
                offset: Some(5),
                fetch: Some(20),
                accept: Some("application/json".to_owned()),
            }
        );
    }

    #[test]
    fn numeric_looking_string_stays_string() {
        // `ehr_id` is a String field; a numeric-looking value must not be coerced.
        let got: Sample =
            build(&path(&[("ehr_id", "12345")]), None, &HeaderMap::new()).expect("params");
        assert_eq!(got.ehr_id, "12345");
        assert_eq!(got.offset, None);
    }

    #[test]
    fn missing_optional_fields_are_none() {
        let got: Sample =
            build(&path(&[("ehr_id", "x")]), None, &HeaderMap::new()).expect("params");
        assert_eq!(got.offset, None);
        assert_eq!(got.accept, None);
    }

    #[test]
    fn non_numeric_offset_is_bad_request() {
        let err = build::<Sample>(
            &path(&[("ehr_id", "x")]),
            Some("offset=notanumber"),
            &HeaderMap::new(),
        )
        .expect_err("should reject");
        assert!(matches!(err, ApiError::BadRequest(_)), "got {err:?}");
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct WithSeq {
        #[serde(rename = "openehr-item-tag")]
        tags: Option<Vec<String>>,
    }

    #[test]
    fn repeated_header_becomes_seq() {
        let mut headers = HeaderMap::new();
        headers.append("openehr-item-tag", HeaderValue::from_static("a"));
        headers.append("openehr-item-tag", HeaderValue::from_static("b"));
        let got: WithSeq = build(&IndexMap::new(), None, &headers).expect("params");
        assert_eq!(got.tags, Some(vec!["a".to_owned(), "b".to_owned()]));
    }

    #[test]
    fn percent_and_plus_decoding() {
        let pairs = form_urlencoded_pairs("q=SELECT%20c&name=a+b");
        assert_eq!(
            pairs,
            vec![
                ("q".to_owned(), "SELECT c".to_owned()),
                ("name".to_owned(), "a b".to_owned()),
            ]
        );
    }

    // ── openehr-item-tag / openehr-version-item-tag ─────────────────────────

    fn parsed(h: &HeaderMap, name: &str) -> Option<Vec<ItemTagHeaderEntry>> {
        parse_item_tag_header(h, name).expect("a well-formed item-tag header")
    }

    fn entry(key: &str, value: Option<&str>, target_path: Option<&str>) -> ItemTagHeaderEntry {
        ItemTagHeaderEntry {
            key: key.to_owned(),
            value: value.map(str::to_owned),
            target_path: target_path.map(str::to_owned),
        }
    }

    #[test]
    fn item_tag_header_absent_is_none() {
        assert_eq!(parsed(&HeaderMap::new(), H_ITEM_TAG), None);
    }

    #[test]
    fn item_tag_header_empty_value_clears() {
        let mut h = HeaderMap::new();
        h.insert(H_ITEM_TAG, HeaderValue::from_static(""));
        // Present-but-empty ⇒ "remove all ITEM_TAGs".
        assert_eq!(parsed(&h, H_ITEM_TAG), Some(Vec::new()));
    }

    #[test]
    fn item_tag_single_pair() {
        let mut h = HeaderMap::new();
        h.insert(
            H_ITEM_TAG,
            HeaderValue::from_static("key=\"category\",value=\"final\""),
        );
        assert_eq!(
            parsed(&h, H_ITEM_TAG),
            Some(vec![entry("category", Some("final"), None)])
        );
    }

    #[test]
    fn version_item_tag_semicolon_list_with_target_path() {
        // The spec example (line 108).
        let mut h = HeaderMap::new();
        h.insert(
            H_VERSION_ITEM_TAG,
            HeaderValue::from_static(
                "key=\"reviewed\",value=\"true\"; key=\"flag\",value=\"follow-up\",target_path=\"/composition/start_time/value\"",
            ),
        );
        let entries = parsed(&h, H_VERSION_ITEM_TAG).expect("entries");
        assert_eq!(
            entries,
            vec![
                entry("reviewed", Some("true"), None),
                entry(
                    "flag",
                    Some("follow-up"),
                    Some("/composition/start_time/value")
                ),
            ]
        );
    }

    #[test]
    fn item_tag_repeated_headers_merge() {
        let mut h = HeaderMap::new();
        h.append(
            H_ITEM_TAG,
            HeaderValue::from_static("key=\"a\",value=\"1\""),
        );
        h.append(
            H_ITEM_TAG,
            HeaderValue::from_static("key=\"b\",value=\"2\""),
        );
        let entries = parsed(&h, H_ITEM_TAG).expect("entries");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "a");
        assert_eq!(entries[1].key, "b");
    }

    #[test]
    fn item_tag_emit_round_trips() {
        let entries = vec![
            entry("reviewed", Some("true"), None),
            entry(
                "flag",
                Some("follow-up"),
                Some("/composition/start_time/value"),
            ),
        ];
        let hv = emit_item_tag_header(&entries).expect("an encodable list");
        let mut h = HeaderMap::new();
        h.insert(H_VERSION_ITEM_TAG, hv);
        assert_eq!(parsed(&h, H_VERSION_ITEM_TAG), Some(entries));
    }

    #[test]
    fn a_valueless_tag_echoes_without_a_value_token() {
        // RM `ITEM_TAG.value` is 0..1 and `Inv_value_valid` forbids a
        // set-but-empty one, so `value=""` would describe a tag this server
        // never stored — and a client mirroring it back would post an invalid
        // tag. The token is omitted instead.
        let hv = emit_item_tag_header(&[entry("marker", None, None)]).expect("an encodable list");
        assert_eq!(hv.to_str().expect("ascii"), r#"key="marker""#);
        // …and it round-trips to the same valueless entry.
        let mut h = HeaderMap::new();
        h.insert(H_ITEM_TAG, hv);
        assert_eq!(
            parsed(&h, H_ITEM_TAG),
            Some(vec![entry("marker", None, None)])
        );
    }

    #[test]
    fn an_unencodable_list_yields_no_header_rather_than_an_empty_one() {
        // An EMPTY `openehr-item-tag` is the release's "remove all ITEM_TAGs"
        // instruction (overview §Usage in Requests), so it must never be the
        // fallback for a list that cannot be rendered: the caller omits the
        // header entirely. A control character in the key is the reachable
        // case: nothing in the RM forbids one (`Inv_key_valid` bars only an
        // empty or whitespace-padded key), while RFC 9110 §5.5 bars it from a
        // field value.
        assert_eq!(emit_item_tag_header(&[entry("a\nb", None, None)]), None);
        // Non-ASCII text is NOT the unencodable case — obs-text is legal in a
        // field value — so such a list still echoes.
        assert!(emit_item_tag_header(&[entry("café", Some("naïve"), None)]).is_some());
    }

    #[test]
    fn a_request_value_of_empty_string_is_the_absent_value() {
        let mut h = HeaderMap::new();
        h.insert(H_ITEM_TAG, HeaderValue::from_static("key=\"k\",value=\"\""));
        assert_eq!(parsed(&h, H_ITEM_TAG), Some(vec![entry("k", None, None)]));
    }

    #[test]
    fn a_quoted_semicolon_does_not_split_the_entry() {
        // A `target_path` is an AQL or RM path (RM `item_tag.adoc`
        // `target_path`), and an AQL predicate may carry a `;` inside a quoted
        // string. Splitting on the raw `;` shattered such an entry into
        // fragments that then parsed as garbage.
        let mut h = HeaderMap::new();
        h.insert(
            H_ITEM_TAG,
            HeaderValue::from_static(
                "key=\"flag\",target_path=\"/items[at0001, 'a;b']/value\"; key=\"other\"",
            ),
        );
        assert_eq!(
            parsed(&h, H_ITEM_TAG),
            Some(vec![
                entry("flag", None, Some("/items[at0001, 'a;b']/value")),
                entry("other", None, None),
            ])
        );
    }

    #[test]
    fn a_keyless_entry_is_refused_not_skipped() {
        // `key` is the one REQUIRED member of an UPDATE_ITEM_TAG
        // (`schemas/common/UpdateItemTag.yaml`), and the header is a wrapper
        // around that operation — so the wrapper cannot admit what the
        // operation refuses. Skipping the entry would silently drop a tag the
        // client believes it set.
        let mut h = HeaderMap::new();
        h.insert(
            H_ITEM_TAG,
            HeaderValue::from_static("key=\"a\"; value=\"orphan\""),
        );
        let refused = parse_item_tag_header(&h, H_ITEM_TAG);
        assert!(
            matches!(refused, Err(ApiError::BadRequest(_))),
            "a keyless entry must be a 400, got {refused:?}"
        );
    }

    #[test]
    fn a_blank_segment_carries_no_entry_and_is_not_an_error() {
        // A trailing `;`, or an empty repeat of the header, carries no entry at
        // all — the release's own empty-value form is meaningful, so a blank
        // segment is not a defect.
        let mut h = HeaderMap::new();
        h.append(H_ITEM_TAG, HeaderValue::from_static("key=\"a\";"));
        h.append(H_ITEM_TAG, HeaderValue::from_static(""));
        assert_eq!(parsed(&h, H_ITEM_TAG), Some(vec![entry("a", None, None)]));
    }

    #[test]
    fn the_entry_validator_enforces_the_rm_invariants() {
        // Both refusals, and the accepting twin.
        assert!(validate_item_tag_entries(&[entry("ok", Some("v"), None)], H_ITEM_TAG).is_ok());
        assert!(validate_item_tag_entries(&[entry("ok", None, None)], H_ITEM_TAG).is_ok());
        for bad in [entry(" padded ", None, None), entry("", None, None)] {
            let refused = validate_item_tag_entries(std::slice::from_ref(&bad), H_ITEM_TAG);
            assert!(
                matches!(refused, Err(ApiError::Unprocessable(_))),
                "{bad:?} breaks Inv_key_valid and must be refused, got {refused:?}"
            );
        }
        let refused = validate_item_tag_entries(&[entry("k", Some(""), None)], H_ITEM_TAG);
        assert!(
            matches!(refused, Err(ApiError::Unprocessable(_))),
            "a set-but-empty value breaks Inv_value_valid, got {refused:?}"
        );
    }
}
