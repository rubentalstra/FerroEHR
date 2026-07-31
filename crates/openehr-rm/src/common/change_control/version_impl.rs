//! Hand-written RM spec function `VERSION.canonical_form()` (hand-written spec behaviour).
//!
//! Spec authority:
//! - RM common §"Digital Signature"
//!   (`docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`):
//!   "a Version object (an `ORIGINAL_VERSION` or `IMPORTED_VERSION`) is
//!   serialised into canonical form which is then hashed to produce a digest …
//!   note that the signature attribute will be Void at this point". For an
//!   `IMPORTED_VERSION` "all attributes of the object are serialised".
//! - `VERSION.canonical_form()`
//!   (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.version.adoc`):
//!   "A canonical serial form of this Version, created by serialising all
//!   attributes except signature, suitable for generating reliable hashes and
//!   signatures."
//!
//! NOTE: the spec leaves the exact serialization `[.tbd]` ("not yet defined by
//! openEHR; ODIN might be preferred"). We serialise the Version to canonical
//! openEHR JSON (the native ITS-JSON codec's encoding — `_type`-tagged,
//! nulls/empties omitted) with the top-level `signature` attribute removed, then
//! canonicalise per RFC 8785 (JSON Canonicalization Scheme, `serde_jcs`) so the
//! signed bytes are deterministic (key ordering, number formatting, string
//! escaping pinned by the RFC). This is the single source of the signed bytes
//! for both signing and verification.
//!
//! The signed input is always assembled as a `serde_json::Value` (the shape the
//! versioning service and the wire boundary already hold), so this module works
//! purely on a `Value` — the canonical-JSON serialization of a *typed* Version
//! is a wire-boundary concern that lives with the codec in `openehr-its`, not
//! here.

use serde_json::Value;

/// Failure to produce a Version canonical form.
#[derive(Debug, thiserror::Error)]
pub enum CanonicalFormError {
    /// RFC 8785 (JCS) canonicalisation failed.
    #[error("RFC 8785 (JCS) canonicalisation: {0}")]
    Canonicalize(#[source] serde_json::Error),
}

/// Produce the spec `canonical_form` of an already-serialised Version JSON value:
/// drop the top-level `signature` attribute (Void during serialisation per RM
/// common §"Digital Signature") and emit the RFC 8785 (JCS) canonical string.
///
/// The application service assembles the Version as a `serde_json::Value` before
/// persistence and calls this, so signing and verification share one source of
/// the signed bytes.
///
/// Spec authority: RM common §"Digital Signature"
/// (`docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`) and
/// `VERSION.canonical_form()`
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.version.adoc`).
///
/// # Errors
/// Returns [`CanonicalFormError::Canonicalize`] if RFC 8785 canonicalisation
/// of the value fails.
pub fn canonical_form_of_json(value: &Value) -> Result<String, CanonicalFormError> {
    let mut value = value.clone();
    if let Value::Object(map) = &mut value {
        map.remove("signature");
    }
    serde_jcs::to_string(&value).map_err(CanonicalFormError::Canonicalize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative `ORIGINAL_VERSION` JSON whose `data` is a real corpus
    /// COMPOSITION (`openehr-its` vendored `minimal_persistent.json`), with fixed
    /// version metadata so the canonical form is deterministic.
    const COMPOSITION: &str = include_str!(
        "../../../../openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/minimal_persistent.json"
    );

    /// Build the `ORIGINAL_VERSION` canonical-JSON `Value` around the corpus
    /// composition, with the given `signature` value (so tests can compare None
    /// vs Some). The signed input is assembled as a `Value` — the shape the
    /// versioning service holds — and canonicalised via [`canonical_form_of_json`].
    fn original_version(signature: Option<&str>) -> Value {
        let data: Value = serde_json::from_str(COMPOSITION).unwrap();
        let mut ov = serde_json::json!({
            "_type": "ORIGINAL_VERSION",
            "contribution": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "CONTRIBUTION",
                "id": {
                    "_type": "HIER_OBJECT_ID",
                    "value": "0198f4a5-9df1-7d1e-8b6f-2b8c00000abc"
                }
            },
            "commit_audit": {
                "_type": "AUDIT_DETAILS",
                "system_id": "ferroehr.local",
                "time_committed": {
                    "_type": "DV_DATE_TIME",
                    "value": "2026-07-07T10:11:12.5Z"
                },
                "change_type": {
                    "_type": "DV_CODED_TEXT",
                    "value": "creation",
                    "defining_code": {
                        "_type": "CODE_PHRASE",
                        "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                        "code_string": "249"
                    }
                },
                "committer": { "_type": "PARTY_IDENTIFIED", "name": "FerroEHR" }
            },
            "uid": {
                "_type": "OBJECT_VERSION_ID",
                "value": "0198f4a5-9df1-7d1e-8b6f-2b8c00000001::ferroehr.local::1"
            },
            "lifecycle_state": {
                "_type": "DV_CODED_TEXT",
                "value": "complete",
                "defining_code": {
                    "_type": "CODE_PHRASE",
                    "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                    "code_string": "532"
                }
            },
            "data": data
        });
        if let Some(sig) = signature {
            ov.as_object_mut()
                .unwrap()
                .insert("signature".to_owned(), Value::String(sig.to_owned()));
        }
        ov
    }

    #[test]
    fn canonical_form_is_byte_stable_across_repeated_calls() {
        let ov = original_version(None);
        let a = canonical_form_of_json(&ov).unwrap();
        let b = canonical_form_of_json(&ov).unwrap();
        assert_eq!(a, b, "canonical_form must be deterministic");
    }

    #[test]
    fn canonical_form_independent_of_signature_value() {
        // RM common §"Digital Signature": the signature attribute is Void during
        // serialisation — so its stored value must not affect the canonical form.
        let none = canonical_form_of_json(&original_version(None)).unwrap();
        let some = canonical_form_of_json(&original_version(Some("sha256:tampered"))).unwrap();
        let other = canonical_form_of_json(&original_version(Some(
            "-----BEGIN PGP SIGNATURE-----\nabc\n-----END PGP SIGNATURE-----",
        )))
        .unwrap();
        assert_eq!(none, some);
        assert_eq!(none, other);
    }

    #[test]
    fn canonical_form_golden() {
        // Golden vector: pins the RFC 8785 canonical form of the corpus
        // ORIGINAL_VERSION so any accidental change to the algorithm is caught.
        let canonical = canonical_form_of_json(&original_version(None)).unwrap();
        insta::assert_snapshot!("original_version_canonical_form", canonical);
    }
}
