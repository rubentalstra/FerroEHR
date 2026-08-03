//! Reference-model conformance for the OPT 1.4 pass (T5, T7 + the RM checks).
//!
//! The AOM2 RM-conformance rules (`AOM2/master08-validation.adoc` lines 70–75,
//! `AOM2/master04.5-constraint_model-class_definitions.adoc`) require "a
//! computational representation of the reference model"; we use the
//! BMM-generated static RM model (`openehr_rm::model`) — the same spec-pinned
//! oracle the AQL planner uses. Rules:
//!
//! - **VCORM** — an object-constraint type name must exist in the RM
//!   (`master04.5` line 325).
//! - **VCARM** — a constrained attribute name must exist on its RM type
//!   (`master08` line 126).
//! - **VCAM** — a container constraint may not sit on a single-valued RM
//!   attribute (`master08` line 132).
//! - **VCAEX** — an attribute's existence, if set, must not widen the RM's
//!   (`master08` line 129).
//! - **VCACA** — a container attribute's cardinality must be the same as, or
//!   narrower than, the RM's (`master08` line 74, `master04.5` line 162).
//! - **VACMCO / VCOC** — the mandatory children must fit the container
//!   cardinality (`master04.5` line 159, restating cADL VCOC
//!   `ADL1.4/master05-cadl.adoc` line 324).

use std::collections::BTreeSet;
use std::sync::LazyLock;

use openehr_its::opt14::{CAttribute, CObject, Cardinality, Intervalofinteger};
use openehr_rm::model;

use super::interval::{iv_lower, iv_upper};
use super::{NodeView, RuleViolation};

/// LOCATABLE meta attributes tolerated on any RM class (see the NOTE in
/// [`check_attribute`]: archie-era OPTs constrain these on PATHABLE-only
/// classes) — the class's OWN attribute set, read from the BMM-generated static
/// RM model, never a hand-kept list: a LOCATABLE attribute added or renamed by
/// a spec-pin bump follows automatically. PATHABLE's inherited members
/// (`parent`, which the flattened model also reports) are excluded, since the
/// tolerance exists precisely for classes the RM derives from PATHABLE alone.
static LOCATABLE_META_ATTRS: LazyLock<BTreeSet<&'static str>> = LazyLock::new(|| {
    let pathable: BTreeSet<&'static str> = model::attributes("PATHABLE").map(|a| a.name).collect();
    model::attributes("LOCATABLE")
        .map(|a| a.name)
        .filter(|name| !pathable.contains(name))
        .collect()
});

/// Legacy `(class, attribute)` pairs tolerated for prior-art OPT compatibility
/// (NOTE): `ELEMENT.null_flavor` is the archetype-tooling (US) spelling of
/// RM `null_flavour` (`org.openehr.rm.data_structures` ELEMENT), and
/// `ITEM_TABLE.rotated` is an RM 1.0.x attribute removed from later RM
/// releases — both appear in widely-deployed OPT 1.4 artifacts (the vendored
/// RIPPLE / `clinical_content` corpus templates).
const LEGACY_RM_ATTRS: &[(&str, &str)] = &[
    ("ELEMENT", "null_flavor"),
    ("ITEM_TABLE", "rotated"),
    // EVENT.offset is a *computed* function in current RM (Iso8601_duration,
    // org.openehr.rm.data_structures event classes) — RM 1.0.x-era tooling
    // emitted it as a constrainable stored attribute.
    ("EVENT", "offset"),
    ("POINT_EVENT", "offset"),
    ("INTERVAL_EVENT", "offset"),
    // DV_PROPORTION.is_integral is a *computed* function in current RM
    // (Boolean, org.openehr.rm.data_types dv_proportion) — RM 1.0.x-era
    // tooling emitted it as a constrainable stored attribute (the vendored
    // Better corpus templates).
    ("DV_PROPORTION", "is_integral"),
];

// ─── VCORM (object constraint type existence) ───────────────────────────────────

/// VCORM: "object constraint type name existence: a type name introducing an
/// object constraint block must be defined in the underlying information model."
/// (`AOM2/master04.5-…class_definitions.adoc` line 325.)
pub(super) fn check_object_type(rm_type: &str, node_id: &str) -> Result<(), RuleViolation> {
    // Strip any generic argument (`DV_INTERVAL<DV_QUANTITY>` → `DV_INTERVAL`);
    // the static model keys on the bare class name.
    let bare = rm_type.split('<').next().unwrap_or(rm_type).trim();
    if bare.is_empty() {
        return Err(RuleViolation::new(
            "VCORM",
            format!("object node '{node_id}' has an empty rm_type_name"),
        ));
    }
    if model::class(bare).is_none() {
        return Err(RuleViolation::new(
            "VCORM",
            format!(
                "type '{rm_type}' (object node '{node_id}') is not defined in the reference model"
            ),
        ));
    }
    Ok(())
}

// ─── VCARM / VCAM / VCAEX / VCACA (attribute conformance) ────────────────────────

/// The RM-conformance checks on a constrained attribute: VCARM (the attribute
/// exists on the RM type), and — once the RM attribute is resolved — VCAM
/// (multiplicity) and VCAEX (existence). Fires only when the enclosing object's
/// RM type is known to the static model; an unknown parent means we cannot
/// judge its attributes (and VCORM already flagged the parent if it was bogus).
pub(super) fn check_attribute(
    attr: &CAttribute,
    attr_name: &str,
    parent_rm: &str,
    existence: &Intervalofinteger,
) -> Result<(), RuleViolation> {
    if model::class(parent_rm).is_none() {
        return Ok(());
    }
    match model::attribute(parent_rm, attr_name) {
        // NOTE (prior-art OPT tolerance): archie/openEHR-SDK tooling
        // models every constrainable node as Locatable, so published OPTs
        // (incl. the vendored IPS template) constrain LOCATABLE meta attributes
        // (`name`, `archetype_node_id`, …) on classes the RM derives from
        // PATHABLE only (e.g. ISM_TRANSITION —
        // org.openehr.rm.composition.ism_transition.adoc inherits PATHABLE).
        // Rejecting them per strict VCARM would refuse real-world templates; the
        // constraints are tolerated (they bind to the serialized meta fields,
        // which canonical JSON carries).
        None if LOCATABLE_META_ATTRS.contains(attr_name)
            || LEGACY_RM_ATTRS.contains(&(parent_rm, attr_name)) =>
        {
            Ok(())
        }
        None => Err(RuleViolation::new(
            // VCARM: attribute name reference model validity (AOM2 line 126).
            "VCARM",
            format!("attribute '{attr_name}' is not defined in reference-model type '{parent_rm}'"),
        )),
        Some(rm_attr) => rm_conformance(attr, attr_name, parent_rm, existence, rm_attr),
    }
}

/// The AOM2 RM-conformance rules that apply to a `C_ATTRIBUTE` once its RM
/// attribute has been resolved: VCAM (multiplicity) and VCAEX (existence).
fn rm_conformance(
    attr: &CAttribute,
    attr_name: &str,
    parent_rm: &str,
    existence: &Intervalofinteger,
    rm_attr: &model::RmAttribute,
) -> Result<(), RuleViolation> {
    let rm_is_multiple = !matches!(rm_attr.container, model::Container::None);

    // VCAM: "archetype attribute reference model multiplicity conformance: the
    // multiplicity … of an attribute must conform to that of the corresponding
    // attribute in the underlying information model." (line 132.) A container
    // (`C_MULTIPLE_ATTRIBUTE`) constraint on a single-valued RM attribute cannot
    // conform.
    let arch_is_multiple = matches!(attr, CAttribute::CMultipleAttribute(_));
    if arch_is_multiple && !rm_is_multiple {
        return Err(RuleViolation::new(
            "VCAM",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' is constrained as a container \
                 (C_MULTIPLE_ATTRIBUTE) but is single-valued in the reference model"
            ),
        ));
    }

    // VCAEX: "archetype attribute reference model existence conformance: the
    // existence of an attribute, if set, must conform, i.e. be the same or
    // narrower, to the existence … in the underlying information model."
    // (line 129.) The RM existence upper bound is always 1; the RM lower bound
    // is 1 for a mandatory attribute and 0 otherwise. Allowing absence (`{0..}`)
    // on an RM-mandatory attribute *widens* it — the one enforceable violation.
    if rm_attr.is_mandatory && iv_lower(existence) == 0 && !existence.lower_unbounded {
        return Err(RuleViolation::new(
            "VCAEX",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' has existence lower bound 0 but the \
                 attribute is mandatory (existence lower bound 1) in the reference model"
            ),
        ));
    }

    // VCACA: "archetype attribute reference model cardinality conformance: the
    // cardinality of an attribute must conform, i.e. be the same or narrower, to
    // the cardinality of the corresponding attribute in the underlying
    // information model." (line 162.)
    if let CAttribute::CMultipleAttribute(multiple) = attr {
        check_rm_cardinality(&multiple.cardinality, attr_name, parent_rm, rm_attr)?;
    }

    Ok(())
}

/// VCACA's numeric arm: the archetype cardinality interval must be CONTAINED in
/// the RM's declared container cardinality — "the same or narrower"
/// (`AOM2/master04.5-…class_definitions.adoc` line 162).
///
/// The RM bounds come from the BMM-generated static model
/// ([`openehr_rm::model::RmAttribute::cardinality`], the BMM `cardinality` of a
/// container attribute); an attribute the BMM leaves unconstrained has no RM
/// interval to conform to and is skipped. Containment is the ordinary interval
/// rule in both directions:
///
/// - the archetype LOWER bound may not fall below the RM's — e.g. `CLUSTER.items`
///   is `List<ITEM> [1..*]`
///   (`RM/docs/UML/classes/org.openehr.rm.data_structures.cluster.adoc`
///   §Attributes), so an OPT stating `items cardinality {0..*}` widens the RM by
///   admitting an empty CLUSTER the RM forbids;
/// - the archetype UPPER bound may not exceed the RM's (an unbounded archetype
///   upper against a finite RM upper is the widest case of that).
///
/// The fully-open `{0..*}` is read as "no cardinality override" rather than as
/// a widening — see the in-body citations.
///
/// NOTE: this is the *numeric* part only. The `ordered`/`unordered`/`unique`
/// half of a cADL cardinality is deliberately NOT judged here: that is exactly
/// what `ADL1.4/master05-cadl.adoc` line 268 hedges ("developers often use lists
/// to facilitate integration, when the actual semantics are intended to be of a
/// set … How such constraints are evaluated in practice may depend somewhat on
/// knowledge of the software system"), and the hedge is about container
/// SEMANTICS, not about the membership range the same paragraph calls a plain
/// constraint.
fn check_rm_cardinality(
    card: &Cardinality,
    attr_name: &str,
    parent_rm: &str,
    rm_attr: &model::RmAttribute,
) -> Result<(), RuleViolation> {
    let Some(rm_card) = rm_attr.cardinality else {
        return Ok(());
    };
    // The fully-open interval states NO cardinality override and defers to the
    // RM, so it can never widen it. Two spec facts force that reading of an
    // OPT 1.4 artefact:
    //
    // - AOM2 types the field `C_ATTRIBUTE.cardinality [0..1]` and gives the
    //   governing principle on its sibling `existence`: "Only set if it
    //   overrides the underlying reference model or parent archetype"
    //   (`AM/docs/UML/classes/org.openehr.am.aom2.c_attribute.adoc`
    //   §Attributes). AOM **1.4** types it `C_MULTIPLE_ATTRIBUTE.cardinality
    //   [1..1]` — MANDATORY
    //   (`…org.openehr.am.aom14.c_multiple_attribute.adoc` §Attributes) — so an
    //   OPT 1.4 has no way to say "not overridden"; it must write an interval
    //   for every container attribute.
    // - cADL spells the open range `{*}`, "a single `*` means the range
    //   `0..*`", and §"'Any' Constraints" fixes what an open constraint means:
    //   "any value permitted by the underlying information model is also
    //   permitted by the archetype" (`AM/docs/ADL1.4/master05-cadl.adoc`) —
    //   deference to the RM, not a widening of it.
    //
    // Every STATED interval (a finite bound on either side) is judged below.
    if iv_lower(&card.interval) == 0 && iv_upper(&card.interval).is_none() {
        return Ok(());
    }
    let arch_lower = iv_lower(&card.interval);
    let widens_lower = i64::from(arch_lower) < i64::from(rm_card.lower);
    let widens_upper = match (iv_upper(&card.interval), rm_card.upper) {
        // An RM-unbounded upper cannot be exceeded.
        (_, None) => false,
        // An unbounded archetype upper against a finite RM upper widens it.
        (None, Some(_)) => true,
        (Some(arch_upper), Some(rm_upper)) => i64::from(arch_upper) > i64::from(rm_upper),
    };
    if widens_lower || widens_upper {
        let rm_upper = rm_card
            .upper
            .map_or_else(|| "*".to_owned(), |u| u.to_string());
        let arch_upper = iv_upper(&card.interval).map_or_else(|| "*".to_owned(), |u| u.to_string());
        return Err(RuleViolation::new(
            "VCACA",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' has cardinality \
                 {{{arch_lower}..{arch_upper}}}, which is wider than the reference \
                 model's {{{}..{rm_upper}}} — an archetype cardinality must be the \
                 same or narrower",
                rm_card.lower
            ),
        ));
    }
    Ok(())
}

// ─── VACMCO / VCOC (occurrences vs cardinality) ─────────────────────────────────

/// VCOC / VACMCO: "it must be possible for … one instance of every mandatory
/// child object … to be included within the cardinality range."
/// (`AOM2/…class_definitions.adoc` line 159, restating cADL VCOC,
/// `ADL1.4/master05-cadl.adoc` line 324.) The sum of the children's occurrence
/// *lower* bounds is the count that MUST appear; it cannot exceed a finite
/// cardinality upper bound. (The maximum-side of the literal cADL wording is
/// intentionally *not* enforced: a single-membership container with several
/// alternative child blocks — each `occurrences 0..1` — is a legal openEHR
/// pattern whose occurrence-maxima sum exceeds the cardinality, cADL
/// §Single-valued/alternative blocks.)
pub(super) fn check_cardinality_occurrences(
    attr_name: &str,
    parent_rm: &str,
    card: &Cardinality,
    children: &[CObject],
) -> Result<(), RuleViolation> {
    let Some(card_upper) = iv_upper(&card.interval) else {
        return Ok(()); // open cardinality upper bound: any number of children fits.
    };
    let required: i64 = children
        .iter()
        .map(|c| i64::from(iv_lower(NodeView::of(c).occurrences)))
        .sum();
    if required > i64::from(card_upper) {
        return Err(RuleViolation::new(
            "VACMCO",
            format!(
                "attribute '{attr_name}' on '{parent_rm}': the sum of the child occurrences \
                 lower bounds ({required}) exceeds the cardinality upper bound ({card_upper}), \
                 so the mandatory children cannot fit"
            ),
        ));
    }
    Ok(())
}
