//! The two first-class targets. Boot/teardown stays in the compose wrappers
//! (`scripts/conformance.sh` for ours; the upstream stack reuses the
//! `docker/benchmark/` dual-stack definitions) — the runner itself is pure
//! HTTP against a URL, per the external-only owner ruling.

use crate::edition::{Edition, EditionPolicy};
use crate::sut::descriptor::{SutDescriptor, SutKind};

/// ehrbase-rs behind the `docker/conformance/` compose stack. Edition is
/// PINNED to development: the ladder must never mask a wire regression in
/// our own server (register 90 §4).
#[must_use]
pub fn ehrbase_rs(
    base_url: String,
    auth: Option<String>,
    admin_auth: Option<String>,
) -> SutDescriptor {
    SutDescriptor {
        name: "ehrbase-rs".to_owned(),
        kind: SutKind::Ours,
        base_url,
        admin_base_url: None,
        auth,
        admin_auth,
        edition_policy: EditionPolicy::Pinned(Edition::Development),
        product_label: format!("ehrbase-rs {}", env!("CARGO_PKG_VERSION")),
    }
}

/// Upstream EHRbase (Java, the official image). Foreign: fairness register
/// applies, results are data, no Certificate. Admin is a sibling mount
/// (`…/rest/admin`) on the same host as the openEHR base.
#[must_use]
pub fn ehrbase_java(
    base_url: String,
    auth: Option<String>,
    admin_auth: Option<String>,
    version_label: &str,
) -> SutDescriptor {
    let admin_base_url = base_url
        .split_once("/rest/openehr")
        .map(|(host, _)| format!("{host}/rest/admin"));
    SutDescriptor {
        name: "ehrbase-java".to_owned(),
        kind: SutKind::Foreign,
        base_url,
        admin_base_url,
        auth,
        admin_auth,
        edition_policy: EditionPolicy::Auto,
        product_label: format!("EHRbase {version_label}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_admin_mount_is_a_sibling() {
        let java = ehrbase_java(
            "http://localhost:8081/ehrbase/rest/openehr/v1".to_owned(),
            None,
            None,
            "2.34.0",
        );
        assert_eq!(
            java.admin_base_url.as_deref(),
            Some("http://localhost:8081/ehrbase/rest/admin")
        );
    }

    #[test]
    fn ours_is_pinned_to_development() {
        let ours = ehrbase_rs("http://x/v1".to_owned(), None, None);
        assert_eq!(ours.pinned_edition(), Some(Edition::Development));
    }
}
