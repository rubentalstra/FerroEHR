//! The reserved-namespace allowlist, the section registry, the two permanent
//! conventional aliases, and the list-typed key registry
//! (`docs/design/configuration.md` §4/§5.7). No openEHR spec governs
//! configuration — our own design.
//!
//! There is deliberately NO legacy-variable remapping here (greenfield, owner
//! ruling 2026-07-15): a pre-redesign spelling is an unknown variable and
//! fails at boot with the exact uniform suggestion (`strict`), never a
//! silently-honoured alias.

/// Reserved-namespace names that are NOT configuration keys and must pass the
/// strict env sweep untouched: the config-file pointer, the healthcheck arg,
/// the build-time vars, and the compose/infra parameterization that can leak
/// into the container environment. These keep their historical single-`_`
/// spelling by design — `EHRBASE_CONFIG` is a file pointer, not a config key,
/// so it never joins the uniform `EHRBASE__` grammar.
pub const ALLOWLIST: &[&str] = &[
    "EHRBASE_CONFIG",
    "EHRBASE_HEALTHCHECK_URL",
    "EHRBASE_GIT_SHA",
    "EHRBASE_BUILD_EPOCH",
    "EHRBASE_RUSTC",
    "EHRBASE_IMAGE",
    "EHRBASE_POSTGRES_IMAGE",
    "EHRBASE_PORT",
    "EHRBASE_DB_PORT",
    "EHRBASE_S3_PORT",
];

/// The eighteen top-level section names — the did-you-mean candidate set for an
/// unknown `EHRBASE__<SECTION>__…` variable.
pub const SECTIONS: &[&str] = &[
    "server",
    "db",
    "log",
    "telemetry",
    "auth",
    "authz",
    "admin",
    "tenancy",
    "smart",
    "management",
    "signing",
    "query",
    "events",
    "fhir",
    "terminology",
    "multimedia",
    "atna",
    "subject_proxy",
];

/// The two PERMANENT conventional aliases (§P-3) — 12-factor ecosystem names,
/// not legacy: they sit BELOW their `EHRBASE__` forms within the env layer.
/// `(external_name, canonical EHRBASE__ env form)`.
pub const CONVENTIONAL: &[(&str, &str)] = &[
    ("DATABASE_URL", "EHRBASE__DB__URL"),
    ("RUST_LOG", "EHRBASE__LOG__FILTER"),
];

/// List-typed key paths — env values for these are comma-separated
/// (`config`'s `with_list_parse_key`), so a scalar value containing a comma is
/// never mis-split.
pub const LIST_KEYS: &[&str] = &[
    "auth.oidc.audiences",
    "auth.oidc.algorithms",
    "authz.rbac.role_claims",
    "smart.endpoints.token_endpoint_auth_methods_supported",
    "smart.endpoints.grant_types_supported",
    "smart.endpoints.response_types_supported",
    "smart.endpoints.code_challenge_methods_supported",
    "smart.endpoints.scopes_supported",
];

/// The `EHRBASE__<SECTION>__<TAIL>` env name for a dotted config key path —
/// the P-4 grammar in the file→env direction, pinned by the tests below.
/// Test-only for now: the deserialize-error enrichment reports serde leaf
/// fields (not full paths), so it cannot reconstruct env provenance yet.
#[cfg(test)]
#[must_use]
pub fn env_name_for(key_path: &str) -> String {
    let tail = key_path
        .split('.')
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>()
        .join("__");
    format!("EHRBASE__{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_name_reconstruction() {
        assert_eq!(
            env_name_for("db.max_connections"),
            "EHRBASE__DB__MAX_CONNECTIONS"
        );
        assert_eq!(
            env_name_for("auth.oidc.issuer"),
            "EHRBASE__AUTH__OIDC__ISSUER"
        );
    }
}
