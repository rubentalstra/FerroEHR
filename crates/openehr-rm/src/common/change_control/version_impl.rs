//! Hand-written RM spec function `VERSION.canonical_form()` (ADR-003/ADR-004).
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
//! PORT NOTE (design `docs/design/version-signing.md` §3.1): the spec leaves the
//! exact serialization `[.tbd]` ("not yet defined by openEHR; ODIN might be
//! preferred"). We serialise the Version to canonical openEHR JSON (the ITS-JSON
//! encoding the `OpenEhrType` derive produces — `_type`-tagged, nulls/empties
//! omitted) with the top-level `signature` attribute removed, then canonicalise
//! per RFC 8785 (JSON Canonicalization Scheme, `serde_jcs`) so the signed bytes
//! are deterministic (key ordering, number formatting, string escaping pinned by
//! the RFC). This is the single source of the signed bytes for both signing and
//! verification.

use serde::Serialize;
use serde_json::Value;

use crate::common::change_control::imported_version::ImportedVersion;
use crate::common::change_control::original_version::OriginalVersion;
use crate::common::change_control::version::Version;

/// Failure to produce a Version [`canonical_form`](OriginalVersion::canonical_form).
#[derive(Debug, thiserror::Error)]
pub enum CanonicalFormError {
    /// The Version could not be serialised to canonical JSON.
    #[error("serialising Version to canonical JSON: {0}")]
    Serialize(#[source] serde_json::Error),
    /// RFC 8785 (JCS) canonicalisation failed.
    #[error("RFC 8785 (JCS) canonicalisation: {0}")]
    Canonicalize(#[source] serde_json::Error),
}

/// Produce the spec `canonical_form` of an already-serialised Version JSON value:
/// drop the top-level `signature` attribute (Void during serialisation per RM
/// common §"Digital Signature") and emit the RFC 8785 (JCS) canonical string.
///
/// This is the shared core used by the typed [`OriginalVersion`] /
/// [`ImportedVersion`] / [`Version`] methods below **and** by the application
/// service (which assembles the Version as a `serde_json::Value` before
/// persistence), so the two paths can never diverge on the signed bytes.
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

/// Serialise a typed Version and canonicalise it (the two-step `canonical_form`).
fn canonical_form<T: Serialize>(version: &T) -> Result<String, CanonicalFormError> {
    let value = serde_json::to_value(version).map_err(CanonicalFormError::Serialize)?;
    canonical_form_of_json(&value)
}

impl<T: Serialize> OriginalVersion<T> {
    /// The spec `canonical_form()` of this `ORIGINAL_VERSION` — RM common
    /// §"Digital Signature"; `VERSION.canonical_form()`. The `signature`
    /// attribute is excluded (see the module doc).
    ///
    /// # Errors
    /// Returns [`CanonicalFormError`] if the Version cannot be serialised to
    /// canonical JSON or RFC 8785 canonicalisation fails.
    pub fn canonical_form(&self) -> Result<String, CanonicalFormError> {
        canonical_form(self)
    }
}

impl<T: Serialize> ImportedVersion<T> {
    /// The spec `canonical_form()` of this `IMPORTED_VERSION` — the whole
    /// wrapper is serialised (RM common §"Digital Signature": "all attributes of
    /// the object are serialised"), `signature` excluded.
    ///
    /// # Errors
    /// Returns [`CanonicalFormError`] on serialisation/canonicalisation failure.
    pub fn canonical_form(&self) -> Result<String, CanonicalFormError> {
        canonical_form(self)
    }
}

impl<T: Serialize> Version<T> {
    /// The spec `canonical_form()` of this Version, dispatched to the concrete
    /// subtype (`VERSION.canonical_form()`).
    ///
    /// # Errors
    /// Returns [`CanonicalFormError`] on serialisation/canonicalisation failure.
    pub fn canonical_form(&self) -> Result<String, CanonicalFormError> {
        canonical_form(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// A representative `ORIGINAL_VERSION` JSON whose `data` is a real corpus
    /// COMPOSITION (`openehr-its` vendored `minimal_persistent.json`), with fixed
    /// version metadata so the canonical form is deterministic.
    const COMPOSITION: &str = include_str!(
        "../../../../openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/minimal_persistent.json"
    );

    /// Build an `ORIGINAL_VERSION<Value>` around the corpus composition, with the
    /// given `signature` value (so tests can compare None vs Some).
    fn original_version(signature: Option<&str>) -> OriginalVersion<Value> {
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
                "system_id": "ehrbase-rs.local",
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
                "committer": { "_type": "PARTY_IDENTIFIED", "name": "EHRbase" }
            },
            "uid": {
                "_type": "OBJECT_VERSION_ID",
                "value": "0198f4a5-9df1-7d1e-8b6f-2b8c00000001::ehrbase-rs.local::1"
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
        serde_json::from_value(ov).unwrap()
    }

    #[test]
    fn canonical_form_is_byte_stable_across_repeated_calls() {
        let ov = original_version(None);
        let a = ov.canonical_form().unwrap();
        let b = ov.canonical_form().unwrap();
        assert_eq!(a, b, "canonical_form must be deterministic");
    }

    #[test]
    fn canonical_form_independent_of_signature_value() {
        // RM common §"Digital Signature": the signature attribute is Void during
        // serialisation — so its stored value must not affect the canonical form.
        let none = original_version(None).canonical_form().unwrap();
        let some = original_version(Some("sha256:tampered"))
            .canonical_form()
            .unwrap();
        let other = original_version(Some(
            "-----BEGIN PGP SIGNATURE-----\nabc\n-----END PGP SIGNATURE-----",
        ))
        .canonical_form()
        .unwrap();
        assert_eq!(none, some);
        assert_eq!(none, other);
    }

    #[test]
    fn canonical_form_of_json_matches_typed() {
        // The Value-based core (used by the service) and the typed method must
        // agree for the same logical Version.
        let ov = original_version(None);
        let typed = ov.canonical_form().unwrap();
        let value = serde_json::to_value(&ov).unwrap();
        let via_json = canonical_form_of_json(&value).unwrap();
        assert_eq!(typed, via_json);
    }

    #[test]
    fn canonical_form_golden() {
        // Golden vector: pins the RFC 8785 canonical form of the corpus
        // ORIGINAL_VERSION so any accidental change to the algorithm is caught.
        let canonical = original_version(None).canonical_form().unwrap();
        insta::assert_snapshot!("original_version_canonical_form", canonical);
    }
}
