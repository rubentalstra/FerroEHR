//! Builders for the ITS-REST version wrappers (`VERSIONED_OBJECT`,
//! `ORIGINAL_VERSION`) from a loaded [`VersionRead`](super::vobject::VersionRead).
//! These surface the full provenance a versioned object carries: its
//! `OBJECT_VERSION_ID`, the CONTRIBUTION that produced it, and its data.

use serde_json::{Value, json};
use uuid::Uuid;

use super::EhrbaseService;
use super::vobject::VersionRead;

impl EhrbaseService {
    /// A `VERSIONED_OBJECT` for `vo_id` owned by `ehr_id`.
    pub(super) fn versioned_object(vo_id: Uuid, ehr_id: Uuid) -> Value {
        json!({
            "_type": "VERSIONED_OBJECT",
            "uid": { "_type": "HIER_OBJECT_ID", "value": vo_id.to_string() },
            "owner_id": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "EHR",
                "id": { "_type": "HIER_OBJECT_ID", "value": ehr_id.to_string() }
            }
        })
    }

    /// An `ORIGINAL_VERSION` wrapping a loaded version: its `OBJECT_VERSION_ID`,
    /// the CONTRIBUTION reference, lifecycle state, and the data itself.
    pub(super) fn original_version(&self, read: &VersionRead) -> Value {
        json!({
            "_type": "ORIGINAL_VERSION",
            "uid": {
                "_type": "OBJECT_VERSION_ID",
                "value": self.object_version_id(read.vo_id, read.sys_version)
            },
            "contribution": {
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "CONTRIBUTION",
                "id": { "_type": "HIER_OBJECT_ID", "value": read.contribution_id.to_string() }
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
            "data": read.canonical
        })
    }
}
