//! Composition validation — the archie `RMObjectValidator` equivalent (P15 PR-C).
//!
//! [`validate_composition`] validates a canonical-JSON COMPOSITION against its
//! operational template (as a flattened [`WebTemplate`]) **plus** the openEHR
//! Reference Model class invariants **plus** the RM-mandated openEHR
//! terminology, collecting *all* violations (not fail-fast), each keyed by an RM
//! path. It composes three existing building blocks: the [`WebTemplate`] tree
//! (P14), [`openehr_rm::validate::validate_rm_value`] (RM class invariants), and
//! [`openehr_term::bundle`] (openEHR terminology).
//!
//! # The three passes
//!
//! Validation runs three independent collecting passes over the instance:
//!
//! 1. **RM-invariant pass** — recurse the whole instance; for every node with a
//!    `_type`, run its RM class invariants ([`validate_rm_value`]). This is
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
//! # `// PORT NOTE:` fidelity
//!
//! The instance-validation *algorithm* is spec-underdetermined — archie's
//! `RMObjectValidator` is the oracle, and it walks the AOM constraint tree
//! directly. We approximate it over the *compacted* `WebTemplate`: the wrapper
//! nodes archie sees (ELEMENT / `ITEM_STRUCTURE` / HISTORY / EVENT) are folded
//! into a child's `aqlPath`, so we navigate the instance by the RM-attribute +
//! `[archetype_node_id]` predicate chain that separates a `WebTemplate` child from
//! its (compacted) parent, counting occurrences per intermediate container. The
//! `C_DV`_* leaf semantics (unit-scoped magnitude ranges, coded-value membership,
//! string patterns) are approximated from the `WebTemplate` `inputs`. Where a
//! check cannot be made reliably (temporal ranges, precision, `depends_on`
//! choices, unresolved archetype slots/internal refs, deep required nodes behind
//! an absent optional wrapper) we skip rather than over-reject — biasing, like
//! archie, toward reporting only confident violations.

mod leaf;
mod subtype;
mod terminology;

use indexmap::IndexMap;
use openehr_rm::validate::validate_rm_value;
use serde_json::Value;

use crate::webtemplate::{WebTemplate, WebTemplateNode};

/// A single validation violation, keyed by the RM path of the offending node.
///
/// Mirrors archie's `RMObjectValidationMessage` (message + path + type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationMessage {
    /// The RM path to the offending node (archetype `aqlPath` for
    /// archetype-conformance violations, RM instance path for RM-invariant /
    /// terminology violations).
    pub path: String,
    /// A human-readable description of the violation.
    pub message: String,
    /// The violation category (mirrors archie `RMObjectValidationMessageType`).
    pub kind: ValidationKind,
}

/// The category of a [`ValidationMessage`] (archie `RMObjectValidationMessageType`).
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
    // Pass 1: RM class invariants over the whole instance (compaction-independent).
    v.rm_invariant_pass(composition, "");
    // Pass 2: RM-mandated openEHR terminology.
    v.terminology_pass(composition, "", None);
    // Pass 3: archetype conformance guided by the WebTemplate tree.
    v.walk(composition, &wt.tree);
    v.out
}

#[derive(Default)]
struct Validator {
    out: Vec<ValidationMessage>,
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

    fn rm_invariant_pass(&mut self, v: &Value, path: &str) {
        let Some(obj) = v.as_object() else { return };
        if obj.contains_key("_type") {
            let mut inv = Vec::new();
            validate_rm_value(v, &mut inv);
            for iv in inv {
                let p = if iv.path.is_empty() {
                    norm_path(path)
                } else {
                    format!("{}/{}", path.trim_end_matches('/'), iv.path)
                };
                self.push(p, iv.message, ValidationKind::Invariant);
            }
        }
        for (k, val) in obj {
            if k.starts_with('_') {
                continue;
            }
            match val {
                Value::Array(a) => {
                    for (i, item) in a.iter().enumerate() {
                        if item.is_object() {
                            self.rm_invariant_pass(item, &format!("{path}/{k}[{i}]"));
                        }
                    }
                }
                Value::Object(_) => self.rm_invariant_pass(val, &format!("{path}/{k}")),
                _ => {}
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

        // Group children by aql path: multiple children sharing a path are a
        // polymorphic type choice (the instance matches whichever alternative it
        // conforms to), so they must be resolved together — never each flagged.
        let mut groups: IndexMap<&str, Vec<&WebTemplateNode>> = IndexMap::new();
        for child in &wt.children {
            groups
                .entry(child.aql_path.as_str())
                .or_default()
                .push(child);
        }
        for group in groups.values() {
            self.check_group(instance, wt, group);
        }

        self.check_cardinalities(instance, wt);
    }

    /// Resolve a group of `WebTemplate` children sharing an aql path (a single
    /// node, or a set of polymorphic type alternatives) against the instance:
    /// occurrence-check the group, then recurse into each matched instance,
    /// routing it to the conforming alternative.
    fn check_group(
        &mut self,
        parent: &Value,
        wt_parent: &WebTemplateNode,
        group: &[&WebTemplateNode],
    ) {
        let first = group[0];
        let Some(rel) = first.aql_path.strip_prefix(&wt_parent.aql_path) else {
            // Not a path descendant of the parent (unexpected) — skip.
            return;
        };
        let segments = parse_segments(rel);
        if segments.is_empty() {
            return;
        }
        // The identity segment is the last one carrying a node-id predicate; if
        // none carries one, the last segment (a plain single-valued attribute).
        let identity_idx = segments
            .iter()
            .rposition(|s| !matches!(s.pred, Pred::Any))
            .unwrap_or(segments.len() - 1);
        let id_seg = &segments[identity_idx];
        let trailing = &segments[identity_idx + 1..];

        // Navigate the intermediate segments to the container node(s).
        let mut containers: Vec<&Value> = vec![parent];
        for seg in &segments[..identity_idx] {
            containers = containers
                .iter()
                .flat_map(|c| get_attr(c, &seg.attr))
                .filter(|n| seg.pred.matches(n))
                .collect();
        }

        // Occurrences are an *archetype-node* constraint: only checked when the
        // matched node is identified by an archetype-node predicate (at-code /
        // archetype id). Plain RM structural attributes (`context`, `value`,
        // `action_archetype_id`, …) are governed by RM cardinality/invariants,
        // not archetype occurrences, so they are not occurrence-checked here.
        //
        // PORT NOTE: `ism_transition` careflow steps are modelled by the
        // WebTemplate builder as separate per-state nodes (careflow synthesis is
        // a documented builder scope gap), yet an ACTION instance carries a
        // single ISM_TRANSITION — occurrence-checking them would spuriously demand
        // every state, so they are skipped (ISM_TRANSITION presence is an RM
        // invariant). `in_context` nodes are supplied structurally.
        let occ_applies = matches!(id_seg.pred, Pred::Node(_) | Pred::NodeNamed(..))
            && id_seg.attr != "ism_transition"
            && !group.iter().any(|c| c.in_context == Some(true));
        let group_min = group.iter().filter_map(|c| c.min).min().unwrap_or(0).max(0);
        let group_max = if group.iter().any(|c| c.max == -1) {
            -1
        } else {
            group.iter().map(|c| c.max).max().unwrap_or(-1)
        };

        for container in &containers {
            let matched: Vec<&Value> = get_attr(container, &id_seg.attr)
                .into_iter()
                .filter(|n| id_seg.pred.matches(n))
                .collect();
            if occ_applies {
                self.emit_occurrences(&first.aql_path, group_min, group_max, matched.len());
            }
            for node in matched {
                for target in navigate_trailing(node, trailing) {
                    if group.len() == 1 {
                        self.walk(target, first);
                    } else {
                        self.visit_choice(target, group);
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
    fn emit_occurrences(&mut self, aql_path: &str, min: i32, max: i32, count: usize) {
        let count_i = i32::try_from(count).unwrap_or(i32::MAX);
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
        if max != -1 && count_i > max {
            self.push(
                aql_path,
                format!("too many occurrences: found {count}, expected at most {max}"),
                ValidationKind::Occurrences,
            );
        }
    }

    /// Container-cardinality check: for each cardinality on this node, count the
    /// children under the constrained attribute path and compare to `min`/`max`.
    fn check_cardinalities(&mut self, instance: &Value, wt: &WebTemplateNode) {
        for card in &wt.cardinalities {
            let Some(rel) = card.path.strip_prefix(&wt.aql_path) else {
                continue;
            };
            let segments = parse_segments(rel);
            if segments.is_empty() {
                continue;
            }
            // Navigate all but the last segment to the container, then count the
            // last attribute's children (cardinality is over the whole set).
            let mut containers: Vec<&Value> = vec![instance];
            for seg in &segments[..segments.len() - 1] {
                containers = containers
                    .iter()
                    .flat_map(|c| get_attr(c, &seg.attr))
                    .filter(|n| seg.pred.matches(n))
                    .collect();
            }
            let last = &segments[segments.len() - 1];
            for container in &containers {
                let count =
                    i32::try_from(get_attr(container, &last.attr).len()).unwrap_or(i32::MAX);
                let min = card.min.unwrap_or(0).max(0);
                if count < min {
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

/// A predicate on a path segment (`[atNNNN]`, `[archetype_id]`, `[atNNNN,'Name']`).
enum Pred {
    /// No predicate — matches every node under the attribute.
    Any,
    /// Matches a node whose `archetype_node_id` equals this id.
    Node(String),
    /// Matches a node whose `archetype_node_id` and `name/value` both match.
    NodeNamed(String, String),
}

impl Pred {
    fn matches(&self, node: &Value) -> bool {
        match self {
            Pred::Any => true,
            Pred::Node(id) => node_id(node) == Some(id.as_str()),
            Pred::NodeNamed(id, name) => {
                node_id(node) == Some(id.as_str()) && instance_name(node).as_deref() == Some(name)
            }
        }
    }
}

/// One parsed path segment: an RM attribute plus an optional predicate.
struct Segment {
    attr: String,
    pred: Pred,
}

/// Parse a relative aql path (`/attr[pred]/attr2/…`) into segments, respecting
/// `[...]` brackets (a `/` inside a predicate does not split).
fn parse_segments(rel: &str) -> Vec<Segment> {
    let mut raw: Vec<&str> = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, c) in rel.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => depth -= 1,
            '/' if depth == 0 => {
                if i > start {
                    raw.push(&rel[start..i]);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < rel.len() {
        raw.push(&rel[start..]);
    }
    raw.into_iter().map(parse_segment).collect()
}

fn parse_segment(seg: &str) -> Segment {
    let Some(open) = seg.find('[') else {
        return Segment {
            attr: seg.to_owned(),
            pred: Pred::Any,
        };
    };
    let attr = seg[..open].to_owned();
    let inner = seg[open + 1..].trim_end_matches(']');
    let pred = match inner.split_once(',') {
        Some((id, name)) => Pred::NodeNamed(
            id.trim().to_owned(),
            name.trim().trim_matches('\'').trim_matches('"').to_owned(),
        ),
        None => {
            if inner.is_empty() {
                Pred::Any
            } else {
                Pred::Node(inner.trim().to_owned())
            }
        }
    };
    Segment { attr, pred }
}

/// The object(s) under an RM attribute: a single object → one, an array → each
/// object element, anything else → none.
fn get_attr<'a>(node: &'a Value, attr: &str) -> Vec<&'a Value> {
    match node.get(attr) {
        Some(Value::Array(a)) => a.iter().filter(|v| v.is_object()).collect(),
        Some(v @ Value::Object(_)) => vec![v],
        _ => Vec::new(),
    }
}

/// Follow a chain of trailing (predicate-free) attributes from a node, returning
/// the reached object nodes. An empty chain returns the node itself.
fn navigate_trailing<'a>(node: &'a Value, segments: &[Segment]) -> Vec<&'a Value> {
    let mut current = vec![node];
    for seg in segments {
        current = current
            .iter()
            .flat_map(|c| get_attr(c, &seg.attr))
            .filter(|n| seg.pred.matches(n))
            .collect();
    }
    current
}

fn node_id(node: &Value) -> Option<&str> {
    node.get("archetype_node_id").and_then(Value::as_str)
}

/// A node's `name/value` (the `name` is a `DV_TEXT`, or occasionally a bare
/// string).
fn instance_name(node: &Value) -> Option<String> {
    match node.get("name") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Object(o)) => o.get("value").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}

/// Normalize an RM instance path (empty → the root `/`).
fn norm_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_owned()
    } else {
        path.to_owned()
    }
}

#[cfg(test)]
mod tests;
