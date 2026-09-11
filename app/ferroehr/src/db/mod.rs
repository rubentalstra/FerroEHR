// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `PostgreSQL` bootstrap: connection settings, pool construction, and the
//! per-schema migration sequence.
//!
//! No openEHR spec governs the persistence mechanism; the storage substrate is
//! our own PG18-native design. This module is the single place the rest of the
//! crate obtains a database handle: [`DbConfig`] (the `[db]` config section)
//! feeds [`connect`] and [`connect_tenant_scoped`] for the clinical domain,
//! [`connect_demographic`] / [`connect_tenant_scoped_demographic`] for the
//! demographic one and [`connect_linkage`] / [`connect_tenant_scoped_linkage`]
//! for the linkage one, and [`prepare`] brings the schema to the state this
//! build requires — on the migration DSN ([`DbConfig::migrate_dsn`]), a fourth
//! credential a deployment may name because preparation spans every schema
//! while each runtime credential holds one domain. The three served domains
//! differ only in the `search_path` their connections carry, so one set of
//! storage functions serves them all. [`verify_domain_isolation`] is the boot gate that refuses to
//! serve when the runtime roles can read across those boundaries. The `sea-query`
//! identifier vocabulary for the live schema lives in [`iden`]. This is the
//! defining module for the whole bootstrap surface, with no re-exports.

pub mod iden;

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};

use crate::config::secret::SecretUrl;

// ── Settings — the `[db]` config section ─────────────────────────────────────

/// The zero-config dev DSN (matches the compose dev stack). Production MUST
/// override it; the boot path warns prominently when
/// [`DbConfig::is_dev_default`] holds.
pub const DEFAULT_URL: &str = "postgres://ferroehr:ferroehr@localhost:5432/ferroehr";

/// What the server does about schema migrations when it boots.
///
/// Migrations are DDL, so a self-migrating server must authenticate as a role
/// that can execute DDL — and a role that can rewrite the schema is a role an
/// application-level SQL flaw can rewrite the schema with. This setting is how
/// a deployment opts out of that: the schema is applied out of band by the
/// migrator role, and the server connects as the DML-only `ferroehr_app` role.
///
/// No openEHR spec governs migration mechanics or database roles — our own
/// design/extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MigrationMode {
    /// Apply the embedded migrations at boot, then verify (the default, and
    /// what makes an empty configuration boot against an empty database).
    #[default]
    Apply,
    /// Never issue DDL: verify that the database already carries exactly this
    /// build's migrations, and refuse to serve when it does not.
    Verify,
}

/// Connection settings for the application `PostgreSQL` database — the `[db]`
/// section of the one config tree ([`crate::config::FerroEhrConfig`]), with no
/// loader of its own.
///
/// No openEHR spec governs persistence — our own design. The DSN is a
/// [`SecretUrl`]: its embedded credentials are redacted from every rendering
/// (`Debug`, `/management/env`, `config check`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DbConfig {
    /// `PostgreSQL` connection DSN (`postgres://user:pass@host:port/db`).
    /// Credentials are redacted from every rendering ([`SecretUrl`]).
    pub url: SecretUrl,
    /// Path to a file holding the DSN, read at boot in place of [`Self::url`].
    ///
    /// The route for a deployment that mounts its database credential as a file
    /// rather than passing it as an environment value, which is readable through
    /// `/proc/<pid>/environ` and inherited by every child process. Setting both
    /// this and a non-default `url` is a boot error.
    pub url_file: Option<PathBuf>,
    /// DSN for the **demographic** pseudonymisation domain, when a deployment
    /// separates the two runtime roles; unset (the default) reuses
    /// [`Self::url`].
    ///
    /// The schema separation is unconditional — the demographic chapter always
    /// reads and writes the `demographic` schema. This key is what turns it
    /// into a ROLE separation as well: point it at a DSN authenticating as
    /// `ferroehr_demographic`, leave [`Self::url`] on `ferroehr_ehr`, and
    /// neither connection can reach the other domain's relations even if a
    /// query tries (GDPR Art. 4(5) and Art. 32(1)(a); EDPB Guidelines 01/2025
    /// require the separation to hold against internal actors). No openEHR
    /// spec governs database roles — our own design/extension.
    pub demographic_url: Option<SecretUrl>,
    /// Path to a file holding [`Self::demographic_url`], read at boot in place
    /// of it — the mounted-secret route, as [`Self::url_file`] is for the
    /// clinical DSN. Setting both is a boot error.
    pub demographic_url_file: Option<PathBuf>,
    /// DSN for the **linkage** pseudonymisation domain, when a deployment
    /// separates the runtime roles; unset (the default) reuses [`Self::url`].
    ///
    /// The third domain, and the one the other two exist to be kept apart
    /// from: `linkage` holds which demographic party is the subject of which
    /// EHR, which is the "additional information" that re-attributes a
    /// pseudonymised record to a person. Point this at a DSN authenticating as
    /// `ferroehr_linkage` and no runtime credential holds both the map and
    /// either side of it (GDPR Art. 4(5) and Art. 32(1)(a); EDPB Guidelines
    /// 01/2025 require the separation to hold against internal actors). No
    /// openEHR spec governs database roles — our own design/extension.
    pub linkage_url: Option<SecretUrl>,
    /// Path to a file holding [`Self::linkage_url`], read at boot in place of
    /// it — the mounted-secret route, as [`Self::url_file`] is for the
    /// clinical DSN. Setting both is a boot error.
    pub linkage_url_file: Option<PathBuf>,
    /// DSN that schema preparation ([`prepare`]) authenticates as; unset (the
    /// default) reuses [`Self::url`].
    ///
    /// Preparing the schema spans EVERY schema, whatever [`Self::migrate`]
    /// says: `apply` issues the DDL of all five migration sets, and `verify`
    /// reads all five `_sqlx_migrations` bookkeeping tables. A runtime
    /// credential that holds one pseudonymisation domain can do neither, so
    /// under the separated-role posture this key is what names the credential
    /// that can, while the pools stay narrow. It is used for that one boot
    /// step on a connection that is closed again — no pool is ever held on it,
    /// and no request is ever served through it. No openEHR spec governs
    /// migration mechanics or database roles — our own design/extension.
    pub migrate_url: Option<SecretUrl>,
    /// Path to a file holding [`Self::migrate_url`], read at boot in place of
    /// it — the mounted-secret route, as [`Self::url_file`] is for the
    /// clinical DSN. Setting both is a boot error.
    pub migrate_url_file: Option<PathBuf>,
    /// Upper bound of the connection pool.
    pub max_connections: u32,
    /// Connections the pool keeps open when idle (avoids cold reopen +
    /// `SET search_path` churn under variable load).
    pub min_connections: u32,
    /// Seconds to wait for a free connection before failing.
    pub acquire_timeout_secs: u64,
    /// `statement_timeout` applied to every pooled connection, in
    /// milliseconds; `0` leaves the server default (usually unlimited).
    ///
    /// This is the backstop the request timeout cannot be. A request over the
    /// HTTP timeout is answered `408` by dropping the handler future, which does
    /// not cancel the statement PostgreSQL is running
    /// (<https://www.postgresql.org/docs/18/runtime-config-client.html>), so
    /// without this a handful of expensive queries can hold every pooled
    /// connection while their clients have already been given up on.
    ///
    /// Set ABOVE the AQL engine's own budget
    /// ([`crate::service::query::config::QueryConfig::timeout_ms`]) so the
    /// engine's typed refusal fires first and this only catches what the engine
    /// does not govern. No openEHR spec governs it — our own design.
    pub statement_timeout_ms: u64,
    /// Whether the server applies its embedded migrations at boot
    /// ([`MigrationMode`]).
    ///
    /// `apply` (the default) keeps a fresh checkout and a fresh database
    /// working with no configuration at all. `verify` is the least-privilege
    /// production posture: the DSN may then authenticate as a role with no DDL
    /// rights, and the server refuses to boot against a database that is not
    /// already migrated to exactly this build.
    pub migrate: MigrationMode,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: SecretUrl::new(DEFAULT_URL),
            url_file: None,
            demographic_url: None,
            demographic_url_file: None,
            linkage_url: None,
            linkage_url_file: None,
            migrate_url: None,
            migrate_url_file: None,
            // Deliberate defaults: 20 max (10 hard-capped realistic write
            // concurrency ×2), 2 min (no cold reopen churn at idle).
            max_connections: 20,
            min_connections: 2,
            acquire_timeout_secs: 30,
            // Twice the engine's own 30 s budget, so the engine refuses first
            // and this remains a backstop rather than the primary control.
            statement_timeout_ms: 60_000,
            migrate: MigrationMode::Apply,
        }
    }
}

impl DbConfig {
    /// Settings for `url` with defaults for everything else.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: SecretUrl::new(url.into()),
            ..Self::default()
        }
    }

    /// Whether the DSN is the built-in dev default (no operator override). The
    /// boot path logs a prominent warning in this case so a production
    /// deployment never silently runs against the dev database.
    #[must_use]
    pub fn is_dev_default(&self) -> bool {
        self.url.expose() == DEFAULT_URL
    }

    /// The DSN the demographic pool connects with: [`Self::demographic_url`]
    /// when the deployment separates the runtime roles, else [`Self::url`].
    #[must_use]
    pub fn demographic_dsn(&self) -> &str {
        self.demographic_url
            .as_ref()
            .map_or_else(|| self.url.expose(), SecretUrl::expose)
    }

    /// The DSN the linkage pool connects with: [`Self::linkage_url`] when the
    /// deployment separates the runtime roles, else [`Self::url`].
    #[must_use]
    pub fn linkage_dsn(&self) -> &str {
        self.linkage_url
            .as_ref()
            .map_or_else(|| self.url.expose(), SecretUrl::expose)
    }

    /// The DSN schema preparation connects with: [`Self::migrate_url`] when
    /// the deployment names a credential for it, else [`Self::url`].
    #[must_use]
    pub fn migrate_dsn(&self) -> &str {
        self.migrate_url
            .as_ref()
            .map_or_else(|| self.url.expose(), SecretUrl::expose)
    }

    /// Whether the demographic domain authenticates as its own database role
    /// (a distinct DSN), rather than sharing the clinical one.
    #[must_use]
    pub fn roles_are_separated(&self) -> bool {
        self.demographic_url.is_some()
    }

    /// Whether the linkage domain authenticates as its own database role
    /// (a distinct DSN), rather than sharing the clinical one.
    #[must_use]
    pub fn linkage_role_is_separated(&self) -> bool {
        self.linkage_url.is_some()
    }

    /// Whether schema preparation authenticates as its own database role
    /// (a distinct DSN), rather than as the clinical runtime one.
    #[must_use]
    pub fn migrator_is_separated(&self) -> bool {
        self.migrate_url.is_some()
    }
}

// ── Error ────────────────────────────────────────────────────────────────────

/// Errors produced by the persistence foundation.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// A driver/pool/query error from `sqlx`.
    #[error("database: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A schema migration failed to apply.
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// The database does not carry exactly this build's migrations, and
    /// [`MigrationMode::Verify`] forbids applying them.
    #[error(
        "the database is not migrated to this build ({0}). \
         [db].migrate is `verify`, so this server will not issue DDL: apply the \
         migrations out of band with the migrator role, then start it again"
    )]
    SchemaNotReady(#[source] SchemaMismatch),

    /// The credential preparing the schema cannot read a migration set's
    /// bookkeeping at all, so the migration state is unknown rather than
    /// wrong.
    #[error(
        "database role `{role}` cannot read the migration state of schema `{schema}`: \
         preparing the schema reads every schema's `_sqlx_migrations` bookkeeping table, and \
         this credential holds no privilege on `{schema}`. Under the separated-domain posture \
         no runtime credential ever can, because each holds one domain: point `[db] \
         migrate_url` (or `migrate_url_file`) at the credential that prepares the schema — \
         unset, it falls back to `[db] url`"
    )]
    SchemaUnreadable {
        /// The schema whose migration bookkeeping could not be read.
        schema: String,
        /// The database role the refused connection authenticates as.
        role: String,
        /// `PostgreSQL`'s own refusal (`SQLSTATE` 42501).
        #[source]
        source: sqlx::Error,
    },

    /// A cold archival tier outlived the primary tier it mirrors, in either
    /// the clinical or the demographic domain.
    #[error(
        "a cold archival tier is present but the primary tier it mirrors is not: \
         `cold` without `ehr.vo_version`, or `cold_demographic` without \
         `demographic.vo_version`. Each pair is one repository and has been wiped \
         apart. The cold tables still hold content, and their column shape was \
         copied from the primary tables as they stood before the wipe — so this \
         server will not adopt them: a re-adopted mirror can differ in shape from the \
         tier it mirrors, and the rows belong to a repository that no longer exists. \
         Restore the whole database from backup (every schema together), or, if the \
         wipe was intended, drop the surviving cold schema (`DROP SCHEMA cold \
         CASCADE` / `DROP SCHEMA cold_demographic CASCADE`) and start again"
    )]
    OrphanedArchiveTier,

    /// A runtime role can read a relation belonging to a pseudonymisation
    /// domain it does not own.
    #[error(
        "the pseudonymisation boundary is not enforced by the database: role `{role}` can reach \
         the {kind} `{relation}`, which belongs to another domain. The clinical record, the \
         identity of its subject, and the map between them must not be reachable by one \
         credential (GDPR Art. 4(5) and \
         Art. 32(1)(a); no openEHR spec governs database roles — our own design). Remedy: \
         `REVOKE ALL ON {relation} FROM {role}` — and, for a function, `REVOKE EXECUTE ON \
         FUNCTION {relation} FROM PUBLIC`, since PUBLIC holds EXECUTE by default \
         (https://www.postgresql.org/docs/18/sql-grant.html)"
    )]
    DomainIsolationBreached {
        /// The runtime role holding the privilege it must not hold.
        role: String,
        /// What kind of object it reaches (`table`, `view`, `sequence`, …).
        kind: String,
        /// The schema-qualified object it reaches.
        relation: String,
    },
}

/// How a database's recorded migration state differs from the one this binary
/// embeds.
///
/// Each variant names a distinct operational situation, so a caller can branch
/// on it rather than match a message: an unmigrated database, a database behind
/// the binary, a database ahead of it, and a database whose bookkeeping is
/// damaged.
#[derive(Debug, thiserror::Error)]
pub enum SchemaMismatch {
    /// The schema carries no `_sqlx_migrations` table at all.
    #[error("schema `{schema}` has never been migrated")]
    NeverMigrated {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
    },

    /// Migrations this binary embeds are absent from the database.
    #[error(
        "schema `{schema}` is missing migration(s) {versions:?} — the database is older than this build"
    )]
    Missing {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The absent migration versions, ascending.
        versions: Vec<i64>,
    },

    /// A migration is recorded as having failed partway through.
    #[error("schema `{schema}` records migration {version} as failed")]
    Failed {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The failed migration's version.
        version: i64,
    },

    /// A migration was applied from source text this binary does not carry.
    #[error("schema `{schema}` applied migration {version} from different source text")]
    ChecksumMismatch {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The diverging migration's version.
        version: i64,
    },

    /// The database carries migrations this binary does not know about.
    #[error(
        "schema `{schema}` carries migration(s) {versions:?} — the database is newer than this build"
    )]
    Ahead {
        /// The `PostgreSQL` schema that carries the migration set.
        schema: String,
        /// The unknown migration versions, ascending.
        versions: Vec<i64>,
    },
}

// ── Pool ─────────────────────────────────────────────────────────────────────

/// Search path applied to every pooled connection serving the **clinical**
/// domain: the EHR tables live in `ehr`, the AQL support functions and the
/// `"C"`/`en_US` collations in `ext`. Set once per physical connection
/// (`after_connect`) so queries may use unqualified table names.
const CLINICAL_SEARCH_PATH: &str = "SET search_path TO ehr, ext, public";

/// Search path applied to every pooled connection serving the **demographic**
/// domain (`demographic/0001_baseline`), whose relations carry the same names
/// and column shape as the clinical ones.
///
/// This one constant is the whole routing mechanism: a pool opened with it
/// reuses every storage function unchanged, because the SQL those functions
/// emit names its relations unqualified and `search_path` decides which schema
/// they resolve in. `ehr` is deliberately absent — a query this pool issues
/// against a clinical relation must fail to resolve rather than quietly cross
/// the pseudonymisation boundary.
///
/// No openEHR spec governs storage layout or database roles — our own
/// design/extension (GDPR Art. 4(5) and Art. 32(1)(a);
/// <https://eur-lex.europa.eu/eli/reg/2016/679/oj>).
const DEMOGRAPHIC_SEARCH_PATH: &str = "SET search_path TO demographic, ext, public";

/// Search path applied to every pooled connection serving the **linkage**
/// domain (`linkage/0001_baseline`), which holds the one relation joining the
/// other two: which demographic party is the subject of which EHR.
///
/// Neither `ehr` nor `demographic` is on it, for the reason the schema exists:
/// a query issued on this pool against either domain's relations must fail to
/// resolve rather than quietly re-join what the split holds apart. The
/// crossing happens one layer up, in the service, over two pools — never
/// inside one statement.
///
/// No openEHR spec governs storage layout or database roles — our own
/// design/extension (GDPR Art. 4(5) and Art. 32(1)(a);
/// <https://eur-lex.europa.eu/eli/reg/2016/679/oj>).
const LINKAGE_SEARCH_PATH: &str = "SET search_path TO linkage, ext, public";

/// Whether a pool stamps the per-request tenant on its connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tenancy {
    /// Single-tenant: no GUC statement on any hook (zero checkout overhead).
    Off,
    /// Multi-tenant: the `ferroehr.tenant_id` session GUC is stamped on every
    /// new connection and re-stamped on every checkout.
    Scoped,
}

/// Everything a freshly-opened physical connection needs before it serves a
/// query: the domain's search path, the statement-timeout backstop, and — when
/// tenancy is on — the request's tenant GUC.
///
/// One implementation for every pool, so a domain or tenancy variant cannot
/// drop a setting. Dropping the timeout in particular silently disarms the
/// DB-side runaway-query guard, and a broken control must never look like a
/// policy outcome.
async fn open_session(
    conn: &mut PgConnection,
    search_path: &'static str,
    statement_timeout: Option<&str>,
    tenancy: Tenancy,
) -> Result<(), sqlx::Error> {
    sqlx::query(search_path).execute(&mut *conn).await?;
    if let Some(statement_timeout) = statement_timeout {
        // A session-level SET on the physical connection, surviving every
        // checkout, where `SET LOCAL` would last one transaction.
        // `AssertSqlSafe` is audited: PostgreSQL's `SET` takes no bind
        // placeholder and the value is a `u64` from our own configuration,
        // never client input.
        sqlx::query(sqlx::AssertSqlSafe(statement_timeout.to_owned()))
            .execute(&mut *conn)
            .await?;
    }
    if tenancy == Tenancy::Scoped {
        stamp_tenant_guc(conn).await?;
    }
    Ok(())
}

/// The pool options for one domain: sizing + acquire timeout from settings,
/// the domain's session setup on every physical connection, and no per-checkout
/// liveness ping. Connection retirement stays on the `sqlx` defaults (an idle
/// reap plus a bounded lifetime — infinite-lived connections are discouraged by
/// the driver, so we do not disable them).
fn pool_options(settings: &DbConfig, search_path: &'static str, tenancy: Tenancy) -> PgPoolOptions {
    // Rendered once here rather than per connection. The value is an integer
    // from our own configuration, never client input, and it is bound as a
    // literal because PostgreSQL's `SET` takes no parameter placeholder.
    let statement_timeout = (settings.statement_timeout_ms > 0)
        .then(|| format!("SET statement_timeout = {}", settings.statement_timeout_ms));
    let options = PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(Duration::from_secs(settings.acquire_timeout_secs))
        // No liveness ping per checkout: the default `test_before_acquire`
        // adds one round trip to EVERY acquisition; a broken connection is
        // detected by its first real statement and retried by the pool.
        .test_before_acquire(false)
        .after_connect(move |conn, _meta| {
            // Cloned per call: `after_connect` takes an `Fn`, so the captured
            // value cannot be moved out of it.
            let statement_timeout = statement_timeout.clone();
            Box::pin(async move {
                open_session(conn, search_path, statement_timeout.as_deref(), tenancy).await
            })
        });
    match tenancy {
        Tenancy::Off => options,
        // `after_connect` covers a connection freshly opened by `acquire`
        // itself under pool growth; `before_acquire` re-stamps a previously
        // idle connection on every checkout (docs.rs,
        // `sqlx::pool::PoolOptions::before_acquire`: "This is _not_ invoked
        // for new connections. Use `after_connect` for those.").
        Tenancy::Scoped => options.before_acquire(|conn, _meta| {
            Box::pin(async move {
                stamp_tenant_guc(conn).await?;
                Ok(true)
            })
        }),
    }
}

/// Create the clinical application connection pool (single-tenant / tenancy-off).
///
/// Every physical connection is initialized with the clinical search path
/// (`ehr, ext, public`) so queries can use unqualified table names, as the
/// schema expects. There is no per-acquire hook: zero checkout overhead when
/// tenancy is off.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] when the DSN does not parse as a `PostgreSQL`
/// URL, the initial connection fails (unreachable host, refused
/// authentication, unknown database), or the search-path initialization
/// statement fails on that first connection.
pub async fn connect(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = pool_options(settings, CLINICAL_SEARCH_PATH, Tenancy::Off)
        .connect(settings.url.expose())
        .await?;
    Ok(pool)
}

/// Create the **demographic** connection pool (single-tenant / tenancy-off).
///
/// The twin of [`connect`] for the pseudonymisation domain: the same pool
/// settings, the demographic search path, and [`DbConfig::demographic_dsn`] —
/// which is `[db].demographic_url` when a deployment separates the two runtime
/// roles, and `[db].url` otherwise. The schema separation is therefore always
/// on; the role separation is the deployment's choice.
///
/// # Errors
///
/// The same failures as [`connect`], against the demographic DSN.
pub async fn connect_demographic(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = pool_options(settings, DEMOGRAPHIC_SEARCH_PATH, Tenancy::Off)
        .connect(settings.demographic_dsn())
        .await?;
    Ok(pool)
}

/// Create the **linkage** connection pool (single-tenant / tenancy-off).
///
/// The twin of [`connect_demographic`] for the third pseudonymisation domain:
/// the same pool settings, the linkage search path, and [`DbConfig::linkage_dsn`]
/// — which is `[db].linkage_url` when a deployment separates the runtime roles,
/// and `[db].url` otherwise. The schema separation is therefore always on; the
/// role separation is the deployment's choice.
///
/// # Errors
///
/// The same failures as [`connect`], against the linkage DSN.
pub async fn connect_linkage(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = pool_options(settings, LINKAGE_SEARCH_PATH, Tenancy::Off)
        .connect(settings.linkage_dsn())
        .await?;
    Ok(pool)
}

/// Stamp the `ferroehr.tenant_id` session GUC on a connection from the
/// current task's tenant context ([`crate::extensions::tenant_context::current`])
/// — `''` (⇒ the reserved default tenant) when no tenant is in scope (a
/// background worker, or a request that resolved no tenant).
async fn stamp_tenant_guc(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    let tenant = crate::extensions::tenant_context::current()
        .map_or_else(String::new, |t| t.tenant_id.to_string());
    sqlx::query("SELECT set_config('ferroehr.tenant_id', $1, false)")
        .bind(tenant)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Creates the **tenant-scoped** clinical pool.
///
/// Wraps [`connect`] with hooks that stamp the `ferroehr.tenant_id` session GUC
/// on every checked-out connection from the current request's tenant context
/// ([`crate::extensions::tenant_context::current`]). Multi-tenancy is our own
/// deployment extension — no openEHR spec governs it.
///
/// This is the seam that scopes **both** autocommit reads and transactions:
/// the service checks out a fresh connection per read and one per write
/// transaction, and each carries the session GUC the RLS `tenant_isolation`
/// policy (and the `tenant_id` column DEFAULT) read. A connection returning
/// to the pool keeps its session-level GUC, so every acquire re-stamps it —
/// to the request's tenant, or to `''` (⇒ the reserved default tenant) when
/// no tenant is in scope — so a reused connection never leaks the previous
/// request's tenant.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] when the DSN does not parse as a `PostgreSQL`
/// URL, the initial connection fails (unreachable host, refused
/// authentication, unknown database), or the search-path initialization
/// statement fails on that first connection.
pub async fn connect_tenant_scoped(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = pool_options(settings, CLINICAL_SEARCH_PATH, Tenancy::Scoped)
        .connect(settings.url.expose())
        .await?;
    Ok(pool)
}

/// Creates the **tenant-scoped demographic** pool — [`connect_demographic`]
/// with the tenant hooks of [`connect_tenant_scoped`].
///
/// The demographic relations carry the same `tenant_id` column, DEFAULT and
/// `tenant_isolation` RLS policy as the clinical ones, so a demographic read is
/// tenant-scoped exactly as a clinical one is.
///
/// # Errors
///
/// The same failures as [`connect_tenant_scoped`], against the demographic DSN.
pub async fn connect_tenant_scoped_demographic(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = pool_options(settings, DEMOGRAPHIC_SEARCH_PATH, Tenancy::Scoped)
        .connect(settings.demographic_dsn())
        .await?;
    Ok(pool)
}

/// Creates the **tenant-scoped linkage** pool — [`connect_linkage`] with the
/// tenant hooks of [`connect_tenant_scoped`].
///
/// `linkage.party_ehr` carries the same `tenant_id` column, DEFAULT and
/// `tenant_isolation` RLS policy as the clinical and demographic relations, and
/// the tenant is a part of its temporal primary key — so a mapping written
/// without the GUC stamped would land on the reserved default tenant whatever
/// the request said, and resolve from there again.
///
/// # Errors
///
/// The same failures as [`connect_tenant_scoped`], against the linkage DSN.
pub async fn connect_tenant_scoped_linkage(settings: &DbConfig) -> Result<PgPool, DbError> {
    let pool = pool_options(settings, LINKAGE_SEARCH_PATH, Tenancy::Scoped)
        .connect(settings.linkage_dsn())
        .await?;
    Ok(pool)
}

/// A demographic pool over the DSN an existing clinical pool already holds, for
/// a caller that has a [`PgPool`] and no [`DbConfig`].
///
/// This is what lets [`crate::service::FerroEhrService::new`] stay synchronous
/// and infallible while still routing the demographic chapter at the
/// demographic schema: `PgPool::connect_options` hands back the connect options
/// the pool was built from, and `PgPoolOptions::connect_lazy_with` builds a pool
/// from them with no I/O at all.
///
/// It carries the pool defaults rather than the deployment's `[db]` tuning,
/// which it cannot see, and it opens no connection until one is asked for
/// (`min_connections(0)`, so constructing a service costs nothing). A
/// deployment that tunes the pool, separates the runtime roles, or enables
/// tenancy supplies its own pool through
/// [`crate::service::FerroEhrService::with_demographic_pool`] instead.
#[must_use]
pub fn demographic_pool_from(pool: &PgPool) -> PgPool {
    let defaults = DbConfig::default();
    let options = pool.connect_options();
    pool_options(&defaults, DEMOGRAPHIC_SEARCH_PATH, Tenancy::Off)
        .min_connections(0)
        .connect_lazy_with(PgConnectOptions::clone(&options))
}

/// A linkage pool over the DSN an existing clinical pool already holds, the
/// twin of [`demographic_pool_from`] for the third domain.
///
/// It exists for the same reason: [`crate::service::FerroEhrService::new`]
/// stays synchronous and infallible while still routing the linkage chapter at
/// the `linkage` schema. It carries the pool defaults rather than the
/// deployment's `[db]` tuning, and opens no connection until one is asked for.
/// A deployment that tunes the pool, separates the runtime roles, or enables
/// tenancy supplies its own pool through
/// [`crate::service::FerroEhrService::with_linkage_pool`] instead.
#[must_use]
pub fn linkage_pool_from(pool: &PgPool) -> PgPool {
    let defaults = DbConfig::default();
    let options = pool.connect_options();
    pool_options(&defaults, LINKAGE_SEARCH_PATH, Tenancy::Off)
        .min_connections(0)
        .connect_lazy_with(PgConnectOptions::clone(&options))
}

// ── Migrations ───────────────────────────────────────────────────────────────

/// The `ext` schema: our openEHR support functions (`openehr_magnitude` and
/// its ISO-8601 helpers). Runs before `ehr`.
static EXT_MIGRATOR: Migrator = sqlx::migrate!("migrations/ext");

/// The `ehr` schema — the greenfield PG18-native CDR schema (no openEHR spec
/// governs the physical schema, spike-validated): the unified
/// per-version `node` table, the temporal `vo_version` table, and the
/// supporting tables.
static EHR_MIGRATOR: Migrator = sqlx::migrate!("migrations/ehr");

/// The `demographic` schema — the demographic pseudonymisation domain: PARTY
/// versioned objects and their change control, physically separated from the
/// clinical schema so no runtime role reads both (GDPR Art. 4(5) and
/// Art. 32(1)(a); no openEHR spec governs storage layout — our own design).
/// Runs after `ehr`: its relations are mirrored from the clinical ones and it
/// moves the parties out of them.
static DEMOGRAPHIC_MIGRATOR: Migrator = sqlx::migrate!("migrations/demographic");

/// The `linkage` schema — the linkage pseudonymisation domain: the map from a
/// demographic party to the EHR whose subject it is, the additional
/// information that re-joins a pseudonymised record to a person (GDPR
/// Art. 4(5) and Art. 32(1)(a); no openEHR spec governs storage layout — our
/// own design). Runs after `demographic`: its grants revoke the clinical and
/// demographic roles from this schema and this schema's role from theirs, so
/// both sets of relations must already exist.
static LINKAGE_MIGRATOR: Migrator = sqlx::migrate!("migrations/linkage");

/// The `audit` schema — the local IHE ATNA Audit Record Repository (the
/// `audit_event` table). Strictly outside the EHR content (BASE
/// `architecture_overview/master07-security.adoc` §Access logging: in-system
/// access logs, never part of the EHR proper); runs after `ehr`.
static AUDIT_MIGRATOR: Migrator = sqlx::migrate!("migrations/audit");

/// The migration sets in application order, each paired with the schema
/// that carries its `_sqlx_migrations` bookkeeping table.
const MIGRATION_SETS: &[(&str, &Migrator)] = &[
    ("ext", &EXT_MIGRATOR),
    ("ehr", &EHR_MIGRATOR),
    ("demographic", &DEMOGRAPHIC_MIGRATOR),
    ("linkage", &LINKAGE_MIGRATOR),
    ("audit", &AUDIT_MIGRATOR),
];

/// Bootstrap done outside the migrations: the five schemas and `btree_gist`
/// (required by the temporal `WITHOUT OVERLAPS` primary key).
const BOOTSTRAP: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS ext",
    "CREATE SCHEMA IF NOT EXISTS ehr",
    "CREATE SCHEMA IF NOT EXISTS demographic",
    "CREATE SCHEMA IF NOT EXISTS linkage",
    "CREATE SCHEMA IF NOT EXISTS audit",
    "CREATE EXTENSION IF NOT EXISTS btree_gist WITH SCHEMA ext",
];

/// A stable fingerprint of the complete embedded migration state: the
/// bootstrap statements plus every migrator's (version, checksum) sequence,
/// in application order.
///
/// Two builds with identical migrations produce the same value; any migration
/// change produces a new one. Test infrastructure keys its migrated template
/// databases on this (no openEHR spec governs test infrastructure — our own
/// design).
#[must_use]
pub fn migration_fingerprint() -> String {
    // FNV-1a over the bootstrap text + each migration's version/checksum —
    // collision-resistant enough for a cache key with a handful of live
    // values, with no hashing dependency.
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(PRIME);
        }
    };
    for statement in BOOTSTRAP {
        eat(statement.as_bytes());
    }
    for (_, migrator) in MIGRATION_SETS {
        for migration in migrator.iter() {
            eat(&migration.version.to_le_bytes());
            eat(&migration.checksum);
        }
    }
    format!("{hash:016x}")
}

/// Bootstrap schemas/extensions and apply the migration sets, `ext` before
/// `ehr`.
///
/// Each migrator runs on a connection whose `search_path` starts with its
/// target schema, so the unqualified DDL and that set's `_sqlx_migrations`
/// bookkeeping table land in the right schema (two independent bookkeeping
/// tables, one per set). Safe to call repeatedly: an already-applied migration
/// is skipped, and its recorded checksum is validated against the embedded
/// source.
///
/// The sequence runs on a connection **detached** from the pool and closed
/// afterwards: the migrators mutate the session `search_path`, and a
/// mid-sequence failure must never return a connection with a non-standard
/// search path to the pool.
///
/// # Errors
///
/// Returns [`DbError::Sqlx`] when no connection can be acquired within the
/// acquire timeout or a bootstrap/`search_path` statement fails (e.g. the
/// role lacks `CREATE` on the database, or `btree_gist` is unavailable), and
/// [`DbError::Migrate`] when a migration fails to apply or an
/// already-applied migration fails checksum validation against the embedded
/// source.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    let mut conn = pool.acquire().await?.detach();
    let outcome = apply_migrations(&mut conn).await;
    close_quietly(conn).await;
    outcome
}

/// Open the one-shot connection schema preparation runs on: the migration DSN
/// ([`DbConfig::migrate_dsn`]), which is `[db].migrate_url` when a deployment
/// names a credential for this step and `[db].url` otherwise.
///
/// Preparation is a boot step rather than a serving path, so it takes a single
/// connection that is closed again instead of a third pool held for the
/// process lifetime, and it carries none of the pools' session setup: no
/// domain `search_path` (the sequence sets its own per set, and the
/// bookkeeping reads are schema-qualified) and no `statement_timeout`, which
/// is the backstop for request-serving statements and would cancel a long
/// migration partway
/// (<https://www.postgresql.org/docs/18/runtime-config-client.html>).
async fn migration_connection(settings: &DbConfig) -> Result<PgConnection, DbError> {
    let conn = PgConnection::connect(settings.migrate_dsn()).await?;
    Ok(conn)
}

/// Close a detached connection, reporting a failure to close as a trace event
/// rather than as the operation's outcome: the work is already done, and a
/// failed close must not mask its result.
async fn close_quietly(conn: PgConnection) {
    if let Err(error) = conn.close().await {
        tracing::debug!(%error, "closing the migration connection failed");
    }
}

/// Applies the embedded migrations on the migration DSN
/// ([`DbConfig::migrate_dsn`]) — the credential half of
/// [`MigrationMode::Apply`].
///
/// [`run_migrations`] over a connection of its own rather than one from a
/// runtime pool, so a deployment whose runtime credentials each hold a single
/// pseudonymisation domain can still prepare a schema that spans all five.
///
/// # Errors
///
/// [`DbError::Sqlx`] when the migration DSN does not parse, the connection
/// fails, or a bootstrap statement is refused (a credential without `CREATE`
/// on the database or on a schema), and [`DbError::Migrate`] when a migration
/// fails to apply or an already-applied one fails checksum validation.
pub async fn apply_schema(settings: &DbConfig) -> Result<(), DbError> {
    let mut conn = migration_connection(settings).await?;
    let outcome = apply_migrations(&mut conn).await;
    close_quietly(conn).await;
    outcome
}

/// Verifies the recorded migration state on the migration DSN
/// ([`DbConfig::migrate_dsn`]) — the credential half of
/// [`MigrationMode::Verify`].
///
/// [`verify_migrations`] over a connection of its own, for the same reason
/// [`apply_schema`] takes one: the check reads all five schemas'
/// `_sqlx_migrations` tables, which no single-domain runtime credential can
/// do. Issues no DDL, so the credential it names needs read access and
/// nothing more.
///
/// # Errors
///
/// [`DbError::SchemaNotReady`] naming the first divergence found,
/// [`DbError::SchemaUnreadable`] when the credential cannot read a set's
/// bookkeeping at all, or [`DbError::Sqlx`] when the connection or the read
/// fails for any other reason.
pub async fn verify_schema(settings: &DbConfig) -> Result<(), DbError> {
    let mut conn = migration_connection(settings).await?;
    let outcome = verify_recorded_state(&mut conn).await;
    close_quietly(conn).await;
    outcome
}

/// Brings the database to the state this build requires, as
/// [`DbConfig::migrate`] directs, and then proves the pseudonymisation
/// boundary holds.
///
/// The boot-path entry point — call it rather than the halves it composes
/// wherever the operator's configuration should decide. [`MigrationMode::Apply`]
/// applies the embedded migrations ([`apply_schema`]);
/// [`MigrationMode::Verify`] issues no DDL and only checks
/// ([`verify_schema`]).
///
/// **The two halves deliberately authenticate as different credentials, and
/// that is the whole point of the split.** Schema preparation spans every
/// schema — the DDL of all five migration sets under `apply`, all five
/// `_sqlx_migrations` bookkeeping tables under `verify` — so it runs on the
/// migration DSN ([`DbConfig::migrate_dsn`]), which a deployment separating
/// its runtime roles points at a credential that can reach them all.
/// [`verify_domain_isolation`] runs on `runtime_pool` instead, because it
/// exists to measure what THAT credential can reach: it reads `pg_catalog`
/// and the `has_*_privilege` functions, which any role may call
/// (<https://www.postgresql.org/docs/18/functions-info.html>), so running it
/// on the migration credential would not fail — it would silently measure a
/// role that holds every domain by design, and the boot gate would stop
/// saying anything about the roles that serve requests.
///
/// # Errors
///
/// In `apply` mode, whatever [`apply_schema`] returns. In `verify` mode,
/// [`DbError::SchemaNotReady`] when the database does not carry exactly this
/// build's migrations, [`DbError::SchemaUnreadable`] when the migration
/// credential cannot read a set's bookkeeping at all, or [`DbError::Sqlx`]
/// when the check itself cannot run. In both modes,
/// [`DbError::DomainIsolationBreached`] when a runtime role can reach another
/// pseudonymisation domain.
pub async fn prepare(settings: &DbConfig, runtime_pool: &PgPool) -> Result<(), DbError> {
    match settings.migrate {
        MigrationMode::Apply => apply_schema(settings).await?,
        MigrationMode::Verify => {
            tracing::info!(
                "[db].migrate is `verify`: this server issues no DDL and requires an \
                 already-migrated database"
            );
            verify_schema(settings).await?;
        }
    }
    verify_domain_isolation(runtime_pool).await
}

/// Verifies, without issuing any DDL, that the database carries exactly the
/// migrations this binary embeds.
///
/// Read-only by construction: it reads each set's `_sqlx_migrations`
/// bookkeeping table and compares versions, success flags and checksums
/// against the embedded sources. That makes it usable both as the boot gate for
/// [`MigrationMode::Verify`] and as an operator check against a running
/// database.
///
/// # Errors
///
/// [`DbError::SchemaNotReady`] naming the first divergence found,
/// [`DbError::SchemaUnreadable`] when this pool's credential cannot read a
/// set's bookkeeping at all, or [`DbError::Sqlx`] when a connection or the
/// bookkeeping read fails for any other reason.
pub async fn verify_migrations(pool: &PgPool) -> Result<(), DbError> {
    let mut conn = pool.acquire().await?;
    verify_recorded_state(&mut conn).await
}

/// Compare every migration set's bookkeeping against its embedded source, on
/// one connection.
///
/// The shared body of [`verify_migrations`] (a pooled connection) and
/// [`verify_schema`] (the one-shot migration connection), so the boot gate and
/// the operator check cannot drift apart.
async fn verify_recorded_state(conn: &mut PgConnection) -> Result<(), DbError> {
    for (schema, migrator) in MIGRATION_SETS {
        verify_set(&mut *conn, schema, migrator).await?;
    }
    Ok(())
}

/// Every runtime role paired with the schemas it must not be able to read.
///
/// Three pseudonymisation domains, mutually barred. `ferroehr_ehr`/
/// `ferroehr_ehr_reader` serve the clinical record; `ferroehr_demographic`/
/// `ferroehr_demographic_reader` serve the identities; `ferroehr_linkage`
/// serves the map that says which identity belongs to which record, and is
/// barred from both — a role holding the map and either side of it would hold
/// the join the split exists to withhold. No openEHR spec governs database
/// roles — our own design/extension.
const DOMAIN_ROLE_BARRIERS: &[(&str, &[&str])] = &[
    (
        "ferroehr_ehr",
        &["demographic", "cold_demographic", "linkage"],
    ),
    (
        "ferroehr_ehr_reader",
        &["demographic", "cold_demographic", "linkage"],
    ),
    ("ferroehr_demographic", &["ehr", "cold", "linkage"]),
    ("ferroehr_demographic_reader", &["ehr", "cold", "linkage"]),
    (
        "ferroehr_linkage",
        &["ehr", "cold", "demographic", "cold_demographic"],
    ),
];

/// Refuses to serve when a runtime role can read anything in a
/// pseudonymisation domain it does not own.
///
/// The separation of the clinical record, the identity of its subject, and the
/// map between them is a property of the DATABASE's grants, not of the
/// application's routing: code that reaches for the wrong schema is a bug this
/// server can fix, while a role that can read two of the three domains defeats
/// the separation no matter how correct the code is (GDPR Art. 4(5) and
/// Art. 32(1)(a),
/// <https://eur-lex.europa.eu/eli/reg/2016/679/oj>; EDPB Guidelines 01/2025
/// require the separation to hold against internal actors). So the grants are
/// checked at boot, and a breach is a refusal rather than a warning.
///
/// Every object kind a read could go through is covered: tables, partitioned
/// tables, foreign tables, views and materialized views (`SELECT`), sequences
/// (`SELECT`/`USAGE`) and functions (`EXECUTE` — which PUBLIC holds by default,
/// PostgreSQL 18 `GRANT` §Notes,
/// <https://www.postgresql.org/docs/18/sql-grant.html>).
///
/// A role that does not exist is skipped rather than failed: role provisioning
/// is a deployment step, and the migrations themselves create the roles only
/// when the migrator holds `CREATEROLE` (dev, compose and the test harness run
/// without them). The check therefore proves what it can see and never invents
/// a failure out of an absent role.
///
/// # Errors
///
/// [`DbError::DomainIsolationBreached`] naming the role, the object kind and
/// the schema-qualified object it can reach, or [`DbError::Sqlx`] when the
/// catalog read itself fails.
pub async fn verify_domain_isolation(pool: &PgPool) -> Result<(), DbError> {
    for (role, forbidden) in DOMAIN_ROLE_BARRIERS {
        // The existence probe runs first and separately: `has_table_privilege`
        // raises `undefined_object` for a role that does not exist
        // (<https://www.postgresql.org/docs/18/functions-info.html>), so it can
        // never be evaluated for one, not even under a WHERE that would discard
        // the row.
        let present: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname = $1)")
                .bind(role)
                .fetch_one(pool)
                .await?;
        if !present {
            continue;
        }
        let forbidden: Vec<String> = forbidden.iter().map(|s| (*s).to_owned()).collect();
        let breach: Option<(String, String)> = sqlx::query_as(
            "SELECT kind, relation FROM (
                 SELECT CASE c.relkind
                            WHEN 'S' THEN 'sequence'
                            WHEN 'v' THEN 'view'
                            WHEN 'm' THEN 'materialized view'
                            WHEN 'f' THEN 'foreign table'
                            ELSE 'table'
                        END AS kind,
                        n.nspname || '.' || c.relname AS relation
                 FROM pg_namespace n
                 JOIN pg_class c ON c.relnamespace = n.oid
                 WHERE n.nspname = ANY($2)
                   AND c.relkind IN ('r', 'p', 'v', 'm', 'f', 'S')
                   AND CASE WHEN c.relkind = 'S'
                            THEN has_sequence_privilege($1, c.oid, 'SELECT,USAGE')
                            ELSE has_table_privilege($1, c.oid, 'SELECT')
                       END
                 UNION ALL
                 SELECT 'function', n.nspname || '.' || p.proname
                 FROM pg_namespace n
                 JOIN pg_proc p ON p.pronamespace = n.oid
                 WHERE n.nspname = ANY($2)
                   AND has_function_privilege($1, p.oid, 'EXECUTE')
             ) reachable
             ORDER BY relation
             LIMIT 1",
        )
        .bind(role)
        .bind(&forbidden)
        .fetch_optional(pool)
        .await?;
        if let Some((kind, relation)) = breach {
            return Err(DbError::DomainIsolationBreached {
                role: (*role).to_owned(),
                kind,
                relation,
            });
        }
    }
    Ok(())
}

/// How many stored versions the reserved default tenant owns.
///
/// The default tenant is the nil uuid, and `ext.current_tenant_id()` resolves
/// an unset `ferroehr.tenant_id` GUC to it, so it owns every row written while
/// tenancy was off. A request that reaches the tenancy middleware without a
/// resolvable tenant runs unscoped and therefore reads exactly this content,
/// which is why a deployment enabling tenancy over an existing store is told
/// at boot what that tenant holds.
///
/// The count is taken over the primary tier only: an archived version is not
/// reachable by a query, and the number exists to say whether the default
/// tenant is empty, not to size the store.
///
/// No openEHR spec governs multi-tenancy — our own design/extension.
///
/// # Errors
///
/// [`DbError::Sqlx`] when the connection or the count fails.
pub async fn default_tenant_versions(pool: &PgPool) -> Result<i64, DbError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ehr.vo_version WHERE tenant_id = '00000000-0000-0000-0000-000000000000'::uuid",
    )
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// `SQLSTATE` 42501 `insufficient_privilege` — what `PostgreSQL` reports for a
/// refused read, whether the missing grant is on the relation or on its schema
/// (`PostgreSQL` docs § Appendix A "`PostgreSQL` Error Codes", class 42,
/// <https://www.postgresql.org/docs/18/errcodes-appendix.html>).
const SQLSTATE_INSUFFICIENT_PRIVILEGE: &str = "42501";

/// Whether `PostgreSQL` refused this statement for lack of privilege, rather
/// than failing for any other reason.
fn is_insufficient_privilege(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(refusal)
            if refusal.code().as_deref() == Some(SQLSTATE_INSUFFICIENT_PRIVILEGE)
    )
}

/// The database role this session authenticates as, for an error that must
/// name the credential and not only the schema.
///
/// A failure to read it renders as `unknown`: this runs only while another
/// error is already being reported, and replacing that error with this one
/// would hide the boot failure the operator has to act on.
async fn current_role(conn: &mut PgConnection) -> String {
    sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(&mut *conn)
        .await
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// Classify a failed bookkeeping read: a privilege refusal becomes
/// [`DbError::SchemaUnreadable`], naming the schema and the credential;
/// anything else stays the driver error it was.
///
/// A bare 42501 about `_sqlx_migrations` is unactionable — it names a table an
/// operator has never heard of and no credential at all — and the separated
/// posture reaches it on the very first set.
async fn unreadable_bookkeeping(
    conn: &mut PgConnection,
    schema: &str,
    error: sqlx::Error,
) -> DbError {
    if !is_insufficient_privilege(&error) {
        return DbError::Sqlx(error);
    }
    DbError::SchemaUnreadable {
        schema: schema.to_owned(),
        role: current_role(&mut *conn).await,
        source: error,
    }
}

/// Compare one migration set's bookkeeping table against its embedded source.
async fn verify_set(
    conn: &mut PgConnection,
    schema: &str,
    migrator: &Migrator,
) -> Result<(), DbError> {
    // The schema name is one of the literals in `MIGRATION_SETS`, never
    // input: `to_regclass` answers NULL for a missing relation rather than
    // failing (PostgreSQL 18 docs, "System Information Functions") — but it
    // still raises 42501 for a schema this credential may not enter, which is
    // why both statements here classify their failure.
    let bookkeeping = format!("{schema}._sqlx_migrations");
    let present: bool = match sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(&bookkeeping)
        .fetch_one(&mut *conn)
        .await
    {
        Ok(present) => present,
        Err(error) => return Err(unreadable_bookkeeping(&mut *conn, schema, error).await),
    };
    if !present {
        return Err(DbError::SchemaNotReady(SchemaMismatch::NeverMigrated {
            schema: schema.to_owned(),
        }));
    }

    let query = format!("SELECT version, success, checksum FROM {bookkeeping} ORDER BY version");
    let applied: Vec<(i64, bool, Vec<u8>)> = match sqlx::query_as(sqlx::AssertSqlSafe(query))
        .fetch_all(&mut *conn)
        .await
    {
        Ok(applied) => applied,
        Err(error) => return Err(unreadable_bookkeeping(&mut *conn, schema, error).await),
    };

    let mut unknown: Vec<i64> = applied.iter().map(|(version, _, _)| *version).collect();
    let mut missing: Vec<i64> = Vec::new();
    for embedded in migrator.iter() {
        let Some((version, success, checksum)) = applied
            .iter()
            .find(|(version, _, _)| *version == embedded.version)
        else {
            missing.push(embedded.version);
            continue;
        };
        unknown.retain(|known| known != version);
        if !success {
            return Err(DbError::SchemaNotReady(SchemaMismatch::Failed {
                schema: schema.to_owned(),
                version: *version,
            }));
        }
        if checksum.as_slice() != embedded.checksum.as_ref() {
            return Err(DbError::SchemaNotReady(SchemaMismatch::ChecksumMismatch {
                schema: schema.to_owned(),
                version: *version,
            }));
        }
    }
    if !missing.is_empty() {
        return Err(DbError::SchemaNotReady(SchemaMismatch::Missing {
            schema: schema.to_owned(),
            versions: missing,
        }));
    }
    if !unknown.is_empty() {
        return Err(DbError::SchemaNotReady(SchemaMismatch::Ahead {
            schema: schema.to_owned(),
            versions: unknown,
        }));
    }
    Ok(())
}

/// The bootstrap + four-migrator sequence on one dedicated connection.
async fn apply_migrations(conn: &mut PgConnection) -> Result<(), DbError> {
    for &statement in BOOTSTRAP {
        sqlx::query(statement).execute(&mut *conn).await?;
    }

    sqlx::query("SET search_path TO ext")
        .execute(&mut *conn)
        .await?;
    EXT_MIGRATOR.run(&mut *conn).await?;

    guard_orphaned_archive_tier(&mut *conn).await?;

    sqlx::query("SET search_path TO ehr, ext")
        .execute(&mut *conn)
        .await?;
    EHR_MIGRATOR.run(&mut *conn).await?;

    sqlx::query("SET search_path TO demographic, ext")
        .execute(&mut *conn)
        .await?;
    DEMOGRAPHIC_MIGRATOR.run(&mut *conn).await?;

    sqlx::query("SET search_path TO linkage, ext")
        .execute(&mut *conn)
        .await?;
    LINKAGE_MIGRATOR.run(&mut *conn).await?;

    sqlx::query("SET search_path TO audit, ext")
        .execute(&mut *conn)
        .await?;
    AUDIT_MIGRATOR.run(&mut *conn).await?;
    Ok(())
}

/// Refuse to migrate a database whose cold archival tier outlived its primary
/// tier, in either domain.
///
/// Two migrations create objects outside the schema whose set records them:
/// `ehr/0007_cold_archive_tier` builds `cold` beside `ehr`, and
/// `demographic/0001_baseline` builds `cold_demographic` beside `demographic`.
/// So a `DROP SCHEMA … CASCADE` — a restore gone wrong, a recreated volume, a
/// wiped test database — leaves the mirror tables standing while the
/// bookkeeping that records them goes away. Re-applying then hits
/// `relation "vo_version" already exists`, which is a permanent boot loop with
/// no error naming the cause.
///
/// Making the migration re-runnable would be the wrong repair: those mirrors
/// were built with `CREATE TABLE … (LIKE …)` against the primary tables as they
/// stood, so adopting a surviving one silently accepts a mirror that may not
/// match the tier it mirrors and re-attaches clinical rows to a repository that
/// is gone. The refusal carries the remedy in its message.
///
/// `to_regclass` is used rather than a catalog join because it answers `NULL` for
/// a missing relation instead of failing
/// (<https://www.postgresql.org/docs/18/functions-info.html>), so one statement
/// covers both a fresh database and a healthy one.
async fn guard_orphaned_archive_tier(conn: &mut PgConnection) -> Result<(), DbError> {
    let orphaned: bool = sqlx::query_scalar(
        "SELECT (to_regclass('cold.vo_version') IS NOT NULL
                 AND to_regclass('ehr.vo_version') IS NULL)
             OR (to_regclass('cold_demographic.vo_version') IS NOT NULL
                 AND to_regclass('demographic.vo_version') IS NULL)",
    )
    .fetch_one(&mut *conn)
    .await?;
    if orphaned {
        return Err(DbError::OrphanedArchiveTier);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_applied() {
        let s = DbConfig::new("postgres://localhost/ferroehr");
        assert_eq!(s.url.expose(), "postgres://localhost/ferroehr");
        assert_eq!(s.max_connections, 20);
        assert_eq!(s.min_connections, 2);
        assert_eq!(s.acquire_timeout_secs, 30);
        assert!(!s.is_dev_default());
    }

    #[test]
    fn default_url_is_the_dev_dsn() {
        assert!(DbConfig::default().is_dev_default());
    }
}
