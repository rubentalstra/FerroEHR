//! The XSD-driven constraint-model emitter: generates a typed Rust model +
//! canonical-XML `ToXml`/`FromXml` impls for each vendored ARCHETYPE-family XSD
//! closure. Three targets, one pipeline (see [`ModelTarget`]):
//!
//! | target | subcommand | closure | root |
//! |---|---|---|---|
//! | `opt14` | `emit-opt` | `Template.xsd` + includes | `<template>` = `OPERATIONAL_TEMPLATE` |
//! | `aom2` | `emit-aom2` | `P_Archetype.xsd` + includes | `<archetype>` = `P_AUTHORED_ARCHETYPE` |
//! | `aom2_model` | `emit-aom2` | `Archetype.xsd` + includes | `<archetype>` = `AUTHORED_ARCHETYPE` |
//!
//! Unlike `emit-xml` (which drives off the BMM model), this emitter builds its
//! [`XmlType`]s directly from the XSD closure. The generate/resolve partition
//! (described below for `opt14`, and identical for the AOM2 targets):
//!
//! - a complexType that `openehr-base`/`openehr-rm` already export (with a
//!   generated `ToXml`/`FromXml`) **resolves** to that crate's prelude — the RM
//!   instance types (`CODE_PHRASE`, `DV_TEXT`, `DATA_VALUE`, …) are never
//!   re-generated;
//! - every other complexType (the AOM/OPT constraint model + the OPT envelope +
//!   the `IntervalOf*` helpers) is **generated** into `opt14`.
//!
//! Abstract types used as polymorphic slots (`C_OBJECT`, `C_ATTRIBUTE`,
//! `C_PRIMITIVE`, `EXPR_ITEM`, `STATE`) become untagged enums that dispatch on
//! `xsi:type`; the type declarations are emitted here, and their `ToXml`/
//! `FromXml` impls are produced by reusing [`crate::render::emit_xml::emit_to_xml`] /
//! [`crate::render::emit_xml::emit_from_xml`] over the [`XmlType`]s this module builds.
//!
//! # NOTE: `opt14` is a deliberately-separate OPT-XML wire adapter
//!
//! This module re-generates the AOM 1.4 `C_*` constraint tree that
//! `openehr-am::v1_4` (BMM-generated) already carries. That duplication is
//! **intentional and scoped**, not an oversight — the two models are *not*
//! structurally reconcilable. In summary, the Ocean **OPT-XML** wire shape
//! (`Template.xsd` + `OpenehrProfile.xsd`, the codegen input here) diverges from
//! the **AOM 1.4 BMM** logical model that drives `v1_4`:
//!
//! - **Different domain-type sets.** OPT-XML has `C_CODE_PHRASE`,
//!   `C_CODE_REFERENCE`, `C_DV_ORDINAL`, `C_DV_QUANTITY`, `C_DV_STATE`; the BMM
//!   `openehr_archetype_profile` has `C_CODED_TEXT`, `C_ORDINAL`, `C_QUANTITY`.
//!   `C_DV_STATE` and `C_CODE_REFERENCE` have no `v1_4` counterpart at all.
//! - **Different leaf shapes.** OPT-XML carries typed `assumed_value`
//!   (`DV_QUANTITY`/`DV_ORDINAL`/`DV_STATE`/`CODE_PHRASE`) and `C_DV_ORDINAL.list`
//!   of `DV_ORDINAL`; the BMM has `assumed_value: Any` (monomorphized to
//!   `serde_json::Value`) and `C_ORDINAL.list` of the constraint type `ORDINAL`.
//! - **Different `Interval` representation.** OPT-XML uses the XSD
//!   `IntervalOf*` shape (generated as `Intervalof*` here); the BMM uses
//!   `openehr_base::Interval<T>`.
//! - **OPT-envelope-only types** with no BMM/AOM-1.4 counterpart
//!   (`OPERATIONAL_TEMPLATE`, `C_ARCHETYPE_ROOT`, `T_COMPLEX_OBJECT`,
//!   `T_ATTRIBUTE`, `T_CONSTRAINT`, `FLAT_ARCHETYPE_ONTOLOGY`, `STATE_MACHINE`).
//!
//! Resolving the shared `C_*` to `v1_4` (the way RM leaves resolve to
//! `openehr_rm`/`openehr_base`) would force lossy mapping in both directions and
//! would require synthesizing an XML codec against types whose shapes do not
//! match the XSD element order / attribute split — the exact lossy shortcut
//! the wire model rejects. So `opt14` stays a standalone XSD-shaped wire adapter.
//!
//! **Drift guard:** because the two models are generated independently (BMM →
//! `v1_4`, XSD → `opt14`), an AOM-1.4 spec bump could silently drift them. The
//! compile-time inventory sentinel in
//! `crates/openehr-its/tests/opt14_v1_4_divergence.rs` fails the build if either
//! model gains or loses a constraint type, forcing a reconciliation + a design-record
//! update.

use crate::load::xsd::XsdModel;
use crate::plan::{XmlField, XmlType, XmlVariant};
use crate::render::emit_xml::{emit_from_xml, emit_to_xml};
use crate::render::naming;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// The Rust module path the OPT model's generated types live at.
///
/// The prelude is a MODEL PARAMETER, not a constant of this emitter: the same
/// XSD-driven pipeline emits several `openehr-its` submodules (`opt14`, `aom2`,
/// `aom2_model`), and every generated impl and defaulted literal must name the
/// module it is being emitted INTO. A hardcoded path here silently emits `aom2`
/// impls that reference `crate::opt14` types.
///
/// Each path ends at the defining `types` module rather than at the parent: the
/// parent re-exported these names with a glob until the zero-re-exports rule
/// removed it, and a generated impl must name where a type is DEFINED.
pub(crate) const OPT_PRELUDE: &str = "crate::opt14::types";

/// The Rust module path the AOM2 persistent-form model's generated types live at.
pub(crate) const AOM2_PRELUDE: &str = "crate::aom2::types";

/// The Rust module path the AOM2 model-form model's generated types live at.
pub(crate) const AOM2_MODEL_PRELUDE: &str = "crate::aom2_model::types";

/// The `opt14` emission target (OPT 1.4 operational templates).
pub(crate) static OPT_TARGET: ModelTarget = ModelTarget {
    generator: "emit-opt",
    prelude: OPT_PRELUDE,
    types_subject: "OPT 1.4 operational templates",
    impls_subject: "OPT 1.4",
    type_label: "AOM/OPT",
    field_label: "OPT",
};

/// The `aom2` emission target (the AOM2 persistent form, `P_Archetype.xsd`).
pub(crate) static AOM2_TARGET: ModelTarget = ModelTarget {
    generator: "emit-aom2",
    prelude: AOM2_PRELUDE,
    types_subject: "AOM2 persistent-form archetypes",
    impls_subject: "AOM2 persistent-form archetype",
    type_label: "AOM2 persistent-form",
    field_label: "AOM2 persistent-form",
};

/// The `aom2_model` emission target (the AOM2 model form, `Archetype.xsd`).
pub(crate) static AOM2_MODEL_TARGET: ModelTarget = ModelTarget {
    generator: "emit-aom2",
    prelude: AOM2_MODEL_PRELUDE,
    types_subject: "AOM2 model-form archetypes",
    impls_subject: "AOM2 model-form archetype",
    type_label: "AOM2 model-form",
    field_label: "AOM2 model-form",
};

/// Resource-metadata types that must be generated from the **OPT XSD** shape
/// rather than resolved to the `openehr-base`/`openehr-rm` impls: the BMM
/// (RM 1.2.0 / BASE 1.3.0) and the OPT XSD (Release 1.0.2) have diverged on
/// their optionality (e.g. `RESOURCE_DESCRIPTION.parent_resource` is mandatory
/// in the BMM but `minOccurs="0"` in the XSD — and the corpus omits it), so the
/// BMM-driven impls reject valid OPT `<description>` blocks. The OPT XSD is the
/// authority for OPT documents, so these are emitted fresh into `opt14`.
const FORCE_GENERATE: &[&str] = &[
    "AUTHORED_RESOURCE",
    "RESOURCE_DESCRIPTION",
    "RESOURCE_DESCRIPTION_ITEM",
    "TRANSLATION_DETAILS",
];

/// `StringDictionaryItem` is an XSD `simpleContent` helper (`<x id="k">v</x>`);
/// it is never generated as a struct — its repeated-element usage is emitted as
/// an order-preserving `IndexMap<String, String>` field.
///
/// NOTE: the XSD models this as an ordered `sequence`, so
/// `IndexMap` (insertion order = document order, keyed `.get()` for the
/// `WebTemplate` consumer) is used rather than the alphabetical `BTreeMap` the RM
/// `emit-xml` path uses — a `ToXml` re-serialization then preserves element
/// order. A genuinely duplicate `id` (not a conformant-OPT case) is still
/// collapsed last-wins by the map. The `OrderedDict` field target
/// (vs `emit-xml`'s `Hash`) selects this shape without affecting the RM codec.
const STRING_DICT_ITEM: &str = "StringDictionaryItem";

/// OPT-envelope sections carried as the verbatim XML subtree
/// (`crate::xml::runtime::XmlAny`) rather than a generated struct.
///
/// `T_VIEW` (the `<view>` presentation block) holds an **anonymous inline
/// complexType** (`T_VIEW.constraints` → nested `items` with an `id` attribute
/// and an `anySimpleType` value) that the XSD reader cannot flatten into a named
/// type; it carries only presentation hints (`pass_through` markers), never the
/// operational definition, so it is kept as read instead of modelled.
///
/// `T_CONSTRAINT` (the top-level `<constraints>` block) is **no longer opaque**
///: it is a named `T_ATTRIBUTE` → `T_COMPLEX_OBJECT` tree carrying
/// node `default_value` overlays, generated like any other type. Its
/// differential children may omit `rm_type_name`/`occurrences`/`node_id`
/// (they carry only `default_value` + `differential_path`); [`lenient_default`]
/// fills those, so the corpus parses cleanly and the `default_value`s are
/// preserved on the model for FLAT default-value population to consume.
const OPAQUE_TYPES: &[&str] = &["T_VIEW"];

/// A default expression for an XSD-mandatory field that real-world OPT exports
/// (Ocean/tooling) nevertheless omit — so `from_xml` fills it instead of
/// erroring. `node_id`/`purpose` fall back to empty; `occurrences`/`existence`
/// (both `IntervalOfInteger`) to the conservative `0..1` (present, optional
/// single) so a missing multiplicity never over-constrains. The expression is
/// emitted in the `opt14` impl context (prelude `crate::opt14`).
///
/// NOTE: a defaulted `occurrences`/`existence` of `0..1` is a
/// *fallback for non-conformant input only* — conformant OPTs always carry the
/// element. It is a guess (a node that should be `1..1` is silently made
/// optional-single), so any downstream multiplicity check (composition validation) must
/// resolve multiplicity from the `definition`/archetype, never trust a defaulted
/// `0..1` from this reader.
///
/// The match is on the field name **and its declared XSD type**, never the name
/// alone: `existence`/`occurrences` are `IntervalOfInteger` elements in the OPT
/// closure but `MultiplicityInterval` elements in the AOM2 model closure, and a
/// name-only match would emit an `Intervalofinteger` literal into a
/// `Multiplicityinterval` field. Keeping the type exact also keeps the concession
/// confined to the shapes it was adjudicated for.
fn lenient_default(field_name: &str, type_name: &str, prelude: &str) -> Option<String> {
    match (field_name, type_name) {
        // `rm_type_name` joins the lenient set for differential
        // `T_COMPLEX_OBJECT` children in real exports: `<constraints>` overlay
        // nodes may carry only `default_value` + `differential_path`
        // (e.g. the corpus `non_unique_aql_paths.opt`).
        ("node_id" | "purpose" | "rm_type_name", "xs:string") => Some("String::new()".to_owned()),
        ("occurrences" | "existence", "IntervalOfInteger") => Some(format!(
            "{prelude}::Intervalofinteger {{ \
             lower_included: Some(true), upper_included: Some(true), \
             lower_unbounded: false, upper_unbounded: false, \
             lower: Some(0), upper: Some(1) }}"
        )),
        _ => None,
    }
}

/// How a referenced XSD type name resolves for a generated field.
enum Resolved {
    /// A Rust primitive (`String`/`bool`/`i32`/`i64`/`f64`).
    Primitive(&'static str),
    /// `xs:anyType`/`xs:anySimpleType`/anonymous-inline → the verbatim XML
    /// subtree carrier `crate::xml::runtime::XmlAny` (attributes, text and children in
    /// document order, re-emitted as read).
    ///
    /// NOTE: `AM aom14 §EXPR_LEAF Class` types `item: Any`, so the payload
    /// domain of the schema's open slots is open too and no closed set of
    /// generated types can be dispatched to.
    Any,
    /// A repeated `StringDictionaryItem` element group → order-preserving
    /// `IndexMap<String,String>` (target `OrderedDict`).
    Hash,
    /// A type emitted by `openehr-base` — the full generation-module type path.
    Base(String),
    /// A type emitted by `openehr-rm` — the full generation-module type path.
    Rm(String),
    /// A generated `opt14` type; the flag is `true` when it is a generated enum
    /// (a polymorphic slot), which single-valued fields must `Box` to stay sized.
    Gen(String, bool),
}

/// Everything about the emission TARGET that varies between the XSD-driven
/// closures this emitter serves (`opt14`, `aom2`, `aom2_model`).
///
/// All of it is a model parameter rather than a constant for the same reason the
/// prelude is: three modules come out of one emitter, and a hardcoded value
/// silently stamps one module's identity onto another's files — an `aom2_model`
/// impl referencing `crate::opt14` types, or an `emit-aom2` output whose banner
/// tells the reader to re-run `emit-opt`.
pub(crate) struct ModelTarget {
    /// The `openehr-codegen` subcommand that regenerates the target, named in
    /// every `// @generated` banner.
    pub(crate) generator: &'static str,
    /// The `openehr-its` module path the emitted types live at
    /// (e.g. [`OPT_PRELUDE`]).
    pub(crate) prelude: &'static str,
    /// What `types.rs` is a model OF ("OPT 1.4 operational templates").
    pub(crate) types_subject: &'static str,
    /// The model name used in the `impls.rs` title ("OPT 1.4").
    pub(crate) impls_subject: &'static str,
    /// The spec-family label substituted into a per-type doc comment: "AOM/OPT"
    /// yields `openEHR AOM/OPT <spec name>.`
    pub(crate) type_label: &'static str,
    /// The spec-family label substituted into a per-field doc comment: "OPT"
    /// yields `… attribute/element of the OPT <spec name> XSD type.`
    pub(crate) field_label: &'static str,
}

/// The generate/resolve model for one XSD closure.
pub(crate) struct OptModel<'a> {
    xsd: &'a XsdModel,
    /// Spec class name → full generation-module path, per dependency crate
    /// (the openehr-base / openehr-rm generation the shared XSD types resolve
    /// to — full defining-module paths, never a prelude).
    base_paths: &'a BTreeMap<String, String>,
    rm_paths: &'a BTreeMap<String, String>,
    /// Concrete + abstract complexTypes we generate (spec names).
    generate: BTreeSet<String>,
    /// The subset of `generate` that are abstract polymorphic slots → enums.
    enum_specs: BTreeSet<String>,
    /// The emission target's identity (module path, banners, doc labels).
    target: &'static ModelTarget,
}

/// A generated field: the `emit_xml` [`XmlField`] plus the Rust type for its
/// struct-field declaration (which `emit_xml`'s impls then infer against).
struct OptField {
    xml: XmlField,
    decl_type: String,
}

impl<'a> OptModel<'a> {
    /// Build the model from the parsed XSD closure and the base/rm
    /// spec-name → module-path maps.
    #[must_use]
    pub(crate) fn new(
        xsd: &'a XsdModel,
        base_paths: &'a BTreeMap<String, String>,
        rm_paths: &'a BTreeMap<String, String>,
        target: &'static ModelTarget,
    ) -> Self {
        let generate: BTreeSet<String> = xsd
            .types
            .keys()
            .filter(|n| {
                n.as_str() != STRING_DICT_ITEM
                    && !OPAQUE_TYPES.contains(&n.as_str())
                    && (FORCE_GENERATE.contains(&n.as_str())
                        || (!base_paths.contains_key(*n) && !rm_paths.contains_key(*n)))
            })
            .cloned()
            .collect();
        let enum_specs: BTreeSet<String> = generate
            .iter()
            .filter(|n| {
                xsd.types.get(*n).is_some_and(|t| t.is_abstract) && !xsd.descendants(n).is_empty()
            })
            .cloned()
            .collect();
        Self {
            xsd,
            base_paths,
            rm_paths,
            generate,
            enum_specs,
            target,
        }
    }

    /// Resolve an XSD type name (element/attribute `type`) to a Rust binding.
    fn resolve(&self, type_name: &str) -> Resolved {
        if type_name.is_empty() {
            return Resolved::Any; // anonymous inline complexType
        }
        if type_name == STRING_DICT_ITEM {
            return Resolved::Hash;
        }
        if OPAQUE_TYPES.contains(&type_name) {
            return Resolved::Any; // the differential/presentation envelope, carried verbatim
        }
        // XSD-namespace primitive (`xs:` / `xsd:`).
        if let Some(local) = type_name
            .strip_prefix("xs:")
            .or_else(|| type_name.strip_prefix("xsd:"))
        {
            return Self::xs_primitive(local);
        }
        if self.xsd.types.contains_key(type_name) {
            let rust = naming::type_name(type_name);
            // A generated type wins over the base/rm resolution (the FORCE_GENERATE
            // resource types live in both sets — the generated one is authoritative).
            if self.generate.contains(type_name) {
                return Resolved::Gen(rust, self.enum_specs.contains(type_name));
            }
            if let Some(path) = self.base_paths.get(type_name) {
                return Resolved::Base(format!("{path}::{rust}"));
            }
            if let Some(path) = self.rm_paths.get(type_name) {
                return Resolved::Rm(format!("{path}::{rust}"));
            }
        }
        // A named `xs:simpleType` (restriction over string/integer): text on the
        // wire — `OPERATOR_KIND`, `Iso8601Date`, `VALIDITY_KIND`, patterns, … .
        //
        // NOTE: the AOM integer-enum `*_KIND` restrictions are carried verbatim as
        // their wire text, round-tripping losslessly and deferring the validity
        // and operator semantics to the consumer (#2271).
        Resolved::Primitive("String")
    }

    /// Map an `xs:`-local primitive to a Rust type.
    fn xs_primitive(local: &str) -> Resolved {
        match local {
            "anyType" | "anySimpleType" => Resolved::Any,
            "boolean" => Resolved::Primitive("bool"),
            "int" | "integer" | "nonNegativeInteger" | "positiveInteger" | "short" => {
                Resolved::Primitive("i32")
            }
            "long" => Resolved::Primitive("i64"),
            "decimal" | "double" | "float" => Resolved::Primitive("f64"),
            // string, token, normalizedString, anyURI, base64Binary, dateTime, …
            _ => Resolved::Primitive("String"),
        }
    }

    /// The unwrapped Rust type text + the declared-slot spec name (for the
    /// `xsi:type`-suppression `declared` argument).
    fn base_decl(res: &Resolved, raw_spec: &str) -> (String, String) {
        match res {
            Resolved::Primitive(p) => ((*p).to_string(), String::new()),
            Resolved::Any => ("crate::xml::runtime::XmlAny".to_string(), String::new()),
            Resolved::Hash => (
                "indexmap::IndexMap<String, String>".to_string(),
                String::new(),
            ),
            Resolved::Base(n) | Resolved::Rm(n) | Resolved::Gen(n, _) => {
                (n.clone(), raw_spec.to_string())
            }
        }
    }

    /// The flattened fields (attributes then elements, ancestor-first) of a
    /// concrete generated type.
    fn fields(&self, spec: &str) -> Vec<OptField> {
        let (attrs, elems) = self.xsd.flattened(spec);
        let mut out = Vec::new();
        for a in &attrs {
            let rust_name = naming::field_ident(&a.name);
            let decl_type = if a.required {
                "String".to_string()
            } else {
                "Option<String>".to_string()
            };
            out.push(OptField {
                xml: XmlField {
                    wire_name: a.name.clone(),
                    rust_name,
                    optional: !a.required,
                    multiple: false,
                    target: String::new(),
                    map_value: None,
                    default: None,
                    nonempty: false,
                },
                decl_type,
            });
        }
        for e in &elems {
            let res = self.resolve(&e.type_name);
            let rust_name = naming::field_ident(&e.name);
            let (base, target) = Self::base_decl(&res, &e.type_name);
            let is_hash = matches!(res, Resolved::Hash);
            let is_gen_enum = matches!(res, Resolved::Gen(_, true));
            let is_bool = matches!(res, Resolved::Primitive("bool"));

            let (decl_type, xml_field);
            if is_hash {
                decl_type = if e.optional {
                    format!("Option<{base}>")
                } else {
                    base
                };
                xml_field = XmlField {
                    wire_name: e.name.clone(),
                    rust_name,
                    optional: e.optional,
                    multiple: false,
                    target: "OrderedDict".to_string(),
                    map_value: Some("String".to_string()),
                    default: None,
                    nonempty: false,
                };
            } else {
                // Box a single-valued reference to a generated enum: those slots
                // (`EXPR_ITEM`, `C_PRIMITIVE`, `STATE`) are recursive.
                let inner = if !e.multiple && is_gen_enum {
                    format!("Box<{base}>")
                } else {
                    base
                };
                decl_type = if e.multiple {
                    format!("Vec<{inner}>")
                } else if e.optional {
                    format!("Option<{inner}>")
                } else {
                    inner
                };
                // A mandatory scalar bool absent on the wire (openEHR `Interval`
                // boundedness flags default false) → fall back to `false`. Some
                // XSD-mandatory fields are omitted by real-world OPT exports
                // (Ocean/tool laxity) — default them leniently so those templates
                // still parse (a wire adapter must ingest imperfect real OPTs).
                let default = if is_bool && !e.optional && !e.multiple {
                    Some("false".to_string())
                } else if !e.optional && !e.multiple {
                    lenient_default(&e.name, &e.type_name, self.target.prelude)
                } else {
                    None
                };
                xml_field = XmlField {
                    wire_name: e.name.clone(),
                    rust_name,
                    optional: e.optional && !e.multiple,
                    multiple: e.multiple,
                    target,
                    map_value: None,
                    default,
                    nonempty: false,
                };
            }
            out.push(OptField {
                xml: xml_field,
                decl_type,
            });
        }
        out
    }

    /// Build the [`XmlType`] for a generated spec (for the `emit_xml` impls).
    fn xml_type(&self, spec: &str) -> Option<XmlType> {
        self.xsd.types.get(spec)?;
        let rust = naming::type_name(spec);
        if self.enum_specs.contains(spec) {
            let descendants = self.xsd.descendants(spec);
            let variants = descendants
                .iter()
                .map(|d| XmlVariant {
                    ident: naming::type_name(d),
                })
                .collect();
            let dispatch = descendants
                .iter()
                .map(|d| (d.clone(), naming::type_name(d)))
                .collect();
            Some(XmlType::Enum {
                spec: spec.to_string(),
                rust,
                generics: Vec::new(),
                variants,
                dispatch,
            })
        } else {
            Some(XmlType::Struct {
                spec: spec.to_string(),
                rust,
                generics: Vec::new(),
                fields: self.fields(spec).into_iter().map(|f| f.xml).collect(),
            })
        }
    }

    /// Emit the type declarations (the target module's `types.rs`).
    #[must_use]
    pub(crate) fn emit_types(&self) -> String {
        let mut b = String::new();
        let _ = write!(
            b,
            "// @generated by openehr-codegen ({}) — DO NOT EDIT.\n\
             //! Typed Rust model for openEHR {}.\n\n",
            self.target.generator, self.target.types_subject
        );
        b.push_str(
            "#![allow(\n    \
             dead_code,\n    \
             non_snake_case,\n    \
             non_camel_case_types,\n    \
             clippy::all,\n    \
             clippy::pedantic,\n    \
             clippy::nursery,\n    \
             reason = \"mechanically generated model text: the XSD is emitted in \
             full under its own spec-owned element/attribute spellings, so naming, \
             style and dead-code lints do not apply — the hand-written runtime \
             carries the lint bar\"\n\
             )]\n\n",
        );
        for spec in &self.generate {
            let Some(ty) = self.xsd.types.get(spec) else {
                continue;
            };
            let rust = naming::type_name(spec);
            let mut doc = format!("/// openEHR {} `{spec}`.\n", self.target.type_label);
            // An `abstract="true"` complexType that NOTHING in the closure derives
            // from cannot be an `xsi:type` dispatch enum, and it is still a slot
            // type real documents must fill (`Archetype.xsd` declares
            // `C_ATTRIBUTE` abstract yet types `C_COMPLEX_OBJECT.attributes` with
            // it). It is emitted as the plain shape a document has to present —
            // never dropped, which would leave a dangling field type.
            //
            // NOTE: such a type is NOT a variant of the enums it descends from —
            // the XSD `xsi:type` rule — while its concrete descendants all are,
            // which `the_concrete_only_variant_reading_loses_no_document_shape` pins.
            if ty.is_abstract && !self.enum_specs.contains(spec) {
                let _ = writeln!(
                    b,
                    "/// openEHR {} `{spec}` — declared `abstract` in the XSD with no\n\
                     /// concrete subtype in this schema closure, so it is emitted as the\n\
                     /// plain shape a conforming document must present at its slots.",
                    self.target.type_label
                );
                doc = String::new();
            }
            if self.enum_specs.contains(spec) {
                let descendants = self.xsd.descendants(spec);
                b.push_str(&doc);
                // These are XML-only models (`ToXml`/`FromXml`); they carry no
                // serde — the types are plain data records parsed from XML.
                b.push_str("#[derive(Debug, Clone, PartialEq)]\n");
                let _ = writeln!(b, "pub enum {rust} {{");
                for d in &descendants {
                    let ident = naming::type_name(d);
                    // A variant is a public item `missing_docs` checks; the XSD
                    // carries no per-subtype prose, so name the subtype.
                    let _ = writeln!(
                        b,
                        "    /// The {} `{d}` subtype of `{spec}`.",
                        self.target.field_label
                    );
                    let _ = writeln!(b, "    {ident}({ident}),");
                }
                b.push_str("}\n\n");
            } else {
                b.push_str(&doc);
                b.push_str("#[derive(Debug, Clone, PartialEq)]\n");
                let _ = writeln!(b, "pub struct {rust} {{");
                for f in self.fields(spec) {
                    let _ = writeln!(
                        b,
                        "    /// The `{}` attribute/element of the {} `{spec}` XSD type.",
                        f.xml.wire_name, self.target.field_label
                    );
                    let _ = writeln!(b, "    pub {}: {},", f.xml.rust_name, f.decl_type);
                }
                b.push_str("}\n\n");
            }
        }
        b
    }

    /// Emit the `ToXml`/`FromXml` impls (the target module's `impls.rs`), reusing
    /// the `emit-xml` per-type emitters over the XSD-derived [`XmlType`]s.
    #[must_use]
    pub(crate) fn emit_impls(&self, unmatched: &mut Vec<(String, String)>) -> String {
        let mut b = String::new();
        let _ = write!(
            b,
            "// @generated by openehr-codegen ({}) — DO NOT EDIT.\n\
             //! Canonical-XML `ToXml`/`FromXml` impls for the {} model.\n\n",
            self.target.generator, self.target.impls_subject
        );
        b.push_str(
            "#![allow(\n    \
             non_snake_case,\n    \
             clippy::all,\n    \
             clippy::pedantic,\n    \
             clippy::nursery,\n    \
             unused_variables,\n    \
             unused_mut,\n    \
             unused_qualifications,\n    \
             unused_imports,\n    \
             reason = \"mechanically generated codec text: every runtime item is \
             named by its full path and every branch shape is emitted uniformly, \
             so style and unused-binding lints do not apply — the hand-written \
             runtime carries the lint bar\"\n\
             )]\n\
             use crate::xml::runtime::{ToXml, FromXml, XmlEvent, XmlError};\n\n",
        );
        for spec in &self.generate {
            if let Some(ty) = self.xml_type(spec) {
                emit_to_xml(&mut b, &ty, self.target.prelude, self.xsd, unmatched);
                emit_from_xml(&mut b, &ty, self.target.prelude, self.xsd, None);
            }
        }
        b
    }
}

/// One `from_xml`/`to_xml` pair to emit for a generated XML module.
pub(crate) struct EntryPoint {
    /// Appended to `from_xml`/`to_xml` so several roots can coexist in one
    /// module (`""` for the primary pair).
    pub(crate) suffix: &'static str,
    /// The generated Rust type of the document root.
    pub(crate) root_rust: &'static str,
    /// The XML root element name.
    pub(crate) root_element: &'static str,
    /// Human phrase for the doc comment ("operational template").
    pub(crate) what: &'static str,
    /// The indefinite article for [`Self::root_rust`] ("an"/"a"), so the
    /// generated prose stays grammatical across roots.
    pub(crate) article: &'static str,
    /// The wire label used in the serialize doc ("OPT 1.4 XML").
    pub(crate) wire: &'static str,
    /// The spec/XSD type name, named in the module docs.
    pub(crate) spec_name: &'static str,
}

/// What a generated XML module's `mod.rs` should say and expose.
pub(crate) struct ModuleSpec {
    /// The `openehr-codegen` subcommand that produced it.
    pub(crate) generator: &'static str,
    /// The module's one-line `//!` title.
    pub(crate) title: &'static str,
    /// The document roots to expose.
    pub(crate) entry_points: &'static [EntryPoint],
    /// Extra `//!` paragraphs appended after the entry-point lines — the
    /// module's adjudications (root-type choice, corpus ceiling). One entry per
    /// paragraph; a blank line is emitted between them.
    pub(crate) notes: &'static [&'static str],
}

/// The `opt14/mod.rs` surface.
pub(crate) static OPT_MODULE: ModuleSpec = ModuleSpec {
    generator: "emit-opt",
    title: "openEHR OPT 1.4 operational-template model + canonical-XML codec.",
    entry_points: &[EntryPoint {
        suffix: "",
        root_rust: "OperationalTemplate",
        root_element: "template",
        what: "operational template",
        article: "an",
        wire: "OPT 1.4 XML",
        spec_name: "OPERATIONAL_TEMPLATE",
    }],
    notes: &[],
};

/// The `aom2/mod.rs` surface (the AOM2 persistent form).
pub(crate) static AOM2_MODULE: ModuleSpec = ModuleSpec {
    generator: "emit-aom2",
    title: "openEHR AOM2 persistent-form archetype model + canonical-XML codec.",
    entry_points: &[EntryPoint {
        suffix: "",
        root_rust: "PAuthoredArchetype",
        root_element: "archetype",
        what: "persistent-form AOM2 archetype",
        article: "a",
        wire: "AOM2 persistent-form archetype XML",
        spec_name: "P_AUTHORED_ARCHETYPE",
    }],
    notes: &[
        "This is the PERSISTENT (`P_AOM`) AOM2 serialization — `P_Archetype.xsd`, whose\n\
         own header calls it \"uses P_AOM types - much more space efficient\". It is the\n\
         form the bundle's 8 `AOM2/examples/*.xml` documents carry\n\
         (`xsi:schemaLocation=\"… ../P_Archetype.xsd\"`). The bundle's other archetype\n\
         serialization — the AOM model form — is [`crate::aom2_model`].",
    ],
};

/// The `aom2_model/mod.rs` surface (the AOM2 model form).
pub(crate) static AOM2_MODEL_MODULE: ModuleSpec = ModuleSpec {
    generator: "emit-aom2",
    title: "openEHR AOM2 model-form archetype model + canonical-XML codec.",
    entry_points: &[EntryPoint {
        suffix: "",
        root_rust: "AuthoredArchetype",
        root_element: "archetype",
        what: "model-form AOM2 archetype",
        article: "a",
        wire: "AOM2 model-form archetype XML",
        spec_name: "AUTHORED_ARCHETYPE",
    }],
    notes: &[
        "This is the AOM MODEL form — `Archetype.xsd`, whose own header calls it \"uses\n\
         AOM-like types - not space-efficient\": the AOM2 classes themselves\n\
         (`C_COMPLEX_OBJECT`, `C_ATTRIBUTE`, `ARCHETYPE_TERMINOLOGY`,\n\
         `MultiplicityInterval`), as opposed to the persistent `P_AOM` form in\n\
         [`crate::aom2`]. Both schemas declare the top-level element `<archetype>`, so\n\
         a document's ROOT TYPE — not its element name — decides which module reads it.",
        "Root type: `Archetype.xsd` declares\n\
         `<xs:element name=\"archetype\" type=\"ARCHETYPE\"/>`, but `ARCHETYPE` is\n\
         `abstract=\"true\"` and no complexType in the closure derives from it —\n\
         `AUTHORED_ARCHETYPE` extends `AUTHORED_RESOURCE` and re-uses the archetype body\n\
         through `<xs:group ref=\"ARCHETYPE\"/>` instead. `AUTHORED_ARCHETYPE` is\n\
         therefore the only instantiable archetype root the schema offers, and the entry\n\
         points here are typed to it; `Archetype` itself is not emitted, because an\n\
         abstract type with no concrete subtype can never appear on the wire.",
        "Corpus ceiling: openEHR publishes NO model-form instance documents. All 8\n\
         `AOM2/examples/*.xml` are persistent-form, `openEHR/adl-archetypes` publishes\n\
         ADL text only, and `specifications-ITS-XML` has no further branch to vendor. So\n\
         this codec is gated by construct → serialize → parse self-consistency\n\
         (`openehr-its` `tests/it/aom2_model_xml.rs`), not by an upstream corpus. That\n\
         ceiling is stated rather than implied.",
    ],
};

/// Emit a generated XML module's `mod.rs` — wiring, re-export, and one
/// `from_xml`/`to_xml` pair per document root.
///
/// Shared by every XSD-driven emitter so the module surface stays identical
/// across them; the roots are the only thing that varies.
#[must_use]
pub(crate) fn emit_module(spec: &ModuleSpec) -> String {
    let mut b = String::new();
    let _ = writeln!(
        b,
        "// @generated by openehr-codegen ({}) — DO NOT EDIT.",
        spec.generator
    );
    let _ = writeln!(b, "//! {}", spec.title);
    let _ = writeln!(b, "//!");
    for e in spec.entry_points {
        let _ = writeln!(
            b,
            "//! Parse {} {} with [`from_xml{}`]; the root element is `<{}>` (`{}`).",
            e.article, e.what, e.suffix, e.root_element, e.spec_name
        );
    }
    for note in spec.notes {
        let _ = writeln!(b, "//!");
        for line in note.lines() {
            let _ = writeln!(b, "//! {line}");
        }
    }
    // `pub mod types`, never a star re-export: the zero-re-exports rule means an
    // import names its defining module, and a glob would also hide which of the
    // three generated modules a name came from at the use site.
    let _ = writeln!(b, "\nmod impls;\npub mod types;");
    for e in spec.entry_points {
        let _ = writeln!(
            b,
            "\n/// Parse {} {} XML document into {} [`types::{}`].\n\
             ///\n\
             /// # Errors\n\
             /// Propagates canonical-XML parse errors.\n\
             pub fn from_xml{}(xml: &str) -> Result<types::{}, crate::xml::runtime::XmlError> {{\n\
             crate::xml::runtime::from_xml(xml)\n\
             }}",
            e.article, e.what, e.article, e.root_rust, e.suffix, e.root_rust
        );
        let _ = writeln!(
            b,
            "\n/// Serialize {} [`types::{}`] back to {} (root `<{}>`,\n\
             /// `http://schemas.openehr.org/v1`).\n\
             ///\n\
             /// # Errors\n\
             /// Propagates canonical-XML serialization errors.\n\
             pub fn to_xml{}(value: &types::{}) -> Result<String, crate::xml::runtime::XmlError> {{\n\
             crate::xml::runtime::to_xml(value, \"{}\", crate::xml::runtime::Namespace::V1)\n\
             }}",
            e.article, e.root_rust, e.wire, e.root_element, e.suffix, e.root_rust, e.root_element
        );
    }
    b
}
