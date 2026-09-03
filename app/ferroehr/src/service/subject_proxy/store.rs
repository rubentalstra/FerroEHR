// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The `sp_*` configuration + sample stores (SM master10 §Persistence:
//! configuration survives for the life of the system; sample rows realize
//! `SUBJECT_VARIABLE.history`/`last_frame` — persisting them is permitted,
//! nothing in the spec forbids it, and it is what makes variables "tracked
//! over time" (master10 §Overview) real across restarts. No openEHR spec
//! governs the storage mechanics — our own design).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use serde_json::Value;
use sqlx::Row;

use crate::service::FerroEhrService;
use crate::service::error::{ServiceError, internal_fault};
use crate::service::status::SmError;
use crate::service::subject_proxy::binding::{DataFrame, SystemCall};
use crate::service::subject_proxy::sample::{DataFrameSample, VariableSample};
use crate::service::subject_proxy::variable::SubjectVariable;

/// Cap on retained samples per (subject, variable): newest N survive. No
/// openEHR spec governs retention — our own design (the history stays a
/// bounded ring, not an unbounded log).
const SAMPLE_RETENTION: i64 = 100;

/// Map a persistence failure to the SM `exception` status (server fault).
pub(super) fn db_err(e: impl Into<ServiceError>) -> SmError {
    SmError::from(e.into())
}

/// A loaded `sp_data_frame` row.
pub(super) struct FrameRow {
    pub frame: DataFrame,
}

impl FerroEhrService {
    /// Whether a subject proxy is registered.
    pub(super) async fn sp_has_subject(&self, subject_id: &str) -> Result<bool, SmError> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sp_subject WHERE subject_id = $1)")
            .bind(subject_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)
    }

    /// Whether an application has registered or uses any data set
    /// (`creating_app_id` or membership of `using_app_ids`).
    pub(super) async fn sp_has_application(&self, application_id: &str) -> Result<bool, SmError> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sp_data_set \
             WHERE creating_app_id = $1 OR using_app_ids ? $1)",
        )
        .bind(application_id)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)
    }

    /// Whether an environment binding is registered.
    pub(super) async fn sp_has_binding(&self, env_id: &str) -> Result<bool, SmError> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sp_binding WHERE env_id = $1)")
            .bind(env_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)
    }

    /// Load a subject's variable definition by canonical name.
    pub(super) async fn sp_variable(
        &self,
        subject_id: &str,
        canonical_name: &str,
    ) -> Result<Option<SubjectVariable>, SmError> {
        let row = sqlx::query(
            "SELECT namespace, name, type_name, currency, ask_user, is_manual, frame_id, \
             frame_path FROM sp_variable WHERE subject_id = $1 AND canonical_name = $2",
        )
        .bind(subject_id)
        .bind(canonical_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.map(|r| row_to_variable(&r)).transpose()
    }

    /// Resolve `var_name` as a data-set-local alias for `subject_id`: search
    /// the subject's data sets for a variable keyed by that local name and
    /// return its canonical name (master10 §Subject Variable Naming: an
    /// application data set "may however use data set-local aliases, for
    /// example `dob` for the canonical name `date_of_birth`").
    pub(super) async fn sp_resolve_alias(
        &self,
        subject_id: &str,
        local_name: &str,
    ) -> Result<Option<String>, SmError> {
        let def: Option<Value> = sqlx::query_scalar(
            "SELECT variables -> $2 FROM sp_data_set \
             WHERE subject_id = $1 AND variables ? $2 \
             ORDER BY id LIMIT 1",
        )
        .bind(subject_id)
        .bind(local_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .flatten();
        let Some(def) = def else { return Ok(None) };
        let var: SubjectVariable = serde_json::from_value(def)
            .map_err(|e| internal_fault("read a stored subject variable", &e))?;
        Ok(Some(var.canonical_name()))
    }

    /// Insert (or, for `add_subject_variable`, replace) a subject variable.
    /// Subject-variable naming validity (SM master10 §Subject Variable Naming:
    /// no whitespace / unprintable characters) is rejected before storing; an
    /// unknown `frame_id` surfaces as a named precondition failure (FK).
    pub(super) async fn sp_upsert_variable(
        &self,
        subject_id: &str,
        var: &SubjectVariable,
        replace: bool,
    ) -> Result<(), SmError> {
        const REPLACE_SQL: &str = "INSERT INTO sp_variable (subject_id, canonical_name, namespace, name, type_name, \
             currency, ask_user, is_manual, frame_id, frame_path) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (subject_id, canonical_name) DO UPDATE SET \
             namespace = EXCLUDED.namespace, name = EXCLUDED.name, \
             type_name = EXCLUDED.type_name, currency = EXCLUDED.currency, \
             ask_user = EXCLUDED.ask_user, is_manual = EXCLUDED.is_manual, \
             frame_id = EXCLUDED.frame_id, frame_path = EXCLUDED.frame_path";
        const INSERT_SQL: &str = "INSERT INTO sp_variable (subject_id, canonical_name, namespace, name, type_name, \
             currency, ask_user, is_manual, frame_id, frame_path) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (subject_id, canonical_name) DO NOTHING";
        if !var.name_valid() {
            return Err(SmError::precondition(format!(
                "subject variable name {:?} (namespace {:?}) is not a valid canonical name \
                 (no whitespace or unprintable characters; SM master10 §Subject Variable Naming)",
                var.name, var.namespace
            )));
        }
        let sql = if replace { REPLACE_SQL } else { INSERT_SQL };
        sqlx::query(sql)
            .bind(subject_id)
            .bind(var.canonical_name())
            .bind(var.namespace.as_deref())
            .bind(&var.name)
            .bind(&var.type_name)
            .bind(var.currency.as_deref())
            .bind(var.ask_user)
            .bind(var.is_manual)
            .bind(&var.frame_id)
            .bind(&var.frame_path)
            .execute(&self.pool)
            .await
            .map_err(|e| frame_fk_err(e, &var.frame_id))?;
        Ok(())
    }

    /// Tighten an existing variable's currency (the
    /// `register_application_data_set` "reducing the currency … if the
    /// currency is lower" branch).
    pub(super) async fn sp_set_currency(
        &self,
        subject_id: &str,
        canonical_name: &str,
        currency: Option<&str>,
    ) -> Result<(), SmError> {
        sqlx::query(
            "UPDATE sp_variable SET currency = $3 \
             WHERE subject_id = $1 AND canonical_name = $2",
        )
        .bind(subject_id)
        .bind(canonical_name)
        .bind(currency)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    /// Load a data frame by service-wide `frame_id`.
    pub(super) async fn sp_frame(&self, frame_id: &str) -> Result<Option<FrameRow>, SmError> {
        let row = sqlx::query(
            "SELECT frame_id, model_type, primary_method, fallback_method \
             FROM sp_data_frame WHERE frame_id = $1",
        )
        .bind(frame_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        let Some(row) = row else { return Ok(None) };
        let parse_method = |v: Option<Value>, which: &str| -> Result<Option<SystemCall>, SmError> {
            v.map(|v| {
                serde_json::from_value(v).map_err(|e| {
                    internal_fault(
                        "read a stored data-frame method",
                        &format!("{which} method of frame {frame_id:?}: {e}"),
                    )
                })
            })
            .transpose()
        };
        let primary: Option<Value> = row.try_get("primary_method").map_err(db_err)?;
        let fallback: Option<Value> = row.try_get("fallback_method").map_err(db_err)?;
        Ok(Some(FrameRow {
            frame: DataFrame {
                id: row.try_get("frame_id").map_err(db_err)?,
                model_type: row.try_get("model_type").map_err(db_err)?,
                primary_method: parse_method(primary, "primary")?,
                fallback_method: parse_method(fallback, "fallback")?,
            },
        }))
    }

    /// Resolve a subject id to an EHR id for openEHR-frame scoping
    /// (`i_data_binding.adoc`'s own TODO: "this service might need to resolve
    /// it through another service" — the EHR Index is that service,
    /// `master07-ehr_index_service.adoc`). Order: literal EHR id (UUID that
    /// exists in `ehr`), then the EHR Index by subject id (any namespace;
    /// `Primary` instances first). `None` = unresolved.
    pub(super) async fn sp_resolve_subject_ehr(
        &self,
        subject_id: &str,
    ) -> Result<Option<uuid::Uuid>, SmError> {
        if let Ok(id) = uuid::Uuid::parse_str(subject_id) {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM ehr WHERE id = $1)")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(db_err)?;
            if exists {
                return Ok(Some(id));
            }
        }
        sqlx::query_scalar(
            "SELECT ehr_id FROM ehr_index WHERE subject_id = $1 \
             ORDER BY (instance_type = 'Primary') DESC, created_at ASC LIMIT 1",
        )
        .bind(subject_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)
    }

    /// Record one retrieval attempt for a variable: the `VARIABLE_SAMPLE`
    /// (always) and the producing `DATA_FRAME_SAMPLE` (when frame-driven), then
    /// enforce the retention cap. "Every retrieval attempt will generate a new
    /// Sample object, regardless of whether data was actually available or
    /// not" (`sample.adoc`).
    pub(super) async fn sp_record_sample(
        &self,
        subject_id: &str,
        canonical_name: &str,
        frame_id: Option<&str>,
        sample: &VariableSample,
        frame_sample: Option<&DataFrameSample>,
    ) -> Result<(), SmError> {
        let sample_json = serde_json::to_value(sample)
            .map_err(|e| internal_fault("serialize a variable sample", &e))?;
        let frame_json = frame_sample
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| internal_fault("serialize a data-frame sample", &e))?;
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query(
            "INSERT INTO sp_sample (subject_id, canonical_name, frame_id, retrieve_time, \
             effective_time, is_unavailable, sample, frame_sample) \
             VALUES ($1, $2, $3, \
             COALESCE($4::timestamptz, now()), $5::timestamptz, $6, $7, $8)",
        )
        .bind(subject_id)
        .bind(canonical_name)
        .bind(frame_id)
        .bind(&sample.retrieve_time)
        .bind(sample.effective_time.as_deref())
        .bind(sample.is_unavailable)
        .bind(&sample_json)
        .bind(frame_json.as_ref())
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        // Retention: keep the newest SAMPLE_RETENTION rows per variable.
        sqlx::query(
            "DELETE FROM sp_sample WHERE id IN ( \
               SELECT id FROM sp_sample \
               WHERE subject_id = $1 AND canonical_name = $2 \
               ORDER BY retrieve_time DESC, id DESC OFFSET $3)",
        )
        .bind(subject_id)
        .bind(canonical_name)
        .bind(SAMPLE_RETENTION)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    /// The newest recorded sample for a variable (freshness candidate):
    /// `(variable sample, frame sample if frame-driven)`.
    pub(super) async fn sp_latest_sample(
        &self,
        subject_id: &str,
        canonical_name: &str,
    ) -> Result<Option<(VariableSample, Option<DataFrameSample>)>, SmError> {
        let row = sqlx::query(
            "SELECT sample, frame_sample FROM sp_sample \
             WHERE subject_id = $1 AND canonical_name = $2 \
             ORDER BY retrieve_time DESC, id DESC LIMIT 1",
        )
        .bind(subject_id)
        .bind(canonical_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.map(|r| parse_sample_row(&r)).transpose()
    }

    /// The retrieve history of a variable, newest first
    /// (`SUBJECT_VARIABLE.history` + `last_frame`).
    pub(super) async fn sp_sample_history(
        &self,
        subject_id: &str,
        canonical_name: &str,
    ) -> Result<Vec<(VariableSample, Option<DataFrameSample>)>, SmError> {
        let rows = sqlx::query(
            "SELECT sample, frame_sample FROM sp_sample \
             WHERE subject_id = $1 AND canonical_name = $2 \
             ORDER BY retrieve_time DESC, id DESC",
        )
        .bind(subject_id)
        .bind(canonical_name)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(parse_sample_row).collect()
    }
}

fn parse_sample_row(
    row: &sqlx::postgres::PgRow,
) -> Result<(VariableSample, Option<DataFrameSample>), SmError> {
    let sample: Value = row.try_get("sample").map_err(db_err)?;
    let frame: Option<Value> = row.try_get("frame_sample").map_err(db_err)?;
    let sample: VariableSample = serde_json::from_value(sample)
        .map_err(|e| internal_fault("read a stored variable sample", &e))?;
    let frame: Option<DataFrameSample> = frame
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| internal_fault("read a stored data-frame sample", &e))?;
    Ok((sample, frame))
}

/// Reassemble a [`SubjectVariable`] definition from an `sp_variable` row
/// (runtime `history`/`last_frame` are materialised separately).
fn row_to_variable(row: &sqlx::postgres::PgRow) -> Result<SubjectVariable, SmError> {
    Ok(SubjectVariable {
        namespace: row.try_get("namespace").map_err(db_err)?,
        name: row.try_get("name").map_err(db_err)?,
        type_name: row.try_get("type_name").map_err(db_err)?,
        currency: row.try_get("currency").map_err(db_err)?,
        ask_user: row.try_get("ask_user").map_err(db_err)?,
        is_manual: row.try_get("is_manual").map_err(db_err)?,
        frame_id: row.try_get("frame_id").map_err(db_err)?,
        frame_path: row.try_get("frame_path").map_err(db_err)?,
        history: Vec::new(),
        last_frame: None,
    })
}

/// Map a foreign-key violation on `sp_variable.frame_id` to a precondition
/// error naming the missing frame (a data-set variable may only bind an
/// existing frame), and everything else to `exception`.
fn frame_fk_err(e: sqlx::Error, frame_id: &str) -> SmError {
    match &e {
        // 23503 = foreign_key_violation.
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23503") => {
            SmError::precondition(format!(
                "variable binds unknown data frame {frame_id:?} (register the binding first)"
            ))
        }
        _ => db_err(e),
    }
}

/// Insert one `sp_data_frame` row within a transaction. A `frame_id` that
/// collides with an existing frame (the service-wide `UNIQUE (frame_id)`) is a
/// `precondition_violation`, not a 500.
pub(super) async fn insert_frame(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    env_id: &str,
    frame: &DataFrame,
) -> Result<(), SmError> {
    let primary = frame
        .primary_method
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| internal_fault("serialize a data-frame primary method", &e))?;
    let fallback = frame
        .fallback_method
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| internal_fault("serialize a data-frame fallback method", &e))?;
    sqlx::query(
        "INSERT INTO sp_data_frame (env_id, frame_id, model_type, primary_method, fallback_method) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(env_id)
    .bind(&frame.id)
    .bind(&frame.model_type)
    .bind(primary.as_ref())
    .bind(fallback.as_ref())
    .execute(&mut **tx)
    .await
    .map_err(|e| match &e {
        // 23505 = unique_violation (a duplicate frame_id).
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
            SmError::precondition(format!("data frame {:?} is already registered", frame.id))
        }
        _ => db_err(e),
    })?;
    Ok(())
}
