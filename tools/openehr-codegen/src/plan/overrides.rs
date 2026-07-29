//! The emitter's **declarative decision data**: every override / mapping the
//! generator applies, as checked-in const tables rather than logic buried in
//! `match` arms. Each entry carries (a) its key, (b) the decision, (c) a spec
//! citation or the explicit "no openEHR spec governs this — our own design"
//! flag, and (d) a one-line reason.
//!
//! Keeping these as data (not code) makes them greppable, diff-reviewable, and
//! — via `tools/openehr-codegen/tests/emitter_invariants.rs` — machine-checked
//! for integrity (every entry names a class/field that exists in the loaded
//! schemas; every entry carries a non-empty citation). The lookup functions
//! ([`back_reference`], [`class_binding`], [`type_override`], [`field_default`],
//! [`primitive`], [`is_mapped_class`]) are thin scans over these tables — the
//! only behaviour they encode is the table content, so a decision change is a
//! data edit, never a control-flow edit.
//!
//! This is pure re-representation: the tables reproduce, byte-for-byte, the
//! decisions the R1 pipeline made inline. Nothing here changes any output.

use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// Primitive type map (spec foundation type → Rust type)
// ─────────────────────────────────────────────────────────────────────────────

/// One primitive spec type mapped to a Rust type.
pub(crate) struct Primitive {
    /// The BMM foundation type name (`Integer`, `String`, …).
    pub spec: &'static str,
    /// The Rust type it lowers to.
    pub rust: &'static str,
    /// Spec citation, or the explicit our-own-design flag.
    pub citation: &'static str,
    /// One-line reason.
    pub reason: &'static str,
}

/// The codegen primitive type map: BASE foundation scalar types → Rust scalars.
/// `Real`/`Double` both lower to `f64`; `String`/`Uri` both to `String`
/// (openEHR URI semantics are broader than any crate URL type — it stays a
/// plain string, the strong type living in hand-written behaviour if ever).
pub(crate) const PRIMITIVES: &[Primitive] = &[
    Primitive {
        spec: "Boolean",
        rust: "bool",
        citation: "BASE foundation_types (Boolean)",
        reason: "Boolean scalar → Rust bool.",
    },
    Primitive {
        spec: "Integer",
        rust: "i32",
        citation: "BASE foundation_types (Integer, 32-bit)",
        reason: "Integer scalar → i32.",
    },
    Primitive {
        spec: "Integer64",
        rust: "i64",
        citation: "BASE foundation_types (Integer64)",
        reason: "64-bit integer scalar → i64.",
    },
    Primitive {
        spec: "Real",
        rust: "f64",
        citation: "BASE foundation_types (Real)",
        reason: "Real scalar → f64.",
    },
    Primitive {
        spec: "Double",
        rust: "f64",
        citation: "BASE foundation_types (Double)",
        reason: "Double scalar → f64.",
    },
    Primitive {
        spec: "String",
        rust: "String",
        citation: "BASE foundation_types (String)",
        reason: "String scalar → Rust String.",
    },
    Primitive {
        spec: "Uri",
        rust: "String",
        citation: "no openEHR spec governs a URI newtype — our own design: openEHR URI \
                   semantics are broader than any crate URL type, so it stays a plain String",
        reason: "Uri lowers to String until a strong-newtype override is justified.",
    },
    Primitive {
        spec: "Octet",
        rust: "u8",
        citation: "BASE foundation_types (Octet)",
        reason: "Octet scalar → u8 (Octet containers become byte vectors).",
    },
    Primitive {
        spec: "Character",
        rust: "char",
        citation: "BASE foundation_types (Character)",
        reason: "Character scalar → Rust char.",
    },
];

/// The Rust type a primitive spec type maps to, or `None` if `name` is not a
/// primitive. Behaviour-identical to the former inline `primitive` match.
pub(crate) fn primitive(name: &str) -> Option<&'static str> {
    PRIMITIVES.iter().find(|p| p.spec == name).map(|p| p.rust)
}

// ─────────────────────────────────────────────────────────────────────────────
// Mapped/skipped classes (mapped to Rust, never emitted)
// ─────────────────────────────────────────────────────────────────────────────

/// A foundation class mapped to Rust and never emitted as a spec type.
pub(crate) struct MappedClass {
    /// The BMM class name.
    pub name: &'static str,
    /// Spec citation, or the explicit our-own-design flag.
    pub citation: &'static str,
    /// One-line reason.
    pub reason: &'static str,
}

/// Foundation classes handled by the Rust type system rather than emitted:
/// container types (→ `Vec`/map/`BTreeSet` via container properties), abstract
/// marker/algebraic traits (no data), functional types, service interfaces (no
/// data), and constant-holder classes (their constants live in `*_impl.rs`).
/// Scalar primitives are handled by [`PRIMITIVES`]. `Multiplicity_interval` and
/// `Cardinality` are NOT here — they emit as real structs (their inherited
/// `Interval<Integer>` binding comes from [`CLASS_BINDINGS`]).
pub(crate) const MAPPED_CLASSES: &[MappedClass] = &[
    // ── containers → Vec / map / set, handled by container properties ──
    MappedClass {
        name: "Container",
        citation: "BASE foundation_types (Container)",
        reason: "Abstract container base → handled by the container property kind.",
    },
    MappedClass {
        name: "List",
        citation: "BASE foundation_types (List<T>)",
        reason: "List<T> → Vec<T>.",
    },
    MappedClass {
        name: "Set",
        citation: "BASE foundation_types (Set<T>)",
        reason: "Set<T> → BTreeSet<T>.",
    },
    MappedClass {
        name: "Array",
        citation: "BASE foundation_types (Array<T>)",
        reason: "Array<T> → Vec<T>.",
    },
    MappedClass {
        name: "Hash",
        citation: "BASE foundation_types (Hash<K,V>)",
        reason: "Hash<K,V> → BTreeMap<K,V>.",
    },
    // ── abstract marker / algebraic traits (no data) ──
    MappedClass {
        name: "Any",
        citation: "BASE foundation_types (Any)",
        reason: "The universal supertype → carries no fields (renders as free-form JSON).",
    },
    MappedClass {
        name: "Ordered",
        citation: "BASE foundation_types (Ordered)",
        reason: "Algebraic ordering trait, no data.",
    },
    MappedClass {
        name: "Numeric",
        citation: "BASE foundation_types (Numeric)",
        reason: "Algebraic numeric trait, no data.",
    },
    MappedClass {
        name: "Ordered_Numeric",
        citation: "BASE foundation_types (Ordered_Numeric)",
        reason: "Algebraic trait, no data.",
    },
    MappedClass {
        name: "Comparable",
        citation: "BASE foundation_types (Comparable)",
        reason: "Algebraic comparison trait, no data.",
    },
    MappedClass {
        name: "Temporal",
        citation: "BASE foundation_types (Temporal)",
        reason: "Algebraic temporal trait, no data.",
    },
    // ── functional types ──
    MappedClass {
        name: "TUPLE",
        citation: "BASE foundation_types (functional package)",
        reason: "Functional tuple type, not a spec data class.",
    },
    MappedClass {
        name: "TUPLE1",
        citation: "BASE foundation_types (functional package)",
        reason: "Functional tuple type, not a spec data class.",
    },
    MappedClass {
        name: "TUPLE2",
        citation: "BASE foundation_types (functional package)",
        reason: "Functional tuple type, not a spec data class.",
    },
    MappedClass {
        name: "ROUTINE",
        citation: "BASE foundation_types (functional package)",
        reason: "Functional routine type, not a spec data class.",
    },
    MappedClass {
        name: "FUNCTION",
        citation: "BASE foundation_types (functional package)",
        reason: "Functional function type, not a spec data class.",
    },
    MappedClass {
        name: "PROCEDURE",
        citation: "BASE foundation_types (functional package)",
        reason: "Functional procedure type, not a spec data class.",
    },
    // ── service interfaces (no data) ──
    MappedClass {
        name: "Env",
        citation: "BASE base_types.builtins (Env service interface; Base Types §Built-in types)",
        reason: "Service interface, no data.",
    },
    MappedClass {
        name: "Locale",
        citation: "BASE base_types.builtins (Locale service interface; Base Types §Built-in types)",
        reason: "Service interface, no data.",
    },
    MappedClass {
        name: "Math",
        citation: "BASE base_types.builtins (Math service interface; Base Types §Built-in types)",
        reason: "Service interface, no data.",
    },
    MappedClass {
        name: "Quantity_converter",
        citation: "BASE base_types.builtins (Quantity_converter service interface; Base Types §Built-in types)",
        reason: "Service interface, no data.",
    },
    MappedClass {
        name: "Statistical_evaluator",
        citation: "BASE base_types.builtins (Statistical_evaluator service interface; Base Types §Built-in types)",
        reason: "Service interface, no data.",
    },
    // ── constant-holder classes (no data; become assoc consts in *_impl.rs) ──
    MappedClass {
        name: "Time_Definitions",
        citation: "BASE foundation_types (Time_Definitions)",
        reason: "Constant holder; its constants become associated consts in *_impl.rs.",
    },
    MappedClass {
        name: "BASIC_DEFINITIONS",
        citation: "BASE base_types.definitions (BASIC_DEFINITIONS; Base Types §Definitions Package)",
        reason: "Constant holder; its constants live in base_types/definitions/definitions_impl.rs.",
    },
    MappedClass {
        name: "OPENEHR_DEFINITIONS",
        citation: "BASE base_types.definitions (OPENEHR_DEFINITIONS; Base Types §Definitions Package)",
        reason: "Constant holder; its constants live in base_types/definitions/definitions_impl.rs.",
    },
];

/// Is `name` a mapped/skipped foundation class (never emitted)? Behaviour-
/// identical to the former `SKIP.contains(&name)`.
pub(crate) fn is_mapped_class(name: &str) -> bool {
    MAPPED_CLASSES.iter().any(|m| m.name == name)
}

// ─────────────────────────────────────────────────────────────────────────────
// Ancestor-generic class bindings
// ─────────────────────────────────────────────────────────────────────────────

/// One ancestor-generic binding: `class`'s generic parameter `param` is bound to
/// the concrete spec type `concrete` (a binding the BMM drops).
pub(crate) struct ClassBinding {
    pub class: &'static str,
    pub param: &'static str,
    pub concrete: &'static str,
    pub citation: &'static str,
    pub reason: &'static str,
}

/// Ancestor-generic bindings the BMM drops (it records `ancestors` and some
/// generic-content property types as bare class names, losing the `<Integer>` /
/// `<COMPOSITION>` argument). The emitter substitutes the concrete type instead
/// of degrading the field to `serde_json::Value`.
pub(crate) const CLASS_BINDINGS: &[ClassBinding] = &[
    ClassBinding {
        class: "Multiplicity_interval",
        param: "T",
        concrete: "Integer",
        citation: "BASE foundation_types — Multiplicity_interval is \"an Interval of Integer\"",
        reason: "openEHR files it under primitive_types without carrying the Interval<Integer> binding.",
    },
    ClassBinding {
        class: "X_VERSIONED_COMPOSITION",
        param: "T",
        concrete: "COMPOSITION",
        citation: "RM ehr_extract (X_VERSIONED_COMPOSITION : X_VERSIONED_OBJECT<COMPOSITION>)",
        reason: "Binds the versioned-content type X_VERSIONED_OBJECT<T> leaves open.",
    },
    ClassBinding {
        class: "X_VERSIONED_EHR_ACCESS",
        param: "T",
        concrete: "EHR_ACCESS",
        citation: "RM ehr_extract (X_VERSIONED_EHR_ACCESS : X_VERSIONED_OBJECT<EHR_ACCESS>)",
        reason: "Binds the versioned-content type X_VERSIONED_OBJECT<T> leaves open.",
    },
    ClassBinding {
        class: "X_VERSIONED_EHR_STATUS",
        param: "T",
        concrete: "EHR_STATUS",
        citation: "RM ehr_extract (X_VERSIONED_EHR_STATUS : X_VERSIONED_OBJECT<EHR_STATUS>)",
        reason: "Binds the versioned-content type X_VERSIONED_OBJECT<T> leaves open.",
    },
    ClassBinding {
        class: "X_VERSIONED_PARTY",
        param: "T",
        concrete: "PARTY",
        citation: "RM ehr_extract (X_VERSIONED_PARTY : X_VERSIONED_OBJECT<PARTY>)",
        reason: "Binds the versioned-content type X_VERSIONED_OBJECT<T> leaves open.",
    },
    ClassBinding {
        class: "X_VERSIONED_FOLDER",
        param: "T",
        concrete: "FOLDER",
        citation: "RM ehr_extract (X_VERSIONED_FOLDER : X_VERSIONED_OBJECT<FOLDER>)",
        reason: "Binds the versioned-content type X_VERSIONED_OBJECT<T> leaves open.",
    },
];

/// The generic-parameter → concrete-type bindings for `class` (empty if none).
/// Behaviour-identical to the former inline `class_binding`.
pub(crate) fn class_binding(class: &str) -> BTreeMap<String, String> {
    CLASS_BINDINGS
        .iter()
        .filter(|b| b.class == class)
        .map(|b| (b.param.to_string(), b.concrete.to_string()))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// REST DTO required-list overrides (docs-text-wins corrections)
// ─────────────────────────────────────────────────────────────────────────────

/// One REST-contract required-list correction: `(dto, field)` is emitted as
/// OPTIONAL although the vendored OAS schema lists it under `required`. Used
/// only where the ITS-REST **docs text** (which wins every conflict with the
/// released OAS — owner rulings 2026-07-24 + 2026-07-28) contradicts the OAS
/// shape.
pub(crate) struct RestOptionalOverride {
    pub dto: &'static str,
    pub field: &'static str,
    pub citation: &'static str,
    pub reason: &'static str,
}

/// The docs-text-wins required-list corrections for the generated ITS-REST
/// contract (`emit-rest`).
pub(crate) const REST_OPTIONAL_OVERRIDES: &[RestOptionalOverride] = &[
    RestOptionalOverride {
        dto: "Query",
        field: "offset",
        citation: "ITS-REST docs/query/Request.md §Common Headers and Query Parameters — \
                   \"`offset` … default `0`\"",
        reason: "A required member cannot default; the stored-query execute body must \
                 accept `{}` (a parameterless stored query).",
    },
    RestOptionalOverride {
        dto: "Query",
        field: "fetch",
        citation: "ITS-REST docs/query/Request.md §Common Headers and Query Parameters — \
                   \"`fetch` … default depends on the implementation\"",
        reason: "A required member cannot default; the stored-query execute body must \
                 accept `{}`.",
    },
    RestOptionalOverride {
        dto: "Query",
        field: "query_parameters",
        citation: "ITS-REST docs/query/Request.md §Query parameters — parameters exist \
                   \"Depending on each query definition\"; a parameterless stored query \
                   binds none",
        reason: "A stored query with no $parameters must be executable with an empty body.",
    },
];

/// The docs-text-wins correction for `(dto, field)`, or `None` — the field is
/// emitted optional and the generated code carries the citation.
pub(crate) fn rest_optional_override(
    dto: &str,
    field: &str,
) -> Option<&'static RestOptionalOverride> {
    REST_OPTIONAL_OVERRIDES
        .iter()
        .find(|o| o.dto == dto && o.field == field)
}

// ─────────────────────────────────────────────────────────────────────────────
// Field-level type overrides
// ─────────────────────────────────────────────────────────────────────────────

/// One field-level type override: `(class, field)` uses `rust_type` instead of
/// the BMM primitive.
pub(crate) struct TypeOverride {
    pub class: &'static str,
    pub field: &'static str,
    pub rust_type: &'static str,
    pub citation: &'static str,
    pub reason: &'static str,
}

/// Field type overrides mapping a `(class, field)` to a proven Rust crate type
/// instead of the BMM primitive. Only unambiguous mappings belong here — where
/// openEHR's semantics are broader than a crate (partial-precision ISO 8601,
/// plain-text URIs) the field stays `String` and the crate is used in the
/// hand-written `*_impl.rs` behaviour instead.
pub(crate) const TYPE_OVERRIDES: &[TypeOverride] = &[TypeOverride {
    class: "UUID",
    field: "value",
    rust_type: "uuid::Uuid",
    citation: "BASE base_types (UUID is an RFC-4122 canonical UUID); the `uuid` crate is our own \
               design choice for the Rust type",
    reason: "A UUID.value is an RFC-4122 canonical UUID — use `uuid::Uuid`. (ISO_OID / \
             INTERNET_ID / OBJECT_VERSION_ID are NOT plain UUIDs.)",
}];

/// The Rust type override for `(class, field)`, or `None`. Behaviour-identical
/// to the former inline `type_override`.
pub(crate) fn type_override(class: &str, field: &str) -> Option<&'static str> {
    TYPE_OVERRIDES
        .iter()
        .find(|o| o.class == class && o.field == field)
        .map(|o| o.rust_type)
}

// ─────────────────────────────────────────────────────────────────────────────
// Serde field defaults (wire-omittable fields)
// ─────────────────────────────────────────────────────────────────────────────

/// One serde default: the field `(owner, field)` defaults to the literal Rust
/// expression `default` when the canonical wire omits it.
pub(crate) struct FieldDefault {
    pub owner: &'static str,
    pub field: &'static str,
    pub default: &'static str,
    pub citation: &'static str,
    pub reason: &'static str,
}

/// Serde defaults for fields the canonical wire may omit. `Interval`'s
/// inclusivity/boundedness flags are mandatory in the BMM but archie/EHRbase
/// omit them: a bounded limit is *included* by default, an unstated limit is
/// *bounded* by default. The value is a literal Rust expression consumed by
/// `#[openehr(default = …)]`.
pub(crate) const FIELD_DEFAULTS: &[FieldDefault] = &[
    FieldDefault {
        owner: "Interval",
        field: "lower_included",
        default: "true",
        citation: "BASE foundation_types (Interval) for the semantics; no openEHR spec governs \
                   the wire omission — archie/EHRbase interop convention (our own design)",
        reason: "A bounded interval limit is included by default.",
    },
    FieldDefault {
        owner: "Interval",
        field: "upper_included",
        default: "true",
        citation: "BASE foundation_types (Interval) for the semantics; no openEHR spec governs \
                   the wire omission — archie/EHRbase interop convention (our own design)",
        reason: "A bounded interval limit is included by default.",
    },
    FieldDefault {
        owner: "Interval",
        field: "lower_unbounded",
        default: "false",
        citation: "BASE foundation_types (Interval) for the semantics; no openEHR spec governs \
                   the wire omission — archie/EHRbase interop convention (our own design)",
        reason: "An unstated interval limit is bounded by default.",
    },
    FieldDefault {
        owner: "Interval",
        field: "upper_unbounded",
        default: "false",
        citation: "BASE foundation_types (Interval) for the semantics; no openEHR spec governs \
                   the wire omission — archie/EHRbase interop convention (our own design)",
        reason: "An unstated interval limit is bounded by default.",
    },
];

/// The serde default expression for `(owner, field)`, or `None`. Behaviour-
/// identical to the former inline `field_default`.
pub(crate) fn field_default(owner: &str, field: &str) -> Option<&'static str> {
    FIELD_DEFAULTS
        .iter()
        .find(|d| d.owner == owner && d.field == field)
        .map(|d| d.default)
}

// ─────────────────────────────────────────────────────────────────────────────
// Owner/parent back-references (cycle-breaking, non-emitted)
// ─────────────────────────────────────────────────────────────────────────────

/// One owner/parent back-reference: `(class, field)` is a navigational pointer
/// from a part to the whole that owns it, omitted from the emitted struct.
pub(crate) struct BackReference {
    pub class: &'static str,
    pub field: &'static str,
    /// The spec citation naming the field a back-reference. **This string is
    /// emitted verbatim** into a `// NOTE:` in the generated struct (see
    /// `render::emit::render_struct_def`), so it is byte-load-bearing: changing
    /// it changes generated output.
    pub citation: &'static str,
    /// One-line reason (the cycle it breaks) — documentation only, not emitted.
    pub reason: &'static str,
}

/// Mandatory single-valued properties designated as **owner/parent
/// back-references** — a navigational association pointing from a part to the
/// whole that owns it, not forward-owned data.
///
/// # Why the emitter special-cases these (owner ruling 2026-07-19)
///
/// The spec is written in reference-semantics languages (Eiffel/Java) where an
/// owner/parent pointer is a trivially-satisfiable back-pointer. In Rust value
/// semantics an *owning* mandatory back-reference (`Box<Owner>`) makes the type
/// a **non-constructible infinite value** (every `ARCHETYPE` owns a
/// `terminology` whose `owner_archetype` is an `ARCHETYPE`, ad infinitum), so an
/// owning emission is a mis-modeling of the spec, not extra strictness. These
/// properties never appear on the canonical JSON/XML wire either. Per the repo
/// convention (root `CLAUDE.md` §Conventions: "Behavioural back-references …
/// use `Weak` or an index, never an owning reference") each is emitted as a
/// non-data back-reference: omitted from the owned struct fields and from serde,
/// behavioural access left to the hand-written `*_impl.rs`. This laxes no
/// forward/owned data — every genuine composition field stays mandatory (see
/// `Model::assert_constructible`, which proves every remaining cycle is broken
/// only at a designated edge here).
///
/// The BMM carries no `is_im_runtime`/`is_im_infrastructure` flag on these
/// (verified against the vendored BMM 2026-07-19), so the designation is an
/// explicit, spec-cited override.
pub(crate) const BACK_REFERENCES: &[BackReference] = &[
    BackReference {
        class: "ARCHETYPE_TERMINOLOGY",
        field: "owner_archetype",
        citation: "AM AOM2 archetype_terminology (owner_archetype: Archetype that owns this terminology)",
        reason: "Back-pointer to the owning archetype; forms the ARCHETYPE ↔ \
                 ARCHETYPE_TERMINOLOGY cycle. \
                 (docs/specs/openehr/AM/docs/UML/classes/\
                 org.openehr.am.aom2.archetype_terminology.adoc)",
    },
    BackReference {
        class: "RESOURCE_DESCRIPTION",
        field: "parent_resource",
        citation: "BASE resource resource_description (parent_resource: Reference to owning resource)",
        reason: "Back-pointer to the owning resource; forms the AUTHORED_RESOURCE ↔ \
                 RESOURCE_DESCRIPTION cycle. \
                 (docs/specs/openehr/BASE/docs/UML/classes/\
                 org.openehr.base.resource.resource_description.adoc)",
    },
    BackReference {
        class: "ARCHETYPE_ONTOLOGY",
        field: "parent_archetype",
        citation: "AM AOM14 archetype_ontology (parent_archetype: Archetype which owns this terminology)",
        reason: "The ADL 1.4 owner back-reference (am14 analogue of am24 owner_archetype), with \
                 the invariant parent_archetype.ontology = Current; forms the ARCHETYPE ↔ \
                 ARCHETYPE_ONTOLOGY cycle. \
                 (docs/specs/openehr/AM/docs/UML/classes/\
                 org.openehr.am.aom14.archetype_ontology.adoc)",
    },
];

/// The spec citation if `(class, field)` is a designated owner/parent
/// back-reference, else `None`. Behaviour-identical to the former inline
/// `back_reference`; the returned string is emitted verbatim into generated
/// output, so it is byte-stable.
pub(crate) fn back_reference(class: &str, field: &str) -> Option<&'static str> {
    BACK_REFERENCES
        .iter()
        .find(|b| b.class == class && b.field == field)
        .map(|b| b.citation)
}

// ─────────────────────────────────────────────────────────────────────────────
// emit-xml BMM-only field allowlist (RM/BASE model vs vendored ITS-XML skew)
// ─────────────────────────────────────────────────────────────────────────────

/// One allowlisted BMM-only field: the owning concrete type's spec name, the
/// field's wire name, the governing spec-version skew, and a one-line reason
/// bucket.
pub(crate) struct XmlBmmOnlyField {
    /// The owning concrete type's spec name.
    pub spec: &'static str,
    /// The field's openEHR wire (property) name.
    pub wire_name: &'static str,
    /// Why the field is absent from the vendored XSD and safe to append — the
    /// concrete RM/BASE-model-vs-ITS-XML-XSD version delta.
    pub citation: &'static str,
    /// One-line reason bucket.
    pub reason: &'static str,
}

/// Allowlist of BMM fields that have **no** matching element or attribute in any
/// vendored ITS-XML XSD (the RM/BASE model outran the vendored ITS-XML schemas).
/// Each accepted `(spec, wire_name)` pair is emitted as a **deterministic
/// trailing element** in BMM field order (so it is never silently dropped), and
/// carries the spec delta that justifies it. Any XSD-covered struct with a BMM
/// field NOT on this list fails codegen (the `check_bmm_field_coverage` guard in
/// `render::emit_xml`), forcing an explicit decision instead of a silent drop.
///
/// The governing spec citation for every entry is the pinned-version delta
/// (`docs/VERSIONS.md`): the emitted model is RM 1.2.0 / BASE 1.3.0, while the
/// vendored canonical-XML schemas are ITS-XML 1.0.2 (namespace `.../v1`) and, for
/// the EHR/demographic/extract closure, ITS-XML 2.0.0 (RM 1.1.0). Where the model
/// added or renamed a field after those XSDs were cut, no XSD slot exists.
///
/// Empty is the desired state: it means every RM-1.2.0 BMM field of an
/// XSD-covered type maps to a vendored-XSD slot.
pub(crate) const XML_BMM_ONLY_ALLOWLIST: &[XmlBmmOnlyField] = &[
    // ── BASE 1.3.0 resource-class growth (ITS-XML 1.0.2 Resource.xsd predates it) ──
    XmlBmmOnlyField {
        spec: "AUTHORED_RESOURCE",
        wire_name: "uid",
        citation: "BASE 1.3.0 added AUTHORED_RESOURCE.uid (HIER_OBJECT_ID); absent from ITS-XML 1.0.2 Resource.xsd.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "AUTHORED_RESOURCE",
        wire_name: "annotations",
        citation: "BASE 1.3.0 added AUTHORED_RESOURCE.annotations (RESOURCE_ANNOTATIONS); absent from ITS-XML 1.0.2 Resource.xsd.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "title",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.title; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "original_namespace",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.original_namespace; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "original_publisher",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.original_publisher; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "custodian_namespace",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.custodian_namespace; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "custodian_organisation",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.custodian_organisation; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "copyright",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.copyright; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "licence",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.licence; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "ip_acknowledgements",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.ip_acknowledgements; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "references",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.references; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "RESOURCE_DESCRIPTION",
        wire_name: "conversion_details",
        citation: "BASE 1.3.0 RESOURCE_DESCRIPTION.conversion_details; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "TRANSLATION_DETAILS",
        wire_name: "version_last_translated",
        citation: "BASE 1.3.0 TRANSLATION_DETAILS.version_last_translated; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "TRANSLATION_DETAILS",
        wire_name: "other_contributors",
        citation: "BASE 1.3.0 TRANSLATION_DETAILS.other_contributors; ITS-XML 1.0.2 Resource.xsd lacks it.",
        reason: "BASE 1.3.0 resource-class growth vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "TRANSLATION_DETAILS",
        wire_name: "accreditaton",
        citation: "BASE 1.3.0 BMM spells this field `accreditaton` (sic); ITS-XML 1.0.2 Resource.xsd uses `accreditation`. Canonical JSON derives from the BMM, so the BMM spelling is emitted for JSON/XML consistency.",
        reason: "BASE 1.3.0 BMM field spelling vs ITS-XML 1.0.2.",
    },
    // ── BASE 1.3.0 CODE_PHRASE.preferred_term ──
    XmlBmmOnlyField {
        spec: "CODE_PHRASE",
        wire_name: "preferred_term",
        citation: "BASE 1.3.0 added CODE_PHRASE.preferred_term; ITS-XML 1.0.2 BaseTypes.xsd CODE_PHRASE has terminology_id + code_string only.",
        reason: "BASE 1.3.0 field addition vs ITS-XML 1.0.2.",
    },
    // ── RM 1.2.0 ENTRY.work_flow_id → workflow_id rename (all ENTRY subtypes) ──
    XmlBmmOnlyField {
        spec: "ACTION",
        wire_name: "workflow_id",
        citation: "RM 1.2.0 renamed ENTRY.work_flow_id → workflow_id; ITS-XML 1.0.2 Content.xsd still declares work_flow_id. Canonical JSON corpus uses workflow_id.",
        reason: "RM 1.2.0 ENTRY.work_flow_id → workflow_id rename vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "ADMIN_ENTRY",
        wire_name: "workflow_id",
        citation: "RM 1.2.0 renamed ENTRY.work_flow_id → workflow_id; ITS-XML 1.0.2 Content.xsd still declares work_flow_id.",
        reason: "RM 1.2.0 ENTRY.work_flow_id → workflow_id rename vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "EVALUATION",
        wire_name: "workflow_id",
        citation: "RM 1.2.0 renamed ENTRY.work_flow_id → workflow_id; ITS-XML 1.0.2 Content.xsd still declares work_flow_id.",
        reason: "RM 1.2.0 ENTRY.work_flow_id → workflow_id rename vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "INSTRUCTION",
        wire_name: "workflow_id",
        citation: "RM 1.2.0 renamed ENTRY.work_flow_id → workflow_id; ITS-XML 1.0.2 Content.xsd still declares work_flow_id.",
        reason: "RM 1.2.0 ENTRY.work_flow_id → workflow_id rename vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "OBSERVATION",
        wire_name: "workflow_id",
        citation: "RM 1.2.0 renamed ENTRY.work_flow_id → workflow_id; ITS-XML 1.0.2 Content.xsd still declares work_flow_id.",
        reason: "RM 1.2.0 ENTRY.work_flow_id → workflow_id rename vs ITS-XML 1.0.2.",
    },
    // ── RM 1.2.0 data-type / data-structure additions ──
    XmlBmmOnlyField {
        spec: "DV_QUANTITY",
        wire_name: "units_system",
        citation: "RM 1.2.0 added DV_QUANTITY.units_system; absent from ITS-XML 1.0.2 BaseTypes.xsd.",
        reason: "RM 1.2.0 field addition vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "DV_QUANTITY",
        wire_name: "units_display_name",
        citation: "RM 1.2.0 added DV_QUANTITY.units_display_name; absent from ITS-XML 1.0.2 BaseTypes.xsd.",
        reason: "RM 1.2.0 field addition vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "ELEMENT",
        wire_name: "null_reason",
        citation: "RM 1.2.0 added ELEMENT.null_reason; ITS-XML 1.0.2 Structure.xsd ELEMENT has value + null_flavour only.",
        reason: "RM 1.2.0 field addition vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "ISM_TRANSITION",
        wire_name: "reason",
        citation: "RM 1.2.0 added ISM_TRANSITION.reason (List<DV_TEXT>); ITS-XML 1.0.2 Content.xsd has current_state/transition/careflow_step only.",
        reason: "RM 1.2.0 field addition vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "FEEDER_AUDIT_DETAILS",
        wire_name: "other_details",
        citation: "RM 1.2.0 added FEEDER_AUDIT_DETAILS.other_details (ITEM_STRUCTURE); absent from ITS-XML 1.0.2 Common.xsd.",
        reason: "RM 1.2.0 field addition vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "FOLDER",
        wire_name: "details",
        citation: "RM 1.2.0 added FOLDER.details (ITEM_STRUCTURE); absent from ITS-XML 1.0.2 Composition.xsd.",
        reason: "RM 1.2.0 field addition vs ITS-XML 1.0.2.",
    },
    XmlBmmOnlyField {
        spec: "EHR",
        wire_name: "tags",
        citation: "RM 1.2.0 added EHR.tags; absent from ITS-XML 2.0.0 (RM 1.1.0) Ehr.xsd, the vendored EHR shape.",
        reason: "RM 1.2.0 field addition vs ITS-XML 2.0.0 (RM 1.1.0).",
    },
    // ── RM 1.2.0 EhrExtract includes_* → include_* renames ──
    XmlBmmOnlyField {
        spec: "EXTRACT_SPEC",
        wire_name: "include_multimedia",
        citation: "RM 1.2.0 renamed EXTRACT_SPEC.includes_multimedia → include_multimedia; ITS-XML 2.0.0 EhrExtract.xsd still declares includes_multimedia.",
        reason: "RM 1.2.0 EhrExtract includes_* → include_* rename vs ITS-XML 2.0.0.",
    },
    XmlBmmOnlyField {
        spec: "EXTRACT_VERSION_SPEC",
        wire_name: "include_revision_history",
        citation: "RM 1.2.0 renamed EXTRACT_VERSION_SPEC.includes_revision_history → include_revision_history; ITS-XML 2.0.0 EhrExtract.xsd still declares includes_revision_history.",
        reason: "RM 1.2.0 EhrExtract includes_* → include_* rename vs ITS-XML 2.0.0.",
    },
    XmlBmmOnlyField {
        spec: "EXTRACT_VERSION_SPEC",
        wire_name: "include_data",
        citation: "RM 1.2.0 renamed EXTRACT_VERSION_SPEC.includes_data → include_data; ITS-XML 2.0.0 EhrExtract.xsd still declares includes_data.",
        reason: "RM 1.2.0 EhrExtract includes_* → include_* rename vs ITS-XML 2.0.0.",
    },
    // ── VERSIONED_OBJECT base fields on the VERSIONED_* container types ──
    // uid/owner_id/time_created come from VERSIONED_OBJECT, defined ONLY in the
    // v2 Common.xsd (RM 1.1.0), which the emit-xml input deliberately does not
    // merge: v1 Version.xsd omits VERSIONED_OBJECT, and pulling in v2 Common.xsd
    // would re-shape the served VERSION-family types. These container types are
    // not served as canonical XML (406 at the REST edge); the base fields are
    // appended in canonical VERSIONED_OBJECT order so nothing is lost.
    XmlBmmOnlyField {
        spec: "VERSIONED_COMPOSITION",
        wire_name: "uid",
        citation: "VERSIONED_OBJECT.uid; base defined only in un-merged v2 Common.xsd (see module note).",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_COMPOSITION",
        wire_name: "owner_id",
        citation: "VERSIONED_OBJECT.owner_id; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_COMPOSITION",
        wire_name: "time_created",
        citation: "VERSIONED_OBJECT.time_created; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_EHR_ACCESS",
        wire_name: "uid",
        citation: "VERSIONED_OBJECT.uid; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_EHR_ACCESS",
        wire_name: "owner_id",
        citation: "VERSIONED_OBJECT.owner_id; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_EHR_ACCESS",
        wire_name: "time_created",
        citation: "VERSIONED_OBJECT.time_created; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_EHR_STATUS",
        wire_name: "uid",
        citation: "VERSIONED_OBJECT.uid; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_EHR_STATUS",
        wire_name: "owner_id",
        citation: "VERSIONED_OBJECT.owner_id; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_EHR_STATUS",
        wire_name: "time_created",
        citation: "VERSIONED_OBJECT.time_created; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_PARTY",
        wire_name: "uid",
        citation: "VERSIONED_OBJECT.uid; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_PARTY",
        wire_name: "owner_id",
        citation: "VERSIONED_OBJECT.owner_id; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
    XmlBmmOnlyField {
        spec: "VERSIONED_PARTY",
        wire_name: "time_created",
        citation: "VERSIONED_OBJECT.time_created; base defined only in un-merged v2 Common.xsd.",
        reason: "VERSIONED_OBJECT base field from the un-merged v2 Common.xsd.",
    },
];

/// Is `(spec, wire_name)` an accepted BMM-only field (on the allowlist)?
pub(crate) fn xml_bmm_only_allowed(spec: &str, wire_name: &str) -> bool {
    XML_BMM_ONLY_ALLOWLIST
        .iter()
        .any(|e| e.spec == spec && e.wire_name == wire_name)
}

// ─────────────────────────────────────────────────────────────────────────────
// Assertion-dialect leaf-predicate → runtime-function table
// ─────────────────────────────────────────────────────────────────────────────

/// One assertion-dialect leaf predicate the invariant classifier recognises
/// (`crate::analyze::invariants::RUNTIME_PREDICATES`), mapped to the named
/// runtime function the invariant-core layer calls to realize it. This is the
/// declarative bridge between the BMM assertion spelling and the hand-written
/// runtime in `openehr-rm`'s `validate.rs`.
pub(crate) struct DialectPredicate {
    /// The BMM assertion-dialect predicate spelling, as it appears in a
    /// `BMM_CLASS.invariants` expression (e.g. `valid_iso8601_date`).
    pub predicate: &'static str,
    /// The runtime function realizing it (in `openehr_rm::validate`, or a spec
    /// method for `is_valid_match_code`).
    pub runtime_fn: &'static str,
    /// Spec citation (BMM/RM section) for the predicate.
    pub citation: &'static str,
    /// One-line reason.
    pub reason: &'static str,
}

/// The assertion-dialect predicate → runtime-function map. Its predicate set is
/// exactly `crate::analyze::invariants::RUNTIME_PREDICATES` (the classifier's
/// recognised runtime-backed leaves) — the emitter-invariant suite pins the two
/// in lockstep, so a new recognised predicate without a runtime hook fails CI.
pub(crate) const DIALECT_PREDICATES: &[DialectPredicate] = &[
    DialectPredicate {
        predicate: "valid_iso8601_date",
        runtime_fn: "openehr_rm::validate::is_valid_iso_date",
        citation: "BASE Iso8601_date (docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_date.adoc)",
        reason: "ISO-8601 date well-formedness (openEHR partial-precision subset).",
    },
    DialectPredicate {
        predicate: "valid_iso8601_time",
        runtime_fn: "openehr_rm::validate::is_valid_iso_time",
        citation: "BASE Iso8601_time (docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_time.adoc)",
        reason: "ISO-8601 time well-formedness (openEHR partial-precision subset).",
    },
    DialectPredicate {
        predicate: "valid_iso8601_date_time",
        runtime_fn: "openehr_rm::validate::is_valid_iso_date_time",
        citation: "BASE Iso8601_date_time (docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_date_time.adoc)",
        reason: "ISO-8601 date-time well-formedness (openEHR partial-precision subset).",
    },
    DialectPredicate {
        predicate: "valid_iso8601_duration",
        runtime_fn: "openehr_rm::validate::is_valid_iso_duration",
        citation: "BASE Iso8601_duration (docs/specs/openehr/BASE/docs/UML/classes/org.openehr.base.foundation_types.iso8601_duration.adoc)",
        reason: "ISO-8601 duration well-formedness (openEHR sign + W designator deviation).",
    },
    DialectPredicate {
        predicate: "valid_percentage",
        runtime_fn: "openehr_rm::validate::valid_percentage",
        citation: "RM DV_AMOUNT.Accuracy_validity (docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_amount.adoc)",
        reason: "0 <= v <= 100 for a percent-recorded accuracy.",
    },
    DialectPredicate {
        predicate: "valid_magnitude_status",
        runtime_fn: "openehr_rm::validate::valid_magnitude_status",
        citation: "RM DV_QUANTIFIED.Magnitude_status_valid (docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_quantified.adoc)",
        reason: "magnitude_status is one of = < > <= >= ~.",
    },
    DialectPredicate {
        predicate: "valid_proportion_kind",
        runtime_fn: "openehr_rm::validate::valid_proportion_kind",
        citation: "RM PROPORTION_KIND (docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.proportion_kind.adoc)",
        reason: "proportion kind code is one of 0..=4.",
    },
    DialectPredicate {
        predicate: "is_valid_match_code",
        runtime_fn: "openehr_rm::data_types::text::term_mapping::TermMapping::is_valid_match_code",
        citation: "RM TERM_MAPPING.Match_valid (docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.term_mapping.adoc)",
        reason: "TERM_MAPPING.match is one of < = > ?.",
    },
];
