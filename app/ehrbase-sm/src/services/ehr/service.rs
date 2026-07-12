//! `I_EHR_SERVICE` (`i_ehr_service.adoc`) and `EHR_SUMMARY`
//! (`ehr_summary.adoc`).

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::common::SmError;
use crate::services::ehr_index::SubjectRef;

/// `EHR_SUMMARY` — "Summary form of `EHR` + `EHR_STATUS` objects convenient
/// for use in service interface" (`ehr_summary.adoc`). All six attributes are
/// mandatory in the SM.
#[derive(Debug, Clone)]
pub struct EhrSummary {
    /// `ehr_id: UUID` — "EHR identifier of this EHR."
    pub ehr_id: String,
    /// `system_id: String` — "Copy of `EHR.system_id`."
    pub system_id: String,
    /// `ehr_status: EHR_STATUS` — "Copy of `EHR.ehr_status`" (canonical JSON).
    pub ehr_status: Value,
    /// `time_created: Iso8601_date_time` — "Copy of `EHR.time_created`."
    pub time_created: String,
    /// `contribution_count: Integer` — "Number of Contributions in this EHR."
    pub contribution_count: i64,
    /// `composition_count: Integer` — "Number of (versioned) Compositions in
    /// this EHR."
    pub composition_count: i64,
}

/// `I_EHR_SERVICE : I_STATUS` — "Primary interface to `EHR_SERVICE` persistent
/// repository" (`i_ehr_service.adoc`). The per-EHR accessor `I_EHR` is
/// realized as the generic handle [`IEhr`](super::IEhr), built from
/// [`EhrService::i_ehr`]; the flat trait calls remain the implementation
/// surface it delegates to.
///
/// Every call is transcribed with its SM name, parameter names, and types
/// (`UUID`→[`Uuid`], `PARTY_REF`→[`SubjectRef`], `EHR_STATUS`→canonical
/// [`Value`], `EHR_SUMMARY`→[`EhrSummary`]); pre/post-conditions and
/// exceptions are in the doc-comment per call.
#[async_trait]
pub trait EhrService: Send + Sync {
    /// `has_ehr (ehr_id: UUID): Boolean` — "True if an EHR with `ehr_id`
    /// exists."
    async fn has_ehr(&self, ehr_id: Uuid) -> Result<bool, SmError>;

    /// `has_ehr_for_subject (a_subject_id: PARTY_REF): Boolean` — "True if an
    /// EHR exists for the given subject id." Error `ehr_does_not_exist`.
    async fn has_ehr_for_subject(&self, a_subject_id: SubjectRef) -> Result<bool, SmError>;

    /// `create_ehr (an_ehr_status: EHR_STATUS [0..1]): UUID` — create an EHR
    /// with a system-generated id.
    ///
    /// - Pre `Subject_empty`: `an_ehr_status.subject = Void`.
    /// - Post `Ehr_created`: `has_ehr(Result)`.
    /// - Default `EHR_STATUS` if absent: `is_modifiable`/`is_queryable` True,
    ///   `subject = PARTY_SELF`.
    async fn create_ehr(&self, an_ehr_status: Option<Value>) -> Result<Uuid, SmError>;

    /// `create_ehr_with_id (an_ehr_id: UUID, an_ehr_status [0..1]): UUID` —
    /// create with a client-supplied id (echoed as a safety check).
    ///
    /// - Pre `Subject_empty` + `Id_available: not has_ehr(an_ehr_id)`.
    /// - Post `has_ehr(Result)`. Error `ehr_create_fail_duplicate_id`.
    async fn create_ehr_with_id(
        &self,
        an_ehr_id: Uuid,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError>;

    /// `create_ehr_for_subject (a_subject_id: PARTY_REF, an_ehr_status [0..1]):
    /// UUID` — create an EHR whose `EHR_STATUS.subject` is set to the subject.
    /// Error `ehr_for_subject_already_exists`.
    async fn create_ehr_for_subject(
        &self,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError>;

    /// `create_ehr_for_subject_with_id (an_ehr_id: UUID, a_subject_id:
    /// PARTY_REF, an_ehr_status [0..1]): UUID` — both ids client-supplied.
    ///
    /// - Pre `Id_available: not has_ehr(an_ehr_id)`. Error
    ///   `ehr_create_fail_duplicate_id`.
    async fn create_ehr_for_subject_with_id(
        &self,
        an_ehr_id: Uuid,
        a_subject_id: SubjectRef,
        an_ehr_status: Option<Value>,
    ) -> Result<Uuid, SmError>;

    /// `get_ehr (an_ehr_id: UUID): EHR_SUMMARY` — the summarised EHR root +
    /// `EHR_STATUS` (`ehr_summary.adoc`).
    ///
    /// - Pre `has_ehr(an_ehr_id)`. Error `ehr_id_does_not_exist`.
    async fn get_ehr(&self, an_ehr_id: Uuid) -> Result<EhrSummary, SmError>;

    /// `get_ehrs_for_subject (a_subject_id: PARTY_REF): List<EHR_SUMMARY>` —
    /// all EHRs whose `ehr_status.subject` matches. Error
    /// `esubject_id_does_not_exist` (spec typo, recorded).
    async fn get_ehrs_for_subject(
        &self,
        a_subject_id: SubjectRef,
    ) -> Result<Vec<EhrSummary>, SmError>;

    /// `i_ehr (ehr_id: UUID): I_EHR` — access the per-EHR interfaces
    /// (`ehr_status`/`directory`/`compositions`/`contributions`). Realized as
    /// the generic handle [`IEhr`](super::IEhr).
    ///
    /// PORT NOTE: this default is the SM accessor sugar — it does not touch
    /// the backend (the `ehr_id_does_not_exist` check is deferred to the
    /// sub-handle calls, which each carry a `has_ehr` precondition).
    fn i_ehr(&self, ehr_id: Uuid) -> super::IEhr<'_, Self>
    where
        Self: Sized,
    {
        super::IEhr::new(self, ehr_id)
    }

    // ── ITS-REST wire assembly (adapter-support, not SM calls) ──────────────

    /// Assemble the RM `EHR` object the ITS-REST `GET /ehr/{ehr_id}` route
    /// returns (the wire body is the RM `EHR`, not `EHR_SUMMARY`).
    ///
    /// PORT NOTE: ITS-REST extension — no SM call emits the RM `EHR` object;
    /// the SM's `get_ehr` returns [`EhrSummary`]. Kept here so the adapter
    /// can build the wire body from a single seam.
    async fn ehr_object(&self, an_ehr_id: Uuid) -> Result<Value, SmError>;

    /// `POST /ehr` / `PUT /ehr/{ehr_id}` create returning the wire `EHR`
    /// object + its resource metadata, so the adapter can honour
    /// `Prefer: return=representation` without a re-read.
    ///
    /// PORT NOTE: adapter convenience over `create_ehr(_with_id)` — the SM
    /// `create_*` calls return only the `UUID`; the wire needs the created
    /// `EHR` body for representation responses.
    async fn ehr_created_object(&self, an_ehr_id: Uuid) -> Result<Value, SmError> {
        self.ehr_object(an_ehr_id).await
    }

    /// Realize `has_ehr_for_subject`/`get_ehrs_for_subject` for the wire
    /// `GET /ehr?subject_id&subject_namespace` route as the RM `EHR` object
    /// of the (single) matching EHR.
    ///
    /// PORT NOTE: ITS-REST extension — the wire route returns one RM `EHR`,
    /// whereas the SM `get_ehrs_for_subject` returns a `List<EHR_SUMMARY>`.
    async fn ehr_object_for_subject(
        &self,
        subject_id: &str,
        subject_namespace: &str,
    ) -> Result<Value, SmError>;
}
