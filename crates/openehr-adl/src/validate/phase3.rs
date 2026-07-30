//! Phase-3 validation: checks performed on the FLAT form after flattening.
//!
//! Orchestration follows
//! `docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc` §Phase 3 -
//! Validation of Flat Form ("carried out after successful generation of the flat
//! form"):
//!
//! * VUNP — every `C_COMPLEX_OBJECT_PROXY` (`use_node`) target path must resolve
//!   to an object node in the flat form (`master04.5`
//!   §`C_COMPLEX_OBJECT_PROXY`, VUNP L482-483).
//! * VACMCO — every object node's occurrences must be satisfiable within its
//!   enclosing attribute's cardinality (`master04.5` §`C_ATTRIBUTE`, VACMCO
//!   L158-159).
//!
//! These run on the flat form ([`crate::flatten::flat_form`]) because a proxy
//! target or a container's full child set may be assembled from several
//! specialisation levels.

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;

use super::{ValidationCode, ValidationIssue, view};
use crate::paths::{Resolution, complex_attributes, object_node_id, resolve};

/// Validate the FLAT form `flat` against the phase-3 catalogue (VUNP, VACMCO).
#[must_use]
pub(super) fn validate_phase3(flat: &Archetype) -> Vec<ValidationIssue> {
    let v = view(flat);
    let mut scan = Phase3 {
        root: v.definition,
        issues: Vec::new(),
    };
    scan.walk(v.definition, "");
    scan.issues
}

struct Phase3<'a> {
    root: &'a CComplexObject,
    issues: Vec<ValidationIssue>,
}

impl Phase3<'_> {
    fn walk(&mut self, node: &CComplexObject, path: &str) {
        for attr in complex_attributes(node) {
            let attr_path = format!("{path}/{}", attr.rm_attribute_name);
            self.check_cardinality_occurrences(attr, &attr_path);
            for child in &attr.children {
                let child_path = child_path(&attr_path, object_node_id(child));
                match child {
                    CObject::CComplexObjectProxy(proxy) => {
                        self.check_proxy(&proxy.target_path, &child_path);
                    }
                    CObject::CComplexObject(CComplexObject::CComplexObject(_)) => {
                        if let CObject::CComplexObject(cco) = child {
                            self.walk(cco, &child_path);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// VUNP: the proxy target path must resolve to an object node in the flat
    /// form (`master04.5` §`C_COMPLEX_OBJECT_PROXY`, VUNP L482-483).
    fn check_proxy(&mut self, target_path: &str, path: &str) {
        if target_path.is_empty() {
            self.issues.push(
                ValidationIssue::new(ValidationCode::Vunp, "use_node proxy has no target path")
                    .at_path(path.to_owned()),
            );
            return;
        }
        if resolve(self.root, target_path) != Resolution::Found {
            self.issues.push(
                ValidationIssue::new(
                    ValidationCode::Vunp,
                    format!(
                        "use_node target path {target_path:?} does not resolve to an object node in the flat form"
                    ),
                )
                .at_path(path.to_owned()),
            );
        }
    }

    /// VACMCO: for a container attribute with a finite cardinality upper bound, it
    /// must be possible to include one instance of every mandatory child (sum of
    /// stated occurrences lower bounds) plus one instance of one optional child
    /// within the cardinality range (`master04.5` §`C_ATTRIBUTE`, VACMCO
    /// L158-159).
    fn check_cardinality_occurrences(&mut self, attr: &CAttribute, path: &str) {
        let Some(card) = attr.cardinality.as_ref() else {
            return;
        };
        if card.interval.upper_unbounded {
            return;
        }
        let Some(upper) = card.interval.upper else {
            return;
        };
        let mut mandatory_floor: i64 = 0;
        let mut has_optional = false;
        for child in &attr.children {
            let Some(occ) = crate::validate::conformance::child_occurrences(child) else {
                continue;
            };
            let lower = if occ.lower_unbounded {
                0
            } else {
                occ.lower.unwrap_or(0)
            };
            if lower == 0 {
                has_optional = true;
            } else {
                mandatory_floor += i64::from(lower);
            }
        }
        let needed = mandatory_floor + i64::from(has_optional);
        if needed > i64::from(upper) {
            self.issues.push(
                ValidationIssue::new(
                    ValidationCode::Vacmco,
                    format!(
                        "the minimum required occurrences ({needed}) of the child objects cannot fit within the cardinality upper bound ({upper})"
                    ),
                )
                .at_path(path.to_owned()),
            );
        }
    }
}

fn child_path(attr_path: &str, node_id: &str) -> String {
    if node_id.is_empty() {
        attr_path.to_owned()
    } else {
        format!("{attr_path}[{node_id}]")
    }
}

#[cfg(test)]
mod tests {
    use super::validate_phase3;
    use crate::assemble::parse_artefact;
    use crate::validate::ValidationCode;

    fn codes(src: &str) -> Vec<ValidationCode> {
        let art = parse_artefact(src).unwrap();
        validate_phase3(&art).into_iter().map(|i| i.code).collect()
    }

    #[test]
    fn vacmco_orphan_mandatory_children_exceed_cardinality() {
        // items cardinality {0..2} but three mandatory children (occurrences {1}
        // each): the minimum 3 cannot fit in 2 (`master04.5` §`C_ATTRIBUTE`
        // VACMCO L158-159).
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-CLUSTER.vacmco.v1.0.0

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tCLUSTER[id1] matches {
\t\titems cardinality matches {0..2} matches {
\t\t\tELEMENT[id2] occurrences matches {1}
\t\t\tELEMENT[id3] occurrences matches {1}
\t\t\tELEMENT[id4] occurrences matches {1}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id3\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id4\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(codes(src).contains(&ValidationCode::Vacmco));
    }

    #[test]
    fn vacmco_clean_when_children_fit() {
        // items cardinality {0..3}, three mandatory children: 3 <= 3, clean.
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-CLUSTER.vacmco_ok.v1.0.0

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tCLUSTER[id1] matches {
\t\titems cardinality matches {0..3} matches {
\t\t\tELEMENT[id2] occurrences matches {1}
\t\t\tELEMENT[id3] occurrences matches {1}
\t\t\tELEMENT[id4] occurrences matches {1}
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id3\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id4\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(!codes(src).contains(&ValidationCode::Vacmco));
    }

    #[test]
    fn vunp_use_node_path_does_not_resolve() {
        // A use_node whose target path points at a non-existent node
        // (`master04.5` §`C_COMPLEX_OBJECT_PROXY` VUNP L482-483).
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-CLUSTER.vunp.v1.0.0

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tCLUSTER[id1] matches {
\t\titems matches {
\t\t\tELEMENT[id2]
\t\t\tuse_node ELEMENT[id5] /items[id99]
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id5\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(codes(src).contains(&ValidationCode::Vunp));
    }

    #[test]
    fn vunp_clean_when_use_node_path_resolves() {
        // A use_node whose target path resolves to a real object node — clean.
        let src = "\
archetype (adl_version=2.0.5; rm_release=1.0.2)
\topenEHR-EHR-CLUSTER.vunp_ok.v1.0.0

language
\toriginal_language = <[ISO_639-1::en]>

description
\tlifecycle_state = <\"draft\">

definition
\tCLUSTER[id1] matches {
\t\titems matches {
\t\t\tELEMENT[id2]
\t\t\tuse_node ELEMENT[id5] /items[id2]
\t\t}
\t}

terminology
\tterm_definitions = <
\t\t[\"en\"] = <
\t\t\t[\"id1\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id2\"] = <text=<\"\"> description=<\"\">>
\t\t\t[\"id5\"] = <text=<\"\"> description=<\"\">>
\t\t>
\t>
";
        assert!(!codes(src).contains(&ValidationCode::Vunp));
    }
}
