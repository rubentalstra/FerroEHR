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
//! synthesized descendants rather than incorrectly. Node- and coded-value-level
//! external `termBindings` and the multiple-coded-text compaction are wired.

use std::collections::HashMap;

use indexmap::IndexMap;
use openehr_its::opt14::{
    ArchetypeTerm, Assertion, CArchetypeRoot, CObject, CPrimitive, Cardinality, Intervalofinteger,
    OperationalTemplate, TermBindingItem, Termbindingset,
};

use super::inputs::{self, Labels};
use super::model::{
    WebTemplate, WebTemplateArchetypeSlot, WebTemplateBindingCodedValue, WebTemplateCardinality,
    WebTemplateClosedAttribute, WebTemplateCodeList, WebTemplateExistence, WebTemplateInput,
    WebTemplateInputType, WebTemplateNode, WebTemplateSlot,
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

/// External term bindings per archetype, keyed by `archetype_id`: the archetype
/// root's `term_bindings`. A node inherits its owning archetype root's bindings.
type TermBindings = HashMap<String, Vec<Termbindingset>>;

/// Build context shared across the walk.
struct Ctx {
    default_language: String,
    languages: Vec<String>,
    ontology: Ontology,
    term_bindings: TermBindings,
}

impl Ctx {
    fn rubric(&self, arch_id: &str, lang: &str, code: &str) -> Option<&Rubric> {
        self.ontology.get(arch_id)?.get(lang)?.get(code)
    }

    fn text(&self, arch_id: &str, code: &str, lang: &str) -> Option<String> {
        self.rubric(arch_id, lang, code)
            .and_then(|r| r.text.clone())
    }

    /// The external term bindings for `code` within `arch_id`'s archetype
    /// (Better `findTermBindings` + `getBindingCodedValue`): for every terminology
    /// whose binding set has an item matching `code`, the bound code phrase as a
    /// `{value, terminologyId}`. Keyed by terminology, first match per terminology
    /// wins, in binding-set order.
    fn term_bindings_for(
        &self,
        arch_id: &str,
        code: &str,
    ) -> IndexMap<String, WebTemplateBindingCodedValue> {
        let mut out = IndexMap::new();
        if code.is_empty() {
            return out;
        }
        let Some(sets) = self.term_bindings.get(arch_id) else {
            return out;
        };
        for set in sets {
            if out.contains_key(&set.terminology) {
                continue;
            }
            if let Some(item) = set.items.iter().find(|it| it.code == code)
                && let Some(bcv) = binding_coded_value(item)
            {
                out.insert(set.terminology.clone(), bcv);
            }
        }
        out
    }
}

/// Better `CodePhraseUtils.getBindingCodedValue`: the bound code phrase's code
/// string with its terminology id (falling back to the binding item's own code
/// when the code phrase carries no terminology). `None` when the bound code
/// string is blank.
fn binding_coded_value(item: &TermBindingItem) -> Option<WebTemplateBindingCodedValue> {
    let code_phrase = &item.value;
    if code_phrase.code_string.trim().is_empty() {
        return None;
    }
    let terminology_id = if code_phrase.terminology_id.value.is_empty() {
        item.code.clone()
    } else {
        code_phrase.terminology_id.value.clone()
    };
    Some(WebTemplateBindingCodedValue {
        value: code_phrase.code_string.clone(),
        terminology_id,
    })
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

    fn term_bindings(&self, code: &str) -> IndexMap<String, WebTemplateBindingCodedValue> {
        self.ctx.term_bindings_for(self.arch_id, code)
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
        term_bindings: collect_term_bindings(opt),
        default_language,
    };

    let root_arch_id = opt.definition.archetype_id.value.clone();
    let root_co = CObject::CArchetypeRoot(opt.definition.clone());
    let mut tree = build_node(&ctx, None, &root_co, "", &root_arch_id);
    tree = compact(tree, 1).unwrap_or(tree_placeholder());
    super::id::build_ids(&mut tree);

    Ok(WebTemplate {
        template_id,
        // PORT NOTE: OPT 1.4 has no semantic-version field (semVer is an ADL2/OPT2
        // concept), so the 1.4 adapter always emits `null` — matching what stock
        // tooling produces for a 1.4 template. A value would only appear for OPT 2.
        sem_ver: None,
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
    // Node-level external term bindings: the archetype root's `term_bindings`
    // whose item code matches this node's constraint node id (Better
    // `WebTemplateBuilder.setTermBindings` via `amNode.nodeId`).
    node.term_bindings = ctx.term_bindings_for(arch_id, name_code);
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
        capture_leaf_constraints(co, node);
    }

    node.cardinalities = cardinalities(co, &node.aql_path);
    node.card_all = all_cardinalities(co, &node.aql_path);
    // Existence is captured only for structural (attribute-recursing) nodes; a
    // DATA_VALUE leaf's constraints (`magnitude`, `is_integral`, `value`, …) are
    // handled by `inputs`/leaf checks, not attribute navigation (F-07-04).
    if recurse_attrs {
        node.existence = existence_constraints(co, &node.aql_path);
        node.closed_attributes = closed_attributes(co, &node.aql_path);
    }
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
    let mut hoisted = Hoisted {
        cardinalities: std::mem::take(&mut node.cardinalities),
        existence: std::mem::take(&mut node.existence),
        card_all: std::mem::take(&mut node.card_all),
        slots: std::mem::take(&mut node.slots),
        closed_attributes: std::mem::take(&mut node.closed_attributes),
    };
    node.children = get_compacted(children, &mut hoisted);
    node.cardinalities = hoisted.cardinalities;
    node.existence = hoisted.existence;
    node.card_all = hoisted.card_all;
    node.slots = hoisted.slots;
    node.closed_attributes = hoisted.closed_attributes;
    // Recurse into the (post-hoist) children.
    let children = std::mem::take(&mut node.children);
    node.children = children
        .into_iter()
        .filter_map(|c| compact(c, depth + 1))
        .collect();
    process_children(node, depth)
}

/// The constraint sets a hoisted wrapper re-homes onto its parent (all keyed by
/// absolute archetype paths, so they stay valid after the hoist — F-07-04).
struct Hoisted {
    cardinalities: Vec<WebTemplateCardinality>,
    existence: Vec<WebTemplateExistence>,
    card_all: Vec<WebTemplateCardinality>,
    slots: Vec<WebTemplateSlot>,
    closed_attributes: Vec<WebTemplateClosedAttribute>,
}

fn get_compacted(children: Vec<WebTemplateNode>, parent: &mut Hoisted) -> Vec<WebTemplateNode> {
    let originals: Vec<(String, i32)> = children
        .iter()
        .map(|c| (c.rm_type.clone(), c.max))
        .collect();
    let mut out = Vec::new();
    for mut child in children {
        if is_compactable(&child, &originals) {
            parent
                .cardinalities
                .append(&mut child.cardinalities.clone());
            // A hoisted wrapper's existence/cardinality constraints (on its own
            // attributes, e.g. HISTORY.events) reference absolute archetype
            // paths, so they stay valid when re-homed on the parent — the walk
            // still enforces them (F-07-04).
            parent
                .existence
                .append(&mut std::mem::take(&mut child.existence));
            parent
                .card_all
                .append(&mut std::mem::take(&mut child.card_all));
            parent.slots.append(&mut std::mem::take(&mut child.slots));
            parent
                .closed_attributes
                .append(&mut std::mem::take(&mut child.closed_attributes));
            // The hoisted wrapper's own RM type is an archetype constraint the
            // compacted tree would otherwise lose: record it as a slot so the
            // walk still rejects a sibling subtype in a narrowed
            // ITEM_STRUCTURE/EVENT slot (AOM 1.4 type conformance,
            // master16 §ITEM_STRUCTURE/§EVENT "Class not allowed").
            parent.slots.push(WebTemplateSlot {
                path: child.aql_path.clone(),
                rm_type: child.rm_type.clone(),
            });
            out.extend(get_compacted(child.children, parent));
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
    compact_multiple_coded_texts(&mut node.children);

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
}

/// Merge multiple sibling coded-text alternatives that share an aql path into a
/// single coded node (Better `WebTemplateCompactor.compactMultipleCodedTexts`):
/// when an ELEMENT `value` is a choice of coded texts, one node carries the
/// union of the alternatives' coded values rather than several sibling nodes
/// with polymorphic (`value`/`value2`) ids.
///
/// Better's rule, ported: children are grouped by path; a group of exactly two
/// coded-text nodes is compacted — if one carries a validation-constrained input
/// and the other does not, the constrained one is kept and the other dropped;
/// otherwise the two coded lists are unioned (dedup by code, order preserved)
/// onto the first node and the second is dropped.
///
/// PORT NOTE: our coded-text nodes fold `defining_code` into the node's `inputs`
/// (Better keeps each `C_CODE_PHRASE` as its own `.../defining_code` node), so the
/// qualifying pair is two same-path `DV_CODED_TEXT`/`CODE_PHRASE` siblings — the
/// equivalent of Better's `path.endsWith("defining_code")` — and the coded lists
/// are unioned directly rather than through Better's separate-node `mergeInputs`.
fn compact_multiple_coded_texts(children: &mut Vec<WebTemplateNode>) {
    // Group child indices by aql path, in first-seen order (Better
    // `mergeChildrenWithMatchingPaths`).
    let mut groups: IndexMap<String, Vec<usize>> = IndexMap::new();
    for (i, child) in children.iter().enumerate() {
        groups.entry(child.aql_path.clone()).or_default().push(i);
    }

    let mut to_remove: Vec<usize> = Vec::new();
    // (keep, drop) pairs whose coded lists are unioned.
    let mut merges: Vec<(usize, usize)> = Vec::new();
    for idxs in groups.values() {
        if idxs.len() != 2 {
            continue; // Better only compacts an exact pair.
        }
        let (a, b) = (idxs[0], idxs[1]);
        if !is_coded_text(&children[a]) || !is_coded_text(&children[b]) {
            continue;
        }
        let constrained_a = is_input_constrained(&children[a]);
        let constrained_b = is_input_constrained(&children[b]);
        if constrained_a && !constrained_b {
            to_remove.push(b);
        } else if constrained_b && !constrained_a {
            to_remove.push(a);
        } else {
            merges.push((a, b));
        }
    }

    for (keep, drop) in merges {
        let drop_list = children[drop]
            .inputs
            .first()
            .map(|i| i.list.clone())
            .unwrap_or_default();
        if let Some(input) = children[keep].inputs.first_mut() {
            for cv in drop_list {
                if !input.list.iter().any(|existing| existing.value == cv.value) {
                    input.list.push(cv);
                }
            }
        }
        to_remove.push(drop);
    }

    to_remove.sort_unstable();
    to_remove.dedup();
    for i in to_remove.into_iter().rev() {
        children.remove(i);
    }
}

/// A coded-text-family node (its `value` is `DV_CODED_TEXT`/`CODE_PHRASE`).
fn is_coded_text(node: &WebTemplateNode) -> bool {
    matches!(node.rm_type.as_str(), "DV_CODED_TEXT" | "CODE_PHRASE")
}

/// Better `isConstrained`: the node's first input carries a validation.
fn is_input_constrained(node: &WebTemplateNode) -> bool {
    node.inputs.first().is_some_and(|i| i.validation.is_some())
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
    // Validation-only constraint sets survive the promotion (absolute paths; a
    // path that is not a descendant of the promoted node's own aql path is
    // neutralised by the walk's prefix-strip).
    to.existence.extend(from.existence.iter().cloned());
    to.card_all.extend(from.card_all.iter().cloned());
    to.slots.extend(from.slots.iter().cloned());
    to.closed_attributes
        .extend(from.closed_attributes.iter().cloned());
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

/// Capture **every** constraining multiple-attribute cardinality for the
/// validation walk (AOM 1.4 `master04-constraint_model_package.adoc`
/// §cardinality): any interval with a lower bound `>= 1` or a bounded upper
/// bound constrains the container. This is the superset of the Better-filtered
/// [`requires_cardinality`] selection, which drops `0..1`/`1..1`/`1..*` — those
/// intervals are still real archetype constraints (master15/16 truth tables)
/// and are enforced from [`WebTemplateNode::card_all`], never serialized.
fn all_cardinalities(co: &CObject, node_path: &str) -> Vec<WebTemplateCardinality> {
    let mut out = Vec::new();
    for attr in inputs::attributes(co) {
        if let openehr_its::opt14::CAttribute::CMultipleAttribute(m) = attr {
            let (min, max) = occurrences(&m.cardinality.interval);
            if min.unwrap_or(0) >= 1 || max != -1 {
                out.push(WebTemplateCardinality {
                    min,
                    max,
                    ids: None,
                    path: format!("{node_path}/{}", m.rm_attribute_name),
                });
            }
        }
    }
    out
}

/// Capture the leaf value constraints the Better `inputs` mapping does not
/// carry, onto the node's validation-only fields:
///
/// - `C_INTEGER.list` / `C_REAL.list` on a numeric datum (`magnitude`, `value`)
///   → [`WebTemplateNode::numeric_lists`] (AOM 1.4 §`C_INTEGER/§C_REAL`);
/// - `C_DURATION.range` on `value` → [`WebTemplateNode::duration_range`]
///   (AOM 1.4 §`C_DURATION`);
/// - `C_CODE_PHRASE` code lists on coded attributes other than
///   `defining_code` (e.g. `DV_MULTIMEDIA.media_type`) →
///   [`WebTemplateNode::code_lists`] (AOM 1.4 §`C_CODE_PHRASE`).
fn capture_leaf_constraints(co: &CObject, node: &mut WebTemplateNode) {
    for datum in ["magnitude", "value", "numerator", "denominator"] {
        match inputs::primitive_under(co, datum) {
            Some(CPrimitive::CInteger(ci)) if !ci.list.is_empty() => {
                node.numeric_lists.push((
                    datum.to_owned(),
                    ci.list.iter().map(|v| f64::from(*v)).collect(),
                ));
            }
            Some(CPrimitive::CReal(cr)) if !cr.list.is_empty() => {
                node.numeric_lists.push((datum.to_owned(), cr.list.clone()));
            }
            _ => {}
        }
    }
    if let Some(CPrimitive::CDuration(d)) = inputs::primitive_under(co, "value")
        && let Some(range) = &d.range
    {
        let min = if range.lower_unbounded {
            None
        } else {
            range.lower.clone()
        };
        let max = if range.upper_unbounded {
            None
        } else {
            range.upper.clone()
        };
        if min.is_some() || max.is_some() {
            node.duration_range = Some(super::model::WebTemplateRange {
                min_op: min.as_ref().map(|_| ">=".to_owned()),
                min: min.map(serde_json::Value::String),
                max_op: max.as_ref().map(|_| "<=".to_owned()),
                max: max.map(serde_json::Value::String),
            });
        }
    }
    // C_TIME/C_DATE_TIME timezone_validity (VALIDITY_KIND: OPT 1.4 XSD 1001 =
    // mandatory, 1002 = optional, 1003 = disallowed). C_DATE has no timezone.
    node.tz_validity = match inputs::primitive_under(co, "value") {
        Some(CPrimitive::CTime(c)) => c.timezone_validity.as_deref().and_then(|s| s.parse().ok()),
        Some(CPrimitive::CDateTime(c)) => {
            c.timezone_validity.as_deref().and_then(|s| s.parse().ok())
        }
        _ => None,
    };
    for attr in inputs::attributes(co) {
        let attr_name = inputs::attribute_name(attr);
        if attr_name == "defining_code" {
            continue; // Modelled by the coded-text `inputs`.
        }
        for child in inputs::attribute_children(attr) {
            if let CObject::CCodePhrase(cp) = child
                && !cp.code_list.is_empty()
            {
                node.code_lists.push(WebTemplateCodeList {
                    attr: attr_name.to_owned(),
                    terminology: cp
                        .terminology_id
                        .as_ref()
                        .map(|t| t.value.clone())
                        .filter(|t| !t.is_empty()),
                    codes: cp.code_list.clone(),
                });
            }
        }
    }
}

// ── existence (AOM 1.4 C_ATTRIBUTE.existence) ─────────────────────────────────

/// Capture the AOM 1.4 `C_ATTRIBUTE.existence` constraints for the mandatory,
/// plain (non-archetype-node-identified) single-valued RM attributes of `co`,
/// keyed by their absolute archetype path (F-07-04).
///
/// Scope: only `C_SINGLE_ATTRIBUTE`s with an existence lower bound `>= 1` whose
/// constraint children carry **no** `node_id`. Archetype-node-identified children
/// are governed by *occurrences* (checked in the walk), and container membership
/// by *cardinality* — existence covers exactly the remaining case: a mandatory
/// plain RM attribute field (e.g. an ELEMENT `value`, a `HISTORY.events`,
/// `COMPOSITION.language`) that must be present. `name` is excluded (a Better
/// `SKIP_PATH`, matched by the archetype-node predicate instead).
///
/// PORT NOTE: AOM 1.4 (`master04-constraint_model_package.adoc` §existence) makes
/// existence "always required" with an unstated default of `{1..1}`; the OPT XML
/// always serialises it, and we honour the declared value (biasing toward
/// confident violations — an unstated/`{0..1}` existence is not enforced).
fn existence_constraints(co: &CObject, node_path: &str) -> Vec<WebTemplateExistence> {
    let mut out = Vec::new();
    for attr in inputs::attributes(co) {
        let openehr_its::opt14::CAttribute::CSingleAttribute(s) = attr else {
            continue;
        };
        let (min, max) = occurrences(&s.existence);
        let min = min.unwrap_or(0);
        // Require at least one non-primitive (object-valued) constraint child —
        // this targets real structural attributes (`value`, `language`, `data`,
        // `events`, `items`, …) and excludes function/primitive constraints
        // (`is_integral`, `lower_included`, …) that never appear as navigable
        // instance attributes. A **childless** mandatory attribute also counts:
        // AOM 1.4 (`master04-constraint_model_package.adoc` §existence) lets an
        // archetype demand an attribute's presence without constraining its
        // value (a bare `C_SINGLE_ATTRIBUTE` with existence `1..1`, e.g. a
        // mandatory `COMPOSITION.context` or `HISTORY.summary`).
        let object_valued = s.children.is_empty()
            || s.children
                .iter()
                .any(|c| !matches!(c, CObject::CPrimitiveObject(_)));
        if min >= 1
            && s.rm_attribute_name != "name"
            && object_valued
            && s.children.iter().all(|c| object_node_id(c).is_empty())
        {
            out.push(WebTemplateExistence {
                min,
                max,
                path: format!("{node_path}/{}", s.rm_attribute_name),
            });
        }
    }
    out
}

// ── closed-archetype constraints (ADR-012 F-07-05 + F-07-10) ──────────────────

/// Capture the closed-archetype constraints for the walk (ADR-012): per
/// attribute of `co` that carries **archetype-node-identified** child
/// alternatives (a fixed at-code / archetype-id sibling set) and/or open
/// `ARCHETYPE_SLOT`s, record the admissible child identities keyed by the
/// attribute's absolute archetype path. Captured from the raw OPT `co` (before
/// the tree build drops slots and before compaction hoists wrappers), so no
/// alternative is lost.
///
/// An attribute whose constraint children carry **no** `node_id` and has no slot
/// is left OPEN — AOM 1.4 (`master04-constraint_model_package.adoc` §node_id
/// L44: a near-leaf with no same-attribute siblings "can safely have no
/// node_id") — matching the RM-metadata / plain-attribute carve-out (ADR-012
/// rule 2): `name`/`value`/`category`/`context` etc. hold non-LOCATABLE values
/// that carry no `archetype_node_id` and so are never subject to sibling closure.
fn closed_attributes(co: &CObject, node_path: &str) -> Vec<WebTemplateClosedAttribute> {
    let mut out = Vec::new();
    for attr in inputs::attributes(co) {
        let attr_name = inputs::attribute_name(attr);
        if attr_name == "name" {
            continue; // Better SKIP_PATH; the name is matched by predicate, not closure.
        }
        // An unresolved internal-ref / constraint-ref makes the admissible set
        // uncertain (target resolution is a documented builder scope gap); leave
        // such an attribute OPEN rather than risk over-rejecting.
        if inputs::attribute_children(attr).iter().any(|c| {
            matches!(
                c,
                CObject::ArchetypeInternalRef(_) | CObject::ConstraintRef(_)
            )
        }) {
            continue;
        }
        let mut allowed_ids: Vec<String> = Vec::new();
        let mut slots: Vec<WebTemplateArchetypeSlot> = Vec::new();
        for child in inputs::attribute_children(attr) {
            if let CObject::ArchetypeSlot(s) = child {
                slots.push(archetype_slot(s));
                continue;
            }
            let id = object_archetype_node_id(child);
            if !id.is_empty() {
                allowed_ids.push(id);
            }
        }
        if allowed_ids.is_empty() && slots.is_empty() {
            continue; // Open attribute (no node-id alternatives, no slot).
        }
        out.push(WebTemplateClosedAttribute {
            path: format!("{node_path}/{attr_name}"),
            allowed_ids,
            slots,
        });
    }
    out
}

/// Build the validation-only slot record from an OPT `ARCHETYPE_SLOT`: its
/// constrained RM type, occurrences bounds, and the archetype-id regexes lifted
/// from the `includes`/`excludes` assertions (AOM 1.4 `ARCHETYPE_SLOT`).
fn archetype_slot(s: &openehr_its::opt14::ArchetypeSlot) -> WebTemplateArchetypeSlot {
    let (min, max) = occurrences(&s.occurrences);
    WebTemplateArchetypeSlot {
        rm_type: s.rm_type_name.clone(),
        min: min.unwrap_or(0).max(0),
        max,
        includes: s.includes.iter().filter_map(slot_pattern).collect(),
        excludes: s.excludes.iter().filter_map(slot_pattern).collect(),
    }
}

/// The archetype-id regex of a slot `ASSERTION`, lifted from its
/// `string_expression` (`archetype_id/value matches {/<regex>/}` — ADL 1.4
/// `master05-cadl.adoc` §Archetype Slots; the OPT always emits this surface
/// form). Archetype ids contain no `/`, so the last `/}` delimits the regex.
fn slot_pattern(a: &Assertion) -> Option<String> {
    let s = a.string_expression.as_deref()?;
    let start = s.find("matches {/")? + "matches {/".len();
    let rest = &s[start..];
    let end = rest.rfind("/}")?;
    Some(rest[..end].to_owned())
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

/// Collect every archetype root's inline `term_bindings`, keyed by archetype id
/// (Better attaches these to the archetype-root `AmNode`, inherited by
/// descendants). First root wins for a repeated archetype id.
fn collect_term_bindings(opt: &OperationalTemplate) -> TermBindings {
    let mut out: TermBindings = HashMap::new();
    collect_root_bindings(&opt.definition, &mut out);
    out
}

fn collect_root_bindings(root: &CArchetypeRoot, out: &mut TermBindings) {
    if !root.term_bindings.is_empty() {
        out.entry(root.archetype_id.value.clone())
            .or_insert_with(|| root.term_bindings.clone());
    }
    // Recurse into nested archetype roots (each carries its own bindings).
    let mut nested = Vec::new();
    for attr in &root.attributes {
        for child in inputs::attribute_children(attr) {
            collect_nested_roots(child, &mut nested);
        }
    }
    for r in nested {
        collect_root_bindings(r, out);
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
