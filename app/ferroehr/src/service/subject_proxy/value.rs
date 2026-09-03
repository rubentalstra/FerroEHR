// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The `VARIABLE_VALUE` hierarchy (`variable_value.adoc` +
//! `variable_value_{single,list,time_series}.adoc`).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `VARIABLE_VALUE` — "Abstract parent of variable value structures"
/// (`variable_value.adoc`).
///
/// Its three concrete descendants form a closed subtype set, so this is an
/// idiomatic Rust enum (the codegen closed-subtype rule) rather than a trait;
/// the payloads are `Any`, carried as canonical JSON [`Value`]s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum VariableValue {
    /// `VARIABLE_VALUE_SINGLE` — "Atomic Variable value" (`value: Any [0..1]`,
    /// `variable_value_single.adoc`).
    #[serde(rename = "VARIABLE_VALUE_SINGLE")]
    Single {
        /// The single value, from the source frame's latest sample.
        value: Option<Value>,
    },
    /// `VARIABLE_VALUE_LIST` — "Variable value in the form of a list"
    /// (`value: List<Any> [0..1]`, `variable_value_list.adoc`).
    #[serde(rename = "VARIABLE_VALUE_LIST")]
    List {
        /// The list of values, from the source frame's latest sample.
        value: Vec<Value>,
    },
    /// `VARIABLE_VALUE_TIME_SERIES` — "Variable value in the form of a
    /// time-series" (`value: Hash<Iso8601_date_time, Any> [0..1]`,
    /// `variable_value_time_series.adoc`).
    #[serde(rename = "VARIABLE_VALUE_TIME_SERIES")]
    TimeSeries {
        /// The time-keyed values (ISO-8601 date-time → `Any`).
        value: BTreeMap<String, Value>,
    },
}

impl VariableValue {
    /// The empty atomic value (`VARIABLE_VALUE_SINGLE` with no `value`) — the
    /// fail-closed result of an extraction that found nothing.
    #[must_use]
    pub fn none() -> Self {
        Self::Single { value: None }
    }
}
