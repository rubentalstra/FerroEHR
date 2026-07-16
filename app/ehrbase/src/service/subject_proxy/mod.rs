//! The Subject Proxy Service engine (`I_SUBJECT_PROXY_SERVICE`,
//! `i_subject_proxy_service.adoc`; master10 `subject_proxy_service`): the
//! `SubjectProxyService` impl on [`EhrbaseService`] over the `sp_*` config +
//! sample stores.
//!
//! Design + gap register: `docs/design/sm-platform/10-subject-proxy.md` (W-3c).
//! The retrieval engine (frame dispatch, primary→fallback, subject-id
//! resolution) is in [`frames`]; currency/freshness in [`freshness`];
//! `frame_path` extraction in [`extract`]; the `sp_*` row mapping in [`store`].
//!
//! PORT NOTE (design-filled preconditions/errors). The SM declares only
//! `__Pre_…__` clauses and no error codes; every unmet precondition surfaces as
//! `SmError(PreconditionViolation, …)` (→ `400`).

pub(crate) mod config;
mod extract;
mod frames;
mod freshness;
mod store;

pub mod binding;
pub mod data_set;
pub mod sample;
pub mod value;
pub mod variable;

pub use config::{SpFhirSystem, SubjectProxyConfig, SubjectProxyFhir};

use serde_json::Value;
use sqlx::PgPool;

use crate::service::status::SmError;
use crate::service::subject_proxy::binding::EnvBinding;
use crate::service::subject_proxy::data_set::{DataSetResult, SubjectDataSet};
use crate::service::subject_proxy::sample::{DataFrameSample, Sample, VariableSample};
use crate::service::subject_proxy::value::VariableValue;
use crate::service::subject_proxy::variable::SubjectVariable;

use super::EhrbaseService;
use store::db_err;

impl EhrbaseService {
    /// The connection pool, shared with the `sp_*` store methods (cheap `Arc`
    /// clone; a borrow of the temporary satisfies `sqlx`'s `Executor`).
    pub(super) fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// Whether the newest stored `sample` still satisfies the variable's
    /// `currency` (master10 §Samples). Unset currency ⇒ "most recent available
    /// is valid" (`subject_variable.adoc`), so any stored sample serves.
    fn sp_sample_is_fresh(currency: Option<&str>, sample: &VariableSample) -> bool {
        match currency {
            None => true,
            Some(cur) => match freshness::parse_currency(cur) {
                Ok(span) => {
                    let at = sample
                        .effective_time
                        .as_deref()
                        .unwrap_or(&sample.retrieve_time);
                    freshness::is_fresh(at, &span)
                }
                // An unparseable stored currency: force a refresh (fail-closed).
                Err(_) => false,
            },
        }
    }

    /// Resolve the current [`VariableSample`] for `var`: serve the newest stored
    /// sample when it is fresh; otherwise (stale / no sample) execute the frame,
    /// extract the typed value, record the attempt, and return the new sample.
    /// A manual variable (`is_manual`) has no frame — it is served from the
    /// store only (its samples arrive via `notify_variable_sample`).
    async fn sp_resolve_sample(
        &self,
        subject_id: &str,
        var: &SubjectVariable,
    ) -> Result<VariableSample, SmError> {
        let canonical = var.canonical_name();
        let latest = self.sp_latest_sample(subject_id, &canonical).await?;

        if let Some((sample, _)) = &latest
            && Self::sp_sample_is_fresh(var.currency.as_deref(), sample)
        {
            return Ok(sample.clone());
        }

        if var.is_manual {
            // No frame to execute — serve the latest stored sample even if stale
            // (a manual variable's freshness is the caller's responsibility).
            return Ok(latest.map_or_else(
                || {
                    Sample::unavailable(
                        "manual variable has no sample yet (push one via notify_variable_sample)",
                    )
                },
                |(sample, _)| sample,
            ));
        }

        // Stale or never sampled: execute the frame and record the attempt.
        let frame_sample: DataFrameSample = self
            .get_frame(subject_id.to_owned(), var.frame_id.clone())
            .await?;

        let var_sample = build_variable_sample(&frame_sample, var);
        self.sp_record_sample(
            subject_id,
            &canonical,
            Some(&var.frame_id),
            &var_sample,
            Some(&frame_sample),
        )
        .await?;
        Ok(var_sample)
    }

    /// Load a data set's stored variable map (local alias → definition).
    async fn sp_data_set_variables(
        &self,
        subject_id: &str,
        data_set_id: &str,
    ) -> Result<Option<Vec<SubjectVariable>>, SmError> {
        let vars: Option<Value> = sqlx::query_scalar(
            "SELECT variables FROM sp_data_set WHERE subject_id = $1 AND id = $2",
        )
        .bind(subject_id)
        .bind(data_set_id)
        .fetch_optional(&self.pool())
        .await
        .map_err(db_err)?;
        let Some(vars) = vars else { return Ok(None) };
        let map: std::collections::BTreeMap<String, SubjectVariable> = serde_json::from_value(vars)
            .map_err(|e| SmError::exception(format!("stored data-set variables malformed: {e}")))?;
        Ok(Some(map.into_values().collect()))
    }
}

/// Turn a producing [`DataFrameSample`] into the [`VariableSample`] to record:
/// an unavailable frame carries its reason forward; an available frame is
/// extracted through `frame_path`/`type_name` (a typing failure becomes an
/// unavailable sample with the reason, never a silently wrong value).
fn build_variable_sample(frame_sample: &DataFrameSample, var: &SubjectVariable) -> VariableSample {
    if frame_sample.is_unavailable {
        let reason = frame_sample
            .unavailable_reason
            .clone()
            .unwrap_or_else(|| "frame retrieve unavailable".to_owned());
        return Sample::unavailable(reason);
    }
    match extract::extract_value(frame_sample, &var.frame_path, &var.type_name) {
        Ok(value) => {
            let sample = Sample::available(value);
            match &frame_sample.effective_time {
                Some(effective) => sample.with_effective_time(effective.clone()),
                None => sample,
            }
        }
        Err(reason) => Sample::unavailable(reason),
    }
}

impl EhrbaseService {
    pub async fn register_subject(
        &self,
        subject_id: String,
        subject_category: Option<String>,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__: not has_subject(subject_id).
        if self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is already registered"
            )));
        }
        // subject_category is "currently not controlled" — free text, default
        // 'individual' (subject_proxy.adoc).
        let category = subject_category.unwrap_or_else(|| "individual".to_owned());
        sqlx::query("INSERT INTO sp_subject (subject_id, subject_category) VALUES ($1, $2)")
            .bind(&subject_id)
            .bind(&category)
            .execute(&self.pool())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    pub async fn add_subject_variable(
        &self,
        subject_id: String,
        var: SubjectVariable,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__: has_subject(subject_id).
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        self.sp_upsert_variable(&subject_id, &var, true).await
    }

    pub async fn register_application_data_set(
        &self,
        definition: SubjectDataSet,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__: has_subject (the subject_id is inside the
        // definition — spec imprecision noted in the trait docs).
        if !self.sp_has_subject(&definition.subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {:?} is not registered",
                definition.subject_id
            )));
        }
        let subject_id = &definition.subject_id;

        // "This may have the effect of creating new subject variables, reducing
        // the currency of existing subject variables, if the currency is lower
        // …, or making no change for variables that already exist."
        for var in definition.variables.values() {
            let canonical = var.canonical_name();
            match self.sp_variable(subject_id, &canonical).await? {
                Some(existing) => {
                    let tightened = freshness::tighter_currency(
                        existing.currency.as_deref(),
                        var.currency.as_deref(),
                    );
                    if tightened != existing.currency {
                        self.sp_set_currency(subject_id, &canonical, tightened.as_deref())
                            .await?;
                    }
                }
                None => self.sp_upsert_variable(subject_id, var, false).await?,
            }
        }

        // Maintain using_app_ids: the creating app is always a user (G-10).
        let mut using: Vec<String> = definition.using_app_ids.clone();
        if let Some(app) = &definition.creating_app_id
            && !using.contains(app)
        {
            using.push(app.clone());
        }
        let using_json = serde_json::to_value(&using)
            .map_err(|e| SmError::exception(format!("serialize using_app_ids: {e}")))?;
        let vars_json = serde_json::to_value(&definition.variables)
            .map_err(|e| SmError::exception(format!("serialize data-set variables: {e}")))?;

        sqlx::query(
            "INSERT INTO sp_data_set (subject_id, id, creating_app_id, using_app_ids, variables) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (subject_id, id) DO UPDATE SET \
               creating_app_id = EXCLUDED.creating_app_id, \
               variables = EXCLUDED.variables, \
               using_app_ids = ( \
                 SELECT COALESCE(jsonb_agg(DISTINCT e), '[]'::jsonb) \
                 FROM jsonb_array_elements(sp_data_set.using_app_ids || EXCLUDED.using_app_ids) AS e)",
        )
        .bind(subject_id)
        .bind(&definition.id)
        .bind(definition.creating_app_id.as_deref())
        .bind(&using_json)
        .bind(&vars_json)
        .execute(&self.pool())
        .await
        .map_err(db_err)?;
        Ok(())
    }

    pub async fn remove_application_data_set(
        &self,
        subject_id: String,
        application_id: String,
    ) -> Result<(), SmError> {
        // __Pre_subject_valid__ + __Pre_application_valid__.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        if !self.sp_has_application(&application_id).await? {
            return Err(SmError::precondition(format!(
                "application {application_id:?} is not registered"
            )));
        }
        // Retract the app from its data sets for this subject, then drop any
        // whose user list has emptied (subject_data_set.adoc: "dump the data set
        // when empty").
        sqlx::query(
            "UPDATE sp_data_set SET using_app_ids = using_app_ids - $2 \
             WHERE subject_id = $1 AND using_app_ids ? $2",
        )
        .bind(&subject_id)
        .bind(&application_id)
        .execute(&self.pool())
        .await
        .map_err(db_err)?;
        sqlx::query(
            "DELETE FROM sp_data_set WHERE subject_id = $1 AND using_app_ids = '[]'::jsonb",
        )
        .bind(&subject_id)
        .execute(&self.pool())
        .await
        .map_err(db_err)?;
        Ok(())
    }

    pub async fn remove_subject_proxy(&self, subject_id: String) -> Result<(), SmError> {
        // __Pre_subject_valid__: has_subject.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        // Cascades sp_variable, sp_data_set and sp_sample.
        sqlx::query("DELETE FROM sp_subject WHERE subject_id = $1")
            .bind(&subject_id)
            .execute(&self.pool())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    pub async fn remove_application(&self, application_id: String) -> Result<(), SmError> {
        // __Pre_application_valid__: has_application.
        if !self.sp_has_application(&application_id).await? {
            return Err(SmError::precondition(format!(
                "application {application_id:?} is not registered"
            )));
        }
        // Retract the app across all subjects; drop the data sets it emptied.
        sqlx::query(
            "UPDATE sp_data_set SET using_app_ids = using_app_ids - $1 WHERE using_app_ids ? $1",
        )
        .bind(&application_id)
        .execute(&self.pool())
        .await
        .map_err(db_err)?;
        sqlx::query("DELETE FROM sp_data_set WHERE using_app_ids = '[]'::jsonb")
            .execute(&self.pool())
            .await
            .map_err(db_err)?;
        Ok(())
    }

    pub async fn get_variable(
        &self,
        subject_id: String,
        var_name: String,
    ) -> Result<VariableValue, SmError> {
        // __Pre_subject_valid__: has_subject.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        // Resolve var_name as a data-set-local alias first, then as a canonical
        // name (master10 §Subject Variable Naming).
        let canonical = self
            .sp_resolve_alias(&subject_id, &var_name)
            .await?
            .unwrap_or(var_name);
        let Some(var) = self.sp_variable(&subject_id, &canonical).await? else {
            return Err(SmError::precondition(format!(
                "no subject variable {canonical:?} for subject {subject_id:?}"
            )));
        };
        let sample = self.sp_resolve_sample(&subject_id, &var).await?;
        // SAMPLE.result is the extracted VARIABLE_VALUE; an unavailable sample
        // has none — the empty atomic value.
        Ok(sample.result.unwrap_or_else(VariableValue::none))
    }

    pub async fn get_data_set(
        &self,
        subject_id: String,
        data_set_id: String,
    ) -> Result<DataSetResult, SmError> {
        // __Pre_subject_valid__: has_subject.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        let Some(variables) = self
            .sp_data_set_variables(&subject_id, &data_set_id)
            .await?
        else {
            return Err(SmError::precondition(format!(
                "no data set {data_set_id:?} for subject {subject_id:?}"
            )));
        };
        // Sample each variable the same way get_variable does.
        let mut samples = Vec::with_capacity(variables.len());
        for var in &variables {
            samples.push(self.sp_resolve_sample(&subject_id, var).await?);
        }
        Ok(DataSetResult {
            name: data_set_id,
            subject_id,
            variables: samples,
        })
    }

    pub async fn has_subject(&self, subject_id: String) -> Result<bool, SmError> {
        self.sp_has_subject(&subject_id).await
    }

    pub async fn has_application(&self, application_id: String) -> Result<bool, SmError> {
        self.sp_has_application(&application_id).await
    }

    pub async fn get_variable_defs(&self, subject_id: String) -> Result<Vec<String>, SmError> {
        // "a list of variable definitions each of the form 'name: Type', where
        // 'name' is the canonical name" (i_subject_proxy_service.adoc).
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT canonical_name, type_name FROM sp_variable \
             WHERE subject_id = $1 ORDER BY canonical_name",
        )
        .bind(&subject_id)
        .fetch_all(&self.pool())
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|(name, type_name)| format!("{name}: {type_name}"))
            .collect())
    }

    pub async fn register_binding(&self, binding: EnvBinding) -> Result<(), SmError> {
        // __Pre_new_env__: not has_binding(binding.env_id).
        if self.sp_has_binding(&binding.env_id).await? {
            return Err(SmError::precondition(format!(
                "environment binding {:?} is already registered",
                binding.env_id
            )));
        }
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        sqlx::query("INSERT INTO sp_binding (env_id, description) VALUES ($1, $2)")
            .bind(&binding.env_id)
            .bind(binding.description.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        for frame in &binding.data_frames {
            store::insert_frame(&mut tx, &binding.env_id, frame).await?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn add_binding_frame(
        &self,
        env_id: String,
        frame: crate::service::subject_proxy::binding::DataFrame,
    ) -> Result<(), SmError> {
        // __Pre_valid_binding__: has_binding(env_id).
        if !self.sp_has_binding(&env_id).await? {
            return Err(SmError::precondition(format!(
                "environment binding {env_id:?} is not registered"
            )));
        }
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        store::insert_frame(&mut tx, &env_id, &frame).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn has_binding(&self, env_id: String) -> Result<bool, SmError> {
        self.sp_has_binding(&env_id).await
    }

    pub async fn reset(&self) -> Result<(), SmError> {
        // "Set back to virgin state … remove all subjects, variables and
        // bindings" (master10 §Persistence). Subjects cascade to variables,
        // data sets and samples; bindings cascade to data frames.
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        sqlx::query("DELETE FROM sp_subject")
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx::query("DELETE FROM sp_binding")
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    pub async fn notify_variable_sample(
        &self,
        subject_id: String,
        var_name: String,
        sample: VariableSample,
    ) -> Result<(), SmError> {
        // Extension precondition (design-filled): has_subject, the variable
        // exists, and it is is_manual or ask_user.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        let canonical = self
            .sp_resolve_alias(&subject_id, &var_name)
            .await?
            .unwrap_or(var_name);
        let Some(var) = self.sp_variable(&subject_id, &canonical).await? else {
            return Err(SmError::precondition(format!(
                "no subject variable {canonical:?} for subject {subject_id:?}"
            )));
        };
        if !(var.is_manual || var.ask_user.unwrap_or(false)) {
            return Err(SmError::precondition(format!(
                "subject variable {canonical:?} is not manual/ask_user — it cannot accept a \
                 pushed sample"
            )));
        }
        // A pushed sample is not frame-driven (no producing DATA_FRAME_SAMPLE).
        self.sp_record_sample(&subject_id, &canonical, None, &sample, None)
            .await
    }

    pub async fn get_subject_variable(
        &self,
        subject_id: String,
        var_name: String,
    ) -> Result<SubjectVariable, SmError> {
        // Extension precondition (design-filled): has_subject and the variable
        // exists.
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        let canonical = self
            .sp_resolve_alias(&subject_id, &var_name)
            .await?
            .unwrap_or(var_name);
        let Some(mut var) = self.sp_variable(&subject_id, &canonical).await? else {
            return Err(SmError::precondition(format!(
                "no subject variable {canonical:?} for subject {subject_id:?}"
            )));
        };
        // Materialise the runtime sample state (SUBJECT_VARIABLE.history +
        // last_frame) from the sample store (subject_variable.adoc).
        let history = self.sp_sample_history(&subject_id, &canonical).await?;
        var.last_frame = history.first().and_then(|(_, frame)| frame.clone());
        var.history = history.into_iter().map(|(sample, _)| sample).collect();
        Ok(var)
    }
}
