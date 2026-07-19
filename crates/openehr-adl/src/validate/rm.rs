//! Reference-model (RM) validation of ADL2 archetypes.
//!
//! The checks that require "a computational representation of the reference
//! model" (`docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc` §Phase 2
//! → Validate Against Reference Model): a `C_OBJECT`'s type must exist in the
//! RM (VCORM) and conform to the type declared for its owning attribute
//! (VCORMT); an attribute name must exist on the enclosing RM type (VCARM); the
//! archetype `existence` (VCAEX), `cardinality` (VCACA) and single/multiple
//! arity (VCAM) of an attribute must conform to the RM. The single-valued-child
//! occurrences rule (VACSO, `master04.5` §Validity Rules: `C_ATTRIBUTE`) and the
//! interior-node terminology-definedness half of VATID (`master07` §Overview: a
//! term definition is optional only for children of single-valued attributes)
//! also live here because both hinge on the RM's single/multiple determination.
//!
//! The checks are generic over a [`RmModel`] so the same code validates against
//! the openEHR RM 1.2.0 ([`ProductionRmModel`], the generated
//! `openehr_rm::model`) or any other reference model the archetype declares
//! (the conformance corpus authors fixtures against openEHR's `TEST_PKG` test
//! schema; a BMM-loaded [`RmModel`] serves those). This pluggable seam realises
//! the RM-adaptation architecture of
//! `docs/specs/openehr/AM/docs/AOM2/master11-rm_adaptation.adoc`.
//!
//! Reference-model type-name matching is case-insensitive and whitespace-
//! ignored, with generic type names composed from RM class names
//! (`docs/specs/openehr/AM/docs/ADL2/master04.3-cadl_complex_types.adoc`
//! §Reference Model Type Matching); [`base_type_name`]/[`normalise_type_name`]
//! implement that lexical layer, shared by every [`RmModel`].

use std::collections::BTreeSet;

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_base::prelude::MultiplicityInterval;
use openehr_rm::model;

use super::{ArchetypeView, ValidationCode, ValidationIssue, view};
use crate::codes::{is_at_code, is_id_code};
use crate::paths::{complex_attributes, complex_rm_type, object_node_id, object_rm_type};

/// A multiplicity bound, extracted from an RM attribute or a cADL constraint.
/// `upper == None` denotes an unbounded (∞) upper limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Inclusive lower bound.
    pub lower: i32,
    /// Inclusive upper bound; `None` = unbounded.
    pub upper: Option<i32>,
}

impl Bounds {
    /// A closed `{lower..upper}` bound.
    #[must_use]
    pub fn new(lower: i32, upper: Option<i32>) -> Self {
        Self { lower, upper }
    }

    /// True if `inner` is the same as, or narrower than (wholly contained
    /// within), `self` — the "conform, i.e. be the same or narrower" test the
    /// existence (VCAEX) and cardinality (VCACA) rules require (`master04.5`
    /// §Validity Rules: `C_ATTRIBUTE`).
    #[must_use]
    pub fn contains(self, inner: Bounds) -> bool {
        inner.lower >= self.lower
            && match (self.upper, inner.upper) {
                // `self` unbounded above ⇒ any inner upper is within it.
                (None, _) => true,
                // `self` bounded but `inner` unbounded ⇒ inner escapes above.
                (Some(_), None) => false,
                (Some(outer), Some(i)) => i <= outer,
            }
    }
}

/// One attribute of an RM type, as reported by a [`RmModel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RmAttr {
    /// The declared value type of the attribute (a generic parameter resolved
    /// to its bound, e.g. `EVENT`, `ITEM_STRUCTURE`, `DV_TEXT`).
    pub declared_type: String,
    /// True if the attribute is a container (multiply-valued) in the RM.
    pub is_multiple: bool,
    /// The RM existence of the attribute (presence of the value/container
    /// itself): `{1..1}` if mandatory, else `{0..1}`.
    pub existence: Bounds,
    /// The RM cardinality of a container attribute, if the model records it;
    /// `None` for a single-valued attribute or a model that does not carry
    /// cardinality (see [`ProductionRmModel`]).
    pub cardinality: Option<Bounds>,
}

/// A reference model against which an archetype is validated — the pluggable
/// seam of `master11-rm_adaptation.adoc`. Implementors answer the four
/// questions the RM checks need: does a type exist, does one type conform to
/// another, and what are an attribute's declared type / multiplicity /
/// existence / cardinality.
///
/// Type-name arguments may be generic (`HISTORY<ITEM_LIST>`) and are matched
/// case-insensitively with whitespace ignored (`master04.3` §Reference Model
/// Type Matching); implementors should route through [`base_type_name`].
pub trait RmModel {
    /// A human-readable identity of the model (for notices/messages).
    fn name(&self) -> &str;

    /// True if `rm_type` is a type defined in the reference model.
    fn type_exists(&self, rm_type: &str) -> bool;

    /// Conformance of `sub` to `sup`: `Some(true)` if `sub` is the same type as,
    /// or a subtype of, `sup`; `Some(false)` if both types are known but `sub`
    /// does not conform; `None` if either type is unknown to the model (so the
    /// caller cannot decide — VCORM reports the unknown type instead).
    fn conforms(&self, sub: &str, sup: &str) -> Option<bool>;

    /// Resolve `attr` on `rm_type` (through inheritance), or `None` if the RM
    /// type has no such attribute (VCARM) or the type is unknown.
    fn attribute(&self, rm_type: &str, attr: &str) -> Option<RmAttr>;
}

/// The base (outer) class name of a possibly-generic RM type name, with
/// surrounding whitespace trimmed (`"Interval<Quantity>"` → `"Interval"`,
/// `"HISTORY <ITEM_LIST>"` → `"HISTORY"`), per `master04.3` §Reference Model
/// Type Matching.
#[must_use]
pub fn base_type_name(rm_type: &str) -> &str {
    rm_type.split('<').next().unwrap_or(rm_type).trim()
}

/// The generic argument type names of a generic RM type name, in order
/// (`"HISTORY<ITEM_LIST>"` → `["ITEM_LIST"]`; a non-generic name → empty).
#[must_use]
pub fn generic_arguments(rm_type: &str) -> Vec<String> {
    let Some(open) = rm_type.find('<') else {
        return Vec::new();
    };
    let Some(close) = rm_type.rfind('>') else {
        return Vec::new();
    };
    let Some(inner) = rm_type.get(open + 1..close) else {
        return Vec::new();
    };
    split_top_level_commas(inner)
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Split a generic argument list on top-level commas (not inside a nested
/// `<...>`).
fn split_top_level_commas(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in inner.chars() {
        match ch {
            '<' => {
                depth += 1;
                cur.push(ch);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                cur.push(ch);
            }
            ',' if depth == 0 => out.push(std::mem::take(&mut cur)),
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// The case-folded, whitespace-free form of an RM type name for matching
/// (`master04.3` §Reference Model Type Matching: case-insensitive, whitespace
/// ignored).
#[must_use]
pub fn normalise_type_name(rm_type: &str) -> String {
    rm_type
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_uppercase)
        .collect()
}

/// The openEHR RM 1.2.0 reference model — the generated `openehr_rm::model`
/// static attribute/type table (`crates/openehr-rm/src/model`), the same oracle
/// the AQL planner types against.
///
/// NOTE: `openehr_rm::model` records an attribute's `is_mandatory` (existence)
/// and container shape but not its RM container *cardinality* interval, so
/// [`RmModel::attribute`] reports a permissive `{0..*}` cardinality for
/// containers here; the tight lower-bound half of VCACA needs the RM cardinality
/// the generated model does not expose (candidate `emit-rm-model` gap). The
/// existence (VCAEX) and arity (VCAM) checks are exact from `is_mandatory` /
/// container shape.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProductionRmModel;

impl RmModel for ProductionRmModel {
    #[allow(clippy::unnecessary_literal_bound)] // the trait returns `&str`; the ODIN-backed model borrows a stored `String`
    fn name(&self) -> &str {
        "openEHR RM 1.2.0"
    }

    fn type_exists(&self, rm_type: &str) -> bool {
        // Only the base (outer) type is required to exist; generic arguments
        // (e.g. `ITEM_LIST` in `HISTORY<ITEM_LIST>`) may be RM classes or
        // foundation types the model excludes, so requiring them would
        // false-positive VCORM (master04.3 §Reference Model Type Matching).
        production_class(base_type_name(rm_type)).is_some()
    }

    fn conforms(&self, sub: &str, sup: &str) -> Option<bool> {
        let child = production_class(base_type_name(sub))?;
        let ancestor = production_class(base_type_name(sup))?;
        Some(model::is_a(child.name, ancestor.name))
    }

    fn attribute(&self, rm_type: &str, attr: &str) -> Option<RmAttr> {
        let class = production_class(base_type_name(rm_type))?;
        let a = model::attribute(class.name, attr)?;
        let is_multiple = a.container != model::Container::None;
        Some(RmAttr {
            declared_type: a.declared_type.to_owned(),
            is_multiple,
            existence: Bounds::new(i32::from(a.is_mandatory), Some(1)),
            // See the type NOTE: the generated model has no cardinality; report
            // the permissive RM container default so VCACA never false-fires.
            cardinality: is_multiple.then(|| Bounds::new(0, None)),
        })
    }
}

/// Look up a class in the generated model by its normalised (case-insensitive)
/// name. The generated table keys are the exact spec class names (upper-case
/// `SCREAMING_SNAKE`), so an upper-cased lookup matches them.
fn production_class(base: &str) -> Option<&'static model::RmClass> {
    // Fast path: exact spec name; fallback: case-fold to match `Section` etc.
    model::class(base).or_else(|| model::class(&base.to_uppercase()))
}

/// Whether the openEHR [`ProductionRmModel`] governs `archetype` — decided from
/// the HRID `rm_publisher`/`rm_package` (a lower-cased `openehr` publisher with
/// a package that is not a test/foreign schema). Archetypes whose publisher or
/// package name a model this build does not carry are not RM-checked by the
/// production model (the caller supplies the appropriate [`RmModel`], or skips —
/// see [`validate_source`]).
#[must_use]
pub fn production_model_governs(archetype: &Archetype) -> bool {
    production_model_governs_view(&view(archetype))
}

fn production_model_governs_view(v: &ArchetypeView<'_>) -> bool {
    let publisher = v.archetype_id.rm_publisher.to_ascii_lowercase();
    let package = v.archetype_id.rm_package.to_ascii_uppercase();
    // The openEHR RM 1.2.0 packages the generated model carries. `TEST_PKG` (the
    // AOM2 test schema) and the component packages not compiled into this build
    // (TASK_PLANNING, foreign publishers) are excluded.
    publisher == "openehr" && !matches!(package.as_str(), "TEST_PKG" | "TASK_PLANNING")
}

/// Validate `archetype` against `rm` — the reference-model checks (VCORM,
/// VCARM, VCORMT, VCAEX, VCACA, VCAM), plus the RM-dependent VACSO and the
/// interior-node half of VATID.
///
/// These are the `master08` §Phase 2 → Validate Against Reference Model checks.
/// Phase gating (they run only when phase-1 basic integrity passed) is applied
/// by [`validate_source`] / [`super::validate`]; called directly, they run
/// unconditionally.
#[must_use]
pub fn validate_phase2_rm(archetype: &Archetype, rm: &dyn RmModel) -> Vec<ValidationIssue> {
    let v = view(archetype);
    let defined: BTreeSet<String> = v
        .terminology
        .term_definitions
        .values()
        .flat_map(|m| m.keys().cloned())
        .collect();
    let mut scan = RmScan {
        rm,
        defined,
        is_specialised: v.is_specialised(),
        issues: Vec::new(),
    };
    let root_type = complex_rm_type(v.definition);
    // VCORM on the root object type (master04.5 §Validity Rules: `C_OBJECT`).
    if !root_type.is_empty() && !rm.type_exists(root_type) {
        scan.push(
            ValidationCode::Vcorm,
            format!(
                "root object type {root_type:?} is not defined in the reference model ({})",
                rm.name()
            ),
            "/",
        );
    }
    scan.walk_complex("", root_type, v.definition);
    scan.issues
}

/// Mutable state threaded through the reference-model walk.
struct RmScan<'a> {
    rm: &'a dyn RmModel,
    /// The union of terminology-defined codes (for the interior-node VATID
    /// half); a code defined in any language counts as defined.
    defined: BTreeSet<String>,
    is_specialised: bool,
    issues: Vec<ValidationIssue>,
}

impl RmScan<'_> {
    fn push(&mut self, code: ValidationCode, msg: impl Into<String>, path: &str) {
        self.issues
            .push(ValidationIssue::new(code, msg).at_path(path.to_owned()));
    }

    /// Walk a complex object whose RM type is `rm_type`, checking every
    /// attribute and its child objects against the reference model.
    fn walk_complex(&mut self, path: &str, rm_type: &str, cco: &CComplexObject) {
        for attr in complex_attributes(cco) {
            // A differential-path attribute does not introduce an attribute
            // block on the enclosing object's RM type — it relocates the
            // constraint to a node elsewhere in the flat parent (master04.5
            // §C_ATTRIBUTE, VDIFP). Its RM validity is checked at the resolved
            // location by the phase-2 specialisation walk, so VCARM/VCAEX/… do
            // not apply against `rm_type` here.
            // TODO: check the differential path's RM-path validity (the "valid
            // with respect to the reference model" half of VDIFP) once the flat
            // form is built.
            if attr.differential_path.is_some() {
                continue;
            }
            let attr_path = format!("{path}/{}", attr.rm_attribute_name);
            let rm_attr = if rm_type.is_empty() {
                None
            } else {
                self.rm.attribute(rm_type, &attr.rm_attribute_name)
            };

            match rm_attr {
                None => {
                    // VCARM: the attribute must be defined in the RM as an
                    // attribute of the enclosing type (master04.5 §Validity
                    // Rules: `C_ATTRIBUTE`). Only reportable when the enclosing RM
                    // type is itself known; an unknown enclosing type is already
                    // VCORM/VCORMT at the level above.
                    if !rm_type.is_empty() && self.rm.type_exists(rm_type) {
                        self.push(
                            ValidationCode::Vcarm,
                            format!(
                                "attribute {:?} is not defined on reference-model type {rm_type:?}",
                                attr.rm_attribute_name
                            ),
                            &attr_path,
                        );
                    }
                    // The declared type is unknown, so no VCORMT/VCAEX/etc for
                    // this attribute; still descend for VCORM on the subtree.
                    self.walk_children_untyped(&attr_path, attr);
                }
                Some(rm_attr) => {
                    self.check_attribute(&attr_path, attr, &rm_attr);
                    self.walk_children(&attr_path, attr, &rm_attr);
                }
            }
        }
    }

    /// The attribute-level RM checks (VCAEX / VCAM / VCACA / VACSO).
    fn check_attribute(&mut self, attr_path: &str, attr: &CAttribute, rm_attr: &RmAttr) {
        // VCAEX: existence, if set, must conform to (be same-or-narrower than)
        // the RM existence (master04.5 §Validity Rules: `C_ATTRIBUTE`).
        if let Some(ex) = attr.existence.as_ref() {
            let arch = bounds_of_multiplicity(ex);
            if !rm_attr.existence.contains(arch) {
                self.push(
                    ValidationCode::Vcaex,
                    format!(
                        "existence {} does not conform to the reference-model existence {}",
                        display_bounds(arch),
                        display_bounds(rm_attr.existence)
                    ),
                    attr_path,
                );
            }
        }

        let has_cardinality = attr.cardinality.is_some();

        // VCAM: single/multiple arity must match the RM. A cardinality declares
        // the attribute a container; if the RM attribute is single-valued that
        // is a mismatch (master04.5 §Validity Rules: `C_ATTRIBUTE`, VCAM).
        if has_cardinality && !rm_attr.is_multiple {
            self.push(
                ValidationCode::Vcam,
                "a cardinality is stated but the reference-model attribute is single-valued",
                attr_path,
            );
        }

        // VCACA: cardinality must conform to the RM container cardinality
        // (master04.5 §Validity Rules: `C_ATTRIBUTE`). Only meaningful when the RM
        // attribute is a container; a cardinality on a single-valued RM
        // attribute is already VCAM above.
        if let (Some(card), true) = (attr.cardinality.as_ref(), rm_attr.is_multiple)
            && let Some(rm_card) = rm_attr.cardinality
        {
            let arch = bounds_of_multiplicity(&card.interval);
            if !rm_card.contains(arch) {
                self.push(
                    ValidationCode::Vcaca,
                    format!(
                        "cardinality {} does not conform to the reference-model cardinality {}",
                        display_bounds(arch),
                        display_bounds(rm_card)
                    ),
                    attr_path,
                );
            }
        }

        // VACSO: the occurrences of a child object of a single-valued attribute
        // cannot have an upper limit greater than 1 (master04.5 §Validity Rules:
        // `C_ATTRIBUTE` — the rules "for single-valued attributes, i.e. when
        // `C_ATTRIBUTE`._is_multiple_ is False"). The single/multiple
        // determination is the RM's, not the parser's cardinality heuristic.
        if !rm_attr.is_multiple {
            for child in &attr.children {
                if let Some(occ) = object_occurrences(child)
                    && let Some(upper) = finite_upper(occ)
                    && upper > 1
                {
                    self.push(
                        ValidationCode::Vacso,
                        format!(
                            "child of single-valued attribute has occurrences upper {upper} > 1"
                        ),
                        attr_path,
                    );
                }
            }
        }
    }

    /// Walk the children of an attribute whose RM declared type is known,
    /// checking each child's type existence/conformance and recursing.
    fn walk_children(&mut self, attr_path: &str, attr: &CAttribute, rm_attr: &RmAttr) {
        for child in &attr.children {
            let child_type = object_rm_type(child);
            let child_path = child_path(attr_path, object_node_id(child));

            if !child_type.is_empty() {
                if !self.rm.type_exists(child_type) {
                    // VCORM: the object type must exist in the RM.
                    self.push(
                        ValidationCode::Vcorm,
                        format!(
                            "object type {child_type:?} is not defined in the reference model ({})",
                            self.rm.name()
                        ),
                        &child_path,
                    );
                } else if self.rm.conforms(child_type, &rm_attr.declared_type) == Some(false) {
                    // VCORMT: the object type must be the same as, or conform to,
                    // the type declared for the owning attribute in the RM
                    // (master04.5 §Validity Rules: `C_OBJECT`, VCORMT).
                    self.push(
                        ValidationCode::Vcormt,
                        format!(
                            "object type {child_type:?} does not conform to the attribute's reference-model type {:?}",
                            rm_attr.declared_type
                        ),
                        &child_path,
                    );
                }
            }

            // Interior-node VATID: a node under a multiply-valued attribute must
            // have its id-code defined in the terminology; for a single-valued
            // attribute a term definition is optional (master07
            // §Overview). The flattened terminology of a specialised archetype
            // is not available here, so this runs on the archetype's own
            // terminology only.
            if rm_attr.is_multiple && !self.is_specialised {
                let nid = object_node_id(child);
                if (is_id_code(nid) || is_at_code(nid)) && !self.defined.contains(nid) {
                    self.push(
                        ValidationCode::Vatid,
                        format!("node id {nid:?} is not defined in the terminology"),
                        &child_path,
                    );
                }
            }

            if let CObject::CComplexObject(cco) = child {
                self.walk_complex(&child_path, child_type, cco);
            }
        }
    }

    /// Walk the children of an attribute whose declared type is unknown (its
    /// name was not defined in the RM — VCARM). No conformance can be judged, so
    /// only VCORM (type existence) is checked, and the subtree is recursed with
    /// each child's own RM type.
    fn walk_children_untyped(&mut self, attr_path: &str, attr: &CAttribute) {
        for child in &attr.children {
            let child_type = object_rm_type(child);
            let child_path = child_path(attr_path, object_node_id(child));
            if !child_type.is_empty() && !self.rm.type_exists(child_type) {
                self.push(
                    ValidationCode::Vcorm,
                    format!(
                        "object type {child_type:?} is not defined in the reference model ({})",
                        self.rm.name()
                    ),
                    &child_path,
                );
            }
            if let CObject::CComplexObject(cco) = child {
                self.walk_complex(&child_path, child_type, cco);
            }
        }
    }
}

/// `Bounds` from a [`MultiplicityInterval`] (existence / occurrences).
fn bounds_of_multiplicity(mi: &MultiplicityInterval) -> Bounds {
    Bounds {
        lower: if mi.lower_unbounded {
            0
        } else {
            mi.lower.unwrap_or(0)
        },
        upper: if mi.upper_unbounded { None } else { mi.upper },
    }
}

/// The occurrences interval of any [`CObject`], if it carries one.
fn object_occurrences(obj: &CObject) -> Option<&MultiplicityInterval> {
    match obj {
        CObject::ArchetypeSlot(s) => s.occurrences.as_ref(),
        CObject::CComplexObject(c) => match c {
            CComplexObject::CComplexObject(d) => d.occurrences.as_ref(),
            CComplexObject::CArchetypeRoot(r) => r.occurrences.as_ref(),
        },
        CObject::CComplexObjectProxy(p) => p.occurrences.as_ref(),
        CObject::CBoolean(o) => o.occurrences.as_ref(),
        CObject::CInteger(o) => o.occurrences.as_ref(),
        CObject::CReal(o) => o.occurrences.as_ref(),
        CObject::CString(o) => o.occurrences.as_ref(),
        CObject::CTerminologyCode(o) => o.occurrences.as_ref(),
        CObject::CDate(o) => o.occurrences.as_ref(),
        CObject::CTime(o) => o.occurrences.as_ref(),
        CObject::CDateTime(o) => o.occurrences.as_ref(),
        CObject::CDuration(o) => o.occurrences.as_ref(),
    }
}

/// The finite upper bound of a multiplicity interval, or `None` if unbounded.
fn finite_upper(mi: &MultiplicityInterval) -> Option<i32> {
    if mi.upper_unbounded { None } else { mi.upper }
}

fn child_path(attr_path: &str, node_id: &str) -> String {
    if node_id.is_empty() {
        attr_path.to_owned()
    } else {
        format!("{attr_path}[{node_id}]")
    }
}

fn display_bounds(b: Bounds) -> String {
    match b.upper {
        Some(u) if u == b.lower => format!("{{{}}}", b.lower),
        Some(u) => format!("{{{}..{u}}}", b.lower),
        None => format!("{{{}..*}}", b.lower),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::assemble::parse_artefact;

    #[test]
    fn base_type_and_generics() {
        assert_eq!(base_type_name("HISTORY<ITEM_LIST>"), "HISTORY");
        assert_eq!(base_type_name("Interval <Quantity>"), "Interval");
        assert_eq!(base_type_name("OBSERVATION"), "OBSERVATION");
        assert_eq!(generic_arguments("HISTORY<ITEM_LIST>"), vec!["ITEM_LIST"]);
        assert_eq!(
            generic_arguments("Map<String, List<Item>>"),
            vec!["String", "List<Item>"]
        );
        assert!(generic_arguments("OBSERVATION").is_empty());
        assert_eq!(
            normalise_type_name("Interval <Quantity>"),
            "INTERVAL<QUANTITY>"
        );
    }

    #[test]
    fn bounds_containment_is_same_or_narrower() {
        let one_to_one = Bounds::new(1, Some(1));
        assert!(one_to_one.contains(Bounds::new(1, Some(1))));
        assert!(!one_to_one.contains(Bounds::new(0, Some(0)))); // {0} not within {1..1}
        let star = Bounds::new(0, None);
        assert!(star.contains(Bounds::new(1, Some(5))));
        assert!(!Bounds::new(1, Some(5)).contains(star)); // {0..*} escapes {1..5}
    }

    #[test]
    fn production_model_selection() {
        let ehr = parse_artefact(
            "archetype (adl_version=2.0.5; rm_release=1.0.2)\n\
             \topenEHR-EHR-OBSERVATION.x.v1.0.0\n\n\
             language\n\toriginal_language = <[ISO_639-1::en]>\n\n\
             description\n\tlifecycle_state = <\"draft\">\n\n\
             definition\n\tOBSERVATION[id1] matches {*}\n\n\
             terminology\n\tterm_definitions = <[\"en\"] = <[\"id1\"] = <text=<\"\"> description=<\"\">>>>\n",
        )
        .unwrap();
        assert!(production_model_governs(&ehr));
    }
}
