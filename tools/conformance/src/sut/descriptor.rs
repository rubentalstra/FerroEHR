//! The SUT descriptor: everything the runner knows about a target before the
//! probe. Adding a target is a config entry (CLI flags or a built-in
//! constant), never new case code — the bring-your-own-endpoint owner ruling.

use std::path::PathBuf;

use serde::Serialize;

use crate::edition::{Edition, EditionPolicy};

/// The class of SUT. Gates ONLY the fairness-register seam (foreign SUTs
/// get the committed adjudication triage; ours never does — the zero-drift
/// guarantee). Every artefact — report, Statement, **Certificate** — is
/// emitted for every SUT (owner ruling 2026-07-13: the framework certifies
/// any openEHR CDR; the Certificate itself states it is a framework
/// self-assessment, never an official openEHR certification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SutKind {
    /// ehrbase-rs — the product this repo builds.
    Ours,
    /// Any other CDR (upstream `EHRbase`, a bring-your-own endpoint): the
    /// fairness register applies.
    Foreign,
}

/// A conformance target.
#[derive(Debug, Clone, Serialize)]
pub struct SutDescriptor {
    /// The short name used in output paths + the fairness-register lookup,
    /// e.g. `"ehrbase-rs"`, `"ehrbase-java"`, or a user-chosen BYO name.
    pub name: String,
    /// The class (gates Certificate emission + fairness-register loading).
    pub kind: SutKind,
    /// The ITS-REST base URL, e.g.
    /// `http://localhost:8080/ehrbase/rest/openehr/v1`.
    pub base_url: String,
    /// An optional sibling admin mount. Upstream `EHRbase` serves the admin
    /// API beside `/rest/openehr` (`…/rest/admin`); ours nests it under the
    /// openEHR base (then `None`).
    pub admin_base_url: Option<String>,
    /// Regular-credential spec (`basic:<u>:<p>` / `bearer:<t>`), if any.
    #[serde(skip)]
    pub auth: Option<String>,
    /// Admin-credential spec, if any.
    #[serde(skip)]
    pub admin_auth: Option<String>,
    /// The edition policy: pinned for our CI (the ladder must never mask
    /// drift in ehrbase-rs), auto for foreign/BYO targets.
    pub edition_policy: EditionPolicy,
    /// The product/version label recorded in results + the Statement (e.g.
    /// `"ehrbase-rs <workspace version>"`, `"EHRbase 2.34.0"`, or the BYO
    /// operator's own label).
    pub product_label: String,
    /// A sibling SUT instance booted in `[signing] mode = "pgp"`, for the
    /// pgp-signing case (`ECC-SIG-005`). The main SUT signs in `digest` mode
    /// (the four digest cases assert `sha256:`); a single server cannot serve
    /// both signature schemes, so the composed run stands up a second,
    /// pgp-keyed instance and names its ITS-REST base URL here. `None`: the pgp
    /// case reports `SKIPPED(SutConfig)` (its off-wire evidence stands).
    #[serde(skip)]
    pub sig_pgp_base_url: Option<String>,
    /// The armored RFC 4880 secret key the pgp-keyed sibling was booted with
    /// (the committed TEST-ONLY key). The pgp case derives the public key from
    /// it to verify the served detached signature. `None` when
    /// [`Self::sig_pgp_base_url`] is unset.
    #[serde(skip)]
    pub sig_pgp_key_path: Option<PathBuf>,
}

impl SutDescriptor {
    /// A bring-your-own-endpoint target: `Foreign`, edition `Auto`, named by
    /// the operator (defaults to `"byo"`).
    #[must_use]
    pub fn byo(name: Option<String>, base_url: String) -> Self {
        let name = name.unwrap_or_else(|| "byo".to_owned());
        SutDescriptor {
            product_label: name.clone(),
            name,
            kind: SutKind::Foreign,
            base_url,
            admin_base_url: None,
            auth: None,
            admin_auth: None,
            edition_policy: EditionPolicy::Auto,
            sig_pgp_base_url: None,
            sig_pgp_key_path: None,
        }
    }

    /// The edition the zero-drift gate pins for this SUT, when pinned.
    #[must_use]
    pub fn pinned_edition(&self) -> Option<Edition> {
        match self.edition_policy {
            EditionPolicy::Pinned(e) => Some(e),
            EditionPolicy::Auto => None,
        }
    }
}
