//! The service interfaces: `I_SUBJECT_PROXY_SERVICE`
//! (`i_subject_proxy_service.adoc`) and the internal `I_DATA_BINDING`
//! (`i_data_binding.adoc`).

use async_trait::async_trait;

use crate::common::SmError;

use super::binding::{DataFrame, EnvBinding};
use super::data_set::{DataSetResult, SubjectDataSet};
use super::sample::{DataFrameSample, VariableSample};
use super::value::VariableValue;
use super::variable::SubjectVariable;

/// `I_SUBJECT_PROXY_SERVICE` — "Service that maintains subject 'proxies'
/// consisting of variables, and also enables applications to associate
/// data-sets with subject variables" (`i_subject_proxy_service.adoc`). One
/// Rust method per SM call, verbatim call names/parameters/pre-conditions,
/// plus two flagged extension calls (`notify_variable_sample`,
/// `get_subject_variable`) — see their docs.
///
/// No default method bodies (compile-time completeness by design): a backend
/// that does not implement a call is a build error, not a silent runtime stub.
#[async_trait]
pub trait SubjectProxyService: Send + Sync {
    /// `register_subject (subject_id: String, subject_category: String [0..1])`
    /// with `__Pre_subject_valid__: not has_subject(subject_id)`. "Register a
    /// new subject. The subject category may … be specified, otherwise it will
    /// be the default category." (`SUBJECT_PROXY.subject_category` is
    /// "currently not controlled" — free text, default `"individual"`.)
    async fn register_subject(
        &self,
        subject_id: String,
        subject_category: Option<String>,
    ) -> Result<(), SmError>;

    /// `add_subject_variable (subject_id: String, var: SUBJECT_VARIABLE)` with
    /// `__Pre_subject_valid__: has_subject(subject_id)`. "Add a new subject
    /// variable definition to the proxy for `subject_id`."
    async fn add_subject_variable(
        &self,
        subject_id: String,
        var: SubjectVariable,
    ) -> Result<(), SmError>;

    /// `register_application_data_set (definition: SUBJECT_DATA_SET)` with
    /// `__Pre_subject_valid__: has_subject(subject_id)` — the `subject_id` is
    /// inside `definition` (spec imprecision). "Register a data-set … This may
    /// have the effect of creating new subject variables, reducing the
    /// currency of existing subject variables, if the currency is lower in the
    /// corresponding data set variable, or making no change for variables that
    /// already exist."
    async fn register_application_data_set(
        &self,
        definition: SubjectDataSet,
    ) -> Result<(), SmError>;

    /// `remove_application_data_set (subject_id: String, application_id: String)`
    /// with `__Pre_subject_valid__: has_subject` + `__Pre_application_valid__:
    /// has_application`. "Remove this data-set from the service."
    async fn remove_application_data_set(
        &self,
        subject_id: String,
        application_id: String,
    ) -> Result<(), SmError>;

    /// `remove_subject (subject_id: String)` with `__Pre_subject_valid__:
    /// has_subject`. "Remove proxy and any data-sets for an existing subject."
    async fn remove_subject(&self, subject_id: String) -> Result<(), SmError>;

    /// `remove_application (application_id: String)` with
    /// `__Pre_application_valid__: has_application`. "Remove all data-sets for
    /// `application_id`, across all subjects."
    async fn remove_application(&self, application_id: String) -> Result<(), SmError>;

    /// `get_variable (subject_id: String, var_name: String): VARIABLE_VALUE`
    /// with `__Pre_subject_valid__: has_subject`. "Get a single variable value
    /// from a data-set." `var_name` resolves first as a data-set-local alias,
    /// then as a canonical name (master10 §Subject Variable Naming: a data set
    /// "may … use data set-local aliases, for example `dob`").
    async fn get_variable(
        &self,
        subject_id: String,
        var_name: String,
    ) -> Result<VariableValue, SmError>;

    /// `get_data_set (subject_id: String, data_set_id: String): DATA_SET_RESULT`
    /// with `__Pre_subject_valid__: has_subject`. "Get a full data set result."
    async fn get_data_set(
        &self,
        subject_id: String,
        data_set_id: String,
    ) -> Result<DataSetResult, SmError>;

    /// `has_subject (subject_id: String): Boolean`. "Return True if subject
    /// with id `subject_id` has been registered in the service."
    async fn has_subject(&self, subject_id: String) -> Result<bool, SmError>;

    /// `has_application (application_id: String): Boolean`. "Return True if
    /// application with id `application_id` has been registered in the
    /// service."
    async fn has_application(&self, application_id: String) -> Result<bool, SmError>;

    /// `get_variable_defs (subject_id: String): List<String>`. "Return a list
    /// of variable definitions each of the form 'name: Type', where 'name' is
    /// the canonical name, i.e. `SUBJECT_VARIABLE.name`."
    async fn get_variable_defs(&self, subject_id: String) -> Result<Vec<String>, SmError>;

    /// `register_binding (binding: ENV_BINDING)` with `__Pre_new_env__: not
    /// has_binding(binding.env_id)`. "Register a binding for an environment,
    /// consisting of a set of variable bindings, each including method(s) for
    /// data retrieval to populate the related variable."
    async fn register_binding(&self, binding: EnvBinding) -> Result<(), SmError>;

    /// `add_binding_frame (env_id: String, frame[1])` with
    /// `__Pre_valid_binding__: has_binding(env_id)`. "Add a retrieve frame
    /// definition to the binding container for an environment."
    ///
    /// PORT NOTE: the SM leaves the `frame` parameter **untyped**
    /// (`add_binding_frame (env_id, frame[1])`); it is typed as [`DataFrame`],
    /// the element type of `ENV_BINDING.data_frames`.
    async fn add_binding_frame(&self, env_id: String, frame: DataFrame) -> Result<(), SmError>;

    /// `has_binding (env_id: String): Boolean`. "Return True if environment
    /// binding with id `env_id` has been registered in the service."
    async fn has_binding(&self, env_id: String) -> Result<bool, SmError>;

    /// `reset` — "Set back to virgin state, i.e. remove all subjects,
    /// variables and bindings" (master10 §Persistence).
    async fn reset(&self) -> Result<(), SmError>;

    /// Record a manually-observed sample for a variable of `subject_id`.
    ///
    /// **Extension — no openEHR spec defines this call; our own design.** It
    /// is the input channel `SUBJECT_VARIABLE.is_manual` ("obtained by manual
    /// notification, typically from a worker observing the subject") and
    /// `ask_user` require: the SM defines the flags but no push operation
    /// (`subject_variable.adoc`). Precondition (design-filled): `has_subject`
    /// and the variable exists and `is_manual` or `ask_user` is set.
    async fn notify_variable_sample(
        &self,
        subject_id: String,
        var_name: String,
        sample: VariableSample,
    ) -> Result<(), SmError>;

    /// Read back a full `SUBJECT_VARIABLE` — definition plus the runtime
    /// sample state (`history`, `last_frame`) materialised from the sample
    /// store.
    ///
    /// **Extension — no openEHR spec defines this call; our own design.** The
    /// SM models `history`/`last_frame` on `SUBJECT_VARIABLE`
    /// (`subject_variable.adoc`) but defines no read returning the class; this
    /// is that read. Precondition (design-filled): `has_subject` and the
    /// variable exists.
    async fn get_subject_variable(
        &self,
        subject_id: String,
        var_name: String,
    ) -> Result<SubjectVariable, SmError>;
}

/// `I_DATA_BINDING` — "Internal interface via which Variable bindings are
/// invoked to obtain data" (`i_data_binding.adoc`).
///
/// PORT NOTE: the SM lists a `bindings: List<ENV_BINDING>` attribute here
/// ("All bindings registered in this service, one per environment"); it is
/// realized by the binding store and surfaced through
/// [`SubjectProxyService::has_binding`] / `register_binding`, not as a getter.
#[async_trait]
pub trait DataBinding: Send + Sync {
    /// `get_frame (subject_id: String, frame_id: String): DATA_FRAME_SAMPLE`.
    /// "Execute a retrieve on a data frame, for a specific subject."
    ///
    /// `subject_id`: "the identifier might not be the primary identifier for a
    /// person …, but instead an identifier of an information resource against
    /// which the query can be made, e.g. an EHR identifier" — with the SM's
    /// own TODO that "this service might need to resolve it through another
    /// service": realized via the EHR Index (`I_EHR_INDEX`) subject-ref
    /// lookup, then literal-EHR-id fallback.
    ///
    /// Executes `primary_method`; a failed/unavailable primary triggers
    /// `fallback_method` when present (`data_frame.adoc`). Every attempt
    /// produces a `SAMPLE` (`sample.adoc`) which is recorded in the sample
    /// store.
    async fn get_frame(
        &self,
        subject_id: String,
        frame_id: String,
    ) -> Result<DataFrameSample, SmError>;
}
