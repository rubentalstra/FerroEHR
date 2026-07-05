//! OPT 1.4 emitter (`emit-opt`, ADR-005): generates a typed Rust model
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
/// a `Hash<String, String>` (`BTreeMap`) map field, the same shape `emit-xml`
/// uses for RM `Hash<String, String>` attributes.
const STRING_DICT_ITEM: &str = "StringDictionaryItem";

/// OPT-envelope sections captured as opaque `serde_json::Value` (parsed by
/// skipping their subtree). Both hold *differential*/*presentation* data — not
/// the operational definition — and contain partial `C_OBJECT`s that omit
/// otherwise-mandatory fields (`rm_type_name`, `occurrences`) plus anonymous
/// inline complexTypes the XSD reader cannot flatten (`T_VIEW.constraints`).
/// They are not needed for template ingestion, so they are skipped losslessly-
/// enough (structure preserved down to these two roots).
const OPAQUE_TYPES: &[&str] = &["T_CONSTRAINT", "T_VIEW"];

/// How a referenced XSD type name resolves for a generated field.
enum Resolved {
    /// A Rust primitive (`String`/`bool`/`i32`/`i64`/`f64`).
    Primitive(&'static str),
    /// `xs:anyType`/`xs:anySimpleType`/anonymous-inline → `serde_json::Value`
    /// (parsed by skipping the subtree — lossy but never errors).
    Value,
    /// A repeated `StringDictionaryItem` element group → `BTreeMap<String,String>`.
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
                "std::collections::BTreeMap<String, String>".to_string(),
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
                    target: "Hash".to_string(),
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
                // boundedness flags default false) → fall back to `false`.
                let default = (is_bool && !e.optional && !e.multiple).then(|| "false".to_string());
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
            "// @generated by openehr-codegen (emit-opt, ADR-005) — DO NOT EDIT.\n\
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
            "// @generated by openehr-codegen (emit-opt, ADR-005) — DO NOT EDIT.\n\
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
        "// @generated by openehr-codegen (emit-opt, ADR-005) — DO NOT EDIT.\n\
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
         }\n"
            .to_string()
    }
}
