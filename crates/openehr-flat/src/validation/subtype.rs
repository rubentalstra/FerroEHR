//! RM subtype relation for type-conformance checks.
//!
//! A static allow-map for the common abstract slots (archie uses the full BMM
//! type hierarchy via reflection; a BMM-generated model arrives with the AQL
//! engine at P16 — until then this covers the abstract slots that actually
//! appear as `WebTemplate` `rmType`s). Conformance is intentionally *permissive*:
//! a violation is reported only when the constraint type is a recognised RM type
//! and the instance type is provably not it nor one of its descendants — never
//! for a type pairing this map does not know about.

/// The concrete descendants of an abstract (or supertype) RM type, keyed by the
/// supertype name. A type is conformant to `sup` iff it equals `sup` or appears
/// in `descendants(sup)`.
fn descendants(sup: &str) -> Option<&'static [&'static str]> {
    Some(match sup {
        "DATA_VALUE" => &[
            "DV_BOOLEAN",
            "DV_STATE",
            "DV_IDENTIFIER",
            "DV_TEXT",
            "DV_CODED_TEXT",
            "DV_PARAGRAPH",
            "DV_ORDINAL",
            "DV_SCALE",
            "DV_ORDERED",
            "DV_QUANTITY",
            "DV_COUNT",
            "DV_PROPORTION",
            "DV_DATE",
            "DV_TIME",
            "DV_DATE_TIME",
            "DV_DURATION",
            "DV_INTERVAL",
            "DV_MULTIMEDIA",
            "DV_PARSABLE",
            "DV_URI",
            "DV_EHR_URI",
        ],
        // DV_TEXT is a concrete type that DV_CODED_TEXT (and DV_PARAGRAPH) refine.
        "DV_TEXT" => &["DV_TEXT", "DV_CODED_TEXT", "DV_PARAGRAPH"],
        "DV_ORDERED" => &[
            "DV_ORDINAL",
            "DV_SCALE",
            "DV_QUANTITY",
            "DV_COUNT",
            "DV_PROPORTION",
            "DV_DATE",
            "DV_TIME",
            "DV_DATE_TIME",
            "DV_DURATION",
        ],
        "DV_QUANTIFIED" => &["DV_QUANTITY", "DV_COUNT", "DV_PROPORTION", "DV_AMOUNT"],
        "DV_AMOUNT" => &["DV_QUANTITY", "DV_COUNT", "DV_PROPORTION"],
        "DV_TEMPORAL" => &["DV_DATE", "DV_TIME", "DV_DATE_TIME", "DV_DURATION"],
        "EVENT" => &["EVENT", "POINT_EVENT", "INTERVAL_EVENT"],
        "ITEM" => &["ITEM", "CLUSTER", "ELEMENT"],
        "ITEM_STRUCTURE" => &["ITEM_TREE", "ITEM_LIST", "ITEM_TABLE", "ITEM_SINGLE"],
        "DATA_STRUCTURE" => &[
            "ITEM_TREE",
            "ITEM_LIST",
            "ITEM_TABLE",
            "ITEM_SINGLE",
            "HISTORY",
        ],
        "CONTENT_ITEM" => &[
            "SECTION",
            "OBSERVATION",
            "EVALUATION",
            "INSTRUCTION",
            "ACTION",
            "ADMIN_ENTRY",
            "GENERIC_ENTRY",
        ],
        "ENTRY" => &[
            "OBSERVATION",
            "EVALUATION",
            "INSTRUCTION",
            "ACTION",
            "ADMIN_ENTRY",
            "GENERIC_ENTRY",
        ],
        "CARE_ENTRY" => &["OBSERVATION", "EVALUATION", "INSTRUCTION", "ACTION"],
        "PARTY_PROXY" => &["PARTY_IDENTIFIED", "PARTY_SELF", "PARTY_RELATED"],
        "PARTY" => &["PERSON", "ORGANISATION", "GROUP", "AGENT", "ROLE", "ACTOR"],
        _ => return None,
    })
}

/// The set of RM types this validator recognises as concrete leaf types — used
/// to decide when a concrete↔concrete mismatch is confident enough to report.
fn is_known_concrete(t: &str) -> bool {
    matches!(
        t,
        "DV_BOOLEAN"
            | "DV_STATE"
            | "DV_IDENTIFIER"
            | "DV_TEXT"
            | "DV_CODED_TEXT"
            | "DV_PARAGRAPH"
            | "DV_ORDINAL"
            | "DV_SCALE"
            | "DV_QUANTITY"
            | "DV_COUNT"
            | "DV_PROPORTION"
            | "DV_DATE"
            | "DV_TIME"
            | "DV_DATE_TIME"
            | "DV_DURATION"
            | "DV_INTERVAL"
            | "DV_MULTIMEDIA"
            | "DV_PARSABLE"
            | "DV_URI"
            | "DV_EHR_URI"
            | "ELEMENT"
            | "CLUSTER"
            | "ITEM_TREE"
            | "ITEM_LIST"
            | "ITEM_TABLE"
            | "ITEM_SINGLE"
            | "HISTORY"
            | "POINT_EVENT"
            | "INTERVAL_EVENT"
            | "SECTION"
            | "OBSERVATION"
            | "EVALUATION"
            | "INSTRUCTION"
            | "ACTION"
            | "ADMIN_ENTRY"
            | "ACTIVITY"
            | "COMPOSITION"
            | "EVENT_CONTEXT"
            | "PARTY_IDENTIFIED"
            | "PARTY_SELF"
            | "PARTY_RELATED"
            | "CODE_PHRASE"
    )
}

/// Strip a generic argument (`DV_INTERVAL<DV_QUANTITY>` → `DV_INTERVAL`).
fn base(t: &str) -> &str {
    t.split('<').next().unwrap_or(t).trim()
}

/// Whether an instance's concrete RM type conforms to a `WebTemplate` constraint
/// type. Equal (modulo generics) or a known descendant conforms; an unknown
/// pairing is treated as conformant (permissive) to avoid over-rejecting.
#[must_use]
pub(crate) fn conforms(instance_type: &str, wt_type: &str) -> bool {
    let (inst, wt) = (base(instance_type), base(wt_type));
    if inst == wt {
        return true;
    }
    match descendants(wt) {
        Some(set) => set.contains(&inst),
        // `wt` is not a known abstract/supertype. If it is a known *concrete*
        // type, any non-matching instance (even an unknown/bogus type such as a
        // `PLACEHOLDER` root) is a violation; otherwise (an RM type this map
        // does not model) stay permissive to avoid over-rejecting.
        None => !is_known_concrete(wt),
    }
}

#[cfg(test)]
mod tests {
    use super::conforms;

    #[test]
    fn equal_and_generic() {
        assert!(conforms("DV_QUANTITY", "DV_QUANTITY"));
        assert!(conforms("DV_INTERVAL", "DV_INTERVAL<DV_QUANTITY>"));
    }

    #[test]
    fn coded_text_conforms_to_text_slot() {
        assert!(conforms("DV_CODED_TEXT", "DV_TEXT"));
        // but a plain text does not satisfy a coded-text slot
        assert!(!conforms("DV_TEXT", "DV_CODED_TEXT"));
    }

    #[test]
    fn event_and_entry_families() {
        assert!(conforms("POINT_EVENT", "EVENT"));
        assert!(conforms("OBSERVATION", "CARE_ENTRY"));
        assert!(conforms("SECTION", "CONTENT_ITEM"));
        assert!(conforms("PARTY_SELF", "PARTY_PROXY"));
    }

    #[test]
    fn confident_concrete_mismatch_is_rejected() {
        assert!(!conforms("DV_TEXT", "DV_QUANTITY"));
        assert!(!conforms("DV_QUANTITY", "DV_CODED_TEXT"));
    }

    #[test]
    fn unknown_pairing_is_permissive() {
        // A type this map does not know about is not reported as wrong.
        assert!(conforms("SOME_FUTURE_TYPE", "ANOTHER_UNKNOWN"));
    }

    #[test]
    fn bogus_type_where_concrete_expected_is_rejected() {
        // A known concrete expected type rejects an unknown/bogus instance type.
        assert!(!conforms("PLACEHOLDER", "COMPOSITION"));
        assert!(!conforms("NONSENSE", "DV_QUANTITY"));
    }
}
