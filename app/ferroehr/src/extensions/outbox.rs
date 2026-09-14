// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The outbox reader registry and the retention prune both domains share.
//!
//! **No openEHR spec governs eventing: our own design/extension.**
//! `event_outbox` has one drainer (the AMQP publisher, which stamps
//! `published_at`) and any number of cursor readers, each keeping a `last_seq`
//! high-water mark in `event_outbox_reader`. The prune deletes a published row
//! only once it is older than the retention window AND at or below every
//! active reader's cursor, in one statement, so a slow or paused reader never
//! loses rows (#3330). Which readers are active is reconciled from the
//! configuration at boot, so a reader that was switched off cannot hold the
//! outbox forever.
//!
//! The outbox is tenant-scoped by row policy, so a reader drains one tenant at
//! a time under that tenant's scope and its cursor is per tenant (#3355): one
//! global mark advanced in one tenant's pass would skip the other tenants'
//! lower sequence numbers. Every function here reads the tenant from the task
//! scope ([`crate::extensions::tenant_context::current`]) and falls back to the
//! reserved default tenant, which is the whole store when tenancy is off.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::extensions::tenant_context::TenantContext;

/// A registered cursor reader over `event_outbox`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxReader(&'static str);

impl OutboxReader {
    /// The FHIR outbound emitter (`fhir.outbound`).
    pub const FHIR_OUTBOUND: Self = Self("fhir-outbound");

    /// The reader's registry key.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.0
    }
}

/// The tenant the current task drains: the scoped one, else the reserved
/// default.
fn scope_tenant() -> Uuid {
    crate::extensions::tenant_context::current().map_or_else(Uuid::nil, |t| t.tenant_id)
}

/// Every tenant a background reader drains in turn.
///
/// Under the stamped `multi` posture, every row of the clinical pool's `tenant`
/// registry (not tenant-scoped, always holding the reserved default); under
/// `single` the default tenant alone, because the pools stamp no request
/// tenant then and a pass for another tenant would write under the default
/// tenant's session and be refused by the row policy.
///
/// # Errors
///
/// Returns the database error when the registry cannot be read.
pub async fn tenants(pool: &PgPool) -> Result<Vec<TenantContext>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, system_id FROM tenant \
         WHERE ext.tenancy_posture() = 'multi' OR id = $1 ORDER BY id",
    )
    .bind(Uuid::nil())
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| {
            Ok(TenantContext {
                tenant_id: row.try_get("id")?,
                system_id: row.try_get("system_id")?,
            })
        })
        .collect()
}

/// Record whether `reader` runs in this deployment for the scoped tenant,
/// keeping its cursor.
///
/// Called at boot for every known reader and every registered tenant, each in
/// that tenant's scope (the registry is tenant-scoped like every other table),
/// and by a running reader at the start of each tenant pass, so a tenant
/// registered after boot gains its row on the reader's first pass. The row is
/// created if missing, so a reader that never advanced still holds (or
/// releases) the floor.
///
/// # Errors
///
/// Returns the database error when the registry cannot be written.
pub async fn reconcile(
    pool: &PgPool,
    reader: OutboxReader,
    active: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO event_outbox_reader (reader, tenant_id, active) VALUES ($1, $2, $3) \
         ON CONFLICT (reader, tenant_id) DO UPDATE \
            SET active = EXCLUDED.active, updated_at = now()",
    )
    .bind(reader.name())
    .bind(scope_tenant())
    .bind(active)
    .execute(pool)
    .await?;
    Ok(())
}

/// The reader's cursor for the scoped tenant: the highest `seq` it has fully
/// processed there, `0` before its first advance.
///
/// # Errors
///
/// Returns the database error when the registry cannot be read.
pub async fn cursor(pool: &PgPool, reader: OutboxReader) -> Result<i64, sqlx::Error> {
    let last: Option<i64> = sqlx::query_scalar(
        "SELECT last_seq FROM event_outbox_reader WHERE reader = $1 AND tenant_id = $2",
    )
    .bind(reader.name())
    .bind(scope_tenant())
    .fetch_optional(pool)
    .await?;
    Ok(last.unwrap_or(0))
}

/// Advance the reader's cursor for the scoped tenant to `seq`, never backwards.
///
/// A reader that advances is running, so the row is registered active by
/// this call; a concurrent or delayed pass cannot move the mark back, which
/// bounds the at-least-once duplication window instead of leaving it open.
///
/// # Errors
///
/// Returns the database error when the registry cannot be written.
pub async fn advance(pool: &PgPool, reader: OutboxReader, seq: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO event_outbox_reader (reader, tenant_id, last_seq, active) \
         VALUES ($1, $2, $3, true) \
         ON CONFLICT (reader, tenant_id) DO UPDATE \
            SET last_seq = GREATEST(event_outbox_reader.last_seq, EXCLUDED.last_seq), \
                active = true, updated_at = now()",
    )
    .bind(reader.name())
    .bind(scope_tenant())
    .bind(seq)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete the scoped tenant's published rows older than the retention window
/// that every active reader has passed, in the domain `pool`'s own outbox.
/// Returns the number pruned.
///
/// The floor is the lowest cursor among the readers registered active for the
/// scoped tenant, read in the same statement; with none the window alone
/// decides. The row policy bounds both the delete and the floor to that tenant.
///
/// # Errors
///
/// Returns the database error when the delete fails.
pub async fn prune(pool: &PgPool, retention_days: i64) -> Result<u64, sqlx::Error> {
    let cutoff = format!("{retention_days} days");
    let result = sqlx::query(
        "DELETE FROM event_outbox \
         WHERE published_at IS NOT NULL \
           AND published_at < now() - $1::interval \
           AND seq <= COALESCE( \
                 (SELECT min(last_seq) FROM event_outbox_reader \
                   WHERE active AND tenant_id = $2), seq)",
    )
    .bind(cutoff)
    .bind(scope_tenant())
    .execute(pool)
    .await?;
    let pruned = result.rows_affected();
    if pruned > 0 {
        tracing::debug!("pruned {pruned} published event rows past retention");
    }
    Ok(pruned)
}
