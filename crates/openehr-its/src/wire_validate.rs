//! The **wire-boundary RM class-invariant dispatcher** — the `_type`→`Validate`
//! typed dispatch tier.
//!
//! This is a wire-boundary operation: it consumes a canonical-JSON node and
//! *deserializes* it into its concrete RM type through the emitted canonical-JSON
//! `serde` impls ([`crate::json::from_canonical_value`]), then runs that type's
//! RM class invariants. It lives in `openehr-its` (not `openehr-rm`) because it
//! drives the codec's entry points, which are defined here; the `Validate` trait and the
//! invariant impls (`*_impl.rs`) stay in `openehr-rm`/`openehr-base` as pure RM
//! model semantics. The two-tier entry point [`validate_rm_value`] calls the
//! allocation-free fast path ([`openehr_rm::validate::try_fast_validate`]) first
//! and falls back to the typed dispatch below.
//!
//! # Fidelity to the reference implementation (archie)
//!
//! The RM class invariants (in `openehr-rm`'s `*_impl.rs`) mirror openEHR's
//! reference implementation archie; this module only routes a node to the right
//! concrete type. A node that does not deserialize into its declared concrete RM
//! type surfaces `does not conform to RM type …` (see `record_type_mismatch`).

use serde_json::Value;

use openehr_base::validate::{InvariantViolation, Validate};

use serde::de::DeserializeOwned;

use crate::json::{JsonParseError, from_canonical_value};
use crate::json_codec::generated::structural::{declared_fields, structural_check};

/// Record a typed-deserialize failure as a validation violation.
///
/// A node that does not deserialize into its declared concrete RM type is NOT
/// "caught by the codec/schema layer" on the commit path: the node codec stores
/// the raw canonical-JSON fragment verbatim (no openEHR spec governs the storage
/// mechanics — our own storage design) and the ITS-JSON schema is not enforced
/// at commit, so a missing mandatory attribute (e.g. `COMPOSITION.composer [1]`)
/// or a wrong nested type (e.g. an `EHR_STATUS.subject` that is not `PARTY_SELF`)
/// reaches here and nowhere else. Per ITS-REST `422_COMPOSITION.yaml` ("converts,
/// but does not validate") this is a validation failure — surface it. (The valid
/// corpus deserializes cleanly at every node, so this never rejects a valid
/// input; if it ever did, that would expose a codegen field-optionality bug to
/// fix in the emitter.)
fn record_type_mismatch(ty: &str, err: &dyn std::fmt::Display, out: &mut Vec<InvariantViolation>) {
    out.push(InvariantViolation::here(format!(
        "does not conform to RM type {ty}: {err}"
    )));
}

fn run<T: DeserializeOwned + Validate>(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    match from_canonical_value::<T>(value) {
        Ok(v) => v.validate_invariants(out),
        Err(e) => record_type_mismatch(ty, &e, out),
    }
}

/// Like [`run`], but deserialize `T` from a copy of `value` whose nested
/// RM-node child collections have been emptied ([`prune_child_nodes`]).
///
/// NOTE (settled perf design — no openEHR spec governs validation-pass
/// mechanics; our own design): the RM-invariant pass ([`validate_rm_value`])
/// is called once per
/// `_type` node while the composition validator recurses the live JSON tree, so
/// deserializing each node's *whole* subtree (as `from_json_value` does for a
/// concrete container type) re-parses every descendant once per ancestor —
/// O(Σ subtree sizes) for overlapping subtrees. This shallow variant is used for
/// the LOCATABLE structural containers whose own class invariants inspect only
/// scalar / single-object attributes (never a child collection): with the child
/// arrays emptied, each node deserializes only its own immediate shape, so the
/// pass is O(total nodes) instead of O(Σ subtree sizes). The node's own
/// single-valued attributes are KEPT (only collections are emptied), so its
/// mandatory-attribute presence and single-object type conformance are still
/// enforced on deserialize — the missing-mandatory-attribute rejection
/// (`422_COMPOSITION`, e.g. a dropped `COMPOSITION.composer [1]`) and every class
/// invariant result are unchanged. Types whose own invariants DO read a child
/// collection (`HISTORY.events`, `ITEM_TABLE.rows`) keep the full [`run`]
/// deserialize.
///
/// NOTE: emptying a child *collection* here means a malformation *inside*
/// an array element is no longer reported at this ancestor's path — it is
/// reported at that element's own recursion step instead (each collection member
/// is a separate `_type` node the composition validator visits and dispatches).
/// For array-element types the dispatcher does not cover (embedded non-LOCATABLE
/// helpers such as `LINK` / `PARTICIPATION`, which carry no class invariant), a
/// structural malformation that the full ancestor deserialize used to surface is
/// no longer surfaced. This narrows only the redundant ancestor-cascade reporting
/// on already-invalid input; the valid path and every test-pinned rejection are
/// byte-identical, and the ITS-JSON schema gate remains the exhaustive
/// structural oracle where one is required (this pass is the RM class-invariant
/// check, not a schema validator — `422_COMPOSITION.yaml`).
fn run_shallow<T: DeserializeOwned + Validate>(
    ty: &str,
    value: &Value,
    out: &mut Vec<InvariantViolation>,
) {
    match from_canonical_value::<T>(&prune_child_nodes(value)) {
        Ok(v) => v.validate_invariants(out),
        Err(e) => record_type_mismatch(ty, &e, out),
    }
}

/// A shallow copy of an RM node with every nested RM-node **collection** emptied,
/// recursing through single-valued nested nodes (which are kept, so the node's
/// own mandatory single attributes stay enforced on deserialize). Scalar arrays
/// (e.g. `DV_MULTIMEDIA.data` octets) are kept as-is. See [`run_shallow`] for why
/// this is sound for the structural container types.
fn prune_child_nodes(value: &Value) -> Value {
    let Value::Object(map) = value else {
        return value.clone();
    };
    let ty = map.get("_type").and_then(Value::as_str);
    let mut out = serde_json::Map::with_capacity(map.len());
    for (key, child) in map {
        let pruned = match child {
            // An array of RM nodes: the children are recursed into (and fully
            // validated) individually by the composition validator, so this
            // node's own invariants never need them.
            //
            // A `1..*` container keeps ONE member (itself pruned), because its
            // emission shape is non-empty by construction
            // (`openehr_base::containers::NonEmptyVec`) and an emptied array
            // would fail to decode — a false structural violation on valid
            // input. Everything else is emptied outright.
            //
            // NOTE: emptying leaves an OPTIONAL container decoding to
            // `Some(vec![])` — a present-but-empty list, the state
            // `x /= Void implies not x.is_empty` forbids. That is safe here
            // because the run_shallow classes' own invariants never read a child
            // collection (this function's documented precondition), and that
            // invariant family is judged on the RAW node by
            // `openehr_rm::validate::nonempty_list_violations`, a layer outside
            // the fast/typed pair — never on this pruned copy.
            Value::Array(items) if items.iter().any(Value::is_object) => {
                if ty.is_some_and(|t| lower_bound_one(t, key)) {
                    Value::Array(items.first().map(prune_child_nodes).into_iter().collect())
                } else {
                    Value::Array(Vec::new())
                }
            }
            // A single nested node: keep it (its presence is a structural
            // constraint this node's deserialize must still enforce), but recurse
            // to empty ITS child collections.
            Value::Object(_) => prune_child_nodes(child),
            // Scalars and scalar arrays: keep verbatim.
            other => other.clone(),
        };
        out.insert(key.clone(), pruned);
    }
    Value::Object(out)
}

/// Whether the static RM model declares `attribute` of `ty` as a container with
/// a cardinality lower bound of 1 — the attributes whose emission shape is
/// `NonEmptyVec<T>` and which therefore cannot decode from an emptied array.
fn lower_bound_one(ty: &str, attribute: &str) -> bool {
    openehr_rm::model::attributes(ty)
        .any(|a| a.name == attribute && a.cardinality.is_some_and(|c| c.lower >= 1))
}

/// Run the **core** (non-terminology) RM class invariants for a single
/// canonical-JSON node, dispatching on its `_type`. A node with no (or an
/// unrecognised) `_type` runs no invariants (returns without appending).
///
/// Two tiers (performance: the RM-invariant pass visits every `_type` node of a
/// commit, ~1.5k for a populated composition, so the per-node cost is
/// load-bearing):
///
/// 1. the **fast path** ([`openehr_rm::validate::try_fast_validate`]) verifies
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
/// invariants ([`openehr_rm::validate::terminology`]) are an orthogonal layer
/// added by [`validate_rm_value`], not part of the fast/typed equivalence.
pub fn validate_rm_invariants(value: &Value, out: &mut Vec<InvariantViolation>) {
    let Some(ty) = value.get("_type").and_then(Value::as_str) else {
        return;
    };
    validate_rm_invariants_as(ty, value, out);
}

/// [`validate_rm_invariants`] with the node's RM type supplied by the caller —
/// the entry a tree walker uses for a node whose wire `_type` is legitimately
/// absent (canonical JSON requires `_type` only where the declared attribute
/// type is abstract; see [`openehr_rm::model::declared_concrete_type`]). The
/// `_type`-reading
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
    if !openehr_rm::validate::try_fast_validate(ty, value, out) {
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
/// 2. a node with **nothing missing** ([`mandatory_data_present`](openehr_rm::validate::incomplete::mandatory_data_present))
///    runs the strict tiers unchanged: the relaxation costs it no strictness;
/// 3. a node that **is** missing mandatory data does not have its TYPED
///    construction driven — that tier's refusal IS the presence rule the state
///    lifts (the generated types make existence and cardinality lower bounds
///    structural). Wrongness is still judged: a positive type contradiction
///    ([`contradicts_rm_type`](openehr_rm::validate::incomplete::contradicts_rm_type))
///    is reported through the authoritative typed tier, and the class
///    invariants run wherever the fast path can still vouch for the node as it
///    stands.
///
/// The orthogonal layers the caller runs beside this one — terminology,
/// `LOCATABLE.Archetyped_valid`, the data-structure shape duties, and the
/// archetype-conformance pass — are NOT relaxed here; only the
/// mandatory-presence and cardinality-lower-bound layers are
/// (`openehr_rm::validate::check_mandatory_containers` /
/// `nonempty_list_violations`, which the relaxed walker skips).
pub fn validate_rm_invariants_relaxed_as(
    ty: &str,
    value: &Value,
    out: &mut Vec<InvariantViolation>,
) {
    if let Some(violation) = undeclared_key(ty, value) {
        out.push(violation);
        return;
    }
    if openehr_rm::validate::incomplete::mandatory_data_present(ty, value) {
        if !openehr_rm::validate::try_fast_validate(ty, value, out) {
            validate_rm_value_typed(ty, value, out);
        }
        return;
    }
    if openehr_rm::validate::incomplete::contradicts_rm_type(ty, value) {
        validate_rm_value_typed(ty, value, out);
        return;
    }
    // The node is missing mandatory data and is not wrong: its class
    // invariants are evaluated where the fast path can still read them off the
    // node as it stands, and a decline is the honest answer — an invariant
    // over an absent attribute has nothing to judge, which is exactly the
    // "data may be missing" case §Incomplete Content admits.
    let _vouched = openehr_rm::validate::try_fast_validate(ty, value, out);
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

/// Run **all** RM class invariants for a single canonical-JSON node — the core
/// (fast/typed) invariants ([`validate_rm_invariants`]) plus the
/// terminology-backed invariants
/// ([`openehr_rm::validate::terminology::validate_rm_terminology`], the
/// openEHR-terminology group and code-set membership checks the generated
/// invariant cores cannot express mechanically). This is the unified dispatcher
/// every consumer calls; the composition validator invokes it per node and
/// prefixes the absolute RM path onto each [`InvariantViolation`].
pub fn validate_rm_value(value: &Value, out: &mut Vec<InvariantViolation>) {
    let Some(ty) = value.get("_type").and_then(Value::as_str) else {
        return;
    };
    // The core path (fast/typed), then the three orthogonal layers — the
    // model-driven mandatory-container lower bounds, the model-driven
    // present-implies-non-empty family, and the terminology-backed invariants
    // (each dispatches on the same `_type`). The core tiers stay a pure pair so
    // the fast-vs-typed equivalence property holds exactly.
    validate_rm_invariants_as(ty, value, out);
    openehr_rm::validate::check_mandatory_containers(ty, value, out);
    openehr_rm::validate::nonempty_list_violations(ty, value, out);
    openehr_rm::validate::terminology::validate_rm_terminology(ty, value, out);
}

/// The typed dispatch tier of [`validate_rm_value`]: deserialize the node into
/// its concrete RM type and run that type's `Validate` impl. Authoritative for
/// every node (the fast path may only *skip* it when its result is provably
/// identical); also the oracle the fast-path equivalence tests compare against.
///
/// The hand-written table below covers the concrete `openehr-rm` /
/// `openehr-base` types that carry a non-terminology class invariant (the ones
/// with a `*_impl.rs` sibling) — those need a typed value to run the invariant
/// on. **Every other class falls through to the GENERATED structural dispatch**
/// ([`structural_check`], emitted by `openehr-codegen -- emit-json`), which
/// decodes the node into that class's own Rust type and discards it: the codec
/// is the structural-conformance authority for the whole emitted model, so a
/// class with no invariant is still refused when it is structurally defective
/// (a missing mandatory attribute, a wrong JSON kind, an unresolvable nested
/// slot `_type`). `DV_INTERVAL` is dispatched with a `DvOrdered` element type so
/// the `Limits_consistent` ordering invariant is reached, falling
/// back to `serde_json::Value` (own boundary-flag invariants only) when the
/// limits do not deserialize as typed `DV_ORDERED` values. The other generic
/// containers (`HISTORY`, `POINT_EVENT`, `INTERVAL_EVENT`) are checked with
/// `serde_json::Value` as the element type — enough for their own (non-child)
/// invariants.
///
/// # The inherited LOCATABLE invariant
///
/// After the table runs, the inherited `LOCATABLE.Archetype_node_id_valid`
/// (`not archetype_node_id.is_empty`,
/// `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.locatable.adoc`
/// §Invariants) is closed out for **every** concrete LOCATABLE descendant via
/// [`openehr_rm::validate::locatable_node_id_violation`], whose applicable-type
/// set is the generated RM model's transitive concrete-descendant closure of
/// LOCATABLE — not a hand-maintained list. The arms above realize it themselves
/// through their typed `Validate` impls, so the violation is appended only when
/// this node's own pass did not already report it (reported exactly once per
/// node). The classes with no arm — `ITEM_TREE`, `ITEM_LIST`, `ITEM_SINGLE`,
/// `EHR_STATUS`, and the demographic / EHR_EXTRACT LOCATABLEs — reach the
/// generated structural fallthrough, which decodes but runs no invariant, so
/// this is where the inherited invariant becomes theirs too.
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
    dispatch_typed(ty, value, out);
    if let Some(v) = openehr_rm::validate::locatable_node_id_violation(ty, value)
        && !out.get(before..).is_some_and(|added| added.contains(&v))
    {
        out.push(v);
    }
}

/// The `_type` → `run::<T>` table of [`validate_rm_value_typed`].
#[expect(
    clippy::too_many_lines,
    reason = "a flat `_type` -> `run::<T>` dispatch table; the length is the size of the RM type set, not logic"
)]
fn dispatch_typed(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    use openehr_base::base_types::identification::archetype_id::ArchetypeId;
    use openehr_base::base_types::identification::internet_id::InternetId;
    use openehr_base::base_types::identification::iso_oid::IsoOid;
    use openehr_base::base_types::identification::object_ref::ObjectRefData;
    use openehr_base::base_types::identification::object_version_id::ObjectVersionId;
    use openehr_base::base_types::identification::party_ref::PartyRef;
    use openehr_base::base_types::identification::terminology_id::TerminologyId;
    use openehr_base::base_types::identification::version_tree_id::VersionTreeId;

    use openehr_rm::common::archetyped::archetyped::Archetyped;
    use openehr_rm::common::archetyped::feeder_audit_details::FeederAuditDetails;
    use openehr_rm::common::directory::folder::Folder;
    use openehr_rm::common::generic::attestation::Attestation;
    use openehr_rm::common::generic::audit_details::AuditDetailsData;
    use openehr_rm::common::generic::party_identified::PartyIdentifiedData;
    use openehr_rm::common::generic::party_related::PartyRelated;
    use openehr_rm::common::tags::item_tag::ItemTag;
    use openehr_rm::composition::composition::Composition;
    use openehr_rm::composition::content::entry::action::Action;
    use openehr_rm::composition::content::entry::activity::Activity;
    use openehr_rm::composition::content::entry::admin_entry::AdminEntry;
    use openehr_rm::composition::content::entry::evaluation::Evaluation;
    use openehr_rm::composition::content::entry::instruction::Instruction;
    use openehr_rm::composition::content::entry::instruction_details::InstructionDetails;
    use openehr_rm::composition::content::entry::observation::Observation;
    use openehr_rm::composition::content::navigation::section::Section;
    use openehr_rm::composition::event_context::EventContext;
    use openehr_rm::data_structures::history::history::History;
    use openehr_rm::data_structures::history::interval_event::IntervalEvent;
    use openehr_rm::data_structures::history::point_event::PointEvent;
    use openehr_rm::data_structures::item_structure::item_table::ItemTable;
    use openehr_rm::data_structures::representation::cluster::Cluster;
    use openehr_rm::data_structures::representation::element::Element;
    use openehr_rm::data_types::basic::dv_identifier::DvIdentifier;
    use openehr_rm::data_types::encapsulated::dv_multimedia::DvMultimedia;
    use openehr_rm::data_types::encapsulated::dv_parsable::DvParsable;
    use openehr_rm::data_types::quantity::date_time::dv_date::DvDate;
    use openehr_rm::data_types::quantity::date_time::dv_date_time::DvDateTime;
    use openehr_rm::data_types::quantity::date_time::dv_duration::DvDuration;
    use openehr_rm::data_types::quantity::date_time::dv_time::DvTime;
    use openehr_rm::data_types::quantity::dv_count::DvCount;
    use openehr_rm::data_types::quantity::dv_interval::DvInterval;
    use openehr_rm::data_types::quantity::dv_ordered::DvOrdered;
    use openehr_rm::data_types::quantity::dv_ordinal::DvOrdinal;
    use openehr_rm::data_types::quantity::dv_proportion::DvProportion;
    use openehr_rm::data_types::quantity::dv_quantity::DvQuantity;
    use openehr_rm::data_types::quantity::dv_scale::DvScale;
    use openehr_rm::data_types::quantity::reference_range::ReferenceRange;
    use openehr_rm::data_types::text::code_phrase::CodePhrase;
    use openehr_rm::data_types::text::dv_text::DvText;
    use openehr_rm::data_types::text::term_mapping::TermMapping;
    use openehr_rm::data_types::time_specification::dv_periodic_time_specification::DvPeriodicTimeSpecification;
    use openehr_rm::data_types::uri::dv_ehr_uri::DvEhrUri;
    use openehr_rm::data_types::uri::dv_uri::DvUriData;
    use openehr_rm::integration::generic_entry::GenericEntry;

    match ty {
        // data_types
        "CODE_PHRASE" => run::<CodePhrase>(ty, value, out),
        // DV_TEXT + DV_CODED_TEXT share the DvText enum (Valid_value /
        // Formatting_valid, dv_text.adoc; DV_CODED_TEXT adds the structural
        // defining_code 1..1 at deserialize).
        "DV_TEXT" | "DV_CODED_TEXT" => run::<DvText>(ty, value, out),
        "DV_URI" => run::<DvUriData>(ty, value, out),
        "DV_EHR_URI" => run::<DvEhrUri>(ty, value, out),
        "DV_IDENTIFIER" => run::<DvIdentifier>(ty, value, out),
        "TERM_MAPPING" => run::<TermMapping>(ty, value, out),
        "DV_MULTIMEDIA" => run::<DvMultimedia>(ty, value, out),
        "DV_PROPORTION" => run::<DvProportion>(ty, value, out),
        "DV_QUANTITY" => run::<DvQuantity>(ty, value, out),
        "DV_COUNT" => run::<DvCount>(ty, value, out),
        "DV_DURATION" => run::<DvDuration>(ty, value, out),
        "DV_DATE" => run::<DvDate>(ty, value, out),
        "DV_TIME" => run::<DvTime>(ty, value, out),
        "DV_DATE_TIME" => run::<DvDateTime>(ty, value, out),
        "DV_ORDINAL" => run::<DvOrdinal>(ty, value, out),
        "DV_SCALE" => run::<DvScale>(ty, value, out),
        "DV_PARSABLE" => run::<DvParsable>(ty, value, out),
        "DV_PERIODIC_TIME_SPECIFICATION" => run::<DvPeriodicTimeSpecification>(ty, value, out),
        "REFERENCE_RANGE" => run::<ReferenceRange>(ty, value, out),
        // DV_INTERVAL: prefer the DV_ORDERED-typed element so the
        // Limits_consistent ordering invariant runs; fall back to
        // Value elements (boundary flags only) for non-DV_ORDERED payloads.
        "DV_INTERVAL" => {
            if let Ok(v) = from_canonical_value::<DvInterval<DvOrdered>>(value) {
                v.validate_invariants(out);
            } else {
                run::<DvInterval<Value>>(ty, value, out);
            }
        }
        // data_structures. HISTORY and ITEM_TABLE keep the full deserialize —
        // their own invariants read a child collection (`events` / `rows`); the
        // rest are structural containers with scalar-only invariants, so they
        // deserialize shallowly (see `run_shallow`).
        "ELEMENT" => run::<Element>(ty, value, out),
        "CLUSTER" => run_shallow::<Cluster>(ty, value, out),
        "HISTORY" => run::<History<Value>>(ty, value, out),
        "POINT_EVENT" => run_shallow::<PointEvent<Value>>(ty, value, out),
        "INTERVAL_EVENT" => run_shallow::<IntervalEvent<Value>>(ty, value, out),
        "ITEM_TABLE" => run::<ItemTable>(ty, value, out),
        // common
        "PARTY_IDENTIFIED" => run::<PartyIdentifiedData>(ty, value, out),
        "PARTY_RELATED" => run::<PartyRelated>(ty, value, out),
        "AUDIT_DETAILS" => run::<AuditDetailsData>(ty, value, out),
        "ATTESTATION" => run::<Attestation>(ty, value, out),
        "FEEDER_AUDIT_DETAILS" => run::<FeederAuditDetails>(ty, value, out),
        "ARCHETYPED" => run::<Archetyped>(ty, value, out),
        // ehr / composition — structural containers (scalar-only invariants),
        // deserialized shallowly (see `run_shallow`). GENERIC_ENTRY's `data:
        // ITEM [1..1]` is a single-valued node, so `run_shallow` keeps it and
        // still enforces its presence.
        "COMPOSITION" => run_shallow::<Composition>(ty, value, out),
        "EVENT_CONTEXT" => run_shallow::<EventContext>(ty, value, out),
        "ACTIVITY" => run_shallow::<Activity>(ty, value, out),
        "INSTRUCTION_DETAILS" => run::<InstructionDetails>(ty, value, out),
        "OBSERVATION" => run_shallow::<Observation>(ty, value, out),
        "INSTRUCTION" => run_shallow::<Instruction>(ty, value, out),
        "ACTION" => run_shallow::<Action>(ty, value, out),
        "EVALUATION" => run_shallow::<Evaluation>(ty, value, out),
        "ADMIN_ENTRY" => run_shallow::<AdminEntry>(ty, value, out),
        "GENERIC_ENTRY" => run_shallow::<GenericEntry>(ty, value, out),
        "SECTION" => run_shallow::<Section>(ty, value, out),
        "FOLDER" => run_shallow::<Folder>(ty, value, out),
        "ITEM_TAG" => run::<ItemTag>(ty, value, out),
        // base identification
        "OBJECT_REF" => run::<ObjectRefData>(ty, value, out),
        "PARTY_REF" => run::<PartyRef>(ty, value, out),
        "VERSION_TREE_ID" => run::<VersionTreeId>(ty, value, out),
        "OBJECT_VERSION_ID" => run::<ObjectVersionId>(ty, value, out),
        "ISO_OID" => run::<IsoOid>(ty, value, out),
        "ARCHETYPE_ID" => run::<ArchetypeId>(ty, value, out),
        "TERMINOLOGY_ID" => run::<TerminologyId>(ty, value, out),
        "INTERNET_ID" => run::<InternetId>(ty, value, out),
        // Every other class: the GENERATED structural dispatch decodes the node
        // into that class's own Rust type and discards it, so the codec is the
        // structural-conformance authority for the whole emitted model instead of
        // only the invariant-bearing classes above.
        other => run_structural(other, value, out),
    }
}

/// The generated-dispatch fallthrough of [`validate_rm_value_typed`]: decode the
/// node as the class its `_type` names and record a structural violation when it
/// does not conform.
///
/// The classes handled by the hand-written dispatch above already decode (their
/// arm calls [`run`] / [`run_shallow`], which reports the same
/// `does not conform to RM type …` on failure), so they never reach here — no
/// node is decoded twice. A `_type` naming no emitted class
/// ([`structural_check`] returns `None`) runs no check, unchanged: an
/// unrecognised type is not a structural claim this layer can adjudicate.
fn run_structural(ty: &str, value: &Value, out: &mut Vec<InvariantViolation>) {
    if let Some(Err(e)) = structural_check(ty, value) {
        record_type_mismatch(ty, &e, out);
    }
}
