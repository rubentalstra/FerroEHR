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
use openehr_rm::paths::select_children;
use openehr_rm::validate::validate_rm_value;
use serde_json::Value;

use crate::path;
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

/// Validate only the **template-independent** passes: RM class invariants + the
/// RM-mandated openEHR terminology. These hold for *every* RM instance whether
/// or not an operational template is referenced (RM invariants and terminology
/// bindings are properties of the instance, not of the archetype — spec 07
/// finding F-07-02). A COMPOSITION committed without a declared `template_id`
/// cannot be archetype-conformance-checked, but must still pass these.
#[must_use]
pub fn validate_rm_and_terminology(composition: &Value) -> Vec<ValidationMessage> {
    let mut v = Validator::default();
    v.rm_invariant_pass(composition, "");
    v.terminology_pass(composition, "", None);
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
        self.check_existence(instance, wt);
        self.check_slots(instance, wt);
    }

    /// Type-conformance check for wrapper constraints the compactor hoisted
    /// away (`ITEM_*` / `HISTORY` / single `EVENT` — AOM 1.4 type conformance;
    /// master16 §`ITEM_STRUCTURE`/§EVENT "Class not allowed"): an instance node
    /// matched at a recorded slot path must conform to one of the RM types the
    /// same-path slot alternatives allow. An abstract recorded type
    /// (`ITEM_STRUCTURE`, `EVENT`) admits every concrete subtype via
    /// [`subtype::conforms`].
    fn check_slots(&mut self, instance: &Value, wt: &WebTemplateNode) {
        let mut groups: IndexMap<&str, Vec<&str>> = IndexMap::new();
        for slot in &wt.slots {
            groups
                .entry(slot.path.as_str())
                .or_default()
                .push(slot.rm_type.as_str());
        }
        for (path, allowed) in groups {
            let Some(rel) = path.strip_prefix(&wt.aql_path) else {
                continue;
            };
            let segments = path::parse(rel);
            let Some((last, intermediate)) = segments.split_last() else {
                continue;
            };
            let containers = path::navigate(&[instance], intermediate);
            for container in &containers {
                for node in select_children(container, last) {
                    let Some(it) = node.get("_type").and_then(Value::as_str) else {
                        continue;
                    };
                    if !allowed.iter().any(|a| subtype::conforms(it, a)) {
                        self.push(
                            path,
                            format!(
                                "class {it} not allowed: the slot is constrained to [{}]",
                                allowed.join(", ")
                            ),
                            ValidationKind::WrongType,
                        );
                    }
                }
            }
        }
    }

    /// AOM 1.4 `C_ATTRIBUTE.existence` check (F-07-04): for each mandatory plain
    /// RM attribute constrained on this node, verify the attribute *field* is
    /// present on the matched instance. Existence is distinct from occurrences
    /// (archetype-node-identified children) and cardinality (container
    /// membership) — it governs whether the attribute field is there at all — so
    /// this fills the gap for plain structural attributes the occurrence check
    /// deliberately skips. Only the lower bound (mandatory presence) is enforced;
    /// the upper bound is governed by RM single-valuedness / cardinality.
    fn check_existence(&mut self, instance: &Value, wt: &WebTemplateNode) {
        for ex in &wt.existence {
            if ex.min < 1 {
                continue;
            }
            let Some(rel) = ex.path.strip_prefix(&wt.aql_path) else {
                continue;
            };
            let segments = path::parse(rel);
            let Some((last, intermediate)) = segments.split_last() else {
                continue;
            };
            // Navigate the intermediate segments to the container node(s).
            let containers = path::navigate(&[instance], intermediate);
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
        group: &[&WebTemplateNode],
    ) {
        let first = group[0];
        let Some(rel) = first.aql_path.strip_prefix(&wt_parent.aql_path) else {
            // Not a path descendant of the parent (unexpected) — skip.
            return;
        };
        let segments = path::parse(rel);
        if segments.is_empty() {
            return;
        }
        // The identity segment is the last one carrying a predicate; if none
        // carries one, the last segment (a plain single-valued attribute).
        let identity_idx = segments
            .iter()
            .rposition(|s| !s.predicate.is_empty())
            .unwrap_or(segments.len() - 1);
        let id_seg = &segments[identity_idx];
        let trailing = &segments[identity_idx + 1..];

        // Navigate the intermediate segments to the container node(s).
        let containers = path::navigate(&[parent], &segments[..identity_idx]);

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
        let occ_applies = id_seg.predicate.archetype_node_id.is_some()
            && id_seg.attribute != "ism_transition"
            && !group.iter().any(|c| c.in_context == Some(true));
        let group_min = group.iter().filter_map(|c| c.min).min().unwrap_or(0).max(0);
        let group_max = if group.iter().any(|c| c.max == -1) {
            -1
        } else {
            group.iter().map(|c| c.max).max().unwrap_or(-1)
        };

        for container in &containers {
            let matched = select_children(container, id_seg);
            if occ_applies {
                self.emit_occurrences(&first.aql_path, group_min, group_max, matched.len());
            }
            for node in matched {
                for target in path::navigate(&[node], trailing) {
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

    /// Container-cardinality check: for each constraining cardinality on this
    /// node, count the children under the constrained attribute path and compare
    /// to `min`/`max`. Iterates [`WebTemplateNode::card_all`] — the full AOM 1.4
    /// §cardinality set (`0..1`/`1..1`/`1..*` included), a superset of the
    /// Better-filtered serialized `cardinalities`.
    fn check_cardinalities(&mut self, instance: &Value, wt: &WebTemplateNode) {
        for card in &wt.card_all {
            let Some(rel) = card.path.strip_prefix(&wt.aql_path) else {
                continue;
            };
            let segments = path::parse(rel);
            let Some((last, intermediate)) = segments.split_last() else {
                continue;
            };
            // Navigate all but the last segment to the container, then count the
            // last attribute's children (cardinality is over the whole set).
            let containers = path::navigate(&[instance], intermediate);
            for container in &containers {
                let count =
                    i32::try_from(select_children(container, last).len()).unwrap_or(i32::MAX);
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
//
// RM-path parsing (`[atNNNN]` / `[archetype-id]` / `[atNNNN,'name']` predicates)
// and per-step navigation over the canonical-JSON tree are the single
// implementation in [`openehr_rm::paths`], reached here via [`crate::path`]
// (`parse` / `navigate` / `select_children`). Only the checks below —
// attribute-presence for the existence rule and RM instance-path
// normalisation — are validation-specific.

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
mod tests;
