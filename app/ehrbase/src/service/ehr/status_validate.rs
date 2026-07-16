//! Structural validation of an `EHR_STATUS` before commit (split out of
//! [`status`](super::status) to keep both files under the size bound).
//!
//! Spec: RM ehr `org.openehr.rm.ehr.ehr_status.adoc` + inherited
//! `rm.common.locatable.adoc`; CNF `master06 §Test Data Sets` (INVALID class 2)
//! is the oracle for the rejected data sets.

use serde_json::Value;

use crate::service::ServiceError;

/// Structurally validate an `EHR_STATUS` before it is committed (on EHR create,
/// `EHR_STATUS` update, or a CONTRIBUTION). Rejects every malformed data set the
/// CNF `master06 §Test Data Sets` (INVALID class 2) enumerates with a `422`.
///
/// Rules — RM ehr §`EHR_STATUS` + inherited `LOCATABLE`:
/// - `_type` present and equal to `EHR_STATUS`;
/// - `name` present (`LOCATABLE.name` 1..1);
/// - `archetype_node_id` present and non-empty (`Archetype_node_id_valid`);
/// - `is_queryable` / `is_modifiable` present booleans (both 1..1);
/// - `subject` present and a `PARTY_SELF` (`EHR_STATUS.subject` 1..1 `PARTY_SELF`;
///   monomorphic, so a foreign concrete `_type` is invalid — enforced via the
///   generated `PartySelf`'s `_type` check). An empty `{}` subject is a valid
///   **anonymous** subject (RM ehr master04 §EHR Status: `PARTY_SELF` "enabling it
///   to be made completely anonymous");
/// - a present `subject.external_ref` is a valid `PARTY_REF` (non-empty
///   `id.value` — `Id_exists`; non-empty `namespace` — `Namespace_valid`); a
///   NULL `external_ref` is permitted;
/// - a present `other_details` is a concrete `ITEM_STRUCTURE` (RM ehr
///   `ehr_status.adoc` `other_details`; RM `data_structures` master04).
fn validate_ehr_status(status: &Value) -> Result<(), ServiceError> {
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
    serde_json::from_value::<openehr_rm::prelude::PartySelf>(subject.clone()).map_err(|e| {
        unproc(format!(
            "EHR_STATUS.subject must be a PARTY_SELF (RM ehr master04 §EHR Status): {e}"
        ))
    })?;

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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::validate_ehr_status;
    use crate::service::ehr::default_ehr_status;

    /// `EHR_STATUS.other_details` must be a concrete `ITEM_STRUCTURE`
    /// (RM ehr `ehr_status.adoc`): the four concrete subtypes pass, a foreign or
    /// missing `_type` rejects.
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
    /// (`master06 §Test Data Sets`, INVALID class 2) must be rejected — with one
    /// spec-cited exception: `001_ehr_status_subject_empty.json` (`subject: {}`)
    /// is spec-VALID (an empty `PARTY_SELF` is a completely anonymous subject,
    /// master04), a documented corpus-vs-spec adjudication.
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
}
