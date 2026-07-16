//! The event-filter subscription store: CRUD over `event_subscription` as
//! methods on `EhrbaseService`.
//!
//! **No openEHR spec governs this — our own design/extension** (the
//! event/subscription semantics are our own model, not any SM interface).
//! Part of the `events` extension; gate: `events.enabled` (a subscription is
//! inert unless the publisher is spawned to bind its queue).
//!
//! A subscription is a small predicate record (`kind` / `change_type` /
//! `template_id` / `archetype`, each NULL = wildcard) that the publisher turns
//! into an AMQP topic binding on the events exchange so the broker fans events
//! out to a durable per-subscription queue ([`super`]). This module owns only
//! the CRUD; queue declaration is the drainer's concern (it re-syncs the
//! enabled set when it changes or the broker connection is fresh — the service
//! is kept broker-free).
//
// The CRUD helpers below read the `pub(crate)` `pool` field of
// `crate::service::EhrbaseService`.

use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::service::status::SmError;

impl EhrbaseService {
    /// Map an `event_subscription` row to its PHI-free JSON record (NULL
    /// predicate = wildcard, rendered as JSON `null`).
    fn subscription_row(row: &sqlx::postgres::PgRow) -> Result<Value, ServiceError> {
        let id: Uuid = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let kind: Option<String> = row.try_get("kind")?;
        let change_type: Option<String> = row.try_get("change_type")?;
        let template_id: Option<String> = row.try_get("template_id")?;
        let archetype: Option<String> = row.try_get("archetype")?;
        let enabled: bool = row.try_get("enabled")?;
        let created_at = row
            .try_get::<jiff_sqlx::Timestamp, _>("created_at")?
            .to_jiff();
        Ok(json!({
            "id": id.to_string(),
            "name": name,
            "kind": kind,
            "change_type": change_type,
            "template_id": template_id,
            "archetype": archetype,
            "enabled": enabled,
            "created_at": created_at.to_string(),
        }))
    }

    /// List every stored subscription (newest first).
    async fn list_subscriptions(&self) -> Result<Vec<Value>, ServiceError> {
        let rows = sqlx::query(
            "SELECT id, name, kind, change_type, template_id, archetype, enabled, created_at \
             FROM event_subscription ORDER BY created_at DESC, id",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(Self::subscription_row).collect()
    }

    /// Fetch one subscription by id, or `NotFound`.
    async fn get_subscription(&self, id: Uuid) -> Result<Value, ServiceError> {
        let row = sqlx::query(
            "SELECT id, name, kind, change_type, template_id, archetype, enabled, created_at \
             FROM event_subscription WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("event subscription {id}")))?;
        Self::subscription_row(&row)
    }

    /// Create a subscription from a JSON body. `name` is required + validated;
    /// the four predicates are optional (absent/`null` = wildcard); `enabled`
    /// defaults to `true`. A duplicate name is a `Conflict`.
    async fn create_subscription(&self, body: &Value) -> Result<Value, ServiceError> {
        let name = validated_name(body)?;
        let row = sqlx::query(
            "INSERT INTO event_subscription \
             (name, kind, change_type, template_id, archetype, enabled) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING id, name, kind, change_type, template_id, archetype, enabled, created_at",
        )
        .bind(&name)
        .bind(predicate(body, "kind"))
        .bind(predicate(body, "change_type"))
        .bind(predicate(body, "template_id"))
        .bind(predicate(body, "archetype"))
        .bind(enabled_flag(body))
        .fetch_one(&self.pool)
        .await
        .map_err(map_insert_error)?;
        Self::subscription_row(&row)
    }

    /// Replace a subscription's predicates + `enabled` (its `name` is immutable —
    /// it is the queue key). `NotFound` if the id is unknown.
    async fn update_subscription(&self, id: Uuid, body: &Value) -> Result<Value, ServiceError> {
        let row = sqlx::query(
            "UPDATE event_subscription \
             SET kind = $2, change_type = $3, template_id = $4, archetype = $5, enabled = $6 \
             WHERE id = $1 \
             RETURNING id, name, kind, change_type, template_id, archetype, enabled, created_at",
        )
        .bind(id)
        .bind(predicate(body, "kind"))
        .bind(predicate(body, "change_type"))
        .bind(predicate(body, "template_id"))
        .bind(predicate(body, "archetype"))
        .bind(enabled_flag(body))
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("event subscription {id}")))?;
        Self::subscription_row(&row)
    }

    /// Delete a subscription by id. `NotFound` if the id is unknown.
    ///
    /// PORT NOTE: the broker queue the deleted subscription bound is not torn
    /// down here — the service is broker-free. A durable queue simply stops
    /// being (re)bound; operators reap orphaned queues out of band. Re-binding
    /// of the *remaining* subscriptions is the drainer's job.
    async fn delete_subscription(&self, id: Uuid) -> Result<(), ServiceError> {
        let deleted = sqlx::query("DELETE FROM event_subscription WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(ServiceError::NotFound(format!("event subscription {id}")));
        }
        Ok(())
    }
}

/// Read `name` from the body and validate it: non-empty, and restricted to
/// `[A-Za-z0-9_.-]` so it is a clean AMQP queue-name suffix
/// (`ehrbase.events.<name>`).
fn validated_name(body: &Value) -> Result<String, ServiceError> {
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ServiceError::BadRequest("event subscription requires a non-empty 'name'".to_owned())
        })?;
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err(ServiceError::BadRequest(
            "event subscription 'name' must match [A-Za-z0-9_.-]".to_owned(),
        ));
    }
    Ok(name.to_owned())
}

/// A predicate field from the body: a non-empty string, else `None` (wildcard).
/// An absent field, JSON `null`, and an empty string are all "wildcard".
fn predicate(body: &Value, key: &str) -> Option<String> {
    body.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// The `enabled` flag from the body (default `true` when absent).
fn enabled_flag(body: &Value) -> bool {
    body.get("enabled").and_then(Value::as_bool).unwrap_or(true)
}

/// Map an INSERT failure: a unique-name violation is a client `Conflict` (409);
/// anything else is the underlying DB error.
fn map_insert_error(e: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db) = &e
        && db.is_unique_violation()
    {
        return ServiceError::Conflict("an event subscription with that name exists".to_owned());
    }
    ServiceError::Database(e)
}

impl EhrbaseService {
    /// List every stored event subscription (newest first) as PHI-free JSON
    /// records.
    ///
    /// # Errors
    /// [`SmError`] wrapping a database failure.
    pub async fn event_subscription_list(&self) -> Result<Vec<Value>, SmError> {
        Ok(self.list_subscriptions().await?)
    }

    /// Create an event subscription from a JSON body (`name` required; the
    /// predicates optional, NULL = wildcard; `enabled` defaults `true`).
    ///
    /// # Errors
    /// `BadRequest` when `name` is missing/empty or not `[A-Za-z0-9_.-]`;
    /// `Conflict` on a duplicate name; otherwise a database failure.
    pub async fn event_subscription_create(&self, a_subscription: Value) -> Result<Value, SmError> {
        Ok(self.create_subscription(&a_subscription).await?)
    }

    /// Fetch one event subscription by id.
    ///
    /// # Errors
    /// `NotFound` when the id is unknown; otherwise a database failure.
    pub async fn event_subscription_get(&self, a_subscription_id: Uuid) -> Result<Value, SmError> {
        Ok(self.get_subscription(a_subscription_id).await?)
    }

    /// Replace a subscription's predicates + `enabled` (the `name` is
    /// immutable — it is the queue key).
    ///
    /// # Errors
    /// `NotFound` when the id is unknown; otherwise a database failure.
    pub async fn event_subscription_update(
        &self,
        a_subscription_id: Uuid,
        a_subscription: Value,
    ) -> Result<Value, SmError> {
        Ok(self
            .update_subscription(a_subscription_id, &a_subscription)
            .await?)
    }

    /// Delete an event subscription by id (the broker queue is not torn down —
    /// see the PORT NOTE on the private helper).
    ///
    /// # Errors
    /// `NotFound` when the id is unknown; otherwise a database failure.
    pub async fn event_subscription_delete(&self, a_subscription_id: Uuid) -> Result<(), SmError> {
        Ok(self.delete_subscription(a_subscription_id).await?)
    }
}
