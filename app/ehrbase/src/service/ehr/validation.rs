//! The commit-validation choke point for every EHR-owned kind — the
//! structural validators for `EHR_STATUS` / `EHR_ACCESS` / FOLDER, the
//! COMPOSITION RM + terminology + template validation, the
//! `validate_for_commit` dispatch shared by the direct and CONTRIBUTION write
//! paths, and the `VERSIONED_COMPOSITION` cross-version invariants.
//!
//! Spec: RM ehr `ehr_status.adoc` / `ehr_access.adoc` /
//! `versioned_composition.adoc`; RM common `folder.adoc` + inherited
//! `locatable.adoc`; RM common master06 §Incomplete Content (the
//! `553|incomplete|` relaxation); ITS-REST `responses/422_COMPOSITION.yaml`;
//! CNF `master06 §Test Data Sets` (INVALID class 2) +
//! `master07-func_tc_ehr_composition.adoc` (the persistent-cardinality
//! convention).

use serde_json::Value;
use sqlx::PgConnection;

use crate::ids::{EhrId, VoId};
use crate::service::EhrbaseService;
use crate::service::error::ServiceError;
use crate::versioning::read::read_current;
use crate::versioning::{Kind, lifecycle};

impl EhrbaseService {
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
        // TODO(perf): scans the EHR's live COMPOSITIONs and reassembles each to
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
    /// through `openehr_flat::validation::validate_*`.
    ///
    /// # Errors
    /// [`ServiceError::ValidationFailed`] carrying every RM/terminology/
    /// template violation (→ 422); [`ServiceError`] from a failing template
    /// resolution.
    pub(super) async fn validate_composition_for_commit(
        &self,
        composition: &Value,
        incomplete: bool,
    ) -> Result<(), ServiceError> {
        let mut messages = openehr_flat::validation::validate_rm_and_terminology(composition);
        let rm_terminology_failures = messages.len();
        if let Some(template_id) = composition
            .pointer("/archetype_details/template_id/value")
            .and_then(Value::as_str)
        {
            let wt = self.web_template_for(template_id).await?;
            messages.extend(if incomplete {
                openehr_flat::validation::validate_archetype_conformance_incomplete(
                    composition,
                    &wt,
                )
            } else {
                openehr_flat::validation::validate_archetype_conformance(composition, &wt)
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
/// # Errors
/// [`ServiceError::Unprocessable`] naming the first violated rule (→ 422).
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
    Ok(())
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
/// # Errors
/// [`ServiceError::Unprocessable`] naming the first violated rule (→ 422).
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
    Ok(())
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
/// - `folders` members recurse.
///
/// # Errors
/// [`ServiceError::Unprocessable`] naming the first violated rule and the
/// offending tree path (→ 422).
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
        if obj
            .get("archetype_node_id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(unproc(format!(
                "{path}: FOLDER.archetype_node_id is mandatory and non-empty \
                 (LOCATABLE.Archetype_node_id_valid)"
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
    walk(folder, "")
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::{Value, json};

    use super::{validate_ehr_access, validate_ehr_status, validate_folder};
    use crate::service::ehr::access::default_ehr_access;
    use crate::service::ehr::service::default_ehr_status;

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
        for ty in ["ITEM_TREE", "ITEM_LIST", "ITEM_SINGLE", "ITEM_TABLE"] {
            validate_ehr_status(&with_other(json!({ "_type": ty, "name": { "_type": "DV_TEXT", "value": "d" }, "archetype_node_id": "at0001" })))
                .unwrap_or_else(|e| panic!("{ty} other_details must be accepted: {e}"));
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
                "name": { "_type": "DV_TEXT", "value": "EHR Status" },
                "subject": subject,
                "is_queryable": true,
                "is_modifiable": true
            });
            validate_ehr_status(&status).expect("anonymous PARTY_SELF EHR_STATUS");
        }
    }

    /// Every vendored `EHR_STATUS` data set the CNF corpus labels invalid
    /// (`master06 §Test Data Sets`, INVALID class 2) must be rejected — with
    /// one spec-cited exception: `001_ehr_status_subject_empty.json`
    /// (`subject: {}`) is spec-VALID (an empty `PARTY_SELF` is a completely
    /// anonymous subject, master04), a documented corpus-vs-spec adjudication.
    #[test]
    fn every_invalid_ehr_status_fixture_is_rejected() {
        const SPEC_VALID_ANONYMOUS: &str = "001_ehr_status_subject_empty.json";
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
            let is_anon = path.file_name().and_then(|n| n.to_str()) == Some(SPEC_VALID_ANONYMOUS);
            if is_anon {
                validate_ehr_status(&status).unwrap_or_else(|e| {
                    panic!(
                        "spec-valid anonymous EHR_STATUS ({SPEC_VALID_ANONYMOUS}) was rejected: {e}"
                    )
                });
            } else {
                assert!(
                    validate_ehr_status(&status).is_err(),
                    "invalid EHR_STATUS fixture was accepted: {}",
                    path.display()
                );
            }
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
            "_type": "EHR_ACCESS", "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1"
        }))
        .expect_err("missing name rejected");
        assert!(err.to_string().contains("name"), "got {err}");
        let err = validate_ehr_access(&json!({
            "_type": "EHR_ACCESS",
            "name": { "_type": "DV_TEXT", "value": "EHR Access" },
            "archetype_node_id": "openEHR-EHR-EHR_ACCESS.generic.v1",
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
}
