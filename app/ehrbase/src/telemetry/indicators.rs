//! Concrete health indicators (binding doc §2). These live in the binary
//! because they touch the pool / audit sender; they implement the
//! [`HealthIndicator`] trait `ehrbase-rest` defines, and are registered into
//! its [`HealthRegistry`] at boot.

use crate::system_log::AuditSender;
use async_trait::async_trait;
use ehrbase_rest::management::{Health, HealthIndicator};
use sqlx::PgPool;

/// `db` — a bounded `SELECT 1` liveness ping. Required for readiness.
#[derive(Debug)]
pub struct DbHealth {
    pool: PgPool,
}

impl DbHealth {
    /// Construct over the application pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HealthIndicator for DbHealth {
    fn name(&self) -> &'static str {
        "db"
    }

    async fn check(&self) -> Health {
        match sqlx::query("SELECT 1").execute(&self.pool).await {
            Ok(_) => Health::up(),
            Err(e) => Health::down(format!("database ping failed: {e}")),
        }
    }
}

/// `migrations` — verifies the greenfield schema is present (the `ehr.node` +
/// `ehr.vo_version` core tables exist). Required for readiness.
#[derive(Debug)]
pub struct MigrationsHealth {
    pool: PgPool,
}

impl MigrationsHealth {
    /// Construct over the application pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HealthIndicator for MigrationsHealth {
    fn name(&self) -> &'static str {
        "migrations"
    }

    async fn check(&self) -> Health {
        let query = "SELECT (to_regclass('ehr.node') IS NOT NULL \
             AND to_regclass('ehr.vo_version') IS NOT NULL) AS applied";
        match sqlx::query_scalar::<_, bool>(query)
            .fetch_one(&self.pool)
            .await
        {
            Ok(true) => Health::up(),
            Ok(false) => Health::down("core schema tables missing (migrations not applied)"),
            Err(e) => Health::down(format!("migration check failed: {e}")),
        }
    }
}

/// `audit_sender` — reports the ATNA sender master switch. Degraded-tolerable
/// (fail-open auditing must not block readiness), so a down never flips
/// readiness — only the aggregate to `DEGRADED`.
#[derive(Debug)]
pub struct AuditHealth {
    sender: AuditSender,
}

impl AuditHealth {
    /// Construct over the audit sender handle.
    #[must_use]
    pub fn new(sender: AuditSender) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl HealthIndicator for AuditHealth {
    fn name(&self) -> &'static str {
        "audit_sender"
    }

    async fn check(&self) -> Health {
        if self.sender.enabled() {
            Health::up()
        } else {
            Health::degraded("audit sender disabled")
        }
    }

    fn required(&self) -> bool {
        false
    }
}
