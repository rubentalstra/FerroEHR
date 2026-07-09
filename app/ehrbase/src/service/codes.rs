//! openEHR change-control terminology codes: audit change-types and version
//! lifecycle states.
//!
//! The wire form of a coded value is its **numeric group code** (an
//! `AUDIT_DETAILS.change_type.defining_code.code_string` MUST be a member code
//! of the `audit_change_type` group, e.g. `"249"`, not the rubric `"creation"`
//! — RM `AUDIT_DETAILS.Change_type_valid`; likewise `ORIGINAL_VERSION.
//! lifecycle_state` is coded from the `version_lifecycle_state` group). We store
//! the numeric code and resolve the human rubric from the `openehr-term` bundle
//! at the render edge — never a hardcoded rubric string (findings F-06-02,
//! F-06-04, F-11-01, F-01-06, F-02-06/07).

use openehr_term::bundle::openehr;

/// The openEHR internal terminology id (`Terminology_id_openehr`).
pub(super) const OPENEHR: &str = "openehr";

/// The `audit_change_type` openEHR terminology group id.
const AUDIT_CHANGE_TYPE: &str = "audit_change_type";
/// The `version_lifecycle_state` openEHR terminology group id.
const VERSION_LIFECYCLE_STATE: &str = "version_lifecycle_state";

/// `audit_change_type` group codes used by the service write paths. The full
/// group (`TERM/computable/XML/en/openehr_terminology.xml`) is `249 creation`,
/// `250 amendment`, `251 modification`, `252 synthesis`, `523 deleted`,
/// `666 attestation`, `816 restoration`, `817 format conversion`,
/// `253 unknown`; membership checks go through [`change_type_code`], so only
/// the codes the service handles by name get a constant here.
pub(super) mod change_type {
    /// `249|creation|` — first version of a versioned object.
    pub(crate) const CREATION: &str = "249";
    /// `251|modification|` — a content change to an existing object.
    pub(crate) const MODIFICATION: &str = "251";
    /// `523|deleted|` — a logical deletion.
    pub(crate) const DELETED: &str = "523";
    /// `666|attestation|` — attaches an `ATTESTATION` to an existing
    /// `ORIGINAL_VERSION` (adds no new version — RM `change_control`
    /// §Contributions; handled by the contribution path's `Action::Attest`).
    pub(crate) const ATTESTATION: &str = "666";
}

/// `version_lifecycle_state` group codes.
pub(super) mod lifecycle {
    /// `532|complete|` — a fully authored version.
    pub(crate) const COMPLETE: &str = "532";
    /// `523|deleted|` — a logically deleted version.
    pub(crate) const DELETED: &str = "523";
}

/// Resolve an inbound audit `change_type` token — either a numeric group code
/// (`"249"`) or a rubric (`"creation"`) — to its canonical numeric group code.
/// `None` when the token is not a member of the `audit_change_type` group
/// (RM `AUDIT_DETAILS.Change_type_valid` — callers must reject, never store,
/// an out-of-group change type; finding F-06-06).
pub(super) fn change_type_code(token: &str) -> Option<String> {
    let t = openehr();
    if t.is_valid_audit_change_type(token) {
        return Some(token.to_owned());
    }
    t.concepts_in_group(AUDIT_CHANGE_TYPE)
        .iter()
        .find(|c| c.rubric.eq_ignore_ascii_case(token))
        .map(|c| c.id.clone())
}

/// Resolve an inbound `lifecycle_state` token — either a numeric group code
/// (`"553"`) or a rubric (`"incomplete"`) — to its canonical numeric
/// `version_lifecycle_state` group code. `None` when the token is not a member
/// of the group (`ORIGINAL_VERSION.Lifecycle_state_valid` — callers must
/// reject, never store, an out-of-group lifecycle state; RM common master06
/// §"Version Lifecycle": the five values are `532|complete|`, `553|incomplete|`,
/// `523|deleted|`, `800|inactive|`, `801|abandoned|`).
pub(super) fn lifecycle_state_code(token: &str) -> Option<String> {
    let t = openehr();
    if t.is_valid_version_lifecycle_state(token) {
        return Some(token.to_owned());
    }
    t.concepts_in_group(VERSION_LIFECYCLE_STATE)
        .iter()
        .find(|c| c.rubric.eq_ignore_ascii_case(token))
        .map(|c| c.id.clone())
}

/// The rubric (English display text) for an `audit_change_type` code; falls back
/// to the code itself if the code is unknown to the bundle.
pub(super) fn change_type_rubric(code: &str) -> String {
    openehr()
        .rubric(AUDIT_CHANGE_TYPE, code, "en")
        .unwrap_or(code)
        .to_owned()
}

/// The rubric (English display text) for a `version_lifecycle_state` code; falls
/// back to the code itself if unknown. Note the SPECPR-51 quirk: `532` is
/// `complete` in this group (`completed` only in `instruction_states`) — the
/// group-scoped lookup resolves it correctly.
pub(super) fn lifecycle_rubric(code: &str) -> String {
    openehr()
        .rubric(VERSION_LIFECYCLE_STATE, code, "en")
        .unwrap_or(code)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_type_codes_are_group_members() {
        let t = openehr();
        for code in [
            change_type::CREATION,
            change_type::MODIFICATION,
            change_type::DELETED,
        ] {
            assert!(t.is_valid_audit_change_type(code), "code {code}");
            // code_string must be numeric (AUDIT_DETAILS.Change_type_valid).
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn rubric_round_trips() {
        assert_eq!(change_type_rubric(change_type::CREATION), "creation");
        assert_eq!(
            change_type_rubric(change_type::MODIFICATION),
            "modification"
        );
        assert_eq!(change_type_rubric(change_type::DELETED), "deleted");
        // SPECPR-51: version lifecycle 532 → "complete" (not "completed").
        assert_eq!(lifecycle_rubric(lifecycle::COMPLETE), "complete");
        assert_eq!(lifecycle_rubric(lifecycle::DELETED), "deleted");
    }

    #[test]
    fn lifecycle_state_code_accepts_code_or_rubric_and_rejects_non_members() {
        // All five normative states (master06 §Version Lifecycle) resolve, by
        // code and by rubric.
        assert_eq!(lifecycle_state_code("532").as_deref(), Some("532"));
        assert_eq!(lifecycle_state_code("complete").as_deref(), Some("532"));
        assert_eq!(lifecycle_state_code("incomplete").as_deref(), Some("553"));
        assert_eq!(lifecycle_state_code("Deleted").as_deref(), Some("523"));
        assert_eq!(lifecycle_state_code("inactive").as_deref(), Some("800"));
        assert_eq!(lifecycle_state_code("abandoned").as_deref(), Some("801"));
        // Out-of-group tokens are rejected (ORIGINAL_VERSION.Lifecycle_state_valid).
        assert_eq!(lifecycle_state_code("not-a-state"), None);
        assert_eq!(lifecycle_state_code("249"), None); // change-type code, wrong group
    }

    #[test]
    fn change_type_code_accepts_code_or_rubric_and_rejects_non_members() {
        assert_eq!(change_type_code("249").as_deref(), Some("249"));
        assert_eq!(change_type_code("creation").as_deref(), Some("249"));
        assert_eq!(change_type_code("Deleted").as_deref(), Some("523"));
        // The full group round-trips (9 codes — verified against the bundle).
        assert_eq!(change_type_code("amendment").as_deref(), Some("250"));
        assert_eq!(change_type_code("synthesis").as_deref(), Some("252"));
        assert_eq!(change_type_code("unknown").as_deref(), Some("253"));
        assert_eq!(change_type_code("666").as_deref(), Some("666"));
        // Out-of-group tokens are rejected, not passed through
        // (AUDIT_DETAILS.Change_type_valid; F-06-06).
        assert_eq!(change_type_code("not-a-change-type"), None);
        assert_eq!(change_type_code("532"), None); // lifecycle code, wrong group
    }
}
