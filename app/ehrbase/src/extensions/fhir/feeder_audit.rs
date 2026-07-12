//! The `FEEDER_AUDIT` provenance builder for the FHIR connector (carved out of
//! `mapping.rs` per register 12, G-12-04).
//!
//! The FHIR connector as a whole is spec-silent (no openEHR spec governs the
//! FHIR↔openEHR mapping — our own design/extension; `crate::extensions`). **This
//! submodule, however, builds RM-typed data**: the `FEEDER_AUDIT` /
//! `FEEDER_AUDIT_DETAILS` provenance stamped on the imported COMPOSITION. That
//! shape is governed by **RM common `feeder_audit`** (`FEEDER_AUDIT`,
//! `FEEDER_AUDIT_DETAILS.system_id`/`time`/`version_id`,
//! `originating_system_item_ids: List<DV_IDENTIFIER>`) —
//! `docs/specs/openehr/RM/docs/common/feeder_audit.adoc`; the `DV_IDENTIFIER`
//! shape (with its `Id_valid` non-empty-id invariant) is RM data_types
//! `DV_IDENTIFIER`. (master14's *integration* model is archetype-level and does
//! not govern this builder; the RM `FEEDER_AUDIT` types do.)
//!
//! Gate: the connector's inbound routes are config-gated in `ehrbase-rest`; this
//! builder only runs on the ingest path.

use serde_json::{Value, json};

/// The `system_id` recorded in the built COMPOSITION's `FEEDER_AUDIT`
/// originating-system audit (RM common `FEEDER_AUDIT_DETAILS`), naming the
/// import channel.
pub(super) const ORIGINATING_SYSTEM: &str = "fhir-connector";

/// The resource's logical id (`id`), or a non-empty fallback (`DV_IDENTIFIER`'s
/// `Id_valid` invariant forbids an empty id).
pub(super) fn resource_id(resource: &Value, resource_type: &str) -> String {
    super::mapping::resolve(resource, "id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("{resource_type}/unknown"), str::to_owned)
}

/// The resource version (`meta.versionId`), if present.
pub(super) fn resource_version(resource: &Value) -> Option<String> {
    super::mapping::resolve(resource, "meta.versionId")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// The FHIR import timestamp for `FEEDER_AUDIT` (ISO 8601, UTC).
pub(super) fn now_iso() -> String {
    jiff::Timestamp::now().to_string()
}

/// Build the `FEEDER_AUDIT` (canonical JSON) recording the FHIR import trail:
/// originating system `fhir-connector`, the resource type/id as an
/// originating-system item id, and the resource version + import time on the
/// originating-system audit (RM common `FEEDER_AUDIT_DETAILS`).
pub(super) fn feeder_audit(
    resource_type: &str,
    resource_id: &str,
    version_id: Option<&str>,
    time_iso: &str,
) -> Value {
    let mut details = json!({
        "_type": "FEEDER_AUDIT_DETAILS",
        "system_id": ORIGINATING_SYSTEM,
        "time": { "_type": "DV_DATE_TIME", "value": time_iso },
    });
    if let Some(v) = version_id {
        details["version_id"] = json!(v);
    }
    json!({
        "_type": "FEEDER_AUDIT",
        "originating_system_item_ids": [
            { "_type": "DV_IDENTIFIER", "id": resource_id, "type": resource_type, "issuer": "FHIR" }
        ],
        "originating_system_audit": details,
    })
}

/// Attach a `FEEDER_AUDIT` to a canonical-JSON COMPOSITION object.
pub(super) fn inject_feeder_audit(comp: &mut Value, feeder_audit: Value) {
    if let Value::Object(m) = comp {
        m.insert("feeder_audit".to_owned(), feeder_audit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeder_audit_shape() {
        let fa = feeder_audit("Observation", "obs-1", Some("2"), "2026-07-11T00:00:00Z");
        assert_eq!(fa["_type"], json!("FEEDER_AUDIT"));
        assert_eq!(
            fa["originating_system_audit"]["system_id"],
            json!(ORIGINATING_SYSTEM)
        );
        assert_eq!(fa["originating_system_audit"]["version_id"], json!("2"));
        assert_eq!(fa["originating_system_item_ids"][0]["id"], json!("obs-1"));
        assert_eq!(
            fa["originating_system_item_ids"][0]["type"],
            json!("Observation")
        );
    }

    #[test]
    fn resource_id_falls_back_when_absent() {
        assert_eq!(
            resource_id(&json!({}), "Observation"),
            "Observation/unknown"
        );
        assert_eq!(resource_id(&json!({ "id": "bp-1" }), "Observation"), "bp-1");
    }

    #[test]
    fn resource_version_reads_meta() {
        assert_eq!(
            resource_version(&json!({ "meta": { "versionId": "3" } })).as_deref(),
            Some("3")
        );
        assert_eq!(resource_version(&json!({})), None);
    }
}
