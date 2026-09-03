// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The `I_SUBJECT_PROXY_SERVICE` operations (`i_subject_proxy_service.adoc`;
//! master10 `subject_proxy_service`) on [`FerroEhrService`], plus the
//! sample-resolution glue shared by the variable/data-set reads.
//!
//! The retrieval engine (frame dispatch, primary→fallback, subject-id
//! resolution) is in [`super::frames`]; currency/freshness in
//! [`super::freshness`]; `frame_path` extraction in [`super::extract`]; the
//! `sp_*` row mapping in [`super::store`].

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use serde_json::Value;

use crate::service::FerroEhrService;
use crate::service::error::internal_fault;
use crate::service::status::SmError;
use crate::service::subject_proxy::binding::{DataFrame, EnvBinding};
use crate::service::subject_proxy::data_set::{DataSetResult, SubjectDataSet};
use crate::service::subject_proxy::sample::{DataFrameSample, Sample, VariableSample};
use crate::service::subject_proxy::store::db_err;
use crate::service::subject_proxy::value::VariableValue;
use crate::service::subject_proxy::variable::SubjectVariable;
use crate::service::subject_proxy::{extract, freshness, store};

impl FerroEhrService {
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
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        let Some(vars) = vars else { return Ok(None) };
        let map: std::collections::BTreeMap<String, SubjectVariable> = serde_json::from_value(vars)
            .map_err(|e| internal_fault("read the stored data-set variables", &e))?;
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

impl FerroEhrService {
    /// SM `register_subject`: register a subject proxy (default
    /// `subject_category = 'individual'` — `subject_proxy.adoc` notes the
    /// category is "currently not controlled", free text).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject is already registered
    ///   (`__Pre_subject_valid__: not has_subject`).
    /// - `exception` — a database fault while writing.
    pub async fn register_subject(
        &self,
        subject_id: String,
        subject_category: Option<String>,
    ) -> Result<(), SmError> {
        if self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is already registered"
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

    /// SM `add_subject_variable`: add (or replace) a variable definition for a
    /// registered subject.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject is not registered
    ///   (`__Pre_subject_valid__`), the variable name/namespace violates
    ///   master10 §Subject Variable Naming (whitespace/unprintable/empty), or
    ///   `frame_id` binds no registered data frame.
    /// - `exception` — a database fault while writing.
    pub async fn add_subject_variable(
        &self,
        subject_id: String,
        var: SubjectVariable,
    ) -> Result<(), SmError> {
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        self.sp_upsert_variable(&subject_id, &var, true).await
    }

    /// SM `register_application_data_set`: register (or refresh) an
    /// application data set for the subject named inside the definition.
    ///
    /// "This may have the effect of creating new subject variables, reducing
    /// the currency of existing subject variables, if the currency is lower …,
    /// or making no change for variables that already exist"
    /// (`i_subject_proxy_service.adoc`). The `using_app_ids` list is
    /// service-maintained: the creating app is always a user.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the definition's subject is not
    ///   registered (`__Pre_subject_valid__` — the `subject_id` is inside the
    ///   definition, a spec imprecision), a new variable's name is invalid, or
    ///   a variable binds an unknown data frame.
    /// - `exception` — a serialization or database fault while writing.
    pub async fn register_application_data_set(
        &self,
        definition: SubjectDataSet,
    ) -> Result<(), SmError> {
        if !self.sp_has_subject(&definition.subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {:?} is not registered",
                definition.subject_id
            )));
        }
        let subject_id = &definition.subject_id;

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

        // Maintain using_app_ids: the creating app is always a user.
        let mut using: Vec<String> = definition.using_app_ids.clone();
        if let Some(app) = &definition.creating_app_id
            && !using.contains(app)
        {
            using.push(app.clone());
        }
        let using_json = serde_json::to_value(&using)
            .map_err(|e| internal_fault("serialize the data-set app ids", &e))?;
        let vars_json = serde_json::to_value(&definition.variables)
            .map_err(|e| internal_fault("serialize the data-set variables", &e))?;

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
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    /// SM `remove_application_data_set`: retract an application from its data
    /// sets for a subject, then drop any data set whose user list has emptied
    /// (`subject_data_set.adoc`: "dump the data set when empty").
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject
    ///   (`__Pre_subject_valid__`) or the application
    ///   (`__Pre_application_valid__`) is not registered.
    /// - `exception` — a database fault while writing.
    pub async fn remove_application_data_set(
        &self,
        subject_id: String,
        application_id: String,
    ) -> Result<(), SmError> {
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
        sqlx::query(
            "UPDATE sp_data_set SET using_app_ids = using_app_ids - $2 \
             WHERE subject_id = $1 AND using_app_ids ? $2",
        )
        .bind(&subject_id)
        .bind(&application_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        sqlx::query(
            "DELETE FROM sp_data_set WHERE subject_id = $1 AND using_app_ids = '[]'::jsonb",
        )
        .bind(&subject_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    /// SM `remove_subject_proxy`: remove a subject proxy and everything under
    /// it (cascades `sp_variable`, `sp_data_set` and `sp_sample`).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject is not registered
    ///   (`__Pre_subject_valid__`).
    /// - `exception` — a database fault while writing.
    pub async fn remove_subject_proxy(&self, subject_id: String) -> Result<(), SmError> {
        if !self.sp_has_subject(&subject_id).await? {
            return Err(SmError::precondition(format!(
                "subject {subject_id:?} is not registered"
            )));
        }
        sqlx::query("DELETE FROM sp_subject WHERE subject_id = $1")
            .bind(&subject_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    /// SM `remove_application`: retract an application across all subjects and
    /// drop the data sets it emptied.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the application is not registered
    ///   (`__Pre_application_valid__`).
    /// - `exception` — a database fault while writing.
    pub async fn remove_application(&self, application_id: String) -> Result<(), SmError> {
        if !self.sp_has_application(&application_id).await? {
            return Err(SmError::precondition(format!(
                "application {application_id:?} is not registered"
            )));
        }
        sqlx::query(
            "UPDATE sp_data_set SET using_app_ids = using_app_ids - $1 WHERE using_app_ids ? $1",
        )
        .bind(&application_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        sqlx::query("DELETE FROM sp_data_set WHERE using_app_ids = '[]'::jsonb")
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    /// SM `get_variable`: the current value of a subject variable, `var_name`
    /// resolved as a data-set-local alias first, then as a canonical name
    /// (master10 §Subject Variable Naming). A fresh stored sample serves
    /// directly; otherwise the frame is executed and the attempt recorded.
    /// An unavailable sample yields the empty atomic value.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject is not registered
    ///   (`__Pre_subject_valid__`), or no variable with that (resolved) name
    ///   exists for the subject.
    /// - `not_implemented` — the variable's frame has no dispatchable executor
    ///   (no method, unknown `model_type`/`call_name`, or an unconfigured FHIR
    ///   system — see `super::frames`).
    /// - `exception` — a database/serialization fault while reading or
    ///   recording the sample.
    pub async fn get_variable(
        &self,
        subject_id: String,
        var_name: String,
    ) -> Result<VariableValue, SmError> {
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
        let sample = self.sp_resolve_sample(&subject_id, &var).await?;
        // SAMPLE.result is the extracted VARIABLE_VALUE; an unavailable sample
        // has none — the empty atomic value.
        Ok(sample.result.unwrap_or_else(VariableValue::none))
    }

    /// SM `get_data_set`: sample every variable of a registered data set (each
    /// resolved exactly as [`Self::get_variable`] resolves one) into a
    /// `DATA_SET_RESULT`.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject is not registered
    ///   (`__Pre_subject_valid__`), or no data set with `data_set_id` exists
    ///   for the subject.
    /// - `not_implemented` — a variable's frame has no dispatchable executor.
    /// - `exception` — a database/serialization fault while reading or
    ///   recording samples.
    pub async fn get_data_set(
        &self,
        subject_id: String,
        data_set_id: String,
    ) -> Result<DataSetResult, SmError> {
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

    /// SM `has_subject`: whether a subject proxy is registered.
    ///
    /// # Errors
    /// `exception` — a database fault while reading.
    pub async fn has_subject(&self, subject_id: String) -> Result<bool, SmError> {
        self.sp_has_subject(&subject_id).await
    }

    /// SM `has_application`: whether an application has registered or uses any
    /// data set.
    ///
    /// # Errors
    /// `exception` — a database fault while reading.
    pub async fn has_application(&self, application_id: String) -> Result<bool, SmError> {
        self.sp_has_application(&application_id).await
    }

    /// SM `get_variable_defs`: "a list of variable definitions each of the
    /// form 'name: Type', where 'name' is the canonical name"
    /// (`i_subject_proxy_service.adoc`), ordered by canonical name.
    ///
    /// # Errors
    /// `exception` — a database fault while reading.
    pub async fn get_variable_defs(&self, subject_id: String) -> Result<Vec<String>, SmError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT canonical_name, type_name FROM sp_variable \
             WHERE subject_id = $1 ORDER BY canonical_name",
        )
        .bind(&subject_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(rows
            .into_iter()
            .map(|(name, type_name)| format!("{name}: {type_name}"))
            .collect())
    }

    /// SM `register_binding`: register an environment binding and its data
    /// frames in one transaction (`env_binding.adoc`; master10 §Bindings).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the `env_id` is already registered
    ///   (`__Pre_new_env__`), or a frame's `frame_id` collides with an existing
    ///   frame (the service-wide unique frame namespace).
    /// - `exception` — a serialization or database fault while writing (rolled
    ///   back).
    pub async fn register_binding(&self, binding: EnvBinding) -> Result<(), SmError> {
        if self.sp_has_binding(&binding.env_id).await? {
            return Err(SmError::precondition(format!(
                "environment binding {:?} is already registered",
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
            store::insert_frame(&mut tx, &binding.env_id, frame).await?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    /// SM `add_binding_frame`: add one data frame to an existing binding.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the `env_id` is not registered
    ///   (`__Pre_valid_binding__`), or the frame's `frame_id` collides with an
    ///   existing frame.
    /// - `exception` — a serialization or database fault while writing.
    pub async fn add_binding_frame(&self, env_id: String, frame: DataFrame) -> Result<(), SmError> {
        if !self.sp_has_binding(&env_id).await? {
            return Err(SmError::precondition(format!(
                "environment binding {env_id:?} is not registered"
            )));
        }
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        store::insert_frame(&mut tx, &env_id, &frame).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    /// SM `has_binding`: whether an environment binding is registered.
    ///
    /// # Errors
    /// `exception` — a database fault while reading.
    pub async fn has_binding(&self, env_id: String) -> Result<bool, SmError> {
        self.sp_has_binding(&env_id).await
    }

    /// SM `reset`: "Set back to virgin state … remove all subjects, variables
    /// and bindings" (master10 §Persistence). Subjects cascade to variables,
    /// data sets and samples; bindings cascade to data frames.
    ///
    /// # Errors
    /// `exception` — a database fault mid-transaction (rolled back).
    pub async fn reset(&self) -> Result<(), SmError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
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

    /// The `notify_variable_sample` extension: push a sample for a manual /
    /// ask-user variable (the channel that realizes `SUBJECT_VARIABLE.is_manual`
    /// and `ask_user` — the spec's own TODO notes `ask_user` "can only work if
    /// access method defined"). A pushed sample is not frame-driven (no
    /// producing `DATA_FRAME_SAMPLE`).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject is not registered, no
    ///   variable with that (resolved) name exists, or the variable is neither
    ///   `is_manual` nor `ask_user` (design-filled extension precondition).
    /// - `exception` — a database/serialization fault while recording.
    pub async fn notify_variable_sample(
        &self,
        subject_id: String,
        var_name: String,
        sample: VariableSample,
    ) -> Result<(), SmError> {
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
        self.sp_record_sample(&subject_id, &canonical, None, &sample, None)
            .await
    }

    /// The `get_subject_variable` extension: a variable's full definition with
    /// its runtime sample state (`SUBJECT_VARIABLE.history` + `last_frame`)
    /// materialised from the sample store (`subject_variable.adoc`). `var_name`
    /// resolves like [`Self::get_variable`]'s.
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — the subject is not registered, or
    ///   no variable with that (resolved) name exists (design-filled extension
    ///   precondition).
    /// - `exception` — a database/serialization fault while reading.
    pub async fn get_subject_variable(
        &self,
        subject_id: String,
        var_name: String,
    ) -> Result<SubjectVariable, SmError> {
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
