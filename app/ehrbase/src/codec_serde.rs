//! serde ↔ native canonical-JSON codec bridge for this crate's own wire DTOs
//! that embed openEHR spec types.
//!
//! The `openehr-*` spec types carry no serde derive — canonical-JSON
//! (de)serialization is the native codec (`openehr_its::json_codec`, via the
//! `openehr_its::json` entry points). A wire DTO defined in this crate that
//! embeds a spec-typed field keeps that field **typed** (the real spec type,
//! never a shadow struct or an untyped `Value`) and routes only that field's
//! (de)serialization through the codec, with `#[serde(with = "…")]`. The bridge
//! meets serde at a `serde_json::Value` seam; the DTO's own derive walks the
//! rest of the object.
//!
//! Modules:
//! - [`spec`] — a single spec-typed field `T` (or `Vec<T>`).
//! - [`spec_opt`] — an `Option<T>` (or `Option<Vec<T>>`) spec-typed field.

use openehr_its::json;
use openehr_its::json_codec::runtime::{FromJson, ToJson};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// `#[serde(with = "crate::codec_serde::spec")]` for a mandatory spec-typed
/// field (`T: ToJson + FromJson`, including `Vec<T>`).
pub mod spec {
    use super::{Deserialize, Deserializer, FromJson, Serialize, Serializer, ToJson, json};

    /// # Errors
    /// Propagates the serde serializer's error.
    pub fn serialize<T: ToJson, S: Serializer>(value: &T, s: S) -> Result<S::Ok, S::Error> {
        json::to_canonical_value(value).serialize(s)
    }

    /// # Errors
    /// Returns a serde error if the value is not a valid canonical encoding of `T`.
    pub fn deserialize<'de, T: FromJson, D: Deserializer<'de>>(d: D) -> Result<T, D::Error> {
        let value = serde_json::Value::deserialize(d)?;
        json::from_canonical_value(&value).map_err(serde::de::Error::custom)
    }
}

/// `#[serde(with = "crate::codec_serde::spec_opt")]` for an optional spec-typed
/// field (`Option<T>` where `T: ToJson + FromJson`, including `Option<Vec<T>>`).
pub mod spec_opt {
    use super::{Deserialize, Deserializer, FromJson, Serialize, Serializer, ToJson, json};

    /// # Errors
    /// Propagates the serde serializer's error.
    #[expect(
        clippy::ref_option,
        reason = "serde's `#[serde(with = ...)]` contract fixes this signature: \
                  the generated code calls `serialize(&self.field, serializer)` \
                  with a reference to the whole `Option`"
    )]
    pub fn serialize<T: ToJson, S: Serializer>(value: &Option<T>, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => json::to_canonical_value(v).serialize(s),
            None => s.serialize_none(),
        }
    }

    /// # Errors
    /// Returns a serde error if a present, non-null value is not a valid
    /// canonical encoding of `T`.
    pub fn deserialize<'de, T: FromJson, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<T>, D::Error> {
        let value = Option::<serde_json::Value>::deserialize(d)?;
        match value {
            Some(v) if !v.is_null() => json::from_canonical_value(&v)
                .map(Some)
                .map_err(serde::de::Error::custom),
            _ => Ok(None),
        }
    }
}
