// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

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

#![expect(
    clippy::disallowed_types,
    reason = "owner-approved 2026-08-03 (#1694): the commit interior carries the canonical \
              fragment the seam produced once; stored-content serving"
)]

use crate::service::status::CallStatusType;
use openehr_base::validate::InvariantViolation;
use serde_json::Value;
use sqlx::PgConnection;

use crate::ids::{EhrId, VoId};
use crate::service::FerroEhrService;
use crate::service::ehr::category::code;
use crate::service::error::{ServiceError, Violation};
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
        // The pre-check reads the EHR's live COMPOSITION ids in one SELECT and
        // reassembles each to read its category + declared template, so cost is
        // linear in the EHR's live COMPOSITION count. It is paid ONLY on a
        // create whose body is persistent (`431|persistent|`) AND declares a
        // template — every event-composition create returns at the two guards
        // above without touching storage.
        // NOTE: openEHR defines no such cardinality — the criterion is the CNF
        // schedule's, still "under debate in the openEHR SEC"
        // (`CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`).
        let vo_ids = crate::storage::version_repo::meta::current_vo_ids(
            &self.pool,
            ehr_id,
            "COMPOSITION",
            Some(lifecycle::state::DELETED),
        )
        .await?;
        for vo_id in vo_ids {
            if let Some(read) = read_current(&self.pool, self.spec_profile, vo_id).await?
                && is_persistent(&read.canonical)
                && composition_template_id(&read.canonical) == Some(template_id)
            {
                // NOTE: SM ehr_call_status_type.adoc declares
                // composition_already_exists; this refusal is precisely a
                // COMPOSITION-exists conflict.
                return Err(ServiceError::sm(
                    CallStatusType::CompositionAlreadyExists,
                    format!(
                        "EHR {ehr_id} already has a persistent COMPOSITION for template \
                         {template_id}; only one create is allowed (subsequent commits \
                         must be modifications)"
                    ),
                ));
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
    /// `incomplete` (a `553|incomplete|` commit, RM common master06
    /// §Incomplete Content) relaxes the existence & cardinality **lower**
    /// limits to zero on BOTH template-driven and RM-driven layers — the
    /// archetype-conformance pass and the RM mandatory-presence layers alike
    /// ("mandatory attributes may be absent … even though they may have
    /// minimum existence and cardinality respectively of one"). Every other
    /// check — class invariants, types, terminology, patterns, coded values —
    /// stays at full strictness ("data may be missing, but it may not be
    /// wrong").
    ///
    /// The template lookup goes through `web_template_for`; the
    /// template-independent passes run through
    /// `openehr_its::rm_instance::validate_rm_and_terminology` and the
    /// archetype-conformance pass through
    /// `openehr_its::flat::validation::validate_archetype_conformance*`.
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
        let mut messages = if incomplete {
            openehr_its::rm_instance::validate_rm_and_terminology_incomplete_as(
                composition,
                "COMPOSITION",
            )
        } else {
            openehr_its::rm_instance::validate_rm_and_terminology(composition)
        };
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
            // resolve against the routed terminology servers — a no-op, free of
            // any remote call, unless `[terminology.external]` is configured
            // (BASE `architecture_overview/master12-terminology.adoc`
            // §"Binding Terminology Value-sets to Archetypes"). The relaxation
            // for a `553|incomplete|` commit does not reach it: a code that IS
            // present may not be wrong (RM common master06 §Incomplete
            // Content). Counted as its own pass.
            let bindings = self.constraint_binding_violations(composition, &wt).await;
            binding_failures = bindings.len();
            messages.extend(bindings);
        }
        if rm_terminology_failures > 0 {
            crate::telemetry::metrics::metrics()
                .validation_failures
                .add(
                    rm_terminology_failures as u64,
                    &[opentelemetry::KeyValue::new("pass", "rm_terminology")],
                );
        }
        if template_failures > 0 {
            crate::telemetry::metrics::metrics()
                .validation_failures
                .add(
                    template_failures as u64,
                    &[opentelemetry::KeyValue::new("pass", "template")],
                );
        }
        if binding_failures > 0 {
            crate::telemetry::metrics::metrics()
                .validation_failures
                .add(
                    binding_failures as u64,
                    &[opentelemetry::KeyValue::new("pass", "constraint_binding")],
                );
        }
        if messages.is_empty() {
            return Ok(());
        }
        let errors = messages
            .into_iter()
            .map(|m| InvariantViolation::at(m.path, m.message))
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
            Kind::EhrStatus => validate_ehr_status(data, incomplete),
            Kind::EhrAccess => validate_ehr_access(data, incomplete),
            Kind::Folder => validate_folder(data, incomplete),
            // The demographic kinds arrive here from the raw-body
            // CONTRIBUTION lane only (the `Kind` was derived from the
            // payload's own `_type`), so they take the full check — decode
            // included; the direct routes enter at `party_invariants`, having
            // decoded already.
            Kind::Agent | Kind::Group | Kind::Organisation | Kind::Person | Kind::Role => {
                crate::service::demographic::validate::party_check(kind.as_str(), data, incomplete)
            }
            Kind::PartyRelationship => {
                crate::service::demographic::validate::relationship_check(data, incomplete)
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
    incomplete: bool,
) -> Result<(), ServiceError> {
    let messages = if incomplete {
        openehr_its::rm_instance::validate_rm_and_terminology_incomplete_as(data, declared)
    } else {
        openehr_its::rm_instance::validate_rm_and_terminology_as(data, declared)
    };
    if messages.is_empty() {
        return Ok(());
    }
    crate::telemetry::metrics::metrics()
        .validation_failures
        .add(
            u64::try_from(messages.len()).unwrap_or(u64::MAX),
            &[opentelemetry::KeyValue::new("pass", "rm_terminology")],
        );
    Err(ServiceError::ValidationFailed(
        messages
            .into_iter()
            .map(|m| InvariantViolation::at(m.path, m.message))
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
    // NOTE: no openEHR spec governs the `spec_profile` gate — our own
    // design/extension: this write-path read compares two root fields and
    // serves nothing, so it stays ungated.
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
        return Err(ServiceError::content_invalid(
            Violation::new(format!(
                "{incoming:?} differs from the versioned object's first version {stored:?}"
            ))
            .with_path("COMPOSITION.archetype_node_id")
            .with_invariant("VERSIONED_COMPOSITION.Archetype_node_id_valid"),
        ));
    }
    let incoming_category = canonical
        .pointer("/category/defining_code/code_string")
        .and_then(Value::as_str);
    if let (Some(stored), Some(incoming)) = (first_category.as_deref(), incoming_category)
        && (stored == code::PERSISTENT) != (incoming == code::PERSISTENT)
    {
        return Err(ServiceError::content_invalid(
            Violation::new(format!(
                "{incoming} changes the persistence of the versioned object \
                 (first version: {stored}) — is_persistent is fixed across versions"
            ))
            .with_path("COMPOSITION.category")
            .with_invariant("VERSIONED_COMPOSITION.Persistent_validity"),
        ));
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
        == Some(code::PERSISTENT)
}

/// The ROOT-ONLY half of `LOCATABLE.Archetyped_valid` for the always-root EHR
/// kinds (`EHR_STATUS`, `EHR_ACCESS` — RM ehr `ehr_status.adoc` /
/// `ehr_access.adoc`, each with an unconditional `Is_archetype_root`):
/// `is_archetype_root xor archetype_details = Void` (RM common
/// `locatable.adoc` §Invariants) means such a root MUST carry the `ARCHETYPED`
/// block, with its mandatory `archetype_id`.
///
/// That direction is the one a per-node pass cannot express: only this chapter
/// knows that an `EHR_STATUS` or `EHR_ACCESS` *is* a root. The other direction
/// (a term-coded node must NOT carry `archetype_details`) and the root-identity
/// rule (`archetype_node_id` equals the stringified
/// `archetype_details.archetype_id`) are the whole-instance pass's
/// (`openehr_rm::v1_2::validate::check_archetyped_valid`), as is
/// `LOCATABLE.Links_valid`, which it applies to every node carrying `links`.
pub(in crate::service) fn validate_root_locatable(
    obj: &serde_json::Map<String, Value>,
    kind: &str,
) -> Result<(), ServiceError> {
    let details = obj
        .get("archetype_details")
        .filter(|v| v.is_object())
        .ok_or_else(|| {
            ServiceError::content_invalid(
                Violation::new(format!(
                    "is mandatory: {kind} is an archetype root (Is_archetype_root), \
                     and a root without ARCHETYPED is invalid"
                ))
                .with_path(format!("{kind}.archetype_details"))
                .with_invariant("LOCATABLE.Archetyped_valid"),
            )
        })?;
    details
        .get("archetype_id")
        .and_then(|a| a.get("value"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ServiceError::content_invalid(
                Violation::new("is mandatory")
                    .with_path(format!("{kind}.archetype_details.archetype_id.value"))
                    .with_invariant("ARCHETYPED.archetype_id 1..1"),
            )
        })?;
    Ok(())
}

/// Validate an `EHR_STATUS` before it is committed (on EHR create,
/// `EHR_STATUS` update, or a CONTRIBUTION). Rejects every malformed data set
/// the CNF `master06 §Test Data Sets` (INVALID class 2) enumerates with a
/// `422`.
///
/// Only the two rules that are properties of the SLOT rather than of the
/// instance live here:
///
/// - the container holds an `EHR_STATUS`, so a foreign or absent `_type` is
///   invalid (nothing inside the instance can know which resource it was
///   posted to);
/// - the ROOT half of `LOCATABLE.Archetyped_valid`
///   ([`validate_root_locatable`]).
///
/// Everything else RM ehr `ehr_status.adoc` and the inherited `LOCATABLE`
/// demand is enforced by the whole-instance pass
/// ([`validate_rm_invariants_for_commit`]) from the generated model, not
/// restated here: `name` / `is_queryable` / `is_modifiable` / `subject`
/// mandatoriness and typing (`EHR_STATUS.subject` is monomorphic `PARTY_SELF`,
/// and an empty `{}` still decodes to the valid **anonymous** subject of RM ehr
/// master04 §EHR Status), the `ITEM_STRUCTURE` typing of `other_details`, the
/// `OBJECT_REF.Id_exists` / `Namespace_valid` rules on a present
/// `subject.external_ref`, `Archetype_node_id_valid`, `Links_valid`, and every
/// invariant below the root.
///
/// `incomplete` (a `553|incomplete|` commit) relaxes the whole-instance
/// pass's existence and cardinality lower bounds, exactly as for every other
/// committable kind — RM common master06 §Incomplete Content defines the
/// relaxation generically, with no content-type exclusion in any released
/// text. The two slot rules stay unconditional: the incomplete state lifts
/// lower bounds, never typing ("All other validity requirements must be
/// satisfied").
///
/// # Errors
/// [`ServiceError::Unprocessable`] for the two slot rules above, or
/// [`ServiceError::ValidationFailed`] carrying the RM-invariant violations
/// (both → 422).
pub(in crate::service) fn validate_ehr_status(
    status: &Value,
    incomplete: bool,
) -> Result<(), ServiceError> {
    let unproc = |m: String| ServiceError::content_invalid(Violation::new(m));
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

    validate_root_locatable(obj, "EHR_STATUS")?;
    validate_rm_invariants_for_commit(status, "EHR_STATUS", incomplete)
}

/// Validate a client-supplied `EHR_ACCESS` before it is committed (via a
/// CONTRIBUTION — there is no direct ITS-REST `EHR_ACCESS` write). RM ehr
/// `ehr_access.adoc`:
///
/// - a foreign `_type` in this slot is invalid (the container holds
///   `EHR_ACCESS` only) — a slot rule, not an instance one;
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
pub(in crate::service) fn validate_ehr_access(
    access: &Value,
    incomplete: bool,
) -> Result<(), ServiceError> {
    let unproc = |m: String| ServiceError::content_invalid(Violation::new(m));
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
    // `EHR_ACCESS.settings` is the RM's one implementation-defined slot —
    // "Instance is a subtype of the type `ACCESS_CONTROL_SETTINGS`, allowing for
    // the use of different access control schemes"
    // (`RM/docs/UML/classes/org.openehr.rm.ehr.ehr_access.adoc` §Attributes) —
    // and that type is abstract with no attributes, no invariants and no
    // RM-defined descendant, so the RM defines NOTHING inside it to judge while
    // a pass that walked in would refuse every legal instance. `Scheme_valid`
    // above is the whole of the RM's demand on the slot.
    // NOTE: `settings` is therefore excluded from the whole-instance RM pass,
    // and only from it — the one RM-mandated OPEN slot.
    let mut without_settings = access.clone();
    if let Some(map) = without_settings.as_object_mut() {
        map.remove("settings");
    }
    validate_rm_invariants_for_commit(&without_settings, "EHR_ACCESS", incomplete)
}

/// Validate a client-supplied FOLDER tree before it is committed (directory
/// create/update and the CONTRIBUTION FOLDER path): the whole-instance RM
/// class-invariant + terminology pass ([`validate_rm_invariants_for_commit`]),
/// which now carries EVERY rule this function once restated by hand:
///
/// - the declared-slot-type conformance rule (root + every member), from the
///   generated RM model — `FOLDER.items` → `OBJECT_REF`, `FOLDER.folders` →
///   `FOLDER`, so a COMPOSITION committed by value into `items` is refused
///   whatever the lifecycle state ("Folder structures do not contain
///   Compositions, only references to them", RM ehr master04 §Folders; RM
///   common master06 §Incomplete Content — "data may be missing, but it may
///   not be wrong");
/// - the PRESENCE layer (`OBJECT_REF`'s mandatory `id`/`namespace`/`type`,
///   `LOCATABLE.name`, `Archetype_node_id_valid`, `Links_valid`, the
///   archetype-root identity rule), which relaxes exactly on a
///   `553|incomplete|` commit — so no lifecycle special-casing is needed.
///   `archetype_details` stays OPTIONAL on a FOLDER (the RM types it 0..1 and
///   FOLDER carries no `Is_archetype_root` invariant).
///
/// # Errors
/// [`ServiceError::ValidationFailed`] carrying every violation found in the
/// tree, each at its own path (→ 422).
pub(in crate::service) fn validate_folder(
    folder: &Value,
    incomplete: bool,
) -> Result<(), ServiceError> {
    validate_rm_invariants_for_commit(folder, "FOLDER", incomplete)
}

/// A `FOLDER.items` `OBJECT_REF` whose `namespace` claims THIS system,
/// collected by [`collect_local_item_refs`] for target resolution.
struct LocalItemRef {
    path: String,
    namespace: String,
    id_value: String,
}

/// Collect every `items` reference in the FOLDER tree whose `namespace` is
/// `local` or this server's own system id, with its RM instance path.
fn collect_local_item_refs(folder: &Value, system_id: &str, at: &str, out: &mut Vec<LocalItemRef>) {
    if let Some(items) = folder.get("items").and_then(Value::as_array) {
        for (i, item) in items.iter().enumerate() {
            let namespace = item
                .get("namespace")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if namespace != "local" && namespace != system_id {
                continue;
            }
            out.push(LocalItemRef {
                path: format!("{at}/items[{i}]"),
                namespace: namespace.to_owned(),
                id_value: item
                    .get("id")
                    .and_then(|id| id.get("value"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
        }
    }
    if let Some(folders) = folder.get("folders").and_then(Value::as_array) {
        for (i, sub) in folders.iter().enumerate() {
            collect_local_item_refs(sub, system_id, &format!("{at}/folders[{i}]"), out);
        }
    }
}

/// The versioned-object root of an `OBJECT_REF.id` value — the uid before any
/// `::` qualification — when it is a uid this store can hold.
fn item_ref_root(id_value: &str) -> Option<uuid::Uuid> {
    let root = id_value.split("::").next().unwrap_or(id_value);
    // NOTE: `Result → Option` is the decision here (reliability.md class): a
    // non-UUID root is a legitimately unresolvable id, reported by the caller
    // as an unresolvable reference — not a swallowed defect.
    uuid::Uuid::parse_str(root).ok()
}

/// Refuse a FOLDER commit whose `items` contain a reference that CLAIMS this
/// system but does not resolve to a versioned object in `ehr_id`.
///
/// Applies to every lifecycle state — an unresolvable local reference is
/// *wrong* data, not *missing* data, so the `553|incomplete|` relaxation (RM
/// common master06 §Incomplete Content: "data may be missing, but it may not
/// be wrong") never reaches it. The reference's `type` facet is deliberately
/// not judged: BASE admits any ancestor name and `ANY` there.
///
/// # Errors
/// [`ServiceError::ValidationFailed`] naming every unresolvable local
/// reference at its tree path (→ 422).
pub(in crate::service) async fn check_folder_item_refs<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    ehr_id: EhrId,
    system_id: &str,
    folder: &Value,
) -> Result<(), ServiceError> {
    // NOTE: no released openEHR text constrains FOLDER.items resolvability
    // (BASE object_ref.adoc; no ITS-REST directory 422) — own-design safety
    // net, register AMB-211: refs claiming this system must resolve.
    let mut refs = Vec::new();
    collect_local_item_refs(folder, system_id, "", &mut refs);
    if refs.is_empty() {
        return Ok(());
    }
    let roots: Vec<uuid::Uuid> = refs
        .iter()
        .filter_map(|item| item_ref_root(&item.id_value))
        .collect();
    let known: std::collections::HashSet<uuid::Uuid> =
        crate::storage::ehr_repo::existing_vo_roots(executor, ehr_id, &roots)
            .await?
            .into_iter()
            .collect();
    let violations: Vec<InvariantViolation> = refs
        .iter()
        .filter(|item| !item_ref_root(&item.id_value).is_some_and(|root| known.contains(&root)))
        .map(|item| {
            InvariantViolation::at(
                &item.path,
                format!(
                    "is an OBJECT_REF claiming this system (namespace {:?}) whose target \
                     {:?} does not resolve to a versioned object in EHR {ehr_id}; a \
                     reference into this system must resolve — foreign-namespace \
                     references are accepted unchecked",
                    item.namespace, item.id_value
                ),
            )
        })
        .collect();
    if violations.is_empty() {
        return Ok(());
    }
    Err(ServiceError::ValidationFailed(violations))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{
        collect_local_item_refs, item_ref_root, validate_ehr_access, validate_ehr_status,
        validate_folder,
    };
    use crate::service::ehr::access::initial_ehr_access;
    use crate::service::ehr::service::initial_ehr_status;
    use crate::service::error::ServiceError;

    /// `EHR_STATUS.other_details` must be a concrete `ITEM_STRUCTURE`
    /// (RM ehr `ehr_status.adoc`): the four concrete subtypes pass, a foreign
    /// or missing `_type` rejects.
    #[test]
    fn ehr_status_other_details_type_is_enforced() {
        let with_other = |other: Value| {
            let mut st = initial_ehr_status();
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
            validate_ehr_status(&with_other(other), false)
                .unwrap_or_else(|e| panic!("{ty} other_details must be accepted: {e:?}"));
        }
        for bad in [
            json!({ "_type": "DV_TEXT", "value": "x" }),
            json!({ "value": "x" }),
        ] {
            let err = validate_ehr_status(&with_other(bad), false)
                .expect_err("non-ITEM_STRUCTURE other_details must be rejected");
            assert!(format!("{err:?}").contains("ITEM_STRUCTURE"), "got {err:?}");
        }
    }

    #[test]
    fn default_and_typical_ehr_status_are_accepted() {
        validate_ehr_status(&initial_ehr_status(), false).expect("default EHR_STATUS");
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
        validate_ehr_status(&identified, false).expect("identified PARTY_SELF EHR_STATUS");
    }

    /// The `553|incomplete|` relaxation reaches `EHR_STATUS` like every other
    /// committable kind: RM common master06 §Incomplete Content lifts the
    /// existence lower bounds ("mandatory attributes may be absent") with no
    /// content-type exclusion in any released text, so a status missing its
    /// mandatory `subject` (1..1) is refused complete and accepted incomplete.
    #[test]
    fn ehr_status_incomplete_lifts_mandatory_presence() {
        let mut status = initial_ehr_status();
        status.as_object_mut().unwrap().remove("subject");
        let err = validate_ehr_status(&status, false)
            .expect_err("a complete EHR_STATUS without its mandatory subject is refused");
        assert!(
            format!("{err:?}").contains("subject"),
            "the refusal names the absent mandatory attribute, got {err:?}"
        );
        validate_ehr_status(&status, true)
            .expect("an incomplete EHR_STATUS may omit mandatory attributes");
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
        let err = validate_ehr_status(&bad, false)
            .expect_err("PARTY_IDENTIFIED subject must be rejected");
        let msg = format!("{err:?}");
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
            validate_ehr_status(&status, false).expect("anonymous PARTY_SELF EHR_STATUS");
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
                validate_ehr_status(&status, false).is_err(),
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
        validate_ehr_access(&initial_ehr_access(), false).expect("the default EHR_ACCESS is valid");
        let err = validate_ehr_access(&json!({ "_type": "EHR_STATUS" }), false)
            .expect_err("foreign _type rejected");
        assert!(format!("{err:?}").contains("EHR_ACCESS"), "got {err:?}");
        let err = validate_ehr_access(&json!({
                   "_type": "EHR_ACCESS", "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID",
                              "value": "openEHR-EHR-EHR_ACCESS.generic.v1" },
            "rm_version": "1.2.0"
        },
               }), false)
        .expect_err("missing name rejected");
        assert!(format!("{err:?}").contains("name"), "got {err:?}");
        let err = validate_ehr_access(
            &json!({
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
            }),
            false,
        )
        .expect_err("settings without a concrete _type rejected (Scheme_valid)");
        assert!(format!("{err:?}").contains("Scheme_valid"), "got {err:?}");
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
        validate_folder(&good, false).expect("a ref-holding folder tree is valid");

        // A COMPOSITION by value inside items is rejected.
        let mut bad = good.clone();
        bad["items"][0] = json!({
            "_type": "COMPOSITION",
            "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
            "name": { "_type": "DV_TEXT", "value": "inline!" }
        });
        let err = validate_folder(&bad, false).expect_err("content by value must be rejected");
        assert!(format!("{err:?}").contains("OBJECT_REF"), "got {err:?}");

        // A sub-folder without a name violates LOCATABLE.name 1..1.
        let mut bad = good;
        bad["folders"][0].as_object_mut().unwrap().remove("name");
        let err = validate_folder(&bad, false).expect_err("nameless sub-folder rejected");
        assert!(format!("{err:?}").contains("name"), "got {err:?}");
    }

    /// The `FOLDER.items` PRESENCE duty is owned by the whole-instance RM pass
    /// (`OBJECT_REF`'s mandatory `id`/`namespace`/`type`), which relaxes exactly
    /// that layer on a `553|incomplete|` commit (RM common master06 §Incomplete
    /// Content) — so `validate_folder` needs no lifecycle special-casing of its
    /// own.
    #[test]
    fn folder_item_presence_is_owned_by_the_rm_pass() {
        let missing_id = json!({
            "_type": "FOLDER",
            "name": { "_type": "DV_TEXT", "value": "root" },
            "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
            "items": [{
                "_type": "OBJECT_REF", "namespace": "local", "type": "VERSIONED_COMPOSITION"
            }]
        });
        let err = validate_folder(&missing_id, false)
            .expect_err("an OBJECT_REF without its mandatory id is refused");
        assert!(
            matches!(err, ServiceError::ValidationFailed(_)),
            "the refusal comes from the RM pass, got {err:?}"
        );
        validate_folder(&missing_id, true)
            .expect("a 553|incomplete| commit may leave mandatory data missing");
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
        validate_folder(&folder_fixture(), false).expect("the baseline folder tree is valid");

        let mut bad = folder_fixture();
        bad["folders"][0]
            .as_object_mut()
            .unwrap()
            .insert("links".into(), json!([]));
        let err = validate_folder(&bad, false).expect_err("an empty links list must be rejected");
        assert!(
            format!("{err:?}").contains("links")
                && format!("{err:?}").contains("at least one member"),
            "got {err:?}"
        );

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
        validate_folder(&good, false).expect("a non-empty links list is valid");
    }

    /// RM common `org.openehr.rm.common.locatable.adoc` §Attributes
    /// (`archetype_node_id`): at an archetype root the node id is the
    /// stringified `archetype_details.archetype_id`. A FOLDER without
    /// `archetype_details` stays valid — the attribute is 0..1.
    #[test]
    fn folder_archetype_root_node_id_must_match_details() {
        let mut bad = folder_fixture();
        bad["archetype_details"]["archetype_id"]["value"] = json!("openEHR-EHR-FOLDER.other.v1");
        let err =
            validate_folder(&bad, false).expect_err("a contradicting root identity is rejected");
        assert!(
            format!("{err:?}").contains("LOCATABLE.archetype_node_id"),
            "got {err:?}"
        );

        let mut without = folder_fixture();
        without.as_object_mut().unwrap().remove("archetype_details");
        validate_folder(&without, false).expect("archetype_details stays optional on a FOLDER");
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
        let mut bad = initial_ehr_status();
        bad["archetype_details"]["rm_version"] = json!("");
        let err =
            validate_ehr_status(&bad, false).expect_err("an empty rm_version must be refused");
        assert!(
            format!("{err:?}").contains("Rm_version_valid"),
            "the refusal should name the invariant, got {err:?}"
        );
        validate_ehr_status(&initial_ehr_status(), false)
            .expect("a populated rm_version is accepted");
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
        let err = validate_folder(
            &with_link(json!({
                "_type": "LINK",
                "type": { "_type": "DV_TEXT", "value": "issue" },
                "target": { "_type": "DV_EHR_URI", "value": "ehr://example.org/x" }
            })),
            false,
        )
        .expect_err("a LINK without its mandatory meaning must be refused");
        assert!(
            format!("{err:?}").contains("meaning"),
            "the refusal should name the missing attribute, got {err:?}"
        );

        validate_folder(
            &with_link(json!({
                "_type": "LINK",
                "meaning": { "_type": "DV_TEXT", "value": "follow up" },
                "type": { "_type": "DV_TEXT", "value": "issue" },
                "target": { "_type": "DV_EHR_URI", "value": "ehr://example.org/x" }
            })),
            false,
        )
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
            let mut status = initial_ehr_status();
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

        let err = validate_ehr_status(&with_other(item_single(None)), false)
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
        validate_ehr_status(
            &with_other(item_single(Some(json!({
                "_type": "ELEMENT",
                "name": { "_type": "DV_TEXT", "value": "e" },
                "archetype_node_id": "at0002",
                "value": { "_type": "DV_TEXT", "value": "v" }
            })))),
            false,
        )
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
            let mut status = initial_ehr_status();
            status
                .as_object_mut()
                .unwrap()
                .insert("other_details".into(), other);
            status
        };
        let err = validate_ehr_status(&with_other(other_details(json!([]))), false)
            .expect_err("a nested present-but-empty links list must be refused");
        assert!(
            format!("{err:?}").contains("links")
                && format!("{err:?}").contains("at least one member"),
            "the refusal names the empty container (#1730 parse class), got {err:?}"
        );

        validate_ehr_status(
            &with_other(other_details(json!([{
                "_type": "LINK",
                "meaning": { "_type": "DV_TEXT", "value": "follow up" },
                "type": { "_type": "DV_TEXT", "value": "issue" },
                "target": { "_type": "DV_EHR_URI", "value": "ehr://example.org/x" }
            }]))),
            false,
        )
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
        let err = validate_folder(&with_feeder(""), false)
            .expect_err("an empty feeder-audit system_id must be refused");
        assert!(
            format!("{err:?}").contains("System_id_valid"),
            "the refusal should name the invariant, got {err:?}"
        );
        validate_folder(&with_feeder("legacy-lab-1"), false)
            .expect("a populated feeder-audit system_id is accepted");
    }

    /// The `items` walker collects exactly the THIS-system namespaces
    /// (`local` + the configured system id), skips foreign and `unknown`
    /// ones, and recurses with `/folders[i]/items[j]` paths.
    #[test]
    fn local_item_ref_walker_classifies_namespaces_and_paths() {
        let item = |namespace: &str, id: &str| {
            json!({
                "_type": "OBJECT_REF",
                "namespace": namespace,
                "type": "VERSIONED_COMPOSITION",
                "id": { "_type": "HIER_OBJECT_ID", "value": id }
            })
        };
        let folder = json!({
            "_type": "FOLDER",
            "name": { "_type": "DV_TEXT", "value": "root" },
            "items": [
                item("local", "a"),
                item("my.system.id", "b"),
                item("unknown", "c"),
                item("this.system", "d"),
            ],
            "folders": [{
                "_type": "FOLDER",
                "name": { "_type": "DV_TEXT", "value": "sub" },
                "items": [ item("local", "e") ]
            }]
        });
        let mut out = Vec::new();
        collect_local_item_refs(&folder, "this.system", "", &mut out);
        let collected: Vec<(&str, &str)> = out
            .iter()
            .map(|r| (r.path.as_str(), r.id_value.as_str()))
            .collect();
        assert_eq!(
            collected,
            [
                ("/items[0]", "a"),
                ("/items[3]", "d"),
                ("/folders[0]/items[0]", "e"),
            ]
        );
    }

    /// `item_ref_root` takes the uid before any `::` qualification and only
    /// accepts a uid this store can hold.
    #[test]
    fn item_ref_root_extracts_the_versioned_object_root() {
        let root = "0192b1c2-aaaa-7bbb-8ccc-0123456789ab";
        let versioned = format!("{root}::some.system::2");
        assert_eq!(
            item_ref_root(&versioned)
                .expect("qualified uid resolves")
                .to_string(),
            root
        );
        assert_eq!(
            item_ref_root(root).expect("bare uid resolves").to_string(),
            root
        );
        assert_eq!(item_ref_root("not-a-uuid"), None);
        assert_eq!(item_ref_root(""), None);
    }
}
