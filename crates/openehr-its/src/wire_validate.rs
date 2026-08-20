// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: MIT AND Apache-2.0

//! The **wire-boundary RM class-invariant dispatcher** — the thin entry points
//! that compose the validation tiers at the codec boundary.
//!
//! The tiers themselves are RM model semantics and live upstream in
//! [`openehr_rm::v1_2::validate`]: the allocation-free fast path
//! ([`openehr_rm::v1_2::validate::try_fast_validate`]) and the authoritative typed
//! dispatch ([`openehr_rm::v1_2::validate::typed_dispatch::dispatch_typed`], the
//! `_type` → concrete-RM-type table). What stays HERE is what genuinely needs
//! this crate:
//!
//! - the GENERATED five-crate structural fallthrough
//!   ([`structural_check`]) the typed table declines to — it spans
//!   `openehr-base`/`-rm`/`-am`/`-term`/`-lang` at once, so it can only be
//!   emitted downstream of all of them;
//! - the undeclared-key refusal, which reads the generated declared-key table
//!   ([`declared_fields`]) and renders the reader's own
//!   [`crate::json::JsonParseError`];
//! - the entry points themselves ([`validate_rm_value`],
//!   [`validate_rm_invariants`], [`validate_rm_invariants_as`],
//!   [`validate_rm_invariants_relaxed_as`], [`validate_rm_value_typed`]), which
//!   fix the ORDER the tiers and orthogonal layers run in.
//!
//! # The invariant source
//!
//! The RM class invariants live upstream (`openehr-rm`'s generated cores +
//! `*_impl.rs`, per the released class tables' §Invariants — the generated
//! register is the per-invariant authority); this module only routes a node
//! to the right tier. A node that does not deserialize into its declared concrete RM type
//! surfaces `does not conform to RM type …` (see
//! [`openehr_rm::v1_2::validate::typed_dispatch::record_type_mismatch`]).

#![expect(
    clippy::disallowed_types,
    reason = "canonical JSON / Simplified Formats operate on the wire value by definition \
              (ITS-JSON; ITS-REST Simplified Formats) — serde_json::Value IS the subject matter \
              (#1694)"
)]

use serde_json::Value;

use openehr_base::validate::InvariantViolation;
use openehr_rm::v1_2::validate::typed_dispatch::record_type_mismatch;

use crate::json::JsonParseError;
use crate::json_codec::generated::structural::{declared_fields, structural_check};

/// Run the **core** (non-terminology) RM class invariants for a single
/// canonical-JSON node, dispatching on its `_type`.
///
/// A node with no (or an unrecognised) `_type` runs no invariants (returns
/// without appending).
///
/// Two tiers (performance: the RM-invariant pass visits every `_type` node of a
/// commit, ~1.5k for a populated composition, so the per-node cost is
/// load-bearing):
///
/// 1. the **fast path** ([`openehr_rm::v1_2::validate::try_fast_validate`]) verifies
///    structural conformance directly against the live JSON node using the
///    generated static RM model and runs the class invariants through the same
///    cores the typed impls call — no deserialization, no allocation,
///    byte-identical output;
/// 2. anything the fast path cannot vouch for falls back to the authoritative
///    **typed dispatch** below ([`validate_rm_value_typed`]), which
///    deserializes into the concrete RM type (surfacing `does not conform to
///    RM type …` for a structural mismatch) and runs the typed invariants.
///
/// This is the tier the fast-path equivalence battery pins (the fast path must
/// vouch only when byte-identical to the typed oracle). The terminology-backed
/// invariants ([`openehr_rm::v1_2::validate::terminology`]) are an orthogonal layer
/// added by [`validate_rm_value`], not part of the fast/typed equivalence.
pub fn validate_rm_invariants(value: &Value, out: &mut Vec<InvariantViolation>) {
    let Some(ty) = value.get("_type").and_then(Value::as_str) else {
        return;
    };
    validate_rm_invariants_as(ty, value, out);
}

/// [`validate_rm_invariants`] with the node's RM type supplied by the caller.
///
/// This is the entry a tree walker uses for a node whose wire `_type` is
/// legitimately absent (canonical JSON requires `_type` only where the declared
/// attribute type is abstract; see
/// [`openehr_rm::v1_2::model::declared_concrete_type`]). The `_type`-reading
/// wrapper stays for callers with no declaration context.
pub fn validate_rm_invariants_as(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    // The strict reader refuses an undeclared wire key, so a node carrying one
    // is not a conformant instance — and the FAST path never decodes, so it
    // would not see it. Checking here, against the generated declared-key table
    // (`declared_fields`, emitted from the SAME field view as the reader's own
    // refusal), keeps the two tiers byte-identical without paying for a decode.
    if let Some(violation) = undeclared_key(ty, value) {
        out.push(violation);
        return;
    }
    if !openehr_rm::v1_2::validate::try_fast_validate(ty, value, out) {
        validate_rm_value_typed(ty, value, out);
    }
}

/// [`validate_rm_invariants_as`] under the `553|incomplete|` **presence
/// relaxation** — the entry the relaxed commit path uses per node.
///
/// RM common `master06-change_control_package.adoc` §Incomplete Content
/// (NOTE): "In the `incomplete` state, a limited form of invalidity is
/// allowed: mandatory attributes may be absent … All other validity
/// requirements must be satisfied. In other words, in an `incomplete` commit,
/// data may be missing, but it may not be wrong."
///
/// The three branches realize that sentence exactly:
///
/// 1. an **undeclared member** is wrong, never missing — refused first, as on
///    the strict path;
/// 2. a node with **nothing missing** ([`mandatory_data_present`](openehr_rm::v1_2::validate::incomplete::mandatory_data_present))
///    runs the strict tiers unchanged: the relaxation costs it no strictness;
/// 3. a node that **is** missing mandatory data does not have its TYPED
///    construction driven — that tier's refusal IS the presence rule the state
///    lifts (the generated types make existence and cardinality lower bounds
///    structural). Wrongness is still judged: a positive type contradiction
///    ([`contradicts_rm_type`](openehr_rm::v1_2::validate::incomplete::contradicts_rm_type))
///    is reported through the authoritative typed tier, and the class
///    invariants run wherever the fast path can still vouch for the node as it
///    stands.
///
/// The orthogonal layers the caller runs beside this one — terminology,
/// `LOCATABLE.Archetyped_valid`, the data-structure shape duties, and the
/// archetype-conformance pass — are NOT relaxed here; only the
/// mandatory-presence and cardinality-lower-bound layers are
/// (`openehr_rm::v1_2::validate::check_mandatory_containers`,
/// which the relaxed walker skips).
pub fn validate_rm_invariants_relaxed_as(
    ty: &str,
    value: &Value,
    out: &mut Vec<InvariantViolation>,
) {
    if let Some(violation) = undeclared_key(ty, value) {
        out.push(violation);
        return;
    }
    if openehr_rm::v1_2::validate::incomplete::mandatory_data_present(ty, value) {
        if !openehr_rm::v1_2::validate::try_fast_validate(ty, value, out) {
            validate_rm_value_typed(ty, value, out);
        }
        return;
    }
    if openehr_rm::v1_2::validate::incomplete::contradicts_rm_type(ty, value) {
        validate_rm_value_typed(ty, value, out);
        return;
    }
    // The node is missing mandatory data and is not wrong: its class
    // invariants are evaluated where the fast path can still read them off the
    // node as it stands, and a decline is the honest answer — an invariant
    // over an absent attribute has nothing to judge, which is exactly the
    // "data may be missing" case §Incomplete Content admits.
    let _vouched = openehr_rm::v1_2::validate::try_fast_validate(ty, value, out);
}

/// The violation for the first undeclared member of `value` under class `ty`,
/// or `None` when every member is declared (or `ty` names no emitted struct).
///
/// Byte-identical to what the typed tier reports for the same node: it builds
/// the reader's own error and runs it through [`record_type_mismatch`].
fn undeclared_key(ty: &str, value: &Value) -> Option<InvariantViolation> {
    let members = value.as_object()?;
    let declared = declared_fields(ty)?;
    let offending = members
        .keys()
        .find(|k| k.as_str() != "_type" && declared.binary_search(&k.as_str()).is_err())?;
    let mut out = Vec::new();
    record_type_mismatch(
        ty,
        &JsonParseError::unknown_field(offending, ty, declared).in_field(offending),
        &mut out,
    );
    out.pop()
}

/// Runs **all** RM class invariants for a single canonical-JSON node.
///
/// The set is the core
/// (fast/typed) invariants ([`validate_rm_invariants`]) plus the
/// terminology-backed invariants
/// ([`openehr_rm::v1_2::validate::terminology::validate_rm_terminology`], the
/// openEHR-terminology group and code-set membership checks the generated
/// invariant cores cannot express mechanically). This is the unified dispatcher
/// every consumer calls; the composition validator invokes it per node and
/// prefixes the absolute RM path onto each [`InvariantViolation`].
pub fn validate_rm_value(value: &Value, out: &mut Vec<InvariantViolation>) {
    let Some(ty) = value.get("_type").and_then(Value::as_str) else {
        return;
    };
    // The core path (fast/typed), then the two orthogonal layers — the
    // model-driven mandatory-container lower bounds and the
    // terminology-backed invariants
    // (each dispatches on the same `_type`). The core tiers stay a pure pair so
    // the fast-vs-typed equivalence property holds exactly.
    validate_rm_invariants_as(ty, value, out);
    openehr_rm::v1_2::validate::check_mandatory_containers(ty, value, out);
    openehr_rm::v1_2::validate::terminology::validate_rm_terminology(ty, value, out);
}

/// The typed dispatch tier of [`validate_rm_value`]: deserialize the node
/// into its concrete RM type and run that type's `Validate` impl.
///
/// Authoritative for every node (the fast path may only *skip* it when its
/// result is provably identical); also the oracle the fast-path equivalence
/// tests compare against.
///
/// The tier is composed here from three parts, in this exact order:
///
/// 1. the **undeclared-key refusal**, ahead of any decode (the private
///    `undeclared_key` walk over [`declared_fields`]);
/// 2. the **typed table**
///    ([`openehr_rm::v1_2::validate::typed_dispatch::dispatch_typed`]), which covers
///    the concrete `openehr-rm` / `openehr-base` types carrying a
///    non-terminology class invariant — those need a typed value to run the
///    invariant on;
/// 3. for every class the table declines (`false`), the **GENERATED structural
///    dispatch** ([`structural_check`], emitted by
///    `openehr-codegen -- emit-json`), which decodes the
///    node into that class's own Rust type and discards it: the codec is the
///    structural-conformance authority for the whole emitted model, so a class
///    with no invariant is still refused when it is structurally defective (a
///    missing mandatory attribute, a wrong JSON kind, an unresolvable nested
///    slot `_type`). This step is what keeps the composition here rather than
///    upstream — the generated dispatch spans every spec crate at once.
///
/// # The inherited LOCATABLE invariant
///
/// After the dispatch runs, the inherited `LOCATABLE.Archetype_node_id_valid`
/// (`not archetype_node_id.is_empty`,
/// `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc`
/// §Invariants) is closed out for **every** concrete LOCATABLE descendant via
/// [`openehr_rm::v1_2::validate::locatable_node_id_violation`], whose applicable-type
/// set is the generated RM model's transitive concrete-descendant closure of
/// LOCATABLE — not a hand-maintained list. The typed table's arms realize it
/// themselves through their typed `Validate` impls, so the violation is appended
/// only when this node's own pass did not already report it (reported exactly
/// once per node). The classes with no arm — `ITEM_TREE`, `ITEM_LIST`,
/// `ITEM_SINGLE`, `EHR_STATUS`, and the demographic / EHR_EXTRACT LOCATABLEs —
/// reach the generated structural fallthrough, which decodes but runs no
/// invariant, so this is where the inherited invariant becomes theirs too.
///
/// Placing the closeout in the typed tier (rather than above both tiers) keeps
/// the fast-vs-typed equivalence property exact: the fast path vouches only for
/// classes whose evaluator already calls the same core, so both tiers report
/// the same single violation.
pub fn validate_rm_value_typed(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    // The undeclared-key refusal is raised here too, ahead of the decode, so
    // this tier's verdict does not depend on WHERE the offending member sits in
    // the object: the reader streams members in document order (it never
    // materializes the whole object first), so a node that is BOTH structurally
    // defective and carries an undeclared key would otherwise report whichever
    // defect the writer happened to put first. The tier above
    // ([`validate_rm_invariants_as`]) applies the same check, so on that path
    // this one never fires.
    if let Some(violation) = undeclared_key(ty, value) {
        out.push(violation);
        return;
    }
    let before = out.len();
    if !openehr_rm::v1_2::validate::typed_dispatch::dispatch_typed(ty, value, out) {
        run_structural(ty, value, out);
    }
    if let Some(v) = openehr_rm::v1_2::validate::locatable_node_id_violation(ty, value)
        && !out.get(before..).is_some_and(|added| added.contains(&v))
    {
        out.push(v);
    }
}

/// The generated-dispatch fallthrough of [`validate_rm_value_typed`]: decode the
/// node as the class its `_type` names and record a structural violation when it
/// does not conform.
///
/// The classes the typed table
/// ([`openehr_rm::v1_2::validate::typed_dispatch::dispatch_typed`]) handles already
/// decode there — its arms report the same `does not conform to RM type …` on
/// failure — and the table returns `true` for them, so they never reach here: no
/// node is decoded twice. A `_type` naming no emitted class
/// ([`structural_check`] returns `None`) runs no check, unchanged: an
/// unrecognised type is not a structural claim this layer can adjudicate.
fn run_structural(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    if let Some(Err(e)) = structural_check(ty, value) {
        record_type_mismatch(ty, &e, out);
    }
}
