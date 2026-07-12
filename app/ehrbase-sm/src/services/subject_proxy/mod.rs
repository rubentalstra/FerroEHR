//! The SM Subject Proxy Service — `I_SUBJECT_PROXY_SERVICE` + the internal
//! `I_DATA_BINDING` interface and their information structures, transcribed
//! literally from the vendored spec.
//!
//! Spec sources
//! (`docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
//! and the `UML/classes/*.adoc` it includes):
//! `i_subject_proxy_service.adoc`, `i_data_binding.adoc`, `subject_proxy.adoc`,
//! `subject_variable.adoc`, `subject_data_set.adoc`, `data_set_result.adoc`,
//! `sample.adoc`, `data_frame_sample.adoc`, `openehr_sample.adoc`,
//! `hl7v2_sample.adoc`, `hl7_fhir_sample.adoc`, `variable_sample.adoc`,
//! `variable_value.adoc` (+ `_single`/`_list`/`_time_series`), `env_binding.adoc`,
//! `data_frame.adoc`. Design + gap register:
//! `docs/design/sm-platform/10-subject-proxy.md` (W-3c).
//!
//! The SPS "allows symbolic variables characterising the real world state of a
//! _subject_ … to be retrieved and tracked over time … The SPS avoids the
//! calling application having to know about the particular standard,
//! representational model, query language or API of the data source"
//! (master10 §Overview).
//!
//! Module map: [`value`] (`VARIABLE_VALUE` hierarchy), [`sample`]
//! (`SAMPLE<T>` + the `DATA_FRAME_SAMPLE` payload family), [`variable`]
//! (`SUBJECT_VARIABLE`), [`data_set`] (`SUBJECT_DATA_SET` / `DATA_SET_RESULT`),
//! [`binding`] (`ENV_BINDING` / `DATA_FRAME` / `SYSTEM_CALL`), [`service`]
//! (the `I_SUBJECT_PROXY_SERVICE` + `I_DATA_BINDING` traits).
//!
//! PORT NOTE (design-filled errors). The SM declares only pre-conditions
//! (`has_subject` / `has_application` / `has_binding` / `not has_*`) and no
//! error codes. Every unmet pre-condition surfaces as
//! [`SmError`](crate::SmError)`(PreconditionViolation, …)` — the exact shape
//! the spec's `__Pre_…__` clauses describe.

pub mod binding;
pub mod data_set;
pub mod sample;
pub mod service;
pub mod value;
pub mod variable;

pub use binding::{DataFrame, EnvBinding, SystemCall, SystemCallBody};
pub use data_set::{DataSetResult, SubjectDataSet};
pub use sample::{DataFrameSample, FramePayload, Sample, VariableSample};
pub use service::{DataBinding, SubjectProxyService};
pub use value::VariableValue;
pub use variable::SubjectVariable;
