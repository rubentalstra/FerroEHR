//! The commit-validation choke point for every EHR-owned kind — the
//! structural validators for `EHR_STATUS` / `EHR_ACCESS` / FOLDER, the
//! COMPOSITION RM + terminology + template validation, the
//! `validate_for_commit` dispatch shared by the direct and CONTRIBUTION write
//! paths, and the `VERSIONED_COMPOSITION` cross-version invariants.
//!
//! Every kind's validator pairs its hand-written root rules with the
//! whole-instance RM class-invariant + terminology pass
//! ([`validate_rm_invariants_for_commit`]) — the RM class invariants are
//! properties of the instance, so they bind below the root of an `EHR_STATUS`
//! or FOLDER exactly as they do inside a COMPOSITION.
//!
//! Spec: RM ehr `ehr_status.adoc` / `ehr_access.adoc` /
//! `versioned_composition.adoc`; RM common `folder.adoc` + inherited
//! `locatable.adoc` (`Links_valid`, `Archetype_node_id_valid`),
//! `archetyped.adoc` (`Rm_version_valid`), `link.adoc` (meaning/type/target
//! 1..1) and `feeder_audit_details.adoc` (`System_id_valid`); RM common
//! master06 §Incomplete Content (the `553|incomplete|` relaxation); ITS-REST
//! `responses/422_COMPOSITION.yaml`; CNF `master06 §Test Data Sets`
//! (INVALID class 2) + `master07-func_tc_ehr_composition.adoc` (the
//! persistent-cardinality convention).

use serde_json::Value;
use sqlx::PgConnection;

use crate::ids::{EhrId, VoId};
use crate::service::FerroEhrService;
use crate::service::error::ServiceError;
use crate::versioning::read::read_current;
use crate::versioning::{Kind, lifecycle};

impl FerroEhrService {
    /// Enforce the CNF persistent-COMPOSITION uniqueness convention: an EHR may
    /// hold only one *live* persistent COMPOSITION per template.
    ///
    /// NOTE: the openEHR RM does **not** define this cardinality — the CNF
    /// schedule records it as "under debate in the openEHR SEC … due to the
    /// lack of information in the openEHR specifications"
    /// (`CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`).
    /// We adopt the CNF criterion (`create_composition-same_opt_twice`). Only
    /// persistent COMPOSITIONs with a declared template are constrained.
    ///
    /// # Errors
    /// [`ServiceError::Conflict`] when a live persistent COMPOSITION for the
    /// same template already exists in the EHR; [`ServiceError::Database`] on
    /// a storage failure.
    pub(super) async fn reject_duplicate_persistent(
        &self,
        ehr_id: EhrId,
        composition: &Value,
    ) -> Result<(), ServiceError> {
        if !is_persistent(composition) {
            return Ok(());
        }
        let Some(template_id) = composition_template_id(composition) else {
            return Ok(());
        };
        // TODO(#1445, perf): scans the EHR's live COMPOSITIONs and reassembles each to
        // read its category + template (template_id is not promoted onto
        // vo_version). An EHR holds few persistent compositions.
        let vo_ids = crate::storage::version_repo::meta::current_vo_ids(
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

    /// Validate an incoming COMPOSITION against its operational template
    /// before it is persisted (the single choke point for the JSON and FLAT
    /// dispatch paths). RM class-invariant + terminology passes run
    /// unconditionally (template-independent); the archetype-conformance pass
    /// is gated on a declared+resolved template. A
    /// declared-but-failing template, or any RM/terminology violation, is a
    /// `422` (ITS-REST `responses/422_COMPOSITION.yaml`); syntactic parse
    /// failures are `400` and caught earlier at the REST negotiation edge.
    ///
    /// `incomplete` (a `553|incomplete|` CONTRIBUTION version, RM common
    /// master06 §Incomplete Content) relaxes the archetype/template existence
    /// & cardinality **lower** limits to zero; RM invariants + terminology
    /// stay at full strictness ("data may be missing, but it may not be
    /// wrong").
    ///
    /// The template lookup goes through `web_template_for` and the passes
    /// through `openehr_its::flat::validation::validate_*`.
    ///
    /// # Errors
    /// [`ServiceError::ValidationFailed`] carrying every RM/terminology/
    /// template violation (→ 422); [`ServiceError`] from a failing template
    /// resolution.
    #[expect(
        clippy::as_conversions,
        reason = "per-pass failure counts widen exactly for the metrics facade: usize \
                  is at most 64 bits on every supported target"
    )]
    pub(super) async fn validate_composition_for_commit(
        &self,
        composition: &Value,
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        let mut messages = openehr_its::flat::validation::validate_rm_and_terminology(composition);
        let rm_terminology_failures = messages.len();
        let mut template_failures = 0;
        let mut binding_failures = 0;
        if let Some(template_id) = composition
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
        {
            let wt = self.web_template_for(template_id).await?;
            messages.extend(if incomplete {
                openehr_its::flat::validation::validate_archetype_conformance_incomplete(
                    composition,
                    &wt,
                )
            } else {
                openehr_its::flat::validation::validate_archetype_conformance(composition, &wt)
            });
            template_failures = messages.len() - rm_terminology_failures;
            // Archetype constraint bindings (ac-code → external value set)
            // resolve against the routed terminology servers. A no-op — and
            // free of any remote call — unless `[terminology.external]` is
            // configured (BASE `architecture_overview/master12-terminology.adoc`
            // §"Binding Terminology Value-sets to Archetypes"). The relaxation
            // for a `553|incomplete|` commit does not reach it: a code that IS
            // present may not be wrong (RM common master06 §Incomplete
            // Content). Counted as its own pass — a bound value set the
            // terminology server rejects is a different operational signal
            // from an archetype-shape violation.
            let bindings = self.constraint_binding_violations(composition, &wt).await;
            binding_failures = bindings.len();
            messages.extend(bindings);
        }
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
        if binding_failures > 0 {
            metrics::counter!(
                crate::telemetry::prometheus::VALIDATION_FAILURES,
                "pass" => "constraint_binding",
            )
            .increment(binding_failures as u64);
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
    /// CONTRIBUTION) — the [`crate::versioning::CommitEnv`]
    /// `validate_for_commit` hook. COMPOSITIONs get full RM + terminology +
    /// template validation; the EHR-owned kinds (`EHR_STATUS` / `EHR_ACCESS`
    /// / FOLDER) get structural RM validation. Shared by the direct
    /// create/update path and the CONTRIBUTION path so neither can bypass
    /// validation.
    ///
    /// The demographic arms (party roots + `PARTY_RELATIONSHIP`) dispatch to
    /// the demographic register (`service/demographic/`).
    ///
    /// # Errors
    /// [`ServiceError::ValidationFailed`] / [`ServiceError::Unprocessable`]
    /// when the content is invalid for its kind (→ 422); [`ServiceError`]
    /// from a failing template resolution on the COMPOSITION arm.
    #[expect(
        clippy::same_name_method,
        reason = "the `CommitEnv` seam (service/commit_env.rs) deliberately \
                  mirrors these chapter method names so the versioning layer \
                  calls them by their own vocabulary; that impl disambiguates \
                  explicitly with `FerroEhrService::<name>(self, …)`"
    )]
    pub(in crate::service) async fn validate_for_commit(
        &self,
        kind: Kind,
        data: &Value,
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        match kind {
            Kind::Composition => self.validate_composition_for_commit(data, incomplete).await,
            Kind::EhrStatus => validate_ehr_status(data),
            Kind::EhrAccess => validate_ehr_access(data),
            Kind::Folder => validate_folder(data),
            Kind::Agent | Kind::Group | Kind::Organisation | Kind::Person | Kind::Role => {
                crate::service::demographic::validate::validate_party_kind_for_commit(kind, data)
            }
            Kind::PartyRelationship => {
                crate::service::demographic::validate::validate_relationship_for_commit(data)
            }
        }
    }
}

/// Run the **template-independent** RM class-invariant + openEHR-terminology
/// passes over a non-COMPOSITION commit body, folding every violation into one
/// `422`.
///
/// The RM class invariants are properties of the *instance*, not of the
/// resource kind: `ARCHETYPED.Rm_version_valid`
/// (`RM/docs/UML/classes/org.openehr.rm.common.archetyped.adoc` §Invariants),
/// `LOCATABLE.Links_valid` and `Archetype_node_id_valid`
/// (`…org.openehr.rm.common.locatable.adoc` §Invariants), `LINK`'s three 1..1
/// attributes `meaning`/`type`/`target`
/// (`…org.openehr.rm.common.link.adoc` §Attributes) and
/// `FEEDER_AUDIT_DETAILS.System_id_valid`
/// (`…org.openehr.rm.common.feeder_audit_details.adoc` §Invariants) bind every
/// node carrying the shape, at any depth — so the pass the COMPOSITION arm
/// runs ([`FerroEhrService::validate_composition_for_commit`]) applies
/// unchanged to `EHR_STATUS` / `EHR_ACCESS` / FOLDER / demographic bodies.
/// Without it, a defect below the root of those kinds is invisible: only the
/// hand-written root checks above ever looked at them.
///
/// `declared` is the root node's RM type, used only when the root's wire
/// `_type` is absent (canonical JSON requires `_type` only on polymorphic
/// slots); every descendant dispatches from its own tag or its parent
/// attribute's declared concrete type.
///
/// # Errors
/// [`ServiceError::ValidationFailed`] carrying every violation, keyed by its RM
/// instance path (→ 422), exactly as the COMPOSITION arm reports its
/// RM/terminology pass.
pub(in crate::service) fn validate_rm_invariants_for_commit(
    data: &Value,
    declared: &str,
) -> Result<(), ServiceError> {
    let messages = openehr_its::flat::validation::validate_rm_and_terminology_as(data, declared);
    if messages.is_empty() {
        return Ok(());
    }
    metrics::counter!(
        crate::telemetry::prometheus::VALIDATION_FAILURES,
        "pass" => "rm_terminology",
    )
    .increment(u64::try_from(messages.len()).unwrap_or(u64::MAX));
    Err(ServiceError::ValidationFailed(
        messages
            .into_iter()
            .map(|m| openehr_its::rest::runtime::ValidationError {
                path: m.path,
                message: m.message,
            })
            .collect(),
    ))
}

/// `VERSIONED_COMPOSITION` cross-version invariants, enforced against the
/// FIRST stored version's root (RM ehr `versioned_composition.adoc`):
///
/// - `Archetype_node_id_valid`: every version's `data.archetype_node_id`
///   equals the first version's — a versioned composition cannot switch
///   archetype;
/// - `Persistent_validity`: every version's `is_persistent` equals the first
///   version's — the persistence category (`431|persistent|`) is fixed for
///   the container's life.
///
/// A violating modification is a `422` naming the invariant. Lifted out of the
/// versioning write path — the EHR chapter owns it. The first-version root
/// read goes through `crate::storage::node_repo`.
///
/// Both write flows run this: the direct update path
/// ([`composition`](super::composition)) inline, and the CONTRIBUTION path
/// (`crate::versioning::contribution::commit_version_set`) through the
/// [`crate::versioning::CommitEnv::pre_composition_modify`] hook — each in its
/// own commit transaction.
///
/// # Errors
/// [`ServiceError::Unprocessable`] naming the violated invariant (→ 422);
/// [`ServiceError::Database`] if the first-version root read fails.
pub(in crate::service) async fn check_versioned_composition_invariants(
    tx: &mut PgConnection,
    vo_id: VoId,
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
/// (`archetype_details.template_id.value`), if any. `pub(crate)` because the
/// CONTRIBUTION commit path stamps `vo_version.template_id` with the same
/// derivation as the direct composition path (the template-delete 409 guard
/// counts that column — physical deletes never orphan committed data).
pub(crate) fn composition_template_id(composition: &Value) -> Option<&str> {
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

/// Structurally validate an `EHR_STATUS` before it is committed (on EHR
/// create, `EHR_STATUS` update, or a CONTRIBUTION). Rejects every malformed
/// data set the CNF `master06 §Test Data Sets` (INVALID class 2) enumerates
/// with a `422`.
///
/// Rules — RM ehr §`EHR_STATUS` + inherited `LOCATABLE`:
/// - `_type` present and equal to `EHR_STATUS`;
/// - `name` present (`LOCATABLE.name` 1..1);
/// - `archetype_node_id` present and non-empty (`Archetype_node_id_valid`);
/// - `is_queryable` / `is_modifiable` present booleans (both 1..1);
/// - `subject` present and a `PARTY_SELF` (`EHR_STATUS.subject` 1..1
///   `PARTY_SELF`; monomorphic, so a foreign concrete `_type` is invalid —
///   enforced via the generated `PartySelf`'s `_type` check). An empty `{}`
///   subject is a valid **anonymous** subject (RM ehr master04 §EHR Status:
///   `PARTY_SELF` "enabling it to be made completely anonymous");
/// - a present `subject.external_ref` is a valid `PARTY_REF` (non-empty
///   `id.value` — `Id_exists`; non-empty `namespace` — `Namespace_valid`); a
///   NULL `external_ref` is permitted;
/// - a present `other_details` is a concrete `ITEM_STRUCTURE` (RM ehr
///   `ehr_status.adoc` `other_details`; RM `data_structures` master04).
///
/// The root rules above are then followed by the whole-instance RM
/// class-invariant + terminology pass
/// ([`validate_rm_invariants_for_commit`]), which reaches every node BELOW the
/// root — `archetype_details`, `links`, `feeder_audit`, and any
/// `other_details` structure.
///
/// # Errors
/// [`ServiceError::Unprocessable`] naming the first violated rule, or
/// [`ServiceError::ValidationFailed`] carrying the RM-invariant violations
/// found below the root (both → 422).
/// The root-LOCATABLE invariants shared by the always-root EHR kinds
/// (`EHR_STATUS`, `EHR_ACCESS` — RM ehr `ehr_status.adoc` /
/// `ehr_access.adoc`, each with an unconditional `Is_archetype_root`):
///
/// - `Archetyped_valid` (RM common `locatable.adoc`: `is_archetype_root xor
///   archetype_details = Void`) — a root MUST carry the `ARCHETYPED` block;
/// - at a root, `archetype_node_id` "is always the stringified form of the
///   `archetype_id` found in the `archetype_details` object"
///   (`locatable.adoc` §`archetype_node_id`);
/// - `Links_valid` (`links /= Void implies not links.is_empty`) — an
///   explicit empty list is RM-invalid (absent is the way to say none).
pub(in crate::service) fn validate_root_locatable(
    obj: &serde_json::Map<String, Value>,
    kind: &str,
) -> Result<(), ServiceError> {
    let unproc = ServiceError::Unprocessable;
    let details = obj
        .get("archetype_details")
        .filter(|v| v.is_object())
        .ok_or_else(|| {
            unproc(format!(
                "{kind}.archetype_details is mandatory: {kind} is an archetype \
                 root (Is_archetype_root) and a root without ARCHETYPED \
                 violates LOCATABLE.Archetyped_valid"
            ))
        })?;
    let declared_id = details
        .get("archetype_id")
        .and_then(|a| a.get("value"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            unproc(format!(
                "{kind}.archetype_details.archetype_id.value is mandatory \
                 (ARCHETYPED.archetype_id 1..1)"
            ))
        })?;
    let node_id = obj
        .get("archetype_node_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if declared_id != node_id {
        return Err(unproc(format!(
            "{kind}.archetype_node_id {node_id:?} must equal \
             archetype_details.archetype_id.value {declared_id:?} at an \
             archetype root (LOCATABLE archetype_node_id)"
        )));
    }
    if let Some(links) = obj.get("links")
        && links.as_array().is_some_and(Vec::is_empty)
    {
        return Err(unproc(format!(
            "{kind}.links must be absent or non-empty (LOCATABLE.Links_valid)"
        )));
    }
    Ok(())
}

pub(in crate::service) fn validate_ehr_status(status: &Value) -> Result<(), ServiceError> {
    let unproc = |m: String| ServiceError::Unprocessable(m);
    let obj = status
        .as_object()
        .ok_or_else(|| unproc("EHR_STATUS must be a JSON object".to_owned()))?;

    match obj.get("_type").and_then(Value::as_str) {
        Some("EHR_STATUS") => {}
        Some(other) => {
            return Err(unproc(format!(
                "EHR_STATUS _type must be \"EHR_STATUS\", got {other:?}"
            )));
        }
        None => {
            return Err(unproc(
                "EHR_STATUS is missing its _type discriminator".to_owned(),
            ));
        }
    }

    if !obj.contains_key("name") {
        return Err(unproc(
            "EHR_STATUS.name is mandatory (LOCATABLE.name 1..1)".to_owned(),
        ));
    }
    match obj.get("archetype_node_id").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => {}
        _ => {
            return Err(unproc(
                "EHR_STATUS.archetype_node_id is mandatory and non-empty \
                 (LOCATABLE.Archetype_node_id_valid)"
                    .to_owned(),
            ));
        }
    }
    validate_root_locatable(obj, "EHR_STATUS")?;
    if !obj.get("is_queryable").is_some_and(Value::is_boolean) {
        return Err(unproc(
            "EHR_STATUS.is_queryable is mandatory (1..1 Boolean)".to_owned(),
        ));
    }
    if !obj.get("is_modifiable").is_some_and(Value::is_boolean) {
        return Err(unproc(
            "EHR_STATUS.is_modifiable is mandatory (1..1 Boolean)".to_owned(),
        ));
    }

    let subject = obj
        .get("subject")
        .filter(|v| v.is_object())
        .ok_or_else(|| unproc("EHR_STATUS.subject is mandatory (1..1 PARTY_SELF)".to_owned()))?;

    // `EHR_STATUS.subject` is typed `PARTY_SELF` (RM ehr master04 §EHR Status).
    // PARTY_SELF is monomorphic, so a foreign concrete `_type` (e.g.
    // PARTY_IDENTIFIED) is invalid; enforce via the generated type's
    // `#[derive(OpenEhrType)]` `_type` check. An absent `_type` / empty `{}`
    // deserialises to an anonymous PARTY_SELF (external_ref None), which is
    // accepted. Scoped to the subject slot to keep the RM-1.2.0-vs-corpus skew
    // off the whole-object guard.
    openehr_its::json::from_canonical_value::<openehr_rm::prelude::PartySelf>(subject).map_err(
        |e| {
            unproc(format!(
                "EHR_STATUS.subject must be a PARTY_SELF (RM ehr master04 §EHR Status): {e}"
            ))
        },
    )?;

    let external_ref = subject
        .as_object()
        .and_then(|s| s.get("external_ref"))
        .filter(|v| !v.is_null());
    if let Some(external_ref) = external_ref {
        let ext = external_ref.as_object().ok_or_else(|| {
            unproc("EHR_STATUS.subject.external_ref must be a PARTY_REF object".to_owned())
        })?;
        match ext.get("id").and_then(Value::as_object) {
            Some(id)
                if id
                    .get("value")
                    .and_then(Value::as_str)
                    .is_some_and(|v| !v.is_empty()) => {}
            _ => {
                return Err(unproc(
                    "EHR_STATUS.subject.external_ref.id.value is mandatory and non-empty \
                     (OBJECT_REF.Id_exists)"
                        .to_owned(),
                ));
            }
        }
        match ext.get("namespace").and_then(Value::as_str) {
            Some(ns) if !ns.is_empty() => {}
            _ => {
                return Err(unproc(
                    "EHR_STATUS.subject.external_ref.namespace is mandatory and non-empty \
                     (OBJECT_REF.Namespace_valid)"
                        .to_owned(),
                ));
            }
        }
    }

    // `EHR_STATUS.other_details` (0..1) is typed `ITEM_STRUCTURE` — an abstract
    // slot whose concrete subtypes are ITEM_TREE / ITEM_LIST / ITEM_SINGLE /
    // ITEM_TABLE (RM data_structures master04). A foreign `_type` is invalid.
    if let Some(other) = obj.get("other_details").filter(|v| !v.is_null()) {
        match other.get("_type").and_then(Value::as_str) {
            Some("ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE") => {}
            other_ty => {
                return Err(unproc(format!(
                    "EHR_STATUS.other_details must be an ITEM_STRUCTURE \
                     (ITEM_TREE/ITEM_LIST/ITEM_SINGLE/ITEM_TABLE), got _type {other_ty:?}"
                )));
            }
        }
    }
    validate_rm_invariants_for_commit(status, "EHR_STATUS")
}

/// Validate a client-supplied `EHR_ACCESS` before it is committed (via a
/// CONTRIBUTION — there is no direct ITS-REST `EHR_ACCESS` write). RM ehr
/// `ehr_access.adoc`:
///
/// - a LOCATABLE: `name` (1..1) and a non-empty `archetype_node_id`
///   (`Archetype_node_id_valid`);
/// - a foreign `_type` in this slot is invalid (the container holds
///   `EHR_ACCESS` only);
/// - `settings` (0..1) is a subtype of the ABSTRACT `ACCESS_CONTROL_SETTINGS`
///   — the RM defines no concrete scheme, so a present `settings` must carry a
///   non-empty concrete `_type`, which `scheme()` names (`Scheme_valid`).
///
/// Followed by the whole-instance RM class-invariant + terminology pass
/// ([`validate_rm_invariants_for_commit`]), which reaches every node below the
/// root.
///
/// # Errors
/// [`ServiceError::Unprocessable`] naming the first violated rule, or
/// [`ServiceError::ValidationFailed`] carrying the RM-invariant violations
/// found below the root (both → 422).
pub(in crate::service) fn validate_ehr_access(access: &Value) -> Result<(), ServiceError> {
    let unproc = |m: String| ServiceError::Unprocessable(m);
    let obj = access
        .as_object()
        .ok_or_else(|| unproc("EHR_ACCESS must be a JSON object".to_owned()))?;
    match obj.get("_type").and_then(Value::as_str) {
        None | Some("EHR_ACCESS") => {}
        Some(other) => {
            return Err(unproc(format!(
                "expected an EHR_ACCESS, got _type {other:?}"
            )));
        }
    }
    if obj.get("name").is_none_or(Value::is_null) {
        return Err(unproc(
            "EHR_ACCESS.name is mandatory (LOCATABLE.name 1..1)".to_owned(),
        ));
    }
    if obj
        .get("archetype_node_id")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(unproc(
            "EHR_ACCESS.archetype_node_id is mandatory and non-empty \
             (LOCATABLE.Archetype_node_id_valid)"
                .to_owned(),
        ));
    }
    // EHR_ACCESS carries the same unconditional Is_archetype_root as
    // EHR_STATUS (RM ehr `ehr_access.adoc`), so the root-LOCATABLE
    // invariants apply identically.
    validate_root_locatable(obj, "EHR_ACCESS")?;
    if let Some(settings) = obj.get("settings").filter(|v| !v.is_null())
        && settings
            .get("_type")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Err(unproc(
            "EHR_ACCESS.settings must be a concrete ACCESS_CONTROL_SETTINGS subtype \
             carrying its _type — the scheme name (EHR_ACCESS.Scheme_valid)"
                .to_owned(),
        ));
    }
    // NOTE: `settings` is excluded from the whole-instance RM pass, and only
    // from it. `EHR_ACCESS.settings` is the RM's one implementation-defined
    // slot — "Instance is a subtype of the type `ACCESS_CONTROL_SETTINGS`,
    // allowing for the use of different access control schemes"
    // (`RM/docs/UML/classes/org.openehr.rm.ehr.ehr_access.adoc` §Attributes) —
    // and the slot's type is abstract with no attributes, no invariants and no
    // RM-defined descendant: "Access Control Settings for the EHR and
    // components. Intended to support multiple access control schemes.
    // Currently implementation dependent."
    // (`…org.openehr.rm.ehr.access_control_settings.adoc`). So the RM defines
    // NOTHING for the pass to judge inside it, while a pass that walked in
    // would refuse every legal instance (the scheme's own `_type` names a class
    // the RM does not declare). `Scheme_valid` above is the whole of the RM's
    // demand on the slot.
    let mut without_settings = access.clone();
    if let Some(map) = without_settings.as_object_mut() {
        map.remove("settings");
    }
    validate_rm_invariants_for_commit(&without_settings, "EHR_ACCESS")
}

/// Validate a client-supplied FOLDER tree before it is committed (directory
/// create/update and the CONTRIBUTION FOLDER path). RM common `folder.adoc` +
/// RM ehr master04 §Folders:
///
/// - each node is a `FOLDER` (foreign `_type` rejected) with `name` (1..1) and
///   a non-empty `archetype_node_id` (`Archetype_node_id_valid`);
/// - `items` members are `OBJECT_REF`s — "Folder structures do not contain
///   Compositions, only references to them" (master04 §Folders): a member must
///   carry `id` + `namespace` + `type`, and a LOCATABLE-by-value payload is
///   rejected;
/// - `links`, when present, is non-empty (`LOCATABLE.Links_valid`:
///   `links /= Void implies not links.is_empty`, RM common
///   `org.openehr.rm.common.locatable.adoc` §Invariants);
/// - a node carrying `archetype_details` is an archetype root, so its
///   `archetype_node_id` is the stringified `archetype_details.archetype_id`
///   (RM common `org.openehr.rm.common.locatable.adoc` §Attributes: "At an
///   archetype root point, the value of this attribute is always the
///   stringified form of the `archetype_id` found in the `archetype_details`
///   object"; RM common `master03-archetyped_package.adoc` §The LOCATABLE
///   Class). `archetype_details` itself stays OPTIONAL on a FOLDER — the RM
///   types it 0..1 and FOLDER carries no `Is_archetype_root` invariant;
/// - `folders` members recurse.
///
/// The per-node rules above are then followed by the whole-instance RM
/// class-invariant + terminology pass
/// ([`validate_rm_invariants_for_commit`]), which reaches the parts of a
/// FOLDER tree the walk above does not inspect — `archetype_details`, each
/// `LINK` in `links`, and `feeder_audit`.
///
/// # Errors
/// [`ServiceError::Unprocessable`] naming the first violated rule and the
/// offending tree path, or [`ServiceError::ValidationFailed`] carrying the
/// RM-invariant violations found anywhere in the tree (both → 422).
pub(in crate::service) fn validate_folder(folder: &Value) -> Result<(), ServiceError> {
    fn walk(node: &Value, path: &str) -> Result<(), ServiceError> {
        let unproc = |m: String| ServiceError::Unprocessable(m);
        let obj = node
            .as_object()
            .ok_or_else(|| unproc(format!("{path}: FOLDER must be a JSON object")))?;
        match obj.get("_type").and_then(Value::as_str) {
            None | Some("FOLDER") => {}
            Some(other) => {
                return Err(unproc(format!(
                    "{path}: expected a FOLDER, got _type {other:?}"
                )));
            }
        }
        if obj.get("name").is_none_or(Value::is_null) {
            return Err(unproc(format!(
                "{path}: FOLDER.name is mandatory (LOCATABLE.name 1..1)"
            )));
        }
        let node_id = obj.get("archetype_node_id").and_then(Value::as_str);
        let Some(node_id) = node_id.filter(|s| !s.is_empty()) else {
            return Err(unproc(format!(
                "{path}: FOLDER.archetype_node_id is mandatory and non-empty \
                 (LOCATABLE.Archetype_node_id_valid)"
            )));
        };
        if obj
            .get("links")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            return Err(unproc(format!(
                "{path}: FOLDER.links is present but empty — a present list must be \
                 non-empty (LOCATABLE.Links_valid)"
            )));
        }
        if let Some(archetype_id) = obj
            .get("archetype_details")
            .and_then(|d| d.get("archetype_id"))
            .and_then(|a| a.get("value"))
            .and_then(Value::as_str)
            && archetype_id != node_id
        {
            return Err(unproc(format!(
                "{path}: archetype root archetype_node_id {node_id:?} is not the \
                 stringified archetype_details.archetype_id {archetype_id:?} — at an \
                 archetype root the two are always the same value \
                 (LOCATABLE.archetype_node_id)"
            )));
        }
        if let Some(items) = obj.get("items").and_then(Value::as_array) {
            for (i, item) in items.iter().enumerate() {
                let ok = item.get("id").is_some_and(Value::is_object)
                    && item
                        .get("namespace")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                    && item
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                    // A LOCATABLE by value carries archetype_node_id — an
                    // OBJECT_REF never does.
                    && item.get("archetype_node_id").is_none();
                if !ok {
                    return Err(unproc(format!(
                        "{path}/items[{i}]: FOLDER.items members must be OBJECT_REFs \
                         (id + namespace + type) — Folder structures do not contain \
                         Compositions by value, only references to them \
                         (RM ehr master04 §Folders)"
                    )));
                }
            }
        }
        if let Some(folders) = obj.get("folders").and_then(Value::as_array) {
            for (i, sub) in folders.iter().enumerate() {
                walk(sub, &format!("{path}/folders[{i}]"))?;
            }
        }
        Ok(())
    }
    walk(folder, "")?;
    validate_rm_invariants_for_commit(folder, "FOLDER")
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{validate_ehr_access, validate_ehr_status, validate_folder};
    use crate::service::ehr::access::default_ehr_access;
    use crate::service::ehr::service::default_ehr_status;
    use crate::service::error::ServiceError;

    /// `EHR_STATUS.other_details` must be a concrete `ITEM_STRUCTURE`
    /// (RM ehr `ehr_status.adoc`): the four concrete subtypes pass, a foreign
    /// or missing `_type` rejects.
    #[test]
    fn ehr_status_other_details_type_is_enforced() {
        let with_other = |other: Value| {
            let mut st = default_ehr_status();
            st.as_object_mut()
                .unwrap()
                .insert("other_details".into(), other);
            st
        };
        // Each subtype is spelled out at its own mandatory shape: `ITEM_SINGLE.item`
        // is `ELEMENT [1..1]` (RM `data_structures`
        // `org.openehr.rm.data_structures.item_single.adoc` §Attributes), while
        // ITEM_TREE/ITEM_LIST `items` and ITEM_TABLE `rows` are 0..1. The
        // whole-instance RM pass reaches `other_details`, so an ITEM_SINGLE
        // without its item is refused on its own merits — the acceptance
        // asserted here is of a *valid* instance of each subtype.
        for other in [
            json!({ "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "d" },
                    "archetype_node_id": "at0001" }),
            json!({ "_type": "ITEM_LIST", "name": { "_type": "DV_TEXT", "value": "d" },
                    "archetype_node_id": "at0001" }),
            json!({ "_type": "ITEM_SINGLE", "name": { "_type": "DV_TEXT", "value": "d" },
                    "archetype_node_id": "at0001",
                    "item": { "_type": "ELEMENT",
                              "name": { "_type": "DV_TEXT", "value": "e" },
                              "archetype_node_id": "at0002",
                              "value": { "_type": "DV_TEXT", "value": "v" } } }),
            json!({ "_type": "ITEM_TABLE", "name": { "_type": "DV_TEXT", "value": "d" },
                    "archetype_node_id": "at0001" }),
        ] {
            let ty = other["_type"].as_str().unwrap().to_owned();
            validate_ehr_status(&with_other(other))
                .unwrap_or_else(|e| panic!("{ty} other_details must be accepted: {e:?}"));
        }
        for bad in [
            json!({ "_type": "DV_TEXT", "value": "x" }),
            json!({ "value": "x" }),
        ] {
            let err = validate_ehr_status(&with_other(bad))
                .expect_err("non-ITEM_STRUCTURE other_details must be rejected");
            assert!(err.to_string().contains("ITEM_STRUCTURE"), "got {err}");
        }
    }

    #[test]
    fn default_and_typical_ehr_status_are_accepted() {
        validate_ehr_status(&default_ehr_status()).expect("default EHR_STATUS");
        // A subject identified via external_ref is still a PARTY_SELF (RM ehr
        // master04 §EHR Status).
        let identified = json!({
            "_type": "EHR_STATUS",
            "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
            "archetype_details": {
                "_type": "ARCHETYPED",
                "archetype_id": { "_type": "ARCHETYPE_ID",
                                  "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                "rm_version": "1.2.0"
            },
            "name": { "_type": "DV_TEXT", "value": "EHR Status" },
            "subject": {
                "_type": "PARTY_SELF",
                "external_ref": {
                    "_type": "PARTY_REF",
                    "namespace": "conformance",
                    "type": "PERSON",
                    "id": { "_type": "GENERIC_ID", "value": "subj-1", "scheme": "id_scheme" }
                }
            },
            "is_queryable": true,
            "is_modifiable": false
        });
        validate_ehr_status(&identified).expect("identified PARTY_SELF EHR_STATUS");
    }

    /// A subject typed with a foreign concrete `PARTY_PROXY` subtype
    /// (`PARTY_IDENTIFIED`) is rejected — `EHR_STATUS.subject` is monomorphic
    /// `PARTY_SELF` (RM ehr master04 §EHR Status).
    #[test]
    fn ehr_status_subject_wrong_type_is_rejected() {
        let bad = json!({
            "_type": "EHR_STATUS",
            "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
            "archetype_details": {
                "_type": "ARCHETYPED",
                "archetype_id": { "_type": "ARCHETYPE_ID",
                                  "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                "rm_version": "1.2.0"
            },
            "name": { "_type": "DV_TEXT", "value": "EHR Status" },
            "subject": {
                "_type": "PARTY_IDENTIFIED",
                "external_ref": {
                    "_type": "PARTY_REF",
                    "namespace": "conformance",
                    "type": "PERSON",
                    "id": { "_type": "GENERIC_ID", "value": "subj-1", "scheme": "id_scheme" }
                }
            },
            "is_queryable": true,
            "is_modifiable": true
        });
        let err = validate_ehr_status(&bad).expect_err("PARTY_IDENTIFIED subject must be rejected");
        let msg = err.to_string();
        assert!(
            msg.contains("PARTY_SELF") && msg.contains("PARTY_IDENTIFIED"),
            "rejection should name the type mismatch, got: {msg}"
        );
    }

    /// An anonymous subject — empty `{}` or `{"_type":"PARTY_SELF"}` with no
    /// `external_ref` — is accepted (RM ehr master04 §EHR Status: "completely
    /// anonymous").
    #[test]
    fn anonymous_ehr_status_subject_is_accepted() {
        for subject in [json!({}), json!({ "_type": "PARTY_SELF" })] {
            let status = json!({
                "_type": "EHR_STATUS",
                "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
                "archetype_details": {
                    "_type": "ARCHETYPED",
                    "archetype_id": { "_type": "ARCHETYPE_ID",
                                      "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
                    "rm_version": "1.2.0"
                },
                "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                "subject": subject,
                "is_queryable": true,
                "is_modifiable": true
            });
            validate_ehr_status(&status).expect("anonymous PARTY_SELF EHR_STATUS");
        }
    }

    /// Every vendored `EHR_STATUS` data set the CNF corpus labels invalid
    /// (`master06 §Test Data Sets`, INVALID class 2) must be rejected.
    ///
    /// Re-adjudicated with the `Archetyped_valid` enforcement: the former
    /// exception `001_ehr_status_subject_empty.json` (its `subject: {}` IS
    /// spec-valid — an empty `PARTY_SELF` is a completely anonymous subject,
    /// RM ehr master04) is nonetheless rejected as a WHOLE, because like
    /// every fixture in this corpus it carries a root `archetype_node_id`
    /// with no `archetype_details` (RM common `locatable.adoc`
    /// `Archetyped_valid`; RM ehr `ehr_status.adoc` `Is_archetype_root`) —
    /// so its INVALID label holds, on different grounds than the corpus
    /// intended. The subject-emptiness half of the old adjudication is
    /// pinned by `anonymous_ehr_status_subject_is_accepted` above.
    #[test]
    fn every_invalid_ehr_status_fixture_is_rejected() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/ehr/invalid"
        );
        let mut checked = 0u32;
        for entry in std::fs::read_dir(dir).expect("read ehr/invalid") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read fixture");
            let status: Value = serde_json::from_str(&text).expect("parse fixture");
            assert!(
                validate_ehr_status(&status).is_err(),
                "invalid EHR_STATUS fixture was accepted: {}",
                path.display()
            );
            checked += 1;
        }
        assert_eq!(checked, 11, "expected 11 invalid EHR_STATUS fixtures");
    }

    /// `EHR_ACCESS` commit validation (RM ehr `ehr_access.adoc`): LOCATABLE
    /// structure enforced, a present `settings` must be a concrete
    /// `ACCESS_CONTROL_SETTINGS` subtype (its `_type` is the scheme name —
    /// `Scheme_valid`).
    #[test]
    fn ehr_access_commit_validation() {
        validate_ehr_access(&default_ehr_access()).expect("the default EHR_ACCESS is valid");
        let err = validate_ehr_access(&json!({ "_type": "EHR_STATUS" }))
            .expect_err("foreign _type rejected");
        assert!(err.to_string().contains("EHR_ACCESS"), "got {err}");
        let err = validate_ehr_access(&json!({
                   "_type": "EHR_ACCESS", "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID",
                              "value": "openEHR-EHR-EHR_ACCESS.generic.v1" },
            "rm_version": "1.2.0"
        },
               }))
        .expect_err("missing name rejected");
        assert!(err.to_string().contains("name"), "got {err}");
        let err = validate_ehr_access(&json!({
            "_type": "EHR_ACCESS",
            "name": { "_type": "DV_TEXT", "value": "EHR Access" },
            "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
            "archetype_details": {
                "_type": "ARCHETYPED",
                "archetype_id": { "_type": "ARCHETYPE_ID",
                                  "value": "openEHR-EHR-EHR_ACCESS.generic.v1" },
                "rm_version": "1.2.0"
            },
            "settings": { "scheme": "acme" }
        }))
        .expect_err("settings without a concrete _type rejected (Scheme_valid)");
        assert!(err.to_string().contains("Scheme_valid"), "got {err}");
    }

    /// FOLDER trees hold `OBJECT_REF` items only — never content by value
    /// (RM ehr master04 §Folders; RM common `folder.adoc`).
    #[test]
    fn folder_items_must_be_object_refs() {
        let good = json!({
            "_type": "FOLDER",
            "name": { "_type": "DV_TEXT", "value": "root" },
            "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
            "items": [{
                "_type": "OBJECT_REF", "namespace": "local", "type": "VERSIONED_COMPOSITION",
                "id": { "_type": "HIER_OBJECT_ID", "value": "8849182c-82ad-4088-a07f-48ead4180515" }
            }],
            "folders": [{
                "_type": "FOLDER",
                "name": { "_type": "DV_TEXT", "value": "sub" },
                "archetype_node_id": "at0001"
            }]
        });
        validate_folder(&good).expect("a ref-holding folder tree is valid");

        // A COMPOSITION by value inside items is rejected.
        let mut bad = good.clone();
        bad["items"][0] = json!({
            "_type": "COMPOSITION",
            "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
            "name": { "_type": "DV_TEXT", "value": "inline!" }
        });
        let err = validate_folder(&bad).expect_err("content by value must be rejected");
        assert!(err.to_string().contains("OBJECT_REF"), "got {err}");

        // A sub-folder without a name violates LOCATABLE.name 1..1.
        let mut bad = good;
        bad["folders"][0].as_object_mut().unwrap().remove("name");
        let err = validate_folder(&bad).expect_err("nameless sub-folder rejected");
        assert!(err.to_string().contains("name"), "got {err}");
    }

    /// A minimal valid FOLDER tree the LOCATABLE-rule tests below perturb.
    fn folder_fixture() -> Value {
        json!({
            "_type": "FOLDER",
            "name": { "_type": "DV_TEXT", "value": "root" },
            "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
            "archetype_details": {
                "_type": "ARCHETYPED",
                "archetype_id": { "_type": "ARCHETYPE_ID",
                                  "value": "openEHR-EHR-FOLDER.generic.v1" },
                "rm_version": "1.2.0"
            },
            "folders": [{
                "_type": "FOLDER",
                "name": { "_type": "DV_TEXT", "value": "sub" },
                "archetype_node_id": "at0001"
            }]
        })
    }

    /// `LOCATABLE.Links_valid` (`links /= Void implies not links.is_empty`,
    /// RM common `org.openehr.rm.common.locatable.adoc` §Invariants): a
    /// present-but-empty `links` list on any FOLDER node is refused; a
    /// non-empty one is accepted.
    #[test]
    fn folder_links_present_must_be_non_empty() {
        validate_folder(&folder_fixture()).expect("the baseline folder tree is valid");

        let mut bad = folder_fixture();
        bad["folders"][0]
            .as_object_mut()
            .unwrap()
            .insert("links".into(), json!([]));
        let err = validate_folder(&bad).expect_err("an empty links list must be rejected");
        assert!(err.to_string().contains("Links_valid"), "got {err}");

        let mut good = folder_fixture();
        good["folders"][0].as_object_mut().unwrap().insert(
            "links".into(),
            json!([{
                "_type": "LINK",
                "meaning": { "_type": "DV_TEXT", "value": "follow up" },
                "type": { "_type": "DV_TEXT", "value": "issue" },
                "target": { "_type": "DV_EHR_URI", "value": "ehr://example/x" }
            }]),
        );
        validate_folder(&good).expect("a non-empty links list is valid");
    }

    /// RM common `org.openehr.rm.common.locatable.adoc` §Attributes
    /// (`archetype_node_id`): at an archetype root the node id is the
    /// stringified `archetype_details.archetype_id`. A FOLDER without
    /// `archetype_details` stays valid — the attribute is 0..1.
    #[test]
    fn folder_archetype_root_node_id_must_match_details() {
        let mut bad = folder_fixture();
        bad["archetype_details"]["archetype_id"]["value"] = json!("openEHR-EHR-FOLDER.other.v1");
        let err = validate_folder(&bad).expect_err("a contradicting root identity is rejected");
        assert!(
            err.to_string().contains("LOCATABLE.archetype_node_id"),
            "got {err}"
        );

        let mut without = folder_fixture();
        without.as_object_mut().unwrap().remove("archetype_details");
        validate_folder(&without).expect("archetype_details stays optional on a FOLDER");
    }

    // ── the whole-instance RM class-invariant pass on the non-COMPOSITION
    //    commit arms ────────────────────────────────────────────────────────
    //
    // The RM class invariants are properties of the instance, not of the
    // resource kind, so a defect BELOW the root of an EHR_STATUS / FOLDER /
    // demographic body is a 422 exactly as it is inside a COMPOSITION. Before
    // `validate_rm_invariants_for_commit` these arms saw only their root.

    /// `ARCHETYPED.Rm_version_valid` (`not rm_version.is_empty`, RM common
    /// `org.openehr.rm.common.archetyped.adoc` §Invariants) on an `EHR_STATUS`
    /// commit: the block is below the root, so only the whole-instance pass
    /// reaches it. The populated twin stays accepted.
    #[test]
    fn ehr_status_empty_rm_version_is_refused() {
        let mut bad = default_ehr_status();
        bad["archetype_details"]["rm_version"] = json!("");
        let err = validate_ehr_status(&bad).expect_err("an empty rm_version must be refused");
        assert!(
            format!("{err:?}").contains("Rm_version_valid"),
            "the refusal should name the invariant, got {err:?}"
        );
        validate_ehr_status(&default_ehr_status()).expect("a populated rm_version is accepted");
    }

    /// `LINK.meaning` is 1..1 (RM common `org.openehr.rm.common.link.adoc`
    /// §Attributes): a FOLDER carrying an incomplete LINK is refused, while the
    /// complete `ehr://` twin is accepted.
    #[test]
    fn folder_link_missing_meaning_is_refused() {
        let with_link = |link: Value| {
            let mut folder = folder_fixture();
            folder
                .as_object_mut()
                .unwrap()
                .insert("links".into(), json!([link]));
            folder
        };
        let err = validate_folder(&with_link(json!({
            "_type": "LINK",
            "type": { "_type": "DV_TEXT", "value": "issue" },
            "target": { "_type": "DV_EHR_URI", "value": "ehr://example.org/x" }
        })))
        .expect_err("a LINK without its mandatory meaning must be refused");
        assert!(
            format!("{err:?}").contains("meaning"),
            "the refusal should name the missing attribute, got {err:?}"
        );

        validate_folder(&with_link(json!({
            "_type": "LINK",
            "meaning": { "_type": "DV_TEXT", "value": "follow up" },
            "type": { "_type": "DV_TEXT", "value": "issue" },
            "target": { "_type": "DV_EHR_URI", "value": "ehr://example.org/x" }
        })))
        .expect("a complete LINK on a folder is accepted");
    }

    /// The invalid twin of the `ITEM_SINGLE` arm of
    /// [`ehr_status_other_details_type_is_enforced`]: `ITEM_SINGLE.item` is
    /// `ELEMENT [1..1]` (RM `data_structures`
    /// `org.openehr.rm.data_structures.item_single.adoc` §Attributes: "*1..1* |
    /// *item*: `ELEMENT`"), so an `EHR_STATUS.other_details` `ITEM_SINGLE`
    /// WITHOUT its item is a semantically invalid instance and must be refused
    /// as a validation failure — the branch the REST adapter renders 422 — not
    /// stored with a hole where the mandatory attribute belongs.
    #[test]
    fn ehr_status_other_details_item_single_without_item_is_refused() {
        let with_other = |other: Value| {
            let mut status = default_ehr_status();
            status
                .as_object_mut()
                .unwrap()
                .insert("other_details".into(), other);
            status
        };
        let item_single = |item: Option<Value>| {
            let mut o = json!({
                "_type": "ITEM_SINGLE",
                "name": { "_type": "DV_TEXT", "value": "details" },
                "archetype_node_id": "at0001"
            });
            if let Some(item) = item {
                o.as_object_mut().unwrap().insert("item".into(), item);
            }
            o
        };

        let err = validate_ehr_status(&with_other(item_single(None)))
            .expect_err("an ITEM_SINGLE without its mandatory item must be refused");
        assert!(
            matches!(err, ServiceError::ValidationFailed(_)),
            "the refusal must be the validation-failed branch, got {err:?}"
        );
        assert!(
            format!("{err:?}").contains("item"),
            "the refusal should name the missing mandatory attribute, got {err:?}"
        );

        // The valid twin, so the refusal is proven specific to the missing
        // mandatory attribute rather than to ITEM_SINGLE as a shape.
        validate_ehr_status(&with_other(item_single(Some(json!({
            "_type": "ELEMENT",
            "name": { "_type": "DV_TEXT", "value": "e" },
            "archetype_node_id": "at0002",
            "value": { "_type": "DV_TEXT", "value": "v" }
        })))))
        .expect("an ITEM_SINGLE carrying its mandatory item is accepted");
    }

    /// `LOCATABLE.Links_valid` (`links /= Void implies not links.is_empty`, RM
    /// common `org.openehr.rm.common.locatable.adoc` §Invariants) on a node
    /// NESTED inside `EHR_STATUS.other_details`.
    #[test]
    fn ehr_status_nested_empty_links_are_refused() {
        let other_details = |links: Value| {
            json!({
                "_type": "ITEM_TREE",
                "name": { "_type": "DV_TEXT", "value": "details" },
                "archetype_node_id": "at0001",
                "items": [{
                    "_type": "CLUSTER",
                    "name": { "_type": "DV_TEXT", "value": "c" },
                    "archetype_node_id": "at0002",
                    "links": links,
                    "items": [{
                        "_type": "ELEMENT",
                        "name": { "_type": "DV_TEXT", "value": "e" },
                        "archetype_node_id": "at0003",
                        "value": { "_type": "DV_TEXT", "value": "v" }
                    }]
                }]
            })
        };
        let with_other = |other: Value| {
            let mut status = default_ehr_status();
            status
                .as_object_mut()
                .unwrap()
                .insert("other_details".into(), other);
            status
        };
        let err = validate_ehr_status(&with_other(other_details(json!([]))))
            .expect_err("a nested present-but-empty links list must be refused");
        assert!(
            format!("{err:?}").contains("Links_valid"),
            "the refusal should name the invariant, got {err:?}"
        );

        validate_ehr_status(&with_other(other_details(json!([{
            "_type": "LINK",
            "meaning": { "_type": "DV_TEXT", "value": "follow up" },
            "type": { "_type": "DV_TEXT", "value": "issue" },
            "target": { "_type": "DV_EHR_URI", "value": "ehr://example.org/x" }
        }]))))
        .expect("a nested non-empty links list is accepted");
    }

    /// `FEEDER_AUDIT_DETAILS.System_id_valid` (`not system_id.is_empty`, RM
    /// common `org.openehr.rm.common.feeder_audit_details.adoc` §Invariants) on
    /// a FOLDER commit.
    #[test]
    fn folder_empty_feeder_system_id_is_refused() {
        let with_feeder = |system_id: &str| {
            let mut folder = folder_fixture();
            folder.as_object_mut().unwrap().insert(
                "feeder_audit".into(),
                json!({
                    "_type": "FEEDER_AUDIT",
                    "originating_system_audit": {
                        "_type": "FEEDER_AUDIT_DETAILS",
                        "system_id": system_id
                    }
                }),
            );
            folder
        };
        let err = validate_folder(&with_feeder(""))
            .expect_err("an empty feeder-audit system_id must be refused");
        assert!(
            format!("{err:?}").contains("System_id_valid"),
            "the refusal should name the invariant, got {err:?}"
        );
        validate_folder(&with_feeder("legacy-lab-1"))
            .expect("a populated feeder-audit system_id is accepted");
    }
}
