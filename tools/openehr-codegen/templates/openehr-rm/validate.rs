//! RM-level validation glue (hand-written spec behaviour).
//!
//! The module is preserved
//! across `openehr-codegen` regeneration — the generator does not emit or overwrite it, so
//! the generator's `declare_hand_written_modules` keeps it and `lib.rs`
//! auto-declares `pub mod validate;`.
//!
//! Everything here judges a canonical-JSON RM node **as a value** — no codec,
//! no template, no wire context — and reports [`InvariantViolation`]s relative
//! to that node:
//!
//! 1. **The allocation-free fast-path RM class-invariant check**
//!    ([`try_fast_validate`] → the private `fast` path) over a live canonical-JSON node, plus
//!    the **shared invariant helpers** used by the sibling `*_impl.rs`
//!    behaviour files (the DV_AMOUNT / DV_QUANTIFIED accuracy + magnitude-status
//!    rules, the LOCATABLE `Archetype_node_id_valid` rule, ISO-8601 value
//!    checks).
//! 2. **The JSON-level per-node checks the typed model cannot express** —
//!    [`check_mandatory_containers`] (model-driven container lower bounds),
//!    [`nonempty_list_violations`] (the `x /= Void implies not x.is_empty`
//!    family, over the BMM-derived rule table), [`check_archetyped_valid`] and
//!    [`check_data_structure_shapes`]. Each is a pure function over the node's
//!    own value, run as its own layer beside the fast/typed core pair so the
//!    equivalence property between those two stays exactly the core property.
//! 3. **The terminology-backed invariants**, in the sibling [`terminology`]
//!    module (they need the openEHR terminology bundle, not just the node).
//! 4. **The `553|incomplete|` presence relaxation predicates**, in the sibling
//!    [`incomplete`] module: the pair of pure model-driven questions ("is
//!    anything missing?" / "is anything wrong?") that RM common
//!    `master06-change_control_package.adoc` §Incomplete Content splits a
//!    node's structural judgement into. Used only by the relaxed
//!    (`553|incomplete|`) commit path; the strict path never calls them.
//! 5. **The typed-dispatch tier**, in the sibling [`typed_dispatch`] module:
//!    the `_type` → concrete-RM-type table that *deserializes* a node through
//!    the emitted canonical-JSON `serde` impls (`crate::json_serde`) and runs
//!    that class's `Validate` impl — the authoritative oracle the fast path may
//!    only skip when its result is provably identical.
//!
//! Kept OUT of this crate, in `openehr-its`: the GENERATED five-crate
//! structural dispatch that [`typed_dispatch::dispatch_typed`] falls through to
//! (it spans `openehr-base`/`-rm`/`-am`/`-term`/`-lang` at once, so it can only
//! be emitted downstream of all of them), the thin wire-boundary entry points
//! that compose the tiers, and the walkers that recurse an instance and prefix
//! absolute RM paths. The wire-boundary entry point
//! `openehr_its::wire_validate::validate_rm_value` calls [`try_fast_validate`]
//! then falls back to [`typed_dispatch::dispatch_typed`].
//!
//! # The invariant source and the diagnostic form
//!
//! The RM class invariants realized here are the released class tables' own
//! invariant expressions, machine-classified from the vendored BMM — the
//! generated register at the head of the crate-private `generated` module
//! (`validate/generated.rs`) is the per-invariant
//! authority (venue + citation + adjudication). A failure renders the uniform
//! message `Invariant <Name> failed on type <RM_TYPE>` (see
//! `invariant_failed`): `<Name>` is the invariant's released class-table name
//! (`docs/specs/openehr/RM/docs/UML/classes/*.adoc` §Invariants), so a
//! violation is identifiable by the spec's own vocabulary. No openEHR spec
//! governs the message wording itself — the `invariant_failed` helper below
//! is its single home.
//!
//! What deliberately does **not** run in the *core/typed* tiers:
//! - **Terminology-bound invariants** (the class-table rules that invoke
//!   `has_code_for_group_id` / `code_set (id).has_code` — `Language_valid`,
//!   `Encoding_valid`, `Category_validity`, `Change_type_valid`, …). They
//!   resolve a code against the openEHR terminology bundle rather than
//!   inspecting the node alone, so they live in the sibling
//!   [`terminology`] module (over `openehr-term`) and are run as a separate
//!   post-core layer — never inside the fast/typed pair, whose equivalence
//!   property is defined over the core invariants only.
//! - **Invariants adjudicated out of the per-node layer** — each carries a
//!   citation-pinned `Excluded` adjudication in the generated register
//!   (aggregate/cross-object rules owned by another layer, derived-function
//!   constraints, and rules over undeclared attributes).
//! - **Cross-child recursion**: each `Validate` impl checks only its own class
//!   invariants; the composition validator recurses into children (and prefixes
//!   the absolute RM path onto each [`InvariantViolation`]).

#![expect(
    clippy::disallowed_types,
    reason = "the wire-boundary validation reads the canonical JSON node before the typed decode \
              (#1694 boundary class)"
)]

use openehr_base::validate::InvariantViolation;
use serde_json::Value;

mod fast;
/// The generated RM class-invariant cores (`openehr-codegen -- emit-validate`):
/// one `pub(crate) fn <name>_core` per mechanically-shaped invariant group, the
/// single source both the typed `Validate` impls and [`fast`] call. This is the
/// ONE hand-declared module for that `// @generated` file — the runtime helpers
/// the cores call (`invariant_failed`, the ISO-8601 validators, the dialect
/// predicates) stay hand-written below.
pub(crate) mod generated;
// The `553|incomplete|` presence relaxation predicates (its own `//!` module
// docs carry the detail — an outer doc attribute here would force rustdoc to
// resolve the module's intra-doc links in THIS module's scope instead of its
// own).
pub mod incomplete;
// The terminology-backed RM class invariants (its own `//!` module docs carry
// the detail — an outer doc attribute here would force rustdoc to resolve the
// module's intra-doc links in THIS module's scope instead of its own).
pub mod terminology;
// The typed-dispatch tier (its own `//!` module docs carry the detail — an
// outer doc attribute here would force rustdoc to resolve the module's
// intra-doc links in THIS module's scope instead of its own).
pub mod typed_dispatch;

/// Run the allocation-free fast-path RM class-invariant check for a single
/// canonical-JSON node, dispatching on its `_type`.
///
/// Returns `true` when the fast path vouched for (fully handled) the node —
/// nothing is appended on `false`.
///
/// This is the public seam the wire-boundary two-tier dispatcher
/// (`openehr_its::wire_validate::validate_rm_value`) calls before falling back to
/// the typed deserialize path. Kept here because the fast path is untyped
/// (walks `&serde_json::Value` against the generated RM model) and needs no
/// canonical-JSON codec — pure RM model semantics.
///
/// NOTE: no openEHR spec governs the fast path — it is our own performance
/// design; the *semantics* it realizes are exactly the RM class invariants of
/// the `*_impl.rs` siblings (see the private `fast` module).
#[must_use]
pub fn try_fast_validate(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) -> bool {
    fast::try_validate(ty, value, out)
}

/// The inherited LOCATABLE `Archetype_node_id_valid` violation for a
/// canonical-JSON node whose RM type is `ty`, or `None` when the node does not
/// violate it.
///
/// `LOCATABLE.Archetype_node_id_valid` (`not archetype_node_id.is_empty`,
/// `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc`
/// §Invariants) is inherited by **every** concrete LOCATABLE descendant, so the
/// set of RM types it applies to is read from the generated static RM model
/// ([`crate::v1_2::model::descendants`] of `LOCATABLE` — the transitive *concrete*
/// descendant closure of the BMM), never from a hand-maintained list: a class
/// the spec adds to the hierarchy is covered the moment the model is
/// regenerated. The violation itself is built by the generated core
/// (`generated::archetype_node_id_core`), so its text has one source.
///
/// Only a **present and empty** `archetype_node_id` violates the invariant. An
/// absent (or non-string) one is a structural defect of a mandatory attribute,
/// reported by the decode/mandatory-attribute layer instead — reporting it here
/// too would double-report the same defect.
///
/// The wire-boundary dispatcher (`openehr_its::wire_validate`) runs this for
/// every node after its class-invariant tier, appending only what that tier did
/// not already report (the concrete LOCATABLEs with a typed `Validate` impl
/// realize the inherited invariant themselves).
#[must_use]
pub fn locatable_node_id_violation(ty: &str, value: &Value) -> Option<InvariantViolation> {
    // Cheap first: the overwhelming majority of nodes carry a non-empty id, so
    // the model lookup is paid only on an actual violation candidate.
    if value.get("archetype_node_id").and_then(Value::as_str) != Some("") {
        return None;
    }
    if !crate::v1_2::model::descendants("LOCATABLE").contains(&ty) {
        return None;
    }
    let mut out = Vec::with_capacity(1);
    generated::archetype_node_id_core(ty, "", &mut out);
    out.pop()
}

/// The model-driven mandatory-container lower-bound check.
///
/// Every attribute the
/// static RM model declares as a MANDATORY container must be present, and one
/// whose BMM cardinality has a lower bound ≥ 1 must be non-empty — e.g.
/// `CLUSTER.items: List<ITEM>` is `1..*`
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_structures.cluster.adoc`
/// §Attributes), so a CLUSTER with an absent or empty `items` does not conform.
///
/// This is a distinct mechanism from the codec decode: a MANDATORY container
/// emits as a bare `Vec<T>` and the canonical-JSON reader treats an absent
/// array as an empty `Vec` (wire tolerance), so the omission is invisible to
/// typed decoding — the lower bound is enforced HERE, from the model, for every
/// class uniformly. The OPTIONAL-attribute family (`x /= Void implies not
/// x.is_empty`, e.g. `COMPOSITION.content`) is a different rule with a different
/// evaluator ([`nonempty_list_violations`]); this function only judges MANDATORY
/// containers, so the two never double-report.
/// `List<Octet>` is exempt by shape: canonical JSON renders it as an inline
/// base64 string, which presence-checks as a non-array member.
///
/// Kept OUTSIDE the fast/typed core pair (the caller runs it as its own layer),
/// so the fast-vs-typed equivalence property stays exactly the core property.
pub fn check_mandatory_containers(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    for attr in crate::v1_2::model::attributes(ty) {
        if !attr.is_mandatory
            || matches!(attr.container, crate::v1_2::model::Container::None)
            || attr.declared_type == "Octet"
        {
            continue;
        }
        match value.get(attr.name) {
            None | Some(Value::Null) => {
                out.push(InvariantViolation::here(format!(
                    "does not conform to RM type {ty}: mandatory container `{}` is absent",
                    attr.name
                )));
            }
            Some(Value::Array(a))
                if a.is_empty() && attr.cardinality.is_some_and(|c| c.lower >= 1) =>
            {
                out.push(InvariantViolation::here(format!(
                    "does not conform to RM type {ty}: mandatory container `{}` is empty \
                     (cardinality lower bound 1)",
                    attr.name
                )));
            }
            _ => {}
        }
    }
}

/// The `x /= Void implies not x.is_empty` invariant family for one node.
///
/// Every rule of the generated `NONEMPTY_LIST_RULES` table (in the private
/// `generated` module) that applies to `ty` is evaluated against the node's own
/// attributes.
///
/// The rule table is READ FROM THE BMM (every class invariant with that exact
/// assertion shape over a container attribute), and a rule applies to its
/// declaring class **and its transitive concrete descendants** — resolved from
/// the generated static RM model ([`crate::v1_2::model::descendants`]), never from a
/// hand-maintained list — so `LOCATABLE.Links_valid`,
/// `DV_ORDERED.Other_reference_ranges_validity` and
/// `ENTRY.Other_participations_valid` reach every descendant, and a class the
/// spec adds to the hierarchy is covered the moment the model is regenerated.
///
/// The optional-container emission shape (`Option<Vec<T>>`) is what makes the
/// family decidable at all: the forbidden state is the attribute PRESENT with
/// zero members, which the canonical-JSON reader now preserves
/// (`openehr_its::json_codec::runtime::optional_container_field`). This
/// evaluator reads it off the node, which is the same value the typed model
/// carries.
///
/// Kept OUTSIDE the fast/typed core pair (the caller runs it as its own layer,
/// exactly like [`check_mandatory_containers`]), so the fast-vs-typed
/// equivalence property stays exactly the core property and no rule can be
/// reported twice.
///
/// Every violation is reported on the node itself (an empty
/// [`InvariantViolation::path`]); the caller prefixes the absolute RM path.
pub fn nonempty_list_violations(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    for (class, attribute, invariant) in generated::NONEMPTY_LIST_RULES {
        if *class != ty && !crate::v1_2::model::descendants(class).contains(&ty) {
            continue;
        }
        let present_and_empty = value
            .get(*attribute)
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty);
        generated::nonempty_list_core(ty, attribute, invariant, present_and_empty, out);
    }
}

/// The declared-slot-type conformance rule, over the BMM-generated attribute
/// model.
///
/// A child node's wire `_type` must name the RM type the parent
/// attribute declares, or a subtype of it (`docs/specs/openehr/ITS-JSON`
/// discipline: `_type` names the instance's RM class; the attribute's
/// declared type comes from the RM UML/BMM — e.g. RM ehr
/// `composition.adoc` §Attributes types `content` `List<CONTENT_ITEM>`, so a
/// `DV_TEXT` member of `content` is a positive type contradiction). This is
/// the WRONGNESS half of slot typing — it never relaxes ("data may be
/// missing, but it may not be wrong", RM common
/// `master06-change_control_package.adoc` §Incomplete Content); the
/// presence/lower-bound half lives in [`check_mandatory_containers`] /
/// [`nonempty_list_violations`].
///
/// Returns `None` (no judgement) when the slot is unknown to the model, the
/// declared type is not a modelled class (a primitive such as `String`), or
/// the attribute is a keyed map (`Hash` — its JSON object is a map, not an
/// RM node). An untagged child is judged elsewhere (the effective-type rule,
/// [`crate::v1_2::model::declared_concrete_type`]).
#[must_use]
pub fn check_declared_slot_type(
    parent_type: &str,
    field: &str,
    wire_type: &str,
) -> Option<InvariantViolation> {
    let attr = crate::v1_2::model::attribute(parent_type, field)?;
    if matches!(attr.container, crate::v1_2::model::Container::Hash) {
        return None;
    }
    crate::v1_2::model::class(attr.declared_type)?;
    if crate::v1_2::model::is_a(wire_type, attr.declared_type) {
        return None;
    }
    Some(InvariantViolation::here(format!(
        "does not conform to RM type {parent_type}: `{field}` is declared \
         {declared} and this member claims `_type` {wire_type}, which is not \
         a {declared}",
        declared = attr.declared_type,
    )))
}

/// The scalar-member arm of the declared-slot-type rule.
///
/// A NON-OBJECT member
/// of a list slot whose declared element type is a modelled RM class is the
/// same positive contradiction a foreign `_type` is — no JSON scalar can be
/// an instance of an RM class (canonical JSON encodes every RM object as a
/// JSON object; ITS-JSON). Same guards as [`check_declared_slot_type`]:
/// `None` for unknown slots, primitive-typed slots (a `List<String>` member
/// IS legitimately a scalar), and keyed maps. Never relaxed.
#[must_use]
pub fn check_slot_member_is_object(parent_type: &str, field: &str) -> Option<InvariantViolation> {
    let attr = crate::v1_2::model::attribute(parent_type, field)?;
    if matches!(attr.container, crate::v1_2::model::Container::Hash) {
        return None;
    }
    crate::v1_2::model::class(attr.declared_type)?;
    Some(InvariantViolation::here(format!(
        "does not conform to RM type {parent_type}: `{field}` is declared \
         {declared} and this member is not a JSON object",
        declared = attr.declared_type,
    )))
}

/// `LOCATABLE.Archetyped_valid`: `is_archetype_root xor archetype_details =
/// Void`
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc`
/// §Invariants).
///
/// The enforceable arm on an instance is: a **non-root** node — one whose
/// `archetype_node_id` is an `at`/`id` term code
/// ([`crate::v1_2::paths::archetype_node_id_is_term_code`]), which per the node-id
/// format can never be the root of an archetyped structure — must NOT carry
/// `archetype_details`.
///
/// NOTE: the converse arm ("an archetype-HRID node must carry
/// `archetype_details`") is NOT enforced, because the invariant's own
/// operand is undefined: `locatable.adoc` §Functions gives
/// `is_archetype_root ()` a Meaning sentence only ("True if this node is
/// the root of an archetyped structure") — no postcondition, no derivation
/// expression. Under the reference-object-model reading, where
/// `is_archetype_root` IS the presence of `archetype_details`, the
/// invariant is a tautology and the converse arm asserts nothing; only
/// under the node-id reading would it mandate `archetype_details` on every
/// archetype-HRID node, and the released text does not choose between the
/// two. An arm that is underivable from the released text is not
/// enforceable, so it is reported rather than gated. Corroboration, not the
/// ground:
/// the CNF valid data sets and the canonical-JSON corpus systematically
/// omit `archetype_details` on nested archetype roots. The COMPOSITION
/// root arm stays separately enforced (`composition_impl.rs`
/// `Is_archetype_root`).
///
/// The second arm is the root node-id **identity** rule: a node that DOES
/// carry `archetype_details` is an archetype root, and
/// `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc`
/// §Attributes (`archetype_node_id`) fixes its value verbatim — "At an
/// archetype root point, the value of this attribute is always the
/// stringified form of the `archetype_id` found in the `archetype_details`
/// object" (restated in
/// `docs/specs/openehr/RM/docs/common/master03-archetyped_package.adoc`
/// §The LOCATABLE Class: "the only exception is at archetype root points in
/// data, where `archetype_node_id` carries the archetype identifier in
/// string form"). A root whose two archetype identities disagree names no
/// resolvable generating archetype.
///
/// Both violations are reported on the node itself (an empty
/// [`InvariantViolation::path`]); the caller prefixes the absolute RM path.
#[must_use]
pub fn check_archetyped_valid(
    node_id: Option<&str>,
    has_archetype_details: bool,
    details_archetype_id: Option<&str>,
) -> Vec<InvariantViolation> {
    let mut out = Vec::new();
    let Some(node_id) = node_id else {
        return out;
    };
    if crate::v1_2::paths::archetype_node_id_is_term_code(node_id) && has_archetype_details {
        out.push(InvariantViolation::here(format!(
            "node {node_id:?} is not an archetype root (at/id term code) and must \
             not carry archetype_details (LOCATABLE.Archetyped_valid)"
        )));
    }
    if let Some(archetype_id) = details_archetype_id
        && archetype_id != node_id
    {
        out.push(InvariantViolation::here(format!(
            "archetype root archetype_node_id {node_id:?} is not the stringified \
             archetype_details.archetype_id {archetype_id:?} — at an archetype root \
             the two are always the same value (LOCATABLE.archetype_node_id)"
        )));
    }
    out
}

/// The `CLUSTER.items` PRESENCE duty.
///
/// It is split out of
/// [`check_data_structure_shapes`] because it is a mandatory-presence rule and
/// therefore the one shape duty the `553|incomplete|` state relaxes (RM common
/// `master06-change_control_package.adoc` §Incomplete Content: "container
/// attributes may be empty, even though they may have minimum existence and
/// cardinality respectively of one").
///
/// `CLUSTER.items` is 1..1 (RM `data_structures` `cluster.adoc`; the ITS-JSON
/// CLUSTER schema lists `items` as required) — after deserialize an absent
/// list collapses into an empty `Vec`, so presence is only checkable here.
/// Reported on the node itself; the caller prefixes the absolute RM path.
#[must_use]
pub fn check_cluster_items_present(
    obj: &serde_json::Map<String, Value>,
    ty: Option<&str>,
) -> Option<InvariantViolation> {
    (ty == Some("CLUSTER") && obj.get("items").and_then(Value::as_array).is_none()).then(|| {
        InvariantViolation::here("CLUSTER.items is mandatory (1..1 List<ITEM>, cluster.adoc)")
    })
}

/// JSON-level data-structure shape duties the typed model cannot express:
///
/// - one `HISTORY`'s events all carry the SAME `ITEM_STRUCTURE` subtype
///   in `data` — "A History of type `HISTORY<ITEM_LIST>` … constrains the
///   type of the data at each Event to be of type `ITEM_LIST` and nothing
///   else" (`docs/specs/openehr/RM/docs/data_structures/master06-history_package.adoc`;
///   `history.adoc` generic parameter) — the monomorphized runtime type
///   cannot see `T`.
///
/// The `CLUSTER.items` presence duty is [`check_cluster_items_present`]'s (it
/// is a mandatory-presence rule, so the incomplete-commit path relaxes it while
/// keeping every duty here). A HISTORY violation carries the `events[i]`
/// sub-path of the offending event; the caller prefixes the absolute RM path.
#[must_use]
pub fn check_data_structure_shapes(
    obj: &serde_json::Map<String, Value>,
    ty: Option<&str>,
) -> Vec<InvariantViolation> {
    let mut out = Vec::new();
    let Some(ty) = ty else {
        return out;
    };
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
                    out.push(InvariantViolation::at(
                        format!("events[{i}]"),
                        format!(
                            "HISTORY events must all carry the same ITEM_STRUCTURE \
                             subtype in data — the history is HISTORY<{locked}> but \
                             this event carries {data_ty} (RM data_structures \
                             master06 §History)"
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
    }
    out
}

/// Build a class-invariant violation in this workspace's uniform message
/// shape: `"Invariant <name> failed on type <RM_TYPE>"`, where `<name>` is the
/// invariant's own BMM name, so a failure is identifiable by that name alone.
///
/// NOTE: no openEHR spec governs the wording of a violation message — the
/// `Invariant <Name> failed on type <RM_TYPE>` shape is our own design, and only
/// `<Name>`/`<RM_TYPE>` are the spec's. The path is left empty (the value
/// itself); the composition validator prefixes the absolute RM path.
#[must_use]
pub(crate) fn invariant_failed(name: &str, rm_type: &str) -> InvariantViolation {
    InvariantViolation::here(format!("Invariant {name} failed on type {rm_type}"))
}

/// `true` when a floating value denotes a whole number — the integrality
/// probe the DV_PROPORTION `Precision_validity` / `Is_integral_validity` /
/// `Fraction_validity` invariants need, which state integrality as
/// `numerator.floor = numerator and denominator.floor = denominator`
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_proportion.adoc`
/// §Invariants).
#[must_use]
#[expect(
    clippy::float_cmp,
    reason = "an exact-integrality test is precisely a bit-equality question (`x.floor() == x`), not a tolerance comparison"
)]
pub(crate) fn is_integral(v: f64) -> bool {
    v.is_finite() && v.floor() == v
}

// ── named runtime realizations of the BMM assertion-dialect predicates ────────
//
// These are the callable runtime helpers the assertion-dialect emitter maps its
// leaf predicates onto (the `plan::overrides` dialect table names each). They
// were previously inlined into the invariant cores below; extracting them under
// the BMM predicate spelling makes the emitter's future generated cores call one
// named runtime function per dialect predicate. Behaviour is identical to the
// former inline forms.

/// BASE/RM `valid_magnitude_status (s)`: `s` is one of `= < > <= >= ~` — the
/// DV_QUANTIFIED `Magnitude_status_valid` predicate
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_quantified.adoc`).
#[must_use]
pub(crate) fn valid_magnitude_status(s: &str) -> bool {
    matches!(s, "=" | "<" | ">" | "<=" | ">=" | "~")
}

/// RM `valid_percentage (v)`: `0 <= v <= 100` — the DV_AMOUNT `Accuracy_validity`
/// predicate for a percent-recorded accuracy
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_amount.adoc`).
#[must_use]
pub(crate) fn valid_percentage(v: f64) -> bool {
    (0.0..=100.0).contains(&v)
}

/// RM `valid_proportion_kind (k)`: `k` is one of the PROPORTION_KIND codes —
/// the DV_PROPORTION `Type_validity` predicate
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.proportion_kind.adoc`).
///
/// The value space is read from the emitted constant set, whose
/// `from_value` maps anything the `BMM_ENUMERATION` does not declare to
/// `Other`, so a re-vendoring that adds a constant widens this predicate with
/// it rather than leaving a hand-written range behind.
#[must_use]
pub(crate) fn valid_proportion_kind(k: i32) -> bool {
    !matches!(
        crate::v1_2::data_types::quantity::proportion_kind::ProportionKind::from_value(k),
        crate::v1_2::data_types::quantity::proportion_kind::ProportionKind::Other(_)
    )
}

// ── ISO-8601 value validation ────────────────────────────────────────────────

// The DV_DATE / DV_TIME / DV_DATE_TIME / DV_DURATION `Value_valid` invariants
// are `valid_iso8601_date` / `_time` / `_date_time` / `_duration` (each class
// page's §Invariants), and those are BASE `Time_Definitions` functions — so
// this crate calls them rather than re-deriving the grammar. It used to carry
// its own reader; the two drifted twice and both drifts shipped (#2273), each
// time letting a value pass validation, commit, and then behave as invalid.

pub(crate) use openehr_base::v1_3::foundation_types::time::time_definitions::{
    valid_iso8601_date, valid_iso8601_date_time, valid_iso8601_duration, valid_iso8601_time,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_date_forms() {
        assert!(valid_iso8601_date("2021"));
        assert!(valid_iso8601_date("2021-05"));
        assert!(valid_iso8601_date("2021-05-17"));
        assert!(valid_iso8601_date("20210517"));
        assert!(!valid_iso8601_date("2021-13"));
        assert!(!valid_iso8601_date("2021-05-32"));
        assert!(!valid_iso8601_date("not-a-date"));
        assert!(!valid_iso8601_date(""));
    }

    /// BASE `Iso8601_date.Day_valid` (`valid_day = d <= days_in_month(m, y)`,
    /// `iso8601_date.adoc` line 107). Calendar-exact month lengths, both the
    /// extended (`YYYY-MM-DD`) and compact (`YYYYMMDD`) forms.
    #[test]
    fn iso_date_day_is_calendar_exact() {
        // 31-day months accept 31; 30-day months reject it.
        assert!(valid_iso8601_date("2021-01-31"));
        assert!(valid_iso8601_date("2021-12-31"));
        assert!(!valid_iso8601_date("2021-04-31")); // April has 30 days
        assert!(!valid_iso8601_date("2021-06-31"));
        assert!(!valid_iso8601_date("2021-09-31"));
        assert!(!valid_iso8601_date("2021-11-31"));
        assert!(valid_iso8601_date("2021-04-30"));

        // February: 28 in a common year, 29 in a leap year, never 30/31.
        assert!(!valid_iso8601_date("2021-02-31"));
        assert!(!valid_iso8601_date("2021-02-30"));
        assert!(!valid_iso8601_date("2021-02-29")); // 2021 is not a leap year
        assert!(valid_iso8601_date("2021-02-28"));
        assert!(valid_iso8601_date("2020-02-29")); // 2020 divisible by 4
        assert!(valid_iso8601_date("2000-02-29")); // 2000 divisible by 400
        assert!(!valid_iso8601_date("1900-02-29")); // 1900 century, not /400

        // Day 00 is never valid.
        assert!(!valid_iso8601_date("2021-05-00"));

        // Compact form is held to the same calendar rule.
        assert!(!valid_iso8601_date("20210431"));
        assert!(!valid_iso8601_date("20210229"));
        assert!(valid_iso8601_date("20200229"));
        assert!(valid_iso8601_date("20210131"));
    }

    #[test]
    fn iso_time_forms() {
        assert!(valid_iso8601_time("10"));
        assert!(valid_iso8601_time("10:30"));
        assert!(valid_iso8601_time("10:30:59"));
        assert!(valid_iso8601_time("10:30:59.250"));
        assert!(valid_iso8601_time("10:30:59Z"));
        assert!(valid_iso8601_time("10:30:59+01:00"));
        assert!(!valid_iso8601_time("25:00"));
        assert!(!valid_iso8601_time("10:61"));
        assert!(!valid_iso8601_time("abc"));
    }

    /// BASE `foundation_types/master06-time_types.adoc` §"ISO 8601 semantics
    /// not included in these types": "partial date/times with fractional
    /// minutes or hours … in openEHR, only fractional seconds are supported".
    /// A fraction is accepted only on full `HH:MM:SS` (seconds present);
    /// fractional hours/minutes are rejected.
    #[test]
    fn iso_time_fraction_only_on_seconds() {
        // Fractional seconds (period or comma) is the sole permitted case.
        assert!(valid_iso8601_time("10:30:59.250"));
        assert!(valid_iso8601_time("10:30:59,5"));
        assert!(valid_iso8601_time("103059.250")); // compact HHMMSS
        assert!(valid_iso8601_time("10:30:59.5+01:00")); // fraction before timezone
        // A fractional hour or minute is rejected in every base form.
        assert!(!valid_iso8601_time("10.5")); // fractional hour
        assert!(!valid_iso8601_time("10:05.5")); // fractional minute (extended)
        assert!(!valid_iso8601_time("1005.5")); // fractional minute (compact HHMM)
        assert!(!valid_iso8601_time("10,5")); // fractional hour, comma
    }

    #[test]
    fn iso_date_time_forms() {
        assert!(valid_iso8601_date_time("2021-05-17T10:30:00"));
        assert!(valid_iso8601_date_time("2021-05-17T10:30:00+02:00"));
        assert!(valid_iso8601_date_time("2021-05-17"));
        assert!(!valid_iso8601_date_time("2021-05-17T99:00"));
        assert!(!valid_iso8601_date_time("nope"));
    }

    #[test]
    fn iso_duration_forms() {
        assert!(valid_iso8601_duration("P1Y"));
        assert!(valid_iso8601_duration("P1Y2M10D"));
        assert!(valid_iso8601_duration("PT2H30M"));
        assert!(valid_iso8601_duration("P1Y2M10DT2H30M"));
        assert!(valid_iso8601_duration("P2W"));
        assert!(valid_iso8601_duration("-P1D"));
        assert!(valid_iso8601_duration("PT0.5S"));
        assert!(!valid_iso8601_duration("P"));
        assert!(!valid_iso8601_duration("1Y"));
        assert!(!valid_iso8601_duration("P1X"));
        assert!(!valid_iso8601_duration("PT"));
    }

    /// This validator and BASE's `Iso8601Time` reader must accept the same
    /// strings: `valid_second` bounds seconds below 60, and a value that passes
    /// here but not there is stored with no magnitude and no arithmetic.
    #[test]
    fn time_validity_agrees_with_the_base_reader() {
        for value in ["10:30:00", "10:30:59", "23:59:59", "10:30:60", "103060", "25:00:00"] {
            let base = openehr_base::v1_3::foundation_types::time::iso8601_time::Iso8601Time {
                value: value.to_owned(),
            }
            .hour()
            .is_some();
            assert_eq!(
                valid_iso8601_time(value),
                base,
                "{value:?}: the RM validator and the BASE reader disagree",
            );
        }
    }

    /// This validator and BASE's `Iso8601Duration` reader must accept the same
    /// strings, or a value passes `Value_valid`, gets stored, and then has no
    /// arithmetic. `P1Y1Y` validated here while BASE refused it and
    /// `iso_duration_to_seconds` summed BOTH years — one value, three in-tree
    /// answers, none of them the production's.
    #[test]
    fn duration_validity_agrees_with_the_base_reader() {
        for value in [
            "P1Y",
            "P1Y2M10DT2H30M",
            "P2W",
            "PT0.5S",
            "P1Y1Y",
            "P2Y1Y",
            "P1D1M",
            "PT1S1H",
            "P1YT",
            "PT1H2H",
            "P1X",
            "PT",
        ] {
            let base = openehr_base::v1_3::foundation_types::time::iso8601_duration::Iso8601Duration {
                value: value.to_owned(),
            }
            .to_seconds()
            .is_some();
            assert_eq!(
                valid_iso8601_duration(value),
                base,
                "{value:?}: the RM validator and the BASE reader disagree",
            );
        }
    }

    /// BASE `master06-time_types.adoc` §Primitive Time Types: "in openEHR, only
    /// fractional seconds are supported" — a decimal fraction on any component
    /// other than seconds is invalid, even though the pattern of designators is
    /// otherwise well-formed.
    #[test]
    fn iso_duration_fraction_only_on_seconds() {
        // Fraction on seconds (period or comma) is the sole permitted case.
        assert!(valid_iso8601_duration("PT2H30M0.5S"));
        assert!(valid_iso8601_duration("PT0,5S"));
        // Fraction on any other component is rejected.
        assert!(!valid_iso8601_duration("P1Y3M4DT2.5H"));
        assert!(!valid_iso8601_duration("PT2H14.5M"));
        assert!(!valid_iso8601_duration("P1.5Y"));
        assert!(!valid_iso8601_duration("P1.5M"));
        assert!(!valid_iso8601_duration("P1.5W"));
        assert!(!valid_iso8601_duration("P1.5D"));
        assert!(!valid_iso8601_duration("PT1.5H"));
        assert!(!valid_iso8601_duration("PT2H14,5M"));
    }

    // NOTE: the `_type`-dispatch tests live in
    // `openehr-its/tests/it/rm_validation.rs`, where the tiers are reachable
    // through the composed wire entry points.

    /// BASE `Iso8601_timezone`: `+` offsets reach +14:00, `-` offsets stop at
    /// -12:00; ±00:00 is accepted (see `is_valid_tz`).
    #[test]
    fn timezone_bounds_are_asymmetric() {
        assert!(valid_iso8601_time("10:00:00+14:00"));
        assert!(valid_iso8601_time("10:00:00-12:00"));
        assert!(valid_iso8601_time("10:00:00+00:00"));
        assert!(valid_iso8601_time("10:00:00-00:00"));
        assert!(
            !valid_iso8601_time("10:00:00+15:00"),
            "+15 exceeds Max_timezone_hour"
        );
        assert!(
            !valid_iso8601_time("10:00:00-13:00"),
            "-13 exceeds Min_timezone_hour"
        );
    }
}
