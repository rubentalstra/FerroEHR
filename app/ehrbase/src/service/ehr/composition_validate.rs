//! COMPOSITION commit validation + the VERSIONED_COMPOSITION cross-version
//! invariant hook (split out of [`composition`](super::composition) to keep
//! both files under the size bound).
//!
//! Spec: RM ehr `org.openehr.rm.ehr.versioned_composition.adoc` (the
//! cross-version invariants), RM common master06 §Incomplete Content (the
//! `553|incomplete|` relaxation), ITS-REST `responses/422_COMPOSITION.yaml`,
//! CNF `master07-func_tc_ehr_composition.adoc` (the persistent-cardinality
//! convention).

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::service::{EhrbaseService, ServiceError};
use crate::versioning::{Kind, lifecycle, read_current};

impl EhrbaseService {
    /// Enforce the CNF persistent-COMPOSITION uniqueness convention: an EHR may
    /// hold only one *live* persistent COMPOSITION per template.
    ///
    /// PORT NOTE: the openEHR RM does **not** define this cardinality — the CNF
    /// schedule records it as "under debate in the openEHR SEC … due to the lack
    /// of information in the openEHR specifications"
    /// (`CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`).
    /// We adopt the CNF criterion (`create_composition-same_opt_twice`). Only
    /// persistent COMPOSITIONs with a declared template are constrained.
    pub(super) async fn reject_duplicate_persistent(
        &self,
        ehr_id: Uuid,
        composition: &Value,
    ) -> Result<(), ServiceError> {
        if !is_persistent(composition) {
            return Ok(());
        }
        let Some(template_id) = composition_template_id(composition) else {
            return Ok(());
        };
        // PERF(port): scans the EHR's live COMPOSITIONs and reassembles each to
        // read its category + template (template_id is not promoted onto
        // vo_version). An EHR holds few persistent compositions.
        let vo_ids = crate::storage::version_repo::current_vo_ids(
            &self.pool,
            ehr_id,
            "COMPOSITION",
            Some(lifecycle::state::DELETED),
        )
        .await?;
        for vo_id in vo_ids {
            if let Some(read) = read_current(&self.pool, vo_id).await?
                && is_persistent(&read.canonical)
                && composition_template_id(&read.canonical) == Some(template_id)
            {
                return Err(ServiceError::Conflict(format!(
                    "EHR {ehr_id} already has a persistent COMPOSITION for template \
                     {template_id}; only one create is allowed (subsequent commits must \
                     be modifications)"
                )));
            }
        }
        Ok(())
    }

    /// Validate an incoming COMPOSITION against its operational template before it
    /// is persisted (the single choke point for the JSON and FLAT dispatch
    /// paths). RM class-invariant + terminology passes run unconditionally
    /// (template-independent); the archetype-conformance pass is gated on a
    /// declared+resolved template (F-07-02). A declared-but-failing template, or
    /// any RM/terminology violation, is a `422`
    /// (ITS-REST `responses/422_COMPOSITION.yaml`); syntactic parse failures are
    /// `400` and caught earlier at the REST negotiation edge.
    ///
    /// `incomplete` (a `553|incomplete|` CONTRIBUTION version, RM common master06
    /// §Incomplete Content) relaxes the archetype/template existence & cardinality
    /// **lower** limits to zero; RM invariants + terminology stay at full
    /// strictness ("data may be missing, but it may not be wrong").
    ///
    /// The template lookup goes through
    /// [`web_template_for`](Self::web_template_for) and the passes through
    /// `openehr_flat::validate_*`.
    pub(super) async fn validate_composition_for_commit(
        &self,
        composition: &Value,
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        let mut messages = openehr_flat::validate_rm_and_terminology(composition);
        let rm_terminology_failures = messages.len();
        if let Some(template_id) = composition
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
        {
            let wt = self.web_template_for(template_id).await?;
            messages.extend(if incomplete {
                openehr_flat::validate_archetype_conformance_incomplete(composition, &wt)
            } else {
                openehr_flat::validate_archetype_conformance(composition, &wt)
            });
        }
        let template_failures = messages.len() - rm_terminology_failures;
        if rm_terminology_failures > 0 {
            metrics::counter!(
                crate::telemetry::prometheus::VALIDATION_FAILURES,
                "pass" => "rm_terminology",
            )
            .increment(rm_terminology_failures as u64);
        }
        if template_failures > 0 {
            metrics::counter!(
                crate::telemetry::prometheus::VALIDATION_FAILURES,
                "pass" => "template",
            )
            .increment(template_failures as u64);
        }
        if messages.is_empty() {
            return Ok(());
        }
        let errors = messages
            .into_iter()
            .map(|m| openehr_its::rest::runtime::ValidationError {
                path: m.path,
                message: m.message,
            })
            .collect();
        Err(ServiceError::ValidationFailed(errors))
    }

    /// Validate a versioned object about to be committed (direct or via a
    /// CONTRIBUTION) — the [`crate::versioning::CommitEnv`] `validate_for_commit`
    /// hook. COMPOSITIONs get full RM + terminology + template validation; the
    /// EHR-owned kinds (`EHR_STATUS` / `EHR_ACCESS` / FOLDER) get structural RM
    /// validation. Shared by the direct create/update path and the CONTRIBUTION
    /// path so neither can bypass validation (F-07-01).
    ///
    /// The demographic arms (party roots + PARTY_RELATIONSHIP) dispatch to the
    /// demographic register (`service/demographic/`).
    pub(in crate::service) async fn validate_for_commit(
        &self,
        kind: Kind,
        data: &Value,
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        match kind {
            Kind::Composition => self.validate_composition_for_commit(data, incomplete).await,
            Kind::EhrStatus => super::validate_ehr_status(data),
            Kind::EhrAccess => super::validate_ehr_access(data),
            Kind::Folder => super::validate_folder(data),
            Kind::Agent | Kind::Group | Kind::Organisation | Kind::Person | Kind::Role => {
                crate::service::demographic::validate_party_kind_for_commit(kind, data)
            }
            Kind::PartyRelationship => {
                crate::service::demographic::validate_relationship_for_commit(data)
            }
        }
    }
}

/// `VERSIONED_COMPOSITION` cross-version invariants, enforced against the FIRST
/// stored version's root (RM ehr `versioned_composition.adoc`):
///
/// - `Archetype_node_id_valid`: every version's `data.archetype_node_id` equals
///   the first version's — a versioned composition cannot switch archetype;
/// - `Persistent_validity`: every version's `is_persistent` equals the first
///   version's — the persistence category (`431|persistent|`) is fixed for the
///   container's life.
///
/// A violating modification is a `422` naming the invariant. Lifted out of the
/// versioning write path — the EHR chapter now owns it (see the
/// `crate::versioning::change` `apply_change` TODO). The first-version root read
/// goes through `crate::storage::node_repo`.
///
/// Only the direct create/update path ([`composition`](super::composition))
/// invokes this today; the CONTRIBUTION path
/// (`crate::versioning::commit_version_set`) does not yet run it for a
/// COMPOSITION modify.
pub(super) async fn check_versioned_composition_invariants(
    tx: &mut PgConnection,
    vo_id: Uuid,
    canonical: &Value,
) -> Result<(), ServiceError> {
    const PERSISTENT: &str = "431";
    let Some(first) = crate::storage::node_repo::first_version_root(tx, vo_id).await? else {
        // No stored content version (e.g. every prior version deleted) — no
        // first-version root to compare against.
        return Ok(());
    };
    let first_ani = first
        .get("archetype_node_id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let first_category = first
        .pointer("/category/defining_code/code_string")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let incoming_ani = canonical.get("archetype_node_id").and_then(Value::as_str);
    if let (Some(stored), Some(incoming)) = (first_ani.as_deref(), incoming_ani)
        && stored != incoming
    {
        return Err(ServiceError::Unprocessable(format!(
            "COMPOSITION archetype_node_id {incoming:?} differs from the versioned \
             object's first version {stored:?} \
             (VERSIONED_COMPOSITION.Archetype_node_id_valid)"
        )));
    }
    let incoming_category = canonical
        .pointer("/category/defining_code/code_string")
        .and_then(Value::as_str);
    if let (Some(stored), Some(incoming)) = (first_category.as_deref(), incoming_category)
        && (stored == PERSISTENT) != (incoming == PERSISTENT)
    {
        return Err(ServiceError::Unprocessable(format!(
            "COMPOSITION category {incoming} changes the persistence of the versioned \
             object (first version: {stored}) — is_persistent is fixed across versions \
             (VERSIONED_COMPOSITION.Persistent_validity)"
        )));
    }
    Ok(())
}

/// The OPT `template_id` a COMPOSITION declares
/// (`archetype_details.template_id.value`), if any.
pub(super) fn composition_template_id(composition: &Value) -> Option<&str> {
    composition
        .pointer("/archetype_details/template_id/value")
        .and_then(Value::as_str)
}

/// Whether a COMPOSITION is `431|persistent|` (RM composition,
/// `COMPOSITION.category` / `is_persistent()`), read from
/// `category.defining_code.code_string`.
fn is_persistent(composition: &Value) -> bool {
    composition
        .pointer("/category/defining_code/code_string")
        .and_then(Value::as_str)
        == Some("431")
}
