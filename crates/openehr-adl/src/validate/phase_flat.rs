//! Flat-form terminology + structure checks for a **specialised** archetype.
//!
//! Four phase-1 checks are properties of the *flat* form, not the differential:
//! for a non-specialised archetype the differential *is* the flat form, so
//! [`super::phase1`] runs them directly; for a specialised archetype the flat
//! form is only available after flattening ([`crate::flatten::flat_form`]), so
//! they are deferred to this pass and run against the flattened archetype:
//!
//! * **VATDF** — every at-code used as a value in the definition must be defined
//!   in the flattened terminology (`master03` §Validity Rules; the flat form
//!   accumulates the parent's term definitions, so an inherited value code is
//!   defined).
//! * **VTVSMD** — every value-set member must be defined in the flattened
//!   terminology (`master07` §Validity Rules).
//! * **VACMCU** (error) / **WACMCL** (warning) — a container attribute's child
//!   occurrences vs its flattened cardinality (`master04.5` §`C_ATTRIBUTE`); a
//!   specialised child may not restate the inherited cardinality, so this is
//!   only decidable on the flat form.
//! * **VCOSU** — object node ids must be unique archetype-wide in the flat form
//!   (`master04.5` §`C_OBJECT`); a differential legitimately re-references an
//!   inherited id at a redefinition, so uniqueness is a flat-form property.
//!
//! Orchestration: [`super::validate`] runs this only for a specialised archetype
//! whose flat form was produced, and only after phase 1 is error-free
//! (`master08` §Overview phase gate). Non-specialised archetypes are untouched
//! here (their equivalents ran in phase 1), so no check double-fires.

use std::collections::{BTreeSet, HashMap};

use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;

use super::phase1::{CodeUsage, collect_usage, occurrences_lower};
use super::{ValidationCode, ValidationIssue};
use crate::aom::access::{aom_type, child_occurrences, complex_attributes, object_node_id};
use crate::aom::interval::finite_upper;
use crate::artefact::view;
use crate::codes::{is_at_code, is_id_code};
use crate::paths::child_path;

/// Run the deferred flat-form checks (VATDF / VTVSMD / VACMCU / WACMCL / VCOSU)
/// against the flattened archetype `flat`.
#[must_use]
pub(super) fn validate_flat_form(
    flat: &openehr_am::am24::aom2::archetype::archetype::Archetype,
) -> Vec<ValidationIssue> {
    let v = view(flat);
    let mut issues = Vec::new();

    let defined: BTreeSet<&str> = v
        .terminology
        .term_definitions
        .values()
        .flat_map(|m| m.keys().map(String::as_str))
        .collect();

    // VATDF: value at-codes used in the flattened definition must be defined.
    let mut usage = CodeUsage::default();
    let root = CObject::CComplexObject(v.definition.clone());
    collect_usage(&root, &mut usage);
    for code in &usage.value_codes {
        if is_at_code(code) && !defined.contains(code.as_str()) {
            issues.push(ValidationIssue::new(
                ValidationCode::Vatdf,
                format!("value code {code:?} is not defined in the flattened terminology"),
            ));
        }
    }

    // VTVSMD: value-set members must be defined in the flattened terminology.
    if let Some(value_sets) = v.terminology.value_sets.as_ref() {
        for set in value_sets.values() {
            for m in &set.members {
                if !defined.contains(m.as_str()) {
                    issues.push(ValidationIssue::new(
                        ValidationCode::Vtvsmd,
                        format!(
                            "value set {:?} member {m:?} is not defined in the flattened terminology",
                            set.id
                        ),
                    ));
                }
            }
        }
    }

    // VACMCU / WACMCL + VCOSU: the flat-form structural walk.
    let mut seen_node_ids: HashMap<String, String> = HashMap::new();
    walk_flat(v.definition, "", &mut seen_node_ids, &mut issues);

    issues
}

/// Walk the flat definition tree: VCOSU archetype-wide node-id uniqueness on
/// every object, VACMCU/WACMCL on every container attribute.
fn walk_flat(
    cco: &CComplexObject,
    path: &str,
    seen: &mut HashMap<String, String>,
    issues: &mut Vec<ValidationIssue>,
) {
    for attr in complex_attributes(cco) {
        let attr_path = format!("{path}/{}", attr.rm_attribute_name);
        // VACMCU / WACMCL: container cardinality vs child occurrences.
        if attr.cardinality.is_some() {
            check_container_cardinality(&attr_path, attr, issues);
        }
        for child in &attr.children {
            let cpath = child_path(&attr_path, object_node_id(child));
            check_node_id_unique(child, &cpath, seen, issues);
            if let CObject::CComplexObject(child_cco) = child {
                walk_flat(child_cco, &cpath, seen, issues);
            }
        }
    }
}

/// VCOSU: an identified (non-primitive) object node's id must be unique across
/// the flat form.
fn check_node_id_unique(
    obj: &CObject,
    path: &str,
    seen: &mut HashMap<String, String>,
    issues: &mut Vec<ValidationIssue>,
) {
    let nid = object_node_id(obj);
    if nid.is_empty() || !(is_id_code(nid) || is_at_code(nid)) {
        return;
    }
    if aom_type(obj).is_primitive() {
        return;
    }
    if let Some(first) = seen.get(nid) {
        issues.push(
            ValidationIssue::new(
                ValidationCode::Vcosu,
                format!("node id {nid:?} is not unique in the flat form (also at {first})"),
            )
            .at_path(path.to_owned()),
        );
    } else {
        seen.insert(nid.to_owned(), path.to_owned());
    }
}

/// VACMCU (error) + WACMCL (warning): container cardinality upper vs child
/// occurrences on the flat form (`master04.5` §`C_ATTRIBUTE`).
fn check_container_cardinality(
    attr_path: &str,
    attr: &CAttribute,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(card) = attr.cardinality.as_ref() else {
        return;
    };
    let Some(card_upper) = finite_upper(&card.interval) else {
        return;
    };
    let mut sum_lower = 0i64;
    for child in &attr.children {
        let Some(occ) = child_occurrences(child) else {
            continue;
        };
        if let Some(u) = finite_upper(occ)
            && i64::from(u) > i64::from(card_upper)
        {
            issues.push(
                ValidationIssue::new(
                    ValidationCode::Vacmcu,
                    format!(
                        "child occurrences upper {u} exceeds the cardinality upper {card_upper}"
                    ),
                )
                .at_path(attr_path.to_owned()),
            );
        }
        sum_lower += i64::from(occurrences_lower(occ));
    }
    if sum_lower > i64::from(card_upper) {
        issues.push(
            ValidationIssue::new(
                ValidationCode::Wacmcl,
                format!("sum of child occurrences lowers {sum_lower} exceeds the cardinality upper {card_upper}"),
            )
            .at_path(attr_path.to_owned()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::validate_flat_form;
    use crate::assemble::parse_artefact;
    use crate::validate::ValidationCode;

    fn codes(src: &str) -> Vec<ValidationCode> {
        let art = parse_artefact(src).unwrap();
        validate_flat_form(&art)
            .into_iter()
            .map(|i| i.code)
            .collect()
    }

    #[test]
    fn vtvsmd_value_set_member_not_defined_in_flat_terminology() {
        // A value-set member (`at0.5`) that is not in the flat term_definitions
        // (`master07` §Validity Rules VTVSMD): the specialised flat-form half.
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-OBSERVATION.flat_vtvsmd.v1.0.0

specialize
\topenEHR-EHR-OBSERVATION.parent.v1

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tOBSERVATION[id1.1] matches {*}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1.1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"ac0.1\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
\tvalue_sets = <
\t\t[\"ac0.1\"] = <id=<\"ac0.1\"> members=<\"at0.5\">>
\t>
";
        assert!(codes(src).contains(&ValidationCode::Vtvsmd));
    }

    #[test]
    fn vcosu_duplicate_node_id_across_flat_form() {
        // The same id-code `id2` at two non-sibling object paths in the flat form
        // (`master04.5` §C_OBJECT VCOSU — archetype-wide uniqueness).
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-OBSERVATION.flat_vcosu.v1.0.0

specialize
\topenEHR-EHR-OBSERVATION.parent.v1

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tOBSERVATION[id1.1] matches {
\t\tdata matches {
\t\t\tHISTORY[id2] occurrences matches {0..1}
\t\t}
\t\tprotocol matches {
\t\t\tITEM_TREE[id2] occurrences matches {0..1}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1.1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(codes(src).contains(&ValidationCode::Vcosu));
    }

    #[test]
    fn vacmcu_child_occurrences_exceed_flat_cardinality() {
        // A child occurrences upper (2) above the container cardinality upper (1)
        // in the flat form (`master04.5` §C_ATTRIBUTE VACMCU).
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-OBSERVATION.flat_vacmcu.v1.0.0

specialize
\topenEHR-EHR-OBSERVATION.parent.v1

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tOBSERVATION[id1.1] matches {
\t\tdata cardinality matches {0..1} matches {
\t\t\tHISTORY[id2] occurrences matches {0..2}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1.1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(codes(src).contains(&ValidationCode::Vacmcu));
    }

    #[test]
    fn vatdf_value_at_code_not_defined_in_flat_terminology() {
        // A value at-code (`at0.9`) used in a `defining_code` leaf but absent from
        // the flat term_definitions (`master03` §Validity Rules VATDF — the
        // specialised flat-form half).
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-OBSERVATION.flat_vatdf.v1.0.0

specialize
\topenEHR-EHR-OBSERVATION.parent.v1

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tOBSERVATION[id1.1] matches {
\t\tdata matches {
\t\t\tELEMENT[id2] matches {
\t\t\t\tvalue matches {
\t\t\t\t\tDV_CODED_TEXT[id3] matches {
\t\t\t\t\t\tdefining_code matches {[at0.9]}
\t\t\t\t\t}
\t\t\t\t}
\t\t\t}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1.1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id3\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(codes(src).contains(&ValidationCode::Vatdf));
    }

    #[test]
    fn clean_flat_form_raises_nothing() {
        // A well-formed flat form raises none of the deferred checks.
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-OBSERVATION.flat_ok.v1.0.0

specialize
\topenEHR-EHR-OBSERVATION.parent.v1

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tOBSERVATION[id1.1] matches {
\t\tdata cardinality matches {0..2} matches {
\t\t\tHISTORY[id2] occurrences matches {0..1}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1.1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(codes(src).is_empty());
    }
}
