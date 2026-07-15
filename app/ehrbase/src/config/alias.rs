//! Transition aliases, retired `*_CONFIG` pointers, the reserved-namespace
//! allowlist, and the list-typed key registry (`docs/design/configuration.md`
//! §4/§5.7). No openEHR spec governs configuration — our own design.

/// Reserved-namespace names that are NOT configuration keys and must pass the
/// strict env sweep untouched: the config-file pointer, the healthcheck arg,
/// the build-time vars, and the compose/infra parameterization that can leak
/// into the container environment.
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
/// unknown `EHRBASE_<SECTION>__…` variable.
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

/// Retired `EHRBASE_*_CONFIG` file pointers + the env-unsettable Basic store:
/// each dies with a bespoke migration message (§4 "dies").
pub const DIES: &[(&str, &str)] = &[
    (
        "EHRBASE_REST_CONFIG",
        "the per-subsystem config files are gone; merge that file's contents into ehrbase.toml \
         (discovered via --config / EHRBASE_CONFIG / ./ehrbase.toml / /etc/ehrbase/ehrbase.toml)",
    ),
    (
        "EHRBASE_MANAGEMENT_CONFIG",
        "merge into ehrbase.toml under [management]",
    ),
    (
        "EHRBASE_AUTHZ_CONFIG",
        "merge into ehrbase.toml under [authz]",
    ),
    (
        "EHRBASE_ATNA_CONFIG",
        "merge into ehrbase.toml under [atna]",
    ),
    (
        "EHRBASE_SIGNING_CONFIG",
        "merge into ehrbase.toml under [signing]",
    ),
    (
        "EHRBASE_EVENTS_CONFIG",
        "merge into ehrbase.toml under [events]",
    ),
    (
        "EHRBASE_FHIR_OUTBOUND_CONFIG",
        "merge into ehrbase.toml under [fhir.outbound]",
    ),
    (
        "EHRBASE_MULTIMEDIA_CONFIG",
        "merge into ehrbase.toml under [multimedia]",
    ),
    (
        "EHRBASE_VALIDATION_CONFIG",
        "merge into ehrbase.toml under [terminology.external]",
    ),
    (
        "EHRBASE_SUBJECT_PROXY_CONFIG",
        "merge into ehrbase.toml under [subject_proxy]",
    ),
    (
        "EHRBASE_REST_AUTH__BASIC__USERS",
        "define Basic users as [[auth.basic.users]] tables in ehrbase.toml (file-only)",
    ),
];

/// Conventional aliases that sit BELOW their `EHRBASE_` forms within the env
/// layer (§P-3): `(external_name, canonical EHRBASE_ env form)`.
pub const CONVENTIONAL: &[(&str, &str)] = &[
    ("DATABASE_URL", "EHRBASE_DB__URL"),
    ("RUST_LOG", "EHRBASE_LOG__FILTER"),
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

/// Map a legacy `EHRBASE_*` variable name to its canonical replacement, or
/// `None` when `old` is already a canonical (new-form) variable. Used both to
/// warn + remap set legacy vars and to recognise legacy names during the strict
/// sweep. `DIES` names are handled before this (they are not aliases).
#[must_use]
pub fn resolve_alias(old: &str) -> Option<String> {
    // Renamed sections (explicit prefixes).
    if let Some(rest) = old.strip_prefix("EHRBASE_OTEL_") {
        return double(rest).map(|r| format!("EHRBASE_TELEMETRY__{r}"));
    }
    if let Some(rest) = old.strip_prefix("EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_") {
        return double(rest).map(|r| format!("EHRBASE_TERMINOLOGY__EXTERNAL__{r}"));
    }
    if let Some(rest) = old.strip_prefix("EHRBASE_FHIR_OUTBOUND_") {
        return double(rest).map(|r| format!("EHRBASE_FHIR__OUTBOUND__{r}"));
    }
    if let Some(rest) = old.strip_prefix("EHRBASE_MANAGEMENT_ENDPOINTS_") {
        return double(rest).map(|r| format!("EHRBASE_MANAGEMENT__ENDPOINTS__{r}"));
    }
    // The REST section fans out to several top-level sections.
    if let Some(rest) = old.strip_prefix("EHRBASE_REST_") {
        return resolve_rest(rest);
    }
    // Same-name sections that changed from a single `_` to `__` separator.
    for section in [
        "DB",
        "LOG",
        "ATNA",
        "SIGNING",
        "EVENTS",
        "MULTIMEDIA",
        "MANAGEMENT",
        "AUTHZ",
    ] {
        let prefix = format!("EHRBASE_{section}_");
        if let Some(rest) = old.strip_prefix(&prefix)
            && let Some(rest) = double(rest)
        {
            return Some(format!("EHRBASE_{section}__{rest}"));
        }
    }
    None
}

/// The `EHRBASE_REST_<rest>` fan-out.
fn resolve_rest(rest: &str) -> Option<String> {
    // Nested (already-`__`) REST sub-sections.
    if let Some(r) = rest.strip_prefix("AUTH__") {
        return Some(format!("EHRBASE_AUTH__{r}"));
    }
    if let Some(r) = rest.strip_prefix("ADMIN__") {
        return Some(format!("EHRBASE_ADMIN__{r}"));
    }
    if let Some(r) = rest.strip_prefix("TENANCY__") {
        return Some(format!("EHRBASE_TENANCY__{r}"));
    }
    if let Some(r) = rest.strip_prefix("SMART__") {
        return Some(format!("EHRBASE_SMART__{r}"));
    }
    if let Some(r) = rest.strip_prefix("SYSTEM__") {
        return Some(format!("EHRBASE_SERVER__IDENTITY__{r}"));
    }
    match rest {
        "TERMINOLOGY__ENABLED" => Some("EHRBASE_TERMINOLOGY__API_ENABLED".to_owned()),
        "FHIR__ENABLED" => Some("EHRBASE_FHIR__API_ENABLED".to_owned()),
        "EVENT_SUBSCRIPTION__ENABLED" => Some("EHRBASE_EVENTS__ADMIN_API".to_owned()),
        "BIND" => Some("EHRBASE_SERVER__BIND".to_owned()),
        "BASE_PATH" => Some("EHRBASE_SERVER__BASE_PATH".to_owned()),
        "MAX_IN_FLIGHT" => Some("EHRBASE_SERVER__MAX_IN_FLIGHT".to_owned()),
        "SWAGGER_UI" => Some("EHRBASE_SERVER__SWAGGER_UI".to_owned()),
        "CORS_PERMISSIVE" => Some("EHRBASE_SERVER__CORS_PERMISSIVE".to_owned()),
        _ => None,
    }
}

/// `Some(rest)` when `rest` is a legacy single-`_` tail (does not itself start
/// with `_`, i.e. the variable is NOT already in `SECTION__…` new form).
fn double(rest: &str) -> Option<String> {
    (!rest.starts_with('_') && !rest.is_empty()).then(|| rest.to_owned())
}

/// The `EHRBASE_<SECTION>__<TAIL>` env name for a dotted config key path —
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
    format!("EHRBASE_{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_to_double_sections() {
        assert_eq!(
            resolve_alias("EHRBASE_DB_MAX_CONNECTIONS").as_deref(),
            Some("EHRBASE_DB__MAX_CONNECTIONS")
        );
        assert_eq!(
            resolve_alias("EHRBASE_ATNA_REPOSITORY_HOST").as_deref(),
            Some("EHRBASE_ATNA__REPOSITORY_HOST")
        );
        // Already new-form → not an alias.
        assert_eq!(resolve_alias("EHRBASE_DB__MAX_CONNECTIONS"), None);
    }

    #[test]
    fn renamed_and_rest_families() {
        assert_eq!(
            resolve_alias("EHRBASE_OTEL_SERVICE_NAME").as_deref(),
            Some("EHRBASE_TELEMETRY__SERVICE_NAME")
        );
        assert_eq!(
            resolve_alias("EHRBASE_REST_BIND").as_deref(),
            Some("EHRBASE_SERVER__BIND")
        );
        assert_eq!(
            resolve_alias("EHRBASE_REST_AUTH__ENABLED").as_deref(),
            Some("EHRBASE_AUTH__ENABLED")
        );
        assert_eq!(
            resolve_alias("EHRBASE_REST_EVENT_SUBSCRIPTION__ENABLED").as_deref(),
            Some("EHRBASE_EVENTS__ADMIN_API")
        );
        assert_eq!(
            resolve_alias("EHRBASE_MANAGEMENT_ENDPOINTS_PROMETHEUS").as_deref(),
            Some("EHRBASE_MANAGEMENT__ENDPOINTS__PROMETHEUS")
        );
        assert_eq!(
            resolve_alias("EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_ENABLED").as_deref(),
            Some("EHRBASE_TERMINOLOGY__EXTERNAL__ENABLED")
        );
    }

    #[test]
    fn env_name_reconstruction() {
        assert_eq!(
            env_name_for("db.max_connections"),
            "EHRBASE_DB__MAX_CONNECTIONS"
        );
        assert_eq!(
            env_name_for("auth.oidc.issuer"),
            "EHRBASE_AUTH__OIDC__ISSUER"
        );
    }
}
