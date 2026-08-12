// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `SUBJECT_VARIABLE` (`subject_variable.adoc`).

use serde::{Deserialize, Serialize};

use super::sample::{DataFrameSample, VariableSample};

/// `SUBJECT_VARIABLE` — a proxy for a single subject variable.
///
/// "A single subject variable whose data may take various forms, including
/// atomic, list and time series … a proxy for a single subject variable,
/// **including sample history over time**" (`subject_variable.adoc`; master10
/// §Data Structures).
///
/// The definition attributes (`namespace` … `frame_path`) are configuration
/// and are persisted (master10 §Persistence); `history` and `last_frame` are
/// the variable's **runtime sample state**, materialised from the sample
/// store when a variable is read back — they are ignored on registration
/// writes (a caller cannot forge history).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectVariable {
    /// `namespace [0..1]` — optional namespace qualifying `name` (e.g.
    /// `cha2ds2vasc`); unset ⇒ the variable is in the 'global' namespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// `name [1]` — canonical name by which this item is known in the service
    /// (e.g. `date_of_birth`).
    pub name: String,
    /// `type_name [1]` — formal type name from the defining model (e.g.
    /// `Quantity`, `Date`, `Boolean`); enforced by extraction (the value is
    /// coerced/validated against it).
    pub type_name: String,
    /// `currency: Iso8601_duration [0..1]` — required currency; unset ⇒ most
    /// recent available is valid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// `ask_user: Boolean [0..1]` — if true, the service should attempt to
    /// obtain the item from a live user. Realized through the manual
    /// notification channel (`notify_variable_sample`) — the spec's own TODO
    /// notes it "can only work if access method defined".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_user: Option<bool>,
    /// `is_manual: Boolean [1]` — true if obtained by manual notification,
    /// "typically from a worker observing the subject in a point of care
    /// situation".
    pub is_manual: bool,
    /// `frame_id [1]` — identifier of the retrieve frame from which to extract
    /// this variable.
    pub frame_id: String,
    /// `frame_path [1]` — path within the frame result at which the value
    /// sits. NOTE: the SM leaves the semantics undefined ("Path within
    /// `last_frame` result"); the documented selector grammar lives with the
    /// extraction engine.
    pub frame_path: String,
    /// `history: List<VARIABLE_SAMPLE> [0..1]` — "Samples constituting the
    /// retrieve history of this variable." Read-model: newest first,
    /// materialised from the sample store; ignored on writes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<VariableSample>,
    /// `last_frame: DATA_FRAME_SAMPLE [0..1]` — "Most recent retrieve frame
    /// from which to extract variable value." Read-model; ignored on writes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_frame: Option<DataFrameSample>,
}

impl SubjectVariable {
    /// Subject-variable naming validity (SM master10 §Subject Variable
    /// Naming): a canonical name "may not contain whitespace or any
    /// unprintable character". Checked over both `name` and `namespace`
    /// (the canonical name is formed from them); the name must be non-empty.
    #[must_use]
    pub fn name_valid(&self) -> bool {
        fn part_ok(s: &str) -> bool {
            !s.is_empty() && s.chars().all(|c| !c.is_whitespace() && !c.is_control())
        }
        part_ok(&self.name) && self.namespace.as_deref().is_none_or(part_ok)
    }

    /// `canonical_name (): String` — "Return canonical name, formed from
    /// `namespace` (if present) and `name` … `namespace::name` … or just
    /// `name`" (`subject_variable.adoc`).
    #[must_use]
    pub fn canonical_name(&self) -> String {
        match &self.namespace {
            Some(ns) if !ns.is_empty() => format!("{ns}::{}", self.name),
            _ => self.name.clone(),
        }
    }

    /// `is_global (): Boolean` with `__Post_result__: Result = (namespace =
    /// Void)`. NOTE: the prose ("True if `namespace` is set") contradicts
    /// the post-condition; the post-condition is the consistent reading —
    /// *global* means **no** namespace (master10 §Subject Variable Naming:
    /// "If `namespace` is not set, the variable is understood to be in the
    /// 'global' namespace").
    #[must_use]
    pub fn is_global(&self) -> bool {
        self.namespace.as_ref().is_none_or(String::is_empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str) -> SubjectVariable {
        SubjectVariable {
            namespace: None,
            name: name.to_owned(),
            type_name: "Quantity".to_owned(),
            currency: None,
            ask_user: None,
            is_manual: false,
            frame_id: "f".to_owned(),
            frame_path: "p".to_owned(),
            history: Vec::new(),
            last_frame: None,
        }
    }

    #[test]
    fn name_validity_rejects_whitespace_and_control() {
        // SM master10 §Subject Variable Naming.
        let mut v = var("systolic_bp");
        assert!(v.name_valid());
        v.name = "systolic bp".to_owned();
        assert!(!v.name_valid(), "whitespace rejected");
        v.name = "sys\u{7}bp".to_owned();
        assert!(!v.name_valid(), "control character rejected");
        v.name = String::new();
        assert!(!v.name_valid(), "empty name rejected");
        v.name = "ok".to_owned();
        v.namespace = Some("my ns".to_owned());
        assert!(!v.name_valid(), "namespace whitespace rejected");
    }

    #[test]
    fn canonical_name_qualifies_with_namespace() {
        // master10 §Subject Variable Naming: `namespace::name`, or bare `name`.
        let global = var("date_of_birth");
        assert_eq!(global.canonical_name(), "date_of_birth");
        assert!(global.is_global());

        let qualified = SubjectVariable {
            namespace: Some("cha2ds2vasc".to_owned()),
            ..global.clone()
        };
        assert_eq!(qualified.canonical_name(), "cha2ds2vasc::date_of_birth");
        // is_global follows the post-condition `Result = (namespace = Void)`.
        assert!(!qualified.is_global());
    }

    #[test]
    fn history_and_last_frame_are_omitted_when_empty() {
        // Read-model fields serialize only when populated, so definition
        // payloads stay pure configuration (master10 §Persistence).
        let v = var("x");
        let json = serde_json::to_value(&v).expect("serialize");
        assert!(json.get("history").is_none());
        assert!(json.get("last_frame").is_none());
    }
}
