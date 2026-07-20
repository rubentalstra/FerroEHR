//! Vendored ITS-REST provenance — the *tested* contract identity.
//!
//! The framework must claim exactly the ITS-REST contract it actually tests.
//! The SUT implements the contract generated (`emit-rest`) from the
//! vendored `-codegen` OAS tree at `crates/openehr-its/vendor/rest-oas/`;
//! the separately vendored spec **text** at `docs/specs/openehr/ITS-REST/`
//! is the source of the per-case `§`-section citations. Both trees are
//! pinned to the SAME upstream commit — since 2026-07-20 the released tag
//! `Release-1.1.0` (published upstream 19-Jul-2026).
//!
//! Owner ruling: the tested ITS-REST identity is DERIVED from the vendored
//! `-codegen` tree's `PROVENANCE.md` at build time (`include_str!`), never a
//! hand-asserted version literal. A pin at a released `Release-X.Y.Z` tag
//! yields that tag as the identity; a pre-release pin yields
//! `development@<commit>`. The reconciliation guard ([`tests`]) fails with a
//! triage message if the two vendored trees ever drift beyond the sanctioned
//! "released spec-text lagging a development OAS" arrangement.

use std::sync::LazyLock;

/// The vendored `-codegen` OAS provenance — the contract the SUT implements.
const REST_OAS_PROVENANCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/openehr-its/vendor/rest-oas/PROVENANCE.md"
));

/// The vendored ITS-REST spec-text provenance — the `§`-citation source.
const DOCS_ITS_REST_PROVENANCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/specs/openehr/ITS-REST/PROVENANCE.md"
));

/// A parsed vendored `PROVENANCE.md` record (the fields we assert on).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The upstream repository slug, e.g. `openEHR/specifications-ITS-REST`.
    pub repo: String,
    /// The pinned commit SHA (full 40-hex).
    pub commit: String,
    /// The pinned reference/branch, if the file names one (`Release-1.0.3`,
    /// `master`).
    pub reference: Option<String>,
}

impl Provenance {
    /// The abbreviated (7-char) commit for human-facing identity strings.
    #[must_use]
    pub fn short_commit(&self) -> &str {
        &self.commit[..self.commit.len().min(7)]
    }
}

/// Extract the first backtick-delimited long-hex token (the commit SHA).
fn find_commit(text: &str) -> Option<String> {
    text.split('`').find_map(|tok| {
        let is_hex = tok.len() >= 7 && tok.bytes().all(|b| b.is_ascii_hexdigit());
        is_hex.then(|| tok.to_ascii_lowercase())
    })
}

/// Parse a vendored ITS-REST `PROVENANCE.md`. Returns `None` if no commit SHA
/// is present (the file shape changed — the guard test will flag it).
#[must_use]
pub fn parse(text: &str) -> Option<Provenance> {
    let commit = find_commit(text)?;
    let repo = text
        .contains("specifications-ITS-REST")
        .then(|| "openEHR/specifications-ITS-REST".to_owned())?;
    // A pinned release tag (`Release-X.Y.Z`) wins; otherwise a branch name.
    let reference = text
        .split(|c: char| c.is_whitespace() || "()`,;".contains(c))
        .find(|tok| {
            tok.strip_prefix("Release-").is_some_and(|v| {
                v.split('.').count() == 3 && v.split('.').all(|p| p.chars().all(char::is_numeric))
            })
        })
        .map(str::to_owned)
        .or_else(|| {
            (text.contains("(master)") || text.contains("master")).then(|| "master".to_owned())
        });
    Some(Provenance {
        repo,
        commit,
        reference,
    })
}

/// The vendored `-codegen` OAS provenance (the tested contract), or `None` if
/// the vendored `rest-oas/PROVENANCE.md` no longer names a commit — a shape
/// change the reconciliation guard test flags in CI.
#[must_use]
pub fn rest_oas() -> Option<&'static Provenance> {
    static P: LazyLock<Option<Provenance>> = LazyLock::new(|| parse(REST_OAS_PROVENANCE));
    P.as_ref()
}

/// The vendored ITS-REST spec-text provenance (the `§`-citation source), or
/// `None` if its `PROVENANCE.md` no longer names a commit (see [`rest_oas`]).
#[must_use]
pub fn docs_its_rest() -> Option<&'static Provenance> {
    static P: LazyLock<Option<Provenance>> = LazyLock::new(|| parse(DOCS_ITS_REST_PROVENANCE));
    P.as_ref()
}

/// The tested ITS-REST identity string for `SpecVersions.its_rest` and the
/// report header — derived from the vendored `-codegen` OAS provenance, never
/// a hand-asserted literal. A pin at a released tag yields that tag
/// (`Release-1.1.0`); a pre-release pin yields `development@<short-commit>`.
/// Falls back to `development@unknown` only if the vendored provenance is
/// unparseable — a state the guard test fails on before it can reach a run.
#[must_use]
pub fn tested_its_rest() -> &'static str {
    static ID: LazyLock<String> = LazyLock::new(|| {
        rest_oas().map_or_else(
            || "development@unknown".to_owned(),
            |p| match p.reference.as_deref() {
                Some(tag) if tag.starts_with("Release-") => tag.to_owned(),
                _ => format!("development@{}", p.short_commit()),
            },
        )
    });
    &ID
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn parses_the_rest_oas_provenance() {
        let p = rest_oas().expect("rest-oas/PROVENANCE.md must parse");
        assert_eq!(p.repo, "openEHR/specifications-ITS-REST");
        assert_eq!(p.commit.len(), 40, "expected a full 40-hex commit SHA");
        assert!(
            p.commit.bytes().all(|b| b.is_ascii_hexdigit()),
            "commit must be hex"
        );
    }

    #[test]
    fn tested_identity_is_derived_from_provenance_not_a_literal() {
        // The report must claim exactly what the vendored pin IS: the release
        // tag when pinned at one, else `development@<commit>` — never a bare
        // hand-asserted version number.
        let id = tested_its_rest();
        assert_ne!(id, "1.0.3", "its_rest must not be a hand-asserted literal");
        assert_ne!(id, "1.1.0", "its_rest must not be a hand-asserted literal");
        let oas = rest_oas().expect("rest-oas/PROVENANCE.md must parse");
        match oas.reference.as_deref() {
            Some(tag) if tag.starts_with("Release-") => {
                assert_eq!(id, tag, "a release-tag pin yields the tag as identity");
            }
            _ => {
                assert!(
                    id.starts_with("development@") && id.ends_with(oas.short_commit()),
                    "a pre-release pin yields development@<commit>: {id}"
                );
            }
        }
    }

    /// The reconciliation guard the owner asked for: the two vendored
    /// ITS-REST trees must reference the same upstream repo, and either be the
    /// same commit (fully reconciled) or the sanctioned "development OAS +
    /// released spec-text" arrangement. Any other divergence fails with a
    /// triage message so a one-sided re-vendor cannot silently make the report
    /// dishonest.
    #[test]
    fn its_rest_trees_are_reconciled_or_a_documented_release_lag() {
        let oas = rest_oas().expect("rest-oas/PROVENANCE.md must parse");
        let docs = docs_its_rest().expect("docs ITS-REST/PROVENANCE.md must parse");
        assert_eq!(
            oas.repo, docs.repo,
            "the two vendored ITS-REST trees name different repos ({} vs {}) — reconcile them",
            oas.repo, docs.repo
        );
        if oas.commit == docs.commit {
            return; // fully reconciled: both trees at one ref (the ideal end state).
        }
        assert!(
            docs.reference
                .as_deref()
                .is_some_and(|r| r.starts_with("Release-")),
            "ITS-REST vendoring drift (triage): the tested -codegen OAS is pinned to \
             {oas_ref}@{oas_short} (owner ruling: the OAS tree is the tested identity) \
             but the spec-text tree is at {docs_ref:?}@{docs_short}, which is NOT a \
             released tag. The only sanctioned divergence is a released spec-text tree \
             (Release-X.Y.Z) lagging a development OAS. Reconcile by re-vendoring both \
             to one ref (scripts/vendor-spec-docs.sh) or update this guard.",
            oas_ref = oas.reference.as_deref().unwrap_or("?"),
            oas_short = oas.short_commit(),
            docs_ref = docs.reference,
            docs_short = docs.short_commit(),
        );
    }
}
