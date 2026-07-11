//! Hand-written RM class invariants for `GENERIC_ENTRY`.
//!
//! Spec: RM 1.2.0 integration `master02-integration_package.adoc` +
//! `org.openehr.rm.integration.generic_entry.adoc` — GENERIC_ENTRY carries
//! only the generic `data: ITEM` (1..1, enforced structurally by the typed
//! deserialize) plus the inherited LOCATABLE duties
//! (`Archetype_node_id_valid`).

use crate::integration::generic_entry::GenericEntry;
use crate::validate::{InvariantViolation, Validate, push_archetype_node_id_valid};

impl Validate for GenericEntry {
    fn validate_invariants(&self, out: &mut Vec<InvariantViolation>) {
        push_archetype_node_id_valid(out, "GENERIC_ENTRY", &self.archetype_node_id);
    }
}
