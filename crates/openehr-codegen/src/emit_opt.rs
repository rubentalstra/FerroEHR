//! OPT 1.4 emitter (`emit-opt`): generates a typed Rust model
//! (`opt14`) + canonical-XML `ToXml`/`FromXml` impls for the openEHR
//! **operational template** XML (`<template>` = `OPERATIONAL_TEMPLATE`).
//!
//! Unlike `emit-xml` (which drives off the BMM model), this emitter builds its
//! [`XmlType`]s directly from the **AM/OPT constraint XSD closure**
//! (`Template.xsd` + its includes). The generate/resolve partition:
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
//! `FromXml` impls are produced by reusing [`emit_xml::emit_to_xml`] /
//! [`emit_xml::emit_from_xml`] over the [`XmlType`]s this module builds.
//!
//! # PORT NOTE: `opt14` is a deliberately-separate OPT-XML wire adapter
//!
//! This module re-generates the AOM 1.4 `C_*` constraint tree that
//! `openehr-am::am14` (BMM-generated) already carries. That duplication is
//! **intentional and scoped**, not an oversight — the two models are *not*
//! structurally reconcilable (see `docs/spec-audit/findings/09-templates-opt14.md`
//! F-09-02 and `the opt14-wire-model design record` for the field-by-field
//! evidence and the full rationale). In summary, the Ocean **OPT-XML** wire shape
//! (`Template.xsd` + `OpenehrProfile.xsd`, the codegen input here) diverges from
//! the **AOM 1.4 BMM** logical model that drives `am14`:
//!
//! - **Different domain-type sets.** OPT-XML has `C_CODE_PHRASE`,
//!   `C_CODE_REFERENCE`, `C_DV_ORDINAL`, `C_DV_QUANTITY`, `C_DV_STATE`; the BMM
//!   `openehr_archetype_profile` has `C_CODED_TEXT`, `C_ORDINAL`, `C_QUANTITY`.
//!   `C_DV_STATE` and `C_CODE_REFERENCE` have no `am14` counterpart at all.
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
//! Resolving the shared `C_*` to `am14` (the way RM leaves resolve to
//! `openehr_rm`/`openehr_base`) would force lossy mapping in both directions and
//! would require synthesizing an XML codec against types whose shapes do not
//! match the XSD element order / attribute split — the exact lossy shortcut
//! the wire model rejects. So `opt14` stays a standalone XSD-shaped wire adapter.
//!
//! **Drift guard:** because the two models are generated independently (BMM →
//! `am14`, XSD → `opt14`), an AOM-1.4 spec bump could silently drift them. The
//! compile-time inventory sentinel in
//! `crates/openehr-its/tests/opt14_am14_divergence.rs` fails the build if either
//! model gains or loses a constraint type, forcing a reconciliation + a design-record
//! update.

use crate::emit::{XmlField, XmlType, XmlVariant};
use crate::emit_xml::{emit_from_xml, emit_to_xml};
use crate::naming;
use crate::xsd::XsdModel;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The Rust module path the generated types live at (an `openehr-its` submodule).
const PRELUDE: &str = "crate::opt14";

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
/// PORT NOTE (F-09-05): the XSD models this as an ordered `sequence`, so
/// `IndexMap` (insertion order = document order, keyed `.get()` for the
/// `WebTemplate` consumer) is used rather than the alphabetical `BTreeMap` the RM
/// `emit-xml` path uses — a `ToXml` re-serialization then preserves element
/// order. A genuinely duplicate `id` (not a conformant-OPT case) is still
/// collapsed last-wins by the map. The `OrderedDict` field target
/// (vs `emit-xml`'s `Hash`) selects this shape without affecting the RM codec.
const STRING_DICT_ITEM: &str = "StringDictionaryItem";

/// OPT-envelope sections captured as opaque `serde_json::Value` (parsed by
/// skipping their subtree).
///
/// `T_VIEW` (the `<view>` presentation block) holds an **anonymous inline
/// complexType** (`T_VIEW.constraints` → nested `items` with an `id` attribute
/// and an `anySimpleType` value) that the XSD reader cannot flatten into a named
/// type; it carries only presentation hints (`pass_through` markers), never the
/// operational definition, so it is skipped.
///
/// `T_CONSTRAINT` (the top-level `<constraints>` block) is **no longer opaque**
/// (F-09-03): it is a named `T_ATTRIBUTE` → `T_COMPLEX_OBJECT` tree carrying
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
/// PORT NOTE (F-09-07): a defaulted `occurrences`/`existence` of `0..1` is a
/// *fallback for non-conformant input only* — conformant OPTs always carry the
/// element. It is a guess (a node that should be `1..1` is silently made
/// optional-single), so any downstream multiplicity check (P15 validation) must
/// resolve multiplicity from the `definition`/archetype, never trust a defaulted
/// `0..1` from this reader.
fn lenient_default(field_name: &str) -> Option<String> {
    match field_name {
        // `rm_type_name` joins the lenient set for differential
        // `T_COMPLEX_OBJECT` children in real exports: `<constraints>` overlay
        // nodes may carry only `default_value` + `differential_path`
        // (e.g. the corpus `non_unique_aql_paths.opt`).
        "node_id" | "purpose" | "rm_type_name" => Some("String::new()".to_owned()),
        "occurrences" | "existence" => Some(
            "crate::opt14::Intervalofinteger { \
             lower_included: Some(true), upper_included: Some(true), \
             lower_unbounded: false, upper_unbounded: false, \
             lower: Some(0), upper: Some(1) }"
                .to_owned(),
        ),
        _ => None,
    }
}

/// How a referenced XSD type name resolves for a generated field.
enum Resolved {
    /// A Rust primitive (`String`/`bool`/`i32`/`i64`/`f64`).
    Primitive(&'static str),
    /// `xs:anyType`/`xs:anySimpleType`/anonymous-inline → `serde_json::Value`
    /// (parsed by skipping the subtree — lossy but never errors).
    Value,
    /// A repeated `StringDictionaryItem` element group → order-preserving
    /// `IndexMap<String,String>` (target `OrderedDict`).
    Hash,
    /// A type exported by `openehr-base` (resolved to its prelude).
    Base(String),
    /// A type exported by `openehr-rm` (resolved to its prelude).
    Rm(String),
    /// A generated `opt14` type; the flag is `true` when it is a generated enum
    /// (a polymorphic slot), which single-valued fields must `Box` to stay sized.
    Gen(String, bool),
}

/// The generate/resolve model for the OPT XSD closure.
pub struct OptModel<'a> {
    xsd: &'a XsdModel,
    base_specs: &'a BTreeSet<String>,
    rm_specs: &'a BTreeSet<String>,
    /// Concrete + abstract complexTypes we generate (spec names).
    generate: BTreeSet<String>,
    /// The subset of `generate` that are abstract polymorphic slots → enums.
    enum_specs: BTreeSet<String>,
}

/// A generated field: the `emit_xml` [`XmlField`] plus the Rust type for its
/// struct-field declaration (which `emit_xml`'s impls then infer against).
struct OptField {
    xml: XmlField,
    decl_type: String,
    rename: Option<String>,
}

impl<'a> OptModel<'a> {
    /// Build the model from the parsed XSD closure and the base/rm export sets.
    #[must_use]
    pub fn new(
        xsd: &'a XsdModel,
        base_specs: &'a BTreeSet<String>,
        rm_specs: &'a BTreeSet<String>,
    ) -> Self {
        let generate: BTreeSet<String> = xsd
            .types
            .keys()
            .filter(|n| {
                n.as_str() != STRING_DICT_ITEM
                    && !OPAQUE_TYPES.contains(&n.as_str())
                    && (FORCE_GENERATE.contains(&n.as_str())
                        || (!base_specs.contains(*n) && !rm_specs.contains(*n)))
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
            base_specs,
            rm_specs,
            generate,
            enum_specs,
        }
    }

    /// Resolve an XSD type name (element/attribute `type`) to a Rust binding.
    fn resolve(&self, type_name: &str) -> Resolved {
        if type_name.is_empty() {
            return Resolved::Value; // anonymous inline complexType
        }
        if type_name == STRING_DICT_ITEM {
            return Resolved::Hash;
        }
        if OPAQUE_TYPES.contains(&type_name) {
            return Resolved::Value; // skip the differential/presentation envelope
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
            if self.base_specs.contains(type_name) {
                return Resolved::Base(rust);
            }
            if self.rm_specs.contains(type_name) {
                return Resolved::Rm(rust);
            }
        }
        // A named `xs:simpleType` (restriction over string/integer): text on the
        // wire — `OPERATOR_KIND`, `Iso8601Date`, `VALIDITY_KIND`, patterns, … .
        //
        // PORT NOTE (F-09-06): the AOM integer-enum `*_KIND` restrictions
        // (`VALIDITY_KIND` = 1001/1002/1003, `OPERATOR_KIND` = 2001..2024) are
        // carried verbatim as their wire text (`"1001"`, `"2001"`), not decoded
        // to a typed enum. This round-trips losslessly (text in, text out) and
        // defers the validity/operator *semantics* to the consumer; the
        // WebTemplate builder does not read these fields, so no fidelity is lost
        // in practice. Emitting typed enums from the XSD `enumeration` facets is
        // a possible future enhancement.
        Resolved::Primitive("String")
    }

    /// Map an `xs:`-local primitive to a Rust type.
    fn xs_primitive(local: &str) -> Resolved {
        match local {
            "anyType" | "anySimpleType" => Resolved::Value,
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
            Resolved::Value => ("serde_json::Value".to_string(), String::new()),
            Resolved::Hash => (
                "indexmap::IndexMap<String, String>".to_string(),
                String::new(),
            ),
            Resolved::Base(n) => (format!("openehr_base::prelude::{n}"), raw_spec.to_string()),
            Resolved::Rm(n) => (format!("openehr_rm::prelude::{n}"), raw_spec.to_string()),
            Resolved::Gen(n, _) => (n.clone(), raw_spec.to_string()),
        }
    }

    /// The flattened fields (attributes then elements, ancestor-first) of a
    /// concrete generated type.
    fn fields(&self, spec: &str) -> Vec<OptField> {
        let (attrs, elems) = self.xsd.flattened(spec);
        let mut out = Vec::new();
        for a in &attrs {
            let rust_name = naming::field_ident(&a.name);
            let rename = naming::serde_rename(&a.name, &rust_name);
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
                },
                decl_type,
                rename,
            });
        }
        for e in &elems {
            let res = self.resolve(&e.type_name);
            let rust_name = naming::field_ident(&e.name);
            let rename = naming::serde_rename(&e.name, &rust_name);
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
                    lenient_default(&e.name)
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
                };
            }
            out.push(OptField {
                xml: xml_field,
                decl_type,
                rename,
            });
        }
        out
    }

    /// Build the [`XmlType`] for a generated spec (for the `emit_xml` impls).
    fn xml_type(&self, spec: &str) -> Option<XmlType> {
        let ty = self.xsd.types.get(spec)?;
        let rust = naming::type_name(spec);
        if ty.is_abstract {
            let descendants = self.xsd.descendants(spec);
            if descendants.is_empty() {
                return None;
            }
            let variants = descendants
                .iter()
                .map(|d| XmlVariant {
                    ident: naming::type_name(d),
                    spec: d.clone(),
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

    /// Emit the type declarations (`opt14/types.rs`).
    #[must_use]
    pub fn emit_types(&self) -> String {
        let mut b = String::new();
        b.push_str(
            "// @generated by openehr-codegen (emit-opt) — DO NOT EDIT.\n\
             //! Typed Rust model for openEHR OPT 1.4 operational templates.\n\n\
             #![allow(dead_code, non_snake_case, non_camel_case_types, clippy::all, clippy::pedantic, clippy::nursery)]\n\n",
        );
        for spec in &self.generate {
            let Some(ty) = self.xsd.types.get(spec) else {
                continue;
            };
            let rust = naming::type_name(spec);
            let doc = format!("/// openEHR AOM/OPT `{spec}`.\n");
            if ty.is_abstract {
                let descendants = self.xsd.descendants(spec);
                if descendants.is_empty() {
                    continue;
                }
                b.push_str(&doc);
                b.push_str(
                    "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n",
                );
                let _ = writeln!(b, "pub enum {rust} {{");
                for d in &descendants {
                    let ident = naming::type_name(d);
                    let _ = writeln!(b, "    {ident}({ident}),");
                }
                b.push_str("}\n\n");
            } else {
                b.push_str(&doc);
                b.push_str(
                    "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]\n",
                );
                let _ = writeln!(b, "pub struct {rust} {{");
                for f in self.fields(spec) {
                    if let Some(rename) = &f.rename {
                        let _ = writeln!(b, "    #[serde(rename = \"{rename}\")]");
                    }
                    let _ = writeln!(b, "    pub {}: {},", f.xml.rust_name, f.decl_type);
                }
                b.push_str("}\n\n");
            }
        }
        b
    }

    /// Emit the `ToXml`/`FromXml` impls (`opt14/impls.rs`), reusing the
    /// `emit-xml` per-type emitters over the XSD-derived [`XmlType`]s.
    #[must_use]
    pub fn emit_impls(&self, unmatched: &mut Vec<(String, String)>) -> String {
        let mut b = String::new();
        b.push_str(
            "// @generated by openehr-codegen (emit-opt) — DO NOT EDIT.\n\
             //! Canonical-XML `ToXml`/`FromXml` impls for the OPT 1.4 model.\n\n\
             #![allow(non_snake_case, clippy::all, clippy::pedantic, clippy::nursery, unused_variables, unused_mut)]\n\
             #[allow(unused_imports)]\n\
             use crate::xml::runtime::{ToXml, FromXml, XmlEvent, XmlError};\n\n",
        );
        for spec in &self.generate {
            if let Some(ty) = self.xml_type(spec) {
                emit_to_xml(&mut b, &ty, PRELUDE, self.xsd, unmatched);
                emit_from_xml(&mut b, &ty, PRELUDE, self.xsd);
            }
        }
        b
    }

    /// Emit `opt14/mod.rs`: module wiring, re-export, and the `from_xml` entry.
    #[must_use]
    pub fn emit_mod() -> String {
        "// @generated by openehr-codegen (emit-opt) — DO NOT EDIT.\n\
         //! openEHR OPT 1.4 operational-template model + canonical-XML codec.\n\
         //!\n\
         //! Parse an operational template with [`from_xml`]; the root element is\n\
         //! `<template>` (`OPERATIONAL_TEMPLATE`).\n\n\
         mod impls;\n\
         mod types;\n\
         pub use types::*;\n\n\
         /// Parse an operational-template XML document into an [`OperationalTemplate`].\n\
         ///\n\
         /// # Errors\n\
         /// Propagates canonical-XML parse errors.\n\
         pub fn from_xml(xml: &str) -> Result<OperationalTemplate, crate::xml::runtime::XmlError> {\n\
         crate::xml::runtime::from_xml(xml)\n\
         }\n\n\
         /// Serialize an [`OperationalTemplate`] back to OPT 1.4 XML (root\n\
         /// `<template>`, `http://schemas.openehr.org/v1`).\n\
         ///\n\
         /// # Errors\n\
         /// Propagates canonical-XML serialization errors.\n\
         pub fn to_xml(opt: &OperationalTemplate) -> Result<String, crate::xml::runtime::XmlError> {\n\
         crate::xml::runtime::to_xml(opt, \"template\", crate::xml::runtime::Namespace::V1)\n\
         }\n"
            .to_string()
    }
}
