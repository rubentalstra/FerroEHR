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

use crate::path;
use crate::webtemplate::{WebTemplate, WebTemplateArchetypeSlot, WebTemplateNode};

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
    /// An instance node is not admitted by any sibling constraint or open slot
    /// under a closed (constrained) attribute (ADR-012 closed-archetype).
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

/// The set of identity `(attribute, archetype_node_id)` pairs that appear in
/// more than one distinct sibling path (relative to `parent_aql`) — i.e. the
/// template distinguishes same-id siblings by name, so the name fallback in
/// [`path::select_children_matched`] must stay off for them.
fn identity_ambiguity<'a>(
    paths: impl Iterator<Item = &'a str>,
    parent_aql: &str,
) -> std::collections::HashSet<(String, String)> {
    let mut seen: std::collections::HashMap<(String, String), u32> =
        std::collections::HashMap::new();
    for p in paths {
        let Some(rel) = p.strip_prefix(parent_aql) else {
            continue;
        };
        let segments = path::parse(rel);
        let Some(id_seg) = segments.iter().rfind(|s| !s.predicate.is_empty()) else {
            continue;
        };
        if let Some(id) = &id_seg.predicate.archetype_node_id {
            *seen
                .entry((id_seg.attribute.clone(), id.clone()))
                .or_default() += 1;
        }
    }
    seen.into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(k, _)| k)
        .collect()
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
        self.check_archetyped_valid(obj, path);
        self.check_nonempty_lists(obj, path);
        self.check_data_structure_shapes(obj, path);
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

    /// `LOCATABLE.Archetyped_valid`: `is_archetype_root xor archetype_details =
    /// Void` (`RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc` L60).
    /// The enforceable arm on an instance is: a **non-root** node — one whose
    /// `archetype_node_id` is an `at`/`id` term code, which per the node-id
    /// format can never be the root of an archetyped structure — must NOT carry
    /// `archetype_details`.
    ///
    /// PORT NOTE (A1 rm-common-change-control-R46): the converse arm ("an
    /// archetype-HRID node must carry `archetype_details`") is NOT enforced —
    /// the reference object model derives `is_archetype_root` from
    /// `archetype_details` presence (making that reading tautological), and the
    /// CNF's own valid data sets + the canonical-JSON corpus systematically omit
    /// `archetype_details` on nested archetype roots (182 occurrences measured
    /// 2026-07-11); the CNF fixtures win over a prose reading that would reject
    /// them (`.claude/rules/spec-adherence.md`). The COMPOSITION root arm stays
    /// separately enforced (`composition_impl.rs` `Is_archetype_root`).
    fn check_archetyped_valid(&mut self, obj: &serde_json::Map<String, Value>, path: &str) {
        let Some(node_id) = obj.get("archetype_node_id").and_then(Value::as_str) else {
            return;
        };
        let is_term_code = node_id
            .strip_prefix("at")
            .or_else(|| node_id.strip_prefix("id"))
            .is_some_and(|rest| rest.chars().next().is_some_and(|c| c.is_ascii_digit()));
        if is_term_code && obj.get("archetype_details").is_some_and(|d| !d.is_null()) {
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
    fn check_nonempty_lists(&mut self, obj: &serde_json::Map<String, Value>, path: &str) {
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
        for (attr, invariant) in [
            ("mappings", "Mappings_valid"),
            ("other_reference_ranges", "Other_reference_ranges_validity"),
        ] {
            if obj
                .get(attr)
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            {
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
        let Some(ty) = obj.get("_type").and_then(Value::as_str) else {
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
    fn check_data_structure_shapes(&mut self, obj: &serde_json::Map<String, Value>, path: &str) {
        let Some(ty) = obj.get("_type").and_then(Value::as_str) else {
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
        // An identity (attribute, archetype_node_id) claimed by more than one
        // distinct child path means the template disambiguates same-id siblings
        // by name — the name fallback must stay off for those (see
        // `path::select_children_matched`).
        let ambiguous = identity_ambiguity(groups.keys().copied(), &wt.aql_path);
        for group in groups.values() {
            self.check_group(instance, wt, group, &ambiguous);
        }

        self.check_cardinalities(instance, wt);
        self.check_existence(instance, wt);
        self.check_slots(instance, wt);
        self.check_closure(instance, wt);
    }

    /// Closed-archetype walk (ADR-012, F-07-05 + F-07-10). Under each constrained
    /// attribute this node records (an attribute with fixed archetype-node
    /// alternatives and/or open `ARCHETYPE_SLOT`s), an instance child bearing an
    /// `archetype_node_id` (i.e. a LOCATABLE — the archetyped-content
    /// discriminator; no RM metadata value carries one) must match a fixed
    /// sibling identity **or** an open slot; any other archetyped child is an
    /// "unexpected node". RM-permitted unconstrained metadata attributes and
    /// wholly-unconstrained attributes are never recorded, so stay open (ADR-012
    /// rule 2). A rejected node is not descended into (the walk already skips it).
    ///
    /// PORT NOTE (ADR-012): AOM 1.4 `valid_value`
    /// (`AM/docs/AOM1.4/master04-constraint_model_package.adoc` §`Valid_value`
    /// L60-62) is a positive-only cascade, silent on unmatched instance nodes;
    /// closed-world rejection follows the AOM2 direction + de-facto CDR behaviour
    /// and lands only behind the ECC zero-drift gate.
    fn check_closure(&mut self, instance: &Value, wt: &WebTemplateNode) {
        for ca in &wt.closed_attributes {
            let Some(rel) = ca.path.strip_prefix(&wt.aql_path) else {
                continue;
            };
            let segments = path::parse(rel);
            let Some((last, intermediate)) = segments.split_last() else {
                continue;
            };
            for container in &path::navigate(&[instance], intermediate) {
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
                    // PORT NOTE (ADR-012 rule 4): an unmatched *archetype-rooted*
                    // child (`openEHR-…` id) is tolerated when the attribute
                    // carries no ARCHETYPE_SLOT constraint — OPT 1.4 flattening
                    // does not enumerate the full slot-fill universe, and the
                    // CNF corpus itself commits ENTRY archetypes the template
                    // does not list (archie accepts). Where slots ARE declared,
                    // archetype-rooted fillers stay subject to slot admission
                    // (include/exclude, F-07-10) below.
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
    fn check_slots(&mut self, instance: &Value, wt: &WebTemplateNode) {
        let mut groups: IndexMap<&str, Vec<&str>> = IndexMap::new();
        for slot in &wt.slots {
            groups
                .entry(slot.path.as_str())
                .or_default()
                .push(slot.rm_type.as_str());
        }
        let ambiguous = identity_ambiguity(groups.keys().copied(), &wt.aql_path);
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
                for node in {
                    let fallback_ok = last.predicate.archetype_node_id.as_ref().is_none_or(|id| {
                        !ambiguous.contains(&(last.attribute.clone(), id.clone()))
                    });
                    path::select_children_matched(container, last, fallback_ok)
                } {
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
        // Existence is a lower-bound (mandatory-presence) constraint — relaxed
        // away for a `553|incomplete|` commit (master06 §"Incomplete Content").
        if self.relax_lower_bounds {
            return;
        }
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
        ambiguous: &std::collections::HashSet<(String, String)>,
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

        let fallback_ok = id_seg
            .predicate
            .archetype_node_id
            .as_ref()
            .is_none_or(|id| !ambiguous.contains(&(id_seg.attribute.clone(), id.clone())));
        for container in &containers {
            let matched = path::select_children_matched(container, id_seg, fallback_ok);
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
    // PORT NOTE (BASE primitives): occurrence/cardinality evaluation here uses
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
            //
            // AOM 1.4 §cardinality vs §existence: cardinality constrains the
            // container's membership **when the attribute is present**; whether
            // the attribute may be absent at all is the C_ATTRIBUTE.existence
            // constraint. An absent (or null) attribute field is therefore not
            // a cardinality violation — an explicitly present empty container
            // (`"content": []`) is.
            let containers = path::navigate(&[instance], intermediate);
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
// implementation in [`openehr_rm::paths`], reached here via [`crate::path`]
// (`parse` / `navigate` / `select_children`). Only the checks below —
// attribute-presence for the existence rule and RM instance-path
// normalisation — are validation-specific.

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
/// closed-slot idiom (AOM 1.4 has no `is_closed`; PORT NOTE: includes then win,
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
mod tests;
