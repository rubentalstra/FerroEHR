//! The SUT descriptor: everything the benchmark knows about a target before it
//! starts driving load. Adding a target is a config entry (CLI flags or a
//! built-in constant), never new driver code — the bring-your-own-endpoint
//! design. Absorbed from the retired ECC harness and pruned to the fields the
//! benchmark uses (the conformance edition ladder and signature-scheme wiring
//! are ECC concerns, dropped here).

/// The class of SUT. The benchmark labels output and picks the reproduce
/// command by this; every artefact is emitted for every SUT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SutKind {
    /// ehrbase-rs — the product this repo builds.
    Ours,
    /// Any other CDR (upstream `EHRbase`, a bring-your-own endpoint).
    Foreign,
}

/// A benchmark target.
#[derive(Debug, Clone)]
pub struct SutDescriptor {
    /// The short name used in output paths, e.g. `"ehrbase-rs"`,
    /// `"ehrbase-java"`, or a user-chosen BYO name.
    pub name: String,
    /// The class (drives output labelling + the reproduce command).
    pub kind: SutKind,
    /// The ITS-REST base URL, e.g.
    /// `http://localhost:8080/ehrbase/rest/openehr/v1`.
    pub base_url: String,
    /// An optional sibling admin mount. Upstream `EHRbase` serves the admin
    /// API beside `/rest/openehr` (`…/rest/admin`); ours nests it under the
    /// openEHR base (then `None`).
    pub admin_base_url: Option<String>,
    /// Regular-credential spec (`basic:<u>:<p>` / `bearer:<t>`), if any.
    pub auth: Option<String>,
    /// Admin-credential spec, if any.
    pub admin_auth: Option<String>,
    /// The product/version label recorded in results (e.g.
    /// `"ehrbase-rs <workspace version>"`, `"EHRbase 2.34.0"`, or the BYO
    /// operator's own label).
    pub product_label: String,
}

impl SutDescriptor {
    /// A bring-your-own-endpoint target: `Foreign`, named by the operator
    /// (defaults to `"byo"`).
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
        }
    }
}
