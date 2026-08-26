// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Serde helpers: ordered maps as `Vec<(K, V)>` (authored key order is part
//! of an artifact's meaning — flow `with:` payload order, capture order —
//! and `serde_json`'s `preserve_order` feature carries it through the YAML
//! front-end).

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::de::{DeserializeOwned, Error as DeError, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// Deserialize a mapping into `Vec<(K, V)>`, rejecting duplicate keys.
///
/// # Errors
/// Fails on a non-mapping value, an unparsable key, or a duplicate key.
pub(crate) fn ordered_map<'de, D, K, V>(deserializer: D) -> Result<Vec<(K, V)>, D::Error>
where
    D: Deserializer<'de>,
    K: FromStr + PartialEq + fmt::Display,
    K::Err: fmt::Display,
    V: DeserializeOwned,
{
    struct MapVisitor<K, V>(PhantomData<(K, V)>);

    impl<'de, K, V> Visitor<'de> for MapVisitor<K, V>
    where
        K: FromStr + PartialEq + fmt::Display,
        K::Err: fmt::Display,
        V: DeserializeOwned,
    {
        type Value = Vec<(K, V)>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a mapping")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut entries: Vec<(K, V)> = Vec::new();
            while let Some((key, value)) = access.next_entry::<String, V>()? {
                let key = K::from_str(&key).map_err(A::Error::custom)?;
                if entries.iter().any(|(k, _)| *k == key) {
                    return Err(A::Error::custom(format!("duplicate key {key}")));
                }
                entries.push((key, value));
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_map(MapVisitor(PhantomData))
}

/// [`ordered_map`] wrapped in `Option` for optional mapping fields.
///
/// # Errors
/// As [`ordered_map`].
pub(crate) fn optional_ordered_map<'de, D, K, V>(
    deserializer: D,
) -> Result<Option<Vec<(K, V)>>, D::Error>
where
    D: Deserializer<'de>,
    K: FromStr + PartialEq + fmt::Display,
    K::Err: fmt::Display,
    V: DeserializeOwned,
{
    struct Wrapper<K, V>(Vec<(K, V)>);
    impl<'de, K, V> Deserialize<'de> for Wrapper<K, V>
    where
        K: FromStr + PartialEq + fmt::Display,
        K::Err: fmt::Display,
        V: DeserializeOwned,
    {
        fn deserialize<D2: Deserializer<'de>>(deserializer: D2) -> Result<Self, D2::Error> {
            ordered_map(deserializer).map(Wrapper)
        }
    }
    Ok(Option::<Wrapper<K, V>>::deserialize(deserializer)?.map(|w| w.0))
}
