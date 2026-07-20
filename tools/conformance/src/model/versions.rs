//! Spec-version awareness (design v4): the framework models *which versions
//! of which specifications* a run is asserted against, as a first-class
//! dimension.
//!
//! Today exactly one set is supported — the latest published version of each
//! pinned specification ([`SpecVersions::LATEST`]). The dimension exists so
//! that supporting an older or newer set later (e.g. an RM 1.1.0 SUT, or the
//! next ITS-REST release) is a matter of adding a set and letting cases
//! declare applicability — not a framework rewrite.

use serde::{Deserialize, Serialize};

/// The specification versions a conformance run asserts against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecVersions {
    /// The openEHR Reference Model version (e.g. `"1.2.0"`).
    pub rm: String,
    /// The tested ITS-REST contract identity (e.g. `"Release-1.1.0"`),
    /// derived from the vendored `-codegen` OAS provenance, not the
    /// released spec-text version.
    pub its_rest: String,
    /// The AQL (QUERY) specification version (e.g. `"1.1.0"`).
    pub aql: String,
    /// The terminology (TERM) version (e.g. `"3.1.0"`).
    pub term: String,
}

impl SpecVersions {
    /// The latest published set — the only set supported today (pins in
    /// `docs/VERSIONS.md`).
    ///
    /// `its_rest` is **not** a hand-asserted literal: it is derived from
    /// the vendored `-codegen` OAS provenance
    /// ([`crate::model::provenance::tested_its_rest`]) so the report claims
    /// exactly the ITS-REST contract the SUT implements — the pinned release
    /// tag (`Release-1.1.0`) or, for a pre-release pin, `development@<commit>`.
    #[must_use]
    pub fn latest() -> Self {
        SpecVersions {
            rm: "1.2.0".to_owned(),
            its_rest: crate::model::provenance::tested_its_rest().to_owned(),
            aql: "1.1.0".to_owned(),
            term: "3.1.0".to_owned(),
        }
    }
}

impl Default for SpecVersions {
    fn default() -> Self {
        SpecVersions::latest()
    }
}
