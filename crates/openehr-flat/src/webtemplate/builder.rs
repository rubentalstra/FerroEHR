//! OPT 1.4 → Better `web-template` walk.
//!
//! Port of Better `builder/WebTemplateBuilder.kt` + the `compactor`/`postprocess`
//! packages, driven by [`openehr_its::opt14`] instead of Better's `AmNode` tree:
//!
//! 1. **build** a node per constraint object (rm type, node id, aql path,
//!    occurrences, rubric names, term bindings), giving `DATA_VALUE/PARTY` leaves
//!    their `inputs` and running the per-rm post-processors;
//! 2. **compact** the tree (Medium compactor: hoist `ITEM_*`/`HISTORY`/single
//!    `EVENT`; promote an `ELEMENT/DATA_VALUE` single child; drop empties);
//! 3. **assign ids** (see [`super::id`]).
//!
//! Scope boundaries for this PR (recorded as `TODO(port)`): required-RM-attribute
//! injection (needs the BMM RM attribute model, P16), `ISM_TRANSITION`/careflow
//! synthesis, the "any" (unconstrained) ELEMENT value expansion, and archetype
//! internal-ref target resolution — such nodes are emitted without their
//! synthesized descendants rather than incorrectly.

use std::collections::HashMap;

use indexmap::IndexMap;
use openehr_its::opt14::{
    ArchetypeTerm, CArchetypeRoot, CObject, CPrimitive, Cardinality, Intervalofinteger,
    OperationalTemplate,
};

use super::inputs::{self, Labels};
use super::model::{
    WebTemplate, WebTemplateCardinality, WebTemplateInput, WebTemplateInputType, WebTemplateNode,
};

const CURRENT_VERSION: &str = "2.3";

/// RM types whose wrapper node is always hoisted away (children lifted to parent).
const ALWAYS_COMPACTABLE: [&str; 6] = [
    "ITEM_TREE",
    "ITEM_LIST",
    "ITEM_SINGLE",
    "ITEM_TABLE",
    "ITEM_STRUCTURE",
    "HISTORY",
];

/// RM types compacted only when singular and without a matching sibling.
const SINGLE_COMPACTABLE: [&str; 3] = ["POINT_EVENT", "INTERVAL_EVENT", "EVENT"];

/// RM types dropped when they end up empty (Better `SKIP_IF_EMPTY`).
const SKIP_IF_EMPTY: [&str; 12] = [
    "CLUSTER",
    "ELEMENT",
    "ITEM_TREE",
    "ITEM_LIST",
    "ITEM_SINGLE",
    "ITEM_TABLE",
    "ITEM_STRUCTURE",
    "HISTORY",
    "POINT_EVENT",
    "INTERVAL_EVENT",
    "EVENT",
    "ITEM",
];

/// A rubric (`text`/`description`) for a code, resolved from an archetype ontology.
#[derive(Default)]
struct Rubric {
    text: Option<String>,
    description: Option<String>,
}

/// Ontology: `archetype_id → language → code → rubric`.
type Ontology = HashMap<String, HashMap<String, HashMap<String, Rubric>>>;

/// Build context shared across the walk.
struct Ctx {
    default_language: String,
    languages: Vec<String>,
    ontology: Ontology,
}

impl Ctx {
    fn rubric(&self, arch_id: &str, lang: &str, code: &str) -> Option<&Rubric> {
        self.ontology.get(arch_id)?.get(lang)?.get(code)
    }

    fn text(&self, arch_id: &str, code: &str, lang: &str) -> Option<String> {
        self.rubric(arch_id, lang, code)
            .and_then(|r| r.text.clone())
    }
}

/// A [`Labels`] view bound to the current archetype, for coded-value rubrics.
struct ArchetypeLabels<'a> {
    ctx: &'a Ctx,
    arch_id: &'a str,
}

impl Labels for ArchetypeLabels<'_> {
    fn text(&self, _terminology: &str, code: &str) -> Option<String> {
        self.ctx
            .text(self.arch_id, code, &self.ctx.default_language)
    }

    fn localized(&self, _terminology: &str, code: &str) -> IndexMap<String, String> {
        let mut out = IndexMap::new();
        for lang in &self.ctx.languages {
            if let Some(t) = self.ctx.text(self.arch_id, code, lang) {
                out.insert(lang.clone(), t);
            }
        }
        out
    }
}

/// Build a [`WebTemplate`] from a parsed OPT 1.4 operational template.
///
/// Better is the interop oracle: field names, `id`/`aqlPath` derivation, the
/// RM-type → `inputs` mapping, and the compaction shape match its reference
/// implementation. `version` is the format version (`"2.3"`).
///
/// # Errors
/// [`crate::FlatError::InvalidTemplate`] if the template lacks a template id.
pub fn build_web_template(opt: &OperationalTemplate) -> Result<WebTemplate, crate::FlatError> {
    let template_id = opt.template_id.value.clone();
    if template_id.is_empty() {
        return Err(crate::FlatError::InvalidTemplate(
            "template_id is mandatory".to_owned(),
        ));
    }
    let default_language = opt.language.code_string.clone();
    let ctx = Ctx {
        languages: collect_languages(opt, &default_language),
        ontology: collect_ontology(opt, &default_language),
        default_language,
    };

    let root_arch_id = opt.definition.archetype_id.value.clone();
    let root_co = CObject::CArchetypeRoot(opt.definition.clone());
    let mut tree = build_node(&ctx, None, &root_co, "", &root_arch_id);
    tree = compact(tree, 1).unwrap_or(tree_placeholder());
    super::id::build_ids(&mut tree);

    Ok(WebTemplate {
        template_id,
        sem_ver: None, // TODO(port): extract semver from the template description.
        version: CURRENT_VERSION.to_owned(),
        default_language: ctx.default_language.clone(),
        languages: ctx.languages.clone(),
        tree,
        other_details: other_details(opt),
    })
}

fn tree_placeholder() -> WebTemplateNode {
    // The root is never removed by compaction; this only satisfies the type.
    WebTemplateNode::new("COMPOSITION".to_owned(), String::new())
}

// ── node build ───────────────────────────────────────────────────────────────

fn build_node(
    ctx: &Ctx,
    attr_name: Option<&str>,
    co: &CObject,
    parent_path: &str,
    parent_arch_id: &str,
) -> WebTemplateNode {
    // A C_ARCHETYPE_ROOT switches the ontology scope to its own archetype.
    let node_arch_id = match co {
        CObject::CArchetypeRoot(r) => r.archetype_id.value.as_str(),
        _ => parent_arch_id,
    };
    let mut node = create_node(ctx, attr_name, co, parent_path, node_arch_id);
    build_children(ctx, co, &mut node, node_arch_id);
    node
}

fn create_node(
    ctx: &Ctx,
    attr_name: Option<&str>,
    co: &CObject,
    parent_path: &str,
    arch_id: &str,
) -> WebTemplateNode {
    let rm_type = object_rm_type(co).to_owned();
    let arch_node_id = object_archetype_node_id(co);
    let (min, max) = occurrences(object_occurrences(co));
    let name_constraint = name_constraint(co);

    let path = build_path(
        parent_path,
        attr_name,
        &arch_node_id,
        name_constraint.as_deref(),
    );
    let mut node = WebTemplateNode::new(rm_type, path);
    node.node_id = if arch_node_id.is_empty() {
        None
    } else {
        Some(arch_node_id)
    };
    node.min = min;
    node.max = max;

    let name_code = object_node_id(co);
    if let Some(nc) = &name_constraint {
        node.name = Some(nc.clone());
        node.localized_name = Some(nc.clone());
    } else if !name_code.is_empty() {
        node.name = ctx.text(arch_id, name_code, &ctx.default_language);
        node.localized_name = node.name.clone();
        for lang in &ctx.languages {
            if let Some(r) = ctx.rubric(arch_id, lang, name_code) {
                if let Some(t) = &r.text {
                    node.localized_names.insert(lang.clone(), t.clone());
                }
                if let Some(d) = &r.description {
                    node.localized_descriptions.insert(lang.clone(), d.clone());
                }
            }
        }
    }
    node.name_code = if name_code.is_empty() {
        None
    } else {
        Some(name_code.to_owned())
    };
    // TODO(port): node-level term bindings come from the archetype ontology's
    // term_bindings; the corpus templates rarely carry them, so `term_bindings`
    // is left empty until that binding model is wired.
    node
}

fn build_children(ctx: &Ctx, co: &CObject, node: &mut WebTemplateNode, arch_id: &str) {
    let is_data_value = node.rm_type.starts_with("DV_");
    let recurse_attrs = !is_data_value || node.rm_type.starts_with("DV_INTERVAL");

    let mut children = Vec::new();
    if recurse_attrs {
        for attr in inputs::attributes(co) {
            let attr_name = inputs::attribute_name(attr);
            if attr_name == "name" {
                continue; // Better SKIP_PATHS.
            }
            for child_co in inputs::attribute_children(attr) {
                if matches!(
                    child_co,
                    CObject::ArchetypeSlot(_) | CObject::ConstraintRef(_)
                ) {
                    continue; // Unfilled slot / constraint ref: no node.
                }
                children.push(build_node(
                    ctx,
                    Some(attr_name),
                    child_co,
                    &node.aql_path,
                    arch_id,
                ));
            }
        }
    }

    if children.is_empty() && has_inputs(&node.rm_type) {
        let labels = ArchetypeLabels { ctx, arch_id };
        let (built, ptypes) = inputs::build_inputs(&node.rm_type, co, &labels);
        node.inputs = built;
        node.proportion_types = ptypes;
    }

    node.cardinalities = cardinalities(co, &node.aql_path);
    node.children = children;
    post_process(node);
}

fn has_inputs(rm_type: &str) -> bool {
    rm_type.starts_with("DV_")
        || rm_type == "PARTY_PROXY"
        || rm_type == "PARTY_IDENTIFIED"
        || rm_type == "CODE_PHRASE"
}

// ── post-processing (Better `postprocess/*`) ─────────────────────────────────

fn post_process(node: &mut WebTemplateNode) {
    match node.rm_type.as_str() {
        "ELEMENT" => post_process_element(node),
        "COMPOSITION" => post_process_composition(node),
        "OBSERVATION" => post_process_observation(node),
        _ => {}
    }
}

fn post_process_element(node: &mut WebTemplateNode) {
    if node.children.len() == 2 {
        let first = node.children[0].first_input_type();
        let second = node.children[1].first_input_type();
        if first == Some(WebTemplateInputType::CodedText)
            && second == Some(WebTemplateInputType::Text)
        {
            compact_to_coded_with_other(node, 0, 1);
        } else if second == Some(WebTemplateInputType::CodedText)
            && first == Some(WebTemplateInputType::Text)
        {
            compact_to_coded_with_other(node, 1, 0);
        }
    }
    if node.children.len() > 1 {
        for child in &mut node.children {
            child.min = Some(0);
        }
    }
}

fn compact_to_coded_with_other(node: &mut WebTemplateNode, coded: usize, text: usize) {
    if let Some(input) = node.children[coded].inputs.first_mut() {
        input.list_open = Some(true);
    }
    let mut other = WebTemplateInput::new(WebTemplateInputType::Text, Some("other"));
    other.list_open = Some(true);
    node.children[coded].inputs.push(other);
    node.children.remove(text);
}

fn post_process_composition(node: &mut WebTemplateNode) {
    let mut category = None;
    let mut language = None;
    for (i, child) in node.children.iter().enumerate() {
        if child.aql_path == "/category" {
            category = Some(i);
        } else if child.aql_path == "/language" {
            language = Some(i);
        }
    }
    if let Some(cat) = category {
        let node_cat = node.children.remove(cat);
        match language {
            Some(lang) if lang > 0 => node.children.insert(lang - 1, node_cat),
            _ => node.children.push(node_cat),
        }
    }
}

fn post_process_observation(node: &mut WebTemplateNode) {
    let data_path = format!("{}/data", node.aql_path);
    let state_prefix = format!("{}/state", node.aql_path);
    let protocol_prefix = format!("{}/protocol", node.aql_path);
    for child in &mut node.children {
        if child.aql_path.starts_with(&state_prefix) || child.aql_path.starts_with(&protocol_prefix)
        {
            child.depends_on = Some(vec![data_path.clone()]);
        }
    }
}

// ── compaction (Better `compactor/*`) ────────────────────────────────────────

fn compact(mut node: WebTemplateNode, depth: usize) -> Option<WebTemplateNode> {
    // Medium: hoist ALWAYS/SINGLE-compactable wrapper children into this node.
    let children = std::mem::take(&mut node.children);
    node.children = get_compacted(children, &mut node.cardinalities);
    // Recurse into the (post-hoist) children.
    let children = std::mem::take(&mut node.children);
    node.children = children
        .into_iter()
        .filter_map(|c| compact(c, depth + 1))
        .collect();
    process_children(node, depth)
}

fn get_compacted(
    children: Vec<WebTemplateNode>,
    parent_cardinalities: &mut Vec<WebTemplateCardinality>,
) -> Vec<WebTemplateNode> {
    let originals: Vec<(String, i32)> = children
        .iter()
        .map(|c| (c.rm_type.clone(), c.max))
        .collect();
    let mut out = Vec::new();
    for child in children {
        if is_compactable(&child, &originals) {
            parent_cardinalities.append(&mut child.cardinalities.clone());
            out.extend(get_compacted(child.children, parent_cardinalities));
        } else {
            out.push(child);
        }
    }
    out
}

fn is_compactable(child: &WebTemplateNode, siblings: &[(String, i32)]) -> bool {
    if ALWAYS_COMPACTABLE.contains(&child.rm_type.as_str()) {
        return true;
    }
    child.max == 1
        && SINGLE_COMPACTABLE.contains(&child.rm_type.as_str())
        && !siblings
            .iter()
            .any(|(rm, _)| rm != &child.rm_type && types_match(&child.rm_type, rm))
}

fn types_match(a: &str, b: &str) -> bool {
    a == b || (a.ends_with("EVENT") && b.ends_with("EVENT"))
}

fn process_children(mut node: WebTemplateNode, depth: usize) -> Option<WebTemplateNode> {
    compact_coded_text_with_other(&mut node.children);

    if !node.has_input() && node.children.len() == 1 && depth > 1 && is_skippable(&node.rm_type) {
        let mut child = node.children.remove(0);
        if node.rm_type == "ELEMENT" && (child.min.is_none() || child.min == Some(0)) {
            child.min = Some(1);
        }
        copy_values(&node, &mut child);
        Some(child)
    } else if node.children.is_empty() && SKIP_IF_EMPTY.contains(&node.rm_type.as_str()) {
        None
    } else {
        Some(node)
    }
}

fn is_skippable(rm_type: &str) -> bool {
    rm_type.starts_with("DV_") || rm_type == "ELEMENT"
}

/// Merge a two-child `DV_CODED_TEXT` + `DV_TEXT` pair with equal paths into a
/// coded-text-with-`other` (Better `compactCodedTextWithOther`, equal-path case).
fn compact_coded_text_with_other(children: &mut Vec<WebTemplateNode>) {
    if children.len() != 2 {
        return;
    }
    let rms = [children[0].rm_type.as_str(), children[1].rm_type.as_str()];
    let is_pair = rms.contains(&"DV_TEXT") && rms.contains(&"DV_CODED_TEXT");
    if is_pair && children[0].aql_path == children[1].aql_path {
        let (coded, text) = if children[0].rm_type == "DV_CODED_TEXT" {
            (0, 1)
        } else {
            (1, 0)
        };
        if let Some(input) = children[coded].inputs.first_mut() {
            input.list_open = Some(true);
        }
        let mut other = WebTemplateInput::new(WebTemplateInputType::Text, Some("other"));
        other.list_open = Some(true);
        children[coded].inputs.push(other);
        children.remove(text);
    }
    // TODO(port): compactMultipleCodedTexts (multiple defining_code merge).
}

/// Copy the skipped wrapper's identity/occurrences onto the promoted child
/// (Better `copyValues`).
fn copy_values(from: &WebTemplateNode, to: &mut WebTemplateNode) {
    to.name.clone_from(&from.name);
    to.localized_name.clone_from(&from.localized_name);
    to.localized_names.clone_from(&from.localized_names);
    for (k, v) in &from.localized_descriptions {
        to.localized_descriptions
            .entry(k.clone())
            .or_insert_with(|| v.clone());
    }
    to.node_id.clone_from(&from.node_id);
    to.name_code.clone_from(&from.name_code);

    let to_is_dv = to.rm_type.starts_with("DV_");
    if to.min.is_none()
        || from.min.is_some_and(|fm| to.min.is_some_and(|tm| tm > fm))
        || (to_is_dv && to.min == Some(1))
    {
        to.min = from.min;
    }
    if from.max == -1 || (to.max != -1 && to.max < from.max) {
        to.max = from.max;
    }
    for (k, v) in &from.annotations {
        to.annotations.insert(k.clone(), v.clone());
    }
    for (k, v) in &from.term_bindings {
        to.term_bindings.insert(k.clone(), v.clone());
    }
}

// ── cardinalities ────────────────────────────────────────────────────────────

fn cardinalities(co: &CObject, node_path: &str) -> Vec<WebTemplateCardinality> {
    let mut out = Vec::new();
    for attr in inputs::attributes(co) {
        if let openehr_its::opt14::CAttribute::CMultipleAttribute(m) = attr
            && requires_cardinality(&m.cardinality, m.children.len())
        {
            let (min, max) = occurrences(&m.cardinality.interval);
            out.push(WebTemplateCardinality {
                min,
                max,
                ids: None,
                path: format!("{node_path}/{}", m.rm_attribute_name),
            });
        }
    }
    out
}

fn requires_cardinality(card: &Cardinality, children_count: usize) -> bool {
    let iv = &card.interval;
    let min = if iv.lower_unbounded { None } else { iv.lower };
    let max = if iv.upper_unbounded { None } else { iv.upper };
    if min.is_none() && max.is_none() {
        return false;
    }
    let Some(min) = min else { return false };
    if min == 0 {
        return false;
    }
    let count = i32::try_from(children_count).unwrap_or(i32::MAX);
    if min == 1 && max == Some(1) && children_count == 1 {
        false
    } else {
        min > 1 || max.is_some_and(|m| m < count)
    }
}

// ── ontology / languages / other_details ─────────────────────────────────────

fn collect_languages(opt: &OperationalTemplate, default_language: &str) -> Vec<String> {
    let mut langs = vec![default_language.to_owned()];
    let mut push = |flat: &openehr_its::opt14::FlatArchetypeOntology| {
        for set in &flat.term_definitions {
            if !langs.contains(&set.language) {
                langs.push(set.language.clone());
            }
        }
    };
    if let Some(o) = &opt.ontology {
        push(o);
    }
    for o in &opt.component_ontologies {
        push(o);
    }
    langs
}

fn collect_ontology(opt: &OperationalTemplate, default_language: &str) -> Ontology {
    let mut ontology: Ontology = HashMap::new();

    let mut register_flat = |flat: &openehr_its::opt14::FlatArchetypeOntology| {
        let arch = ontology.entry(flat.archetype_id.clone()).or_default();
        for set in &flat.term_definitions {
            let lang = arch.entry(set.language.clone()).or_default();
            for term in &set.items {
                lang.entry(term.code.clone())
                    .or_insert_with(|| rubric_of(term));
            }
        }
    };
    if let Some(o) = &opt.ontology {
        register_flat(o);
    }
    for o in &opt.component_ontologies {
        register_flat(o);
    }

    // Inline C_ARCHETYPE_ROOT term_definitions are the flattened default-language
    // rubrics; register them (without overwriting an explicit ontology entry).
    collect_inline_terms(&opt.definition, default_language, &mut ontology);
    ontology
}

fn collect_inline_terms(root: &CArchetypeRoot, default_language: &str, ontology: &mut Ontology) {
    let arch = ontology.entry(root.archetype_id.value.clone()).or_default();
    let lang = arch.entry(default_language.to_owned()).or_default();
    for term in &root.term_definitions {
        lang.entry(term.code.clone())
            .or_insert_with(|| rubric_of(term));
    }
    // Recurse into nested archetype roots (each registers its own subtree's
    // rubrics under its own archetype id). Walk by reference — no cloning.
    let mut nested = Vec::new();
    for attr in &root.attributes {
        for child in inputs::attribute_children(attr) {
            collect_nested_roots(child, &mut nested);
        }
    }
    for r in nested {
        collect_inline_terms(r, default_language, ontology);
    }
}

/// Collect archetype roots at or below `co`, without descending past a root
/// (each found root handles its own subtree).
fn collect_nested_roots<'a>(co: &'a CObject, out: &mut Vec<&'a CArchetypeRoot>) {
    if let CObject::CArchetypeRoot(r) = co {
        out.push(r);
        return;
    }
    for attr in inputs::attributes(co) {
        for child in inputs::attribute_children(attr) {
            collect_nested_roots(child, out);
        }
    }
}

fn rubric_of(term: &ArchetypeTerm) -> Rubric {
    Rubric {
        text: term.items.get("text").cloned(),
        description: term.items.get("description").cloned(),
    }
}

fn other_details(opt: &OperationalTemplate) -> IndexMap<String, String> {
    let mut out = IndexMap::new();
    if let Some(desc) = &opt.description
        && let Some(details) = &desc.other_details
    {
        // Better `extractOtherDetails` keeps only `is_singleton`.
        if let Some(v) = details.get("is_singleton") {
            out.insert("is_singleton".to_owned(), v.clone());
        }
    }
    out
}

// ── opt14 object metadata ────────────────────────────────────────────────────

fn object_rm_type(co: &CObject) -> &str {
    match co {
        CObject::CComplexObject(c) => &c.rm_type_name,
        CObject::CArchetypeRoot(c) => &c.rm_type_name,
        CObject::CCodePhrase(c) => &c.rm_type_name,
        CObject::CCodeReference(c) => &c.rm_type_name,
        CObject::CDvOrdinal(c) => &c.rm_type_name,
        CObject::CDvQuantity(c) => &c.rm_type_name,
        CObject::CDvState(c) => &c.rm_type_name,
        CObject::CPrimitiveObject(c) => &c.rm_type_name,
        CObject::CDefinedObject(c) => &c.rm_type_name,
        CObject::ArchetypeInternalRef(c) => &c.rm_type_name,
        CObject::ArchetypeSlot(c) => &c.rm_type_name,
        CObject::ConstraintRef(c) => &c.rm_type_name,
        CObject::TComplexObject(c) => &c.rm_type_name,
    }
}

fn object_node_id(co: &CObject) -> &str {
    match co {
        CObject::CComplexObject(c) => &c.node_id,
        CObject::CArchetypeRoot(c) => &c.node_id,
        CObject::CCodePhrase(c) => &c.node_id,
        CObject::CCodeReference(c) => &c.node_id,
        CObject::CDvOrdinal(c) => &c.node_id,
        CObject::CDvQuantity(c) => &c.node_id,
        CObject::CDvState(c) => &c.node_id,
        CObject::CPrimitiveObject(c) => &c.node_id,
        CObject::CDefinedObject(c) => &c.node_id,
        CObject::ArchetypeInternalRef(c) => &c.node_id,
        CObject::ArchetypeSlot(c) => &c.node_id,
        CObject::ConstraintRef(c) => &c.node_id,
        CObject::TComplexObject(c) => &c.node_id,
    }
}

/// The RM `archetype_node_id`: the archetype id at archetype roots, else the
/// constraint node id (`atNNNN`).
fn object_archetype_node_id(co: &CObject) -> String {
    match co {
        CObject::CArchetypeRoot(c) => c.archetype_id.value.clone(),
        _ => object_node_id(co).to_owned(),
    }
}

fn object_occurrences(co: &CObject) -> &Intervalofinteger {
    match co {
        CObject::CComplexObject(c) => &c.occurrences,
        CObject::CArchetypeRoot(c) => &c.occurrences,
        CObject::CCodePhrase(c) => &c.occurrences,
        CObject::CCodeReference(c) => &c.occurrences,
        CObject::CDvOrdinal(c) => &c.occurrences,
        CObject::CDvQuantity(c) => &c.occurrences,
        CObject::CDvState(c) => &c.occurrences,
        CObject::CPrimitiveObject(c) => &c.occurrences,
        CObject::CDefinedObject(c) => &c.occurrences,
        CObject::ArchetypeInternalRef(c) => &c.occurrences,
        CObject::ArchetypeSlot(c) => &c.occurrences,
        CObject::ConstraintRef(c) => &c.occurrences,
        CObject::TComplexObject(c) => &c.occurrences,
    }
}

/// `(min, max)` from an occurrences/cardinality interval; `max == -1` unbounded.
fn occurrences(iv: &Intervalofinteger) -> (Option<i32>, i32) {
    let min = if iv.lower_unbounded { None } else { iv.lower };
    let max = if iv.upper_unbounded { None } else { iv.upper };
    (min, max.unwrap_or(-1))
}

/// A single fixed name constraint (`name/value` `C_STRING` with one value), used
/// for the `[atNNNN,'Name']` path predicate.
fn name_constraint(co: &CObject) -> Option<String> {
    for name_child in inputs::attr_children(co, "name") {
        if let Some(CPrimitive::CString(cs)) = inputs::primitive_under(name_child, "value")
            && cs.list.len() == 1
        {
            return cs.list.first().cloned();
        }
    }
    None
}

fn build_path(
    parent_path: &str,
    attr_name: Option<&str>,
    arch_node_id: &str,
    name_constraint: Option<&str>,
) -> String {
    let Some(attr) = attr_name else {
        return String::new(); // root
    };
    let predicate = if arch_node_id.is_empty() {
        String::new()
    } else if let Some(name) = name_constraint {
        format!("[{arch_node_id},'{name}']")
    } else {
        format!("[{arch_node_id}]")
    };
    format!("{parent_path}/{attr}{predicate}")
}
