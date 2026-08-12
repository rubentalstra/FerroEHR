// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Shared test-database harness for every DB-backed test in the workspace.
//!
//! One `PostgreSQL` 18 server + one migrated **template database** per
//! migration fingerprint + one `CREATE DATABASE … TEMPLATE …` clone per
//! test, instead of the retired one-container-plus-full-migration-run per
//! test. No openEHR spec governs test infrastructure — our own design.
//!
//! Server resolution, in order:
//!
//! 1. **`FERROEHR_TEST_PG_URL`** — a DSN to an existing `PostgreSQL` 18 server
//!    whose role may `CREATE DATABASE` (CI provides the workflow's
//!    `postgres:18.4` container; a local developer server works too).
//! 2. Otherwise a **reusable named testcontainer**
//!    (`ferroehr-testkit-pg18`, `postgres:18`) is started — or adopted if a
//!    previous run left it — via `testcontainers`' reusable-containers
//!    support, tuned with the non-durable settings the `PostgreSQL` docs
//!    describe for throwaway data (`fsync=off`, `synchronous_commit=off`,
//!    `full_page_writes=off`; `PostgreSQL` docs § "Non-Durable Settings")
//!    and an explicit shared-memory size (`SHM_SIZE_BYTES`). The container is
//!    deliberately left running across runs — reclaim it with
//!    `docker rm -f ferroehr-testkit-pg18`.
//!
//! The template database (`ferroehr_tk_tpl_<fingerprint>`) is created and
//! migrated exactly once per migration fingerprint, guarded by a `PostgreSQL`
//! advisory lock so concurrent test processes (nextest runs one process per
//! test) converge on a single build. Completion is stamped as the database
//! comment, readable via `shobj_description` without connecting to the
//! template — connections to a template block cloning (`PostgreSQL` docs
//! § CREATE DATABASE). Clones are named `ferroehr_tk_<secs>_<rand>`.
//!
//! Every test process sweeps stale clones and outdated templates **once at
//! initialization, before it hands out any database** — the drops are
//! force-free, so a clone another process is still using is skipped as benign
//! (see [`sweep_stale`]). Unused clones therefore cannot accumulate across
//! runs, which matters: thousands of databases inflate the server's
//! cumulative-statistics area until dynamic shared memory is exhausted.

// Doctests are copy-paste templates: they must use `?`, never unwrap
// (C-QUESTION-MARK, https://rust-lang.github.io/api-guidelines/documentation.html#c-question-mark).
#![doc(test(attr(deny(warnings))))]
use std::time::{Duration, Instant};

use sqlx::{Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt, ReuseDirective};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

use ferroehr::db;
use ferroehr::db::DbConfig;

/// Prefix for every database the harness creates (templates + clones), so a
/// sweep can never touch anything else.
const DB_PREFIX: &str = "ferroehr_tk_";

/// The fixed name of the reusable local `PostgreSQL` container.
const CONTAINER_NAME: &str = "ferroehr-testkit-pg18";

/// Explicit shared-memory (`/dev/shm`) size for the reusable container, in
/// bytes — one gibibyte.
///
/// The official `postgres` image documents setting this limit explicitly
/// (<https://hub.docker.com/_/postgres>, "set shared memory limit when using
/// docker compose": `shm_size`), because the container default is 64 MB while
/// `PostgreSQL` allocates its cumulative-statistics area and every
/// parallel-query segment from *dynamic* shared memory — POSIX shared memory
/// under `/dev/shm` on Linux (`dynamic_shared_memory_type = posix`;
/// `PostgreSQL` docs § Resource Consumption). Once that fills, EVERY dynamic
/// allocation fails with
/// `could not resize shared memory segment ...: No space left on device`
/// and every database-backed test in the workspace goes red — observed at
/// ~62 MB used by a server carrying ~3000 leaked clone databases.
///
/// The size is applied at container CREATION only: `ReuseDirective::Always`
/// adopts an existing container as-is, so an already-running 64 MB container
/// must be reclaimed (`docker rm -f ferroehr-testkit-pg18`) for a changed value
/// to take effect.
const SHM_SIZE_BYTES: u64 = 1024 * 1024 * 1024;

/// `SQLSTATE` 55006 `object_in_use` — what `PostgreSQL` reports when
/// `DROP DATABASE` is refused because sessions are still connected
/// (`PostgreSQL` docs § DROP DATABASE; § Appendix A "`PostgreSQL` Error
/// Codes", class 55 "Object Not In Prerequisite State").
const SQLSTATE_OBJECT_IN_USE: &str = "55006";

/// Environment variable naming an externally provided server (CI, local dev).
const ENV_URL: &str = "FERROEHR_TEST_PG_URL";

/// Advisory-lock key serializing template builds across test processes.
const TEMPLATE_LOCK_KEY: i64 = 0x0EB2_7E57_0001;

/// Advisory-lock key electing the single startup sweeper.
const SWEEP_LOCK_KEY: i64 = 0x0EB2_7E57_0002;

/// Clones younger than this grace window are never swept.
///
/// A running test's clone can momentarily have NO session on it — the clone
/// pools run `min_connections = 0` and `sqlx` closes idle connections — so
/// connectedness alone cannot tell "live" from "leaked"; the name-embedded
/// creation time can. The window is deliberately short: it is the bound on
/// how long a leaked clone (a process killed before its `Drop` cleanup landed)
/// can linger before the next process's startup sweep reclaims it.
const SWEEP_GRACE: Duration = Duration::from_mins(30);

/// Failures of the harness itself (server acquisition, template build,
/// cloning).
///
/// Test call sites `.expect()` on these — a testkit error always means broken
/// test infrastructure, never application behaviour.
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
/// template.
///
/// Dropping the guard best-effort-drops the database; a sweep at harness init
/// reclaims anything a killed process left behind.
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
        //
        // NOTE: `WITH (FORCE)` is correct HERE and wrong in the sweep — the
        // connections it terminates are our own clone's, while the sweep drops
        // OTHER processes' leftovers and must never kill a live test's sessions.
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
    // The clone pool's establishment deadline (see the retry loop below).
    const POOL_DEADLINE: Duration = Duration::from_secs(15);
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
    // server's `max_connections` (the container is started with 200 — see the
    // `with_cmd` below; the PostgreSQL image default is 100) and open nothing
    // eagerly — a single test never needs the production pool sizing.
    let mut config = DbConfig::new(url.clone());
    config.max_connections = 10;
    config.min_connections = 0;
    // The pool's validating first connect gets the same bounded-retry treatment
    // `connect_ready` gives the admin connect, and for the same reason: under a
    // full-parallel `nextest` run the shared server's accept path is briefly
    // saturable, so a clone's first handshake can surface a transient protocol
    // error. That is server load, not a test defect — retry the ESTABLISHMENT
    // briefly and surface the last error loudly at the deadline. Never a per-test
    // retry.
    let start = Instant::now();
    let pool = loop {
        match db::connect(&config).await {
            Ok(pool) => break pool,
            Err(error) => {
                if start.elapsed() >= POOL_DEADLINE {
                    return Err(TestkitError::from(error));
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
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

/// The shared server's admin DSN, acquired once per test process. Acquisition
/// includes the readiness wait and the startup sweep, so no caller can be
/// handed a clone before stale ones have been reclaimed.
async fn server() -> Result<&'static str, TestkitError> {
    SERVER
        .get_or_try_init(|| async {
            let url = resolve_server_url().await?;
            let mut admin = connect_ready(&url).await?;
            startup_sweep(&mut admin).await;
            drop(admin.close().await);
            Ok(url)
        })
        .await
        .map(String::as_str)
}

/// The admin DSN of the externally provided server, or of the reusable
/// container (started or adopted).
#[expect(
    clippy::disallowed_methods,
    reason = "`FERROEHR_TEST_PG_URL` is the harness's OWN environment contract (CI \
              hands it the workflow server); testkit is test tooling and must not \
              depend on the server's config tree, which is what that ban protects"
)]
async fn resolve_server_url() -> Result<String, TestkitError> {
    if let Ok(url) = std::env::var(ENV_URL) {
        return Ok(url);
    }
    let container = CONTAINER.get_or_try_init(start_container).await?;
    let host = container.get_host().await?.to_string();
    let port = container.get_host_port_ipv4(5432).await?;
    Ok(format!(
        "postgres://postgres:postgres@{host}:{port}/postgres"
    ))
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
            // Never leave /dev/shm at the image default (see SHM_SIZE_BYTES):
            // 64 MB is exhausted by the cumulative-statistics area of a server
            // carrying many databases, after which every dynamic-shared-memory
            // allocation fails and the whole DB-backed suite goes red.
            .with_shm_size(SHM_SIZE_BYTES)
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
    let start = Instant::now();
    loop {
        match PgConnection::connect(admin_url).await {
            Ok(conn) => return Ok(conn),
            Err(error) => {
                if start.elapsed() >= DEADLINE {
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
    let stamp = format!("ferroehr-testkit fingerprint={fingerprint} complete");

    let mut admin = connect_ready(admin_url).await?;

    // Fast path: a completed template is stamped as the database comment,
    // readable without connecting to it (connections to a template block
    // `CREATE DATABASE … TEMPLATE`; PostgreSQL docs § CREATE DATABASE).
    // The sweep is NOT run here: it already ran once for this process while
    // the server was acquired, before any clone could be handed out.
    if template_complete(&mut admin, &template, &stamp).await? {
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
/// sweep: `ferroehr_tk_<secs-hex>_<rand>`.
#[expect(
    clippy::disallowed_methods,
    reason = "non-key randomness: a v4 suffix that makes a clone's database name \
              unique across concurrent test processes — no index locality to gain, \
              so the uuidv7 key rule does not apply"
)]
fn fresh_name() -> String {
    let secs = now_secs();
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
    let start = Instant::now();
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
                if start.elapsed() >= DEADLINE {
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

/// Wall-clock seconds since the Unix epoch — the time base embedded in every
/// clone name and read back by the sweep.
///
/// Wall-clock time comes from `jiff`, the pinned time library
///; the monotonic deadlines above use
/// [`std::time::Instant`] instead, which is what they actually measure.
/// Negative timestamps (a clock set before 1970) floor at 0 so a clone name
/// always parses back.
fn now_secs() -> u64 {
    u64::try_from(jiff::Timestamp::now().as_second()).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Sweep
// ---------------------------------------------------------------------------

/// What one sweep pass did — the startup log line, and the seam the harness's
/// own tests assert on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepReport {
    /// Stale databases the pass selected for dropping.
    pub candidates: usize,
    /// Databases actually dropped.
    pub dropped: usize,
    /// Databases another test process was still connected to, so the
    /// force-free drop was refused and the database left alone — the benign,
    /// expected outcome under a parallel run.
    pub in_use: usize,
}

/// Reclaim stale harness databases (and stale harness roles) now.
///
/// The harness sweeps once per test process at initialization, before it hands
/// out any database; this is the explicit entry point for a manual reclaim and
/// for the harness's own tests. It is safe to call at any time: the drops are
/// force-free, so a database some other process is still connected to is
/// skipped, never torn out from under it.
///
/// # Errors
///
/// Returns [`TestkitError`] when the shared server cannot be acquired or
/// connected to. Individual drops never fail the call — a database another
/// process is using is counted as [`SweepReport::in_use`], and any other drop
/// failure is logged and ignored, because the sweep is hygiene, not
/// correctness.
pub async fn sweep_stale() -> Result<SweepReport, TestkitError> {
    let server = server().await?;
    let mut admin = connect_ready(server).await?;
    let report = sweep(&mut admin).await;
    drop(admin.close().await);
    Ok(report)
}

/// The once-per-process sweep run while the server is acquired, elected by an
/// advisory try-lock: under a fully parallel nextest run exactly one process
/// sweeps at a time and the losers skip it, since the winner is already
/// reclaiming the very same set.
async fn startup_sweep(admin: &mut PgConnection) {
    let elected: Result<Option<bool>, sqlx::Error> =
        sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(SWEEP_LOCK_KEY)
            .fetch_optional(&mut *admin)
            .await;
    if !matches!(elected, Ok(Some(true))) {
        return;
    }

    let report = sweep(&mut *admin).await;
    let candidates = report.candidates;
    let dropped = report.dropped;
    let in_use = report.in_use;
    tracing::info!(
        "testkit startup sweep: {candidates} stale database(s), {dropped} dropped, {in_use} in use"
    );

    drop(
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(SWEEP_LOCK_KEY)
            .execute(&mut *admin)
            .await,
    );
}

/// Drop every stale harness database — clones older than [`SWEEP_GRACE`], and
/// templates other than the current migration fingerprint's — plus the stale
/// harness roles.
///
/// The drops are deliberately **force-free**: `DROP DATABASE` fails while any
/// session is connected unless `FORCE` is used (`PostgreSQL` docs
/// § DROP DATABASE), reporting [`SQLSTATE_OBJECT_IN_USE`]. That refusal means
/// exactly "another test process owns this clone", so it is skipped as benign.
/// Together with the grace window — which spares a clone whose owner currently
/// holds no pooled connection — this is what makes the sweep safe to run at the
/// start of every test process against one shared server.
///
/// Every failure is swallowed (logged): the sweep is hygiene, not correctness.
async fn sweep(admin: &mut PgConnection) -> SweepReport {
    let mut report = SweepReport {
        candidates: 0,
        dropped: 0,
        in_use: 0,
    };
    // The live template is derived, not passed in: the sweep runs before the
    // template is ensured, and the fingerprint is a pure function of the
    // migrations compiled into this binary.
    let current_template = format!("{DB_PREFIX}tpl_{}", db::migration_fingerprint());
    let now = now_secs();

    let listed: Result<Vec<String>, sqlx::Error> = sqlx::query_scalar(
        "SELECT datname FROM pg_database WHERE datname LIKE $1 AND NOT datistemplate",
    )
    .bind(format!("{DB_PREFIX}%"))
    .fetch_all(&mut *admin)
    .await;
    let names = match listed {
        Ok(names) => names,
        Err(error) => {
            tracing::warn!("testkit sweep: cannot list harness databases: {error}");
            Vec::new()
        }
    };

    for name in names {
        if !stale(&name, &current_template, now) {
            continue;
        }
        report.candidates += 1;
        // No `WITH (FORCE)`: a clone with live sessions belongs to a test
        // process that is still running and must survive this sweep.
        let sql = format!("DROP DATABASE IF EXISTS {name}");
        match sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&mut *admin)
            .await
        {
            Ok(_) => report.dropped += 1,
            Err(error) => {
                if refusal_is_benign(sqlstate(&error).as_deref()) {
                    report.in_use += 1;
                    tracing::debug!("testkit sweep: {name} is in use, skipped");
                } else {
                    tracing::warn!("testkit sweep: dropping {name} failed: {error}");
                }
            }
        }
    }

    // Roles are cluster-global: tests that must create login roles name them
    // `<clone-db-name>_<suffix>` so the same staleness parse applies (their
    // owned objects lived in the already-dropped clone).
    let listed_roles: Result<Vec<String>, sqlx::Error> =
        sqlx::query_scalar("SELECT rolname FROM pg_roles WHERE rolname LIKE $1")
            .bind(format!("{DB_PREFIX}%"))
            .fetch_all(&mut *admin)
            .await;
    for role in listed_roles.unwrap_or_default() {
        if !stale(&role, &current_template, now) {
            continue;
        }
        let sql = format!("DROP ROLE IF EXISTS {role}");
        if let Err(error) = sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&mut *admin)
            .await
        {
            tracing::debug!("testkit sweep: dropping role {role} failed: {error}");
        }
    }

    report
}

/// The `SQLSTATE` a failure carries, when it was reported by the server at all
/// (a transport or pool failure carries none).
fn sqlstate(error: &sqlx::Error) -> Option<String> {
    let code = sqlx::error::DatabaseError::code(error.as_database_error()?)?;
    Some(code.into_owned())
}

/// Whether a refused sweep drop is the benign "another test process is still
/// connected to this database" outcome rather than a broken-harness signal.
fn refusal_is_benign(code: Option<&str>) -> bool {
    code == Some(SQLSTATE_OBJECT_IN_USE)
}

/// Whether a harness database is stale: an outdated template (any
/// `…tpl_…` other than the current one) or a clone whose name-embedded
/// creation time is older than [`SWEEP_GRACE`].
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
    now_secs.saturating_sub(created) >= SWEEP_GRACE.as_secs()
}

#[cfg(test)]
mod tests {
    use super::{
        DB_PREFIX, SQLSTATE_OBJECT_IN_USE, SWEEP_GRACE, clone_url, fresh_name, redacted,
        refusal_is_benign, stale,
    };

    #[test]
    fn clone_url_replaces_database_segment() {
        assert_eq!(
            clone_url("postgres://u:p@h:5432/postgres", "ferroehr_tk_x"),
            "postgres://u:p@h:5432/ferroehr_tk_x"
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
        let current = "ferroehr_tk_tpl_abcd";
        assert!(!stale(current, current, now));
        assert!(stale("ferroehr_tk_tpl_old0", current, now));
        let young = format!("{DB_PREFIX}{:x}_aaaa", now - 10);
        assert!(!stale(&young, current, now));
        let old = format!("{DB_PREFIX}{:x}_aaaa", now - SWEEP_GRACE.as_secs() - 1);
        assert!(stale(&old, current, now));
        assert!(!stale("unrelated_db", current, now));
    }

    /// The live template is excluded from the sweep at every age: it carries no
    /// creation time in its name and must survive arbitrarily long runs, since
    /// every clone is made from it.
    #[test]
    fn stale_never_sweeps_the_live_template() {
        let current = "ferroehr_tk_tpl_deadbeef";
        for now in [0u64, 0x1000_0000, u64::MAX] {
            assert!(
                !stale(current, current, now),
                "live template swept at {now}"
            );
        }
        // A template built from any OTHER migration fingerprint is stale.
        assert!(stale("ferroehr_tk_tpl_0", current, 0));
        assert!(stale("ferroehr_tk_tpl_feedface", current, 0));
    }

    /// A clone name whose embedded creation time does not parse is never swept:
    /// an unknown age is not evidence of staleness.
    #[test]
    fn stale_spares_unparseable_clone_names() {
        let current = "ferroehr_tk_tpl_abcd";
        let now = 0x1000_0000u64;
        assert!(!stale("ferroehr_tk_notahexnumber_x", current, now));
        assert!(!stale("ferroehr_tk_nounderscore", current, now));
    }

    /// `PostgreSQL` refuses a force-free `DROP DATABASE` on a database with
    /// live sessions with SQLSTATE 55006 (`object_in_use`) — for the sweep that
    /// is a benign "another test process owns this clone", not a harness
    /// failure. Any other SQLSTATE, or none at all, is not benign.
    #[test]
    fn only_object_in_use_is_a_benign_sweep_refusal() {
        assert_eq!(SQLSTATE_OBJECT_IN_USE, "55006");
        assert!(refusal_is_benign(Some(SQLSTATE_OBJECT_IN_USE)));
        assert!(!refusal_is_benign(Some("55000"))); // object_not_in_prerequisite_state
        assert!(!refusal_is_benign(Some("42501"))); // insufficient_privilege
        assert!(!refusal_is_benign(Some("3D000"))); // invalid_catalog_name
        assert!(!refusal_is_benign(None)); // transport/pool failure
    }
}
