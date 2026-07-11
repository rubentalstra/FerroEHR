//! SM-6 Subject Proxy Service — `I_SUBJECT_PROXY_SERVICE` +
//! `I_DATA_BINDING` (SM master10) over the `sp_*` configuration stores
//! (migration `0010_subject_proxy_stores.sql`).
//!
//! Spec: `docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
//! and its included `UML/classes/*.adoc`. Catalog traits + information
//! structures: [`ehrbase_sm::services::subject_proxy`]. Design:
//! `docs/design/sm-platform/08-target-architecture.md` §4.4.
//!
//! ## Realized path — the openEHR frame
//!
//! The only executed retrieval method is the openEHR frame
//! ([`FrameMethod::Aql`]): [`DataBinding::get_frame`] binds the `subject_id`
//! into the frame's AQL as `$subject_id` and runs it through the existing Query
//! seam ([`EhrbaseService::execute_aql`], the same engine the REST QUERY API
//! uses), yielding an [`FramePayload::Openehr`] carrying the ITS-REST 1.0.3
//! `RESULT_SET`. [`SubjectProxyService::get_variable`] then extracts the value at
//! the variable's `frame_path`.
//!
//! ## Stubbed seams
//!
//! FHIR / `HL7v2` frames are carried verbatim in the store but never executed:
//! `get_frame` rejects them `NotImplemented` (the "FHIR/HL7v2 frame seams
//! stubbed as typed rejections" of the SM-6 task).
//!
//! ## PORT NOTEs
//!
//! - **`frame_path` semantics.** The SM types `SUBJECT_VARIABLE.frame_path` only
//!   as "Path within `last_frame` result" — undefined for a `RESULT_SET`. We
//!   define it as a **`RESULT_SET` column selector** (matched against a column's
//!   `name`): 0 rows ⇒ `VARIABLE_VALUE_SINGLE{None}`, 1 row ⇒ `…SINGLE{value}`,
//!   many rows ⇒ `VARIABLE_VALUE_LIST`. The time-series form
//!   (`VARIABLE_VALUE_TIME_SERIES`, needing a paired time column) is deferred.
//! - **currency / freshness.** `SUBJECT_VARIABLE.currency` +
//!   `SAMPLE.effective_time` freshness caching (design 08 §4.4 `sp_sample`
//!   store) is deferred; every `get_variable`/`get_data_set` re-executes the
//!   bound frame — always "most recent available", which `currency = Void`
//!   explicitly permits (`subject_variable.adoc`).
//! - **`register_application_data_set` currency-tightening.** The spec's
//!   "reducing the currency of existing subject variables, if the currency is
//!   lower" branch is deferred: openEHR ISO-8601 durations (nominal
//!   months/years) have no total order without a reference instant. Data-set
//!   registration therefore takes the "creating new subject variables … or
//!   making no change for variables that already exist" branch (insert-if-absent).
//! - **subject-id resolution.** `I_DATA_BINDING.get_frame`'s own TODO — the
//!   `subject_id` "might … be an identifier of an information resource …, e.g.
//!   an EHR identifier". We bind it into the frame AQL as `$subject_id` and, when
//!   it parses as a UUID, scope the query to that EHR; no external
//!   identity-resolution service (MPI) is consulted.
//! - **no wire.** Native-API-only (ITS-REST vendors no Subject Proxy endpoints);
//!   extension routes + YAML/JSON ingestion are a later SM-6 wave.

use async_trait::async_trait;
use serde_json::Value;
use sqlx::Row;

use ehrbase_sm::{
    AqlQueryRequest, CallStatusType, DataBinding, DataFrame, DataFrameSample, DataSetResult,
    EnvBinding, FrameMethod, FramePayload, Sample, SmError, SubjectDataSet, SubjectProxyService,
    SubjectVariable, VariableSample, VariableValue,
};

use super::{EhrbaseService, ServiceError};

/// Map a persistence failure to the SM `exception` status (server fault).
fn db_err(e: impl Into<ServiceError>) -> SmError {
    SmError::from(e.into())
}

impl EhrbaseService {
    /// Whether a subject proxy is registered.
    async fn sp_has_subject(&self, subject_id: &str) -> Result<bool, SmError> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sp_subject WHERE subject_id = $1)")
            .bind(subject_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)
    }

    /// Whether an application has registered a data set.
    async fn sp_has_application(&self, application_id: &str) -> Result<bool, SmError> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sp_data_set WHERE creating_app_id = $1)")
            .bind(application_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)
    }

    /// Whether an environment binding is registered.
    async fn sp_has_binding(&self, env_id: &str) -> Result<bool, SmError> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sp_binding WHERE env_id = $1)")
            .bind(env_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_err)
    }

    /// Load a subject's variable definition by canonical name.
    async fn sp_variable(
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

    /// Insert (or, for `add_subject_variable`, replace) a subject variable.
    async fn sp_upsert_variable(
        &self,
        subject_id: &str,
        var: &SubjectVariable,
        replace: bool,
    ) -> Result<(), SmError> {
        // `replace` (add_subject_variable) overwrites an existing definition;
        // otherwise (register_application_data_set) create-if-absent, leaving
        // existing definitions unchanged (see the currency-tightening PORT NOTE).
        // Two literal statements keep the SQL static (sqlx `SqlSafeStr`).
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
        // Subject-variable naming validity (SM master10 §Subject Variable
        // Naming): no whitespace / unprintable characters in the canonical
        // name (namespace + name); reject before storing.
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
            .map_err(db_err)?;
        Ok(())
    }

    /// Resolve a variable through its bound frame into a [`VariableValue`]:
    /// execute the frame for `subject_id`, then extract `frame_path` from the
    /// result. Shared by `get_variable` and `get_data_set`.
    async fn sp_resolve_value(
        &self,
        subject_id: &str,
        var: &SubjectVariable,
    ) -> Result<VariableValue, SmError> {
        let sample = self
            .get_frame(subject_id.to_owned(), var.frame_id.clone())
            .await?;
        Ok(extract_frame_value(&sample, &var.frame_path))
    }
}

#[async_trait]
impl SubjectProxyService for EhrbaseService {
    async fn register_subject(
        &self,
        subject_id: String,
        subject_category: Option<String>,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__: not has_subject(subject_id).
        if self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is already registered (not has_subject)"
            )));
        }
        let category = subject_category.unwrap_or_else(|| "individual".to_owned());
        sqlx::query("INSERT INTO sp_subject (subject_id, subject_category) VALUES ($1, $2)")
            .bind(&subject_id)
            .bind(&category)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn add_subject_variable(
        &self,
        subject_id: String,
        var: SubjectVariable,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__: has_subject(subject_id).
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "no subject {subject_id:?} (has_subject precondition failed)"
            )));
        }
        self.sp_upsert_variable(&subject_id, &var, true).await
    }

    async fn register_application_data_set(
        &self,
        definition: SubjectDataSet,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__: has_subject(subject_id) — the subject_id is
        // inside `definition` (spec imprecision, digest §2.2).
        if !self.sp_has_subject(&definition.subject_id).await? {
            return Err(SmError::precondition(format!(
                "no subject {:?} for data set {:?} (has_subject precondition failed)",
                definition.subject_id, definition.id
            )));
        }
        let variables_json = serde_json::to_value(&definition.variables)
            .map_err(|e| SmError::exception(format!("serialize data-set variables: {e}")))?;
        let using_app_ids = serde_json::to_value(&definition.using_app_ids)
            .map_err(|e| SmError::exception(format!("serialize using_app_ids: {e}")))?;
        sqlx::query(
            "INSERT INTO sp_data_set (subject_id, id, creating_app_id, using_app_ids, variables) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (subject_id, id) DO UPDATE SET \
             creating_app_id = EXCLUDED.creating_app_id, \
             using_app_ids = EXCLUDED.using_app_ids, variables = EXCLUDED.variables",
        )
        .bind(&definition.subject_id)
        .bind(&definition.id)
        .bind(definition.creating_app_id.as_deref())
        .bind(&using_app_ids)
        .bind(&variables_json)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        // "This may have the effect of creating new subject variables … or
        // making no change for variables that already exist" — insert-if-absent
        // the canonical variables (currency-tightening branch deferred, PORT NOTE).
        for var in definition.variables.values() {
            self.sp_upsert_variable(&definition.subject_id, var, false)
                .await?;
        }
        Ok(())
    }

    async fn remove_application_data_set(
        &self,
        subject_id: String,
        application_id: String,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__: has_subject; __Pre_application_valid__:
        // has_application.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "no subject {subject_id:?} (has_subject precondition failed)"
            )));
        }
        if !self.sp_has_application(&application_id).await? {
            return Err(SmError::precondition(format!(
                "no application {application_id:?} (has_application precondition failed)"
            )));
        }
        sqlx::query("DELETE FROM sp_data_set WHERE subject_id = $1 AND creating_app_id = $2")
            .bind(&subject_id)
            .bind(&application_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn remove_subject(&self, subject_id: String) -> Result<(), SmError> {
        // __Pre_subject_valid__: has_subject.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "no subject {subject_id:?} (has_subject precondition failed)"
            )));
        }
        // Variables + data sets cascade (FK ON DELETE CASCADE).
        sqlx::query("DELETE FROM sp_subject WHERE subject_id = $1")
            .bind(&subject_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn remove_application(&self, application_id: String) -> Result<(), SmError> {
        // __Pre_application_valid__: has_application.
        if !self.sp_has_application(&application_id).await? {
            return Err(SmError::precondition(format!(
                "no application {application_id:?} (has_application precondition failed)"
            )));
        }
        // "Remove all data-sets for application_id, across all subjects."
        sqlx::query("DELETE FROM sp_data_set WHERE creating_app_id = $1")
            .bind(&application_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn get_variable(
        &self,
        subject_id: String,
        var_name: String,
    ) -> Result<VariableValue, SmError> {
        // __Pre_subject_valid__: has_subject.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "no subject {subject_id:?} (has_subject precondition failed)"
            )));
        }
        let var = self
            .sp_variable(&subject_id, &var_name)
            .await?
            .ok_or_else(|| {
                SmError::precondition(format!(
                    "subject {subject_id:?} has no variable {var_name:?}"
                ))
            })?;
        self.sp_resolve_value(&subject_id, &var).await
    }

    async fn get_data_set(
        &self,
        subject_id: String,
        data_set_id: String,
    ) -> Result<DataSetResult, SmError> {
        // __Pre_subject_valid__: has_subject.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "no subject {subject_id:?} (has_subject precondition failed)"
            )));
        }
        let variables_json: Option<Value> = sqlx::query_scalar(
            "SELECT variables FROM sp_data_set WHERE subject_id = $1 AND id = $2",
        )
        .bind(&subject_id)
        .bind(&data_set_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        let variables_json = variables_json.ok_or_else(|| {
            SmError::precondition(format!(
                "subject {subject_id:?} has no data set {data_set_id:?}"
            ))
        })?;
        let variables: std::collections::BTreeMap<String, SubjectVariable> =
            serde_json::from_value(variables_json).map_err(|e| {
                SmError::exception(format!("stored data-set variables are malformed: {e}"))
            })?;

        // DATA_SET_RESULT.variables: List<VARIABLE_SAMPLE> — resolve each.
        let mut samples: Vec<VariableSample> = Vec::with_capacity(variables.len());
        for var in variables.values() {
            let value = self.sp_resolve_value(&subject_id, var).await?;
            samples.push(Sample::available(value));
        }
        Ok(DataSetResult {
            name: data_set_id,
            subject_id,
            variables: samples,
        })
    }

    async fn has_subject(&self, subject_id: String) -> Result<bool, SmError> {
        self.sp_has_subject(&subject_id).await
    }

    async fn has_application(&self, application_id: String) -> Result<bool, SmError> {
        self.sp_has_application(&application_id).await
    }

    async fn get_variable_defs(&self, subject_id: String) -> Result<Vec<String>, SmError> {
        // No precondition in the SM; entries are "name: Type".
        let rows = sqlx::query(
            "SELECT canonical_name, type_name FROM sp_variable WHERE subject_id = $1 \
             ORDER BY canonical_name",
        )
        .bind(&subject_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter()
            .map(|r| {
                let name: String = r.try_get("canonical_name").map_err(db_err)?;
                let type_name: String = r.try_get("type_name").map_err(db_err)?;
                Ok(format!("{name}: {type_name}"))
            })
            .collect()
    }

    async fn register_binding(&self, binding: EnvBinding) -> Result<(), SmError> {
        // __Pre_new_env__: not has_binding(binding.env_id).
        if self.sp_has_binding(&binding.env_id).await? {
            return Err(SmError::precondition(format!(
                "environment binding {:?} is already registered (not has_binding)",
                binding.env_id
            )));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query("INSERT INTO sp_binding (env_id, description) VALUES ($1, $2)")
            .bind(&binding.env_id)
            .bind(binding.description.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        for frame in &binding.data_frames {
            insert_frame(&mut tx, &binding.env_id, frame).await?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn add_binding_frame(&self, env_id: String, frame: DataFrame) -> Result<(), SmError> {
        // __Pre_valid_binding__: has_binding(env_id).
        if !self.sp_has_binding(&env_id).await? {
            return Err(SmError::precondition(format!(
                "no environment binding {env_id:?} (has_binding precondition failed)"
            )));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        insert_frame(&mut tx, &env_id, &frame).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn has_binding(&self, env_id: String) -> Result<bool, SmError> {
        self.sp_has_binding(&env_id).await
    }

    async fn reset(&self) -> Result<(), SmError> {
        // master10 §Persistence: "remove all subjects, variables and bindings".
        // sp_variable/sp_data_set cascade from sp_subject; sp_data_frame from
        // sp_binding.
        sqlx::query("TRUNCATE sp_subject, sp_binding CASCADE")
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }
}

#[async_trait]
impl DataBinding for EhrbaseService {
    async fn get_frame(
        &self,
        subject_id: String,
        frame_id: String,
    ) -> Result<DataFrameSample, SmError> {
        // Resolve the frame service-wide by frame_id (UNIQUE across bindings).
        let row = sqlx::query(
            "SELECT model_type, primary_method, fallback_method FROM sp_data_frame \
             WHERE frame_id = $1",
        )
        .bind(&frame_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?
        .ok_or_else(|| SmError::precondition(format!("no data frame with id {frame_id:?}")))?;

        let model_type: String = row.try_get("model_type").map_err(db_err)?;
        let primary: Value = row.try_get("primary_method").map_err(db_err)?;
        let method: FrameMethod = serde_json::from_value(primary).map_err(|e| {
            SmError::exception(format!(
                "stored frame method for {frame_id:?} is malformed: {e}"
            ))
        })?;

        match method {
            FrameMethod::Aql { query_text } => {
                // Bind the subject id into the frame's AQL as $subject_id, and —
                // when it is a UUID — scope the query to that EHR. I_DATA_BINDING
                // notes the subject_id "might … be an identifier of an
                // information resource against which the query can be made, e.g.
                // an EHR identifier"; we realize exactly that (PORT NOTE: no
                // external identity-resolution step, no MPI lookup).
                let mut request = AqlQueryRequest::default();
                if uuid::Uuid::parse_str(&subject_id).is_ok() {
                    request.ehr_id = Some(subject_id.clone());
                }
                request
                    .parameters
                    .insert("subject_id".to_owned(), Value::String(subject_id));
                match self.execute_aql(&query_text, None, &request).await {
                    Ok(outcome) => Ok(Sample::available(FramePayload::Openehr {
                        result_set: outcome.result_set,
                    })),
                    // A query failure is a frame that could not retrieve — surface
                    // it as an unavailable sample carrying the reason, not a hard
                    // error (SAMPLE.is_unavailable / unavailable_reason).
                    Err(e) => Ok(Sample::unavailable(e.message)),
                }
            }
            // FHIR / HL7v2 frames are stubbed seams — typed rejection.
            FrameMethod::Fhir { .. } | FrameMethod::Hl7v2 { .. } => Err(SmError::new(
                CallStatusType::NotImplemented,
                format!(
                    "data frame {frame_id:?} (model_type {model_type:?}) uses a \
                     FHIR/HL7v2 retrieval method, which is not implemented"
                ),
            )),
        }
    }
}

/// Insert one `sp_data_frame` row within a transaction. A `frame_id` that
/// collides with an existing frame (the service-wide `UNIQUE (frame_id)`) is a
/// `precondition_violation`, not a 500.
async fn insert_frame(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    env_id: &str,
    frame: &DataFrame,
) -> Result<(), SmError> {
    let primary = serde_json::to_value(&frame.primary_method)
        .map_err(|e| SmError::exception(format!("serialize frame primary_method: {e}")))?;
    let fallback = frame
        .fallback_method
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| SmError::exception(format!("serialize frame fallback_method: {e}")))?;
    sqlx::query(
        "INSERT INTO sp_data_frame (env_id, frame_id, model_type, primary_method, fallback_method) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(env_id)
    .bind(&frame.id)
    .bind(&frame.model_type)
    .bind(&primary)
    .bind(&fallback)
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

/// Reassemble a [`SubjectVariable`] from an `sp_variable` row.
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
    })
}

/// Extract a [`VariableValue`] at `frame_path` from a data-frame sample.
///
/// `frame_path` is a `RESULT_SET` **column selector** (matched against a column's
/// `name`; PORT NOTE in the module docs): the values of that column across all
/// rows become `SINGLE{None}` (0 rows / unavailable / unknown column),
/// `SINGLE{value}` (1 row), or `LIST` (many rows).
fn extract_frame_value(sample: &DataFrameSample, frame_path: &str) -> VariableValue {
    let none = || VariableValue::Single { value: None };
    if sample.is_unavailable {
        return none();
    }
    let Some(FramePayload::Openehr { result_set }) = &sample.result else {
        return none();
    };
    let Some(columns) = result_set.get("columns").and_then(Value::as_array) else {
        return none();
    };
    let Some(idx) = columns
        .iter()
        .position(|c| c.get("name").and_then(Value::as_str) == Some(frame_path))
    else {
        return none();
    };
    let Some(rows) = result_set.get("rows").and_then(Value::as_array) else {
        return none();
    };
    let values: Vec<Value> = rows.iter().filter_map(|r| r.get(idx)).cloned().collect();
    match values.len() {
        0 => none(),
        1 => VariableValue::Single {
            value: values.into_iter().next(),
        },
        _ => VariableValue::List { value: values },
    }
}
