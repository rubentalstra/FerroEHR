//! Rebuild a generated `*Params` struct from the three HTTP sources.
//!
//! The ITS-REST contract (ADR-005) combines an operation's path, query, and
//! header parameters into a single generated `*Params` struct (see
//! `openehr_its::rest::generated`). axum, by contrast, extracts those three
//! sources separately. This module bridges the two with a small **type-directed
//! deserializer**: it merges the sources into a multi-map keyed by parameter
//! name (headers keyed by their canonical HTTP name, matching the `#[serde(rename
//! = "…")]` the generator emits) and deserializes the target struct from it,
//! coercing each value to whatever type the target field asks for.
//!
//! Why type-directed rather than building a `serde_json::Value`: query/header
//! values arrive as strings, but the generated params mix `String`, `i64`
//! (`offset`/`fetch`), `Option`, and `Vec` fields. A JSON map cannot represent
//! `"5"`-the-string and `5`-the-integer without knowing the target type — so we
//! let serde drive the coercion (`deserialize_i64` parses, `deserialize_str`
//! passes through), which no off-the-shelf crate does across all three sources
//! at once.

use http::HeaderMap;
use indexmap::IndexMap;
use serde::de::value::Error as ValueError;
use serde::de::{self, DeserializeOwned, Deserializer, IntoDeserializer, MapAccess, Visitor};

use openehr_its::rest::runtime::ApiError;

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
        // Header names are lower-cased by `http`; the generated renames use the
        // canonical spelling (`Accept`, `Content-Type`, `openehr-item-tag`).
        // Deserialization is case-sensitive on the rename, so expose both the
        // wire name and the canonical spelling used by the contract.
        let entry: Vec<String> = headers
            .get_all(name)
            .iter()
            .filter_map(|v| v.to_str().ok().map(str::to_owned))
            .collect();
        if entry.is_empty() {
            continue;
        }
        values.insert(canonical_header_name(name.as_str()), entry);
    }

    P::deserialize(RequestValuesDeserializer { values })
        .map_err(|e| ApiError::BadRequest(format!("invalid request parameters: {e}")))
}

/// Map a lower-cased header name to the canonical spelling the ITS-REST
/// contract's `#[serde(rename)]` expects. Unknown headers are passed through
/// unchanged (they simply do not match any field and are ignored).
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

/// Minimal `application/x-www-form-urlencoded` pair splitter for query strings.
/// (`+`→space, percent-decoding.) Kept local to avoid a dependency purely for
/// query splitting; robustness matches the query-string shapes the contract
/// uses.
fn form_urlencoded_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect()
}

/// Look up a single (percent-decoded) query-string parameter by key.
pub(crate) fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    let query = query?;
    form_urlencoded_pairs(query)
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

/// Percent-decode one form-urlencoded token: `+` is a space
/// (application/x-www-form-urlencoded), then WHATWG percent-decoding via the
/// `urlencoding` crate (invalid UTF-8 tolerated lossily; incomplete escapes
/// recovered per the WHATWG URL standard).
fn percent_decode(s: &str) -> String {
    let plus_decoded = s.replace('+', " ");
    String::from_utf8_lossy(&urlencoding::decode_binary(plus_decoded.as_bytes())).into_owned()
}

/// Deserializer over the merged `name → [values]` multi-map. Only map/struct
/// shapes are meaningful at the top level; everything routes through the map
/// accessor.
struct RequestValuesDeserializer {
    values: IndexMap<String, Vec<String>>,
}

impl<'de> Deserializer<'de> for RequestValuesDeserializer {
    type Error = ValueError;

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
    type Error = ValueError;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if self.cursor >= self.entries.len() {
            return Ok(None);
        }
        let (key, val) = self.entries[self.cursor].clone();
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
    fn first(&self) -> Result<&str, ValueError> {
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
    type Error = ValueError;

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

impl IntoDeserializer<'_, ValueError> for ScalarDeserializer {
    type Deserializer = Self;
    fn into_deserializer(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
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
}
