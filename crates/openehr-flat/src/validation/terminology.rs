//! RM-mandated openEHR-terminology validation — the composition validator's
//! terminology pass.
//!
//! The slot → vocabulary binding table and the membership decisions are the
//! single, shared hook in [`openehr_its::rm_terminology`] (this crate does not
//! re-derive them): the same table the wire-boundary dispatcher
//! ([`openehr_its::rm_validate::validate_rm_value`]) enforces. This pass keeps
//! only the composition-validator *presentation* — a [`ValidationKind::Terminology`]
//! message carrying the human-readable vocabulary label and the RM instance path
//! — recursing the instance with the running-path buffer.
//!
//! Two families of terminology binding are enforced, both properties of the RM
//! instance (independent of the archetype / `WebTemplate`): openEHR terminology
//! *group* codes (`has_code_for_group_id`, guarded by `terminology_id = "openehr"`)
//! and openEHR / ISO / IANA code-set codes (`code_set (id).has_code`, unguarded).
//! See [`openehr_its::rm_terminology`] for the per-slot spec citations
//! (`docs/specs/openehr/RM/docs/UML/classes/`) resolved against the terminology
//! bundle in [`openehr_term::bundle`] (TERM 3.1.0).

use openehr_its::rm_terminology::{Slot, slot_is_violated, slots_for};
use serde_json::Value;

use super::{ValidationKind, Validator, norm_path};

impl Validator {
    // `path` is the single reusable running-path buffer pushed/popped per step,
    // mirroring [`super::Validator::rm_invariant_pass`]: a segment is appended
    // before a coded-slot check or a recursion and truncated back after, so the
    // full path string is materialized only when a violation is recorded.
    pub(super) fn terminology_pass(
        &mut self,
        v: &Value,
        path: &mut String,
        _parent_type: Option<&str>,
    ) {
        use std::fmt::Write as _;
        let Some(obj) = v.as_object() else { return };
        let this_type = obj.get("_type").and_then(Value::as_str).unwrap_or("");

        // Coded slots fixed by the owning RM type (the shared binding table).
        for slot in slots_for(this_type) {
            if let Some(node) = obj.get(slot.field) {
                let base = path.len();
                let _ = write!(path, "/{}", slot.field);
                self.check_code(slot, node, path);
                path.truncate(base);
            }
        }

        for (k, val) in obj {
            if k.starts_with('_') {
                continue;
            }
            match val {
                Value::Array(a) => {
                    for (i, item) in a.iter().enumerate() {
                        if item.is_object() {
                            let base = path.len();
                            let _ = write!(path, "/{k}[{i}]");
                            self.terminology_pass(item, path, Some(this_type));
                            path.truncate(base);
                        }
                    }
                }
                Value::Object(_) => {
                    let base = path.len();
                    let _ = write!(path, "/{k}");
                    self.terminology_pass(val, path, Some(this_type));
                    path.truncate(base);
                }
                _ => {}
            }
        }
    }

    /// Validate a coded node against its slot binding, recording a
    /// [`ValidationKind::Terminology`] violation naming the vocabulary label.
    fn check_code(&mut self, slot: &Slot, node: &Value, path: &str) {
        if !slot_is_violated(slot, node) {
            return;
        }
        // `slot_is_violated` returned true, so the node carries an in-scope code;
        // re-read it for the message. (The bare `defining_code`/`code_string`
        // read cannot fail here.)
        let code = node
            .get("defining_code")
            .unwrap_or(node)
            .get("code_string")
            .and_then(Value::as_str)
            .unwrap_or("");
        self.push(
            norm_path(path),
            format!(
                "code '{code}' is not a valid {} (openEHR terminology)",
                slot.binding.label()
            ),
            ValidationKind::Terminology,
        );
    }
}
