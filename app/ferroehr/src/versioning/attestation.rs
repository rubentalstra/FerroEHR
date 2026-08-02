//! Attestation: attaching an `ATTESTATION` to an `ORIGINAL_VERSION` at or after
//! committal.
//!
//! Spec: RM common `master06-change_control_package.adoc` §Attestation + RM
//! common `master04-generic_package.adoc` §Attestation. An `ATTESTATION` is an
//! `AUDIT_DETAILS` subtype (`items?`, `reason`, `proof`, `is_pending`); it "can
//! be added at any time after committal" and a `666|attestation|` member of a
//! CONTRIBUTION adds **no** new version. Attestations of an old version are not
//! valid for a new version (they are keyed to `(vo_id, sys_version)` and never
//! copied forward).
//!
//! NOTE (which attestations the `VERSION.signature` covers, master06 §Digital
//! Signature + §Attestation): the two arrival routes stand on opposite sides of
//! the signature.
//!
//! * Committed WITH the version (`UPDATE_VERSION.attestations` — SM
//!   `UML/classes/update_version.adoc` §Attributes; master06 §Attestation,
//!   "Signing content at committal"): these are attributes of the Version at the
//!   moment it is serialised, and §Digital Signature signs "the entire Version
//!   object (note that the signature attribute will be Void at this point)" —
//!   `signature` is the ONLY exclusion. They are therefore completed BEFORE the
//!   signature is computed ([`complete_accompanying`]) and stored with
//!   `at_committal = true`, so the bytes signed at commit are the bytes served
//!   at read.
//! * Added afterwards (the `666|attestation|` CONTRIBUTION member, [`attest`];
//!   §Attestation: "Attestations can be added at any time after committal of the
//!   content being attested"; §Contributions: "a new `ATTESTATION` is added to
//!   the attestations list of an **existing** `ORIGINAL_VERSION`"): these
//!   necessarily post-date the signature, so they are stored with
//!   `at_committal = false` and appended to the served version AFTER
//!   verification, never entering its canonical form.

use openehr_rm::prelude::{Attestation, DvEhrUri, DvMultimedia, DvText};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::ids::{EhrId, VoId};
use crate::service::error::ServiceError;
use crate::service::status::CallStatusType;
use crate::versioning::Kind;
use crate::versioning::audit::{
    change_type, change_type_rubric, decode_description, dv_date_time, dv_text, openehr_coded_text,
    party_proxy,
};
use crate::versioning::change::Committed;
use crate::versioning::object_version_id::TreeId;

/// A `666|attestation|` of an **existing** `ORIGINAL_VERSION` committed within a
/// CONTRIBUTION (master06 §Contributions — adds no new version). Carried
/// alongside the change set so it commits in the same transaction.
pub(crate) struct PendingAttest {
    pub(crate) vo_id: VoId,
    pub(crate) kind: Kind,
    /// The target version to attest (from `preceding_version_uid` — trunk or
    /// branch).
    pub(crate) expected: TreeId,
    /// The wire `UPDATE_ATTESTATION` partial, completed into a full RM
    /// `ATTESTATION` at commit time.
    pub(crate) partial: Value,
}

/// Attach an `ATTESTATION` to an **existing** `ORIGINAL_VERSION` (a
/// `666|attestation|` version item; master06 §Contributions — no new version,
/// `sys_period` untouched). Realizes `VERSIONED_OBJECT.commit_attestation`
/// precondition `has_version_id` (master06 §Versioned Objects). `attestation`
/// is the already-completed full RM `ATTESTATION`.
///
/// # Errors
/// [`ServiceError::NotFound`] when the target `(vo_id, tree, kind)` does not
/// exist or does not belong to `ehr_id`; the storage errors of the target
/// lookup / attestation insert.
#[expect(
    clippy::too_many_arguments,
    reason = "the parts of an attestation act plus its commit instant; a \
              parameter struct would not read clearer at the one call site"
)]
pub(crate) async fn attest(
    tx: &mut PgConnection,
    ehr_id: Option<EhrId>,
    vo_id: VoId,
    kind: Kind,
    expected: TreeId,
    attestation: &Value,
    contribution_id: Uuid,
    time_committed: jiff::Timestamp,
) -> Result<Committed, ServiceError> {
    // The target lookup (`version_repo::attestation::attestation_target`) yields the owning
    // EHR (compared against the caller's), the storage ordinal the attestation
    // keys to, and the target's `creating_system_id` (carried into the outbox).
    let target = crate::storage::version_repo::attestation::attestation_target(
        tx,
        vo_id,
        expected.columns(),
        kind.as_str(),
    )
    .await?;
    let Some(target) = target.filter(|t| t.ehr_id == ehr_id) else {
        return Err(ServiceError::sm(
            CallStatusType::ObjectVersionDoesNotExist,
            format!("{} version {vo_id}::{expected}", kind.as_str()),
        ));
    };
    crate::storage::version_repo::attestation::insert_attestation(
        tx,
        vo_id,
        target.sys_version,
        contribution_id,
        // AFTER committal by construction: this route attaches to a version
        // that already exists (master06 §Contributions, "an existing
        // `ORIGINAL_VERSION`"), so the attestation post-dates that version's
        // signature and stays outside its canonical form.
        false,
        attestation,
    )
    .await?;
    Ok(Committed {
        vo_id,
        sys_version: target.sys_version,
        tree: expected,
        creating_system_id: target.creating_system_id,
        kind,
        // A 666 attestation adds no new version; it is announced in the
        // contribution's outbox envelope as a change to the existing version.
        // The code lives on the ATTESTATION's OWN inherited `change_type`, not
        // on a `commit_audit` it does not have (register AMB-190 — the
        // §Contributions row's `ATTESTATION._commit_audit_._change_type_` path
        // names no attribute of the class).
        change_type: change_type::ATTESTATION.to_owned(),
        template_id: None,
        // The contribution's commit-act time — a 666 attestation adds no new
        // version, so this is the instant the attestation itself committed.
        time_committed,
    })
}

/// Complete the attestations committed together with a NEW version
/// (`UPDATE_VERSION.attestations`; master06 §Attestation "Signing content at
/// committal") into full canonical RM `ATTESTATION`s.
///
/// This runs BEFORE the version's signature is computed, and its output is both
/// what gets signed and what gets stored ([`insert_at_committal_attestations`])
/// — one completion, so the signed bytes and the served bytes cannot drift
/// (master06 §Digital Signature).
///
/// # Errors
/// The [`complete_attestation`] `Unprocessable` rejections.
pub(crate) fn complete_accompanying(
    partials: &[Value],
    system_id: &str,
    committer_fallback: &Value,
    now: jiff::Timestamp,
) -> Result<Vec<Value>, ServiceError> {
    partials
        .iter()
        .map(|partial| complete_attestation(partial, system_id, committer_fallback, now))
        .collect()
}

/// Persist the already-completed attestations of a just-written version, marked
/// `at_committal` — they are inside that version's signed canonical form
/// (master06 §Digital Signature; see the module docs).
///
/// # Errors
/// The storage error of the attestation insert.
pub(crate) async fn insert_at_committal_attestations(
    tx: &mut PgConnection,
    vo_id: VoId,
    sys_version: i32,
    contribution_id: Uuid,
    completed: &[Value],
) -> Result<(), ServiceError> {
    for full in completed {
        crate::storage::version_repo::attestation::insert_attestation(
            tx,
            vo_id,
            sys_version,
            contribution_id,
            true,
            full,
        )
        .await?;
    }
    Ok(())
}

/// The attributes `ATTESTATION` declares on top of the `AUDIT_DETAILS` it
/// inherits from — `attested_view`, `proof`, `items`, `reason`, `is_pending`
/// (RM common `UML/classes/org.openehr.rm.common.attestation.adoc`
/// §Attributes) — each decoded into the RM type its class table gives it.
///
/// This is the ONLY part of an attestation a client supplies: the inherited
/// `system_id` / `time_committed` / `change_type` are the server's (ITS-REST
/// overview `Requests_and_responses.md` §"openehr-version and
/// openehr-audit-details": "The `time_committed` attribute is always set by
/// the server"), and `committer` / `description` are shared with every audit.
#[derive(Debug, Clone)]
pub(crate) struct AttestationParts {
    /// `ATTESTATION.attested_view` (0..1).
    pub(crate) attested_view: Option<DvMultimedia>,
    /// `ATTESTATION.proof` (0..1).
    pub(crate) proof: Option<String>,
    /// `ATTESTATION.items` (0..1; empty ≙ absent — `Items_valid` forbids a
    /// present-but-empty list).
    pub(crate) items: Vec<DvEhrUri>,
    /// `ATTESTATION.reason` (1..1).
    pub(crate) reason: DvText,
    /// `ATTESTATION.is_pending` (1..1).
    pub(crate) is_pending: bool,
}

impl AttestationParts {
    /// Decode + invariant-check the `ATTESTATION`-declared attributes of a
    /// client-supplied attestation payload — a wire `UPDATE_ATTESTATION`, an
    /// RM `ATTESTATION` used as a version's `commit_audit`, or the canonical
    /// fragment [`Self::fragment`] stored for one (the three carry the same
    /// attribute names in the same encodings, so one decoder serves all).
    ///
    /// # Errors
    /// [`ServiceError::Unprocessable`] when an RM invariant fails:
    /// - `reason` absent (mandatory, 1..1), or a coded `reason` whose
    ///   `defining_code` is not in the openEHR `attestation reason` group
    ///   (`ATTESTATION.Reason_valid`);
    /// - `is_pending` absent or not a `Boolean` (mandatory, 1..1);
    /// - `items` present but not a non-empty list (`ATTESTATION.Items_valid`);
    /// - `reason` / `attested_view` / `proof` / `items` present but not
    ///   decodable as the RM type the class table gives them (`DV_TEXT`,
    ///   `DV_MULTIMEDIA`, `String`, `List<DV_EHR_URI>` — RM common
    ///   `UML/classes/org.openehr.rm.common.attestation.adoc` §Attributes).
    pub(crate) fn decode(partial: &Value) -> Result<Self, ServiceError> {
        // reason (1..1)
        let reason = partial.get("reason").ok_or_else(|| {
            ServiceError::Unprocessable("ATTESTATION.reason is required (1..1)".to_owned())
        })?;
        // Reason_valid: a coded reason's defining_code must be a member of the
        // openEHR `attestation reason` group.
        if reason.get("_type").and_then(Value::as_str) == Some("DV_CODED_TEXT") {
            let code = reason
                .pointer("/defining_code/code_string")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if !openehr_term::bundle::openehr().is_valid_attestation_reason(code) {
                return Err(ServiceError::Unprocessable(format!(
                    "ATTESTATION.reason.defining_code {code:?} is not in the openEHR \
                     `attestation reason` group (ATTESTATION.Reason_valid)"
                )));
            }
        }
        // is_pending (1..1, Boolean)
        let is_pending = partial
            .get("is_pending")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                ServiceError::Unprocessable(
                    "ATTESTATION.is_pending is required (1..1 Boolean)".to_owned(),
                )
            })?;
        // items (0..1); Items_valid: non-empty when present.
        //
        // NOTE: the spec's two statements of `items` SCOPE conflict, and this
        // decoder deliberately enforces the class-model one. RM common
        // master04-generic_package.adoc §Attestation requires every member to
        // resolve inside the attested object ("otherwise the list must contain
        // a set of paths to items within the item to which the attestation is
        // attached"), while the class table for the same attribute admits the
        // opposite ("Although not recommended, these may include fine-grained
        // items which have been attested in some other system" —
        // UML/classes/org.openehr.rm.common.attestation.adoc §Attributes), and
        // master04 itself later calls fine granularity unrecommended rather
        // than invalid ("there is nothing stopping it including fine-grained
        // items"). The class model is the machine-readable reading and the only
        // one either source gives a checkable predicate for, so we enforce
        // exactly that: `List<DV_EHR_URI>` typing plus `Items_valid`, and NO
        // containment check — a cross-system path is accepted verbatim.
        // Register entry AMB-180.
        //
        // A JSON `null` is the ABSENT encoding of an optional attribute, not a
        // present-but-invalid list — the same reading `attested_view` and
        // `proof` below already apply, and the one the native `UPDATE_VERSION`
        // DTO emits for `Option::None`. So it is filtered out before
        // `Items_valid` is evaluated; a present-but-EMPTY list (`[]`) still
        // fails, which is what the invariant actually forbids.
        let items = partial.get("items").filter(|v| !v.is_null());
        if let Some(items) = items
            && items.as_array().is_none_or(Vec::is_empty)
        {
            return Err(ServiceError::Unprocessable(
                "ATTESTATION.items must be a non-empty list when present \
                 (ATTESTATION.Items_valid)"
                    .to_owned(),
            ));
        }
        Ok(Self {
            attested_view: partial
                .get("attested_view")
                .filter(|v| !v.is_null())
                .map(|v| decode(v, "ATTESTATION.attested_view", "DV_MULTIMEDIA"))
                .transpose()?,
            // NOTE: `proof` is an OPAQUE CLIENT FACT — accepted as the `String`
            // its class table declares, stored and served byte-for-byte, never
            // parsed and never verified server-side. RM common
            // master04-generic_package.adoc §Attestation defines the openPGP
            // process over "the entire Attestation object (note that the proof
            // attribute will be Void at this point)" and then withdraws the
            // definition — "[.tbd] *To Be Determined*: The exact serialisation
            // is not yet defined by openEHR" — so there is no canonical form to
            // recompute against; and the inherited `AUDIT_DETAILS` attributes
            // are re-minted here at completion (`system_id`, `time_committed`,
            // `change_type` are the server's), so the stored object is not the
            // object the client signed in any case. This is NOT the VERSION
            // `signature` mechanism, which this server does generate and verify
            // for its own signatures (master06 §Digital Signature —
            // `crate::versioning::integrity`); there the server owns both the
            // canonicalisation and the key. Register entry AMB-181.
            proof: partial
                .get("proof")
                .filter(|v| !v.is_null())
                .map(|v| {
                    v.as_str().map(str::to_owned).ok_or_else(|| {
                        ServiceError::Unprocessable(
                            "ATTESTATION.proof must be a String (0..1)".to_owned(),
                        )
                    })
                })
                .transpose()?,
            items: match items {
                None => Vec::new(),
                Some(items) => items
                    .as_array()
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, v)| decode(v, &format!("ATTESTATION.items[{i}]"), "DV_EHR_URI"))
                    .collect::<Result<_, _>>()?,
            },
            reason: decode(reason, "ATTESTATION.reason", "DV_TEXT")?,
            is_pending,
        })
    }

    /// These attributes as a canonical JSON fragment — the storage form of a
    /// commit audit that is an `ATTESTATION` (`audit.attestation`), each
    /// attribute in the encoding `openehr-its` gives its RM type and absent
    /// optionals omitted. Round-trips through [`Self::decode`].
    pub(crate) fn fragment(&self) -> Value {
        let mut map = serde_json::Map::new();
        if let Some(attested_view) = &self.attested_view {
            map.insert(
                "attested_view".to_owned(),
                openehr_its::json::to_canonical_value(attested_view),
            );
        }
        if let Some(proof) = &self.proof {
            map.insert("proof".to_owned(), Value::String(proof.clone()));
        }
        if !self.items.is_empty() {
            map.insert(
                "items".to_owned(),
                Value::Array(
                    self.items
                        .iter()
                        .map(openehr_its::json::to_canonical_value)
                        .collect(),
                ),
            );
        }
        map.insert(
            "reason".to_owned(),
            openehr_its::json::to_canonical_value(&self.reason),
        );
        map.insert("is_pending".to_owned(), Value::Bool(self.is_pending));
        Value::Object(map)
    }
}

/// Complete a wire `UPDATE_ATTESTATION` partial into a full canonical RM
/// `ATTESTATION` (RM common master04 §Attestation; ITS-REST
/// `UpdateAttestation`). The server supplies the inherited `AUDIT_DETAILS`
/// fields it owns — `system_id`, `time_committed`, and the `666|attestation|`
/// `change_type` — exactly as `UPDATE_AUDIT` → `AUDIT_DETAILS` (master06
/// §Version Update Semantics), then adds the `ATTESTATION`-specific attributes.
/// `committer` comes from the partial when present, else the CONTRIBUTION's
/// committer (master06 §Committal).
///
/// The completed value is built as the generated `openehr-rm` [`Attestation`]
/// and serialized through the native codec, so it carries `_type` first and the
/// BMM's own attribute order — and so every attribute the client supplied is
/// decoded into its RM type rather than passed through unread.
///
/// # Errors
/// The [`AttestationParts::decode`] rejections, and
/// [`ServiceError::Unprocessable`] when `committer` is not a canonical
/// `PARTY_PROXY` or `description` is neither a string nor a canonical
/// `DV_TEXT`.
pub(crate) fn complete_attestation(
    partial: &Value,
    system_id: &str,
    committer_fallback: &Value,
    now: jiff::Timestamp,
) -> Result<Value, ServiceError> {
    let parts = AttestationParts::decode(partial)?;
    // committer: from the partial if present, else the CONTRIBUTION committer.
    let committer = partial
        .get("committer")
        .cloned()
        .unwrap_or_else(|| committer_fallback.clone());
    // description: the inherited AUDIT_DETAILS.description (0..1). ITS-REST
    // types it `UDvText` — `oneOf` [`DV_TEXT`, `DV_CODED_TEXT`]
    // (`specifications/schemas/data_types/UDvText.yaml`, an object, never a bare
    // string) — while SM `UPDATE_AUDIT.description` is `String [0..1]`
    // (`SM/docs/UML/classes/update_audit.adoc` §Attributes), which grounds the
    // plain-string branch. Both spellings are read, and the object spelling is
    // decoded WHOLE: `description` is typed `DV_TEXT` at 0..1 (RM common
    // `UML/classes/org.openehr.rm.common.audit_details.adoc` §Attributes,
    // inherited by `…org.openehr.rm.common.attestation.adoc`), and
    // `DV_CODED_TEXT` is a substitutable subtype — reducing it to its `value`
    // would drop the `defining_code` of a committed attestation permanently.
    let description = partial
        .get("description")
        .filter(|d| !d.is_null())
        .map(|d| match d {
            Value::String(s) => Ok(dv_text(s)),
            other => decode_description(other),
        })
        .transpose()?;
    // The inherited AUDIT_DETAILS fields are the server's, exactly as any
    // audit's; `openehr-its` writes the whole ATTESTATION in BMM order.
    Ok(openehr_its::json::to_canonical_value(&Attestation {
        system_id: system_id.to_owned(),
        time_committed: dv_date_time(&now),
        change_type: openehr_coded_text(
            change_type::ATTESTATION,
            change_type_rubric(change_type::ATTESTATION),
        ),
        description,
        committer: party_proxy(&committer)?,
        attested_view: parts.attested_view,
        proof: parts.proof,
        items: openehr_base::containers::present(parts.items),
        reason: parts.reason,
        is_pending: parts.is_pending,
    }))
}

/// Decode one client-supplied `ATTESTATION` attribute into its RM type,
/// reporting the attribute by name so a `422` says which field was wrong.
fn decode<T: serde::de::DeserializeOwned>(
    value: &Value,
    attribute: &str,
    rm_type: &str,
) -> Result<T, ServiceError> {
    openehr_its::json::from_canonical_value::<T>(value).map_err(|e| {
        ServiceError::Unprocessable(format!("{attribute} is not a valid {rm_type}: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// A committer the fallback path can use — `PARTY_IDENTIFIED` with a name
    /// satisfies `Basic_validity` + `Name_valid`.
    fn committer() -> Value {
        json!({ "_type": "PARTY_IDENTIFIED", "name": "Dr Jones" })
    }

    fn now() -> jiff::Timestamp {
        "2026-07-07T10:11:12Z"
            .parse()
            .expect("the literal is a valid RFC 3339 instant")
    }

    /// The completed `ATTESTATION` is the canonical serialization of the
    /// generated RM type: `_type` first, then the BMM's own attribute order
    /// (the inherited `AUDIT_DETAILS` attributes, then `attested_view`,
    /// `proof`, `items`, `reason`, `is_pending` — RM common
    /// `UML/classes/org.openehr.rm.common.attestation.adoc` §Attributes).
    #[test]
    fn completed_attestation_is_in_bmm_attribute_order() {
        let att = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "proof": "signed-by-hand",
                "items": [{ "_type": "DV_EHR_URI", "value": "ehr://x/y" }]
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect("a complete, well-typed UPDATE_ATTESTATION partial");
        let keys: Vec<&str> = att
            .as_object()
            .map(|m| m.keys().map(String::as_str).collect())
            .unwrap_or_default();
        assert_eq!(
            keys,
            vec![
                "_type",
                "system_id",
                "time_committed",
                "change_type",
                "committer",
                "proof",
                "items",
                "reason",
                "is_pending",
            ]
        );
        assert_eq!(
            att.get("_type").and_then(Value::as_str),
            Some("ATTESTATION")
        );
    }

    /// A `DV_CODED_TEXT` `description` survives completion whole. The
    /// inherited `AUDIT_DETAILS.description` is typed `DV_TEXT` at 0..1 (RM
    /// common `UML/classes/org.openehr.rm.common.audit_details.adoc`
    /// §Attributes, inherited by `…org.openehr.rm.common.attestation.adoc`)
    /// and `DV_CODED_TEXT` is a substitutable subtype — so the `_type`, the
    /// display `value` and the whole `defining_code` come back unchanged
    /// rather than being flattened to the plain text they share.
    #[test]
    fn coded_description_round_trips() {
        let description = json!({
            "_type": "DV_CODED_TEXT",
            "value": "amended after multidisciplinary review",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "local" },
                "code_string": "at0004"
            }
        });
        let att = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "description": description.clone()
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect("a coded description is a valid DV_TEXT");
        assert_eq!(att.get("description"), Some(&description));
    }

    /// The SM spelling of the same attribute — `UPDATE_AUDIT.description` is
    /// `String [0..1]` (`SM/docs/UML/classes/update_audit.adoc` §Attributes) —
    /// still completes into the plain `DV_TEXT` it denotes.
    #[test]
    fn string_description_becomes_plain_dv_text() {
        let att = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "description": "countersigned"
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect("the SM string spelling is accepted");
        assert_eq!(
            att.get("description"),
            Some(&json!({ "_type": "DV_TEXT", "value": "countersigned" }))
        );
    }

    /// A `description` that is neither a string nor a canonical `DV_TEXT` is
    /// refused (422) instead of silently dropped.
    #[test]
    fn malformed_description_is_refused() {
        let err = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "description": { "_type": "DV_CODED_TEXT", "value": "no defining_code" }
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect_err("a DV_CODED_TEXT without its mandatory defining_code must be refused");
        assert!(
            matches!(&err, ServiceError::Unprocessable(m) if m.contains("AUDIT_DETAILS.description")),
            "got {err:?}"
        );
    }

    /// `ATTESTATION.attested_view` is a `DV_MULTIMEDIA` (0..1). A value that
    /// does not decode as one is refused (422) naming the attribute, rather
    /// than stored verbatim: the completed attestation is built as the RM type,
    /// so every client-supplied attribute is read.
    #[test]
    fn malformed_attested_view_is_refused() {
        // DV_MULTIMEDIA.media_type is 1..1 (attestation.adoc §Attributes →
        // dv_multimedia.adoc), so this object cannot be one.
        for bad in [json!({ "_type": "DV_MULTIMEDIA" }), json!("a screenshot")] {
            let err = complete_attestation(
                &json!({
                    "reason": { "_type": "DV_TEXT", "value": "witness" },
                    "is_pending": false,
                    "attested_view": bad
                }),
                "ferroehr.local",
                &committer(),
                now(),
            )
            .expect_err("a malformed attested_view must be refused");
            match err {
                ServiceError::Unprocessable(msg) => assert!(
                    msg.contains("ATTESTATION.attested_view") && msg.contains("DV_MULTIMEDIA"),
                    "should name the attribute and its RM type, got {msg}"
                ),
                other => panic!("expected Unprocessable, got {other:?}"),
            }
        }
    }

    /// `ATTESTATION.proof` is a `String` (0..1) — a non-string is refused.
    #[test]
    fn non_string_proof_is_refused() {
        let err = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "proof": { "value": "not-a-string" }
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect_err("a non-string proof must be refused");
        assert!(
            matches!(&err, ServiceError::Unprocessable(m) if m.contains("ATTESTATION.proof")),
            "got {err:?}"
        );
    }

    /// `ATTESTATION.items` is a `List<DV_EHR_URI>` — a member that is not one
    /// is refused, naming its index.
    #[test]
    fn malformed_items_member_is_refused() {
        let err = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "items": [{ "_type": "DV_EHR_URI", "value": "ehr://x/y" }, 7]
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect_err("a malformed items member must be refused");
        assert!(
            matches!(&err, ServiceError::Unprocessable(m) if m.contains("ATTESTATION.items[1]")),
            "got {err:?}"
        );
    }

    /// A coded `reason` whose `defining_code` is outside the openEHR
    /// `attestation reason` group is refused (`ATTESTATION.Reason_valid`,
    /// `org.openehr.rm.common.attestation.adoc` §Invariants).
    #[test]
    fn out_of_group_coded_reason_is_refused() {
        let err = complete_attestation(
            &json!({
                "reason": {
                    "_type": "DV_CODED_TEXT",
                    "value": "signed",
                    "defining_code": {
                        "_type": "CODE_PHRASE",
                        "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                        // 249 is `creation` (audit change type), not an
                        // `attestation reason` member ({240 signed, 648 witnessed}).
                        "code_string": "249"
                    }
                },
                "is_pending": false
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect_err("an out-of-group coded reason must be refused");
        assert!(
            matches!(&err, ServiceError::Unprocessable(m) if m.contains("Reason_valid")),
            "got {err:?}"
        );
    }

    /// A JSON `null` `items` is the ABSENT encoding of the 0..1 attribute, so
    /// it is accepted and yields no list — `ATTESTATION.Items_valid` (`items
    /// /= Void implies not items.is_empty`) constrains a PRESENT list, and
    /// Void is exactly what a null spells. This is the shape the native
    /// `UPDATE_VERSION.attestations` DTO emits for `Option::None`, so a
    /// rejection here would make the at-committal attestation route
    /// unreachable through the typed API.
    #[test]
    fn null_items_is_absent_not_invalid() {
        let att = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "items": Value::Null,
                "proof": Value::Null,
                "attested_view": Value::Null,
                "description": Value::Null
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect("a null optional is absent, not invalid");
        assert_eq!(att.get("items"), None, "no items list is serialized");
    }

    /// A present-but-empty `items` list is refused (`ATTESTATION.Items_valid`:
    /// `items /= Void implies not items.is_empty`,
    /// `org.openehr.rm.common.attestation.adoc` §Invariants).
    #[test]
    fn present_but_empty_items_is_refused() {
        let err = complete_attestation(
            &json!({
                "reason": { "_type": "DV_TEXT", "value": "witness" },
                "is_pending": false,
                "items": []
            }),
            "ferroehr.local",
            &committer(),
            now(),
        )
        .expect_err("a present-but-empty items list must be refused");
        assert!(
            matches!(&err, ServiceError::Unprocessable(m) if m.contains("Items_valid")),
            "got {err:?}"
        );
    }
}
