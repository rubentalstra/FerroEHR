//! The two first-class benchmark targets. Boot/teardown stays in the compose
//! wrappers (`docker/benchmark/` for the dual stack) — the driver itself is
//! pure HTTP against a URL. Absorbed from the retired ECC harness.

use crate::sutclient::descriptor::{SutDescriptor, SutKind};

/// ehrbase-rs behind the benchmark compose stack.
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
        product_label: format!("ehrbase-rs {}", env!("CARGO_PKG_VERSION")),
    }
}

/// Upstream `EHRbase` (Java, the official image). Foreign: results are data.
/// Admin is a sibling mount (`…/rest/admin`) on the same host as the openEHR
/// base.
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
        product_label: format!("EHRbase {version_label}"),
    }
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
    fn ours_is_named_and_labelled() {
        let ours = ehrbase_rs("http://x/v1".to_owned(), None, None);
        assert_eq!(ours.name, "ehrbase-rs");
        assert_eq!(ours.kind, SutKind::Ours);
        assert!(ours.product_label.starts_with("ehrbase-rs "));
    }
}
