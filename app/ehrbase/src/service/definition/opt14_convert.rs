//! OPT-1.4 → ADL2 conversion front end.
//!
//! The in-CDR 1.4 → 2 converter (`openehr_adl::adl14::convert::convert`) takes an
//! assembled *source archetype* (`openehr_am::am24`), so only stored 1.4 source
//! archetypes convert directly. A stored 1.4 **operational template** is a
//! *specialisation-flattened* artefact whose `definition` is a single
//! `C_ARCHETYPE_ROOT` tree with the component archetypes embedded inline as
//! nested `C_ARCHETYPE_ROOT` nodes — each embedded root carries its own
//! independent at-code space. Feeding that flattened tree to the converter as
//! one archetype is impossible: the component code spaces collide (every
//! embedded archetype re-uses `at0000`, `at0001`, …).
//!
//! This front end therefore **decomposes** the OPT into one 1.4-shaped `am24`
//! source archetype per embedded `C_ARCHETYPE_ROOT` (the top root plus each
//! nested one), each with its own scoped at-code space, and converts each
//! through the existing converter core. At every embedded-root boundary the
//! child is replaced in the parent by an open `ARCHETYPE_SLOT` (a fresh
//! parent-space at-code the converter renumbers), and the parent → child fill
//! edge is recorded in the returned [`OptConversion::structure`] so the
//! composition structure the flattening erased is preserved. This is the "one
//! converted source per embedded root with the structure recorded in the
//! conversion log" representation.
//!
//! NOTE: no openEHR spec governs 1.4 → 2 conversion — the entire `adl14` design,
//! including this OPT front end (decomposition strategy, slot substitution, code
//! allocation), is **our own design/extension** (the vendored ITS-REST OAS
//! declares no conversion operation; `openehr_adl::adl14` carries the same
//! flag). The `opt14` object model is `openehr_its::opt14` (the AOM 1.4 / OPT 1.4
//! model); the target is `openehr_am::am24::aom2` in the 1.4-shaped form
//! `openehr_adl::assemble::parse_artefact_adl14` produces from ADL 1.4 text, so
//! the converter core is fed exactly the shape it was built for.
//!
//! Home: this lives in the `ehrbase` service layer (not `openehr-adl`) because
//! the `opt14` DTOs live in `openehr-its`, and `openehr-adl`'s crate contract is
//! "no REST" — `openehr-its` carries the ITS-REST contract, so an
//! `openehr-adl → openehr-its` dependency would invert that boundary. The
//! service layer already depends on both `openehr_its::opt14` and
//! `openehr_adl::adl14`, so it is the existing meeting point.

use std::collections::BTreeMap;

use openehr_adl::adl14::convert::{ConvertConfig, ConvertError, convert};
use openehr_adl::adl14::log::ConversionLog;
use openehr_adl::source::parse_hrid;
use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::archetype::authored_archetype::{
    AuthoredArchetype, AuthoredArchetypeData,
};
use openehr_am::am24::aom2::constraint_model::archetype_slot::ArchetypeSlot;
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_complex_object::{
    CComplexObject, CComplexObjectData,
};
use openehr_am::am24::aom2::constraint_model::c_complex_object_proxy::CComplexObjectProxy;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_am::am24::aom2::constraint_model::primitive::c_boolean::CBoolean;
use openehr_am::am24::aom2::constraint_model::primitive::c_date::CDate;
use openehr_am::am24::aom2::constraint_model::primitive::c_date_time::CDateTime;
use openehr_am::am24::aom2::constraint_model::primitive::c_duration::CDuration;
use openehr_am::am24::aom2::constraint_model::primitive::c_integer::CInteger;
use openehr_am::am24::aom2::constraint_model::primitive::c_real::CReal;
use openehr_am::am24::aom2::constraint_model::primitive::c_string::CString;
use openehr_am::am24::aom2::constraint_model::primitive::c_terminology_code::CTerminologyCode;
use openehr_am::am24::aom2::constraint_model::primitive::c_time::CTime;
use openehr_am::am24::aom2::terminology::archetype_term::ArchetypeTerm;
use openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_am::am24::resource::resource_description::ResourceDescription;
use openehr_base::prelude::{
    Cardinality, Interval, MultiplicityInterval, PointInterval, ProperInterval, ProperIntervalData,
    TerminologyCode, Uuid,
};
use openehr_its::opt14;

/// A 1.4 → 2 OPT-conversion failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum OptConvertError {
    /// An embedded archetype root carried an unparseable `ARCHETYPE_ID`.
    #[error("OPT embedded root has an invalid archetype id {0:?}: {1}")]
    Hrid(String, String),
    /// The converter core rejected a decomposed source.
    #[error("converting decomposed source {0:?}: {1}")]
    Convert(String, ConvertError),
}

/// A parent → child slot-fill edge recovered from the flattened OPT: the
/// composition structure that flattening erased. `parent_path` is the RM path
/// within the parent archetype to the substituted slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FillEdge {
    pub(crate) parent_archetype_id: String,
    pub(crate) parent_path: String,
    pub(crate) slot_node_id: String,
    pub(crate) child_archetype_id: String,
}

/// One converted source archetype: the source archetype id it came from and its
/// ADL2 source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConvertedRoot {
    pub(crate) archetype_id: String,
    pub(crate) adl2: String,
}

/// The result of converting a stored 1.4 OPT: one ADL2 source per embedded
/// archetype root (root first, then in document order), plus the recovered
/// composition structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptConversion {
    pub(crate) roots: Vec<ConvertedRoot>,
    pub(crate) structure: Vec<FillEdge>,
}

/// Convert a parsed OPT-1.4 into one ADL2 source per embedded archetype root.
///
/// Each embedded `C_ARCHETYPE_ROOT` is decomposed into a scoped 1.4-shaped
/// `am24` source archetype and run through the `openehr_adl::adl14` converter
/// core; embedded children are replaced by open slots in their parent and the
/// fill edges recorded in [`OptConversion::structure`].
///
/// # Errors
/// - [`OptConvertError::Hrid`] if an embedded root's archetype id does not parse.
/// - [`OptConvertError::Convert`] if the converter rejects a decomposed source.
pub(crate) fn convert_opt_to_adl2(
    opt: &opt14::OperationalTemplate,
) -> Result<OptConversion, OptConvertError> {
    let (archetypes, structure) = convert_opt_to_archetypes(opt)?;
    let roots = archetypes
        .into_iter()
        .map(|(archetype_id, art)| ConvertedRoot {
            archetype_id,
            adl2: openehr_adl::printer::print(&art),
        })
        .collect();
    Ok(OptConversion { roots, structure })
}

/// The conversion core: decompose the OPT and convert each embedded root,
/// returning the converted `am24` archetypes (id + object) and the recovered
/// fill structure. [`convert_opt_to_adl2`] prints these to ADL2 text; tests
/// validate the objects directly (the converter's `validate_phase1` oracle).
///
/// # Errors
/// As [`convert_opt_to_adl2`].
/// One decomposed source per embedded OPT root, keyed by its archetype id.
pub(crate) type ConvertedRoots = Vec<(String, Archetype)>;

/// The minimal valid `RESOURCE_DESCRIPTION` for a decomposed OPT root: VARD
/// (`master03` §Validity Rules) requires a description to be specified; the
/// lifecycle state uses the 1.4→2 converter's own mapping (`unmanaged`).
fn minimal_description() -> ResourceDescription {
    ResourceDescription {
        title: None,
        original_author: std::collections::BTreeMap::new(),
        original_namespace: None,
        original_publisher: None,
        other_contributors: Vec::new(),
        lifecycle_state: "unmanaged".to_owned(),
        custodian_namespace: None,
        custodian_organisation: None,
        copyright: None,
        licence: None,
        ip_acknowledgements: None,
        references: None,
        resource_package_uri: None,
        conversion_details: None,
        details: None,
        other_details: None,
    }
}

pub(crate) fn convert_opt_to_archetypes(
    opt: &opt14::OperationalTemplate,
) -> Result<(ConvertedRoots, Vec<FillEdge>), OptConvertError> {
    let mut dx = Decomposer {
        language: opt.language.code_string.clone(),
        root_ontology: opt.ontology.as_ref(),
        component_ontologies: &opt.component_ontologies,
        units: Vec::new(),
        edges: Vec::new(),
    };
    // Unit 0 is the OPT's top root; nested roots are appended in document order
    // as they are discovered.
    dx.process_root(&opt.definition, "", true);

    let cfg = ConvertConfig::default();
    let mut archetypes = Vec::with_capacity(dx.units.len());
    for unit in dx.units {
        let mut log = ConversionLog::new();
        let hrid = parse_hrid(&unit.archetype_id)
            .map_err(|e| OptConvertError::Hrid(unit.archetype_id.clone(), e))?;
        let data = AuthoredArchetypeData {
            parent_archetype_id: None,
            archetype_id: hrid,
            is_differential: false,
            definition: unit.definition,
            terminology: Box::new(unit.terminology),
            rules: Vec::new(),
            rm_overlay: None,
            uid: None,
            original_language: term_code("ISO_639-1", &dx.language),
            // A decomposed OPT root carries no RESOURCE_DESCRIPTION of its own,
            // but VARD (`master03` §Validity Rules) requires one — synthesize
            // the minimal valid description with the converter's own lifecycle
            // mapping (`unmanaged`; see `adl14::convert::transform_description`).
            // No openEHR spec governs 1.4→2 conversion — our own design.
            description: Some(Box::new(minimal_description())),
            is_controlled: None,
            annotations: None,
            translations: None,
            adl_version: Some("1.4".to_owned()),
            build_uid: Uuid {
                value: uuid::Uuid::nil(),
            },
            rm_release: String::new(),
            is_generated: true,
            other_meta_data: BTreeMap::new(),
        };
        // NOTE: `is_differential`, `adl_version`, `rm_release`, `is_generated`
        // above are the converter's starting point only — `convert` overrides
        // them (`is_differential` from the absent parent; the ADL/RM stamps from
        // `ConvertConfig`).
        let art =
            Archetype::AuthoredArchetype(Box::new(AuthoredArchetype::AuthoredArchetype(data)));
        let converted = convert(&art, &cfg, &mut log)
            .map_err(|e| OptConvertError::Convert(unit.archetype_id.clone(), e))?;
        archetypes.push((unit.archetype_id, converted));
    }
    Ok((archetypes, dx.edges))
}

/// One decomposed archetype root awaiting conversion.
struct RawUnit {
    archetype_id: String,
    definition: CComplexObject,
    terminology: ArchetypeTerminology,
}

struct Decomposer<'a> {
    language: String,
    root_ontology: Option<&'a opt14::FlatArchetypeOntology>,
    component_ontologies: &'a [opt14::FlatArchetypeOntology],
    units: Vec<RawUnit>,
    edges: Vec<FillEdge>,
}

impl Decomposer<'_> {
    /// Decompose one `C_ARCHETYPE_ROOT` into a scoped 1.4-shaped source archetype
    /// (pushed onto `units`), recursing into any embedded child roots. `is_top`
    /// selects the OPT's top-level `ontology` vs a `component_ontologies` entry
    /// for the archetype's flattened terminology.
    fn process_root(&mut self, root: &opt14::CArchetypeRoot, path: &str, is_top: bool) {
        let archetype_id = root.archetype_id.value.clone();
        // Slot at-codes are allocated strictly above every at-code node id used
        // by THIS root's own retained subtree (child roots are excluded — their
        // codes belong to the child's space), so a substituted slot never
        // collides with a real node in the parent's code space.
        let mut slot_num = max_at_num(&root.attributes) + 1;
        let attributes = self.map_attributes(&root.attributes, &archetype_id, path, &mut slot_num);
        let definition = CComplexObject::CComplexObject(CComplexObjectData {
            parent: None,
            soc_parent: None,
            rm_type_name: root.rm_type_name.clone(),
            // A source-archetype root declares no occurrences of its own.
            occurrences: None,
            node_id: root.node_id.clone(),
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            attributes,
            attribute_tuples: Vec::new(),
        });
        let terminology = self.build_terminology(&archetype_id, root, is_top);
        self.units.push(RawUnit {
            archetype_id,
            definition,
            terminology,
        });
    }

    fn map_attributes(
        &mut self,
        attrs: &[opt14::CAttribute],
        archetype_id: &str,
        path: &str,
        slot_num: &mut i64,
    ) -> Vec<CAttribute> {
        let mut out = Vec::with_capacity(attrs.len());
        for attr in attrs {
            out.push(self.map_attribute(attr, archetype_id, path, slot_num));
        }
        out
    }

    fn map_attribute(
        &mut self,
        attr: &opt14::CAttribute,
        archetype_id: &str,
        path: &str,
        slot_num: &mut i64,
    ) -> CAttribute {
        let (rm_attribute_name, existence, children, cardinality, is_multiple) = match attr {
            opt14::CAttribute::CSingleAttribute(a) => (
                a.rm_attribute_name.clone(),
                &a.existence,
                &a.children,
                None,
                false,
            ),
            opt14::CAttribute::CMultipleAttribute(a) => (
                a.rm_attribute_name.clone(),
                &a.existence,
                &a.children,
                Some(map_cardinality(&a.cardinality)),
                true,
            ),
        };
        let attr_path = format!("{path}/{rm_attribute_name}");
        let mut mapped_children = Vec::with_capacity(children.len());
        for c in children {
            mapped_children.push(self.map_object(c, archetype_id, &attr_path, slot_num));
        }
        CAttribute {
            parent: None,
            soc_parent: None,
            rm_attribute_name,
            existence: Some(map_mult(existence)),
            children: mapped_children,
            differential_path: None,
            cardinality,
            is_multiple,
        }
    }

    // Several arms share an identical-looking body (e.g. the domain
    // constrainers → a loose `complex(...)`), but each binds a DISTINCT `opt14`
    // variant type under the same name, so they cannot be merged into one
    // or-pattern arm — the bindings have incompatible types.
    #[allow(clippy::match_same_arms)]
    fn map_object(
        &mut self,
        obj: &opt14::CObject,
        archetype_id: &str,
        path: &str,
        slot_num: &mut i64,
    ) -> CObject {
        match obj {
            // An embedded archetype root: decompose it as its own source and
            // leave an open slot (fresh parent-space code) in this parent.
            opt14::CObject::CArchetypeRoot(child) => {
                let slot_node_id = format!("at{}", *slot_num);
                *slot_num += 1;
                let child_id = child.archetype_id.value.clone();
                self.edges.push(FillEdge {
                    parent_archetype_id: archetype_id.to_owned(),
                    parent_path: format!("{path}[{slot_node_id}]"),
                    slot_node_id: slot_node_id.clone(),
                    child_archetype_id: child_id,
                });
                let slot = CObject::ArchetypeSlot(ArchetypeSlot {
                    parent: None,
                    soc_parent: None,
                    rm_type_name: child.rm_type_name.clone(),
                    occurrences: Some(map_mult(&child.occurrences)),
                    node_id: slot_node_id,
                    alternative_ids: Vec::new(),
                    is_deprecated: None,
                    sibling_order: None,
                    // An open slot: the concrete fill identity is recorded in the
                    // conversion structure, not re-imposed as an include here.
                    // TODO: reconstruct include/exclude assertions naming the
                    // filled archetype id (needs a BEOM assertion builder).
                    includes: Vec::new(),
                    excludes: Vec::new(),
                    is_closed: false,
                });
                self.process_root(child, path, false);
                slot
            }
            opt14::CObject::CComplexObject(c) => {
                let attributes = self.map_attributes(&c.attributes, archetype_id, path, slot_num);
                complex(&c.rm_type_name, &c.node_id, &c.occurrences, attributes)
            }
            // A `T_COMPLEX_OBJECT` is a template-node complex object; its
            // `default_value` is an operational-template artefact not carried into
            // the converted source. TODO: carry OPT `default_value`s.
            opt14::CObject::TComplexObject(c) => {
                let attributes = self.map_attributes(&c.attributes, archetype_id, path, slot_num);
                complex(&c.rm_type_name, &c.node_id, &c.occurrences, attributes)
            }
            opt14::CObject::CDefinedObject(c) => {
                complex(&c.rm_type_name, &c.node_id, &c.occurrences, Vec::new())
            }
            opt14::CObject::ArchetypeInternalRef(r) => {
                CObject::CComplexObjectProxy(CComplexObjectProxy {
                    parent: None,
                    soc_parent: None,
                    rm_type_name: r.rm_type_name.clone(),
                    occurrences: Some(map_mult(&r.occurrences)),
                    node_id: r.node_id.clone(),
                    alternative_ids: Vec::new(),
                    is_deprecated: None,
                    sibling_order: None,
                    target_path: r.target_path.clone(),
                })
            }
            opt14::CObject::ArchetypeSlot(s) => CObject::ArchetypeSlot(ArchetypeSlot {
                parent: None,
                soc_parent: None,
                rm_type_name: s.rm_type_name.clone(),
                occurrences: Some(map_mult(&s.occurrences)),
                node_id: s.node_id.clone(),
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                // TODO: map the 1.4 slot include/exclude `ASSERTION`s (BEOM
                // expression trees) rather than emitting an open slot.
                includes: Vec::new(),
                excludes: Vec::new(),
                is_closed: false,
            }),
            // A coded-value constraint → the 1.4-shaped `C_TERMINOLOGY_CODE` the
            // converter rewrites (`terminology::code[,code…][;assumed]`).
            opt14::CObject::CCodePhrase(c) => terminology_code(
                &c.rm_type_name,
                &c.node_id,
                &c.occurrences,
                code_constraint(
                    c.terminology_id.as_ref().map(|t| t.value.as_str()),
                    &c.code_list,
                    c.assumed_value.as_ref().map(|a| a.code_string.as_str()),
                ),
            ),
            opt14::CObject::CCodeReference(c) => terminology_code(
                &c.rm_type_name,
                &c.node_id,
                &c.occurrences,
                // TODO: carry `referenceSetUri` as a term binding.
                code_constraint(
                    c.terminology_id.as_ref().map(|t| t.value.as_str()),
                    &c.code_list,
                    c.assumed_value.as_ref().map(|a| a.code_string.as_str()),
                ),
            ),
            // A `CONSTRAINT_REF` names a (deprecated) constraint-binding code; we
            // emit an unconstrained terminology-code node rather than a dangling
            // `ac`-reference that would fail `VACDF`. TODO: resolve the
            // constraint binding to a value set.
            opt14::CObject::ConstraintRef(r) => {
                terminology_code(&r.rm_type_name, &r.node_id, &r.occurrences, String::new())
            }
            // The domain constrainer types carry structured value constraints
            // (ordinal value/symbol tuples, quantity magnitude/unit lists, state
            // machines) that the 1.4-text converter lowers to attribute-tuple
            // `C_COMPLEX_OBJECT`s. Reconstructing those tuples from the object
            // model is not done here; a loose (unconstrained) domain-typed complex
            // object is emitted, which is a valid ADL2 constraint.
            // TODO: reconstruct `DV_ORDINAL`/`DV_QUANTITY`/`DV_STATE` value
            // constraints as attribute tuples.
            opt14::CObject::CDvOrdinal(c) => {
                complex(&c.rm_type_name, &c.node_id, &c.occurrences, Vec::new())
            }
            opt14::CObject::CDvQuantity(c) => {
                complex(&c.rm_type_name, &c.node_id, &c.occurrences, Vec::new())
            }
            opt14::CObject::CDvState(c) => {
                complex(&c.rm_type_name, &c.node_id, &c.occurrences, Vec::new())
            }
            opt14::CObject::CPrimitiveObject(c) => map_primitive_object(c),
        }
    }

    /// Assemble the 1.4-shaped terminology for one archetype root: the inline
    /// `C_ARCHETYPE_ROOT.term_definitions`/`term_bindings` (keyed under the OPT's
    /// language) merged with the matching flattened ontology
    /// (`ontology` for the top root, `component_ontologies[archetype_id]` for an
    /// embedded root). Extra (unused) codes are harmless — they raise at most a
    /// `WOUC` warning, never a phase-1 error.
    fn build_terminology(
        &self,
        archetype_id: &str,
        root: &opt14::CArchetypeRoot,
        is_top: bool,
    ) -> ArchetypeTerminology {
        let mut term_definitions: BTreeMap<String, BTreeMap<String, ArchetypeTerm>> =
            BTreeMap::new();
        let mut term_bindings: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

        // Inline terms on the root node are language-less; key them under the
        // OPT's authoring language.
        {
            let inline = term_definitions.entry(self.language.clone()).or_default();
            for t in &root.term_definitions {
                inline.insert(t.code.clone(), map_term(t));
            }
        }
        for set in &root.term_bindings {
            let bucket = term_bindings.entry(set.terminology.clone()).or_default();
            for item in &set.items {
                bucket.insert(item.code.clone(), item.value.code_string.clone());
            }
        }

        // The flattened ontology (per-language) for this archetype: the top
        // root's is the OPT `ontology`; an embedded root matches a
        // `component_ontologies` entry by archetype id.
        let ontology = if is_top {
            self.root_ontology
        } else {
            self.component_ontologies
                .iter()
                .find(|o| o.archetype_id == archetype_id)
        };
        if let Some(ont) = ontology {
            for set in &ont.term_definitions {
                let bucket = term_definitions.entry(set.language.clone()).or_default();
                for t in &set.items {
                    bucket.insert(t.code.clone(), map_term(t));
                }
            }
            for set in &ont.term_bindings {
                let bucket = term_bindings.entry(set.terminology.clone()).or_default();
                for item in &set.items {
                    bucket.insert(item.code.clone(), item.value.code_string.clone());
                }
            }
        }

        // A binding whose key is not a code DEFINED in this root's slice is
        // unexpressible after decomposition (1.4 OPTs may bind path keys or
        // codes scoped to another embedded root) and would raise VTTBK
        // (`master03` §Validity Rules — binding keys must be defined codes).
        // Drop it, logged: the binding's home is the root that defines the
        // code, which carries it in its own slice.
        let defined: std::collections::BTreeSet<&str> = term_definitions
            .values()
            .flat_map(|by_code| by_code.keys().map(String::as_str))
            .collect();
        for by_key in term_bindings.values_mut() {
            by_key.retain(|key, _| {
                let keep = defined.contains(key.as_str());
                if !keep {
                    tracing::debug!(
                        archetype_id,
                        binding_key = key.as_str(),
                        "opt14 conversion: dropping term binding outside this root's code scope"
                    );
                }
                keep
            });
        }
        term_bindings.retain(|_, by_key| !by_key.is_empty());

        ArchetypeTerminology {
            is_differential: false,
            original_language: self.language.clone(),
            concept_code: root.node_id.clone(),
            term_definitions,
            term_bindings: (!term_bindings.is_empty()).then_some(term_bindings),
            value_sets: None,
            terminology_extracts: None,
        }
    }
}

/// The maximum at-code number among a root's own node ids (not descending into
/// embedded child roots — their codes belong to the child's space).
fn max_at_num(attrs: &[opt14::CAttribute]) -> i64 {
    let mut max = 0;
    for attr in attrs {
        let children = match attr {
            opt14::CAttribute::CSingleAttribute(a) => &a.children,
            opt14::CAttribute::CMultipleAttribute(a) => &a.children,
        };
        for child in children {
            max = max.max(max_at_num_obj(child));
        }
    }
    max
}

// Arms share the body shape `at_num(&x.node_id)` but each binds a distinct
// `opt14` variant type, so they cannot be merged into one or-pattern arm.
#[allow(clippy::match_same_arms)]
fn max_at_num_obj(obj: &opt14::CObject) -> i64 {
    match obj {
        // Do NOT descend into an embedded root: its at-codes are its own space.
        opt14::CObject::CArchetypeRoot(_) => 0,
        opt14::CObject::CComplexObject(c) => at_num(&c.node_id).max(max_at_num(&c.attributes)),
        opt14::CObject::TComplexObject(c) => at_num(&c.node_id).max(max_at_num(&c.attributes)),
        opt14::CObject::CDefinedObject(c) => at_num(&c.node_id),
        opt14::CObject::ArchetypeInternalRef(r) => at_num(&r.node_id),
        opt14::CObject::ArchetypeSlot(s) => at_num(&s.node_id),
        opt14::CObject::ConstraintRef(r) => at_num(&r.node_id),
        opt14::CObject::CCodePhrase(c) => at_num(&c.node_id),
        opt14::CObject::CCodeReference(c) => at_num(&c.node_id),
        opt14::CObject::CDvOrdinal(c) => at_num(&c.node_id),
        opt14::CObject::CDvQuantity(c) => at_num(&c.node_id),
        opt14::CObject::CDvState(c) => at_num(&c.node_id),
        opt14::CObject::CPrimitiveObject(c) => at_num(&c.node_id),
    }
}

/// The numeric value of an `atNNNN` node id's first segment (0 for a non-at
/// code, e.g. an empty id or an already-id code).
fn at_num(node_id: &str) -> i64 {
    let Some(rest) = node_id.strip_prefix("at") else {
        return 0;
    };
    rest.split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

// ── object constructors (the 1.4-shaped `am24` common fields) ────────────────

fn complex(
    rm_type_name: &str,
    node_id: &str,
    occurrences: &opt14::Intervalofinteger,
    attributes: Vec<CAttribute>,
) -> CObject {
    CObject::CComplexObject(CComplexObject::CComplexObject(CComplexObjectData {
        parent: None,
        soc_parent: None,
        rm_type_name: rm_type_name.to_owned(),
        occurrences: Some(map_mult(occurrences)),
        node_id: node_id.to_owned(),
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value: None,
        attributes,
        attribute_tuples: Vec::new(),
    }))
}

fn terminology_code(
    rm_type_name: &str,
    node_id: &str,
    occurrences: &opt14::Intervalofinteger,
    constraint: String,
) -> CObject {
    CObject::CTerminologyCode(CTerminologyCode {
        parent: None,
        soc_parent: None,
        rm_type_name: rm_type_name.to_owned(),
        occurrences: Some(map_mult(occurrences)),
        node_id: node_id.to_owned(),
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value: None,
        assumed_value: None,
        is_enumerated_type_constraint: None,
        constraint,
        constraint_status: None,
    })
}

/// Map a `C_PRIMITIVE_OBJECT` (its wrapped `C_PRIMITIVE`) to the matching `am24`
/// primitive constraint node. The converter passes primitive nodes through
/// untouched; phase-1 (no RM repo) does not validate primitive-constraint
/// internals, so a faithful-but-minimal mapping is sufficient and safe.
#[allow(clippy::too_many_lines)] // one arm per primitive C_* struct literal
fn map_primitive_object(c: &opt14::CPrimitiveObject) -> CObject {
    let rm = c.rm_type_name.as_str();
    let node_id = c.node_id.as_str();
    let occ = &c.occurrences;
    let Some(item) = c.item.as_deref() else {
        // A primitive object with no inner constraint → an unconstrained string.
        return c_string(rm, node_id, occ, Vec::new(), None);
    };
    match item {
        opt14::CPrimitive::CBoolean(p) => {
            let mut constraint = Vec::new();
            if p.true_valid {
                constraint.push(true);
            }
            if p.false_valid {
                constraint.push(false);
            }
            CObject::CBoolean(CBoolean {
                parent: None,
                soc_parent: None,
                rm_type_name: rm.to_owned(),
                occurrences: Some(map_mult(occ)),
                node_id: node_id.to_owned(),
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: p.assumed_value,
                is_enumerated_type_constraint: None,
                constraint,
            })
        }
        opt14::CPrimitive::CString(p) => {
            let constraint = if !p.list.is_empty() {
                p.list.clone()
            } else if let Some(pat) = &p.pattern {
                vec![pat.clone()]
            } else {
                Vec::new()
            };
            c_string(rm, node_id, occ, constraint, p.assumed_value.clone())
        }
        opt14::CPrimitive::CInteger(p) => {
            let constraint = p.range.as_ref().map(int_interval).into_iter().collect();
            CObject::CInteger(CInteger {
                parent: None,
                soc_parent: None,
                rm_type_name: rm.to_owned(),
                occurrences: Some(map_mult(occ)),
                node_id: node_id.to_owned(),
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: p.assumed_value.map(f64::from),
                is_enumerated_type_constraint: None,
                constraint,
            })
        }
        opt14::CPrimitive::CReal(p) => {
            let constraint = p.range.as_ref().map(real_interval).into_iter().collect();
            CObject::CReal(CReal {
                parent: None,
                soc_parent: None,
                rm_type_name: rm.to_owned(),
                occurrences: Some(map_mult(occ)),
                node_id: node_id.to_owned(),
                alternative_ids: Vec::new(),
                is_deprecated: None,
                sibling_order: None,
                default_value: None,
                assumed_value: p.assumed_value,
                is_enumerated_type_constraint: None,
                constraint,
            })
        }
        // Temporal constraints: carry the ISO8601 `pattern` (the common 1.4
        // form); the range half is dropped. TODO: carry temporal ranges as
        // `Interval<Iso8601_*>` once ISO8601 bound parsing is threaded here.
        opt14::CPrimitive::CDate(p) => CObject::CDate(CDate {
            parent: None,
            soc_parent: None,
            rm_type_name: rm.to_owned(),
            occurrences: Some(map_mult(occ)),
            node_id: node_id.to_owned(),
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint: Vec::new(),
            pattern_constraint: p.pattern.clone(),
        }),
        opt14::CPrimitive::CDateTime(p) => CObject::CDateTime(CDateTime {
            parent: None,
            soc_parent: None,
            rm_type_name: rm.to_owned(),
            occurrences: Some(map_mult(occ)),
            node_id: node_id.to_owned(),
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint: Vec::new(),
            pattern_constraint: p.pattern.clone(),
        }),
        opt14::CPrimitive::CDuration(p) => CObject::CDuration(CDuration {
            parent: None,
            soc_parent: None,
            rm_type_name: rm.to_owned(),
            occurrences: Some(map_mult(occ)),
            node_id: node_id.to_owned(),
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint: Vec::new(),
            pattern_constraint: p.pattern.clone(),
        }),
        opt14::CPrimitive::CTime(p) => CObject::CTime(CTime {
            parent: None,
            soc_parent: None,
            rm_type_name: rm.to_owned(),
            occurrences: Some(map_mult(occ)),
            node_id: node_id.to_owned(),
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            assumed_value: None,
            is_enumerated_type_constraint: None,
            constraint: Vec::new(),
            pattern_constraint: p.pattern.clone(),
        }),
    }
}

fn c_string(
    rm_type_name: &str,
    node_id: &str,
    occurrences: &opt14::Intervalofinteger,
    constraint: Vec<String>,
    assumed_value: Option<String>,
) -> CObject {
    CObject::CString(CString {
        parent: None,
        soc_parent: None,
        rm_type_name: rm_type_name.to_owned(),
        occurrences: Some(map_mult(occurrences)),
        node_id: node_id.to_owned(),
        alternative_ids: Vec::new(),
        is_deprecated: None,
        sibling_order: None,
        default_value: None,
        assumed_value,
        is_enumerated_type_constraint: None,
        constraint,
    })
}

// ── value helpers ────────────────────────────────────────────────────────────

/// Build the 1.4 `C_TERMINOLOGY_CODE.constraint` encoding
/// (`terminology::code[,code…][;assumed]`) the converter rewrites. An empty code
/// list yields an empty (unconstrained) string.
fn code_constraint(terminology: Option<&str>, codes: &[String], assumed: Option<&str>) -> String {
    if codes.is_empty() {
        return String::new();
    }
    let term = terminology.unwrap_or("local");
    let mut out = format!("{term}::{}", codes.join(","));
    if let Some(a) = assumed {
        out.push(';');
        out.push_str(a);
    }
    out
}

fn map_term(t: &opt14::ArchetypeTerm) -> ArchetypeTerm {
    let text = t.items.get("text").cloned().unwrap_or_default();
    let description = t.items.get("description").cloned().unwrap_or_default();
    let other_items: BTreeMap<String, String> = t
        .items
        .iter()
        .filter(|(k, _)| k.as_str() != "text" && k.as_str() != "description")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    // Keep the map absent rather than empty for the common no-extras case.
    let other_items = (!other_items.is_empty()).then_some(other_items);
    ArchetypeTerm {
        code: t.code.clone(),
        text,
        description,
        other_items,
    }
}

fn map_mult(iv: &opt14::Intervalofinteger) -> MultiplicityInterval {
    MultiplicityInterval {
        lower: iv.lower,
        upper: iv.upper,
        lower_unbounded: iv.lower_unbounded,
        upper_unbounded: iv.upper_unbounded,
        lower_included: iv.lower_included.unwrap_or(!iv.lower_unbounded),
        upper_included: iv.upper_included.unwrap_or(!iv.upper_unbounded),
    }
}

fn map_cardinality(c: &opt14::Cardinality) -> Cardinality {
    Cardinality {
        interval: map_mult(&c.interval),
        is_ordered: c.is_ordered,
        is_unique: c.is_unique,
    }
}

fn int_interval(iv: &opt14::Intervalofinteger) -> Interval<i32> {
    if iv.lower == iv.upper && iv.lower.is_some() {
        let v = iv.lower.unwrap_or_default();
        return Interval::PointInterval(PointInterval {
            lower: Some(v),
            upper: Some(v),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        });
    }
    Interval::ProperInterval(ProperInterval::ProperInterval(ProperIntervalData {
        lower: iv.lower,
        upper: iv.upper,
        lower_unbounded: iv.lower_unbounded,
        upper_unbounded: iv.upper_unbounded,
        lower_included: iv.lower_included.unwrap_or(!iv.lower_unbounded),
        upper_included: iv.upper_included.unwrap_or(!iv.upper_unbounded),
    }))
}

fn real_interval(iv: &opt14::Intervalofreal) -> Interval<f64> {
    if iv.lower == iv.upper && iv.lower.is_some() {
        let v = iv.lower.unwrap_or_default();
        return Interval::PointInterval(PointInterval {
            lower: Some(v),
            upper: Some(v),
            lower_unbounded: false,
            upper_unbounded: false,
            lower_included: true,
            upper_included: true,
        });
    }
    Interval::ProperInterval(ProperInterval::ProperInterval(ProperIntervalData {
        lower: iv.lower,
        upper: iv.upper,
        lower_unbounded: iv.lower_unbounded,
        upper_unbounded: iv.upper_unbounded,
        lower_included: iv.lower_included.unwrap_or(!iv.lower_unbounded),
        upper_included: iv.upper_included.unwrap_or(!iv.upper_unbounded),
    }))
}

fn term_code(terminology_id: &str, code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: terminology_id.to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::print_stdout
)] // test assertions/diagnostics
mod tests {
    use openehr_adl::validate::{Severity, validate_phase1};

    use super::*;

    /// The vendored OPT corpus (real Ocean/EHRbase-generated operational
    /// templates). One minimal COMPOSITION+OBSERVATION, one minimal EVALUATION
    /// (carries a `C_DV_QUANTITY`), and a large multi-archetype template
    /// (`Vital Signs`: 12 embedded roots, ordinals/quantities/integers/reals/
    /// strings/booleans/code phrases).
    const MINIMAL_OBSERVATION: &str =
        "tests/resources/service/knowledge/opt/minimal_observation.opt";
    const MINIMAL_EVALUATION: &str = "tests/resources/service/knowledge/opt/minimal_evaluation.opt";
    const VITAL_SIGNS: &str =
        "tests/resources/service/knowledge/opt/Vital Signs Encounter (Composition).opt";

    fn parse_opt(rel: &str) -> opt14::OperationalTemplate {
        let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
        let xml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        opt14::from_xml(&xml).unwrap_or_else(|e| panic!("parse OPT {rel}: {e:?}"))
    }

    /// Every converted root converts, is a plain authored archetype, and passes
    /// the AOM2 phase-1 catalogue clean — the converter's own oracle property
    /// (`crates/openehr-adl/tests/adl14_conversion.rs` `assert_structural_match`
    /// runs the same `validate_phase1(&got, None)` gate).
    fn assert_converts_clean(rel: &str, min_roots: usize) {
        let opt = parse_opt(rel);
        let (archetypes, structure) =
            convert_opt_to_archetypes(&opt).unwrap_or_else(|e| panic!("convert {rel}: {e}"));
        assert!(
            archetypes.len() >= min_roots,
            "{rel}: expected >= {min_roots} converted roots, got {}",
            archetypes.len()
        );
        for (id, art) in &archetypes {
            assert!(
                matches!(art, Archetype::AuthoredArchetype(_)),
                "{rel}/{id}: not a plain authored archetype"
            );
            let errors: Vec<&str> = validate_phase1(art, None)
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .map(|i| i.code.mnemonic())
                .collect();
            assert!(
                errors.is_empty(),
                "{rel}/{id}: converted output failed phase-1 validation: {errors:?}"
            );
        }
        // Every recorded fill edge names a real parent/child archetype id.
        let ids: std::collections::BTreeSet<&str> =
            archetypes.iter().map(|(id, _)| id.as_str()).collect();
        for edge in &structure {
            assert!(
                ids.contains(edge.parent_archetype_id.as_str()),
                "{rel}: fill edge parent {:?} is not a converted root",
                edge.parent_archetype_id
            );
            assert!(
                ids.contains(edge.child_archetype_id.as_str()),
                "{rel}: fill edge child {:?} is not a converted root",
                edge.child_archetype_id
            );
        }
    }

    #[test]
    fn minimal_observation_converts_clean() {
        // COMPOSITION root + one embedded OBSERVATION root = 2 sources, 1 edge.
        assert_converts_clean(MINIMAL_OBSERVATION, 2);
    }

    #[test]
    fn minimal_evaluation_converts_clean() {
        assert_converts_clean(MINIMAL_EVALUATION, 2);
    }

    #[test]
    fn vital_signs_converts_clean() {
        // A large multi-archetype template: 12 embedded C_ARCHETYPE_ROOTs, so
        // the COMPOSITION root plus its components.
        assert_converts_clean(VITAL_SIGNS, 2);
    }

    /// Two full conversions of the same OPT yield identical archetypes (each
    /// unit converts under a fresh, deterministic log) — the converter's
    /// idempotency oracle, at the OPT-front-end level.
    #[test]
    fn conversion_is_deterministic() {
        let opt = parse_opt(MINIMAL_OBSERVATION);
        let (first, first_edges) = convert_opt_to_archetypes(&opt).expect("first convert");
        let (second, second_edges) = convert_opt_to_archetypes(&opt).expect("second convert");
        assert_eq!(first, second, "OPT conversion is not deterministic");
        assert_eq!(
            first_edges, second_edges,
            "fill structure is not deterministic"
        );
    }

    /// The embedded OBSERVATION root is decomposed into its own source and the
    /// COMPOSITION → OBSERVATION fill is recorded in the structure log.
    #[test]
    fn embedded_root_recorded_in_structure() {
        let opt = parse_opt(MINIMAL_OBSERVATION);
        let (archetypes, structure) = convert_opt_to_archetypes(&opt).expect("convert");
        assert!(
            archetypes
                .iter()
                .any(|(id, _)| id == "openEHR-EHR-OBSERVATION.minimal.v1"),
            "the embedded OBSERVATION was not decomposed into its own source"
        );
        assert!(
            structure.iter().any(|e| {
                e.parent_archetype_id == "openEHR-EHR-COMPOSITION.minimal.v1"
                    && e.child_archetype_id == "openEHR-EHR-OBSERVATION.minimal.v1"
            }),
            "the COMPOSITION → OBSERVATION fill edge was not recorded: {structure:?}"
        );
    }
}
