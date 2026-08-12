// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Representation formats the console negotiates with the CDR.
//!
//! The ITS-REST canonical forms plus the Simplified Formats media types (spec:
//! `docs/specs/openehr/ITS-REST/docs/simplified_formats/master05-rm_mapping.adoc`
//! — negotiation is strict `Accept`/`Content-Type`, no `?format=` parameter).

use serde::{Deserialize, Serialize};

/// One selectable representation of an openEHR resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReprFormat {
    /// Canonical JSON (`application/json`).
    CanonicalJson,
    /// Canonical XML (`application/xml`).
    CanonicalXml,
    /// Simplified FLAT (`application/openehr.wt.flat+json`).
    Flat,
    /// Simplified STRUCTURED (`application/openehr.wt.structured+json`).
    Structured,
    /// The Web Template rendering of a template (`application/openehr.wt+json`).
    WebTemplate,
}

impl ReprFormat {
    /// The exact media type sent as `Accept` / `Content-Type`.
    #[must_use]
    pub fn media_type(self) -> &'static str {
        match self {
            Self::CanonicalJson => "application/json",
            Self::CanonicalXml => "application/xml",
            Self::Flat => "application/openehr.wt.flat+json",
            Self::Structured => "application/openehr.wt.structured+json",
            Self::WebTemplate => "application/openehr.wt+json",
        }
    }

    /// Short human label for the format selector.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::CanonicalJson => "JSON",
            Self::CanonicalXml => "XML",
            Self::Flat => "FLAT",
            Self::Structured => "STRUCTURED",
            Self::WebTemplate => "WT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReprFormat;

    #[test]
    fn media_types_match_the_simplified_formats_spec() {
        // docs/specs/openehr/ITS-REST/docs/simplified_formats/master05-rm_mapping.adoc
        assert_eq!(
            ReprFormat::Flat.media_type(),
            "application/openehr.wt.flat+json"
        );
        assert_eq!(
            ReprFormat::Structured.media_type(),
            "application/openehr.wt.structured+json"
        );
        assert_eq!(
            ReprFormat::WebTemplate.media_type(),
            "application/openehr.wt+json"
        );
        assert_eq!(ReprFormat::CanonicalJson.media_type(), "application/json");
        assert_eq!(ReprFormat::CanonicalXml.media_type(), "application/xml");
    }
}
