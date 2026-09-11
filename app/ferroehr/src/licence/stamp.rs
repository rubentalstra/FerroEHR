// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The identifier stamp: sixteen keyed bits inside every server-minted `UUIDv7`.
//!
//! RFC 9562 §5.7 lays a version-7 UUID out as a 48-bit millisecond timestamp,
//! the version and variant bits, and 74 bits of `rand_a` + `rand_b`. The
//! `uuid` crate fills the leading part of those bits with its monotonic
//! counter and the trailing part with CSPRNG output. The stamp replaces the
//! LAST two bytes, which are always CSPRNG output under every counter width
//! the crate offers, so ordering within a millisecond and the counter's
//! uniqueness guarantee are untouched, and 46 CSPRNG bits remain (RFC 9562
//! §6.9 asks for a CSPRNG and sets no minimum).
//!
//! The two bytes are `HMAC-SHA256(SHA-256(material), timestamp_ms)[0..2]`,
//! where `material` names the licence (or the public fail-safe constant).
//! Being a function of the identifier's own timestamp, the stamp differs on
//! every millisecond, so a set of identifiers shows no fixed pattern, yet
//! anyone holding the material recomputes it. Cost per identifier: one HMAC
//! over eight bytes with a precomputed key schedule.
//!
//! What it deliberately is not: a secret. The fail-safe material is a public
//! constant and every licence id is printed in its token. The stamp's value
//! is that removing it means rewriting every version identifier in the
//! store, and that its absence on a `FerroEHR`-shaped database proves an
//! altered build. It carries no patient data and nothing about the
//! deployment beyond the licence id.

use std::fmt;

use hmac::{Hmac, KeyInit as _, Mac as _};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

/// Key material of the fail-safe stamp: a build that embeds no usable token.
pub const FAIL_SAFE_KEY: &[u8] = b"ferroehr/no-licence/v1";
/// Domain separator prefixed to a licence id to form its stamp material.
pub const LICENCE_KEY_PREFIX: &[u8] = b"ferroehr/licence/v1/";
/// Width of the stamp, in bits.
pub const STAMP_BITS: u32 = 16;

/// The identifier is not a version-7 UUID, so it carries no timestamp to stamp.
#[derive(Debug, thiserror::Error)]
#[error("identifier {0} is not a version 7 UUID")]
pub struct NotVersion7(pub Uuid);

/// A precomputed stamp key.
///
/// Construct once, clone freely: cloning copies two SHA-256 states, and the
/// per-identifier work is one short HMAC.
#[derive(Clone)]
pub struct StampKey {
    mac: Hmac<Sha256>,
}

impl fmt::Debug for StampKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StampKey").finish_non_exhaustive()
    }
}

impl StampKey {
    /// A key over arbitrary material.
    ///
    /// The HMAC key is `SHA-256(material)`, zero-padded to the block size as
    /// HMAC itself defines, so construction cannot fail and any implementation
    /// that keys HMAC-SHA256 with the 32-byte digest computes the same stamp.
    #[must_use]
    pub fn new(material: &[u8]) -> Self {
        let digest = Sha256::digest(material);
        let mut key = hmac::digest::Key::<Hmac<Sha256>>::default();
        for (slot, byte) in key.iter_mut().zip(digest.iter()) {
            *slot = *byte;
        }
        Self {
            mac: Hmac::<Sha256>::new(&key),
        }
    }

    /// The key a build with no usable embedded token mints with; unreachable once one is embedded.
    #[must_use]
    pub fn fail_safe() -> Self {
        Self::new(FAIL_SAFE_KEY)
    }

    /// The key a deployment holding the licence `licence_id` mints with.
    #[must_use]
    pub fn for_licence(licence_id: Uuid) -> Self {
        let mut material = Vec::with_capacity(LICENCE_KEY_PREFIX.len() + 16);
        material.extend_from_slice(LICENCE_KEY_PREFIX);
        material.extend_from_slice(licence_id.as_bytes());
        Self::new(&material)
    }

    /// The two stamp bytes for an identifier minted at `timestamp_ms`.
    #[must_use]
    pub fn stamp16(&self, timestamp_ms: u64) -> [u8; 2] {
        let mut mac = self.mac.clone();
        mac.update(&timestamp_ms.to_be_bytes());
        let tag: [u8; 32] = mac.finalize().into_bytes().into();
        [tag[0], tag[1]]
    }

    /// A fresh, stamped, time-ordered identifier.
    #[must_use]
    pub fn mint(&self) -> Uuid {
        self.stamp_unchecked(Uuid::now_v7())
    }

    /// Stamp a version-7 identifier minted elsewhere.
    ///
    /// # Errors
    /// [`NotVersion7`] for any other UUID version; those carry no timestamp
    /// and are never stamped.
    pub fn stamp(&self, id: Uuid) -> Result<Uuid, NotVersion7> {
        if id.get_version_num() != 7 {
            return Err(NotVersion7(id));
        }
        Ok(self.stamp_unchecked(id))
    }

    /// Whether `id` carries this key's stamp. Always `false` for non-v7 ids.
    #[must_use]
    pub fn carries(&self, id: Uuid) -> bool {
        let Some(ts) = timestamp_ms(id) else {
            return false;
        };
        let bytes = id.as_bytes();
        self.stamp16(ts) == [bytes[14], bytes[15]]
    }

    fn stamp_unchecked(&self, id: Uuid) -> Uuid {
        let tag = self.stamp16(leading_48_bits(id));
        let mut bytes = id.into_bytes();
        bytes[14] = tag[0];
        bytes[15] = tag[1];
        Uuid::from_bytes(bytes)
    }
}

/// The millisecond Unix timestamp of a version-7 UUID, `None` otherwise.
#[must_use]
pub fn timestamp_ms(id: Uuid) -> Option<u64> {
    (id.get_version_num() == 7).then(|| leading_48_bits(id))
}

fn leading_48_bits(id: Uuid) -> u64 {
    let b = id.as_bytes();
    u64::from_be_bytes([0, 0, b[0], b[1], b[2], b[3], b[4], b[5]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minted_id_is_v7_and_carries_its_key() {
        let key = StampKey::fail_safe();
        let id = key.mint();
        assert_eq!(id.get_version_num(), 7);
        assert!(key.carries(id));
    }

    #[test]
    fn a_stamped_id_keeps_everything_but_its_last_two_bytes() {
        let key = StampKey::fail_safe();
        let fresh = Uuid::now_v7();
        let stamped = key.stamp(fresh).unwrap();
        assert_eq!(&fresh.as_bytes()[..14], &stamped.as_bytes()[..14]);
        assert_eq!(timestamp_ms(stamped), timestamp_ms(fresh));
        assert!(key.carries(stamped));
    }

    #[test]
    fn keys_do_not_recognise_each_other() {
        let fail_safe = StampKey::fail_safe();
        let licensed = StampKey::for_licence(Uuid::now_v7());
        let id = licensed.mint();
        assert!(licensed.carries(id));
        // One chance in 65536; a flake here is a real defect, not noise.
        assert!(!fail_safe.carries(id));
    }

    #[test]
    fn the_stamp_varies_with_the_millisecond() {
        let key = StampKey::fail_safe();
        assert_ne!(key.stamp16(1_000), key.stamp16(1_001));
        assert_eq!(key.stamp16(1_000), key.stamp16(1_000));
    }

    #[test]
    fn the_key_is_hmac_over_the_sha256_of_the_material() {
        // The definition an independent implementation must match.
        let mut reference = Hmac::<Sha256>::new_from_slice(&Sha256::digest(FAIL_SAFE_KEY)).unwrap();
        reference.update(&1_000u64.to_be_bytes());
        let tag: [u8; 32] = reference.finalize().into_bytes().into();
        assert_eq!(StampKey::fail_safe().stamp16(1_000), [tag[0], tag[1]]);
    }

    #[test]
    fn non_v7_ids_are_refused_and_never_match() {
        let key = StampKey::fail_safe();
        let v4 = Uuid::from_u128(0x1234_5678_9abc_4def_8000_0000_0000_0000);
        assert!(matches!(key.stamp(v4), Err(NotVersion7(_))));
        assert!(!key.carries(v4));
        assert_eq!(timestamp_ms(v4), None);
    }

    #[test]
    fn timestamp_is_the_leading_48_bits() {
        let id = Uuid::from_u128(0x0192_a7e1_2345_7000_8000_0000_0000_0000);
        assert_eq!(timestamp_ms(id), Some(0x0192_a7e1_2345));
    }
}
