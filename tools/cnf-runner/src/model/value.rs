// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Templated values — the payload shapes of `flow[].with` and assertion
//! operands.
//!
//! A value tree is JSON-shaped; every string in it is parsed as a
//! [`Template`], so an illegal `${…}` reference anywhere in a case is a
//! load-time error (the reference/sentinel grammar check of the schedule's
//! CI gate list).

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer};

use crate::refgrammar::{RefError, Template, ValueRef};

/// A JSON-shaped value whose strings are reference templates.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplatedValue {
    /// JSON `null`.
    Null,
    /// A JSON boolean.
    Bool(bool),
    /// A JSON number, kept in its authored lexical form.
    Number(serde_json::Number),
    /// A string carrying `${…}` references, resolved per row.
    Text(Template),
    /// A JSON array of templated values.
    Seq(Vec<TemplatedValue>),
    /// Key order preserved as authored.
    Map(Vec<(String, TemplatedValue)>),
}

impl TemplatedValue {
    /// Convert from a parsed JSON value, validating every embedded string.
    ///
    /// # Errors
    /// Returns [`RefError`] for any illegal `${…}` reference.
    pub fn from_value(value: &serde_json::Value) -> Result<Self, RefError> {
        Ok(match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(*b),
            serde_json::Value::Number(n) => Self::Number(n.clone()),
            serde_json::Value::String(s) => Self::Text(Template::parse(s)?),
            serde_json::Value::Array(items) => Self::Seq(
                items
                    .iter()
                    .map(Self::from_value)
                    .collect::<Result<_, _>>()?,
            ),
            serde_json::Value::Object(map) => Self::Map(
                map.iter()
                    .map(|(k, v)| Ok((k.clone(), Self::from_value(v)?)))
                    .collect::<Result<_, RefError>>()?,
            ),
        })
    }

    /// Every reference anywhere in the tree.
    #[must_use]
    pub fn refs(&self) -> Vec<&ValueRef> {
        let mut out = Vec::new();
        self.collect_refs(&mut out);
        out
    }

    fn collect_refs<'a>(&'a self, out: &mut Vec<&'a ValueRef>) {
        match self {
            Self::Text(t) => out.extend(t.refs()),
            Self::Seq(items) => {
                for item in items {
                    item.collect_refs(out);
                }
            }
            Self::Map(entries) => {
                for (_, v) in entries {
                    v.collect_refs(out);
                }
            }
            Self::Null | Self::Bool(_) | Self::Number(_) => {}
        }
    }
}

impl<'de> Deserialize<'de> for TemplatedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::from_value(&value).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_validate_everywhere() {
        let ok = serde_json::json!({
            "versions": [
                { "data": "${ds:cnf.composition.minimal_event.v1}", "change_type": "creation" },
                { "data": "${ds:cnf.composition.minimal_event.v2}", "preceding_version_uid": "${v1}" }
            ]
        });
        let v = TemplatedValue::from_value(&ok).unwrap();
        assert_eq!(v.refs().len(), 3);

        let bad = serde_json::json!({ "nested": [{ "x": "${step2.body}" }] });
        assert!(TemplatedValue::from_value(&bad).is_err());
    }
}
