// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The licence: what a licence token holds, how the server verifies it, and
//! the identifier stamp that records which licence was in force when a
//! record was written.
//!
//! `FerroEHR` is BUSL-1.1. Every build embeds the licensor's `non-commercial`
//! grant, what the licence gives everyone; production use in the course of a
//! business needs a `commercial` grant, installed through `[licence] file`.
//! The server behaves identically under either. Nothing here gates a
//! feature, an endpoint, a limit or a message; the only effect of the licence
//! in force is which [`stamp::StampKey`] the server mints identifiers with,
//! so that production data says which grant it was written under.
//!
//! - [`document`]: the licence document, six fields of canonical JSON.
//! - [`token`]: the file the licensee installs, a cleartext-signed licence
//!   followed by the issuer's public certificate (RFC 9580 §7 and §10.1).
//! - [`verify`]: the chain check against the embedded primary key.
//! - [`stamp`]: sixteen keyed bits in the last two bytes of every
//!   server-minted `UUIDv7`.
//! - [`config`] and [`state`]: the `[licence]` section and the boot outcome.
//!
//! No openEHR spec governs licensing or identifier entropy beyond RFC 9562's
//! layout, which the stamp respects. Our own design.

pub mod config;
pub mod document;
pub mod stamp;
pub mod state;
pub mod token;
pub mod verify;

use pgp::composed::{Deserializable as _, SignedPublicKey};
use pgp::packet::PublicKey;

/// The licensor's public certificates the server trusts, armored, embedded at
/// build time. Only their primary key packets are trust anchors; signing
/// subkeys arrive with each token.
///
/// Empty until the licensor's key ceremony has produced a certificate, in
/// which case every token is refused as `UntrustedPrimary` and the server
/// runs with no licence.
pub const ANCHOR_CERTIFICATES: &[&str] = &[];

/// The licence token every build carries: the licensor's `non-commercial` grant.
///
/// A deployment without its own token runs under this explicit, signed
/// licence and stamps identifiers with its key; a configured `[licence] file`
/// takes precedence.
///
/// Empty until the licensor has minted it; a build then runs with the
/// fail-safe stamp and no licence. Once present, a unit test proves it verifies
/// against [`ANCHOR_CERTIFICATES`], so a release can never ship a token its
/// own verifier refuses.
pub const EMBEDDED_TOKEN: &str = "";

/// An embedded anchor certificate did not parse.
#[derive(Debug, thiserror::Error)]
#[error("embedded licence anchor {index} is not an armored public certificate")]
pub struct AnchorError {
    /// Position in [`ANCHOR_CERTIFICATES`].
    pub index: usize,
    /// The parse failure.
    #[source]
    pub source: pgp::errors::Error,
}

/// The primary key packets of every embedded anchor certificate.
///
/// # Errors
/// [`AnchorError`] naming the first certificate that does not parse; a
/// build-time constant, so a unit test keeps this unreachable in a release.
pub fn anchors() -> Result<Vec<PublicKey>, AnchorError> {
    ANCHOR_CERTIFICATES
        .iter()
        .enumerate()
        .map(|(index, armored)| {
            SignedPublicKey::from_string(armored)
                .map(|(certificate, _headers)| certificate.primary_key)
                .map_err(|source| AnchorError { index, source })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_anchor_parses() {
        let anchors = anchors().expect("embedded anchors parse");
        assert_eq!(anchors.len(), ANCHOR_CERTIFICATES.len());
    }

    /// A build never ships an embedded token its own verifier refuses, and the
    /// embedded grant is the non-commercial one.
    #[test]
    fn the_embedded_token_verifies_as_non_commercial_when_present() {
        if EMBEDDED_TOKEN.is_empty() {
            return;
        }
        let anchors = anchors().expect("embedded anchors parse");
        let token = token::Token::parse(EMBEDDED_TOKEN).expect("embedded token parses");
        let today = jiff::Timestamp::now()
            .to_zoned(jiff::tz::TimeZone::UTC)
            .date();
        let verified = verify::verify(&token, &anchors, today).expect("embedded token verifies");
        assert_eq!(verified.licence.permitted_use, document::Use::NonCommercial);
    }
}
