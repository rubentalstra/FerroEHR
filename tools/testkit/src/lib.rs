//! Shared test-database harness for every DB-backed test in the workspace.
//!
//! One `PostgreSQL` 18 server + one migrated **template database** per
//! migration fingerprint + one `CREATE DATABASE … TEMPLATE …` clone per
//! test, instead of the retired one-container-plus-full-migration-run per
//! test. No openEHR spec governs test infrastructure — our own design.
//!
//! Server resolution, in order:
//!
//! 1. **`EHRBASE_TEST_PG_URL`** — a DSN to an existing `PostgreSQL` 18 server
//!    whose role may `CREATE DATABASE` (CI provides the workflow's
//!    `postgres:18.4` container; a local developer server works too).
//! 2. Otherwise a **reusable named testcontainer**
//!    (`ehrbase-testkit-pg18`, `postgres:18`) is started — or adopted if a
//!    previous run left it — via `testcontainers`' reusable-containers
//!    support, tuned with the non-durable settings the `PostgreSQL` docs
//!    describe for throwaway data (`fsync=off`, `synchronous_commit=off`,
//!    `full_page_writes=off`; `PostgreSQL` docs § "Non-Durable Settings").
//!    The container is deliberately left running across runs — reclaim it
//!    with `docker rm -f ehrbase-testkit-pg18`.
//!
//! The template database (`ehrbase_tk_tpl_<fingerprint>`) is created and
//! migrated exactly once per migration fingerprint, guarded by a `PostgreSQL`
//! advisory lock so concurrent test processes (nextest runs one process per
//! test) converge on a single build. Completion is stamped as the database
//! comment, readable via `shobj_description` without connecting to the
//! template — connections to a template block cloning (`PostgreSQL` docs
//! § CREATE DATABASE). Clones are named `ehrbase_tk_<secs>_<rand>`; stale
//! clones and outdated templates are swept opportunistically under an
//! advisory try-lock.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::{Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt, ReuseDirective};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

use ehrbase::db;
use ehrbase::db::DbConfig;

/// Prefix for every database the harness creates (templates + clones), so a
/// sweep can never touch anything else.
const DB_PREFIX: &str = "ehrbase_tk_";

/// The fixed name of the reusable local `PostgreSQL` container.
const CONTAINER_NAME: &str = "ehrbase-testkit-pg18";

/// Environment variable naming an externally provided server (CI, local dev).
const ENV_URL: &str = "EHRBASE_TEST_PG_URL";

/// Advisory-lock key serializing template builds across test processes.
const TEMPLATE_LOCK_KEY: i64 = 0x0EB2_7E57_0001;

/// Advisory-lock key electing the (single, opportunistic) sweeper.
const SWEEP_LOCK_KEY: i64 = 0x0EB2_7E57_0002;

/// Clones and outdated templates older than this are swept.
const SWEEP_AGE: Duration = Duration::from_hours(2);

/// Failures of the harness itself (server acquisition, template build,
/// cloning). Test call sites `.expect()` on these — a testkit error always
/// means broken test infrastructure, never application behaviour.
#[derive(Debug, thiserror::Error)]
pub enum TestkitError {
    /// Starting or adopting the reusable `PostgreSQL` container failed.
    #[error("testkit: container: {0}")]
    Container(#[from] testcontainers::TestcontainersError),
    /// A query/connection against the server failed.
    #[error("testkit: database: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// Pool construction or migration via the platform library failed.
    #[error("testkit: platform db: {0}")]
    Db(#[from] db::DbError),
    /// The server did not accept connections within the readiness deadline.
    #[error("testkit: server at {url} not ready within {seconds}s: {last}")]
    NotReady {
        /// Redacted server location (host:port/db).
        url: String,
        /// Deadline that elapsed.
        seconds: u64,
        /// The last connection error observed.
        last: String,
    },
}

/// One fresh, fully migrated database for one test: a clone of the migrated
/// template. Dropping the guard best-effort-drops the database; a sweep at
/// harness init reclaims anything a killed process left behind.
#[derive(Debug)]
pub struct TestDb {
    pool: PgPool,
    url: String,
    name: String,
    admin_url: String,
}

impl TestDb {
    /// A handle to the clone's connection pool (cheaply cloneable).
    #[must_use]
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// The clone's DSN, for call sites that build their own configuration.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The database name (unique per call).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Best-effort immediate cleanup on a detached thread — `Drop` cannot
        // await, and the test process may exit before this lands; the init
        // sweep is the durable backstop.
        let admin_url = self.admin_url.clone();
        let name = self.name.clone();
        drop(std::thread::Builder::new().spawn(move || {
            if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                rt.block_on(async {
                    if let Ok(mut conn) = PgConnection::connect(&admin_url).await {
                        let sql = format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)");
                        drop(
                            sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
                                .execute(&mut conn)
                                .await,
                        );
                        drop(conn.close().await);
                    }
                });
            }
        }));
    }
}

/// A fresh, fully migrated database on the shared test server.
///
/// Call it as many times per test as independent databases are needed (e.g.
/// dump/load source + destination).
///
/// # Errors
///
/// Returns [`TestkitError`] when the server cannot be acquired, the template
/// cannot be built, or the clone/pool fails — always test infrastructure,
/// never application behaviour.
pub async fn db() -> Result<TestDb, TestkitError> {
    let server = server().await?;
    let template = TEMPLATE.get_or_try_init(|| ensure_template(server)).await?;
    provision(server, Some(template)).await
}

/// A fresh, **pristine** (non-migrated) database on the shared test server —
/// for harnesses that lay down their own DDL (e.g. the storage spike).
///
/// # Errors
///
/// Returns [`TestkitError`] when the server cannot be acquired or the
/// database/pool creation fails — always test infrastructure, never
/// application behaviour.
pub async fn empty_db() -> Result<TestDb, TestkitError> {
    let server = server().await?;
    provision(server, None).await
}

/// Create a uniquely named database (a clone of `template`, or empty) and
/// pool it.
async fn provision(server: &str, template: Option<&str>) -> Result<TestDb, TestkitError> {
    let name = fresh_name();
    let mut admin = connect_ready(server).await?;
    if let Some(template) = template {
        create_clone(&mut admin, &name, template).await?;
    } else {
        let sql = format!("CREATE DATABASE {name}");
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&mut admin)
            .await?;
    }
    drop(admin.close().await);

    let url = clone_url(server, &name);
    // Many tests share ONE server now: cap each clone's pool well below the
    // server's `max_connections` (100 by default) and open nothing eagerly —
    // a single test never needs the production pool sizing.
    let mut config = DbConfig::new(url.clone());
    config.max_connections = 10;
    config.min_connections = 0;
    let pool = db::connect(&config).await?;
    Ok(TestDb {
        pool,
        url,
        name,
        admin_url: server.to_owned(),
    })
}

// ---------------------------------------------------------------------------
// Server acquisition
// ---------------------------------------------------------------------------

/// The admin DSN of the shared server, resolved once per test process.
static SERVER: OnceCell<String> = OnceCell::const_new();

/// The reusable container guard. With `ReuseDirective::Always` dropping it
/// never stops the container; the guard is kept only so the handle lives for
/// the process.
static CONTAINER: OnceCell<ContainerAsync<Postgres>> = OnceCell::const_new();

/// The template database name, ensured once per test process (and, via the
/// advisory lock inside, once per server across processes).
static TEMPLATE: OnceCell<String> = OnceCell::const_new();

async fn server() -> Result<&'static str, TestkitError> {
    SERVER
        .get_or_try_init(|| async {
            if let Ok(url) = std::env::var(ENV_URL) {
                return Ok(url);
            }
            let container = CONTAINER.get_or_try_init(start_container).await?;
            let host = container.get_host().await?.to_string();
            let port = container.get_host_port_ipv4(5432).await?;
            Ok(format!(
                "postgres://postgres:postgres@{host}:{port}/postgres"
            ))
        })
        .await
        .map(String::as_str)
}

/// Start — or adopt — the reusable named `PostgreSQL` 18 container. Concurrent
/// first boots race on the fixed container name: the losers see a name
/// conflict from the daemon and retry, at which point the reuse lookup finds
/// the winner's container.
async fn start_container() -> Result<ContainerAsync<Postgres>, TestkitError> {
    let mut last = None;
    for attempt in 0..5u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(250 * u64::from(attempt))).await;
        }
        let request = Postgres::default()
            .with_tag("18")
            // Non-durable settings for throwaway test data (PostgreSQL docs
            // § "Non-Durable Settings") — the whole server is disposable.
            .with_cmd([
                "postgres",
                "-c",
                "fsync=off",
                "-c",
                "synchronous_commit=off",
                "-c",
                "full_page_writes=off",
                // Headroom for a fully parallel nextest run against the one
                // shared server (each clone pool is capped at 10).
                "-c",
                "max_connections=200",
            ])
            .with_container_name(CONTAINER_NAME)
            .with_reuse(ReuseDirective::Always);
        match request.start().await {
            Ok(container) => return Ok(container),
            Err(error) => last = Some(error),
        }
    }
    match last {
        Some(error) => Err(TestkitError::Container(error)),
        // Unreachable: five attempts either returned or set `last`.
        None => Err(TestkitError::NotReady {
            url: CONTAINER_NAME.to_owned(),
            seconds: 0,
            last: "container start never attempted".to_owned(),
        }),
    }
}

/// Connect to the admin database, retrying while the server finishes booting
/// (an adopted reused container may still be initializing).
async fn connect_ready(admin_url: &str) -> Result<PgConnection, TestkitError> {
    const DEADLINE: Duration = Duration::from_mins(1);
    let start = SystemTime::now();
    loop {
        match PgConnection::connect(admin_url).await {
            Ok(conn) => return Ok(conn),
            Err(error) => {
                if start.elapsed().unwrap_or(DEADLINE) >= DEADLINE {
                    return Err(TestkitError::NotReady {
                        url: redacted(admin_url),
                        seconds: DEADLINE.as_secs(),
                        last: error.to_string(),
                    });
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

// ---------------------------------------------------------------------------
// Template database
// ---------------------------------------------------------------------------

/// Ensure the migrated template database for the current migration
/// fingerprint exists, building it under an advisory lock when absent.
/// Returns the template's name.
async fn ensure_template(admin_url: &str) -> Result<String, TestkitError> {
    let fingerprint = db::migration_fingerprint();
    let template = format!("{DB_PREFIX}tpl_{fingerprint}");
    let stamp = format!("ehrbase-testkit fingerprint={fingerprint} complete");

    let mut admin = connect_ready(admin_url).await?;

    // Fast path: a completed template is stamped as the database comment,
    // readable without connecting to it (connections to a template block
    // `CREATE DATABASE … TEMPLATE`; PostgreSQL docs § CREATE DATABASE).
    if template_complete(&mut admin, &template, &stamp).await? {
        sweep(&mut admin, &template).await;
        drop(admin.close().await);
        return Ok(template);
    }

    // Slow path: serialize the build across processes. The lock is
    // session-scoped — hold this connection until the template is stamped.
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(TEMPLATE_LOCK_KEY)
        .execute(&mut admin)
        .await?;
    let outcome = build_template(&mut admin, admin_url, &template, &stamp).await;
    drop(
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(TEMPLATE_LOCK_KEY)
            .execute(&mut admin)
            .await,
    );
    drop(admin.close().await);
    outcome?;
    Ok(template)
}

/// Whether `template` exists and carries the completion stamp.
async fn template_complete(
    admin: &mut PgConnection,
    template: &str,
    stamp: &str,
) -> Result<bool, TestkitError> {
    // The scalar itself is nullable: `shobj_description` is NULL for a
    // template that exists but has no comment yet (a build in progress in
    // another process, or a carcass) — `NULL = $2` is NULL, and a
    // non-nullable decode explodes with `UnexpectedNullError`. NULL means
    // "not complete", never an error.
    let complete: Option<Option<bool>> = sqlx::query_scalar(
        "SELECT shobj_description(oid, 'pg_database') = $2 \
         FROM pg_database WHERE datname = $1",
    )
    .bind(template)
    .bind(stamp)
    .fetch_optional(admin)
    .await?;
    Ok(complete.flatten() == Some(true))
}

/// Build the template under the (already held) advisory lock: re-check,
/// drop any incomplete carcass, create, migrate through the platform
/// library, then stamp completion as the database comment.
async fn build_template(
    admin: &mut PgConnection,
    admin_url: &str,
    template: &str,
    stamp: &str,
) -> Result<(), TestkitError> {
    if template_complete(admin, template, stamp).await? {
        return Ok(());
    }
    let drop_sql = format!("DROP DATABASE IF EXISTS {template} WITH (FORCE)");
    sqlx::raw_sql(sqlx::AssertSqlSafe(drop_sql))
        .execute(&mut *admin)
        .await?;
    let create_sql = format!("CREATE DATABASE {template}");
    sqlx::raw_sql(sqlx::AssertSqlSafe(create_sql))
        .execute(&mut *admin)
        .await?;

    // Migrate through the exact code the binary ships, then close the pool
    // fully — a lingering connection to the template blocks every clone.
    let template_url = clone_url(admin_url, template);
    let pool = db::connect(&DbConfig::new(template_url)).await?;
    let migrated = db::run_migrations(&pool).await;
    pool.close().await;
    migrated?;

    let comment_sql = format!("COMMENT ON DATABASE {template} IS '{stamp}'");
    sqlx::raw_sql(sqlx::AssertSqlSafe(comment_sql))
        .execute(&mut *admin)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Clones
// ---------------------------------------------------------------------------

/// A unique clone name embedding its creation time (hex seconds) for the
/// sweep: `ehrbase_tk_<secs-hex>_<rand>`.
fn fresh_name() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let rand = uuid::Uuid::new_v4().simple().to_string();
    let short = rand.get(..12).unwrap_or(rand.as_str());
    format!("{DB_PREFIX}{secs:x}_{short}")
}

/// Clone the template. Concurrent clones of one template are legal but can
/// transiently conflict on the template's lock — retry briefly.
async fn create_clone(
    admin: &mut PgConnection,
    name: &str,
    template: &str,
) -> Result<(), TestkitError> {
    const DEADLINE: Duration = Duration::from_mins(1);
    let start = SystemTime::now();
    loop {
        // Default WAL_LOG strategy: no forced checkpoints per clone
        // (PostgreSQL docs § CREATE DATABASE, `STRATEGY`).
        let sql = format!("CREATE DATABASE {name} TEMPLATE {template}");
        match sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&mut *admin)
            .await
        {
            Ok(_) => return Ok(()),
            Err(error) => {
                if start.elapsed().unwrap_or(DEADLINE) >= DEADLINE {
                    return Err(TestkitError::Sqlx(error));
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// The DSN for database `name` on the server `admin_url` points at.
fn clone_url(admin_url: &str, name: &str) -> String {
    match admin_url.rfind('/') {
        Some(cut) => {
            let base = admin_url.get(..cut).unwrap_or(admin_url);
            format!("{base}/{name}")
        }
        None => format!("{admin_url}/{name}"),
    }
}

/// `host:port/db` with credentials stripped, for error messages.
fn redacted(url: &str) -> String {
    url.rsplit('@').next().unwrap_or("<unparseable>").to_owned()
}

// ---------------------------------------------------------------------------
// Sweep
// ---------------------------------------------------------------------------

/// Opportunistically drop stale harness databases: clones older than
/// [`SWEEP_AGE`] and templates other than the current one. Elected by an
/// advisory try-lock; every failure is ignored — the sweep is hygiene, not
/// correctness.
async fn sweep(admin: &mut PgConnection, current_template: &str) {
    let elected: Result<Option<bool>, sqlx::Error> =
        sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(SWEEP_LOCK_KEY)
            .fetch_optional(&mut *admin)
            .await;
    if !matches!(elected, Ok(Some(true))) {
        return;
    }

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT datname FROM pg_database WHERE datname LIKE $1 AND NOT datistemplate",
    )
    .bind(format!("{DB_PREFIX}%"))
    .fetch_all(&mut *admin)
    .await
    .unwrap_or_default();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    for name in names {
        if !stale(&name, current_template, now) {
            continue;
        }
        let sql = format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)");
        drop(
            sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
                .execute(&mut *admin)
                .await,
        );
    }

    // Roles are cluster-global: tests that must create login roles name them
    // `<clone-db-name>_<suffix>` so the same staleness parse applies (their
    // owned objects lived in the already-dropped clone).
    let roles: Vec<String> =
        sqlx::query_scalar("SELECT rolname FROM pg_roles WHERE rolname LIKE $1")
            .bind(format!("{DB_PREFIX}%"))
            .fetch_all(&mut *admin)
            .await
            .unwrap_or_default();
    for role in roles {
        if !stale(&role, current_template, now) {
            continue;
        }
        let sql = format!("DROP ROLE IF EXISTS {role}");
        drop(
            sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
                .execute(&mut *admin)
                .await,
        );
    }

    drop(
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(SWEEP_LOCK_KEY)
            .execute(&mut *admin)
            .await,
    );
}

/// Whether a harness database is stale: an outdated template (any
/// `…tpl_…` other than the current one) or a clone whose name-embedded
/// creation time is older than [`SWEEP_AGE`].
fn stale(name: &str, current_template: &str, now_secs: u64) -> bool {
    let Some(rest) = name.strip_prefix(DB_PREFIX) else {
        return false;
    };
    if let Some(_fingerprint) = rest.strip_prefix("tpl_") {
        return name != current_template;
    }
    let Some((secs_hex, _)) = rest.split_once('_') else {
        return false;
    };
    let Ok(created) = u64::from_str_radix(secs_hex, 16) else {
        return false;
    };
    now_secs.saturating_sub(created) >= SWEEP_AGE.as_secs()
}

#[cfg(test)]
mod tests {
    use super::{DB_PREFIX, SWEEP_AGE, clone_url, fresh_name, redacted, stale};

    #[test]
    fn clone_url_replaces_database_segment() {
        assert_eq!(
            clone_url("postgres://u:p@h:5432/postgres", "ehrbase_tk_x"),
            "postgres://u:p@h:5432/ehrbase_tk_x"
        );
    }

    #[test]
    fn fresh_names_are_unique_and_prefixed() {
        let a = fresh_name();
        let b = fresh_name();
        assert_ne!(a, b);
        assert!(a.starts_with(DB_PREFIX));
    }

    #[test]
    fn redaction_strips_credentials() {
        assert_eq!(redacted("postgres://u:secret@h:5432/db"), "h:5432/db");
    }

    #[test]
    fn stale_spares_current_template_and_young_clones() {
        let now = 0x1000_0000u64;
        let current = "ehrbase_tk_tpl_abcd";
        assert!(!stale(current, current, now));
        assert!(stale("ehrbase_tk_tpl_old0", current, now));
        let young = format!("{DB_PREFIX}{:x}_aaaa", now - 10);
        assert!(!stale(&young, current, now));
        let old = format!("{DB_PREFIX}{:x}_aaaa", now - SWEEP_AGE.as_secs() - 1);
        assert!(stale(&old, current, now));
        assert!(!stale("unrelated_db", current, now));
    }
}
