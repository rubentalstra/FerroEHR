//! The SM `I_EHR_EXTRACT_SERVICE` interface — the literal openEHR Platform
//! Service Model call set for importing and exporting EHR Extracts
//! (`docs/specs/openehr/SM/docs/UML/classes/i_ehr_extract_service.adoc`).
//! Design digest: `docs/design/sm-platform/10-message-integration.md` §2.
//!
//! The substance the SM interface points at is the RM **EHR Extract IM**
//! (STABLE, `docs/specs/openehr/RM/docs/ehr_extract/`): an `EXTRACT` carries
//! `EXTRACT_CHAPTER`s of `OPENEHR_CONTENT_ITEM`s, each wrapping an
//! `X_VERSIONED_OBJECT<T>` (a sharable, data-oriented `VERSIONED_OBJECT` —
//! master05). The creation algorithm is master09 §Creation Semantics.
//!
//! PORT NOTE (naming): the SM interface is spelled `I_EHR_EXTRACT_SERVICE`
//! here; the blueprint/design call it `I_EHR_EXTRACT`, and the CNF schedule
//! references a phantom singular `export_ehr()`/`export_ehr_extract()` pair
//! (`docs/design/sm-platform/10-message-integration.md` §2/§3). The four call
//! names below are the vendored SM `.adoc` spelling verbatim.
//!
//! PORT NOTE (no pre/post/errors): the three SM interface files declare **no**
//! preconditions, postconditions, or errors. The preconditions below are filled
//! by design — `has_ehr(an_ehr_id)` for the whole-EHR export, the target EHR
//! existing for imports — surfaced as [`SmError`] over `CALL_STATUS_TYPE`
//! (`ehr_id_does_not_exist`, `precondition_violation`).
//!
//! PORT NOTE (no wire): ITS-REST vendors zero extract/message endpoints and the
//! Messaging conformance profile is OPTIONS-only (not required for
//! CORE/STANDARD). Like [`AdminDumpLoad`](super::AdminDumpLoad), this interface
//! is therefore a native-API-only capability and is **not** part of the
//! [`Platform`](crate::Platform) union (which is "everything the ITS-REST
//! surface dispatches to"). The concrete service implements it directly.

use async_trait::async_trait;
use uuid::Uuid;

use openehr_rm::ehr_extract::common::extract::Extract;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;

use crate::error::SmError;

/// `I_EHR_EXTRACT_SERVICE` — import/export of EHR Extracts, one Rust method per
/// SM call (`i_ehr_extract_service.adoc`).
///
/// An exported `EXTRACT` is returned as canonical openEHR JSON
/// ([`serde_json::Value`]) — the same read-surface convention every other SM
/// catalog interface uses (reads return the canonical RM object as a `Value`);
/// the wire shape is exactly the vendored RM `ehr_extract` types
/// (`openehr_rm::ehr_extract`). Each SM call returns `List<EXTRACT>`, so the
/// exports return `Vec<Value>` (one `EXTRACT` per element).
///
/// No default method bodies (compile-time completeness by design): a backend that
/// does not implement a call is a build error, not a silent `501`.
#[async_trait]
pub trait EhrExtractService: Send + Sync {
    /// `export_ehrs (an_ehr_id: UUID): List<EXTRACT>` — "Export whole EHR for
    /// one or more subjects." Returns one `EXTRACT` carrying every
    /// versioned object of the EHR (`EHR_STATUS`, `EHR_ACCESS`, the directory
    /// `FOLDER`, and every `COMPOSITION`) at its latest version — the
    /// `EXTRACT_VERSION_SPEC` default (latest-only; `extract_version_spec.adoc`).
    ///
    /// Precondition (design-filled): `has_ehr(an_ehr_id)` — an unknown EHR is
    /// `ehr_id_does_not_exist`.
    ///
    /// PORT NOTE: the SM signature takes a single `UUID` yet the meaning says
    /// "one or more subjects" — a spec inconsistency. We honour the signature
    /// (one EHR id → a one-element `List<EXTRACT>`); multi-subject export is the
    /// caller iterating, or [`export_ehr_extracts`](Self::export_ehr_extracts)
    /// with a multi-entity `EXTRACT_MANIFEST`.
    async fn export_ehrs(&self, an_ehr_id: Uuid) -> Result<Vec<serde_json::Value>, SmError>;

    /// `export_ehr_extracts (extract_spec: EXTRACT_SPEC): List<EXTRACT>` —
    /// "Export an extract for one or more EHRs." One `EXTRACT` per entity in
    /// `extract_spec.manifest.entities`, honouring the `EXTRACT_VERSION_SPEC`
    /// (`include_all_versions`, `include_revision_history`, `include_data`;
    /// `extract_version_spec.adoc`) and each entity's `item_list`
    /// (version-container uids; `extract_entity_manifest.adoc`).
    ///
    /// PORT NOTE (deferred selectors): `EXTRACT_SPEC.criteria` (AQL primary-set
    /// selection, master09) and `EXTRACT_VERSION_SPEC.commit_time_interval` are
    /// not applied in this first stage — a request that sets either (with no
    /// `item_list` to fall back on) is a `precondition_violation` naming the
    /// unsupported selector, rather than a silent over-broad export. Both are
    /// slated for the query-integration wave (`$ehr`-bound AQL).
    async fn export_ehr_extracts(
        &self,
        extract_spec: ExtractSpec,
    ) -> Result<Vec<serde_json::Value>, SmError>;

    /// `import_ehr (an_ehr_id: UUID [0..1], an_extract: EXTRACT)` — "Import a
    /// whole EHR, optionally providing a fixed EHR identifier ... to match the
    /// identifier of EHR(s) for the same patient in other EHR services."
    ///
    /// Clones the EHR into an **empty target** (RM common master06 §Copying
    /// Case 1): the target id is `an_ehr_id` when given, else the source EHR id
    /// is reused (`ehr/master04` §"EHR Identifier Allocation"). Each received
    /// `ORIGINAL_VERSION` is committed wrapped in an `IMPORTED_VERSION` — a local
    /// import CONTRIBUTION records the local committal (`249|creation|`), the
    /// wrapped original's identity / `commit_audit` / data are preserved verbatim.
    /// A target id that already exists is `ehr_create_fail_duplicate_id`.
    async fn import_ehr(&self, an_ehr_id: Option<Uuid>, an_extract: Extract)
    -> Result<(), SmError>;

    /// `import_ehr_extract (an_ehr_id: UUID, an_extract: EXTRACT)` — "Import an
    /// EHR Extract into an existing EHR" (RM common master06 §Copying Cases 2/3:
    /// first receipt of an item clones its `VERSIONED_OBJECT` with the received
    /// `uid.object_id()`; a subsequent copy appends newer trunk versions). An
    /// unknown EHR is `ehr_id_does_not_exist`.
    async fn import_ehr_extract(&self, an_ehr_id: Uuid, an_extract: Extract)
    -> Result<(), SmError>;
}
