//! Hand-written spec functions of `REVISION_HISTORY`.
//!
//! Spec: RM
//! `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.revision_history.adoc`
//! §Functions declares two, each pinned by a postcondition over `items`:
//!
//! - `most_recent_version ()`: "The version id of the most recent item, as a
//!   String" — `Post: Result.is_equal (items.last.version_id.value)`.
//! - `most_recent_version_time_committed ()`: "The commit date/time of the most
//!   recent item, as a String" —
//!   `Post: Result.is_equal (items.last.audits.first.time_committed.value)`.
//!
//! Both read the LAST item, because `items` is "The items in this history in
//! most-recent-last order" (§Attributes), and the second reads the FIRST audit
//! of that item, because `REVISION_HISTORY_ITEM.audits` holds "the commit audit
//! … there may also be further attestations"
//! (`…org.openehr.rm.common.revision_history_item.adoc` §Attributes).
//!
//! NOTE: the spec types both functions `1..1`, but the BMM `List` attributes
//! they walk emit as `Vec`, so a value with no items — or an item with no
//! audits, which `REVISION_HISTORY_ITEM.Audit_valid` (`not audits.is_empty`)
//! forbids — is representable in the Rust type and not in the spec model. Each
//! function therefore returns an `Option`, `None` exactly on that
//! spec-unrepresentable input, so a caller can never read a fabricated id or
//! instant out of an empty history.

use crate::common::generic::audit_details::AuditDetails;
use crate::common::generic::revision_history::RevisionHistory;

impl RevisionHistory {
    /// `REVISION_HISTORY.most_recent_version`: the version id of the most
    /// recent item (the last one — `items` is in most-recent-last order), as
    /// its `String` value.
    ///
    /// Returns `None` when the history holds no items, a state the spec model
    /// cannot express (see the module docs).
    #[must_use]
    pub fn most_recent_version(&self) -> Option<&str> {
        self.items.last().map(|i| i.version_id.value.as_str())
    }

    /// `REVISION_HISTORY.most_recent_version_time_committed`: the commit
    /// date/time of the most recent item — the `time_committed` of that item's
    /// FIRST audit, which is its commit audit — as its `String` value.
    ///
    /// Returns `None` when the history holds no items, or the most recent item
    /// carries no audit (`REVISION_HISTORY_ITEM.Audit_valid` forbids the
    /// latter; see the module docs).
    #[must_use]
    pub fn most_recent_version_time_committed(&self) -> Option<&str> {
        self.items.last()?.audits.first().map(|a| match a {
            AuditDetails::AuditDetails(d) => d.time_committed.value.as_str(),
            AuditDetails::Attestation(a) => a.time_committed.value.as_str(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::generic::attestation::Attestation;
    use crate::common::generic::audit_details::AuditDetailsData;
    use crate::common::generic::party_proxy::PartyProxy;
    use crate::common::generic::party_self::PartySelf;
    use crate::common::generic::revision_history_item::RevisionHistoryItem;
    use crate::data_types::quantity::date_time::dv_date_time::DvDateTime;
    use crate::data_types::text::code_phrase::CodePhrase;
    use crate::data_types::text::dv_coded_text::DvCodedText;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_base::prelude::{ObjectVersionId, TerminologyId};

    fn dv_date_time(value: &str) -> DvDateTime {
        DvDateTime {
            normal_status: None,
            normal_range: None,
            other_reference_ranges: openehr_base::containers::present(Vec::new()),
            magnitude_status: None,
            accuracy: None,
            value: value.to_owned(),
        }
    }

    fn change_type(code: &str, rubric: &str) -> DvCodedText {
        DvCodedText {
            value: rubric.to_owned(),
            hyperlink: None,
            formatting: None,
            mappings: openehr_base::containers::present(Vec::new()),
            language: None,
            encoding: None,
            defining_code: CodePhrase {
                terminology_id: TerminologyId {
                    value: "openehr".to_owned(),
                },
                code_string: code.to_owned(),
                preferred_term: None,
            },
        }
    }

    fn commit_audit(time: &str) -> AuditDetails {
        AuditDetails::AuditDetails(AuditDetailsData {
            system_id: "ferroehr.local".to_owned(),
            time_committed: dv_date_time(time),
            change_type: change_type("249", "creation"),
            description: None,
            committer: PartyProxy::PartySelf(PartySelf { external_ref: None }),
        })
    }

    /// An ATTESTATION appended AFTER the commit audit — the second audit of an
    /// item (`revision_history_item.adoc` §Attributes: "there may also be
    /// further attestations"), whose later `time_committed` must NOT be the
    /// value the function reports.
    fn attestation(time: &str) -> AuditDetails {
        AuditDetails::Attestation(Attestation {
            system_id: "ferroehr.local".to_owned(),
            time_committed: dv_date_time(time),
            change_type: change_type("666", "attestation"),
            description: None,
            committer: PartyProxy::PartySelf(PartySelf { external_ref: None }),
            attested_view: None,
            proof: None,
            items: openehr_base::containers::present(Vec::new()),
            reason: DvText::DvText(DvTextData {
                value: "witnessed".to_owned(),
                hyperlink: None,
                formatting: None,
                mappings: openehr_base::containers::present(Vec::new()),
                language: None,
                encoding: None,
            }),
            is_pending: false,
        })
    }

    fn item(version_id: &str, audits: Vec<AuditDetails>) -> RevisionHistoryItem {
        RevisionHistoryItem {
            version_id: ObjectVersionId {
                value: version_id.to_owned(),
            },
            audits,
        }
    }

    fn history() -> RevisionHistory {
        RevisionHistory {
            items: vec![
                item(
                    "8849182c-82ad-4088-a07f-48ead4180515::ferroehr.local::1",
                    vec![commit_audit("2026-07-07T10:11:12Z")],
                ),
                item(
                    "8849182c-82ad-4088-a07f-48ead4180515::ferroehr.local::2",
                    vec![
                        commit_audit("2026-07-08T09:00:00Z"),
                        attestation("2026-07-09T09:00:00Z"),
                    ],
                ),
            ],
        }
    }

    /// `Post: Result.is_equal (items.last.version_id.value)`.
    #[test]
    fn most_recent_version_is_the_last_items_version_id() {
        let h = history();
        assert_eq!(
            h.most_recent_version(),
            h.items.last().map(|i| i.version_id.value.as_str())
        );
        assert_eq!(
            h.most_recent_version(),
            Some("8849182c-82ad-4088-a07f-48ead4180515::ferroehr.local::2")
        );
    }

    /// `Post: Result.is_equal (items.last.audits.first.time_committed.value)`
    /// — the FIRST audit of the last item, never a later attestation's instant.
    #[test]
    fn most_recent_time_committed_is_the_last_items_first_audit() {
        let h = history();
        assert_eq!(
            h.most_recent_version_time_committed(),
            Some("2026-07-08T09:00:00Z")
        );
    }

    /// The spec-unrepresentable inputs report absence rather than a fabricated
    /// value (module docs).
    #[test]
    fn empty_shapes_report_none() {
        let empty = RevisionHistory { items: Vec::new() };
        assert_eq!(empty.most_recent_version(), None);
        assert_eq!(empty.most_recent_version_time_committed(), None);

        let auditless = RevisionHistory {
            items: vec![item("x::y::1", Vec::new())],
        };
        assert_eq!(auditless.most_recent_version(), Some("x::y::1"));
        assert_eq!(auditless.most_recent_version_time_committed(), None);
    }
}
