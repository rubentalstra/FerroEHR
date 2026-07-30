//! Phase-1 (basic integrity, standalone) validation checks.
//!
//! Orchestration follows `docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc`
//! §Phase 1 - Basic Integrity: basic identification checks first, then
//! structural, then terminology (the latter gated behind a clean terminology
//! structure and no basic error — master08 "basic errors first", and a code
//! cannot be checked against a missing/inconsistent terminology). Every check
//! cites the spec file + section that defines it.
//!
//! Not run in phase 1 (the variant is present as the catalogue vocabulary):
//! the reference-model checks live in [`super::rm`]; VDIFP + VSONIF against the
//! flat parent in [`super::phase2`]; the flat-form terminology/structure halves
//! (VATDF/VTVSMD/VACMCU/VCOSU for a specialised archetype) in
//! [`super::phase_flat`]; the external-reference resolution half of VARXR in
//! [`super::fillers`] / [`super::phase2`]; and the pure reference-model path
//! halves of VRANP/VRRLP/VRMVP (a reference-model path walk, `super::rm`).
//! VETDF (a code bound to an external terminology must exist there) needs a live
//! terminology-service resolver the network-free spec engine cannot hold — it is
//! validated by the application's terminology service (see the [`ValidationCode`]
//! `Vetdf` doc).

use std::collections::{BTreeSet, HashMap};

use openehr_am::am24::aom2::constraint_model::archetype_slot::ArchetypeSlot;
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_attribute_tuple::CAttributeTuple;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_am::am24::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
use openehr_am::am24::beom::core::assertion::Assertion;
use openehr_base::base_types::definitions::definitions_impl::LOCAL_TERMINOLOGY_ID;
use openehr_base::prelude::Interval;
use openehr_base::prelude::MultiplicityInterval;
use openehr_lang::odin::{OdinKey, OdinValue};

use super::conformance::effective_occurrences_adl14;
use super::{ArchetypeRepository, ArchetypeView, ValidationCode, ValidationIssue, view};
use crate::cadl::Dialect;
use crate::codes::{
    self, is_ac_code, is_at_code, is_id_code, is_root_code_at_depth, is_valid_code,
};
use crate::paths::{
    Resolution, complex_attributes, complex_node_id, complex_rm_type, has_node_id_predicate,
    object_node_id, resolve,
};
use crate::source::SourceArtefact;

/// Run the phase-1 catalogue over `v`, appending issues to `issues`.
///
/// `dialect` selects the validity catalogue: [`Dialect::Adl2`] runs the full
/// AOM2 phase-1 catalogue; [`Dialect::Adl14`] runs the subset that corresponds
/// to the ADL 1.4 / AOM 1.4 standalone validity rules (see
/// [`super::validate_source_phase1_adl14`] for the correspondence + the
/// suppressed AOM2-only rules, each spec-cited at its check site below).
pub(super) fn run(
    v: &ArchetypeView<'_>,
    repo: Option<&ArchetypeRepository>,
    source: Option<(&SourceArtefact, &str)>,
    dialect: Dialect,
    issues: &mut Vec<ValidationIssue>,
) {
    // ── basic identification / meta-data checks (master08 §Basic checks +
    //    §AUTHORED_ARCHETYPE meta-data checks) ──────────────────────────────
    let mut basic = Vec::new();
    check_identification(v, repo, dialect, &mut basic);

    // ── terminology structure (STCNT / VOLT) — gates the code checks ───────
    let term_status = terminology_structure(v);
    match term_status {
        TermStructure::Empty => {
            // STCNT: any missing mandatory part, e.g. the `terminology` section
            // (master08 §Basic checks; no full vendored text — NOTE-flagged).
            if v.kind != crate::source::ArtefactKind::TemplateOverlay {
                basic.push(ValidationIssue::new(
                    ValidationCode::Stcnt,
                    "the terminology section defines no term_definitions",
                ));
            }
        }
        TermStructure::MissingOriginalLanguage => {
            // VOLT: original language available in the terminology section
            // (master08 §AUTHORED_ARCHETYPE meta-data checks; NOTE-flagged).
            issues.push(ValidationIssue::new(
                ValidationCode::Volt,
                format!(
                    "the original language {:?} has no term_definitions bucket",
                    original_language(v)
                ),
            ));
        }
        TermStructure::Ok => {}
    }

    // ── structural definition walk (always runs; independent rules) ────────
    check_structure(v, dialect, issues);
    check_annotations(v, issues);
    check_rm_overlay(v, issues);
    check_resource_description_languages(v, issues); // VRDLA
    if let Some((src, text)) = source {
        check_object_key_unique(src, issues); // VOKU (source-level)
        check_rule_paths(v, src, text, issues); // VRRLP (raw rules text)
        if dialect == Dialect::Adl14 {
            check_concept_term_adl14(v, src, issues); // VARCN (terminology half)
        }
    }

    let basic_clean = basic.is_empty();
    issues.append(&mut basic);

    // ── terminology + code checks (gated: basic clean + terminology Ok) ────
    if basic_clean && term_status == TermStructure::Ok {
        check_terminology(v, dialect, issues);
    }
}

// ── basic identification / meta-data ──────────────────────────────────────

fn check_identification(
    v: &ArchetypeView<'_>,
    repo: Option<&ArchetypeRepository>,
    dialect: Dialect,
    out: &mut Vec<ValidationIssue>,
) {
    let h = v.archetype_id;
    let is_overlay = v.kind == crate::source::ArtefactKind::TemplateOverlay;

    // VARID: archetype identifier validity (master03 §Validity Rules) — applies
    // to all except TEMPLATE_OVERLAY (G2). The id must have all mandatory parts.
    if !is_overlay
        && (h.rm_publisher.is_empty()
            || h.rm_package.is_empty()
            || h.rm_class.is_empty()
            || h.concept_id.is_empty()
            || !is_three_part_version(&h.release_version))
    {
        out.push(ValidationIssue::new(
            ValidationCode::Varid,
            format!(
                "archetype identifier is not well-formed: publisher={:?} package={:?} class={:?} concept={:?} version={:?}",
                h.rm_publisher, h.rm_package, h.rm_class, h.concept_id, h.release_version
            ),
        ));
    }

    // VARDT: definition typename matches the RM class of the identifier
    // (master03 §Validity Rules).
    let root_rm = complex_rm_type(v.definition);
    if !root_rm.is_empty() && !h.rm_class.is_empty() && root_rm != h.rm_class {
        out.push(ValidationIssue::new(
            ValidationCode::Vardt,
            format!(
                "definition root type {root_rm:?} does not match the identifier RM class {:?}",
                h.rm_class
            ),
        ));
    }

    // VARCN: root concept code form for the specialisation level (master03
    // §Validity Rules — the FORM half; terminology-definedness is checked by
    // VATID, per the master08 grouping and the corpus `regression` oracle).
    let root_id = complex_node_id(v.definition);
    if !root_id.is_empty() && !is_root_code_at_depth(root_id, v.specialisation_level()) {
        out.push(ValidationIssue::new(
            ValidationCode::Varcn,
            format!(
                "root node id {root_id:?} is not a valid root code at specialisation depth {}",
                v.specialisation_level()
            ),
        ));
    }

    // VARAV / VARRV: adl_version / rm_release 3-part version formats (master03
    // §Validity Rules). AOM2-only: an ADL 1.4 artefact carries a `1.4`-form
    // `adl_version` (two-part, optional metadata) and NO `rm_release`, and AOM
    // 1.4 defines no 3-part-version rule for either (ADL1.4 master08 §Syntax
    // Specification, `arch_identification` meta-data; AOM1.4 master03 ARCHETYPE
    // §Invariants — version validity is only `version = archetype_id.version_id`).
    // So both are suppressed in the 1.4 dialect: applying the AOM2 rule would
    // reject every valid 1.4 archetype.
    if !is_overlay && dialect == Dialect::Adl2 {
        match v.adl_version {
            Some(a) if is_three_part_version(a) => {}
            _ => out.push(ValidationIssue::new(
                ValidationCode::Varav,
                format!(
                    "adl_version {:?} is not a valid 3-part version",
                    v.adl_version
                ),
            )),
        }
        if !is_three_part_version(v.rm_release) {
            out.push(ValidationIssue::new(
                ValidationCode::Varrv,
                format!(
                    "rm_release {:?} is not a valid 3-part version",
                    v.rm_release
                ),
            ));
        }
    }
    if !is_overlay {
        // VDEOL / VARD: original language + description present (master03
        // §Validity Rules, G2).
        if v.original_language.is_none() {
            out.push(ValidationIssue::new(
                ValidationCode::Vdeol,
                "no original_language section",
            ));
        }
        if v.description.is_none() {
            out.push(ValidationIssue::new(
                ValidationCode::Vard,
                "no description section",
            ));
        }
    }

    // VACSD / VASID: specialisation depth + parent id (master03 §Validity Rules,
    // G3). Standalone half for non-specialised; parent-dependent half via `repo`.
    check_specialisation_depth(v, repo, out);

    // VALC: language conformance to the flat parent (master03 §Validity Rules,
    // G3) — needs the parent; runs only when resolvable.
    check_language_conformance(v, repo, out);
}

/// VARCN (ADL 1.4, terminology half): the `concept` section's term must exist
/// in the archetype ontology — `ADL1.4/master08-adl.adoc` §Validity Rules
/// VARCN: "archetype concept validity. The archetype must have an archetype
/// term value in the concept section. The term must exist in the archetype
/// ontology."
///
/// The FORM half (a root code well-formed for the specialisation depth) is
/// checked for every dialect in [`check_identification`]; this half needs the
/// parsed 1.4 `concept` clause, which only the source-level entry points carry
/// (ADL2 has no concept section — `ADL2/master07.09`). A concept term is judged
/// defined when any language's `term_definitions` bucket carries it, matching
/// the definedness union [`check_terminology`] uses. An ABSENT concept section
/// is refused earlier, at assembly ([`crate::error::SyntaxErrorCode::Saco`]).
fn check_concept_term_adl14(
    v: &ArchetypeView<'_>,
    source: &SourceArtefact,
    out: &mut Vec<ValidationIssue>,
) {
    let Some(code) = source.concept.as_deref() else {
        return;
    };
    let defined = v
        .terminology
        .term_definitions
        .values()
        .any(|bucket| bucket.contains_key(code));
    if !defined {
        out.push(ValidationIssue::new(
            ValidationCode::Varcn,
            format!("concept term {code:?} is not defined in the archetype ontology"),
        ));
    }
}

/// VACSD: the specialisation depth of the archetype must be one greater than
/// the parent's (master03 §Validity Rules). A non-specialised archetype must be
/// at depth 0; a specialised one needs its parent's depth (via `repo`).
/// VASID: the parent id in the `specialise` clause must be the immediate
/// parent's id (master03 §Validity Rules).
fn check_specialisation_depth(
    v: &ArchetypeView<'_>,
    repo: Option<&ArchetypeRepository>,
    out: &mut Vec<ValidationIssue>,
) {
    let level = v.specialisation_level();
    let Some(parent_id) = v.parent_archetype_id else {
        // Not specialised — depth must be 0.
        if level != 0 {
            out.push(ValidationIssue::new(
                ValidationCode::Vacsd,
                format!(
                    "non-specialised archetype has root specialisation depth {level}, expected 0"
                ),
            ));
        }
        return;
    };

    // Specialised — a specialised archetype must be at depth >= 1 regardless.
    if level == 0 {
        out.push(ValidationIssue::new(
            ValidationCode::Vacsd,
            "specialised archetype has a level-0 root code (expected depth >= 1)",
        ));
    }

    let Some(parent) = repo.and_then(|r| r.get(parent_id)) else {
        return; // parent unresolved (missing parent is a separate concern)
    };
    let parent_view = view(parent);
    let parent_level = parent_view.specialisation_level();
    if level != parent_level + 1 {
        out.push(ValidationIssue::new(
            ValidationCode::Vacsd,
            format!("specialisation depth {level} is not one greater than the parent depth {parent_level}"),
        ));
    }
    // VASID: the stated parent id must be the immediate parent's id.
    let stated = super::raw_id_lookup_key(parent_id);
    let actual = super::hrid_lookup_key(parent_view.archetype_id);
    if stated != actual {
        out.push(ValidationIssue::new(
            ValidationCode::Vasid,
            format!("stated parent id {stated:?} is not the immediate parent id {actual:?}"),
        ));
    }
}

/// VALC: the languages of a specialised archetype must be the same as or a
/// subset of the flat parent's (master03 §Validity Rules).
///
/// The reference is the **flattened** parent's language set (`ADL2/master09.02`
/// §Differential and Flat Forms — a specialised archetype conforms to its flat
/// parent, which accumulates the whole lineage's languages). Using the parent's
/// own (un-flattened) languages would false-reject a child language inherited by
/// the parent from further up the lineage; flattening the parent avoids that.
/// The flat parent is obtained via [`crate::flatten::flat_form`]; if it cannot
/// be built (a lineage parent is missing), the check falls back to the declared
/// parent's own languages (never firing on an inherited language it cannot see).
fn check_language_conformance(
    v: &ArchetypeView<'_>,
    repo: Option<&ArchetypeRepository>,
    out: &mut Vec<ValidationIssue>,
) {
    let Some(parent_id) = v.parent_archetype_id else {
        return;
    };
    let Some(repo) = repo else {
        return;
    };
    let Some(parent) = repo.get(parent_id) else {
        return;
    };
    let flat_parent = crate::flatten::flat_form(parent, repo).ok();
    let parent_langs = match flat_parent.as_ref() {
        Some(flat) => languages(&view(flat)),
        None => languages(&view(parent)),
    };
    for lang in languages(v) {
        if !parent_langs.contains(&lang) {
            out.push(ValidationIssue::new(
                ValidationCode::Valc,
                format!("language {lang:?} is not present in the flattened parent archetype"),
            ));
        }
    }
}

// ── structural definition walk ────────────────────────────────────────────

/// Mutable state threaded through the definition walk. The structural walk only
/// raises issues; the terminology checks re-derive code usage in a second pass
/// ([`collect_usage`]), so no code sets are accumulated here.
struct Scan<'a> {
    v: &'a ArchetypeView<'a>,
    dialect: Dialect,
    issues: Vec<ValidationIssue>,
    /// node id → first path seen (VCOSU uniqueness).
    seen_node_ids: HashMap<String, String>,
}

fn check_structure(v: &ArchetypeView<'_>, dialect: Dialect, issues: &mut Vec<ValidationIssue>) {
    let mut scan = Scan {
        v,
        dialect,
        issues: Vec::new(),
        seen_node_ids: HashMap::new(),
    };
    let root = CObject::CComplexObject(v.definition.clone());
    // The root object always requires a node id (the concept code, `at0000`/
    // `id1`); child requirement is decided per owning attribute in
    // [`Scan::walk_attribute`].
    scan.walk_object("", &root, true);
    issues.append(&mut scan.issues);
}

impl Scan<'_> {
    fn push(&mut self, code: ValidationCode, msg: impl Into<String>, path: &str) {
        self.issues
            .push(ValidationIssue::new(code, msg).at_path(path.to_owned()));
    }

    fn walk_object(&mut self, path: &str, obj: &CObject, require_node_id: bool) {
        let nid = object_node_id(obj);
        let is_identified = !matches!(
            obj,
            CObject::CBoolean(_)
                | CObject::CInteger(_)
                | CObject::CReal(_)
                | CObject::CString(_)
                | CObject::CTerminologyCode(_)
                | CObject::CDate(_)
                | CObject::CTime(_)
                | CObject::CDateTime(_)
                | CObject::CDuration(_)
        );

        // VCOID: every (non-primitive) object node must have a node id
        // (master04.5 §`C_OBJECT`). In the ADL 1.4 dialect this is relaxed to
        // the AOM 1.4 node_id rule via `require_node_id` (see
        // [`Scan::walk_attribute`]): AOM1.4 master04 §Node_id and Paths + ADL1.4
        // master08 §Definition Section ("any leaf or near-leaf node which has no
        // sibling nodes from the same attribute can safely have no node_id").
        // A 1.4 `use_node` (a `C_COMPLEX_OBJECT_PROXY` / ARCHETYPE_INTERNAL_REF)
        // is a *reference* to another node, not a node definition, and carries
        // no node id of its own in 1.4 (unlike ADL2's `use_node TYPE[id]`), so
        // it is exempt in the 1.4 dialect (AOM1.4 master04 §Node_id and Paths).
        let is_proxy_ref =
            self.dialect == Dialect::Adl14 && matches!(obj, CObject::CComplexObjectProxy(_));
        if is_identified && nid.is_empty() && require_node_id && !is_proxy_ref {
            self.push(
                ValidationCode::Vcoid,
                "object node has no node identifier",
                path,
            );
        }
        // VCOSU: object node ids must be unique archetype-wide (master04.5
        // §`C_OBJECT`). Synthetic primitive ids are exempt. Deferred for a
        // specialised archetype: a differential legitimately re-references an
        // inherited node id at a redefinition, so uniqueness is a flat-form
        // property (run on the flattened form in [`super::phase_flat`]). AOM2-only:
        // AOM 1.4 node ids are only *sibling*-unique (AOM1.4 master04 §Node_id and
        // Paths — "guarantees sibling node unique identification"), so a valid 1.4
        // archetype may repeat an at-code at non-sibling paths; the archetype-wide
        // check is skipped in the 1.4 dialect, which instead gets the
        // sibling-scoped check in [`Scan::walk_complex`].
        if is_identified
            && !nid.is_empty()
            && !self.v.is_specialised()
            && self.dialect == Dialect::Adl2
        {
            if let Some(first) = self.seen_node_ids.get(nid) {
                let dup = format!("node id {nid:?} is not unique (also at {first})");
                self.push(ValidationCode::Vcosu, dup, path);
            } else {
                self.seen_node_ids.insert(nid.to_owned(), path.to_owned());
            }
        }

        match obj {
            CObject::CComplexObject(cco) => self.walk_complex(path, cco),
            CObject::ArchetypeSlot(slot) => self.check_slot(path, slot),
            CObject::CTerminologyCode(tc) => {
                // VATCV (code form) applies only to ADL2 constraint codes; the
                // ADL 1.4 dialect preserves 1.4 terminology constraints
                // verbatim (`local, at0004`, `[openehr::524]`, listed forms) in
                // the constraint string — these are not ADL2 code forms, and
                // their validity is ontology-definedness (ADL1.4 master08
                // §Local Constraint Codes / VATDF/VACDF), not the ADL2 regex.
                if self.dialect == Dialect::Adl2 {
                    self.check_terminology_code_form(path, &tc.constraint);
                }
            }
            CObject::CBoolean(_)
            | CObject::CInteger(_)
            | CObject::CReal(_)
            | CObject::CString(_)
            | CObject::CDate(_)
            | CObject::CTime(_)
            | CObject::CDateTime(_)
            | CObject::CDuration(_) => self.check_primitive_assumed(path, obj),
            // NOTE: VUNP (`C_COMPLEX_OBJECT_PROXY` target-path validity) is a
            // flat-form (phase-3) check — a proxy target may be assembled from
            // several specialisation levels — so it runs in [`super::phase3`]
            // against the flattened form (`master08` §Phase 3; `master04.5`
            // §`C_COMPLEX_OBJECT_PROXY` VUNP L482-483), not in the phase-1 walk.
            CObject::CComplexObjectProxy(_) => {}
        }
    }

    fn walk_complex(&mut self, path: &str, cco: &CComplexObject) {
        // VARXNC / VARXAV / VARXTV: `C_ARCHETYPE_ROOT` validity (master08 §Phase 1
        // §Various Structure Validation).
        //
        // NOTE: VARXR (external-reference *resolution*) is a phase-2 check that
        // needs the supplier repository, so it is not run in the standalone
        // phase-1 walk: a `C_ARCHETYPE_ROOT` filling a parent slot is resolved by
        // the specialisation validator ([`super::phase2::check_slot_filler`],
        // `master04.5` §`C_ARCHETYPE_ROOT`) and a `use_archetype` filler by
        // [`super::fillers::validate_fillers`] (`master08` §Phase 2).
        if let CComplexObject::CArchetypeRoot(r) = cco {
            if r.node_id.is_empty() {
                self.push(
                    ValidationCode::Varxnc,
                    "C_ARCHETYPE_ROOT has no node id",
                    path,
                );
            }
            if r.rm_type_name.is_empty() {
                self.push(
                    ValidationCode::Varxtv,
                    "C_ARCHETYPE_ROOT has no RM type",
                    path,
                );
            }
            if !r.archetype_ref.is_empty() && !is_archetype_id(&r.archetype_ref) {
                self.push(
                    ValidationCode::Varxav,
                    format!(
                        "C_ARCHETYPE_ROOT reference {:?} is not a valid archetype id",
                        r.archetype_ref
                    ),
                    path,
                );
            }
        }

        // VCATU: sibling attributes uniquely named (master04.5 §`C_COMPLEX_OBJECT`).
        let mut seen_attrs = BTreeSet::new();
        for attr in complex_attributes(cco) {
            if !seen_attrs.insert(attr.rm_attribute_name.as_str()) {
                self.push(
                    ValidationCode::Vcatu,
                    format!(
                        "attribute {:?} is defined more than once",
                        attr.rm_attribute_name
                    ),
                    path,
                );
            }
        }

        for attr in complex_attributes(cco) {
            // VCOSU (AOM 1.4 sibling scope): in the 1.4 dialect node ids are only
            // *sibling*-unique — children under the same container attribute must
            // have distinct node ids (AOM1.4 master04 §Node_id and Paths —
            // "guarantees sibling node unique identification"). ADL2 uses the
            // stronger archetype-wide uniqueness (walk_object above / phase_flat),
            // so this sibling-scoped pass is 1.4-only.
            if self.dialect == Dialect::Adl14 {
                let mut sibling_ids: BTreeSet<&str> = BTreeSet::new();
                for child in &attr.children {
                    let cid = object_node_id(child);
                    if !cid.is_empty()
                        && (is_id_code(cid) || is_at_code(cid))
                        && !sibling_ids.insert(cid)
                    {
                        let cpath = child_path(&format!("{path}/{}", attr.rm_attribute_name), cid);
                        self.push(
                            ValidationCode::Vcosu,
                            format!("node id {cid:?} is not unique among siblings"),
                            &cpath,
                        );
                    }
                }
            }
            self.walk_attribute(path, attr);
        }
    }

    fn walk_attribute(&mut self, parent_path: &str, attr: &CAttribute) {
        let attr_path = format!("{parent_path}/{}", attr.rm_attribute_name);

        // VDIFV: a differential path is only valid in a specialised archetype
        // (master04.5 §`C_ATTRIBUTE`).
        if attr.differential_path.is_some() && !self.v.is_specialised() {
            self.push(
                ValidationCode::Vdifv,
                "differential path in a non-specialised archetype",
                &attr_path,
            );
        }

        // VACMCU/WACMCL compare a child's occurrences against its owning
        // attribute's *stated* cardinality (master04.5 §`C_ATTRIBUTE`). They run
        // only when a cardinality is present (which reliably means the attribute
        // is a container) and the archetype is its own flat form — a specialised
        // archetype may not restate the inherited cardinality.
        //
        // NOTE: VACSO ("child of a single-valued attribute cannot have
        // occurrences upper > 1") is a reference-model check — a single-valued
        // attribute is `C_ATTRIBUTE._is_multiple_` False, an RM-derived property
        // the parser's `is_multiple = cardinality present` heuristic cannot
        // supply (it misclassifies e.g. `CLUSTER.items`); it runs in
        // [`super::rm`]. For a specialised archetype VACMCU/WACMCL run on the
        // flattened form ([`super::phase_flat`]) — a differential may not restate
        // the inherited cardinality — so they are gated to the non-specialised
        // (own-flat-form) case here.
        if !self.v.is_specialised() && attr.is_multiple {
            self.check_container_cardinality(&attr_path, attr);
        }

        // VCOC is the ADL 1.4 formalism's own cardinality/occurrences rule,
        // distinct from the AOM2 VACMCU/WACMCL pair above, so it runs on the 1.4
        // dialect only. Gated to the non-specialised (own-flat-form) case for the
        // same reason as VACMCU: a specialised archetype need not restate the
        // inherited cardinality, so the sums here would be computed against a
        // cardinality that is not the effective one.
        if self.dialect == Dialect::Adl14 && !self.v.is_specialised() {
            self.check_vcoc(&attr_path, attr);
        }

        // Whether a child object is required to carry a node id. AOM2 requires
        // one on every non-primitive object (master04.5 §`C_OBJECT`); AOM 1.4
        // requires one only for children of a container (multiple) attribute —
        // "any leaf or near-leaf node which has no sibling nodes from the same
        // attribute can safely have no node_id" (AOM1.4 master04 §Node_id and
        // Paths; ADL1.4 master08 §Definition Section).
        let require_child_node_id = match self.dialect {
            Dialect::Adl2 => true,
            Dialect::Adl14 => attr.is_multiple,
        };
        for child in &attr.children {
            let cpath = child_path(&attr_path, object_node_id(child));
            self.walk_object(&cpath, child, require_child_node_id);
        }
    }

    /// VACMCU (error) + WACMCL (warning): container cardinality vs child
    /// occurrences (master04.5 §`C_ATTRIBUTE`).
    fn check_container_cardinality(&mut self, attr_path: &str, attr: &CAttribute) {
        let Some(card) = attr.cardinality.as_ref() else {
            return;
        };
        let Some(card_upper) = occurrences_upper_finite(Some(&card.interval)) else {
            return; // open cardinality upper — nothing to bound
        };
        let mut sum_lower = 0i64;
        for child in &attr.children {
            let Some(occ) = object_occurrences(child) else {
                continue;
            };
            // VACMCU: a finite child occurrences upper must be <= cardinality upper.
            if let Some(u) = occurrences_upper_finite(Some(occ))
                && i64::from(u) > i64::from(card_upper)
            {
                self.push(
                    ValidationCode::Vacmcu,
                    format!(
                        "child occurrences upper {u} exceeds the cardinality upper {card_upper}"
                    ),
                    attr_path,
                );
            }
            sum_lower += i64::from(occurrences_lower(occ));
        }
        // WACMCL: the sum of child occurrences lowers should be below the
        // cardinality upper (advisory warning).
        if sum_lower > i64::from(card_upper) {
            self.push(
                ValidationCode::Wacmcl,
                format!("sum of child occurrences lowers {sum_lower} exceeds the cardinality upper {card_upper}"),
                attr_path,
            );
        }
    }

    /// VCOC (ADL 1.4): cardinality/occurrences validity —
    /// `ADL1.4/master05-cadl.adoc` §Occurrences L321-324: "the interval
    /// represented by: (the sum of all occurrences minimum values) .. (the sum of
    /// all occurrences maximum values) must be inside the interval of the
    /// cardinality."
    ///
    /// The children's occurrences are the EFFECTIVE ones
    /// ([`effective_occurrences_adl14`]): a child with no stated `occurrences` is
    /// `{1..1}` (master05 L316), and a `use_node` with none takes the referenced
    /// node's (master05 L515) — without those defaults the sums would be computed
    /// from zero-width intervals and the rule would never see a real archetype's
    /// child set.
    ///
    /// NOTE (adjudication): the sum-LOWER half of the literal containment — that
    /// `cardinality.lower <= Σ occurrences.lower` — is NOT raised, because
    /// openEHR's own regression corpus contradicts it: `aa.v1.adl`,
    /// `dimensions.v1.adl` and their siblings are tagged `regression = "PASS"`
    /// while pairing `items cardinality {1..*}` with a single child of
    /// `occurrences {0..1}` (Σ lower = 0 < 1). AOM 2, where this rule's successor
    /// lives, splits VCOC the same way: the upper-bound half is the ERROR VACMCU
    /// and the lower-bound half is the WARNING WACMCL
    /// (`AOM2/master04.5-constraint_model-class_definitions.adoc` §Validity Rules:
    /// `C_ATTRIBUTE`). What IS raised is every part of the containment failure that
    /// is a genuine defect and corpus-consistent: a sum whose maximum exceeds the
    /// cardinality upper (the child set can overfill the container) and a sum whose
    /// maximum cannot reach the cardinality lower (the container's minimum is
    /// unsatisfiable). Both are strict cases of "not inside the interval of the
    /// cardinality".
    fn check_vcoc(&mut self, attr_path: &str, attr: &CAttribute) {
        let Some(card) = attr.cardinality.as_ref() else {
            return; // no cardinality ⇒ not a container ⇒ VCOC does not apply.
        };
        if attr.children.is_empty() {
            return;
        }
        let card_lower = occurrences_lower(&card.interval);
        let card_upper = occurrences_upper_finite(Some(&card.interval));

        // Σ of the children's effective occurrences maxima; `None` = unbounded
        // (any child with an open upper makes the sum open).
        let mut sum_upper: Option<i64> = Some(0);
        for child in &attr.children {
            let occ = effective_occurrences_adl14(self.v.definition, child);
            match (sum_upper, occ.upper) {
                (Some(sum), Some(u)) => sum_upper = Some(sum + i64::from(u)),
                (_, None) => sum_upper = None,
                (None, _) => {}
            }
        }
        let Some(sum_upper) = sum_upper else {
            return; // an unbounded sum is inside any cardinality with an open upper
        };

        if let Some(cu) = card_upper
            && sum_upper > i64::from(cu)
        {
            self.push(
                ValidationCode::Vcoc,
                format!(
                    "the sum of the children's occurrences maxima ({sum_upper}) is outside the \
                     cardinality {card_lower}..{cu}"
                ),
                attr_path,
            );
        }
        if sum_upper < i64::from(card_lower) {
            self.push(
                ValidationCode::Vcoc,
                format!(
                    "the sum of the children's occurrences maxima ({sum_upper}) cannot reach the \
                     cardinality lower bound ({card_lower})"
                ),
                attr_path,
            );
        }
    }

    fn check_slot(&mut self, path: &str, slot: &ArchetypeSlot) {
        // VDSEV / VDSIV: slot include/exclude consistency (master04.5
        // §`ARCHETYPE_SLOT`, the verbatim Eiffel if/elseif chain — exactly one
        // branch fires):
        //
        //   if      includes not empty and =  any then not (excludes empty or /= any) ==> VDSEV
        //   elseif  includes not empty and /= any then not (excludes empty or =  any) ==> VDSEV
        //   elseif  excludes not empty and =  any then not (includes empty or /= any) ==> VDSIV
        //   elseif  excludes not empty and /= any then not (includes empty or =  any) ==> VDSIV
        //
        // NOTE: with the include-side branches evaluated first, VDSIV is only
        // reachable when `includes` is empty — in which case its own guard
        // ("includes empty") makes the condition false. So on a real slot (which
        // always has an `include`) every inconsistency reports as VDSEV; VDSIV
        // is defined by the spec but structurally unreachable through this table.
        let inc_empty = slot.includes.is_empty();
        let exc_empty = slot.excludes.is_empty();
        let inc_any = !inc_empty && slot.includes.iter().all(is_any_assertion);
        let exc_any = !exc_empty && slot.excludes.iter().all(is_any_assertion);

        // A real slot always carries an `include`, so only the include-side
        // branches of the spec table are reachable (see the NOTE above): a
        // violation is reported as VDSEV whether the offending pairing is an
        // any-include/any-exclude or a specific-include/specific-exclude.
        if !inc_empty {
            let contradictory = if inc_any {
                exc_non_any_and_any(exc_empty, exc_any)
            } else {
                !(exc_empty || exc_any)
            };
            if contradictory {
                self.push(
                    ValidationCode::Vdsev,
                    "slot 'include' and 'exclude' constraints are contradictory",
                    path,
                );
            }
        }

        // VDFAI: archetype ids in slot assertions must be valid (master04.5
        // §`ARCHETYPE_SLOT`).
        for a in slot.includes.iter().chain(slot.excludes.iter()) {
            for id in assertion_archetype_ids(a) {
                if !is_archetype_id(&id) {
                    self.push(
                        ValidationCode::Vdfai,
                        format!("slot assertion references an invalid archetype id {id:?}"),
                        path,
                    );
                }
            }
        }
    }

    /// VATCV: a terminology constraint code must be a well-formed code
    /// (master08 §Code Validation; NOTE-flagged, no full vendored text). The
    /// definedness / value-set / assumed-value checks run in the gated
    /// terminology pass ([`check_terminology`]).
    fn check_terminology_code_form(&mut self, path: &str, constraint: &str) {
        // Strip an optional operational `@terminology` binding suffix.
        let code = constraint.split('@').next().unwrap_or(constraint).trim();
        if !code.is_empty() && !is_valid_code(code) {
            self.push(
                ValidationCode::Vatcv,
                format!("terminology constraint code {code:?} is not a valid code"),
                path,
            );
        }
    }

    /// VOBAV: a primitive assumed value must fall within its own constraint
    /// (master04.5 §`C_PRIMITIVE_OBJECT`).
    ///
    /// The enumerable primitives (Boolean / String) test list membership; the
    /// ordered primitives (Integer / Real and the temporal `Iso8601_*` types)
    /// test point-in-interval containment (`master04.5` §`C_ORDERED` — the value
    /// space is a list of `Interval`s, `has` = a point falls in some interval).
    /// A primitive with an empty constraint (`any_allowed`) admits any assumed
    /// value. The temporal branches use `Interval::has_definite` so an
    /// incomparable (undecidable) value never raises a false violation.
    fn check_primitive_assumed(&mut self, path: &str, obj: &CObject) {
        match obj {
            CObject::CBoolean(b) => {
                if let Some(av) = b.assumed_value
                    && !b.constraint.is_empty()
                    && !b.constraint.contains(&av)
                {
                    self.push(
                        ValidationCode::Vobav,
                        "boolean assumed value is not in the constraint",
                        path,
                    );
                }
            }
            CObject::CInteger(i) => {
                // The generated model types the integer assumed value as `f64`;
                // a valid integer assumed value is a whole number lying in some
                // constraint interval.
                if let Some(av) = i.assumed_value
                    && !i.constraint.is_empty()
                {
                    #[expect(clippy::cast_possible_truncation, reason = "guarded by fract()")]
                    let inside =
                        av.fract() == 0.0 && i.constraint.iter().any(|iv| iv.has(&(av as i32)));
                    if !inside {
                        self.push(
                            ValidationCode::Vobav,
                            "integer assumed value is not within any constraint interval",
                            path,
                        );
                    }
                }
            }
            CObject::CReal(r) => {
                if let Some(av) = r.assumed_value
                    && !r.constraint.is_empty()
                    && !r.constraint.iter().any(|iv| iv.has(&av))
                {
                    self.push(
                        ValidationCode::Vobav,
                        "real assumed value is not within any constraint interval",
                        path,
                    );
                }
            }
            // The temporal primitives carry their assumed value + constraint as
            // `Interval<Iso8601_*>`; `openehr-base` now provides ISO 8601 ordering
            // (`PartialOrd`), so the same point-in-interval VOBAV test applies.
            // Temporal values are only partially ordered (partial dates,
            // timezone normalisation), so use `has_definite`: a violation is
            // raised only when the assumed value is definitely outside EVERY
            // interval; an incomparable (undecidable) pairing leaves containment
            // unknown and never raises.
            CObject::CDate(d) => {
                if let Some(av) = &d.assumed_value
                    && temporal_assumed_violates(&d.constraint, av)
                {
                    self.push(
                        ValidationCode::Vobav,
                        "date assumed value is not within any constraint interval",
                        path,
                    );
                }
            }
            CObject::CTime(t) => {
                if let Some(av) = &t.assumed_value
                    && temporal_assumed_violates(&t.constraint, av)
                {
                    self.push(
                        ValidationCode::Vobav,
                        "time assumed value is not within any constraint interval",
                        path,
                    );
                }
            }
            CObject::CDateTime(dt) => {
                if let Some(av) = &dt.assumed_value
                    && temporal_assumed_violates(&dt.constraint, av)
                {
                    self.push(
                        ValidationCode::Vobav,
                        "date/time assumed value is not within any constraint interval",
                        path,
                    );
                }
            }
            CObject::CDuration(du) => {
                if let Some(av) = &du.assumed_value
                    && temporal_assumed_violates(&du.constraint, av)
                {
                    self.push(
                        ValidationCode::Vobav,
                        "duration assumed value is not within any constraint interval",
                        path,
                    );
                }
            }
            CObject::CString(s) => {
                if let Some(av) = &s.assumed_value
                    && !s.constraint.is_empty()
                    && !s.constraint.iter().any(|c| c == av)
                {
                    self.push(
                        ValidationCode::Vobav,
                        "string assumed value is not in the constraint list",
                        path,
                    );
                }
            }
            _ => {}
        }
    }
}

/// VOBAV for an ordered temporal primitive: a non-empty constraint list is
/// violated only when the assumed value is *definitely* outside every interval
/// (`Interval::has_definite` returns `Some(false)` for all of them). An
/// undecidable pairing (`None`, e.g. a partial date whose completions overlap a
/// bound) leaves containment unknown and never raises — mirroring the numeric
/// `has`-based check while staying conservative under partial order.
fn temporal_assumed_violates<T: PartialOrd>(constraint: &[Interval<T>], av: &T) -> bool {
    !constraint.is_empty()
        && constraint
            .iter()
            .all(|iv| iv.has_definite(av) == Some(false))
}

// ── terminology checks (gated) ────────────────────────────────────────────

// One orchestration function covering the whole terminology/code catalogue; the
// individual rules are extracted into helpers below, so the length is inherent
// to the number of codes checked in sequence.
#[expect(
    clippy::too_many_lines,
    reason = "the individual rules are already extracted into helpers below; the remaining length is the number of terminology codes checked in sequence"
)]
fn check_terminology(v: &ArchetypeView<'_>, dialect: Dialect, issues: &mut Vec<ValidationIssue>) {
    let term = v.terminology;
    let level = v.specialisation_level();

    // Union of all defined codes across languages (a code defined in any
    // language counts as defined for the definedness checks).
    let defined: BTreeSet<&str> = term
        .term_definitions
        .values()
        .flat_map(|m| m.keys().map(String::as_str))
        .collect();

    // Re-scan the definition for referenced codes (usage + assumed values).
    let mut usage = CodeUsage::default();
    let root = CObject::CComplexObject(v.definition.clone());
    collect_usage(&root, &mut usage);

    // VATID: the root concept code must be defined in the terminology (master08
    // §Code Validation; NOTE-flagged, no full vendored text).
    //
    // NOTE: the per-node id-code definedness half is a reference-model check —
    // whether an interior node id-code must be defined depends on the RM
    // multiplicity of its owning attribute (master07 §Overview: "for nodes that
    // are children of single-valued attribute, a term definition is optional");
    // it runs in [`super::rm`]. Phase-1 checks only the always-local root
    // concept code.
    let root_id = complex_node_id(v.definition);
    if !root_id.is_empty()
        && (is_id_code(root_id) || is_at_code(root_id))
        && !defined.contains(root_id)
    {
        issues.push(ValidationIssue::new(
            ValidationCode::Vatid,
            format!("root concept code {root_id:?} is not defined in the terminology"),
        ));
    }

    // VATDF (ADL 1.4, node-id half): in ADL 1.4 EVERY at-code used as a node
    // identifier in the definition must be defined in the ontology's
    // term_definitions (ADL1.4 master08 §Validity Rules VATDF; AOM1.4
    // `ARCHETYPE.node_ids_valid`). ADL2 defers the interior-node-id definedness
    // to the RM phase (the master07 single-valued-attribute optionality above),
    // but the 1.4 formalism has no such optionality for a code that IS present —
    // "each archetype term used as a node identifier … must be defined". The
    // 1.4 phase-1 subset runs phase 1 only, so this closes VATDF's interior half
    // for a 1.4 upload (`used ⇒ defined`; a non-specialised 1.4 archetype is its
    // own flat form).
    if dialect == Dialect::Adl14 && !v.is_specialised() {
        for code in &usage.node_codes {
            if is_at_code(code) && !defined.contains(code.as_str()) {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vatdf,
                    format!("node identifier code {code:?} is not defined in the terminology"),
                ));
            }
        }
    }

    // VATDF: at-codes used in term constraints defined in the terminology of the
    // flattened form (master03 §Validity Rules). For a specialised archetype the
    // flat form is not available here, so this runs only when the archetype
    // is its own flat form (non-specialised); the specialised flat-form half runs
    // in [`super::phase_flat`].
    // VACDF: ac-codes defined in the current archetype (master03 — "current",
    // not flattened; runs for all). VATCD: code level <= archetype level.
    let flat_self = !v.is_specialised();
    for code in &usage.value_codes {
        if is_at_code(code) {
            if flat_self && !defined.contains(code.as_str()) {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vatdf,
                    format!("value code {code:?} is not defined in the terminology"),
                ));
            }
        } else if is_ac_code(code) && !defined.contains(code.as_str()) {
            issues.push(ValidationIssue::new(
                ValidationCode::Vacdf,
                format!("constraint code {code:?} is not defined in the terminology"),
            ));
        }
        // VATCD: at/id codes at a level greater than the archetype level.
        if !is_ac_code(code)
            && let Some(d) = codes::specialisation_depth(code)
            && d > level
        {
            issues.push(ValidationIssue::new(
                ValidationCode::Vatcd,
                format!("code {code:?} has specialisation level {d} > archetype level {level}"),
            ));
        }
    }

    // VATDA: an assumed value at-code must be a member of the referenced value
    // set (master03 §Validity Rules).
    for (path, ac, assumed) in &usage.assumed_refs {
        let members = term
            .value_sets
            .as_ref()
            .and_then(|vs| vs.get(ac))
            .is_some_and(|vs| vs.members.iter().any(|m| m == assumed));
        if !members {
            issues.push(
                ValidationIssue::new(
                    ValidationCode::Vatda,
                    format!("assumed value {assumed:?} is not a member of value set {ac:?}"),
                )
                .at_path(path.clone()),
            );
        }
    }

    // VTSD: every defined term/constraint code is at the archetype's
    // specialisation level (differential) or the same-or-less (flat)
    // (master07 §Validity Rules). ac-codes are a flat code space (master07
    // §Specialisation Depth) so only an over-level ac-code is invalid, never the
    // strict differential-equality test.
    for code in &defined {
        if let Some(d) = codes::specialisation_depth(code) {
            // A 1.4 specialised archetype is a FLAT artefact (its ontology
            // legitimately carries inherited codes at lower levels alongside
            // the level-N additions), even though the 1.4-shaped model is
            // marked `is_differential` for the converter's re-differentiation
            // pass. So the 1.4 dialect always uses the flat-form rule
            // (`d <= level`), never the differential `d == level`
            // (AOM1.4 master07 §Specialisation Depth).
            let differential = v.is_differential && dialect == Dialect::Adl2;
            let bad = if is_ac_code(code) {
                d > level
            } else if differential {
                d != level
            } else {
                d > level
            };
            if bad {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vtsd,
                    format!("terminology code {code:?} specialisation level {d} is invalid for archetype level {level}"),
                ));
            }
        }
    }

    // VATCV (defined-code form): every defined code must be a valid code form
    // (master08 §Code Validation). Value-code form on definition-referenced
    // codes is covered in the walk.
    for code in &defined {
        if !is_valid_code(code) {
            issues.push(ValidationIssue::new(
                ValidationCode::Vatcv,
                format!("terminology code {code:?} is not a valid code form"),
            ));
        }
    }

    // VTLC: every code defined in one language must be defined in all languages
    // (master07 §Validity Rules).
    check_language_coverage(term, issues);

    // VOTM: every language declared in description/translations must have
    // term_definitions (master03 §Validity Rules).
    for l in languages(v) {
        if !term.term_definitions.contains_key(&l) {
            issues.push(ValidationIssue::new(
                ValidationCode::Votm,
                format!("language {l:?} has no term_definitions"),
            ));
        }
    }

    // VTVSID / VTVSMD / VTVSUQ: value-set integrity (master07 §Validity Rules).
    check_value_sets(term, &defined, !v.is_specialised(), issues);

    // VTTBK / VTCBK: term/constraint binding key validity (master07 §Validity
    // Rules).
    check_bindings(v, &defined, issues);

    // WOUC: a defined at/ac code that is never used in the definition (archie
    // parity; no openEHR spec governs this — our own design/extension).
    // Suppressed in the 1.4 dialect: 1.4 value codes are carried inside the
    // verbatim terminology-constraint strings (not recognised as ADL2 code
    // usage), so the "unused" heuristic is unreliable on a 1.4-shaped model and
    // would flag legitimately-used codes.
    if dialect == Dialect::Adl2 {
        let mut used_all: BTreeSet<&str> = usage.value_codes.iter().map(String::as_str).collect();
        used_all.extend(usage.node_codes.iter().map(String::as_str));
        // value-set membership also counts as "use" of a member at-code.
        if let Some(vs) = term.value_sets.as_ref() {
            for set in vs.values() {
                used_all.insert(set.id.as_str());
                for m in &set.members {
                    used_all.insert(m.as_str());
                }
            }
        }
        for code in &defined {
            // The root concept code and id-code node ids are structural, not
            // "unused" terms; WOUC targets value at-codes and ac-codes.
            if (is_at_code(code) || is_ac_code(code))
                && *code != complex_node_id(v.definition)
                && !used_all.contains(code)
            {
                issues.push(ValidationIssue::new(
                    ValidationCode::Wouc,
                    format!("terminology code {code:?} is defined but unused in the definition"),
                ));
            }
        }
    }
}

fn check_language_coverage(
    term: &openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology,
    issues: &mut Vec<ValidationIssue>,
) {
    let langs: Vec<&String> = term.term_definitions.keys().collect();
    if langs.len() < 2 {
        return;
    }
    let all_codes: BTreeSet<&str> = term
        .term_definitions
        .values()
        .flat_map(|m| m.keys().map(String::as_str))
        .collect();
    for (lang, codes) in &term.term_definitions {
        for code in &all_codes {
            if !codes.contains_key(*code) {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vtlc,
                    format!("code {code:?} is missing in language {lang:?}"),
                ));
            }
        }
    }
}

fn check_value_sets(
    term: &openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology,
    defined: &BTreeSet<&str>,
    flat_self: bool,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(vs) = term.value_sets.as_ref() else {
        return;
    };
    for set in vs.values() {
        // VTVSID: the value-set id must be defined in the terminology of the
        // current archetype (master07 — "current", runs for all).
        if !defined.contains(set.id.as_str()) {
            issues.push(ValidationIssue::new(
                ValidationCode::Vtvsid,
                format!(
                    "value set id {:?} is not defined in the terminology",
                    set.id
                ),
            ));
        }
        // VTVSUQ: members must be unique within the value set.
        let mut seen = BTreeSet::new();
        for m in &set.members {
            if !seen.insert(m.as_str()) {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vtvsuq,
                    format!("value set {:?} has a duplicate member {m:?}", set.id),
                ));
            }
        }
        // VTVSMD: members must be defined in the terminology of the *flattened*
        // form (master07). Runs only when the archetype is its own flat form; the
        // specialised flat-form half runs in [`super::phase_flat`].
        if flat_self {
            for m in &set.members {
                if !defined.contains(m.as_str()) {
                    issues.push(ValidationIssue::new(
                        ValidationCode::Vtvsmd,
                        format!(
                            "value set {:?} member {m:?} is not defined in the terminology",
                            set.id
                        ),
                    ));
                }
            }
        }
    }
}

fn check_bindings(
    v: &ArchetypeView<'_>,
    defined: &BTreeSet<&str>,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(bindings) = v.terminology.term_bindings.as_ref() else {
        return;
    };
    for terms in bindings.values() {
        for key in terms.keys() {
            if is_ac_code(key) {
                // VTCBK: a constraint (ac) binding key must be a defined ac-code.
                if !defined.contains(key.as_str()) {
                    issues.push(ValidationIssue::new(
                        ValidationCode::Vtcbk,
                        format!("constraint binding key {key:?} is not a defined ac-code"),
                    ));
                }
            } else if is_at_code(key) || is_id_code(key) {
                // VTTBK: a term binding key must be a defined at-code.
                if !defined.contains(key.as_str()) {
                    issues.push(ValidationIssue::new(
                        ValidationCode::Vttbk,
                        format!("term binding key {key:?} is not a defined at-code"),
                    ));
                }
            } else if !key.starts_with('/') {
                // VTTBK: a non-code key that is not even a path (a bare word) is
                // never a valid binding target (master07 §Validity Rules).
                issues.push(ValidationIssue::new(
                    ValidationCode::Vttbk,
                    format!("term binding key {key:?} is neither an at-code nor a path"),
                ));
            } else if has_node_id_predicate(key) {
                // VTTBK: a node-id-predicated path must resolve within the
                // archetype (master07 §Validity Rules). A pure-RM path (no
                // predicate) is a reference-model concern (`super::rm`).
                if resolve(v.definition, key) != Resolution::Found {
                    issues.push(ValidationIssue::new(
                        ValidationCode::Vttbk,
                        format!("term binding key path {key:?} is not valid in the archetype"),
                    ));
                }
            }
        }
    }
}

// ── annotations / rm_overlay / rules ──────────────────────────────────────

/// VRANP: each annotation path must be a valid archetype path or an RM path
/// valid for the root class (master03 §Validity Rules).
///
/// NOTE: only paths carrying a node-id predicate are resolved against the
/// archetype here; a pure reference-model path (no `[id…]` predicate) is a
/// reference-model question (`super::rm`).
fn check_annotations(v: &ArchetypeView<'_>, issues: &mut Vec<ValidationIssue>) {
    let Some(annotations) = v.annotations else {
        return;
    };
    for paths in annotations.documentation.values() {
        for path in paths.keys() {
            if has_node_id_predicate(path) && resolve(v.definition, path) != Resolution::Found {
                issues.push(
                    ValidationIssue::new(
                        ValidationCode::Vranp,
                        format!("annotation path {path:?} is not valid in the archetype"),
                    )
                    .at_path(path.clone()),
                );
            }
        }
    }
}

/// VRDLA: resource-description language-code consistency — the `language` code
/// declared inside a `description.details` / `translations` block must match
/// the block's own language key (e.g. a `["zh-cn"]` block whose inner
/// `language` is `zh` is inconsistent).
///
/// NOTE: no openEHR spec governs this — our own design/extension (archie
/// `ErrorType.VRDLA` parity, adjudicated from `validity/basics`); it has no
/// full vendored AOM2 text.
fn check_resource_description_languages(v: &ArchetypeView<'_>, issues: &mut Vec<ValidationIssue>) {
    if let Some(desc) = v.description
        && let Some(details) = desc.details.as_ref()
    {
        for (key, item) in details {
            if !item.language.code_string.is_empty() && item.language.code_string != *key {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vrdla,
                    format!(
                        "description details block {key:?} declares inconsistent language {:?}",
                        item.language.code_string
                    ),
                ));
            }
        }
    }
    if let Some(translations) = v.translations {
        for (key, tr) in translations {
            if !tr.language.code_string.is_empty() && tr.language.code_string != *key {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vrdla,
                    format!(
                        "translation block {key:?} declares inconsistent language {:?}",
                        tr.language.code_string
                    ),
                ));
            }
        }
    }
}

/// VRMVP / VRMVAV: `rm_overlay` visibility path + alias validity (master06
/// §Validity). The path's node-id-predicated part must resolve; the alias must
/// be a defined at-code.
///
/// NOTE: the pure-RM tail of a visibility path is a reference-model concern
/// (`super::rm`).
fn check_rm_overlay(v: &ArchetypeView<'_>, issues: &mut Vec<ValidationIssue>) {
    let Some(overlay) = v.rm_overlay else {
        return;
    };
    let Some(map) = overlay.rm_visibility.as_ref() else {
        return;
    };
    let defined: BTreeSet<&str> = v
        .terminology
        .term_definitions
        .values()
        .flat_map(|m| m.keys().map(String::as_str))
        .collect();
    for (path, vis) in map {
        if has_node_id_predicate(path) && resolve(v.definition, path) == Resolution::NotFound {
            issues.push(
                ValidationIssue::new(
                    ValidationCode::Vrmvp,
                    format!("rm_visibility path {path:?} is not valid in the archetype"),
                )
                .at_path(path.clone()),
            );
        }
        if let Some(alias) = vis.alias.as_ref() {
            let code = &alias.code_string;
            if !defined.contains(code.as_str()) {
                issues.push(ValidationIssue::new(
                    ValidationCode::Vrmvav,
                    format!("rm_visibility alias {code:?} is not a defined at-code"),
                ));
            }
        }
    }
}

/// VRRLP: each path mentioned in a rule must be found within the archetype
/// (master03 §Validity Rules).
///
/// NOTE: implemented by scanning the raw `rules` section text for node-id-
/// predicated path literals and resolving them; the RM-valid-extension half is
/// a reference-model concern (`super::rm`). Pure-RM rule paths are accepted
/// here.
fn check_rule_paths(
    v: &ArchetypeView<'_>,
    src: &SourceArtefact,
    source_text: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(span) = src.rules.as_ref() else {
        return;
    };
    let Some(text) = source_text.get(span.bytes.clone()) else {
        return;
    };
    for path in scan_predicated_paths(text) {
        if resolve(v.definition, &path) == Resolution::NotFound {
            issues.push(
                ValidationIssue::new(
                    ValidationCode::Vrrlp,
                    format!("rule path {path:?} is not found within the archetype"),
                )
                .at_path(path.clone()),
            );
        }
    }
}

/// VOKU: within any ODIN keyed list, each item must have a unique key
/// (master03 §Validity Rules). Checked on the raw parsed ODIN (the assembled
/// model's `BTreeMap`s already dedupe keys).
fn check_object_key_unique(src: &SourceArtefact, issues: &mut Vec<ValidationIssue>) {
    for section in [
        // The `language` section is keyed too — `master07.07` types
        // `translations` as `Hash<TRANSLATION_DETAILS, String>`, so a repeated
        // `translations = <["de"] = <…> ["de"] = <…>>` key is the same VOKU
        // violation as a repeated terminology code, and went unreported while
        // this section was left out of the scanned set.
        src.language.as_ref(),
        src.description.as_ref(),
        src.terminology.as_ref(),
        src.annotations.as_ref(),
        src.rm_overlay.as_ref(),
        src.component_terminologies.as_ref(),
        src.revision_history.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        check_odin_key_unique(section, issues);
    }
}

fn check_odin_key_unique(value: &OdinValue, issues: &mut Vec<ValidationIssue>) {
    match value {
        OdinValue::KeyedList(items) => {
            let mut seen = BTreeSet::new();
            for (k, val) in items {
                let key = odin_key_string(k);
                if !seen.insert(key.clone()) {
                    issues.push(ValidationIssue::new(
                        ValidationCode::Voku,
                        format!("duplicate key {key:?} in a keyed list"),
                    ));
                }
                check_odin_key_unique(val, issues);
            }
        }
        OdinValue::Object(map) => {
            for val in map.values() {
                check_odin_key_unique(val, issues);
            }
        }
        OdinValue::List(items) => {
            for val in items {
                check_odin_key_unique(val, issues);
            }
        }
        // A `(TYPE)` cast is a parser hint, not a level of the data
        // (`LANG/docs/odin/master05-content` §Adding Type Information) — walk
        // through it so a cast block's keys are checked like any other.
        OdinValue::Typed { value, .. } => check_odin_key_unique(value, issues),
        _ => {}
    }
}

// ── helpers ───────────────────────────────────────────────────────────────

#[derive(PartialEq, Eq, Clone, Copy)]
enum TermStructure {
    Empty,
    MissingOriginalLanguage,
    Ok,
}

fn terminology_structure(v: &ArchetypeView<'_>) -> TermStructure {
    let term = v.terminology;
    if term.term_definitions.is_empty() {
        return TermStructure::Empty;
    }
    if !term.term_definitions.contains_key(&original_language(v)) {
        return TermStructure::MissingOriginalLanguage;
    }
    TermStructure::Ok
}

fn original_language(v: &ArchetypeView<'_>) -> String {
    if v.terminology.original_language.is_empty() {
        v.original_language
            .map(|c| c.code_string.clone())
            .unwrap_or_default()
    } else {
        v.terminology.original_language.clone()
    }
}

/// The set of languages the archetype declares (original + translations +
/// description details).
fn languages(v: &ArchetypeView<'_>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if let Some(c) = v.original_language {
        out.insert(c.code_string.clone());
    }
    if let Some(t) = v.translations {
        out.extend(t.keys().cloned());
    }
    out
}

/// `is_three_part_version` — a `major.minor.patch` version (with an optional
/// pre-release suffix), per the archetype identification version rule.
fn is_three_part_version(s: &str) -> bool {
    let core = s.split('-').next().unwrap_or(s);
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// True if `id` conforms to the archetype-id form
/// `[ns::]publisher-package-class.concept.vN…` (the slot/root meta-pattern
/// `^.+-.+-.+\..*\..+$`, master04.3).
fn is_archetype_id(id: &str) -> bool {
    let body = id.rsplit("::").next().unwrap_or(id);
    let Some((prefix, rest)) = body.split_once('.') else {
        return false;
    };
    // publisher-package-class (three hyphen-separated non-empty parts)
    let hyphen_parts: Vec<&str> = prefix.split('-').collect();
    if hyphen_parts.len() < 3 || hyphen_parts.iter().any(|p| p.is_empty()) {
        return false;
    }
    // concept.version — must have a version segment starting with a digit or 'v'
    rest.contains('.') && rest.split('.').next_back().is_some_and(|_| true)
}

pub(super) fn occurrences_upper_finite(mi: Option<&MultiplicityInterval>) -> Option<i32> {
    let mi = mi?;
    if mi.upper_unbounded { None } else { mi.upper }
}

pub(super) fn occurrences_lower(mi: &MultiplicityInterval) -> i32 {
    mi.lower.unwrap_or(0)
}

pub(super) fn object_occurrences(obj: &CObject) -> Option<&MultiplicityInterval> {
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

/// True if a slot assertion expresses "any archetype" (its regex constraint is
/// a match-anything pattern), for the include/exclude consistency table.
fn is_any_assertion(a: &Assertion) -> bool {
    let Some(body) = a
        .string_expression
        .as_deref()
        .and_then(assertion_constraint_body)
    else {
        return false;
    };
    // The regex constraint inside `matches { … }`; an "any" slot is `/.*/` /
    // `/.+/` (or the bare universal pattern).
    let regex = body.trim().trim_matches('/').trim();
    regex == ".*" || regex == ".+"
}

/// Helper for the VDSEV branch-1 condition `not (excludes empty or /= any)`.
fn exc_non_any_and_any(exc_empty: bool, exc_any: bool) -> bool {
    !exc_empty && exc_any
}

/// The content between the first `{` and its matching `}` of a slot assertion's
/// preserved `string_expression` (the `matches { … }` constraint body) — used
/// so the leading `archetype_id/value` path (which itself contains `/`) is not
/// mistaken for the constraint regex.
fn assertion_constraint_body(text: &str) -> Option<&str> {
    let open = text.find('{')?;
    let close = text.rfind('}')?;
    if close > open {
        text.get(open + 1..close).map(str::trim)
    } else {
        None
    }
}

/// The archetype-id literals referenced by a slot assertion (scanned from the
/// preserved `string_expression` — the constraint targets an id via a regex).
fn assertion_archetype_ids(a: &Assertion) -> Vec<String> {
    // Slot assertions constrain `archetype_id/value matches {/regex/}`; the
    // regex, when it is a literal id (no meta-characters), is itself the id.
    // VDFAI's subject is the ARCHETYPE IDENTIFIER (ADL1.4 master05 §Archetype
    // Slots) — an assertion targeting another property (`domain_concept`,
    // `short_concept_name`, a path) constrains something that is not an
    // archetype id, so extracting its regex as one is a false positive (#767
    // audit: `domain_concept matches {/medication\.v1/}` yielded the bogus
    // id `medication.v1`).
    let targets_archetype_id = a
        .string_expression
        .as_deref()
        .is_some_and(|s| s.trim_start().starts_with("archetype_id"));
    if !targets_archetype_id {
        return Vec::new();
    }
    let Some(body) = a
        .string_expression
        .as_deref()
        .and_then(assertion_constraint_body)
    else {
        return Vec::new();
    };
    let regex = body.trim().trim_matches('/');
    // A literal id regex contains no unescaped regex meta-characters beyond the
    // escaped `\.` dots.
    let literal = regex.replace("\\.", ".");
    if literal.is_empty() || literal.contains(['*', '+', '?', '(', ')', '[', ']', '|', '^', '$']) {
        Vec::new()
    } else {
        vec![literal]
    }
}

/// Scan free text for node-id-predicated archetype path literals
/// (`/…[idN]…`), for the raw-text VRRLP check.
fn scan_predicated_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes.get(i) == Some(&b'/') {
            let start = i;
            i += 1;
            while bytes
                .get(i)
                .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'{')
            {
                i += 1;
            }
            if let Some(seg) = text.get(start..i)
                && (seg.contains("[id") || seg.contains("[at"))
            {
                out.push(seg.trim_end_matches(['/', ',']).to_owned());
            }
        } else {
            i += 1;
        }
    }
    out
}

fn odin_key_string(k: &OdinKey) -> String {
    match k {
        OdinKey::String(s) | OdinKey::Date(s) | OdinKey::Time(s) | OdinKey::DateTime(s) => {
            s.clone()
        }
        OdinKey::Integer(i) => i.to_string(),
    }
}

fn child_path(attr_path: &str, node_id: &str) -> String {
    if node_id.is_empty() {
        attr_path.to_owned()
    } else {
        format!("{attr_path}[{node_id}]")
    }
}

/// The second-order attribute tuples of a [`CComplexObject`] (either subtype).
fn complex_attribute_tuples(cco: &CComplexObject) -> &[CAttributeTuple] {
    match cco {
        CComplexObject::CComplexObject(d) => &d.attribute_tuples,
        CComplexObject::CArchetypeRoot(r) => &r.attribute_tuples,
    }
}

// ── code-usage collector (second pass for the terminology checks) ──────────

#[derive(Default)]
pub(super) struct CodeUsage {
    pub(super) value_codes: BTreeSet<String>,
    pub(super) node_codes: BTreeSet<String>,
    pub(super) assumed_refs: Vec<(String, String, String)>,
}

pub(super) fn collect_usage(obj: &CObject, usage: &mut CodeUsage) {
    collect_usage_at(obj, "", usage);
}

fn collect_usage_at(obj: &CObject, path: &str, usage: &mut CodeUsage) {
    let nid = object_node_id(obj);
    if !nid.is_empty() && (is_id_code(nid) || is_at_code(nid)) {
        let is_primitive = matches!(
            obj,
            CObject::CBoolean(_)
                | CObject::CInteger(_)
                | CObject::CReal(_)
                | CObject::CString(_)
                | CObject::CTerminologyCode(_)
                | CObject::CDate(_)
                | CObject::CTime(_)
                | CObject::CDateTime(_)
                | CObject::CDuration(_)
        );
        if !is_primitive {
            usage.node_codes.insert(nid.to_owned());
        }
    }
    match obj {
        CObject::CComplexObject(cco) => {
            for attr in complex_attributes(cco) {
                let apath = format!("{path}/{}", attr.rm_attribute_name);
                for child in &attr.children {
                    let cpath = child_path(&apath, object_node_id(child));
                    collect_usage_at(child, &cpath, usage);
                }
            }
            // Second-order tuples (e.g. ordinals) carry primitive constraints
            // outside the normal attribute tree (master04.4); collect their
            // terminology-code values too.
            for tuple in complex_attribute_tuples(cco) {
                for prim_tuple in &tuple.tuples {
                    for member in &prim_tuple.members {
                        if let CPrimitiveObject::CTerminologyCode(tc) = member {
                            usage.value_codes.extend(constraint_codes(&tc.constraint));
                        }
                    }
                }
            }
        }
        CObject::CTerminologyCode(tc) => {
            let codes = constraint_codes(&tc.constraint);
            if let Some(a) = tc.assumed_value.as_ref()
                && let Some(ac) = codes.iter().find(|c| is_ac_code(c))
            {
                usage
                    .assumed_refs
                    .push((path.to_owned(), ac.clone(), a.code_string.clone()));
            }
            usage.value_codes.extend(codes);
        }
        _ => {}
    }
}

/// The ARCHETYPE-LOCAL codes a `C_TERMINOLOGY_CODE.constraint` string names.
///
/// Two spellings reach this function, because the constraint string is the
/// verbatim carrier for both dialects:
///
/// - **ADL 2** — a single `at`/`ac` code, optionally suffixed with an operational
///   binding `@terminology` (`ADL2/master08-terminology_integration.adoc`).
/// - **ADL 1.4** — the qualified/listed custom-syntax form
///   `terminology::code[,code]*[;assumed]` of
///   `ADL1.4/master09-customising_adl.adoc` §Custom Syntax, which the 1.4 dialect
///   preserves verbatim. This is the DOMINANT 1.4 spelling of a coded value set,
///   so without decomposing it here the definedness rules never see its codes.
///
/// Only `local::` codes are archetype terms: VATDF/VACDF
/// (`ADL1.4/master08-adl.adoc` §Validity Rules) judge definedness against the
/// archetype's OWN `term_definitions`/`constraint_definitions`, and a code of an
/// external terminology (`[openehr::127]`, `[ISO_639-1::en]`) is not an archetype
/// term — its resolution is a terminology-service question (VETDF), not this one.
/// The ADL 1.4 assumed code (after `;`) IS an archetype term and is included.
fn constraint_codes(constraint: &str) -> Vec<String> {
    // Drop any ADL2 operational-binding suffix first.
    let body = constraint.split('@').next().unwrap_or(constraint).trim();
    let Some((terminology, rest)) = body.split_once("::") else {
        // ADL2 form: the constraint IS the code.
        return if body.is_empty() {
            Vec::new()
        } else {
            vec![body.to_owned()]
        };
    };
    if terminology.trim() != LOCAL_TERMINOLOGY_ID {
        return Vec::new();
    }
    // `code[,code]*[;assumed]`
    let (codes, assumed) = match rest.split_once(';') {
        Some((codes, assumed)) => (codes, Some(assumed)),
        None => (rest, None),
    };
    codes
        .split(',')
        .chain(assumed)
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod temporal_vobav_tests {
    use openehr_base::prelude::{Interval, Iso8601Date, ProperInterval, ProperIntervalData};

    use super::temporal_assumed_violates;

    fn date(v: &str) -> Iso8601Date {
        Iso8601Date {
            value: v.to_owned(),
        }
    }

    /// A closed date interval `[lo, hi]`.
    fn date_interval(lo: &str, hi: &str) -> Interval<Iso8601Date> {
        Interval::ProperInterval(ProperInterval::ProperInterval(ProperIntervalData {
            lower: Some(date(lo)),
            upper: Some(date(hi)),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        }))
    }

    #[test]
    fn assumed_date_outside_the_interval_raises() {
        let constraint = vec![date_interval("2020-01-01", "2020-12-31")];
        // 2019-06-15 is definitely before the interval ⇒ VOBAV fires.
        assert!(temporal_assumed_violates(&constraint, &date("2019-06-15")));
    }

    #[test]
    fn assumed_date_inside_the_interval_does_not_raise() {
        let constraint = vec![date_interval("2020-01-01", "2020-12-31")];
        assert!(!temporal_assumed_violates(&constraint, &date("2020-06-15")));
    }

    #[test]
    fn incomparable_assumed_date_does_not_raise() {
        let constraint = vec![date_interval("2020-01-01", "2020-12-31")];
        // The partial year 2020 overlaps the interval — containment is
        // undecidable, so it must NOT raise (honest incomparability).
        assert!(!temporal_assumed_violates(&constraint, &date("2020")));
    }

    #[test]
    fn empty_constraint_admits_any_assumed_value() {
        assert!(!temporal_assumed_violates::<Iso8601Date>(
            &[],
            &date("2019-06-15")
        ));
    }
}

#[cfg(test)]
mod constraint_code_tests {
    use super::constraint_codes;

    /// The ADL2 spelling: the constraint IS the code, with any operational
    /// binding suffix (`ADL2/master08-terminology_integration.adoc`) stripped.
    #[test]
    fn adl2_single_code_and_binding_suffix() {
        assert_eq!(constraint_codes("at1"), vec!["at1".to_owned()]);
        assert_eq!(constraint_codes("ac1@snomed"), vec!["ac1".to_owned()]);
        assert!(constraint_codes("").is_empty());
    }

    /// The ADL 1.4 spelling (`ADL1.4/master09-customising_adl.adoc` §Custom
    /// Syntax) decomposes into its listed codes plus the assumed code — all of
    /// them archetype terms VATDF must see.
    #[test]
    fn adl14_listed_codes_and_assumed_code() {
        assert_eq!(
            constraint_codes("local::at0136,at0137"),
            vec!["at0136".to_owned(), "at0137".to_owned()]
        );
        assert_eq!(
            constraint_codes("local::at0136,at0137;at0136"),
            vec![
                "at0136".to_owned(),
                "at0137".to_owned(),
                "at0136".to_owned()
            ]
        );
        assert_eq!(constraint_codes("local::ac0001"), vec!["ac0001".to_owned()]);
    }

    /// Codes of an EXTERNAL terminology are not archetype terms, so VATDF/VACDF
    /// definedness (`ADL1.4/master08-adl.adoc` §Validity Rules, judged against the
    /// archetype's own terminology) does not apply to them.
    #[test]
    fn external_terminology_codes_are_not_archetype_terms() {
        assert!(constraint_codes("openehr::127").is_empty());
        assert!(constraint_codes("openehr::253,271,273").is_empty());
        assert!(constraint_codes("ISO_639-1::en").is_empty());
    }
}
