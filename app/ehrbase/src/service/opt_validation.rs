//! Ingestion-side artefact validity for OPT 1.4 upload (B2 task 6).
//!
//! openEHR formalizes the validity rules a CDR should apply to an *uploaded*
//! archetype/template artefact in the AOM2 validation catalogue
//! (`docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc`) and the AOM2
//! class-definition rule blocks
//! (`AM/docs/AOM2/master04.5-constraint_model-class_definitions.adoc`,
//! `AM/docs/AOM2/master07-terminology_package.adoc`). OPT 1.4 has no normative
//! prose chapter (blueprint `docs/blueprint/03-am.md` §Spec defects 1), so the
//! only formalized catalogue of standalone-artefact checks is AOM2/08; those
//! rules are the oracle here, applied to the *flattened* OPT 1.4 tree
//! (`openehr_its::opt14::OperationalTemplate`).
//!
//! Every violation is reported through [`ServiceError::sm`] with
//! `CallStatusType::PreconditionViolation` (→ ITS-REST `400 Bad Request`),
//! carrying the AOM2 rule code in the message text. `400` is what the CNF
//! `I_DEFINITION_ADL14` upload/validate suites assert for an invalid OPT
//! (`docs/specs/openehr/CNF/tests/platform/robot/I_DEFINITION_ADL14/`
//! `validate_opt/…invalid_opt.robot`: "server rejected OPT with status code
//! 400"; the `_resources/keywords/template_opt1.4_keywords.robot` upload
//! keyword asserts the same); the ECC upload-invalid case accepts any `4xx`.
//!
//! # Reference-model conformance checks
//!
//! The RM-conformance rules (VCORM/VCARM/VCAEX/VCACA/VCAM, AOM2/08 lines 70–75)
//! require "a computational representation of the reference model". We use the
//! BMM-generated static RM model (`openehr_rm::model`, ADR-008 §3) — the same
//! spec-pinned oracle the AQL planner uses.
//!
//! # Codes deliberately not implemented here (inapplicable to OPT 1.4)
//!
//! See the per-check `PORT NOTE`s below for VCACA (RM cardinality bounds are not
//! exposed by the static model), the AOM2 phase-2 *specialisation* rules
//! (VSANCE/VSANCC/VSONCT/…, meaningful only for a differential child against its
//! flat parent — an OPT is already flat), and the VCORMEN* enumeration rules
//! (no RM enumeration metadata in the static model).

use std::collections::HashSet;

use openehr_its::opt14::{
    ArchetypeInternalRef, ArchetypeSlot, CArchetypeRoot, CAttribute, CObject, CPrimitive,
    Cardinality, ConstraintRef, FlatArchetypeOntology, Intervalofinteger, OperationalTemplate,
};
use openehr_rm::model;

use ehrbase_sm::CallStatusType;

use super::ServiceError;

/// One artefact-validity violation: the AOM2 rule code + a human detail.
struct Violation {
    code: &'static str,
    detail: String,
}

/// LOCATABLE meta attributes tolerated on any RM class (see the PORT NOTE at
/// the VCARM check: archie-era OPTs constrain these on PATHABLE-only classes).
const LOCATABLE_META_ATTRS: &[&str] = &[
    "name",
    "archetype_node_id",
    "uid",
    "links",
    "archetype_details",
    "feeder_audit",
];

/// Legacy `(class, attribute)` pairs tolerated for prior-art OPT compatibility
/// (PORT NOTE): `ELEMENT.null_flavor` is the archetype-tooling (US) spelling of
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

impl Violation {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// Validate an uploaded OPT 1.4 artefact against the AOM2/08 standalone-artefact
/// validity rules. The first violation found is returned as a `400` carrying the
/// AOM2 rule code (`"<CODE>: <detail>"`); a fully valid artefact returns `Ok`.
pub(super) fn validate_opt_artefact(opt: &OperationalTemplate) -> Result<(), ServiceError> {
    check(opt).map_err(|v| {
        ServiceError::sm(
            CallStatusType::PreconditionViolation,
            format!("{}: {}", v.code, v.detail),
        )
    })
}

/// The per-walk validation context: the globally-collected code sets plus
/// whether the artefact declares any `constraint_definitions` at all (which
/// gates VACDF — see `check_constraint_ref`).
struct Ctx {
    defined_at: HashSet<String>,
    defined_ac: HashSet<String>,
    has_constraint_defs: bool,
}

fn check(opt: &OperationalTemplate) -> Result<(), Violation> {
    // Terminology-side rules first (cheap; no tree recursion needed beyond code
    // collection).
    let ctx = Ctx {
        defined_at: collect_defined_at_codes(opt),
        defined_ac: collect_defined_ac_codes(opt),
        has_constraint_defs: flat_ontologies(opt)
            .iter()
            .any(|o| o.constraint_definitions.iter().any(|s| !s.items.is_empty())),
    };
    check_term_bindings(opt, &ctx.defined_at)?; // VTTBK
    check_constraint_bindings(opt, &ctx.defined_ac)?; // VTCBK
    check_language_consistency(opt)?; // VTLC

    // RM-conformance + structural rules walk the flattened definition tree.
    // The root definition is a `C_ARCHETYPE_ROOT`; its `rm_type_name` is the
    // top RM type (VCORM).
    check_object_type(
        opt.definition.rm_type_name.as_str(),
        &opt.definition.node_id,
    )?;
    check_node_id(&opt.definition.node_id, &ctx.defined_at)?; // VATID (root)
    // VARID / VARDT on the root archetype id (ADL1.4 master08 lines 544/556).
    check_archetype_id(
        &opt.definition.archetype_id.value,
        opt.definition.rm_type_name.as_str(),
    )?;
    for attr in &opt.definition.attributes {
        walk_attribute(attr, opt.definition.rm_type_name.as_str(), &ctx)?;
    }
    Ok(())
}

// ─── tree walk ────────────────────────────────────────────────────────────────

/// Recurse into one constrained attribute of an object whose RM type is
/// `parent_rm`.
fn walk_attribute(attr: &CAttribute, parent_rm: &str, ctx: &Ctx) -> Result<(), Violation> {
    let (attr_name, existence, children, cardinality) = match attr {
        CAttribute::CSingleAttribute(a) => (
            a.rm_attribute_name.as_str(),
            &a.existence,
            &a.children,
            None,
        ),
        CAttribute::CMultipleAttribute(a) => (
            a.rm_attribute_name.as_str(),
            &a.existence,
            &a.children,
            Some(&a.cardinality),
        ),
    };

    // C_ATTRIBUTE invariant Rm_attribute_name_valid: `not rm_attribute_name.
    // is_empty` (AOM1.4 c_attribute class file, Invariants).
    if attr_name.is_empty() {
        return Err(Violation::new(
            "Rm_attribute_name_valid",
            format!("an attribute constraint under '{parent_rm}' has an empty rm_attribute_name"),
        ));
    }

    // C_ATTRIBUTE invariant Existence_set: `existence.lower >= 0 and
    // existence.upper <= 1` (AOM1.4 c_attribute class file, Invariants).
    if iv_lower(existence) < 0
        || existence.upper_unbounded
        || iv_upper(existence).is_none_or(|u| u > 1)
        || iv_upper(existence).is_some_and(|u| u < iv_lower(existence))
    {
        return Err(Violation::new(
            "Existence_set",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' has an existence outside 0..1 \
                 (existence.lower >= 0 and existence.upper <= 1)"
            ),
        ));
    }

    // C_SINGLE_ATTRIBUTE invariant Members_valid: every alternative child
    // satisfies `co.occurrences.upper <= 1` — a single-valued attribute can
    // hold at most one value (AOM1.4 c_single_attribute class file,
    // Invariants; also cADL: occurrences upper > 1 only under a container
    // attribute, AOM1.4 c_object class file, `occurrences`).
    if cardinality.is_none() {
        for child in children {
            let occ = co_occurrences(child);
            if occ.upper_unbounded || iv_upper(occ).is_some_and(|u| u > 1) {
                return Err(Violation::new(
                    "Members_valid",
                    format!(
                        "attribute '{attr_name}' on '{parent_rm}' is single-valued but child \
                         object '{}' has occurrences upper > 1",
                        co_node_id(child)
                    ),
                ));
            }
        }
    }

    // RM-conformance checks fire only when the enclosing object's RM type is
    // known to the static model; an unknown parent means we cannot judge its
    // attributes (and VCORM already flagged the parent if it was a bogus type).
    if model::class(parent_rm).is_some() {
        match model::attribute(parent_rm, attr_name) {
            // PORT NOTE (prior-art OPT tolerance): archie/openEHR-SDK tooling
            // models every constrainable node as Locatable, so published OPTs
            // (incl. the vendored IPS template) constrain LOCATABLE meta
            // attributes (`name`, `archetype_node_id`, …) on classes the RM
            // derives from PATHABLE only (e.g. ISM_TRANSITION —
            // org.openehr.rm.composition.ism_transition.adoc inherits
            // PATHABLE). Rejecting them per strict VCARM would refuse
            // real-world templates; the constraints are tolerated (they bind
            // to the serialized meta fields, which canonical JSON carries).
            None if LOCATABLE_META_ATTRS.contains(&attr_name)
                || LEGACY_RM_ATTRS.contains(&(parent_rm, attr_name)) => {}
            None => {
                // VCARM: attribute name reference model validity (AOM2 line 126).
                return Err(Violation::new(
                    "VCARM",
                    format!(
                        "attribute '{attr_name}' is not defined in reference-model type \
                         '{parent_rm}'"
                    ),
                ));
            }
            Some(rm_attr) => {
                rm_conformance(attr, attr_name, parent_rm, existence, rm_attr)?;
            }
        }
    }

    // VACMCO / VCOC: occurrences-vs-cardinality (container attributes only).
    if let Some(card) = cardinality {
        check_cardinality_occurrences(attr_name, parent_rm, card, children)?;
    }

    // Recurse into each child object.
    for child in children {
        walk_object(child, ctx)?;
    }
    Ok(())
}

// ─── AOM 1.4 constraint-model invariants (per-node-kind) ────────────────────────

/// VARID: the archetype id must conform to the openEHR archetype-identifier
/// syntax (ADL1.4 master08 line 544; BASE `base_types` master05 §Syntaxes), and
/// VARDT: the RM type named by the constraint node must match the type slot of
/// the identifier's first segment (ADL1.4 master08 line 556; composite
/// identifiers compare case-insensitively, BASE `base_types` master05
/// §"Composite Identifiers and Case").
fn check_archetype_id(id: &str, rm_type_name: &str) -> Result<(), Violation> {
    if !is_archetype_id_shaped(id) {
        return Err(Violation::new(
            "VARID",
            format!("'{id}' is not a valid openEHR archetype identifier"),
        ));
    }
    // qualified_rm_entity = rm_originator '-' rm_name '-' rm_entity; the
    // rm_entity is everything after the second '-'.
    let qualified = id.split('.').next().unwrap_or("");
    let entity = qualified
        .match_indices('-')
        .nth(1)
        .map_or("", |(i, _)| &qualified[i + 1..]);
    let bare_rm = rm_type_name.split('<').next().unwrap_or(rm_type_name);
    if !entity.eq_ignore_ascii_case(bare_rm) {
        return Err(Violation::new(
            "VARDT",
            format!(
                "the definition node's RM type '{rm_type_name}' does not match the type slot \
                 '{entity}' of archetype id '{id}'"
            ),
        ));
    }
    Ok(())
}

/// Archetype-identifier shape for uploaded artefacts:
/// `rm_originator-rm_name-rm_entity.domain_concept.v<version>` (BASE `base_types`
/// master05 §Syntaxes). Tolerances beyond the strict BASE `name-str` grammar,
/// both adjudicated against real published templates (never against CNF valid
/// fixtures, which all conform strictly):
///
/// - the version may be multi-part numeric (`v1.0.0`) — the ADL2-era archetype
///   HRID form appears in deployed OPT 1.4 exports (the vendored
///   `Request_for_Pancreas_Special_Urgency_Listing` corpus template);
/// - PORT NOTE: concept segments tolerate `(`/`)` and digit-leading segments —
///   Ocean/LANIT tooling emits concept names like
///   `t_neurologist_examination(1-17)_lanit` (vendored Better corpus); the
///   strict grammar would refuse real-world templates.
fn is_archetype_id_shaped(id: &str) -> bool {
    fn alphanum_str(s: &str) -> bool {
        let mut chars = s.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
    // Split the version off the tail: the last `.v` followed by a digit.
    let Some((head, version)) = id.rsplit_once(".v") else {
        return false;
    };
    let version_ok = !version.is_empty()
        && version
            .split('.')
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        && version.split('.').count() <= 3;
    let Some((qualified, concept)) = head.split_once('.') else {
        return false;
    };
    let entity_parts: Vec<&str> = qualified.split('-').collect();
    let entity_ok = entity_parts.len() == 3 && entity_parts.iter().all(|p| alphanum_str(p));
    let concept_ok = !concept.is_empty()
        && concept
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '(' | ')' | '.'));
    version_ok && entity_ok && concept_ok
}

/// `ARCHETYPE_SLOT` checks: VDFAI — an archetype identifier mentioned in a slot
/// must conform to the archetype-identifier syntax (ADL1.4 master08 line 573).
/// Slot include/exclude expressions are Perl regexes over archetype ids (cADL
/// §Archetype Slots), so only a *literal* pattern (regex-escaped dots, no other
/// metacharacters) is decidable as an identifier; genuine regexes are left to
/// runtime slot admission.
fn check_slot(slot: &ArchetypeSlot) -> Result<(), Violation> {
    for assertion in slot.includes.iter().chain(&slot.excludes) {
        let Some(pattern) = slot_assertion_pattern(assertion) else {
            continue;
        };
        for alt in pattern.split('|') {
            let Some(literal) = regex_literal(alt) else {
                continue;
            };
            if !is_archetype_id_shaped(&literal) {
                return Err(Violation::new(
                    "VDFAI",
                    format!(
                        "slot '{}' names '{literal}', which is not a valid openEHR archetype \
                         identifier",
                        slot.node_id
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The archetype-id regex carried by a slot assertion (`archetype_id/value
/// matches {/…/}`), if the expression has that shape.
fn slot_assertion_pattern(a: &openehr_its::opt14::Assertion) -> Option<String> {
    use openehr_its::opt14::ExprItem;
    fn find_pattern(e: &ExprItem) -> Option<String> {
        match e {
            ExprItem::ExprLeaf(l) => l
                .item
                .as_str()
                .map(|s| s.trim_matches('/').to_owned())
                .filter(|s| s.contains("openEHR") || s.contains('\\') || s.contains('.')),
            ExprItem::ExprBinaryOperator(b) => {
                find_pattern(&b.right_operand).or_else(|| find_pattern(&b.left_operand))
            }
            ExprItem::ExprUnaryOperator(u) => find_pattern(&u.operand),
        }
    }
    find_pattern(&a.expression)
}

/// If a regex alternative is a literal archetype id (only `\.`-escaped dots,
/// no other metacharacters), return the unescaped literal; else `None`.
fn regex_literal(alt: &str) -> Option<String> {
    let mut out = String::with_capacity(alt.len());
    let mut chars = alt.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('.') => out.push('.'),
                _ => return None,
            },
            '.' | '*' | '+' | '?' | '[' | ']' | '(' | ')' | '{' | '}' | '^' | '$' | '|' => {
                return None;
            }
            _ => out.push(c),
        }
    }
    (!out.is_empty()).then_some(out)
}

/// `ARCHETYPE_INTERNAL_REF` invariant `Target_path_valid`: `target_path /= Void
/// and then not target_path.is_empty` (AOM1.4 `archetype_internal_ref` class
/// file, Invariants); the path must be an absolute archetype path (VDFPT,
/// ADL1.4 master08 line 576). (Flattened OPTs expand internal refs, so this
/// fires only on a malformed artefact — the whole vendored corpus carries
/// none.)
fn check_internal_ref(r: &ArchetypeInternalRef) -> Result<(), Violation> {
    if r.target_path.is_empty() || !r.target_path.starts_with('/') {
        return Err(Violation::new(
            "Target_path_valid",
            format!(
                "internal reference '{}' has an invalid target_path '{}' (must be a non-empty \
                 absolute path)",
                r.node_id, r.target_path
            ),
        ));
    }
    Ok(())
}

/// VACDF: each constraint code (`acNNNN`) used in the definition must be
/// defined in the `constraint_definitions` part of the ontology (ADL1.4
/// master08 line 566).
///
/// PORT NOTE (flattened-OPT tolerance): deployed OPT 1.4 exports routinely
/// carry `CONSTRAINT_REF` nodes with NO `constraint_definitions` sets at all
/// (Ocean Template Designer drops the constraint vocabulary on flatten — the
/// vendored RIPPLE/Better corpus templates). VACDF is therefore enforced only
/// when the artefact declares a constraint vocabulary; an artefact with none
/// is tolerated.
fn check_constraint_ref(r: &ConstraintRef, ctx: &Ctx) -> Result<(), Violation> {
    if ctx.has_constraint_defs && !ctx.defined_ac.contains(&r.reference) {
        return Err(Violation::new(
            "VACDF",
            format!(
                "constraint reference '{}' (node '{}') is not defined in constraint_definitions",
                r.reference, r.node_id
            ),
        ));
    }
    Ok(())
}

/// The `C_PRIMITIVE`-level checks: `C_BOOLEAN` satisfiability, `C_DEFINED_OBJECT`
/// `Assumed_value_valid` for list/range-constrained primitives, and the
/// `C_DATE`/`C_TIME`/`C_DATE_TIME` `Pattern_validity` + `C_DURATION` pattern syntax.
fn check_primitive(p: &CPrimitive, node_id: &str) -> Result<(), Violation> {
    match p {
        CPrimitive::CBoolean(b) => {
            // C_BOOLEAN (AOM1.4 c_boolean class file, Description): true_valid
            // and false_valid cannot both be False — the constraint would be
            // unsatisfiable.
            if !b.true_valid && !b.false_valid {
                return Err(Violation::new(
                    "C_BOOLEAN_validity",
                    format!(
                        "node '{node_id}': true_valid and false_valid are both false — the \
                         boolean constraint is unsatisfiable"
                    ),
                ));
            }
            if let Some(assumed) = b.assumed_value {
                let ok = if assumed { b.true_valid } else { b.false_valid };
                if !ok {
                    return Err(Violation::new(
                        "Assumed_value_valid",
                        format!(
                            "node '{node_id}': the assumed boolean value {assumed} is not \
                             permitted by the true_valid/false_valid flags"
                        ),
                    ));
                }
            }
        }
        CPrimitive::CString(s) => {
            // Assumed_value_valid against a closed value list (C_STRING,
            // AOM1.4 c_string class file; cADL: string constraints are
            // case-sensitive).
            if let Some(assumed) = &s.assumed_value
                && !s.list.is_empty()
                && s.list_open != Some(true)
                && !s.list.contains(assumed)
            {
                return Err(Violation::new(
                    "Assumed_value_valid",
                    format!(
                        "node '{node_id}': the assumed string '{assumed}' is not in the closed \
                         value list"
                    ),
                ));
            }
        }
        CPrimitive::CInteger(c) => {
            if let Some(assumed) = c.assumed_value {
                let list_ok = c.list.is_empty() || c.list.contains(&assumed);
                let range_ok = c.range.as_ref().is_none_or(|r| int_in_range(assumed, r));
                if !list_ok || !range_ok {
                    return Err(Violation::new(
                        "Assumed_value_valid",
                        format!(
                            "node '{node_id}': the assumed integer {assumed} is outside the \
                             constrained list/range"
                        ),
                    ));
                }
            }
        }
        CPrimitive::CReal(c) => {
            if let Some(assumed) = c.assumed_value {
                #[allow(clippy::float_cmp)] // list membership is exact per cADL
                let list_ok = c.list.is_empty() || c.list.contains(&assumed);
                let range_ok = c.range.as_ref().is_none_or(|r| real_in_range(assumed, r));
                if !list_ok || !range_ok {
                    return Err(Violation::new(
                        "Assumed_value_valid",
                        format!(
                            "node '{node_id}': the assumed real {assumed} is outside the \
                             constrained list/range"
                        ),
                    ));
                }
            }
        }
        // C_DATE/C_DATE_TIME/C_TIME invariant Pattern_validity:
        // `pattern /= Void implies valid_iso8601_*_constraint_pattern(pattern)`
        // (AOM1.4 c_date/c_date_time/c_time class files); the legal patterns
        // and the optional→optional/disallowed, disallowed→disallowed
        // field-ordering are cADL §Constraints on Dates/Times (ADL1.4 master05
        // lines 858–892).
        CPrimitive::CDate(c) => {
            if let Some(pattern) = &c.pattern
                && !valid_date_pattern(pattern)
            {
                return Err(pattern_violation(node_id, pattern, "date"));
            }
        }
        CPrimitive::CTime(c) => {
            if let Some(pattern) = &c.pattern
                && !valid_time_pattern(pattern)
            {
                return Err(pattern_violation(node_id, pattern, "time"));
            }
        }
        CPrimitive::CDateTime(c) => {
            if let Some(pattern) = &c.pattern
                && !valid_date_time_pattern(pattern)
            {
                return Err(pattern_violation(node_id, pattern, "date-time"));
            }
        }
        // C_DURATION: the pattern must be `P[Y][M][W][D][T[H][M][S]]` — openEHR
        // deviates from strict ISO 8601 by allowing `W` to be mixed with the
        // other designators (cADL §Duration Constraints, ADL1.4 master05 lines
        // 934–980).
        CPrimitive::CDuration(c) => {
            if let Some(pattern) = &c.pattern
                && !valid_duration_pattern(pattern)
            {
                return Err(pattern_violation(node_id, pattern, "duration"));
            }
        }
    }
    Ok(())
}

/// Duplicate codes in a terminology-code code list are invalid (ADL2
/// master04.6 STCDC — "constraint code list contains duplicate codes"; the
/// same defect in an OPT 1.4 `C_CODE_PHRASE` list).
fn check_code_list(code_list: &[String], node_id: &str) -> Result<(), Violation> {
    let mut seen = HashSet::new();
    for code in code_list {
        // Empty entries are tooling noise, not codes (Ocean exports emit
        // repeated empty <code_list/> elements — the vendored UK AoMRC corpus
        // template); only real codes participate in the duplicate check.
        if code.is_empty() {
            continue;
        }
        if !seen.insert(code) {
            return Err(Violation::new(
                "STCDC",
                format!("node '{node_id}': code '{code}' is duplicated in the code list"),
            ));
        }
    }
    Ok(())
}

fn pattern_violation(node_id: &str, pattern: &str, kind: &str) -> Violation {
    Violation::new(
        "Pattern_validity",
        format!("node '{node_id}': '{pattern}' is not a valid {kind} constraint pattern"),
    )
}

/// `C_DEFINED_OBJECT` invariant `Assumed_value_valid` for the code-carrying domain
/// types (`C_CODE_PHRASE` / `C_CODE_REFERENCE`): the assumed code must be one of
/// the constrained codes when the code list is closed and non-empty.
fn check_assumed_code(
    assumed: Option<&openehr_base::prelude::CodePhrase>,
    code_list: &[String],
    node_id: &str,
) -> Result<(), Violation> {
    if let Some(assumed) = assumed
        && !code_list.is_empty()
        && !code_list.contains(&assumed.code_string)
    {
        return Err(Violation::new(
            "Assumed_value_valid",
            format!(
                "node '{node_id}': the assumed code '{}' is not in the constrained code list",
                assumed.code_string
            ),
        ));
    }
    Ok(())
}

// ─── temporal + duration constraint-pattern validity ────────────────────────────

/// `yyyy-<mm|??|XX>-<dd|??|XX>` with the field-ordering rule: optional (`??`)
/// may be followed only by optional/disallowed; disallowed (`XX`) only by
/// disallowed (ADL1.4 master05 lines 858–866).
fn valid_date_pattern(p: &str) -> bool {
    let parts: Vec<&str> = p.split('-').collect();
    let [y, m, d] = parts.as_slice() else {
        return false;
    };
    *y == "yyyy" && field_chain_valid(&[(m, "mm"), (d, "dd")])
}

/// `<HH|??|XX>:<MM|??|XX>:<SS|??|XX>` with the same field-ordering rule and an
/// optional trailing timezone requirement (`Z` / `±hh` / `±hh:mm` / `±hhmm` —
/// ADL1.4 master05 lines 852–854, 896–910: a timezone can be required, never
/// prohibited).
fn valid_time_pattern(p: &str) -> bool {
    let body = p
        .strip_suffix('Z')
        .or_else(|| strip_tz_offset(p))
        .unwrap_or(p);
    let parts: Vec<&str> = body.split(':').collect();
    let [h, m, s] = parts.as_slice() else {
        return false;
    };
    *h == "HH" && field_chain_valid(&[(m, "MM"), (s, "SS")])
}

/// `<date>T<time>` (ADL1.4 master05 lines 868–892). The date and time fields
/// form ONE monotonic ordering chain (Month→Day→Hour→Minute→Second — the
/// `C_DATE_TIME` `*_validity_optional`/`*_validity_disallowed` invariants).
/// Unlike `C_TIME`, `C_DATE_TIME` has an `hour_validity`, so `??`/`XX` hours are
/// legal here (e.g. `yyyy-??-??T??:??:??`, the CNF RIPPLE conformance
/// template).
fn valid_date_time_pattern(pattern: &str) -> bool {
    let Some((date, time)) = pattern.split_once('T') else {
        return false;
    };
    let date_parts: Vec<&str> = date.split('-').collect();
    let [y, mo, dy] = date_parts.as_slice() else {
        return false;
    };
    let body = time
        .strip_suffix('Z')
        .or_else(|| strip_tz_offset(time))
        .unwrap_or(time);
    let time_parts: Vec<&str> = body.split(':').collect();
    let [h, mi, s] = time_parts.as_slice() else {
        return false;
    };
    *y == "yyyy" && field_chain_valid(&[(mo, "mm"), (dy, "dd"), (h, "HH"), (mi, "MM"), (s, "SS")])
}

/// One monotonic field chain: mandatory (`mm`/`dd`/…) → any; optional (`??`) →
/// optional or disallowed; disallowed (`XX`) → disallowed.
fn field_chain_valid(fields: &[(&&str, &str)]) -> bool {
    let mut state = 0u8; // 0 = mandatory so far, 1 = optional seen, 2 = disallowed seen
    for (actual, mandatory_form) in fields {
        let level = if **actual == *mandatory_form {
            0
        } else if **actual == "??" {
            1
        } else if **actual == "XX" {
            2
        } else {
            return false;
        };
        if level < state {
            return false;
        }
        state = state.max(level);
    }
    true
}

/// A time-pattern timezone suffix `±hh`, `±hh:mm`, or `±hhmm` — strip it if
/// present (the pattern grammar writes them literally, e.g. `HH:MM:SS+hh:mm`).
fn strip_tz_offset(p: &str) -> Option<&str> {
    for suffix in ["+hh:mm", "-hh:mm", "+hhmm", "-hhmm", "+hh", "-hh"] {
        if let Some(body) = p.strip_suffix(suffix) {
            return Some(body);
        }
    }
    None
}

/// `P` followed by an in-order subset of `Y M W D`, optionally `T` + an
/// in-order non-empty subset of `H M S`; at least one designator overall.
fn valid_duration_pattern(p: &str) -> bool {
    let Some(rest) = p.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (rest, None),
    };
    if !in_order_subset(date_part, &['Y', 'M', 'W', 'D']) {
        return false;
    }
    match time_part {
        Some(t) => !t.is_empty() && in_order_subset(t, &['H', 'M', 'S']),
        None => !date_part.is_empty(),
    }
}

/// `s` uses only characters from `order`, each at most once, in order.
fn in_order_subset(s: &str, order: &[char]) -> bool {
    let mut pos = 0usize;
    for c in s.chars() {
        match order[pos..].iter().position(|o| *o == c) {
            Some(i) => pos += i + 1,
            None => return false,
        }
    }
    true
}

fn int_in_range(v: i32, r: &Intervalofinteger) -> bool {
    let lower_ok = r.lower_unbounded || r.lower.is_none_or(|l| v >= l);
    let upper_ok = r.upper_unbounded || r.upper.is_none_or(|u| v <= u);
    lower_ok && upper_ok
}

fn real_in_range(v: f64, r: &openehr_its::opt14::Intervalofreal) -> bool {
    let lower_ok = r.lower_unbounded || r.lower.is_none_or(|l| v >= l);
    let upper_ok = r.upper_unbounded || r.upper.is_none_or(|u| v <= u);
    lower_ok && upper_ok
}

/// Check one child object node, then recurse into its own attributes.
fn walk_object(obj: &CObject, ctx: &Ctx) -> Result<(), Violation> {
    let rm_type = co_rm_type(obj);
    let node_id = co_node_id(obj);

    // VCORM: object constraint type-name existence (AOM2 line 325). A
    // primitive-object node carries a foundation primitive type name (STRING,
    // INTEGER, …) which is intentionally absent from the RM model, so it is
    // exempt.
    if !matches!(obj, CObject::CPrimitiveObject(_)) {
        check_object_type(rm_type, node_id)?;
    }

    // VATID: every at-code used as a node_id must be defined in terminology.
    check_node_id(node_id, &ctx.defined_at)?;

    // AOM 1.4 per-node-kind invariants (the constraint-model class files).
    match obj {
        CObject::CArchetypeRoot(root) => {
            // VARID / VARDT on every flattened slot-filler root.
            check_archetype_id(&root.archetype_id.value, &root.rm_type_name)?;
        }
        CObject::ArchetypeSlot(slot) => check_slot(slot)?,
        CObject::ArchetypeInternalRef(r) => check_internal_ref(r)?,
        CObject::ConstraintRef(r) => check_constraint_ref(r, ctx)?,
        CObject::CPrimitiveObject(p) => {
            if let Some(item) = &p.item {
                check_primitive(item, node_id)?;
            }
        }
        CObject::CCodePhrase(c) => {
            check_code_list(&c.code_list, node_id)?;
            check_assumed_code(c.assumed_value.as_ref(), &c.code_list, node_id)?;
        }
        CObject::CCodeReference(c) => {
            check_code_list(&c.code_list, node_id)?;
            check_assumed_code(c.assumed_value.as_ref(), &c.code_list, node_id)?;
        }
        CObject::CDvOrdinal(c) => {
            // C_DEFINED_OBJECT invariant Assumed_value_valid (AOM1.4
            // c_defined_object class file): the assumed ordinal must be one of
            // the constrained (symbol, value) pairs.
            if let Some(assumed) = &c.assumed_value
                && !c.list.is_empty()
                && !c.list.iter().any(|o| o.value == assumed.value)
            {
                return Err(Violation::new(
                    "Assumed_value_valid",
                    format!(
                        "node '{node_id}': the assumed DV_ORDINAL value {} is not one of the \
                         constrained ordinal values",
                        assumed.value
                    ),
                ));
            }
        }
        CObject::CDvQuantity(c) => {
            // Assumed_value_valid: the assumed quantity's units must be one of
            // the constrained unit items, and its magnitude inside that item's
            // magnitude range.
            if let Some(assumed) = &c.assumed_value
                && !c.list.is_empty()
            {
                let Some(item) = c.list.iter().find(|i| i.units == assumed.units) else {
                    return Err(Violation::new(
                        "Assumed_value_valid",
                        format!(
                            "node '{node_id}': the assumed DV_QUANTITY units '{}' are not among \
                             the constrained units",
                            assumed.units
                        ),
                    ));
                };
                if let Some(range) = &item.magnitude
                    && !real_in_range(assumed.magnitude, range)
                {
                    return Err(Violation::new(
                        "Assumed_value_valid",
                        format!(
                            "node '{node_id}': the assumed DV_QUANTITY magnitude {} is outside \
                             the constrained magnitude range for units '{}'",
                            assumed.magnitude, assumed.units
                        ),
                    ));
                }
            }
        }
        _ => {}
    }

    // Recurse into a nested C_ARCHETYPE_ROOT's terminology scope-wise via the
    // global set already collected; structurally we just descend its attributes.
    for attr in co_attributes(obj) {
        walk_attribute(attr, rm_type, ctx)?;
    }
    Ok(())
}

// ─── RM conformance (VCORM/VCARM/VCAEX/VCACA/VCAM) ──────────────────────────────

/// VCORM: `object constraint type name existence: a type name introducing an
/// object constraint block must be defined in the underlying information model.`
/// (`AOM2/master04.5-…class_definitions.adoc` line 325.)
fn check_object_type(rm_type: &str, node_id: &str) -> Result<(), Violation> {
    // Strip any generic argument (`DV_INTERVAL<DV_QUANTITY>` → `DV_INTERVAL`);
    // the static model keys on the bare class name.
    let bare = rm_type.split('<').next().unwrap_or(rm_type).trim();
    if bare.is_empty() {
        return Err(Violation::new(
            "VCORM",
            format!("object node '{node_id}' has an empty rm_type_name"),
        ));
    }
    if model::class(bare).is_none() {
        return Err(Violation::new(
            "VCORM",
            format!(
                "type '{rm_type}' (object node '{node_id}') is not defined in the reference model"
            ),
        ));
    }
    Ok(())
}

/// The AOM2 RM-conformance rules that apply to a `C_ATTRIBUTE` once its RM
/// attribute has been resolved: VCAM (multiplicity), VCAEX (existence), plus the
/// VCORMT type-conformance of each child against the RM attribute's declared
/// type.
fn rm_conformance(
    attr: &CAttribute,
    attr_name: &str,
    parent_rm: &str,
    existence: &Intervalofinteger,
    rm_attr: &model::RmAttribute,
) -> Result<(), Violation> {
    let rm_is_multiple = !matches!(rm_attr.container, model::Container::None);

    // VCAM: `archetype attribute reference model multiplicity conformance: the
    // multiplicity … of an attribute must conform to that of the corresponding
    // attribute in the underlying information model.` (line 132.) A container
    // (`C_MULTIPLE_ATTRIBUTE`) constraint on a single-valued RM attribute cannot
    // conform.
    let arch_is_multiple = matches!(attr, CAttribute::CMultipleAttribute(_));
    if arch_is_multiple && !rm_is_multiple {
        return Err(Violation::new(
            "VCAM",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' is constrained as a container \
                 (C_MULTIPLE_ATTRIBUTE) but is single-valued in the reference model"
            ),
        ));
    }

    // VCAEX: `archetype attribute reference model existence conformance: the
    // existence of an attribute, if set, must conform, i.e. be the same or
    // narrower, to the existence … in the underlying information model.`
    // (line 129.) The RM existence upper bound is always 1; the RM lower bound
    // is 1 for a mandatory attribute and 0 otherwise. Allowing absence (`{0..}`)
    // on an RM-mandatory attribute *widens* it — the one enforceable violation.
    if rm_attr.is_mandatory && iv_lower(existence) == 0 && !existence.lower_unbounded {
        return Err(Violation::new(
            "VCAEX",
            format!(
                "attribute '{attr_name}' on '{parent_rm}' has existence lower bound 0 but the \
                 attribute is mandatory (existence lower bound 1) in the reference model"
            ),
        ));
    }

    // VCACA: `archetype attribute reference model cardinality conformance …`
    // (line 162). PORT NOTE: the static RM model (`openehr_rm::model`) exposes an
    // attribute's *container kind* (`None`/`List`/`Set`/`Hash`) but not the RM's
    // numeric cardinality bounds, and cADL itself hedges that RM-cardinality
    // enforcement "may depend somewhat on knowledge of the software system"
    // (blueprint 03-am §Spec defects 12; `ADL1.4/master05-cadl.adoc` line 268).
    // The enforceable part of VCACA — that a container constraint may not sit on
    // a single-valued RM attribute — is already covered by VCAM above; the
    // numeric-bound part is not checkable without RM cardinality metadata.

    Ok(())
}

// ─── VACMCO / VCOC (occurrences vs cardinality) ─────────────────────────────────

/// VCOC / VACMCO: `it must be possible for … one instance of every mandatory
/// child object … to be included within the cardinality range.`
/// (`AOM2/…class_definitions.adoc` line 159, restating cADL VCOC,
/// `ADL1.4/master05-cadl.adoc` line 324, per blueprint 03-am req 8.) The sum of
/// the children's occurrence *lower* bounds is the count that MUST appear; it
/// cannot exceed a finite cardinality upper bound. (The maximum-side of the
/// literal cADL wording is intentionally *not* enforced: a single-membership
/// container with several alternative child blocks — each `occurrences 0..1` —
/// is a legal openEHR pattern whose occurrence-maxima sum exceeds the
/// cardinality, cADL §Single-valued/alternative blocks.)
fn check_cardinality_occurrences(
    attr_name: &str,
    parent_rm: &str,
    card: &Cardinality,
    children: &[CObject],
) -> Result<(), Violation> {
    let Some(card_upper) = iv_upper(&card.interval) else {
        return Ok(()); // open cardinality upper bound: any number of children fits.
    };
    let required: i64 = children
        .iter()
        .map(|c| i64::from(iv_lower(co_occurrences(c))))
        .sum();
    if required > i64::from(card_upper) {
        return Err(Violation::new(
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

// ─── VATID (node-id codes defined in terminology) ───────────────────────────────

/// VATID: `check that all codes mentioned in `definition` are defined in
/// terminology` (`AOM2/master08-validation.adoc` line 56). Applied to at-code
/// `node_id`s (the addressable, sibling-identifying codes, AOM14/04 §`Node_id`).
/// Empty `node_ids` (non-addressable leaves) and non-`at`/`id` codes are exempt.
/// The defined-code set is collected globally across the flattened OPT (the
/// definition roots + every `component_ontologies` set), which is deliberately
/// lenient about per-archetype scoping — it still catches a `node_id` that is
/// defined nowhere while never mis-rejecting a correctly-scoped code.
fn check_node_id(node_id: &str, defined_at: &HashSet<String>) -> Result<(), Violation> {
    if !is_at_code(node_id) {
        return Ok(());
    }
    if !defined_at.contains(node_id) {
        return Err(Violation::new(
            "VATID",
            format!("node_id '{node_id}' is used in the definition but not defined in terminology"),
        ));
    }
    Ok(())
}

/// An addressable archetype term code: `at0000`, `at0001.1`, or the ADL2 `id`
/// form. A bare, empty, or free-text `node_id` is not an at-code.
fn is_at_code(code: &str) -> bool {
    let rest = code
        .strip_prefix("at")
        .or_else(|| code.strip_prefix("id"))
        .unwrap_or("");
    rest.starts_with(|c: char| c.is_ascii_digit())
}

// ─── VTTBK / VTCBK (binding key validity) ───────────────────────────────────────

/// VTTBK: `terminology term binding key valid. Every term binding must be to
/// either a defined archetype term ('at-code') or to a path that is valid in the
/// flat archetype.` (`AOM2/master07-terminology_package.adoc` line 77.) A `/`
/// path key is accepted without full path resolution (conservative — never
/// mis-reject a real flat path).
fn check_term_bindings(
    opt: &OperationalTemplate,
    defined_at: &HashSet<String>,
) -> Result<(), Violation> {
    let check = |code: &str| -> Result<(), Violation> {
        // PORT NOTE (flattened-OPT tolerance): a *specialised* at-code
        // (`at0.23`, dot-notation — AOM2 §specialisation depth) may be bound
        // without a re-emitted local term definition: archie-era flattening
        // keeps parent-archetype bindings whose definitions live in the parent
        // (the vendored blood-pressure corpus OPTs carry these). A dotted
        // at-code is therefore accepted as a valid binding key.
        let specialised = code.starts_with("at") && code.contains('.');
        if code.starts_with('/') || !is_at_code(code) || specialised || defined_at.contains(code) {
            return Ok(());
        }
        Err(Violation::new(
            "VTTBK",
            format!(
                "term binding key '{code}' is neither a defined archetype term (at-code) nor a path"
            ),
        ))
    };
    for set in &opt.definition.term_bindings {
        for item in &set.items {
            check(&item.code)?;
        }
    }
    for onto in flat_ontologies(opt) {
        for set in &onto.term_bindings {
            for item in &set.items {
                check(&item.code)?;
            }
        }
    }
    // Nested C_ARCHETYPE_ROOTs carry their own term_bindings.
    for root in nested_roots(opt) {
        for set in &root.term_bindings {
            for item in &set.items {
                check(&item.code)?;
            }
        }
    }
    Ok(())
}

/// VTCBK: `terminology constraint binding key valid. Every constraint binding
/// must be to a defined archetype constraint code ('ac-code').`
/// (`AOM2/master07-terminology_package.adoc` line 80.)
fn check_constraint_bindings(
    opt: &OperationalTemplate,
    defined_ac: &HashSet<String>,
) -> Result<(), Violation> {
    for onto in flat_ontologies(opt) {
        for set in &onto.constraint_bindings {
            for item in &set.items {
                if !defined_ac.contains(&item.code) {
                    return Err(Violation::new(
                        "VTCBK",
                        format!(
                            "constraint binding key '{}' is not a defined archetype constraint \
                             code (ac-code)",
                            item.code
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

// ─── VTLC (language consistency) ────────────────────────────────────────────────

/// VTLC: `language consistency. Languages consistent: all term codes and
/// constraint codes exist in all languages.`
/// (`AOM2/master07-terminology_package.adoc` line 74.) Applied to each
/// `FlatArchetypeOntology` whose `term_definitions` / `constraint_definitions`
/// are grouped per language: every language must define the same code set.
///
/// PORT NOTE: the per-`C_ARCHETYPE_ROOT` `term_definitions` (a flat
/// `Vec<ARCHETYPE_TERM>`, single-language) carry no language grouping, so VTLC
/// is inert for a single-language OPT — the multi-language code sets live only
/// in `ontology` / `component_ontologies`.
fn check_language_consistency(opt: &OperationalTemplate) -> Result<(), Violation> {
    for onto in flat_ontologies(opt) {
        language_consistent(&codes_by_language(&onto.term_definitions), "term")?;
        language_consistent(
            &codes_by_language(&onto.constraint_definitions),
            "constraint",
        )?;
    }
    Ok(())
}

fn codes_by_language(
    sets: &[openehr_its::opt14::Codedefinitionset],
) -> Vec<(String, HashSet<String>)> {
    sets.iter()
        .map(|s| {
            (
                s.language.clone(),
                s.items.iter().map(|t| t.code.clone()).collect(),
            )
        })
        .collect()
}

fn language_consistent(by_lang: &[(String, HashSet<String>)], kind: &str) -> Result<(), Violation> {
    if by_lang.len() < 2 {
        return Ok(());
    }
    let (ref_lang, ref_codes) = &by_lang[0];
    for (lang, codes) in &by_lang[1..] {
        if codes != ref_codes {
            let missing: Vec<&str> = ref_codes
                .symmetric_difference(codes)
                .map(String::as_str)
                .collect();
            return Err(Violation::new(
                "VTLC",
                format!(
                    "the {kind} code set differs between languages '{ref_lang}' and '{lang}' \
                     (e.g. {missing:?}); all codes must exist in all languages"
                ),
            ));
        }
    }
    Ok(())
}

// ─── code collection + accessors ────────────────────────────────────────────────

/// Every archetype term (`at`/`id`) code defined anywhere in the flattened OPT.
fn collect_defined_at_codes(opt: &OperationalTemplate) -> HashSet<String> {
    let mut out = HashSet::new();
    out.extend(
        opt.definition
            .term_definitions
            .iter()
            .map(|t| t.code.clone()),
    );
    for root in nested_roots(opt) {
        out.extend(root.term_definitions.iter().map(|t| t.code.clone()));
    }
    for onto in flat_ontologies(opt) {
        for set in &onto.term_definitions {
            out.extend(set.items.iter().map(|t| t.code.clone()));
        }
    }
    out
}

/// Every archetype constraint (`ac`) code defined in the flattened OPT.
fn collect_defined_ac_codes(opt: &OperationalTemplate) -> HashSet<String> {
    let mut out = HashSet::new();
    for onto in flat_ontologies(opt) {
        for set in &onto.constraint_definitions {
            out.extend(set.items.iter().map(|t| t.code.clone()));
        }
    }
    out
}

/// `ontology` + every `component_ontologies` entry.
fn flat_ontologies(opt: &OperationalTemplate) -> Vec<&FlatArchetypeOntology> {
    opt.ontology
        .iter()
        .chain(opt.component_ontologies.iter())
        .collect()
}

/// Every nested `C_ARCHETYPE_ROOT` under the definition (the flattened slot
/// fillers), excluding the root definition itself.
fn nested_roots(opt: &OperationalTemplate) -> Vec<&CArchetypeRoot> {
    let mut out = Vec::new();
    for attr in &opt.definition.attributes {
        collect_roots_in_attr(attr, &mut out);
    }
    out
}

fn collect_roots_in_attr<'a>(attr: &'a CAttribute, out: &mut Vec<&'a CArchetypeRoot>) {
    let children = match attr {
        CAttribute::CSingleAttribute(a) => &a.children,
        CAttribute::CMultipleAttribute(a) => &a.children,
    };
    for child in children {
        if let CObject::CArchetypeRoot(root) = child {
            out.push(root);
            for a in &root.attributes {
                collect_roots_in_attr(a, out);
            }
        } else {
            for a in co_attributes(child) {
                collect_roots_in_attr(a, out);
            }
        }
    }
}

fn iv_lower(iv: &Intervalofinteger) -> i32 {
    if iv.lower_unbounded {
        0
    } else {
        iv.lower.unwrap_or(0)
    }
}

fn iv_upper(iv: &Intervalofinteger) -> Option<i32> {
    if iv.upper_unbounded { None } else { iv.upper }
}

fn co_rm_type(obj: &CObject) -> &str {
    match obj {
        CObject::ArchetypeInternalRef(o) => &o.rm_type_name,
        CObject::ArchetypeSlot(o) => &o.rm_type_name,
        CObject::ConstraintRef(o) => &o.rm_type_name,
        CObject::CArchetypeRoot(o) => &o.rm_type_name,
        CObject::CCodePhrase(o) => &o.rm_type_name,
        CObject::CCodeReference(o) => &o.rm_type_name,
        CObject::CComplexObject(o) => &o.rm_type_name,
        CObject::CDefinedObject(o) => &o.rm_type_name,
        CObject::CDvOrdinal(o) => &o.rm_type_name,
        CObject::CDvQuantity(o) => &o.rm_type_name,
        CObject::CDvState(o) => &o.rm_type_name,
        CObject::CPrimitiveObject(o) => &o.rm_type_name,
        CObject::TComplexObject(o) => &o.rm_type_name,
    }
}

fn co_node_id(obj: &CObject) -> &str {
    match obj {
        CObject::ArchetypeInternalRef(o) => &o.node_id,
        CObject::ArchetypeSlot(o) => &o.node_id,
        CObject::ConstraintRef(o) => &o.node_id,
        CObject::CArchetypeRoot(o) => &o.node_id,
        CObject::CCodePhrase(o) => &o.node_id,
        CObject::CCodeReference(o) => &o.node_id,
        CObject::CComplexObject(o) => &o.node_id,
        CObject::CDefinedObject(o) => &o.node_id,
        CObject::CDvOrdinal(o) => &o.node_id,
        CObject::CDvQuantity(o) => &o.node_id,
        CObject::CDvState(o) => &o.node_id,
        CObject::CPrimitiveObject(o) => &o.node_id,
        CObject::TComplexObject(o) => &o.node_id,
    }
}

fn co_occurrences(obj: &CObject) -> &Intervalofinteger {
    match obj {
        CObject::ArchetypeInternalRef(o) => &o.occurrences,
        CObject::ArchetypeSlot(o) => &o.occurrences,
        CObject::ConstraintRef(o) => &o.occurrences,
        CObject::CArchetypeRoot(o) => &o.occurrences,
        CObject::CCodePhrase(o) => &o.occurrences,
        CObject::CCodeReference(o) => &o.occurrences,
        CObject::CComplexObject(o) => &o.occurrences,
        CObject::CDefinedObject(o) => &o.occurrences,
        CObject::CDvOrdinal(o) => &o.occurrences,
        CObject::CDvQuantity(o) => &o.occurrences,
        CObject::CDvState(o) => &o.occurrences,
        CObject::CPrimitiveObject(o) => &o.occurrences,
        CObject::TComplexObject(o) => &o.occurrences,
    }
}

const NO_ATTRS: &[CAttribute] = &[];

fn co_attributes(obj: &CObject) -> &[CAttribute] {
    match obj {
        CObject::CArchetypeRoot(o) => &o.attributes,
        CObject::CComplexObject(o) => &o.attributes,
        CObject::TComplexObject(o) => &o.attributes,
        _ => NO_ATTRS,
    }
}

#[cfg(test)]
mod tests;
