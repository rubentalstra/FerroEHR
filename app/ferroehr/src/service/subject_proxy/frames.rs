// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! `I_DATA_BINDING` implementation (`i_data_binding.adoc`): execute a retrieve
//! frame for a subject, with the primary→fallback pipeline of
//! `data_frame.adoc` ("Alternative method to use if primary retrieve method
//! fails").
//!
//! Dispatch is `model_type` × `call_name` (master10 §Specifying a Binding), not
//! an invented enum tag: a `QUERY_CALL`/`aql_query` (or any `openehr…`
//! `model_type`) routes to the openEHR AQL executor; an `API_CALL`/`fhir_get`
//! routes to the config-gated FHIR executor; `HL7v2` frames are typed
//! rejections.
//!
//! The pipeline outcome model (`data_frame.adoc`): a dispatch-impossible frame
//! (no method, unknown `model_type` or `call_name`, an unwired executor) is
//! `Err(SmError::not_implemented)`, while a frame that executed and failed
//! (backend down, no `query_text`, subject unresolved, AQL error) is
//! `Ok(SAMPLE{is_unavailable})`, a real sample per `sample.adoc` ("Every
//! retrieval attempt will generate a new Sample … regardless of whether data was
//! actually available or not").
//!
//! A failed or unavailable primary tries `fallback_method` when present.
//! Frame-level `get_frame` persists no samples: that belongs to the variable
//! read paths, which own the variable context the `sp_sample` FK requires.

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use serde_json::Value;

use crate::service::FerroEhrService;
use crate::service::query::request::AqlQueryRequest;
use crate::service::status::{CallStatusType, SmError};
use crate::service::subject_proxy::binding::{DataFrame, SystemCall};
use crate::service::subject_proxy::sample::{DataFrameSample, FramePayload, Sample};

use super::store::FrameRow;

/// `not_implemented` — a dispatch-impossible outcome (no executor for the
/// frame's `model_type`/`call_name`). Distinct from an executed-but-failed
/// retrieve, which is a `SAMPLE{is_unavailable}` (`Ok`).
fn not_implemented(message: impl Into<String>) -> SmError {
    SmError::new(CallStatusType::NotImplemented, message)
}

impl FerroEhrService {
    /// SM `I_DATA_BINDING.get_frame`: execute the registered retrieve frame
    /// `frame_id` for `subject_id`, running the primary→fallback pipeline and
    /// returning the produced `DATA_FRAME_SAMPLE` (available, or unavailable
    /// with the reason). Nothing is persisted here (see the module docs).
    ///
    /// # Errors
    /// - `precondition_violation` (`400`) — no data frame with `frame_id` is
    ///   registered.
    /// - `not_implemented` — the frame is dispatch-impossible on every leg
    ///   that ran: no retrieval method, an unknown `model_type`/`call_name`
    ///   (e.g. `HL7v2`), or a FHIR call with no configured executor / no
    ///   `system_id` / an unconfigured system.
    /// - `exception` — a database fault while loading the frame or resolving
    ///   the subject.
    pub async fn get_frame(
        &self,
        subject_id: String,
        frame_id: String,
    ) -> Result<DataFrameSample, SmError> {
        let Some(FrameRow { frame }) = self.sp_frame(&frame_id).await? else {
            return Err(SmError::precondition(format!(
                "no data frame {frame_id:?} is registered (register the binding first)"
            )));
        };

        // Primary retrieve.
        let primary = self
            .sp_dispatch_method(&subject_id, &frame, frame.primary_method.as_ref())
            .await;
        if let Ok(sample) = &primary
            && !sample.is_unavailable
        {
            return Ok(sample.clone());
        }

        // A failed (dispatch-impossible) or unavailable primary triggers the
        // fallback when present (`data_frame.adoc`).
        if let Some(fallback) = frame.fallback_method.as_ref() {
            match self
                .sp_dispatch_method(&subject_id, &frame, Some(fallback))
                .await
            {
                // Fallback executed (available or unavailable) — its sample wins.
                Ok(sample) => return Ok(sample),
                // Fallback dispatch-impossible: keep an executed-but-unavailable
                // primary; if the primary was also dispatch-impossible, the
                // frame is unretrievable (dispatch-impossible on both).
                Err(fallback_err) => return primary.or(Err(fallback_err)),
            }
        }

        // No fallback: return the primary outcome (Ok-unavailable or Err).
        primary
    }

    /// Dispatch one `SYSTEM_CALL` to its executor. `Ok` = the call executed and
    /// produced a `SAMPLE` (available or unavailable); `Err(not_implemented)` =
    /// no executor could be dispatched at all.
    async fn sp_dispatch_method(
        &self,
        subject_id: &str,
        frame: &DataFrame,
        method: Option<&SystemCall>,
    ) -> Result<DataFrameSample, SmError> {
        let Some(call) = method else {
            return Err(not_implemented(format!(
                "data frame {:?} has no retrieval method",
                frame.id
            )));
        };
        let call_name = call.call_name();
        let model = frame.model_type.to_lowercase();

        // openEHR: a QUERY_CALL/aql_query, or any `openehr…` model_type.
        let is_openehr = (matches!(call, SystemCall::Query(_))
            && call_name.as_deref() == Some("aql_query"))
            || model.starts_with("openehr");
        if is_openehr {
            return Ok(self.sp_dispatch_openehr(subject_id, call).await);
        }

        // FHIR: an API_CALL/fhir_get. A config-gated `reqwest` GET of the frame's
        // `query_text` (with `$subject_id` substitution) against a configured
        // FHIR system, yielding an HL7_FHIR_SAMPLE (`hl7_fhir_sample.adoc`).
        if matches!(call, SystemCall::Api(_)) && call_name.as_deref() == Some("fhir_get") {
            return self.sp_dispatch_fhir(subject_id, frame, call).await;
        }

        // HL7v2 and everything else: no transport in scope.
        Err(not_implemented(format!(
            "no executor for data frame {:?} (model_type {:?}, call {call_name:?})",
            frame.id, frame.model_type
        )))
    }

    /// The openEHR executor: run the frame's AQL text through the internal AQL
    /// engine, scoped to the subject's resolved EHR, binding `$subject_id`.
    /// Executed-but-failed outcomes (no query text, unresolved subject, AQL
    /// error) are `SAMPLE{is_unavailable}`, never a dispatch error.
    async fn sp_dispatch_openehr(&self, subject_id: &str, call: &SystemCall) -> DataFrameSample {
        let Some(query_text) = call.body().query_text.as_deref() else {
            return Sample::unavailable("openEHR frame has no query_text to execute");
        };

        // Resolve the subject id to an EHR (literal EHR uuid, then EHR Index):
        // an unresolved openEHR subject yields an unavailable sample with reason
        // (`i_data_binding.adoc`).
        let ehr_id = match self.sp_resolve_subject_ehr(subject_id).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Sample::unavailable(format!(
                    "could not resolve subject {subject_id:?} to an EHR (no literal EHR id \
                     and no EHR Index entry)"
                ));
            }
            Err(e) => return Sample::unavailable(e.message),
        };

        let mut request = AqlQueryRequest {
            ehr_ids: vec![ehr_id.to_string()],
            ..AqlQueryRequest::default()
        };
        request.parameters.insert(
            "subject_id".to_owned(),
            Value::String(subject_id.to_owned()),
        );

        match self.execute_aql(query_text, None, &request).await {
            Ok(outcome) => Sample::available(FramePayload::Openehr {
                result_set: outcome.result_set,
            }),
            // Executed but failed: a real (unavailable) sample so the pipeline
            // can fall back (`data_frame.adoc`).
            Err(e) => Sample::unavailable(e.message),
        }
    }

    /// The FHIR executor (`i_data_binding.adoc`; `hl7_fhir_sample.adoc`): GET the
    /// frame's `query_text` (a FHIR search/read URL template) against the
    /// configured `system_id`, `$subject_id` substituted with the URL-encoded
    /// subject id (the remote system owns subject resolution — no EHR lookup).
    ///
    /// Fail-closed dispatch (`Err(not_implemented)`): no FHIR executor
    /// configured, no `system_id`, or a `system_id` matching no configured
    /// system — never an arbitrary outbound request. An executed-but-failed
    /// retrieve (no `query_text`, non-2xx, timeout, malformed body) is an
    /// `Ok(SAMPLE{is_unavailable})` so the primary→fallback pipeline runs
    /// (`data_frame.adoc`).
    async fn sp_dispatch_fhir(
        &self,
        subject_id: &str,
        frame: &DataFrame,
        call: &SystemCall,
    ) -> Result<DataFrameSample, SmError> {
        let Some(fhir) = self.subject_proxy_fhir.as_ref() else {
            return Err(not_implemented(format!(
                "no FHIR executor is configured (data frame {:?}); configure \
                 FERROEHR_SUBJECT_PROXY__SYSTEMS to enable FHIR retrieval",
                frame.id
            )));
        };
        let body = call.body();
        let Some(system_id) = body.system_id.as_deref() else {
            return Err(not_implemented(format!(
                "FHIR data frame {:?} has no system_id to route to",
                frame.id
            )));
        };
        // Fail-closed: an unconfigured system is a typed rejection, never a
        // request to an arbitrary host.
        if !fhir.has_system(system_id) {
            return Err(not_implemented(format!(
                "FHIR data frame {:?} targets system {system_id:?}, which is not a \
                 configured subject-proxy FHIR system",
                frame.id
            )));
        }
        let Some(query_text) = body.query_text.as_deref() else {
            return Ok(Sample::unavailable(
                "FHIR frame has no query_text to retrieve",
            ));
        };
        // `$subject_id` → the URL-encoded subject id (never hand-roll a percent
        // codec — `urlencoding::encode`).
        let encoded = urlencoding::encode(subject_id);
        let query_path = query_text.replace("$subject_id", &encoded);

        match fhir.get(system_id, &query_path).await {
            Ok(fetch) => {
                let sample = Sample::available(FramePayload::Hl7Fhir {
                    resource: fetch.resource,
                });
                Ok(match fetch.effective_time {
                    Some(t) => sample.with_effective_time(t),
                    None => sample,
                })
            }
            // Executed but failed → unavailable sample (feeds primary→fallback).
            Err(reason) => Ok(Sample::unavailable(reason)),
        }
    }
}
