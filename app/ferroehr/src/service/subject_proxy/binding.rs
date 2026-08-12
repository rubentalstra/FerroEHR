// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Bindings: `ENV_BINDING`, `DATA_FRAME`, and the `SYSTEM_CALL` retrieval
//! methods (`env_binding.adoc`, `data_frame.adoc`; master10 §Bindings +
//! §Specifying a Binding).

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694 family 6): FHIR resources are an external standard \
              with no RM type (typed-FHIR evaluation tracked separately)"
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The attribute body shared by the two `SYSTEM_CALL` descendants master10
/// §Specifying a Binding exercises (`system_id`, `call_name`, `parameters`,
/// `query_text`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemCallBody {
    /// Target system identifier (e.g. `ehr1.nhs.org.uk`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_id: Option<String>,
    /// Named call on the target system (e.g. `aql_query`, `fhir_get`,
    /// `REST_get`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_name: Option<String>,
    /// Call parameters, keyed by parameter name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,
    /// Query/URL text (e.g. the AQL text, a FHIR search URL template). May
    /// reference `$subject_id`, bound by the executor at retrieval time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_text: Option<String>,
}

/// `SYSTEM_CALL` — the type of `DATA_FRAME.primary_method`/`fallback_method`.
///
/// Defined in `data_frame.adoc` (referencing the openEHR PROC Task Planning
/// `SYSTEM_CALL` class), with the two descendants shown in master10
/// §Specifying a Binding: `API_CALL` and `QUERY_CALL`.
///
/// NOTE: the PROC/Task Planning spec is **not vendored** in this
/// workspace, so `SYSTEM_CALL` is modelled as exactly the descendant set and
/// attribute set master10's own binding examples exercise — re-checked on any
/// future PROC vendoring. The spec's YAML examples use informal `!!API_CALL`
/// local tags; ingestion accepts the `_type` discriminator (consistent with
/// canonical JSON) in both JSON and YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum SystemCall {
    /// `API_CALL` — a named API invocation on a target system (e.g.
    /// `fhir_get`, `REST_get`).
    #[serde(rename = "API_CALL")]
    Api(SystemCallBody),
    /// `QUERY_CALL` — a query execution on a target system (e.g. `aql_query`).
    #[serde(rename = "QUERY_CALL")]
    Query(SystemCallBody),
}

impl SystemCall {
    /// The shared attribute body.
    #[must_use]
    pub fn body(&self) -> &SystemCallBody {
        match self {
            Self::Api(body) | Self::Query(body) => body,
        }
    }

    /// The named call, lower-cased for dispatch (`aql_query`, `fhir_get`, …).
    #[must_use]
    pub fn call_name(&self) -> Option<String> {
        self.body().call_name.as_deref().map(str::to_lowercase)
    }
}

/// `DATA_FRAME` — "Data retrieval frame, consisting of primary and fallback
/// retrieval methods (i.e. calls, or parameters for standard calls), and most
/// recent result" (`data_frame.adoc`).
///
/// NOTE: the runtime "most recent result" is not persisted with the
/// frame configuration (master10 §Persistence) — retrieval results live in
/// the sample store, surfaced as `SUBJECT_VARIABLE.last_frame`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataFrame {
    /// `id [1]` — the frame identifier (e.g. `openEHR::vital_signs`),
    /// referenced by `SUBJECT_VARIABLE.frame_id`.
    pub id: String,
    /// `model_type [1]` — name of the underlying model/type system, e.g.
    /// `"openehr"`, `"hl7v2"`, `"hl7-fhir"` ("Currently not standardised").
    pub model_type: String,
    /// `primary_method: SYSTEM_CALL [0..1]` — the method used to perform the
    /// retrieval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_method: Option<SystemCall>,
    /// `fallback_method: SYSTEM_CALL [0..1]` — "Alternative method to use if
    /// primary retrieve method fails."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_method: Option<SystemCall>,
}

/// `ENV_BINDING` — an execution environment bound to subject variables.
///
/// "Binding for an execution environment to a set of subject variables … a set
/// of retrieval methods (e.g. API invocations, queries) each defined by a
/// _data frame_ …, for a particular execution environment, and _independent of
/// any particular subject_" (`env_binding.adoc`; master10 §Bindings).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvBinding {
    /// `env_id [1]` — identifier of the environment this binding is designed
    /// for.
    pub env_id: String,
    /// `description [0..1]` — informal description of the environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `data_frames: List<DATA_FRAME> [0..1]` — the frames of this binding.
    #[serde(default)]
    pub data_frames: Vec<DataFrame>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The master10 §Specifying a Binding example (adapted to the `_type`
    /// discriminator — see the `SystemCall` NOTE), ingested from YAML,
    /// round-tripped through JSON.
    #[test]
    fn master10_binding_yaml_round_trips() {
        let yaml = r#"
env_id: prod
description: deployment environment
data_frames:
  - id: "OracleMPI::basic_demographics"
    model_type: OracleMPI
    primary_method:
      _type: API_CALL
      system_id: pas3.nhs.org.uk
      call_name: REST_get
      parameters:
        xxxx: abc
        yyyy: def
  - id: "openEHR::vital_signs"
    model_type: openEHR-EHR
    primary_method:
      _type: QUERY_CALL
      system_id: ehr1.nhs.org.uk
      call_name: aql_query
      query_text: SELECT c FROM EHR e CONTAINS COMPOSITION c
  - id: "fhir::demographics"
    model_type: HL7-FHIR_DSTU4_UK
    primary_method:
      _type: API_CALL
      system_id: ehr1.nhs.org.uk
      call_name: fhir_get
      query_text: Patient/$subject_id
    fallback_method:
      _type: QUERY_CALL
      call_name: aql_query
      query_text: SELECT e/ehr_id/value FROM EHR e
"#;
        let binding: EnvBinding = serde_norway::from_str(yaml).expect("parse YAML");
        assert_eq!(binding.env_id, "prod");
        assert_eq!(binding.data_frames.len(), 3);

        let mpi = &binding.data_frames[0];
        let Some(SystemCall::Api(body)) = &mpi.primary_method else {
            panic!("MPI frame is an API_CALL");
        };
        assert_eq!(body.call_name.as_deref(), Some("REST_get"));
        assert_eq!(body.parameters.len(), 2);

        let vitals = &binding.data_frames[1];
        let Some(SystemCall::Query(body)) = &vitals.primary_method else {
            panic!("vital_signs frame is a QUERY_CALL");
        };
        assert_eq!(body.call_name.as_deref(), Some("aql_query"));
        assert!(
            body.query_text
                .as_deref()
                .is_some_and(|q| q.contains("SELECT"))
        );

        // DATA_FRAME.fallback_method carried (`data_frame.adoc`).
        assert!(binding.data_frames[2].fallback_method.is_some());

        // JSON round-trip: the same `_type`-tagged shape.
        let json = serde_json::to_value(&binding).expect("to json");
        assert_eq!(
            json["data_frames"][0]["primary_method"]["_type"],
            "API_CALL"
        );
        let back: EnvBinding = serde_json::from_value(json).expect("from json");
        assert_eq!(binding, back);
    }
}
