//! Canonical-JSON serde helpers shared across the RM classes.
//!
//! ITS-JSON serializes raw binary RM attributes (`DV_MULTIMEDIA.data`,
//! `DV_MULTIMEDIA.integrity_check`) as inline base64 strings
//! (`.claude/rules/serialization.md`). These `#[serde(with = ...)]` modules
//! encode/decode with the standard alphabet, padded — matching Jackson's
//! default `byte[]` handling in EHRbase.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Deserializer, Serializer};

/// `#[serde(with = "crate::serde_support::base64_vec")]` for `Vec<u8>`.
pub mod base64_vec {
    use super::{Deserialize, Deserializer, Engine, STANDARD, Serializer};

    /// Serializes the byte buffer as a base64 string.
    ///
    /// # Errors
    ///
    /// Only the serializer's own errors; encoding itself cannot fail.
    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    /// Deserializes a base64 string into a byte buffer.
    ///
    /// # Errors
    ///
    /// The deserializer's own errors, or invalid base64 content.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let encoded = String::deserialize(deserializer)?;
        STANDARD
            .decode(encoded.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

/// `#[serde(with = "crate::serde_support::base64_option")]` for
/// `Option<Vec<u8>>`. Combine with
/// `#[serde(skip_serializing_if = "Option::is_none", default)]` so an absent
/// value is omitted entirely (nulls are never emitted in canonical JSON).
pub mod base64_option {
    use super::{Deserialize, Deserializer, Engine, STANDARD, Serializer};

    /// Serializes `Some(bytes)` as a base64 string; `None` only occurs when
    /// the field skipped `skip_serializing_if`, and serializes as null.
    ///
    /// # Errors
    ///
    /// Only the serializer's own errors.
    pub fn serialize<S: Serializer>(
        bytes: &Option<Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match bytes {
            Some(b) => serializer.serialize_str(&STANDARD.encode(b)),
            None => serializer.serialize_none(),
        }
    }

    /// Deserializes an optional base64 string.
    ///
    /// # Errors
    ///
    /// The deserializer's own errors, or invalid base64 content.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<u8>>, D::Error> {
        let encoded: Option<String> = Option::deserialize(deserializer)?;
        encoded
            .map(|s| {
                STANDARD
                    .decode(s.as_bytes())
                    .map_err(serde::de::Error::custom)
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Probe {
        #[serde(with = "crate::serde_support::base64_vec")]
        data: Vec<u8>,
        #[serde(
            with = "crate::serde_support::base64_option",
            skip_serializing_if = "Option::is_none",
            default
        )]
        thumb: Option<Vec<u8>>,
    }

    #[test]
    fn round_trips_and_omits_none() {
        let probe = Probe {
            data: vec![1, 2, 254],
            thumb: None,
        };
        let json = serde_json::to_string(&probe).expect("serialize");
        assert_eq!(json, r#"{"data":"AQL+"}"#);
        let back: Probe = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, probe);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ITS-JSON canonical rules (inline base64 DV_MULTIMEDIA.data) — .claude/rules/serialization.md; ITS-JSON @ 5acae05
//   source_loc: n/a (serialization infrastructure, no spec class)
//   confidence: high
//   todos: 0
//   note: STANDARD padded alphabet matches Jackson's byte[] default in EHRbase; used via serde(with) from data_types::encapsulated
// ─────────────────────────────────────────────
