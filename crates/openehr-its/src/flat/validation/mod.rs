//! Composition validation.
//!
//! [`validate_composition`] validates a canonical-JSON COMPOSITION against its
//! operational template (as a flattened [`WebTemplate`]) **plus** the openEHR
//! Reference Model class invariants **plus** the RM-mandated openEHR
//! terminology, collecting *all* violations (not fail-fast), each keyed by an RM
//! path. It composes three building blocks: the [`WebTemplate`] tree,
//! [`crate::rm_validate::validate_rm_invariants`] (the core RM class
//! invariants), and the shared terminology hook
//! [`crate::rm_terminology`] (openEHR terminology, backed by
//! [`openehr_term::bundle`]).
//!
//! # The three passes
//!
//! Validation runs three independent collecting passes over the instance:
//!
//! 1. **RM-invariant pass** — recurse the whole instance; for every node with a
//!    `_type`, run its core RM class invariants ([`validate_rm_invariants`]).
//!    This is
//!    independent of the (compacted) `WebTemplate`, so class invariants on nodes
//!    the `WebTemplate` folds away (ELEMENT / `ITEM_TREE` / HISTORY / EVENT) are
//!    still checked. Paths are RM *instance* paths (`/content[0]/…`).
//! 2. **Terminology pass** — recurse the instance; validate the RM-mandated
//!    openEHR-terminology-group codes (composition `category`, context
//!    `setting`, `null_flavour`, `ISM_TRANSITION` `current_state`, PARTICIPATION
//!    `function`/`mode`, …) against [`openehr_term::bundle`].
//! 3. **`WebTemplate` (archetype-conformance) pass** — walk the instance guided by
//!    the `WebTemplate` tree, matching by `aql_path` + `archetype_node_id`, and
//!    check type conformance, occurrences, cardinality, and leaf domain
//!    constraints (coded lists / numeric ranges / string patterns). Paths are
//!    the archetype `aqlPath` of the constraining node.
//!
//! # Simplified-input surface
//!
//! The COMPOSITION passes above see the already-built RM tree. Two of the
//! Simplified-Formats input rules the ITS-REST spec lists under
//! `simplified_formats/master04-basic_concepts.adoc` §Validation are properties
//! of the FLAT/parsed *input*, checked before conversion to RM:
//! [`validate_flat_other`] (the `|other` open-value-set rules) and
//! [`validate_context`] (the mandatory `ctx/language` + `ctx/territory` context
//! fields). Both return the same [`ValidationMessage`] report shape.
//!
//! # Fidelity
//!
//! The instance-validation *algorithm* is spec-underdetermined — the AOM 1.4
//! constraint model defines a positive-only cascade (`Valid_value`) over the
//! archetype constraint tree and is silent on unmatched instance nodes. We
//! approximate that walk over the *compacted* `WebTemplate`: the wrapper nodes
//! the AOM constraint tree carries (ELEMENT / `ITEM_STRUCTURE` / HISTORY / EVENT)
//! are folded into a child's `aqlPath`, so we navigate the instance by the
//! RM-attribute + `[archetype_node_id]` predicate chain that separates a
//! `WebTemplate` child from its (compacted) parent, counting occurrences per
//! intermediate container. The `C_DV_*` leaf semantics (unit-scoped magnitude
//! ranges, coded-value membership, string patterns) are approximated from the
//! `WebTemplate` `inputs`. Where a check cannot be made reliably (temporal
//! ranges, precision, `depends_on` choices, unresolved archetype slots/internal
//! refs, deep required nodes behind an absent optional wrapper) we skip rather
//! than over-reject — biasing toward reporting only confident violations.

mod leaf;
mod subtype;
mod terminology;

use crate::rm_validate::validate_rm_invariants;
use indexmap::IndexMap;
use openehr_rm::paths::PathSegment;
use serde_json::{Map, Value};

use crate::flat::path;
use crate::flat::rmpath;
use crate::flat::sim::{SimDocument, SimNode, is_present};
use crate::flat::webtemplate::{WebTemplate, WebTemplateArchetypeSlot, WebTemplateNode};

/// A single validation violation, keyed by the RM path of the offending node
/// (a message + path + violation kind).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMessage {
    /// The RM path to the offending node (archetype `aqlPath` for
    /// archetype-conformance violations, RM instance path for RM-invariant /
    /// terminology violations).
    pub path: String,
    /// A human-readable description of the violation.
    pub message: String,
    /// The violation category.
    pub kind: ValidationKind,
}

/// The category of a [`ValidationMessage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationKind {
    /// An instance node's RM type does not conform to the constraint's type.
    WrongType,
    /// A mandatory node (`min >= 1`) is absent.
    Required,
    /// The number of matching nodes is outside the occurrences range.
    Occurrences,
    /// A container attribute's child count is outside its cardinality range.
    Cardinality,
    /// A numeric value is outside the constrained range.
    RangeError,
    /// A string value does not match the constrained pattern.
    PatternError,
    /// A coded value is not among the constrained coded options.
    CodedValue,
    /// A code is not valid in its RM-mandated openEHR terminology group.
    Terminology,
    /// An RM class invariant failed.
    Invariant,
    /// An instance node is not admitted by any sibling constraint or open slot
    /// under a closed (constrained) attribute (closed-world admission per the
    /// AOM2 direction — `AM/docs/AOM2/master04.2` `Rm_type_name` matching).
    Unexpected,
    /// Any other violation.
    Other,
}

/// Validate a canonical-JSON COMPOSITION against its `WebTemplate`, RM invariants,
/// and the RM-mandated openEHR terminology. Returns every violation found (the
/// validator does not stop at the first error); an empty result means the
/// composition is valid to the extent this validator checks.
#[must_use]
pub fn validate_composition(composition: &Value, wt: &WebTemplate) -> Vec<ValidationMessage> {
    let mut v = Validator::default();
    // One reusable path buffer across passes 1 and 2 (each leaves it empty).
    let mut path = String::new();
    // Pass 1: RM class invariants over the whole instance (compaction-independent).
    v.rm_invariant_pass(composition, &mut path);
    // Pass 2: RM-mandated openEHR terminology.
    v.terminology_pass(composition, &mut path, None);
    // Pass 3: archetype conformance guided by the WebTemplate tree.
    v.walk(composition, &wt.tree);
    v.out
}

/// Validate only the **template-independent** passes: RM class invariants + the
/// RM-mandated openEHR terminology. These hold for *every* RM instance whether
/// or not an operational template is referenced (RM invariants and terminology
/// bindings are properties of the instance, not of the archetype). A COMPOSITION
/// committed without a declared `template_id` cannot be archetype-conformance-
/// checked, but must still pass these.
#[must_use]
pub fn validate_rm_and_terminology(composition: &Value) -> Vec<ValidationMessage> {
    let mut v = Validator::default();
    let mut path = String::new();
    v.rm_invariant_pass(composition, &mut path);
    v.terminology_pass(composition, &mut path, None);
    v.out
}

/// Validate only the archetype-conformance pass against a resolved
/// [`WebTemplate`] (type conformance, occurrences, cardinality, and leaf domain
/// constraints). Callers run [`validate_rm_and_terminology`] separately for the
/// template-independent checks, so this is the additional pass a *declared*
/// template contributes.
#[must_use]
pub fn validate_archetype_conformance(
    composition: &Value,
    wt: &WebTemplate,
) -> Vec<ValidationMessage> {
    let mut v = Validator::default();
    v.walk(composition, &wt.tree);
    v.out
}

/// Archetype-conformance pass for a `553|incomplete|` commit: identical to
/// [`validate_archetype_conformance`] but with existence/occurrences/cardinality
/// **lower** limits treated as zero (RM common master06 §"Incomplete Content":
/// "in an `incomplete` commit, data may be missing, but it may not be wrong …
/// all existence and cardinality lower limits set to zero"). Type conformance,
/// upper limits, and every leaf/value constraint are still enforced — the
/// relaxation tolerates *missing* data, not *wrong* data. The caller must run
/// the RM-invariant + terminology passes at full strictness regardless
/// (they are properties of the instance, not archetype lower bounds).
#[must_use]
pub fn validate_archetype_conformance_incomplete(
    composition: &Value,
    wt: &WebTemplate,
) -> Vec<ValidationMessage> {
    let mut v = Validator {
        relax_lower_bounds: true,
        ..Validator::default()
    };
    v.walk(composition, &wt.tree);
    v.out
}

/// Validate the `|other` open-value-set rules on a **FLAT input** map, before
/// conversion to RM (ITS-REST `simplified_formats/master04-basic_concepts.adoc`
/// §"Open Value-Sets and the `|other` Suffix"):
///
/// * `|other` MUST NOT co-occur with `|code`/`|value`/`|terminology`/
///   `|preferred_term` on the same leaf path;
/// * `|other` MUST be rejected when the leaf's WT constraint is a **closed**
///   coded list (`listOpen: false`).
///
/// This is the validation-**report** form of the same two rules the conversion
/// path enforces as hard rejections: [`crate::flat::map::build_leaf`] returns
/// [`crate::flat::error::FlatError::OtherSuffixConflict`] /
/// [`crate::flat::error::FlatError::OtherOnClosedValueSet`] when it hits them during a
/// build, so a converter fails fast. This function instead collects them as
/// [`ValidationMessage`]s (validators report, converters reject), for a caller
/// that wants to surface every violation on the raw FLAT input without running a
/// conversion. An empty result means the input satisfies the `|other` rules this
/// check covers.
#[must_use]
pub fn validate_flat_other(doc: &Map<String, Value>, wt: &WebTemplate) -> Vec<ValidationMessage> {
    const EXCLUSIVE: &[&str] = &["code", "value", "terminology", "preferred_term"];
    let mut out = Vec::new();
    // Group each FLAT key by its segment path (`master04 §Field Identifiers`),
    // collecting the datum suffix (`master05` per-type suffix) present on each.
    let mut per_leaf: IndexMap<String, LeafGroup> = IndexMap::new();
    for key in doc.keys() {
        let Ok(fk) = path::FlatKey::parse(key) else {
            continue; // a malformed key is the conversion path's rejection, not this one's
        };
        if fk.is_ctx() {
            continue;
        }
        // The segment portion is everything before the first pipe (the printed
        // path); the first suffix is the leaf's datum suffix (`master05`).
        let path_str = key.split('|').next().unwrap_or(key).to_owned();
        let suffix = fk.suffixes.first().map(|s| s.name.clone());
        let entry = per_leaf.entry(path_str).or_insert_with(|| LeafGroup {
            segments: fk.segments,
            suffixes: Vec::new(),
        });
        if let Some(s) = suffix {
            entry.suffixes.push(s);
        }
    }
    for (path_str, group) in &per_leaf {
        if !group.suffixes.iter().any(|s| s == "other") {
            continue;
        }
        // Mutual exclusion.
        if let Some(conflict) = group
            .suffixes
            .iter()
            .find(|s| EXCLUSIVE.contains(&s.as_str()))
        {
            out.push(ValidationMessage {
                path: path_str.clone(),
                message: format!(
                    "`|other` is mutually exclusive with `|{conflict}` on the same leaf \
                     (master04 §Open Value-Sets)"
                ),
                kind: ValidationKind::CodedValue,
            });
        }
        // Closed-list rejection: resolve the WT node for this path.
        if let Some(node) = find_node_by_segments(wt, &group.segments)
            && node.inputs.iter().any(|i| i.list_open == Some(false))
        {
            out.push(ValidationMessage {
                path: path_str.clone(),
                message: "`|other` is not allowed on a closed value-set (listOpen: false) \
                          (master04 §Open Value-Sets)"
                    .to_owned(),
                kind: ValidationKind::CodedValue,
            });
        }
    }
    out
}

/// One FLAT leaf-path grouping for [`validate_flat_other`]: the parsed segments
/// (for WT resolution) and the datum suffixes present on the path.
struct LeafGroup {
    segments: Vec<path::Segment>,
    suffixes: Vec<String>,
}

/// Validate that the **mandatory context fields** are present on a parsed
/// simplified document (ITS-REST `simplified_formats/master04-basic_concepts.adoc`
/// §Validation: "Mandatory context fields (language, territory) are present";
/// §Context: "Mandatory: language, territory").
///
/// A field counts as present when the `ctx/` node carries it with a non-empty
/// bare value, or — because master05 §COMPOSITION also permits the root-level
/// path spelling (`<root>/language|code`, `<root>/territory|code`) — when a
/// non-`ctx` data root carries a child of that name. Returns one
/// [`ValidationMessage`] (`Required`, keyed `ctx/<field>`) per absent field.
#[must_use]
pub fn validate_context(doc: &SimDocument) -> Vec<ValidationMessage> {
    let mut out = Vec::new();
    let ctx = doc.child("ctx");
    for field in ["language", "territory"] {
        let in_ctx = ctx
            .and_then(|c| c.child(field))
            .and_then(SimNode::bare)
            .is_some_and(is_present);
        // master05 §COMPOSITION permits the root-level path spelling too; a value
        // there satisfies the mandatory-field rule, so it is not falsely flagged.
        let in_root = doc.children.iter().any(|(name, child)| {
            name.as_str() != "ctx"
                && child
                    .occurrences
                    .iter()
                    .any(|occ| occ.child(field).is_some())
        });
        if !in_ctx && !in_root {
            out.push(ValidationMessage {
                path: format!("ctx/{field}"),
                message: format!(
                    "mandatory context field '{field}' is absent (language and territory \
                     are required)"
                ),
                kind: ValidationKind::Required,
            });
        }
    }
    out
}

/// Resolve the [`WebTemplateNode`] a FLAT leaf's parsed `segments` address by
/// descending the WT tree on json-id segments (indices ignored, the leading root
/// id checked). Returns `None` for an unknown path or a `_`-prefixed RM-attribute
/// segment (which addresses no template node).
fn find_node_by_segments<'a>(
    wt: &'a WebTemplate,
    segments: &[path::Segment],
) -> Option<&'a WebTemplateNode> {
    let mut iter = segments.iter();
    // The first segment is the root template id.
    let first = iter.next()?;
    if first.name != wt.tree.id {
        return None;
    }
    let mut node = &wt.tree;
    for seg in iter {
        if seg.is_rm_attribute() {
            return None; // an RM-attribute segment addresses no template node
        }
        node = node
            .children
            .iter()
            .find(|c| c.id == seg.name || c.alt_json_id.as_deref() == Some(seg.name.as_str()))?;
    }
    Some(node)
}

/// Name-based routing for same-archetype-id sibling constraints.
///
/// Maps each identity `(attribute, archetype_node_id)` that the template repeats
/// across more than one distinct sibling path (relative to `parent_aql`) — i.e.
/// the template differentiates same-`archetype_node_id` siblings by their
/// runtime `name` (RM common `master03-archetyped_package.adoc` §"The
/// `LOCATABLE` class" L33-35; AOM 1.4 `master04-constraint_model_package.adoc`
/// §`node_id` L41) — to the set of explicit `name/value` `C_STRING` constraints
/// its *name-qualified* siblings carry.
///
/// A present key marks the identity as name-differentiated, which routing uses
/// two ways ([`select_group_children`]): a name-qualified sibling
/// matches strictly (id + name, no id-only fallback), and the one *unqualified*
/// sibling (whose `name` is unconstrained) admits every same-id instance
/// **except** those bearing a name a name-qualified sibling claims.
pub(crate) type SiblingNameIndex = std::collections::HashMap<(String, String), Vec<String>>;

// ── pre-parsed archetype-conformance walk plan ─────────────────────────────────
//
// The archetype-conformance walk ([`Validator::walk`]) navigates the instance
// guided by the constraint paths a [`WebTemplateNode`] carries
// (`closed_attributes`, `existence`, `card_all`, `slots`, and the children's
// `aqlPath`s). Those paths are **template-static**, so they are parsed once at
// [`crate::flat::webtemplate::build_web_template`] time (via [`prepare_walk`]) and the
// parsed form + sibling groups cached on the node as [`WebTemplateNode::walk`],
// rather than re-parsed on every instance-node visit. A hand-built node that
// never went through the builder has no cached plan; the walk then builds a
// [`NodeWalk`] on the fly once per visit (identical result), so every consumer
// reads exactly one code path.
//
// No openEHR spec governs the WebTemplate model or this plan — our own
// design/extension (the walk *semantics* it serves cite AOM 1.4 / RM common).

/// A pre-parsed archetype-conformance walk plan for one [`WebTemplateNode`].
///
/// Every `Vec` is index-aligned with the corresponding node constraint vector so
/// the walk can zip them; `None` marks a constraint path that is not an
/// extension of the node's own `aqlPath`.
#[derive(Debug, Clone)]
pub(crate) struct NodeWalk {
    /// Child sibling groups, in first-seen order (grouping the node's `children`
    /// by `aqlPath`).
    pub(crate) child_groups: Vec<ChildGroup>,
    /// Name-based sibling routing index over the child groups.
    pub(crate) child_names: SiblingNameIndex,
    /// Parsed relative segments per `existence` constraint (index-aligned with
    /// [`WebTemplateNode::existence`]).
    pub(crate) existence: Vec<Option<Vec<PathSegment>>>,
    /// Parsed relative segments per `card_all` constraint (index-aligned with
    /// [`WebTemplateNode::card_all`]).
    pub(crate) cardinalities: Vec<Option<Vec<PathSegment>>>,
    /// Parsed relative segments per `closed_attributes` constraint (index-aligned
    /// with [`WebTemplateNode::closed_attributes`]).
    pub(crate) closed: Vec<Option<Vec<PathSegment>>>,
    /// Slot groups (grouped by slot path, first-seen order).
    pub(crate) slot_groups: Vec<SlotGroup>,
    /// Name-based sibling routing index over the slot groups.
    pub(crate) slot_names: SiblingNameIndex,
}

/// A sibling group of `WebTemplate` children sharing one `aqlPath` (a single
/// node, or a set of polymorphic type alternatives).
#[derive(Debug, Clone)]
pub(crate) struct ChildGroup {
    /// Indices into [`WebTemplateNode::children`] of the group's members.
    pub(crate) members: Vec<usize>,
    /// The group's `aqlPath` parsed relative to the parent node's `aqlPath`; an
    /// empty vec means the child path is not a strict extension of the parent's
    /// (or equals it) — the walk skips such a group.
    pub(crate) segments: Vec<PathSegment>,
}

/// A group of hoisted-wrapper slot constraints sharing one absolute path.
#[derive(Debug, Clone)]
pub(crate) struct SlotGroup {
    /// The allowed RM types (first-seen order — the `join(", ")` message order).
    pub(crate) allowed: Vec<String>,
    /// The absolute slot path (the violation path the walk pushes).
    pub(crate) path: String,
    /// The slot path parsed relative to the node's `aqlPath` (empty = not an
    /// extension; the walk skips it).
    pub(crate) segments: Vec<PathSegment>,
}

impl NodeWalk {
    /// Build the walk plan for `node` from its (final, post-compaction) constraint
    /// vectors and children. Parses each constraint path once.
    pub(crate) fn build(node: &WebTemplateNode) -> Self {
        let aql = node.aql_path.as_str();

        // Child groups: iterate children in order, grouping by `aqlPath`.
        let mut child_groups: Vec<ChildGroup> = Vec::new();
        let mut child_idx: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for (i, child) in node.children.iter().enumerate() {
            let key = child.aql_path.as_str();
            let next = child_groups.len();
            let gi = *child_idx.entry(key).or_insert(next);
            if gi == next {
                let segments = key.strip_prefix(aql).map(rmpath::parse).unwrap_or_default();
                child_groups.push(ChildGroup {
                    members: Vec::new(),
                    segments,
                });
            }
            child_groups[gi].members.push(i);
        }
        let child_names =
            sibling_index_from_segments(child_groups.iter().map(|g| g.segments.as_slice()));

        // Slot groups: grouped by absolute slot path, first-seen order.
        let mut slot_groups: Vec<SlotGroup> = Vec::new();
        let mut slot_idx: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for slot in &node.slots {
            let key = slot.path.as_str();
            let next = slot_groups.len();
            let gi = *slot_idx.entry(key).or_insert(next);
            if gi == next {
                let segments = key.strip_prefix(aql).map(rmpath::parse).unwrap_or_default();
                slot_groups.push(SlotGroup {
                    allowed: Vec::new(),
                    path: slot.path.clone(),
                    segments,
                });
            }
            slot_groups[gi].allowed.push(slot.rm_type.clone());
        }
        let slot_names =
            sibling_index_from_segments(slot_groups.iter().map(|g| g.segments.as_slice()));

        let rel = |p: &str| p.strip_prefix(aql).map(rmpath::parse);

        Self {
            child_groups,
            child_names,
            existence: node.existence.iter().map(|e| rel(&e.path)).collect(),
            cardinalities: node.card_all.iter().map(|c| rel(&c.path)).collect(),
            closed: node
                .closed_attributes
                .iter()
                .map(|c| rel(&c.path))
                .collect(),
            slot_groups,
            slot_names,
        }
    }
}

/// Build the name-based sibling routing index from a set of already-parsed
/// relative sibling paths (one per distinct child/slot `aqlPath`).
///
/// Per identity `(attribute, archetype_node_id)`, count the distinct sibling
/// paths carrying it and collect the explicit `name/value` constraints; keep only
/// identities carried by more than one sibling. A path with no predicate-bearing
/// segment, or no `archetype_node_id`, is skipped.
fn sibling_index_from_segments<'a>(
    groups: impl Iterator<Item = &'a [PathSegment]>,
) -> SiblingNameIndex {
    let mut acc: std::collections::HashMap<(String, String), (u32, Vec<String>)> =
        std::collections::HashMap::new();
    for segs in groups {
        let Some(id_seg) = segs.iter().rfind(|s| !s.predicate.is_empty()) else {
            continue;
        };
        let Some(id) = &id_seg.predicate.archetype_node_id else {
            continue;
        };
        let entry = acc
            .entry((id_seg.attribute.clone(), id.clone()))
            .or_default();
        entry.0 += 1;
        if let Some(name) = &id_seg.predicate.name_value
            && !entry.1.contains(name)
        {
            entry.1.push(name.clone());
        }
    }
    acc.into_iter()
        .filter(|(_, (count, _))| *count > 1)
        .map(|(k, (_, names))| (k, names))
        .collect()
}

/// Populate the archetype-conformance walk plan on `node` and, recursively, every
/// descendant — called once by [`crate::flat::webtemplate::build_web_template`] on the
/// finalized tree so the validation walk never re-parses a template-static
/// constraint path.
pub(crate) fn prepare_walk(node: &mut WebTemplateNode) {
    let plan = NodeWalk::build(node);
    node.walk = Some(Box::new(plan));
    for child in &mut node.children {
        prepare_walk(child);
    }
}

#[derive(Default)]
struct Validator {
    out: Vec<ValidationMessage>,
    /// When set, existence/occurrences/cardinality **lower** limits are treated
    /// as zero (missing/empty is tolerated), realizing the RM common master06
    /// §"Incomplete Content" relaxation for a `553|incomplete|` commit: "in an
    /// `incomplete` commit, data may be missing, but it may not be wrong … all
    /// existence and cardinality lower limits set to zero". Upper limits, type
    /// conformance, and every leaf/value constraint (`RangeError`,
    /// `PatternError`, `CodedValue`, `WrongType`) are still enforced — only the
    /// "not enough / absent" violations are suppressed.
    relax_lower_bounds: bool,
}

impl Validator {
    fn push(&mut self, path: impl Into<String>, message: impl Into<String>, kind: ValidationKind) {
        self.out.push(ValidationMessage {
            path: path.into(),
            message: message.into(),
            kind,
        });
    }

    // ── Pass 1: RM class invariants over the whole instance ───────────────────

    // `path` is a single reusable buffer pushed/popped per recursion step: a
    // node's running RM instance path is appended before descending and truncated
    // back after, so the full path string is materialized only when a violation
    // is actually recorded — not `format!`-allocated afresh at every one of the
    // ~1.5k nodes an IPS commit visits.
    fn rm_invariant_pass(&mut self, v: &Value, path: &mut String) {
        use std::fmt::Write as _;
        let Some(obj) = v.as_object() else { return };
        // One projection pass over the node's entries collects every field the
        // per-node checks below read, so none of them pays a hashed map lookup
        // (this runs for every node of every commit; the only remaining per-node
        // gets are gated behind a matching `_type`).
        let mut has_type = false;
        let mut ty: Option<&str> = None;
        let mut node_id: Option<&str> = None;
        let mut has_archetype_details = false;
        let mut mappings_empty = false;
        let mut reference_ranges_empty = false;
        for (k, val) in obj {
            match k.as_str() {
                "_type" => {
                    has_type = true;
                    ty = val.as_str();
                }
                "archetype_node_id" => node_id = val.as_str(),
                "archetype_details" => has_archetype_details = !val.is_null(),
                "mappings" => mappings_empty = val.as_array().is_some_and(Vec::is_empty),
                "other_reference_ranges" => {
                    reference_ranges_empty = val.as_array().is_some_and(Vec::is_empty);
                }
                _ => {}
            }
        }
        if has_type {
            // The core (fast/typed) RM invariants only — the terminology-backed
            // invariants are enforced by the dedicated `terminology_pass` (its
            // own `ValidationKind::Terminology` rendering), so calling the
            // core-only entry here avoids double-reporting them.
            let mut inv = Vec::new();
            validate_rm_invariants(v, &mut inv);
            for iv in inv {
                let p = if iv.path.is_empty() {
                    norm_path(path)
                } else {
                    format!("{}/{}", path.trim_end_matches('/'), iv.path)
                };
                self.push(p, iv.message, ValidationKind::Invariant);
            }
        }
        self.check_archetyped_valid(node_id, has_archetype_details, path);
        self.check_nonempty_lists(obj, ty, mappings_empty, reference_ranges_empty, path);
        self.check_data_structure_shapes(obj, ty, path);
        for (k, val) in obj {
            if k.starts_with('_') {
                continue;
            }
            match val {
                Value::Array(a) => {
                    for (i, item) in a.iter().enumerate() {
                        if item.is_object() {
                            let base = path.len();
                            let _ = write!(path, "/{k}[{i}]");
                            self.rm_invariant_pass(item, path);
                            path.truncate(base);
                        }
                    }
                }
                Value::Object(_) => {
                    let base = path.len();
                    let _ = write!(path, "/{k}");
                    self.rm_invariant_pass(val, path);
                    path.truncate(base);
                }
                _ => {}
            }
        }
    }

    /// `LOCATABLE.Archetyped_valid`: `is_archetype_root xor archetype_details =
    /// Void` (`RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc` L60).
    /// The enforceable arm on an instance is: a **non-root** node — one whose
    /// `archetype_node_id` is an `at`/`id` term code, which per the node-id
    /// format can never be the root of an archetyped structure — must NOT carry
    /// `archetype_details`.
    ///
    /// NOTE: the converse arm ("an archetype-HRID node must carry
    /// `archetype_details`") is NOT enforced — the reference object model derives
    /// `is_archetype_root` from `archetype_details` presence (making that reading
    /// tautological), and the CNF's own valid data sets + the canonical-JSON
    /// corpus systematically omit `archetype_details` on nested archetype roots
    /// (182 occurrences measured); the CNF fixtures win over a prose reading that
    /// would reject them (`.claude/rules/spec-adherence.md`). The COMPOSITION root
    /// arm stays separately enforced (`composition_impl.rs` `Is_archetype_root`).
    fn check_archetyped_valid(
        &mut self,
        node_id: Option<&str>,
        has_archetype_details: bool,
        path: &str,
    ) {
        let Some(node_id) = node_id else {
            return;
        };
        let is_term_code = node_id
            .strip_prefix("at")
            .or_else(|| node_id.strip_prefix("id"))
            .is_some_and(|rest| rest.chars().next().is_some_and(|c| c.is_ascii_digit()));
        if is_term_code && has_archetype_details {
            self.push(
                norm_path(path),
                format!(
                    "node {node_id:?} is not an archetype root (at/id term code) and must \
                     not carry archetype_details (LOCATABLE.Archetyped_valid)"
                ),
                ValidationKind::Invariant,
            );
        }
    }

    /// The RM's "present implies non-empty" list invariants, checkable only at
    /// the JSON level (after typed deserialize an absent list and a
    /// present-empty list are both an empty `Vec`):
    ///
    /// - `COMPOSITION.Content_valid`: `content /= Void implies not
    ///   content.is_empty` (`composition.adoc`);
    /// - `EVENT_CONTEXT.Participations_validity` (`event_context.adoc`);
    /// - `SECTION.Items_valid` (`section.adoc`);
    /// - `ENTRY.Other_participations_valid` (`entry.adoc`, every concrete
    ///   ENTRY subtype);
    /// - `INSTRUCTION.Activities_valid` (`instruction.adoc`).
    fn check_nonempty_lists(
        &mut self,
        obj: &serde_json::Map<String, Value>,
        ty: Option<&str>,
        mappings_empty: bool,
        reference_ranges_empty: bool,
        path: &str,
    ) {
        const RULES: &[(&str, &str, &str)] = &[
            ("COMPOSITION", "content", "Content_valid"),
            ("EVENT_CONTEXT", "participations", "Participations_validity"),
            ("SECTION", "items", "Items_valid"),
            (
                "OBSERVATION",
                "other_participations",
                "Other_participations_valid",
            ),
            (
                "EVALUATION",
                "other_participations",
                "Other_participations_valid",
            ),
            (
                "INSTRUCTION",
                "other_participations",
                "Other_participations_valid",
            ),
            (
                "ACTION",
                "other_participations",
                "Other_participations_valid",
            ),
            (
                "ADMIN_ENTRY",
                "other_participations",
                "Other_participations_valid",
            ),
            (
                "GENERIC_ENTRY",
                "other_participations",
                "Other_participations_valid",
            ),
            ("INSTRUCTION", "activities", "Activities_valid"),
        ];
        // Attribute-keyed rules that apply on ANY node carrying the attribute
        // (like the terminology pass's null_flavour handling):
        // `DV_TEXT.Mappings_valid` and `DV_ORDERED.Other_reference_ranges_validity`
        // (`dv_text.adoc` / `dv_ordered.adoc`) — no other RM attribute shares
        // these names.
        for (attr, invariant, is_empty) in [
            ("mappings", "Mappings_valid", mappings_empty),
            (
                "other_reference_ranges",
                "Other_reference_ranges_validity",
                reference_ranges_empty,
            ),
        ] {
            if is_empty {
                self.push(
                    norm_path(path),
                    format!(
                        "{attr} is present but empty — a present list must be \
                         non-empty ({invariant})"
                    ),
                    ValidationKind::Invariant,
                );
            }
        }
        let Some(ty) = ty else {
            return;
        };
        for (rule_ty, attr, invariant) in RULES {
            if *rule_ty == ty
                && obj
                    .get(*attr)
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)
            {
                self.push(
                    norm_path(path),
                    format!(
                        "{ty}.{attr} is present but empty — a present list must be \
                         non-empty ({ty}.{invariant})"
                    ),
                    ValidationKind::Invariant,
                );
            }
        }
    }

    /// JSON-level data-structure shape duties the typed model cannot express:
    ///
    /// - `CLUSTER.items` is 1..1 (RM `data_structures` `cluster.adoc`; the
    ///   ITS-JSON CLUSTER schema lists `items` as required) — after
    ///   deserialize an absent list collapses into an empty `Vec`, so
    ///   presence is only checkable here;
    /// - one `HISTORY`'s events all carry the SAME `ITEM_STRUCTURE` subtype
    ///   in `data` — "A History of type `HISTORY<ITEM_LIST>` … constrains the
    ///   type of the data at each Event to be of type `ITEM_LIST` and nothing
    ///   else" (RM `data_structures` master06; `history.adoc` generic
    ///   parameter) — the monomorphized runtime type cannot see `T`.
    fn check_data_structure_shapes(
        &mut self,
        obj: &serde_json::Map<String, Value>,
        ty: Option<&str>,
        path: &str,
    ) {
        let Some(ty) = ty else {
            return;
        };
        if ty == "CLUSTER" && obj.get("items").and_then(Value::as_array).is_none() {
            self.push(
                norm_path(path),
                "CLUSTER.items is mandatory (1..1 List<ITEM>, cluster.adoc)".to_owned(),
                ValidationKind::Invariant,
            );
        }
        if ty == "HISTORY"
            && let Some(events) = obj.get("events").and_then(Value::as_array)
        {
            let mut first: Option<&str> = None;
            for (i, event) in events.iter().enumerate() {
                let Some(data_ty) = event.pointer("/data/_type").and_then(Value::as_str) else {
                    continue;
                };
                match first {
                    None => first = Some(data_ty),
                    Some(locked) if locked != data_ty => {
                        self.push(
                            norm_path(&format!("{path}/events[{i}]")),
                            format!(
                                "HISTORY events must all carry the same ITEM_STRUCTURE \
                                 subtype in data — the history is HISTORY<{locked}> but \
                                 this event carries {data_ty} (RM data_structures \
                                 master06 §History)"
                            ),
                            ValidationKind::Invariant,
                        );
                    }
                    Some(_) => {}
                }
            }
        }
    }

    // ── Pass 3: WebTemplate archetype-conformance walk ────────────────────────

    /// Visit an instance node matched to a `WebTemplate` node: check type
    /// conformance, leaf domain constraints, then descend into the `WebTemplate`
    /// children and container cardinalities.
    fn walk(&mut self, instance: &Value, wt: &WebTemplateNode) {
        if let Some(inst_type) = instance.get("_type").and_then(Value::as_str)
            && !subtype::conforms(inst_type, &wt.rm_type)
        {
            self.push(
                &wt.aql_path,
                format!(
                    "expected RM type conforming to {} but found {inst_type}",
                    wt.rm_type
                ),
                ValidationKind::WrongType,
            );
            // A wrong-typed node cannot be meaningfully checked further.
            return;
        }

        if !wt.inputs.is_empty() {
            leaf::check_inputs(self, instance, wt);
        }

        // The archetype-conformance walk is driven by the node's pre-parsed
        // [`NodeWalk`]: child sibling groups (polymorphic type choices resolved
        // together, never each flagged), the name-routing index, and the parsed
        // constraint paths, all computed ONCE at build time. A hand-built node
        // with no cached plan builds one on the fly (identical result), so every
        // check reads exactly one code path.
        let built;
        let plan: &NodeWalk = if let Some(p) = &wt.walk {
            p
        } else {
            built = NodeWalk::build(wt);
            &built
        };
        for group in &plan.child_groups {
            self.check_group(instance, wt, group, &plan.child_names);
        }

        self.check_cardinalities(instance, wt, plan);
        self.check_existence(instance, wt, plan);
        self.check_slots(instance, plan);
        self.check_closure(instance, wt, plan);
    }

    /// Closed-archetype walk. Under each constrained
    /// attribute this node records (an attribute with fixed archetype-node
    /// alternatives and/or open `ARCHETYPE_SLOT`s), an instance child bearing an
    /// `archetype_node_id` (i.e. a LOCATABLE — the archetyped-content
    /// discriminator; no RM metadata value carries one) must match a fixed
    /// sibling identity **or** an open slot; any other archetyped child is an
    /// "unexpected node". RM-permitted unconstrained metadata attributes and
    /// wholly-unconstrained attributes are never recorded, so stay open (closed-world capture
    /// rule 2). A rejected node is not descended into (the walk already skips it).
    ///
    /// NOTE: AOM 1.4 `valid_value`
    /// (`AM/docs/AOM1.4/master04-constraint_model_package.adoc` §`Valid_value`
    /// L60-62) is a positive-only cascade, silent on unmatched instance nodes;
    /// closed-world rejection follows the AOM2 direction + de-facto CDR behaviour
    /// and lands only behind the ECC zero-drift gate.
    fn check_closure(&mut self, instance: &Value, wt: &WebTemplateNode, plan: &NodeWalk) {
        for (ca, segs) in wt.closed_attributes.iter().zip(&plan.closed) {
            let Some(segments) = segs else {
                continue;
            };
            let Some((last, intermediate)) = segments.split_last() else {
                continue;
            };
            for container in &rmpath::navigate(&[instance], intermediate) {
                let mut slot_counts = vec![0usize; ca.slots.len()];
                for child in children_under_attr(container, &last.attribute) {
                    let Some(nid) = child
                        .get("archetype_node_id")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                    else {
                        // Not a LOCATABLE (RM metadata / PATHABLE value): not
                        // subject to sibling closure.
                        continue;
                    };
                    if ca.allowed_ids.iter().any(|a| a == nid) {
                        continue; // Matches a fixed sibling alternative.
                    }
                    // NOTE (the closed-world admission rule): an unmatched *archetype-rooted*
                    // child (`openEHR-…` id) is tolerated when the attribute
                    // carries no ARCHETYPE_SLOT constraint — OPT 1.4 flattening
                    // does not enumerate the full slot-fill universe, and the
                    // CNF corpus itself commits ENTRY archetypes the template
                    // does not list. Where slots ARE declared, archetype-rooted
                    // fillers stay subject to slot admission (include/exclude)
                    // below.
                    if ca.slots.is_empty() && nid.starts_with("openEHR-") {
                        continue;
                    }
                    let ct = child.get("_type").and_then(Value::as_str).unwrap_or("");
                    match ca.slots.iter().position(|s| slot_admits(s, ct, nid)) {
                        Some(i) => slot_counts[i] += 1,
                        None => self.push(
                            &ca.path,
                            format!(
                                "unexpected node '{nid}' under '{}': no matching archetype \
                                 constraint or slot",
                                last.attribute
                            ),
                            ValidationKind::Unexpected,
                        ),
                    }
                }
                // Slot occurrences on the matched fillers.
                for (i, slot) in ca.slots.iter().enumerate() {
                    let count = i32::try_from(slot_counts[i]).unwrap_or(i32::MAX);
                    if slot_counts[i] == 0 {
                        if slot.min >= 1 {
                            self.push(
                                &ca.path,
                                format!(
                                    "mandatory archetype slot (occurrences {}..) under '{}' \
                                     has no filler",
                                    slot.min, last.attribute
                                ),
                                ValidationKind::Required,
                            );
                        }
                    } else if slot.max != -1 && count > slot.max {
                        self.push(
                            &ca.path,
                            format!(
                                "too many slot fillers under '{}': found {}, expected at most {}",
                                last.attribute, slot_counts[i], slot.max
                            ),
                            ValidationKind::Occurrences,
                        );
                    }
                }
            }
        }
    }

    /// Type-conformance check for wrapper constraints the compactor hoisted
    /// away (`ITEM_*` / `HISTORY` / single `EVENT` — AOM 1.4 type conformance;
    /// master16 §`ITEM_STRUCTURE`/§EVENT "Class not allowed"): an instance node
    /// matched at a recorded slot path must conform to one of the RM types the
    /// same-path slot alternatives allow. An abstract recorded type
    /// (`ITEM_STRUCTURE`, `EVENT`) admits every concrete subtype via
    /// [`subtype::conforms`].
    fn check_slots(&mut self, instance: &Value, plan: &NodeWalk) {
        for sg in &plan.slot_groups {
            let Some((last, intermediate)) = sg.segments.split_last() else {
                continue;
            };
            let containers = rmpath::navigate(&[instance], intermediate);
            for container in &containers {
                for node in select_group_children(container, last, &plan.slot_names) {
                    let Some(it) = node.get("_type").and_then(Value::as_str) else {
                        continue;
                    };
                    if !sg.allowed.iter().any(|a| subtype::conforms(it, a)) {
                        self.push(
                            &sg.path,
                            format!(
                                "class {it} not allowed: the slot is constrained to [{}]",
                                sg.allowed.join(", ")
                            ),
                            ValidationKind::WrongType,
                        );
                    }
                }
            }
        }
    }

    /// AOM 1.4 `C_ATTRIBUTE.existence` check: for each mandatory plain
    /// RM attribute constrained on this node, verify the attribute *field* is
    /// present on the matched instance. Existence is distinct from occurrences
    /// (archetype-node-identified children) and cardinality (container
    /// membership) — it governs whether the attribute field is there at all — so
    /// this fills the gap for plain structural attributes the occurrence check
    /// deliberately skips. Only the lower bound (mandatory presence) is enforced;
    /// the upper bound is governed by RM single-valuedness / cardinality.
    fn check_existence(&mut self, instance: &Value, wt: &WebTemplateNode, plan: &NodeWalk) {
        // Existence is a lower-bound (mandatory-presence) constraint — relaxed
        // away for a `553|incomplete|` commit (master06 §"Incomplete Content").
        if self.relax_lower_bounds {
            return;
        }
        for (ex, segs) in wt.existence.iter().zip(&plan.existence) {
            if ex.min < 1 {
                continue;
            }
            let Some(segments) = segs else {
                continue;
            };
            let Some((last, intermediate)) = segments.split_last() else {
                continue;
            };
            // Navigate the intermediate segments to the container node(s).
            let containers = rmpath::navigate(&[instance], intermediate);
            for container in &containers {
                if attr_absent(container, &last.attribute) {
                    self.push(
                        ex.path.clone(),
                        format!(
                            "mandatory attribute '{}' is missing (existence {}..)",
                            last.attribute, ex.min
                        ),
                        ValidationKind::Required,
                    );
                }
            }
        }
    }

    /// Resolve a group of `WebTemplate` children sharing an aql path (a single
    /// node, or a set of polymorphic type alternatives) against the instance:
    /// occurrence-check the group, then recurse into each matched instance,
    /// routing it to the conforming alternative.
    fn check_group(
        &mut self,
        parent: &Value,
        wt_parent: &WebTemplateNode,
        group: &ChildGroup,
        names: &SiblingNameIndex,
    ) {
        // Segments were parsed once at build time; an empty slice means the child
        // path is not a strict extension of the parent's (or equals it) — the
        // `strip_prefix … else return` / `segments.is_empty()` early-outs, both
        // preserved here.
        let segments = &group.segments;
        if segments.is_empty() {
            return;
        }
        let members: Vec<&WebTemplateNode> = group
            .members
            .iter()
            .map(|&i| &wt_parent.children[i])
            .collect();
        let first = members[0];
        // The identity segment is the last one carrying a predicate; if none
        // carries one, the last segment (a plain single-valued attribute).
        let identity_idx = segments
            .iter()
            .rposition(|s| !s.predicate.is_empty())
            .unwrap_or(segments.len() - 1);
        // A node whose RM type does NOT inherit `LOCATABLE` carries no
        // `archetype_node_id` in canonical JSON: only `LOCATABLE` adds
        // `archetype_node_id`/`name` (RM common
        // `UML/classes/org.openehr.rm.common.locatable.adoc`), and e.g.
        // `EVENT_CONTEXT` inherits `PATHABLE` directly (RM
        // `UML/classes/org.openehr.rm.composition.event_context.adoc` §Inherit).
        // A template may still archetype such a node (`/context[at0001]`), but no
        // conforming instance can bear that at-code — so the walk matches it
        // STRUCTURALLY by its attribute position (predicate stripped) and never
        // applies archetype-node-id occurrences to it. This is a correction toward
        // the RM inheritance graph, not a relaxation: for a `LOCATABLE` node the
        // node-id match + occurrences apply exactly as before.
        //
        // Only the TERMINAL-identity case consults `first.rm_type`: when a trailing
        // plain-attribute path follows the identity segment (e.g.
        // `items[at0004]/value`), the identity node is the archetyped intermediate
        // (`ELEMENT`/`CLUSTER`/…, always `LOCATABLE`) and `first.rm_type` is the
        // deeper leaf type (`DV_QUANTITY`, a non-`LOCATABLE` `DATA_VALUE`) — which
        // must NOT strip the intermediate's node id.
        let raw_id_seg = &segments[identity_idx];
        let trailing = &segments[identity_idx + 1..];
        let identity_is_locatable = !trailing.is_empty() || is_locatable(&first.rm_type);
        // `ism_transition` is also non-`LOCATABLE` (PATHABLE), but it is left
        // UNMATCHED deliberately — the WebTemplate builder models its careflow
        // steps as separate per-state nodes (a documented builder scope gap, see
        // the occurrences NOTE below), and structurally matching them would check
        // the instance's single ISM_TRANSITION against every per-state constraint.
        // Its presence is an RM invariant, so keeping it unmatched here is sound.
        let structural_match = !identity_is_locatable
            && !raw_id_seg.predicate.is_empty()
            && raw_id_seg.attribute != "ism_transition";
        let structural_id_seg;
        let id_seg: &PathSegment = if structural_match {
            let mut stripped = raw_id_seg.clone();
            stripped.predicate = openehr_rm::paths::Predicate::default();
            structural_id_seg = stripped;
            &structural_id_seg
        } else {
            raw_id_seg
        };

        // Navigate the intermediate segments to the container node(s).
        let containers = rmpath::navigate(&[parent], &segments[..identity_idx]);

        // Occurrences are an *archetype-node* constraint: only checked when the
        // matched node is identified by an archetype-node predicate (at-code /
        // archetype id) on a `LOCATABLE` node that can bear one. Plain RM
        // structural attributes (`context`, `value`, `action_archetype_id`, …) and
        // non-`LOCATABLE` nodes (e.g. `EVENT_CONTEXT`, matched structurally above)
        // are governed by RM cardinality/invariants, not archetype occurrences, so
        // they are not occurrence-checked here.
        //
        // NOTE: `ism_transition` careflow steps are modelled by the
        // WebTemplate builder as separate per-state nodes (careflow synthesis is
        // a documented builder scope gap), yet an ACTION instance carries a
        // single ISM_TRANSITION — occurrence-checking them would spuriously demand
        // every state, so they are skipped (ISM_TRANSITION presence is an RM
        // invariant). `in_context` nodes are supplied structurally.
        let occ_applies = identity_is_locatable
            && id_seg.predicate.archetype_node_id.is_some()
            && id_seg.attribute != "ism_transition"
            && !members.iter().any(|c| c.in_context == Some(true));
        let group_min = members
            .iter()
            .filter_map(|c| c.min)
            .min()
            .unwrap_or(0)
            .max(0);
        let group_max = if members.iter().any(|c| c.max == -1) {
            -1
        } else {
            members.iter().map(|c| c.max).max().unwrap_or(-1)
        };

        for container in &containers {
            let matched = select_group_children(container, id_seg, names);
            if occ_applies {
                self.emit_occurrences(&first.aql_path, group_min, group_max, matched.len());
            }
            for node in matched {
                for target in rmpath::navigate(&[node], trailing) {
                    if members.len() == 1 {
                        self.walk(target, first);
                    } else {
                        self.visit_choice(target, &members);
                    }
                }
            }
        }
    }

    /// Route a matched instance node to the type-choice alternative it conforms
    /// to; if it conforms to none, report a single `WrongType`.
    fn visit_choice(&mut self, target: &Value, group: &[&WebTemplateNode]) {
        match target.get("_type").and_then(Value::as_str) {
            Some(it) => {
                if let Some(alt) = group.iter().find(|c| subtype::conforms(it, &c.rm_type)) {
                    self.walk(target, alt);
                } else {
                    let expected: Vec<&str> = group.iter().map(|c| c.rm_type.as_str()).collect();
                    self.push(
                        &group[0].aql_path,
                        format!(
                            "value type {it} conforms to none of the permitted types [{}]",
                            expected.join(", ")
                        ),
                        ValidationKind::WrongType,
                    );
                }
            }
            // No `_type`: recurse against the first alternative; the RM-invariant
            // pass covers structural issues.
            None => self.walk(target, group[0]),
        }
    }

    /// Emit occurrence violations for a matched-node `count` against `[min, max]`
    /// (`max == -1` is unbounded).
    // NOTE (BASE primitives): occurrence/cardinality evaluation here uses
    // the WebTemplate's flattened `(min, max)` integers (`max = -1` =
    // unbounded). This is behaviorally equivalent to BASE
    // `Multiplicity_interval.has(count)` for OPT 1.4, whose occurrence
    // intervals are always closed integer bounds
    // (org.openehr.base.foundation_types.multiplicity_interval.adoc); the
    // spec-cited reference semantics + truth-table tests live in
    // `openehr-base` `multiplicity_interval_impl.rs` / `cardinality_impl.rs`.
    fn emit_occurrences(&mut self, aql_path: &str, min: i32, max: i32, count: usize) {
        let count_i = i32::try_from(count).unwrap_or(i32::MAX);
        // The lower-bound occurrences checks (absent / too-few) are relaxed away
        // for a `553|incomplete|` commit; the upper bound still applies
        // (master06 §"Incomplete Content": lower limits set to zero).
        if !self.relax_lower_bounds {
            if count == 0 {
                if min >= 1 {
                    self.push(
                        aql_path,
                        format!("mandatory node is missing (occurrences {min}..)"),
                        ValidationKind::Required,
                    );
                }
            } else if count_i < min {
                self.push(
                    aql_path,
                    format!("too few occurrences: found {count}, expected at least {min}"),
                    ValidationKind::Occurrences,
                );
            }
        }
        if max != -1 && count_i > max {
            self.push(
                aql_path,
                format!("too many occurrences: found {count}, expected at most {max}"),
                ValidationKind::Occurrences,
            );
        }
    }

    /// Container-cardinality check: for each constraining cardinality on this
    /// node, count the children under the constrained attribute path and compare
    /// to `min`/`max`. Iterates [`WebTemplateNode::card_all`] — the full AOM 1.4
    /// §cardinality set (`0..1`/`1..1`/`1..*` included), a superset of the
    /// serialized `cardinalities` (which the metadata document filters).
    fn check_cardinalities(&mut self, instance: &Value, wt: &WebTemplateNode, plan: &NodeWalk) {
        for (card, segs) in wt.card_all.iter().zip(&plan.cardinalities) {
            let Some(segments) = segs else {
                continue;
            };
            let Some((last, intermediate)) = segments.split_last() else {
                continue;
            };
            // Navigate all but the last segment to the container, then count the
            // last attribute's children (cardinality is over the whole set).
            //
            // AOM 1.4 §cardinality vs §existence: cardinality constrains the
            // container's membership **when the attribute is present**; whether
            // the attribute may be absent at all is the C_ATTRIBUTE.existence
            // constraint (the vendored Multi_list corpus template pairs
            // `content` cardinality 1..* with existence 0..1, and its valid
            // no-content composition relies on the distinction). An absent (or
            // null) attribute field is therefore not a cardinality violation —
            // a template that requires members expresses it as existence 1..1;
            // the RM list invariants forbid the present-empty `[]` encoding.
            let containers = rmpath::navigate(&[instance], intermediate);
            for container in &containers {
                if matches!(container.get(&last.attribute), None | Some(Value::Null)) {
                    continue;
                }
                let count =
                    i32::try_from(openehr_rm::paths::select_children(container, last).len())
                        .unwrap_or(i32::MAX);
                let min = card.min.unwrap_or(0).max(0);
                // The cardinality lower bound is relaxed away for a
                // `553|incomplete|` commit (master06 §"Incomplete Content": "all
                // existence and cardinality lower limits set to zero"); the upper
                // bound still applies.
                if count < min && !self.relax_lower_bounds {
                    self.push(
                        card.path.clone(),
                        format!("cardinality violated: {count} children, expected at least {min}"),
                        ValidationKind::Cardinality,
                    );
                }
                if card.max != -1 && count > card.max {
                    self.push(
                        card.path.clone(),
                        format!(
                            "cardinality violated: {count} children, expected at most {}",
                            card.max
                        ),
                        ValidationKind::Cardinality,
                    );
                }
            }
        }
    }
}

// ── path navigation ───────────────────────────────────────────────────────────
//
// RM-path parsing (`[atNNNN]` / `[archetype-id]` / `[atNNNN,'name']` predicates)
// and per-step navigation over the canonical-JSON tree are the single
// implementation in [`openehr_rm::paths`], reached here via [`crate::flat::rmpath`]
// (`parse` / `navigate` / `select_children`). Only the checks below —
// attribute-presence for the existence rule and RM instance-path
// normalisation — are validation-specific.

/// Select the instance children a WebTemplate sibling group claims from
/// `container`, applying the name discriminator for same-`archetype_node_id`
/// siblings so an instance is routed to exactly the sibling whose name
/// constraint it satisfies (RM common `master03-archetyped_package.adoc`
/// §"The `LOCATABLE` class": the runtime `name` distinguishes sibling nodes
/// that share an `archetype_node_id`; BASE
/// `architecture_overview/master11-paths.adoc` §"Using a Name-based Predicate").
///
/// `names` is the [`SiblingNameIndex`] computed once per parent from the sibling
/// paths. Three cases on the group's identity segment `id_seg`:
///
/// * **name-qualified, name-differentiated** (`Some` name, identity in `names`):
///   strict `(archetype_node_id, name)` match with the id-only fallback OFF —
///   the sibling set relies on the name, so widening to id-only would claim a
///   sibling's instances.
/// * **unqualified, name-differentiated** (no name, identity in `names`): match
///   by `archetype_node_id` minus the instances the name-qualified siblings
///   claim ([`rmpath::select_children_excluding_names`]) — the residual arm.
/// * **not name-differentiated** (identity absent from `names`): strict match
///   with the id-only fallback ON, tolerating a runtime-renamed instance when
///   the archetype does not constrain the name (master03 §"The `LOCATABLE`
///   class" L35).
fn select_group_children<'a>(
    container: &'a Value,
    id_seg: &openehr_rm::paths::PathSegment,
    names: &SiblingNameIndex,
) -> Vec<&'a Value> {
    let claimed = id_seg
        .predicate
        .archetype_node_id
        .as_ref()
        .and_then(|id| names.get(&(id_seg.attribute.clone(), id.clone())));
    match (id_seg.predicate.name_value.as_ref(), claimed) {
        (Some(_), Some(_)) => rmpath::select_children_matched(container, id_seg, false),
        (None, Some(claimed)) => {
            rmpath::select_children_excluding_names(container, id_seg, claimed)
        }
        (_, None) => rmpath::select_children_matched(container, id_seg, true),
    }
}

/// Whether an RM type inherits `LOCATABLE` and therefore carries an
/// `archetype_node_id` (and `name`) in canonical JSON (RM common
/// `UML/classes/org.openehr.rm.common.locatable.adoc`). Backed by the
/// BMM-generated RM inheritance graph ([`openehr_rm::model::is_a`]). A type the
/// model does not recognise is treated as `LOCATABLE` (the historic default —
/// stay strict rather than silently widen matching for an unknown type). Generic
/// arguments are stripped first.
fn is_locatable(rm_type: &str) -> bool {
    let base = rm_type.split('<').next().unwrap_or(rm_type).trim();
    openehr_rm::model::class(base).is_none() || openehr_rm::model::is_a(base, "LOCATABLE")
}

/// The instance child objects directly under `attr` (array elements or a single
/// object), skipping non-object values — the sibling set for closed-world.
fn children_under_attr<'a>(node: &'a Value, attr: &str) -> Vec<&'a Value> {
    match node.get(attr) {
        Some(Value::Array(a)) => a.iter().filter(|v| v.is_object()).collect(),
        Some(v @ Value::Object(_)) => vec![v],
        _ => Vec::new(),
    }
}

/// Whether an open `ARCHETYPE_SLOT` admits a filler of RM type `child_type` and
/// archetype id `archetype_id` (AOM 1.4 `ARCHETYPE_SLOT`): the type must conform
/// to the slot's `rm_type`, the id must match at least one `includes` regex (an
/// empty `includes` = open to the type), and match no `excludes` regex. A blanket
/// match-all (`.*`) exclude is ignored when `includes` is non-empty — the ADL 1.4
/// closed-slot idiom (AOM 1.4 has no `is_closed`; NOTE: includes then win,
/// matching de-facto CDR behaviour).
fn slot_admits(slot: &WebTemplateArchetypeSlot, child_type: &str, archetype_id: &str) -> bool {
    if !slot.rm_type.is_empty() && !subtype::conforms(child_type, &slot.rm_type) {
        return false;
    }
    let include_ok = slot.includes.is_empty()
        || slot
            .includes
            .iter()
            .any(|p| leaf::matches_pattern(p, archetype_id));
    if !include_ok {
        return false;
    }
    for ex in &slot.excludes {
        if !slot.includes.is_empty() && matches!(ex.trim(), ".*" | ".+") {
            continue; // closed-slot idiom: a specific includes list wins.
        }
        if leaf::matches_pattern(ex, archetype_id) {
            return false;
        }
    }
    true
}

/// Whether an RM attribute field is absent (missing, JSON `null`, or an empty
/// array) on a node — the negation of "the attribute is present" for the
/// existence check.
fn attr_absent(node: &Value, attr: &str) -> bool {
    match node.get(attr) {
        None | Some(Value::Null) => true,
        Some(Value::Array(a)) => a.is_empty(),
        Some(_) => false,
    }
}

/// Normalize an RM instance path (empty → the root `/`).
fn norm_path(p: &str) -> String {
    if p.is_empty() {
        "/".to_owned()
    } else {
        p.to_owned()
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions + the #[ignore] measurement harnesses' report output
mod tests {
    //! Per-rule unit tests for the composition validator, built on hand-shaped
    //! `WebTemplate` nodes + minimal instances (no OPT parsing) so each rule is
    //! exercised in isolation through the private [`Validator`] walk. End-to-end
    //! corpus + public-seam tests live in `tests/validation.rs`.

    use serde_json::{Value, json};

    use super::*;
    use crate::flat::webtemplate::{
        WebTemplateCardinality, WebTemplateClosedAttribute, WebTemplateCodeList,
        WebTemplateCodedValue, WebTemplateExistence, WebTemplateInput, WebTemplateInputType,
        WebTemplateRange, WebTemplateSlot, WebTemplateValidation,
    };

    fn node(rm: &str, path: &str) -> WebTemplateNode {
        WebTemplateNode::new(rm.to_owned(), path.to_owned())
    }

    /// Run only the `WebTemplate` (archetype-conformance) pass for a matched root.
    fn walk_only(instance: &Value, root: &WebTemplateNode) -> Vec<ValidationMessage> {
        let mut v = Validator::default();
        v.walk(instance, root);
        v.out
    }

    fn kinds(msgs: &[ValidationMessage]) -> Vec<ValidationKind> {
        msgs.iter().map(|m| m.kind).collect()
    }

    // ── occurrences ──────────────────────────────────────────────────────────

    #[test]
    fn occurrences_too_few() {
        let mut root = node("COMPOSITION", "");
        let mut sec = node("SECTION", "/content[at0001]");
        sec.min = Some(2);
        sec.max = 5;
        root.children = vec![sec];

        let inst = json!({
            "_type": "COMPOSITION", "archetype_node_id": "x",
            "content": [{"_type": "SECTION", "archetype_node_id": "at0001",
                         "name": {"_type": "DV_TEXT", "value": "s"}}]
        });
        let msgs = walk_only(&inst, &root);
        assert!(
            kinds(&msgs).contains(&ValidationKind::Occurrences),
            "expected Occurrences (too few), got {msgs:?}"
        );
    }

    #[test]
    fn occurrences_too_many() {
        let mut root = node("COMPOSITION", "");
        let mut sec = node("SECTION", "/content[at0001]");
        sec.min = Some(0);
        sec.max = 1;
        root.children = vec![sec];

        let one = json!({"_type": "SECTION", "archetype_node_id": "at0001",
                         "name": {"_type": "DV_TEXT", "value": "s"}});
        let inst = json!({
            "_type": "COMPOSITION", "archetype_node_id": "x",
            "content": [one.clone(), one.clone(), one]
        });
        let msgs = walk_only(&inst, &root);
        assert!(
            msgs.iter()
                .any(|m| m.kind == ValidationKind::Occurrences && m.message.contains("too many")),
            "expected Occurrences (too many), got {msgs:?}"
        );
    }

    #[test]
    fn occurrences_required_missing() {
        let mut root = node("COMPOSITION", "");
        let mut sec = node("SECTION", "/content[at0001]");
        sec.min = Some(1);
        sec.max = 1;
        root.children = vec![sec];

        let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
        let msgs = walk_only(&inst, &root);
        assert!(
            kinds(&msgs).contains(&ValidationKind::Required),
            "expected Required, got {msgs:?}"
        );
    }

    // ── cardinality ────────────────────────────────────────────────────────────

    #[test]
    fn cardinality_violation() {
        let mut root = node("COMPOSITION", "");
        root.card_all = vec![WebTemplateCardinality {
            min: Some(1),
            max: 2,
            ids: None,
            path: "/content".to_owned(),
        }];
        let entry = json!({"_type": "OBSERVATION", "archetype_node_id": "a"});
        let inst = json!({
            "_type": "COMPOSITION", "archetype_node_id": "x",
            "content": [entry.clone(), entry.clone(), entry]
        });
        let msgs = walk_only(&inst, &root);
        assert!(
            kinds(&msgs).contains(&ValidationKind::Cardinality),
            "expected Cardinality (>max), got {msgs:?}"
        );
    }

    // ── incomplete-lifecycle (553) relaxation ────────────────────────────────
    // RM common master06 §"Incomplete Content": existence/occurrences/cardinality
    // lower limits treated as zero; upper limits and value/type constraints stay.

    /// Run the archetype-conformance pass with the `553|incomplete|` relaxation.
    fn walk_incomplete(instance: &Value, root: &WebTemplateNode) -> Vec<ValidationMessage> {
        let mut v = Validator {
            relax_lower_bounds: true,
            ..Validator::default()
        };
        v.walk(instance, root);
        v.out
    }

    #[test]
    fn incomplete_suppresses_required_missing_occurrences() {
        let mut root = node("COMPOSITION", "");
        let mut sec = node("SECTION", "/content[at0001]");
        sec.min = Some(1);
        sec.max = 1;
        root.children = vec![sec];

        let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
        // Strict: a mandatory node is missing.
        assert!(kinds(&walk_only(&inst, &root)).contains(&ValidationKind::Required));
        // Incomplete: the lower bound is zeroed, so nothing is emitted.
        assert!(
            walk_incomplete(&inst, &root).is_empty(),
            "incomplete commit must tolerate a missing mandatory node"
        );
    }

    #[test]
    fn incomplete_suppresses_too_few_but_keeps_too_many() {
        // too few (min 2, one present) → suppressed under incomplete.
        let mut root = node("COMPOSITION", "");
        let mut sec = node("SECTION", "/content[at0001]");
        sec.min = Some(2);
        sec.max = 3;
        root.children = vec![sec];
        let one = json!({"_type": "SECTION", "archetype_node_id": "at0001",
                         "name": {"_type": "DV_TEXT", "value": "s"}});
        let too_few = json!({
            "_type": "COMPOSITION", "archetype_node_id": "x", "content": [one.clone()]
        });
        assert!(
            walk_incomplete(&too_few, &root).is_empty(),
            "incomplete commit must tolerate too-few occurrences"
        );

        // too many (three present, max 3 ok; four exceeds) → still enforced (upper
        // bound is not relaxed: missing is tolerated, wrong is not).
        let too_many = json!({
            "_type": "COMPOSITION", "archetype_node_id": "x",
            "content": [one.clone(), one.clone(), one.clone(), one]
        });
        assert!(
            walk_incomplete(&too_many, &root)
                .iter()
                .any(|m| m.kind == ValidationKind::Occurrences && m.message.contains("too many")),
            "incomplete commit must still reject too-many occurrences"
        );
    }

    #[test]
    fn incomplete_suppresses_cardinality_lower_but_keeps_upper() {
        // Lower-bound cardinality violation → suppressed.
        let mut low = node("COMPOSITION", "");
        low.card_all = vec![WebTemplateCardinality {
            min: Some(2),
            max: -1,
            ids: None,
            path: "/content".to_owned(),
        }];
        let entry = json!({"_type": "OBSERVATION", "archetype_node_id": "a"});
        let one_child = json!({
            "_type": "COMPOSITION", "archetype_node_id": "x", "content": [entry.clone()]
        });
        assert!(
            walk_incomplete(&one_child, &low).is_empty(),
            "incomplete commit must tolerate a below-minimum container"
        );

        // Upper-bound cardinality violation → still enforced.
        let mut high = node("COMPOSITION", "");
        high.card_all = vec![WebTemplateCardinality {
            min: Some(1),
            max: 2,
            ids: None,
            path: "/content".to_owned(),
        }];
        let three = json!({
            "_type": "COMPOSITION", "archetype_node_id": "x",
            "content": [entry.clone(), entry.clone(), entry]
        });
        assert!(
            kinds(&walk_incomplete(&three, &high)).contains(&ValidationKind::Cardinality),
            "incomplete commit must still reject an over-maximum container"
        );
    }

    #[test]
    fn incomplete_suppresses_existence() {
        let mut root = node("COMPOSITION", "");
        root.existence = vec![WebTemplateExistence {
            min: 1,
            max: 1,
            path: "/context".to_owned(),
        }];
        // The mandatory `context` field is absent.
        let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x"});
        assert!(kinds(&walk_only(&inst, &root)).contains(&ValidationKind::Required));
        assert!(
            walk_incomplete(&inst, &root).is_empty(),
            "incomplete commit must tolerate a missing mandatory attribute (existence)"
        );
    }

    // ── numeric range ────────────────────────────────────────────────────────

    fn count_node_range(min: i64, max: i64) -> WebTemplateNode {
        let mut n = node("DV_COUNT", "/count");
        let mut input = WebTemplateInput::new(WebTemplateInputType::Integer, None);
        input.validation = Some(WebTemplateValidation {
            range: Some(WebTemplateRange {
                min_op: Some(">=".to_owned()),
                min: Some(Value::from(min)),
                max_op: Some("<=".to_owned()),
                max: Some(Value::from(max)),
            }),
            ..Default::default()
        });
        n.inputs = vec![input];
        n
    }

    #[test]
    fn range_error_out_of_bounds() {
        let n = count_node_range(0, 10);
        let inst = json!({"_type": "DV_COUNT", "magnitude": 42});
        let msgs = walk_only(&inst, &n);
        assert_eq!(kinds(&msgs), vec![ValidationKind::RangeError], "{msgs:?}");
    }

    #[test]
    fn range_ok_in_bounds() {
        let n = count_node_range(0, 10);
        let inst = json!({"_type": "DV_COUNT", "magnitude": 5});
        assert!(walk_only(&inst, &n).is_empty());
    }

    // ── DV_MULTIMEDIA.size (C_INTEGER list / range) ──────────────────────────
    // RM `data_types` §DV_MULTIMEDIA (`size: Integer`); AOM 1.4
    // `master04-constraint_model_package.adoc` §C_INTEGER (list + range).

    #[test]
    fn multimedia_size_out_of_range_reported() {
        let mut n = node("DV_MULTIMEDIA", "/media");
        n.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Text, None)];
        n.numeric_ranges = vec![(
            "size".to_owned(),
            WebTemplateRange {
                min_op: Some(">=".to_owned()),
                min: Some(Value::from(200)),
                max_op: Some("<=".to_owned()),
                max: Some(Value::from(1000)),
            },
        )];
        let bad = json!({"_type": "DV_MULTIMEDIA", "uri": "http://x", "size": 123});
        assert_eq!(
            kinds(&walk_only(&bad, &n)),
            vec![ValidationKind::RangeError]
        );
        let good = json!({"_type": "DV_MULTIMEDIA", "uri": "http://x", "size": 500});
        assert!(walk_only(&good, &n).is_empty());
    }

    #[test]
    fn multimedia_size_not_in_list_reported() {
        let mut n = node("DV_MULTIMEDIA", "/media");
        n.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Text, None)];
        n.numeric_lists = vec![("size".to_owned(), vec![10.0, 100.0, 1000.0])];
        let bad = json!({"_type": "DV_MULTIMEDIA", "uri": "http://x", "size": 123});
        assert_eq!(
            kinds(&walk_only(&bad, &n)),
            vec![ValidationKind::CodedValue]
        );
        let good = json!({"_type": "DV_MULTIMEDIA", "uri": "http://x", "size": 100});
        assert!(walk_only(&good, &n).is_empty());
    }

    // ── DV_IDENTIFIER mandatory (existence 1..1) constrained sub-attribute ────
    // RM `data_types` §DV_IDENTIFIER; AOM 1.4 §existence + §C_STRING. An OPT that
    // constrains and mandates `issuer` rejects a value that omits it (the `id`
    // absence is caught separately by the RM invariant).

    #[test]
    fn dv_identifier_mandatory_issuer_enforced() {
        let mut n = node("DV_IDENTIFIER", "/value");
        let mut issuer = WebTemplateInput::new(WebTemplateInputType::Text, Some("issuer"));
        issuer.list = vec![WebTemplateCodedValue::new("XYZ", Some("XYZ".to_owned()))];
        issuer.list_open = Some(false);
        n.inputs = vec![
            WebTemplateInput::new(WebTemplateInputType::Text, Some("id")),
            issuer,
        ];
        n.existence = vec![WebTemplateExistence {
            min: 1,
            max: 1,
            path: "/value/issuer".to_owned(),
        }];
        // issuer absent → Required.
        let absent = json!({"_type": "DV_IDENTIFIER", "id": "x"});
        assert!(kinds(&walk_only(&absent, &n)).contains(&ValidationKind::Required));
        // issuer present but not in the list → CodedValue.
        let wrong = json!({"_type": "DV_IDENTIFIER", "id": "x", "issuer": "ABC"});
        assert!(kinds(&walk_only(&wrong, &n)).contains(&ValidationKind::CodedValue));
        // issuer present and conforming → clean.
        let ok = json!({"_type": "DV_IDENTIFIER", "id": "x", "issuer": "XYZ"});
        assert!(walk_only(&ok, &n).is_empty());
    }

    // ── DV_SCALE generic (C_REAL value list + symbol C_CODE_PHRASE) ───────────
    // AOM 1.4 has no C_DV_SCALE, so DV_SCALE constrains `symbol.defining_code`
    // as a C_CODE_PHRASE code_list (AOM 1.4 §C_CODE_PHRASE; RM §DV_SCALE): a
    // symbol not in the list is rejected, with no (symbol, value) pair check.

    #[test]
    fn dv_scale_generic_symbol_membership_enforced() {
        let mut n = node("DV_SCALE", "/value");
        let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, None);
        // Generic form: coded symbols carry no scale/ordinal number.
        input.list = vec![
            WebTemplateCodedValue::new("at0005", Some("mild".to_owned())),
            WebTemplateCodedValue::new("at0006", Some("severe".to_owned())),
        ];
        n.inputs = vec![input];
        n.numeric_lists = vec![("value".to_owned(), vec![1.5, 2.4])];
        // Symbol not in the code list (value in the real list) → CodedValue.
        let bad = json!({
            "_type": "DV_SCALE", "value": 1.5,
            "symbol": {"_type": "DV_CODED_TEXT", "value": "?",
                "defining_code": {"_type": "CODE_PHRASE",
                    "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
                    "code_string": "at0666"}}
        });
        assert!(kinds(&walk_only(&bad, &n)).contains(&ValidationKind::CodedValue));
        // Symbol in the list, value in the list → clean (no spurious pair check).
        let ok = json!({
            "_type": "DV_SCALE", "value": 1.5,
            "symbol": {"_type": "DV_CODED_TEXT", "value": "mild",
                "defining_code": {"_type": "CODE_PHRASE",
                    "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
                    "code_string": "at0005"}}
        });
        assert!(walk_only(&ok, &n).is_empty(), "{:?}", walk_only(&ok, &n));
    }

    // ── DV_TIME partial value vs C_TIME pattern (over-rejection fix) ──────────
    // ADL 1.4 `master05-cadl.adoc` §"Date, Time and Date/Time" Patterns: `?` =
    // optional field, `X` = disallowed field; an hour-only value "10" satisfies
    // both "HH:??:??" and "HH:XX:XX".

    fn time_node(pattern: &str) -> WebTemplateNode {
        let mut n = node("DV_TIME", "/time");
        let mut input = WebTemplateInput::new(WebTemplateInputType::Time, None);
        input.validation = Some(WebTemplateValidation {
            pattern: Some(pattern.to_owned()),
            ..Default::default()
        });
        n.inputs = vec![input];
        n
    }

    #[test]
    fn dv_time_hour_only_accepts_optional_and_prohibited_patterns() {
        for pattern in ["HH:??:??", "HH:XX:XX"] {
            let n = time_node(pattern);
            let inst = json!({"_type": "DV_TIME", "value": "10"});
            assert!(
                walk_only(&inst, &n).is_empty(),
                "hour-only \"10\" must satisfy pattern {pattern}: {:?}",
                walk_only(&inst, &n)
            );
        }
    }

    // ── string pattern ─────────────────────────────────────────────────────────

    #[test]
    fn pattern_error() {
        let mut n = node("DV_TEXT", "/text");
        let mut input = WebTemplateInput::new(WebTemplateInputType::Text, None);
        input.validation = Some(WebTemplateValidation {
            pattern: Some("[A-Z]+".to_owned()),
            ..Default::default()
        });
        n.inputs = vec![input];

        let bad = json!({"_type": "DV_TEXT", "value": "abc"});
        assert_eq!(
            kinds(&walk_only(&bad, &n)),
            vec![ValidationKind::PatternError]
        );
        let good = json!({"_type": "DV_TEXT", "value": "ABC"});
        assert!(walk_only(&good, &n).is_empty());
    }

    // ── coded value membership ─────────────────────────────────────────────────

    fn coded_node(codes: &[&str]) -> WebTemplateNode {
        let mut n = node("DV_CODED_TEXT", "/coded");
        let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, Some("code"));
        input.list = codes
            .iter()
            .map(|c| WebTemplateCodedValue::new(*c, Some((*c).to_owned())))
            .collect();
        n.inputs = vec![input];
        n
    }

    #[test]
    fn coded_value_not_in_list() {
        let n = coded_node(&["at0001", "at0002"]);
        let bad = json!({
            "_type": "DV_CODED_TEXT", "value": "x",
            "defining_code": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
                "code_string": "at0099"}
        });
        assert_eq!(
            kinds(&walk_only(&bad, &n)),
            vec![ValidationKind::CodedValue]
        );

        let good = json!({
            "_type": "DV_CODED_TEXT", "value": "x",
            "defining_code": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
                "code_string": "at0001"}
        });
        assert!(walk_only(&good, &n).is_empty());
    }

    #[test]
    fn coded_value_external_terminology_is_skipped() {
        // A SNOMED code is not validated against the archetype's internal list.
        let n = coded_node(&["at0001"]);
        let external = json!({
            "_type": "DV_CODED_TEXT", "value": "x",
            "defining_code": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "SNOMED-CT"},
                "code_string": "999"}
        });
        assert!(walk_only(&external, &n).is_empty());
    }

    // ── type conformance ───────────────────────────────────────────────────────

    #[test]
    fn wrong_type_reported() {
        let n = node("DV_QUANTITY", "/q");
        let inst = json!({"_type": "DV_TEXT", "value": "x"});
        assert_eq!(
            kinds(&walk_only(&inst, &n)),
            vec![ValidationKind::WrongType]
        );
    }

    #[test]
    fn coded_text_in_text_slot_conforms() {
        // DV_CODED_TEXT is-a DV_TEXT: no WrongType.
        let n = node("DV_TEXT", "/t");
        let inst = json!({
            "_type": "DV_CODED_TEXT", "value": "x",
            "defining_code": {"_type": "CODE_PHRASE",
                "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"},
                "code_string": "at0001"}
        });
        assert!(
            !kinds(&walk_only(&inst, &n)).contains(&ValidationKind::WrongType),
            "DV_CODED_TEXT should conform to a DV_TEXT slot"
        );
    }

    // ── AOM 1.4 C_ATTRIBUTE.existence ────────────────────────────────────────

    #[test]
    fn existence_mandatory_attribute_missing_reported() {
        // A node requiring a mandatory `value` attribute (existence {1..1}); the
        // instance omits it → Required.
        let mut n = node("ELEMENT", "/items[at0001]");
        n.existence = vec![WebTemplateExistence {
            min: 1,
            max: 1,
            path: "/items[at0001]/value".to_owned(),
        }];
        let inst = json!({"_type": "ELEMENT", "archetype_node_id": "at0001",
            "name": {"_type": "DV_TEXT", "value": "e"}});
        let msgs = walk_only(&inst, &n);
        assert!(
            msgs.iter()
                .any(|m| m.kind == ValidationKind::Required && m.path.ends_with("/value")),
            "expected a Required existence violation for the missing value, got {msgs:?}"
        );
    }

    #[test]
    fn existence_present_attribute_is_clean() {
        let mut n = node("ELEMENT", "/items[at0001]");
        n.existence = vec![WebTemplateExistence {
            min: 1,
            max: 1,
            path: "/items[at0001]/value".to_owned(),
        }];
        let inst = json!({"_type": "ELEMENT", "archetype_node_id": "at0001",
            "name": {"_type": "DV_TEXT", "value": "e"},
            "value": {"_type": "DV_TEXT", "value": "v"}});
        assert!(
            walk_only(&inst, &n).is_empty(),
            "a present mandatory value should be clean"
        );
    }

    #[test]
    fn existence_empty_array_counts_as_absent() {
        let mut n = node("COMPOSITION", "");
        n.existence = vec![WebTemplateExistence {
            min: 1,
            max: -1,
            path: "/content".to_owned(),
        }];
        let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
        let msgs = walk_only(&inst, &n);
        assert!(
            msgs.iter().any(|m| m.kind == ValidationKind::Required),
            "an empty mandatory container attribute is absent, got {msgs:?}"
        );
    }

    // ── path parsing ─────────────────────────────────────────────────────────

    #[test]
    fn segment_parsing_respects_brackets() {
        // Parsing routes through the single `openehr_rm::paths` implementation
        // via `crate::flat::rmpath`; this asserts the validator sees the same segments.
        let segs = crate::flat::rmpath::parse(
            "/content[openEHR-EHR-SECTION.x.v1]/items[at0004,'Sys']/value",
        );
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].attribute, "content");
        assert_eq!(
            segs[0].predicate.archetype_node_id.as_deref(),
            Some("openEHR-EHR-SECTION.x.v1")
        );
        assert_eq!(segs[0].predicate.name_value, None);
        assert_eq!(
            segs[1].predicate.archetype_node_id.as_deref(),
            Some("at0004")
        );
        assert_eq!(segs[1].predicate.name_value.as_deref(), Some("Sys"));
        assert!(segs[2].predicate.is_empty());
    }

    // ── CNF-hardening additions (master15/16/17 truth tables) ─────────────────

    /// `1..*` container cardinality with zero members → Cardinality (master15
    /// CONT-COMP-content_card_1plus; AOM 1.4 §cardinality).
    #[test]
    fn cardinality_one_plus_empty_rejected() {
        let mut root = node("COMPOSITION", "");
        root.card_all = vec![WebTemplateCardinality {
            min: Some(1),
            max: -1,
            ids: None,
            path: "/content".to_owned(),
        }];
        let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
        let msgs = walk_only(&inst, &root);
        assert!(
            kinds(&msgs).contains(&ValidationKind::Cardinality),
            "expected Cardinality for 1..* with 0 members, got {msgs:?}"
        );
    }

    /// A bare mandatory attribute (existence `1..1`, no value constraint) must be
    /// present (master15 `context_mand`; AOM 1.4 §existence).
    #[test]
    fn bare_mandatory_attribute_absent_rejected() {
        let mut root = node("COMPOSITION", "");
        root.existence = vec![WebTemplateExistence {
            min: 1,
            max: 1,
            path: "/context".to_owned(),
        }];
        let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x"});
        let msgs = walk_only(&inst, &root);
        assert!(
            kinds(&msgs).contains(&ValidationKind::Required),
            "expected Required for absent mandatory context, got {msgs:?}"
        );
    }

    /// A hoisted-wrapper slot narrowed to a concrete subtype rejects a sibling
    /// subtype and accepts the narrowed one; an abstract slot accepts any subtype
    /// (master16 §`ITEM_STRUCTURE/§EVENT` "Class not allowed").
    #[test]
    fn slot_narrowing() {
        let mut eval = node("EVALUATION", "/content[at0001]");
        eval.slots = vec![WebTemplateSlot {
            path: "/content[at0001]/data[at0002]".to_owned(),
            rm_type: "ITEM_LIST".to_owned(),
        }];
        let wrong = json!({"_type": "EVALUATION", "archetype_node_id": "at0001",
            "data": {"_type": "ITEM_TREE", "archetype_node_id": "at0002", "items": []}});
        let msgs = walk_only(&wrong, &eval);
        assert!(
            msgs.iter()
                .any(|m| m.kind == ValidationKind::WrongType && m.message.contains("not allowed")),
            "expected WrongType for ITEM_TREE in an ITEM_LIST slot, got {msgs:?}"
        );
        let right = json!({"_type": "EVALUATION", "archetype_node_id": "at0001",
            "data": {"_type": "ITEM_LIST", "archetype_node_id": "at0002", "items": []}});
        assert!(
            walk_only(&right, &eval).is_empty(),
            "narrowed subtype accepted"
        );

        eval.slots[0].rm_type = "ITEM_STRUCTURE".to_owned();
        let any = json!({"_type": "EVALUATION", "archetype_node_id": "at0001",
            "data": {"_type": "ITEM_TABLE", "archetype_node_id": "at0002", "rows": []}});
        assert!(
            walk_only(&any, &eval).is_empty(),
            "abstract ITEM_STRUCTURE slot admits any subtype"
        );
    }

    /// `C_INTEGER.list` on `DV_COUNT.magnitude` (master17.3 CONT-DV_COUNT-validate_list).
    #[test]
    fn numeric_list_membership() {
        let mut count = node("DV_COUNT", "/value");
        count.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Integer, None)];
        count.numeric_lists = vec![("magnitude".to_owned(), vec![3.0])];
        let bad = json!({"_type": "DV_COUNT", "magnitude": 7});
        let msgs = walk_only(&bad, &count);
        assert!(
            kinds(&msgs).contains(&ValidationKind::CodedValue),
            "expected CodedValue for magnitude off the list, got {msgs:?}"
        );
        let good = json!({"_type": "DV_COUNT", "magnitude": 3});
        assert!(walk_only(&good, &count).is_empty());
    }

    /// `DV_PROPORTION` `type` kind membership (master17.3 CONT-DV_PROPORTION-*).
    #[test]
    fn proportion_kind_membership() {
        let mut prop = node("DV_PROPORTION", "/value");
        prop.inputs = vec![WebTemplateInput::new(
            WebTemplateInputType::Decimal,
            Some("numerator"),
        )];
        prop.proportion_types = vec!["percent".to_owned()];
        let bad =
            json!({"_type": "DV_PROPORTION", "numerator": 1.0, "denominator": 2.0, "type": 0});
        let msgs = walk_only(&bad, &prop);
        assert!(
            kinds(&msgs).contains(&ValidationKind::CodedValue),
            "expected CodedValue for ratio where percent required, got {msgs:?}"
        );
        let good =
            json!({"_type": "DV_PROPORTION", "numerator": 42.0, "denominator": 100.0, "type": 2});
        assert!(walk_only(&good, &prop).is_empty());
    }

    /// `C_DATE` pattern + range (master17.4 CONT-DV_DATE-validate_constraint/-range).
    #[test]
    fn temporal_pattern_and_range() {
        let mut date = node("DV_DATE", "/value");
        let mut input = WebTemplateInput::new(WebTemplateInputType::Date, None);
        input.validation = Some(WebTemplateValidation {
            pattern: Some("yyyy-mm-dd".to_owned()),
            range: Some(WebTemplateRange {
                min_op: Some(">=".to_owned()),
                min: Some(json!("2021-01-01")),
                max_op: Some("<=".to_owned()),
                max: Some(json!("2021-12-31")),
            }),
            precision: None,
        });
        date.inputs = vec![input];

        let partial = json!({"_type": "DV_DATE", "value": "2021"});
        assert!(
            kinds(&walk_only(&partial, &date)).contains(&ValidationKind::PatternError),
            "partial date violates yyyy-mm-dd"
        );
        let out = json!({"_type": "DV_DATE", "value": "2025-06-01"});
        assert!(
            kinds(&walk_only(&out, &date)).contains(&ValidationKind::RangeError),
            "date outside the range"
        );
        let ok = json!({"_type": "DV_DATE", "value": "2021-10-18"});
        assert!(walk_only(&ok, &date).is_empty());
    }

    /// `C_TIME` pattern: a partial time violates HH:MM:SS (master17.4 CONT-DV_TIME).
    #[test]
    fn time_pattern_partial_rejected() {
        let mut time = node("DV_TIME", "/value");
        let mut input = WebTemplateInput::new(WebTemplateInputType::Time, None);
        input.validation = Some(WebTemplateValidation {
            pattern: Some("HH:MM:SS".to_owned()),
            range: None,
            precision: None,
        });
        time.inputs = vec![input];
        assert!(
            kinds(&walk_only(
                &json!({"_type": "DV_TIME", "value": "22"}),
                &time
            ))
            .contains(&ValidationKind::PatternError)
        );
        assert!(walk_only(&json!({"_type": "DV_TIME", "value": "22:18:16"}), &time).is_empty());
    }

    /// `C_DURATION` allowed fields + range (master17.4 CONT-DV_DURATION-*).
    #[test]
    fn duration_fields_and_range() {
        let mut dur = node("DV_DURATION", "/value");
        dur.inputs = vec![
            WebTemplateInput::new(WebTemplateInputType::Integer, Some("hour")),
            WebTemplateInput::new(WebTemplateInputType::Integer, Some("minute")),
        ];
        dur.duration_range = Some(WebTemplateRange {
            min_op: Some(">=".to_owned()),
            min: Some(json!("PT0S")),
            max_op: Some("<=".to_owned()),
            max: Some(json!("PT1H")),
        });
        assert!(
            kinds(&walk_only(
                &json!({"_type": "DV_DURATION", "value": "P1Y"}),
                &dur
            ))
            .contains(&ValidationKind::PatternError),
            "year field forbidden by the pattern"
        );
        assert!(
            kinds(&walk_only(
                &json!({"_type": "DV_DURATION", "value": "PT5H"}),
                &dur
            ))
            .contains(&ValidationKind::RangeError),
            "PT5H outside [PT0S,PT1H]"
        );
        assert!(walk_only(&json!({"_type": "DV_DURATION", "value": "PT30M"}), &dur).is_empty());
    }

    /// An enumerated **external** `C_CODE_PHRASE` list constrains membership
    /// (master17.2 CONT-DV_CODED_TEXT-validate_ext_term; AOM 1.4 §`C_CODE_PHRASE`).
    #[test]
    fn external_code_list_membership() {
        let mut coded = node("DV_CODED_TEXT", "/value");
        let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, Some("code"));
        input.terminology = Some("SNOMED-CT".to_owned());
        input.list = vec![WebTemplateCodedValue::new("73211009", None)];
        coded.inputs = vec![input];

        let bad = json!({"_type": "DV_CODED_TEXT", "value": "x", "defining_code": {
            "terminology_id": {"value": "SNOMED-CT"}, "code_string": "99999999"}});
        assert!(
            kinds(&walk_only(&bad, &coded)).contains(&ValidationKind::CodedValue),
            "external code off the enumerated list"
        );
        let good = json!({"_type": "DV_CODED_TEXT", "value": "x", "defining_code": {
            "terminology_id": {"value": "SNOMED-CT"}, "code_string": "73211009"}});
        assert!(walk_only(&good, &coded).is_empty());
        // A code from a different terminology than the constraint's is not judged.
        let other = json!({"_type": "DV_CODED_TEXT", "value": "x", "defining_code": {
            "terminology_id": {"value": "ICD10"}, "code_string": "A00"}});
        assert!(walk_only(&other, &coded).is_empty());
    }

    /// `C_CODE_PHRASE` on a coded attribute outside `defining_code`
    /// (`DV_MULTIMEDIA.media_type` — master17.6 CONT-DV_MULTIMEDIA-validate_media_type).
    #[test]
    fn media_type_code_list() {
        let mut mm = node("DV_MULTIMEDIA", "/value");
        mm.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Text, None)];
        mm.code_lists = vec![WebTemplateCodeList {
            attr: "media_type".to_owned(),
            terminology: Some("IANA_media-types".to_owned()),
            codes: vec!["image/png".to_owned()],
        }];
        let bad = json!({"_type": "DV_MULTIMEDIA", "size": 1, "media_type": {
            "terminology_id": {"value": "IANA_media-types"}, "code_string": "image/gif"}});
        assert!(
            kinds(&walk_only(&bad, &mm)).contains(&ValidationKind::CodedValue),
            "media_type off the enumerated list"
        );
        let good = json!({"_type": "DV_MULTIMEDIA", "size": 1, "media_type": {
            "terminology_id": {"value": "IANA_media-types"}, "code_string": "image/png"}});
        assert!(walk_only(&good, &mm).is_empty());
    }

    // ── closed-archetype walk ─────────────────────────────────────────────────

    /// A COMPOSITION whose `content` is closed to `openEHR-EHR-SECTION.x.v1`: the
    /// defined section is accepted, a foreign OBSERVATION is rejected as unexpected
    /// (closed-world rule 1).
    #[test]
    fn closed_world_rejects_foreign_content() {
        let mut root = node("COMPOSITION", "");
        root.closed_attributes = vec![WebTemplateClosedAttribute {
            path: "/content".to_owned(),
            allowed_ids: vec!["openEHR-EHR-SECTION.x.v1".to_owned()],
            slots: vec![],
        }];
        let inst = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
            {"_type": "SECTION", "archetype_node_id": "openEHR-EHR-SECTION.x.v1",
             "name": {"_type": "DV_TEXT", "value": "s"}},
            {"_type": "OBSERVATION", "archetype_node_id": "openEHR-EHR-OBSERVATION.foreign.v1",
             "name": {"_type": "DV_TEXT", "value": "o"}}
        ]});
        // The closed-world admission rule: an unmatched *archetype-rooted* child is
        // tolerated (the flat OPT does not enumerate the full slot-fill universe;
        // the CNF corpus itself commits such ENTRYs).
        let msgs = walk_only(&inst, &root);
        assert!(
            msgs.is_empty(),
            "foreign archetype-rooted content is tolerated (the closed-world admission rule), got {msgs:?}"
        );
        // At-coded children remain closed: an at-coded child matching no sibling
        // constraint is rejected (closed-world rule 1).
        let at_foreign = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
            {"_type": "SECTION", "archetype_node_id": "at0099",
             "name": {"_type": "DV_TEXT", "value": "s"}}
        ]});
        let msgs = walk_only(&at_foreign, &root);
        assert!(
            msgs.iter()
                .any(|m| m.kind == ValidationKind::Unexpected && m.message.contains("at0099")),
            "expected an Unexpected violation for the foreign at-coded child, got {msgs:?}"
        );
        let ok = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
            {"_type": "SECTION", "archetype_node_id": "openEHR-EHR-SECTION.x.v1",
             "name": {"_type": "DV_TEXT", "value": "s"}}]});
        assert!(
            walk_only(&ok, &root).is_empty(),
            "the defined section is admitted"
        );
    }

    /// A metadata value (no `archetype_node_id`, i.e. non-LOCATABLE) under a closed
    /// attribute is never flagged (closed-world rule 2 — the `archetype_node_id`
    /// discriminator).
    #[test]
    fn closed_world_ignores_metadata_values() {
        let mut root = node("ELEMENT", "/items[at0001]");
        root.closed_attributes = vec![WebTemplateClosedAttribute {
            path: "/items[at0001]/value".to_owned(),
            allowed_ids: vec!["at9999".to_owned()],
            slots: vec![],
        }];
        let inst = json!({"_type": "ELEMENT", "archetype_node_id": "at0001",
            "name": {"_type": "DV_TEXT", "value": "e"},
            "value": {"_type": "DV_QUANTITY", "magnitude": 1.0, "units": "kg"}});
        assert!(
            walk_only(&inst, &root).is_empty(),
            "a DATA_VALUE with no archetype_node_id must not be flagged by closure"
        );
    }

    // ── ARCHETYPE_SLOT enforcement ────────────────────────────────────────────

    fn obs_slot(includes: &[&str], excludes: &[&str], min: i32, max: i32) -> WebTemplateNode {
        let mut root = node("COMPOSITION", "");
        root.closed_attributes = vec![WebTemplateClosedAttribute {
            path: "/content".to_owned(),
            allowed_ids: vec![],
            slots: vec![WebTemplateArchetypeSlot {
                rm_type: "OBSERVATION".to_owned(),
                min,
                max,
                includes: includes.iter().map(|s| (*s).to_owned()).collect(),
                excludes: excludes.iter().map(|s| (*s).to_owned()).collect(),
            }],
        }];
        root
    }

    fn content_obs(archetype_id: &str, rm_type: &str) -> Value {
        json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
            {"_type": rm_type, "archetype_node_id": archetype_id,
             "name": {"_type": "DV_TEXT", "value": "o"}}]})
    }

    #[test]
    fn slot_admits_include_rejects_others() {
        let root = obs_slot(&[r"openEHR-EHR-OBSERVATION\..*"], &[], 0, -1);
        assert!(
            walk_only(
                &content_obs("openEHR-EHR-OBSERVATION.bp.v1", "OBSERVATION"),
                &root
            )
            .is_empty(),
            "an include-matching filler is admitted"
        );
        let msgs = walk_only(
            &content_obs("openEHR-EHR-EVALUATION.x.v1", "EVALUATION"),
            &root,
        );
        assert!(
            msgs.iter().any(|m| m.kind == ValidationKind::Unexpected),
            "a wrong-rm-type filler is rejected, got {msgs:?}"
        );
    }

    #[test]
    fn slot_exclude_rejects_matching_filler() {
        let root = obs_slot(
            &[r"openEHR-EHR-OBSERVATION\..*"],
            &[r"openEHR-EHR-OBSERVATION\.secret\..*"],
            0,
            -1,
        );
        let msgs = walk_only(
            &content_obs("openEHR-EHR-OBSERVATION.secret.v1", "OBSERVATION"),
            &root,
        );
        assert!(
            msgs.iter().any(|m| m.kind == ValidationKind::Unexpected),
            "an exclude-matching filler is rejected, got {msgs:?}"
        );
    }

    #[test]
    fn slot_blanket_exclude_ignored_when_includes_present() {
        // ADL 1.4 closed-slot idiom: specific include + a blanket `.*` exclude. The
        // specific include wins (AOM 1.4 has no is_closed) — the filler passes.
        let root = obs_slot(&[r"openEHR-EHR-OBSERVATION\.bp\.v1"], &[".*"], 0, -1);
        assert!(
            walk_only(
                &content_obs("openEHR-EHR-OBSERVATION.bp.v1", "OBSERVATION"),
                &root
            )
            .is_empty(),
            "a specific include overrides a blanket `.*` exclude"
        );
    }

    #[test]
    fn slot_occurrences_min_and_max() {
        let root = obs_slot(&[r"openEHR-EHR-OBSERVATION\..*"], &[], 1, 1);
        let empty = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": []});
        assert!(
            walk_only(&empty, &root)
                .iter()
                .any(|m| m.kind == ValidationKind::Required),
            "an unfilled mandatory slot is Required"
        );
        let two = json!({"_type": "COMPOSITION", "archetype_node_id": "x", "content": [
            {"_type": "OBSERVATION", "archetype_node_id": "openEHR-EHR-OBSERVATION.a.v1",
             "name": {"_type": "DV_TEXT", "value": "a"}},
            {"_type": "OBSERVATION", "archetype_node_id": "openEHR-EHR-OBSERVATION.b.v1",
             "name": {"_type": "DV_TEXT", "value": "b"}}]});
        assert!(
            walk_only(&two, &root)
                .iter()
                .any(|m| m.kind == ValidationKind::Occurrences),
            "too many slot fillers is Occurrences"
        );
    }

    // ── DV_ORDINAL / DV_SCALE (symbol, value) pairing ─────────────────────────

    fn ordinal_node(rm: &str, scale: bool) -> WebTemplateNode {
        let mut n = node(rm, "/value");
        let mut input = WebTemplateInput::new(WebTemplateInputType::CodedText, None);
        let mk = |code: &str, v: i32| {
            let mut cv = WebTemplateCodedValue::new(code, None);
            if scale {
                cv.scale = Some(f64::from(v));
            } else {
                cv.ordinal = Some(v);
            }
            cv
        };
        input.list = vec![mk("at0014", 0), mk("at0015", 1)];
        n.inputs = vec![input];
        n
    }

    fn ordinal_value(rm: &str, v: &Value, code: &str) -> Value {
        json!({"_type": rm, "value": v, "symbol": {"_type": "DV_CODED_TEXT",
            "value": "s", "defining_code": {"_type": "CODE_PHRASE",
            "terminology_id": {"_type": "TERMINOLOGY_ID", "value": "local"}, "code_string": code}}})
    }

    #[test]
    fn ordinal_pair_must_match() {
        let n = ordinal_node("DV_ORDINAL", false);
        assert!(walk_only(&ordinal_value("DV_ORDINAL", &json!(0), "at0014"), &n).is_empty());
        assert!(
            kinds(&walk_only(
                &ordinal_value("DV_ORDINAL", &json!(1), "at0014"),
                &n
            ))
            .contains(&ValidationKind::CodedValue),
            "value 1 does not pair with symbol at0014"
        );
        assert!(
            kinds(&walk_only(
                &ordinal_value("DV_ORDINAL", &json!(0), "at0666"),
                &n
            ))
            .contains(&ValidationKind::CodedValue),
            "symbol at0666 is off the list"
        );
    }

    #[test]
    fn scale_pair_must_match() {
        let n = ordinal_node("DV_SCALE", true);
        assert!(walk_only(&ordinal_value("DV_SCALE", &json!(0.0), "at0014"), &n).is_empty());
        assert!(
            kinds(&walk_only(
                &ordinal_value("DV_SCALE", &json!(1.0), "at0014"),
                &n
            ))
            .contains(&ValidationKind::CodedValue),
            "scale 1.0 does not pair with symbol at0014"
        );
    }

    // ── C_STRING fail-closed with fancy-regex fallback ────────────────────────

    #[test]
    fn c_string_backreference_is_enforced() {
        // `(a)\1` uses a backreference the `regex` crate rejects; `fancy-regex`
        // compiles it, so the pattern is enforced instead of silently passing.
        let mut n = node("DV_TEXT", "/text");
        let mut input = WebTemplateInput::new(WebTemplateInputType::Text, None);
        input.validation = Some(WebTemplateValidation {
            pattern: Some(r"(a)\1".to_owned()),
            ..Default::default()
        });
        n.inputs = vec![input];
        assert!(
            kinds(&walk_only(&json!({"_type": "DV_TEXT", "value": "ab"}), &n))
                .contains(&ValidationKind::PatternError),
            "`ab` must fail the backreference pattern rather than silently pass"
        );
        assert!(walk_only(&json!({"_type": "DV_TEXT", "value": "aa"}), &n).is_empty());
    }

    // ── C_TIME / C_DATE_TIME timezone_validity ────────────────────────────────

    #[test]
    fn timezone_validity_mandatory_and_disallowed() {
        let mut n = node("DV_TIME", "/value");
        n.inputs = vec![WebTemplateInput::new(WebTemplateInputType::Time, None)];
        // 1001 = mandatory timezone.
        n.tz_validity = Some(1001);
        assert!(
            kinds(&walk_only(
                &json!({"_type": "DV_TIME", "value": "10:30:00"}),
                &n
            ))
            .contains(&ValidationKind::PatternError),
            "a missing mandatory timezone is rejected"
        );
        assert!(walk_only(&json!({"_type": "DV_TIME", "value": "10:30:00Z"}), &n).is_empty());
        // 1003 = disallowed timezone.
        n.tz_validity = Some(1003);
        assert!(
            kinds(&walk_only(
                &json!({"_type": "DV_TIME", "value": "10:30:00+01:00"}),
                &n
            ))
            .contains(&ValidationKind::PatternError),
            "a present disallowed timezone is rejected"
        );
        assert!(walk_only(&json!({"_type": "DV_TIME", "value": "10:30:00"}), &n).is_empty());
    }

    // ── name-differentiated same-archetype-id siblings ────────────────────────
    //
    // A template may fill the same archetype twice under one container, the two
    // fills differentiated by their runtime `name` (RM common
    // `master03-archetyped_package.adoc` §"The `LOCATABLE` class" L33-35: a `name`
    // distinguishes sibling nodes that share an `archetype_node_id`; AOM 1.4
    // `master04-constraint_model_package.adoc` §`node_id` L41: node ids "guarantee
    // sibling node unique identification"). Templates realise this by putting a
    // fixed `name/value` `C_STRING` on all-but-one sibling, so one sibling stays
    // *unqualified* (its `name` unconstrained). Each instance must be routed to
    // exactly the sibling whose name it matches, and the unqualified sibling must
    // admit only the instances no name-qualified sibling claims.

    /// Two same-archetype siblings under `items`, one unqualified (name "A", inner
    /// `items` closed to `at0004`) and one name-qualified ('B', inner `items`
    /// closed to `at0013`).
    fn name_diff_parent() -> WebTemplateNode {
        let mut root = node("CLUSTER", "");
        let sib_a = {
            let mut n = node("CLUSTER", "/items[openEHR-EHR-CLUSTER.c.v1]");
            n.name = Some("A".to_owned());
            n.min = Some(0);
            n.max = 1;
            n.closed_attributes = vec![WebTemplateClosedAttribute {
                path: "/items[openEHR-EHR-CLUSTER.c.v1]/items".to_owned(),
                allowed_ids: vec!["at0004".to_owned()],
                slots: vec![],
            }];
            n
        };
        let sib_b = {
            let mut n = node("CLUSTER", "/items[openEHR-EHR-CLUSTER.c.v1,'B']");
            n.name = Some("B".to_owned());
            n.min = Some(0);
            n.max = 1;
            n.closed_attributes = vec![WebTemplateClosedAttribute {
                path: "/items[openEHR-EHR-CLUSTER.c.v1,'B']/items".to_owned(),
                allowed_ids: vec!["at0013".to_owned()],
                slots: vec![],
            }];
            n
        };
        root.children = vec![sib_a, sib_b];
        root
    }

    /// A same-archetype CLUSTER instance with `name` and a single at-coded child.
    fn c_instance(name: &str, child_id: &str) -> Value {
        json!({
            "_type": "CLUSTER", "archetype_node_id": "openEHR-EHR-CLUSTER.c.v1",
            "name": {"_type": "DV_TEXT", "value": name},
            "items": [{"_type": "ELEMENT", "archetype_node_id": child_id,
                       "name": {"_type": "DV_TEXT", "value": "leaf"},
                       "value": {"_type": "DV_TEXT", "value": "v"}}]
        })
    }

    fn unexpected_of(msgs: &[ValidationMessage]) -> Vec<&ValidationMessage> {
        msgs.iter()
            .filter(|m| m.kind == ValidationKind::Unexpected)
            .collect()
    }

    #[test]
    fn name_diff_siblings_route_each_instance_to_its_own_overlay() {
        let root = name_diff_parent();
        // Both instances present, each carrying its own overlay's child.
        let inst = json!({
            "_type": "CLUSTER", "archetype_node_id": "x",
            "name": {"_type": "DV_TEXT", "value": "root"},
            "items": [c_instance("A", "at0004"), c_instance("B", "at0013")]
        });
        let msgs = walk_only(&inst, &root);
        assert!(
            unexpected_of(&msgs).is_empty(),
            "both name-differentiated siblings should validate against their own \
             overlay, got {msgs:?}"
        );
    }

    #[test]
    fn name_qualified_siblings_child_in_unqualified_instance_is_unexpected() {
        let root = name_diff_parent();
        // `at0013` belongs to sibling 'B' only; inside the instance named "A" it
        // must still be Unexpected (true rejections preserved).
        let inst = json!({
            "_type": "CLUSTER", "archetype_node_id": "x",
            "name": {"_type": "DV_TEXT", "value": "root"},
            "items": [c_instance("A", "at0013")]
        });
        let msgs = walk_only(&inst, &root);
        assert!(
            unexpected_of(&msgs)
                .iter()
                .any(|m| m.message.contains("at0013")),
            "at0013 in the unqualified sibling's instance must be Unexpected, got {msgs:?}"
        );
    }

    #[test]
    fn unqualified_sibling_admits_a_runtime_named_residual_instance() {
        let root = name_diff_parent();
        // An instance whose name matches NO name-qualified sibling ("other") routes
        // to the unqualified (residual) sibling — its `name` being unconstrained,
        // master03 §"The `LOCATABLE` class" L35. Its own overlay child (`at0004`)
        // therefore validates clean…
        let ok = json!({
            "_type": "CLUSTER", "archetype_node_id": "x",
            "name": {"_type": "DV_TEXT", "value": "root"},
            "items": [c_instance("other", "at0004")]
        });
        assert!(
            unexpected_of(&walk_only(&ok, &root)).is_empty(),
            "a residual-named instance must validate against the unqualified overlay"
        );
        // …but a child that overlay forbids (`at0013`) is still Unexpected there.
        let bad = json!({
            "_type": "CLUSTER", "archetype_node_id": "x",
            "name": {"_type": "DV_TEXT", "value": "root"},
            "items": [c_instance("other", "at0013")]
        });
        assert!(
            unexpected_of(&walk_only(&bad, &root))
                .iter()
                .any(|m| m.message.contains("at0013")),
            "the residual instance is still closed-world checked against the \
             unqualified overlay"
        );
    }

    // ── validation-walk cost measurement (not a gate) ─────────────────────────

    /// Count the `_type`-bearing nodes reachable in `v` (the units both
    /// template-independent passes visit).
    fn count_type_nodes(v: &Value) -> usize {
        match v {
            Value::Object(obj) => {
                let self_count = usize::from(obj.contains_key("_type"));
                self_count
                    + obj
                        .iter()
                        .filter(|(k, _)| !k.starts_with('_'))
                        .map(|(_, val)| count_type_nodes(val))
                        .sum::<usize>()
            }
            Value::Array(a) => a.iter().map(count_type_nodes).sum(),
            _ => 0,
        }
    }

    /// Time `iters` runs of `f`, returning microseconds per run.
    fn time_pass(iters: u32, mut f: impl FnMut() -> usize) -> f64 {
        let start = std::time::Instant::now();
        let mut sink = 0usize;
        for _ in 0..iters {
            sink = sink.wrapping_add(f());
        }
        std::hint::black_box(sink);
        start.elapsed().as_secs_f64() * 1e6 / f64::from(iters)
    }

    /// MEASUREMENT (not a correctness gate): quantify the pre-tx template-
    /// independent validation walk over the populated IPS example (~1.5k `_type`
    /// nodes). The RM-invariant and terminology passes each traverse the whole
    /// instance independently; this splits and times them. Ignored by default
    /// (timing, not correctness); run:
    /// `cargo nextest run -p openehr-its --run-ignored all \
    ///   -E 'test(measure_ips_validation_walk_cost)' --no-capture`.
    #[test]
    #[ignore = "measurement, not a correctness gate — run with --run-ignored all"]
    fn measure_ips_validation_walk_cost() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/cnf-runner/artifacts/corpus/templates/ckm/international-patient-summary.example.json"
        );
        let comp: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read IPS example"))
                .expect("parse IPS example");
        let node_count = count_type_nodes(&comp);

        // Warm up (allocator, branch predictors, the lazily-initialized bundle).
        for _ in 0..5 {
            std::hint::black_box(validate_rm_and_terminology(&comp).len());
        }

        let iters = 50;
        let t_rm = time_pass(iters, || {
            let mut v = Validator::default();
            v.rm_invariant_pass(&comp, &mut String::new());
            v.out.len()
        });
        let t_term = time_pass(iters, || {
            let mut v = Validator::default();
            v.terminology_pass(&comp, &mut String::new(), None);
            v.out.len()
        });
        let t_both = time_pass(iters, || validate_rm_and_terminology(&comp).len());

        eprintln!("IPS validation walk cost ({node_count} _type nodes, {iters} iters):");
        eprintln!("  pass 1 rm_invariant_pass : {t_rm:>8.1} us/op");
        eprintln!("  pass 2 terminology_pass  : {t_term:>8.1} us/op");
        eprintln!("  combined (1+2)           : {t_both:>8.1} us/op");
    }

    /// MEASUREMENT (not a correctness gate): quantify the archetype-conformance
    /// **walk** (pass 3) over the populated IPS example against its OPT-built
    /// `WebTemplate`, plus the full [`validate_composition`]. Ignored by default
    /// (timing, not correctness); run:
    /// `cargo nextest run -p openehr-its --run-ignored all \
    ///   -E 'test(measure_ips_validation_full_cost)' --no-capture`.
    #[test]
    #[ignore = "measurement, not a correctness gate — run with --run-ignored all"]
    fn measure_ips_validation_full_cost() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/cnf-runner/artifacts/corpus/templates/ckm"
        );
        let opt_xml = std::fs::read_to_string(format!("{dir}/international-patient-summary.opt"))
            .expect("read IPS OPT");
        let opt = crate::opt14::from_xml(&opt_xml).expect("parse IPS OPT");
        let wt = crate::flat::webtemplate::build_web_template(&opt).expect("build IPS WebTemplate");
        let comp: Value = serde_json::from_str(
            &std::fs::read_to_string(format!("{dir}/international-patient-summary.example.json"))
                .expect("read IPS example"),
        )
        .expect("parse IPS example");
        let node_count = count_type_nodes(&comp);

        // Warm up (allocator, branch predictors, the lazily-initialized bundle).
        for _ in 0..5 {
            std::hint::black_box(validate_composition(&comp, &wt).len());
        }

        // Public entry points only, so this harness compiles unchanged.
        let iters = 50;
        let t_rmterm = time_pass(iters, || validate_rm_and_terminology(&comp).len());
        let t_walk = time_pass(iters, || validate_archetype_conformance(&comp, &wt).len());
        let t_all = time_pass(iters, || validate_composition(&comp, &wt).len());

        eprintln!("IPS full validation cost ({node_count} _type nodes, {iters} iters):");
        eprintln!("  passes 1+2 rm+terminology      : {t_rmterm:>8.1} us/op");
        eprintln!("  pass 3 walk (archetype conf.)  : {t_walk:>8.1} us/op");
        eprintln!("  full validate_composition      : {t_all:>8.1} us/op");
    }
}
