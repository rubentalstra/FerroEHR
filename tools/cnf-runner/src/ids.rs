// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Identifier newtypes for the schedule artifacts.
//!
//! Every identifier space in the CNF 2.0 artifact set is a distinct type so a
//! swapped-argument mistake is a compile error, and each carries its lexical
//! rule at parse time (CNF `platform_test_schedule/master03-overview.adoc`
//! defines the case-id families; the rest are this framework's own closed
//! grammars — no openEHR spec governs them: our own design).

use std::fmt;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Lexical error for any identifier newtype in this module.
#[derive(Debug, Error)]
#[error("invalid {kind}: {value:?} ({rule})")]
pub struct IdError {
    kind: &'static str,
    value: String,
    rule: &'static str,
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident, $kind:literal, $rule:literal, $check:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parse and lexically validate the identifier.
            ///
            /// # Errors
            /// Returns [`IdError`] when the value violates the identifier's
            /// lexical rule.
            pub fn parse(value: &str) -> Result<Self, IdError> {
                // The rule arrives either as a non-capturing closure literal or
                // as a fn item; binding it to a fn pointer accepts both and
                // keeps the expansion free of a directly-called closure.
                let rule_holds: fn(&str) -> bool = $check;
                if rule_holds(value) {
                    Ok(Self(value.to_owned()))
                } else {
                    Err(IdError { kind: $kind, value: value.to_owned(), rule: $rule })
                }
            }

            /// The identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = IdError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Self::parse(&s).map_err(D::Error::custom)
            }
        }
    };
}

string_id!(
    /// A global CNF case id.
    ///
    /// The families are `<SERVICE_COMPONENT>.<operation>-<variant>`,
    /// `CONT-<TYPE>-<variant>`, `SF-<FORM>-<variant>`, … . Family membership
    /// is checked by the validator; lexically an id is non-empty ASCII
    /// without whitespace.
    CaseId,
    "case id",
    "non-empty, ASCII, no whitespace",
    |s: &str| !s.is_empty() && s.is_ascii() && !s.contains(char::is_whitespace)
);

string_id!(
    /// A verdict-bearing capability name from the capability matrix
    /// (e.g. `EhrOperations`).
    CapabilityName,
    "capability name",
    "ASCII identifier",
    is_ident
);

string_id!(
    /// A corpus manifest key (`cnf.composition.minimal_event.v1`): lowercase
    /// dotted segments.
    CorpusKey,
    "corpus key",
    "dot-separated lowercase segments of [a-z0-9_-]",
    |s: &str| {
        !s.is_empty()
            && s.split('.').all(|seg| {
                !seg.is_empty()
                    && seg.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
            })
    }
);

string_id!(
    /// An ambiguity-register id (`AMB-<n>`).
    AmbiguityId,
    "ambiguity id",
    "AMB-<number>",
    |s: &str| {
        s.strip_prefix("AMB-").is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    }
);

string_id!(
    /// An option tag realizing one branch of an `option_select` ambiguity
    /// (e.g. `adl14-duplicate-conflict`).
    OptionTag,
    "option tag",
    "non-empty [a-z0-9_-]",
    |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
);

string_id!(
    /// A named projection over a corpus data set (referenced as
    /// `${ds:<key>#<view>}`).
    ViewName,
    "view name",
    "ASCII identifier",
    is_ident
);

string_id!(
    /// A named row-to-instance synthesis recipe (referenced as
    /// `${recipe:<name>(row)}`).
    RecipeName,
    "recipe name",
    "ASCII identifier",
    is_ident
);

string_id!(
    /// A named SUT instance from `ixit.json` (the flow `on:` selector;
    /// default `sut`).
    InstanceName,
    "instance name",
    "ASCII identifier",
    is_ident
);

string_id!(
    /// A case-scoped capture / `requires`-handle name (`ehr_id`, `v2_uid`).
    CaptureName,
    "capture name",
    "ASCII identifier",
    is_ident
);

/// An SM operation anchor: `I_<INTERFACE>.<operation>` — resolved by the
/// validator against the vendored SM UML class exports
/// (`docs/specs/openehr/SM/docs/UML/classes/`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmOperationRef {
    interface: String,
    operation: String,
}

impl SmOperationRef {
    /// Parse `I_<INTERFACE>.<operation>`.
    ///
    /// # Errors
    /// Returns [`IdError`] when the value is not an `I_…`-prefixed interface
    /// name followed by exactly one `.` and an operation identifier.
    pub fn parse(value: &str) -> Result<Self, IdError> {
        let err = |rule| IdError {
            kind: "SM operation",
            value: value.to_owned(),
            rule,
        };
        let (interface, operation) = value
            .split_once('.')
            .ok_or_else(|| err("must be I_<INTERFACE>.<operation>"))?;
        if !interface.starts_with("I_")
            || !interface
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(err("interface must be I_UPPER_SNAKE"));
        }
        if !operation
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase())
            || !operation
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(err("operation must be lower_snake"));
        }
        Ok(Self {
            interface: interface.to_owned(),
            operation: operation.to_owned(),
        })
    }

    /// The SM interface name (`I_EHR_SERVICE`).
    #[must_use]
    pub fn interface(&self) -> &str {
        &self.interface
    }

    /// The operation name (`create_ehr`).
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Build a reference to a sibling operation on the same interface (the
    /// flow `call:` short form, which "resolves against `sm_operation`'s
    /// interface" per the case-core contract).
    #[must_use]
    pub fn sibling(&self, operation: &str) -> Self {
        Self {
            interface: self.interface.clone(),
            operation: operation.to_owned(),
        }
    }
}

impl fmt::Display for SmOperationRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.interface, self.operation)
    }
}

impl std::str::FromStr for SmOperationRef {
    type Err = IdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for SmOperationRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SmOperationRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sm_operation_parses_and_prints() {
        let op = SmOperationRef::parse("I_EHR_SERVICE.create_ehr").unwrap();
        assert_eq!(op.interface(), "I_EHR_SERVICE");
        assert_eq!(op.operation(), "create_ehr");
        assert_eq!(op.to_string(), "I_EHR_SERVICE.create_ehr");
        assert_eq!(op.sibling("get_ehr").to_string(), "I_EHR_SERVICE.get_ehr");
    }

    #[test]
    fn sm_operation_rejects_malformed() {
        assert!(SmOperationRef::parse("EHR_SERVICE.create_ehr").is_err());
        assert!(SmOperationRef::parse("I_EHR_SERVICE").is_err());
        assert!(SmOperationRef::parse("I_EHR_SERVICE.CreateEhr").is_err());
    }

    #[test]
    fn ambiguity_id_shape() {
        assert!(AmbiguityId::parse("AMB-13").is_ok());
        assert!(AmbiguityId::parse("AMB-").is_err());
        assert!(AmbiguityId::parse("amb-1").is_err());
    }

    #[test]
    fn corpus_key_shape() {
        assert!(CorpusKey::parse("cnf.composition.minimal_event.v1").is_ok());
        assert!(CorpusKey::parse("cnf.set.bp-10").is_ok());
        assert!(CorpusKey::parse("Cnf.Upper").is_err());
        assert!(CorpusKey::parse("cnf..double_dot").is_err());
    }
}
