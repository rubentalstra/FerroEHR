//! BMM object model + loader (openEHR **LANG 1.0.0**, `P_BMM` persisted form).
//!
//! This is `openehr-codegen`'s **internal** BMM reader — the generator's own
//! tooling, not an openEHR spec artifact (so it lives here, not in the generated
//! `openehr-lang` crate). It models more of the meta-model than the emitter
//! currently consumes, hence the module-wide `dead_code` allowance.
#![allow(dead_code)]
//!
//! Loads a vendored `*.bmm.json` schema (the canonical JSON serialization of
//! the openEHR BMM meta-model) into a typed [`BmmSchema`]: packages (the module
//! tree), classes (with ancestors, abstractness, generic parameters), and each
//! class's ordered properties (single / generic / container, with cardinality
//! and mandatory-ness).
//!
//! JSON is used rather than the ODIN form because it is a cleaner, structured
//! serialization of the identical meta-model (real arrays, structured
//! `cardinality`, explicit `_type` tags) and `serde_json` parses it robustly.
//! (An ODIN text reader for ADL/ODIN *instance* parsing is future runtime work
//! (P8/P9), not for BMM ingestion.
//!
//! This is the deterministic model that `openehr-codegen` walks to emit the
//! openEHR spec crates (ADR-004). What BMM captures (and we model): structure.
//! What it does not (function bodies, invariant logic) stays hand-written per
//! ADR-003 — but invariant EL text and function signatures remain available in
//! the JSON for scaffolding.

use serde_json::Value;
use std::collections::BTreeMap;

/// A loaded BMM schema (one vendored `*.bmm.json` file).
#[derive(Debug, Clone)]
pub struct BmmSchema {
    /// `schema_name`, e.g. `"rm"`.
    pub schema_name: String,
    /// `rm_release`, e.g. `"1.1.0"`.
    pub rm_release: String,
    /// `bmm_version`, e.g. `"2.4"`.
    pub bmm_version: String,
    /// Ids of included schemas (e.g. `"openehr_base_1.2.0"`).
    pub includes: Vec<String>,
    /// Top-level packages (a tree; see [`BmmPackage`]).
    pub packages: Vec<BmmPackage>,
    /// All class definitions, keyed by class name.
    pub classes: BTreeMap<String, BmmClass>,
}

/// A BMM package node in the package tree.
#[derive(Debug, Clone)]
pub struct BmmPackage {
    /// Dotted package name, e.g. `"org.openehr.rm.common"` or a leaf `"archetyped"`.
    pub name: String,
    /// Class names declared directly in this package.
    pub classes: Vec<String>,
    /// Sub-packages.
    pub packages: Vec<BmmPackage>,
}

/// A BMM class definition.
#[derive(Debug, Clone)]
pub struct BmmClass {
    /// Class name, e.g. `"DV_QUANTITY"`.
    pub name: String,
    /// Class documentation (verbatim), if any.
    pub documentation: Option<String>,
    /// Immediate ancestors (may include foundation classes like `"Interval"`).
    pub ancestors: Vec<String>,
    /// Whether the class is abstract (`is_abstract = true`).
    pub is_abstract: bool,
    /// Generic parameter definitions (`generic_parameter_defs`), if generic.
    pub generic_params: Vec<BmmGenericParam>,
    /// Properties in declaration order.
    pub properties: Vec<BmmProperty>,
}

/// A generic parameter of a class, e.g. `T conforms_to DV_ORDERED`.
#[derive(Debug, Clone)]
pub struct BmmGenericParam {
    /// Parameter name, e.g. `"T"`.
    pub name: String,
    /// The `conforms_to_type` bound, if any.
    pub conforms_to: Option<String>,
}

/// A BMM property (attribute) of a class.
#[derive(Debug, Clone)]
pub struct BmmProperty {
    /// Property name, e.g. `"magnitude"`.
    pub name: String,
    /// Property documentation (verbatim), if any.
    pub documentation: Option<String>,
    /// `is_mandatory = true` → an obligatory attribute (else optional/`Option`).
    pub is_mandatory: bool,
    /// The property's shape and type.
    pub kind: BmmPropKind,
}

/// The shape of a [`BmmProperty`].
#[derive(Debug, Clone)]
pub enum BmmPropKind {
    /// A single-valued property (`P_BMM_SINGLE_PROPERTY`,
    /// `P_BMM_SINGLE_PROPERTY_OPEN`, or `P_BMM_GENERIC_PROPERTY`).
    Single(BmmType),
    /// A container property (`P_BMM_CONTAINER_PROPERTY`), e.g. `List<LINK>`.
    Container {
        /// `List`, `Set`, `Array`, `Hash`, …
        container_type: String,
        /// The element type.
        item: BmmType,
        /// Structured cardinality, if present.
        cardinality: Option<BmmCardinality>,
    },
}

/// A structured container cardinality (`{lower, upper_unbounded, upper}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BmmCardinality {
    /// Lower bound (default 0).
    pub lower: u32,
    /// Upper bound, or `None` if unbounded.
    pub upper: Option<u32>,
}

/// A BMM type reference: a named type, or a generic instantiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BmmType {
    /// A simple named type (`Real`, `DV_TEXT`, or a generic parameter like `T`).
    Simple(String),
    /// A generic instantiation, e.g. `DV_INTERVAL<DV_QUANTITY>`.
    Generic {
        /// The root (open) type, e.g. `DV_INTERVAL`.
        root: String,
        /// The actual type arguments.
        params: Vec<BmmType>,
    },
}

impl BmmType {
    /// The underlying root/simple type name (ignoring generic args).
    #[must_use]
    pub fn root_name(&self) -> &str {
        match self {
            BmmType::Simple(s) => s,
            BmmType::Generic { root, .. } => root,
        }
    }
}

impl BmmSchema {
    /// Parse a BMM schema from its JSON serialization.
    ///
    /// # Errors
    /// Returns [`serde_json::Error`] if the JSON does not parse.
    pub fn parse_json(src: &str) -> Result<Self, serde_json::Error> {
        let doc: Value = serde_json::from_str(src)?;
        Ok(Self::from_value(&doc))
    }

    /// Build a schema from an already-parsed JSON value.
    #[must_use]
    pub fn from_value(doc: &Value) -> Self {
        let s = |k: &str| {
            doc.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };

        let includes = doc
            .get("includes")
            .and_then(Value::as_object)
            .map(|m| {
                m.values()
                    .filter_map(|v| v.get("id").and_then(Value::as_str).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let packages = doc
            .get("packages")
            .and_then(Value::as_object)
            .map(|m| m.values().map(parse_package).collect())
            .unwrap_or_default();

        // Classes come from two sibling sections: `class_definitions` (the
        // model's own classes) and `primitive_types` (foundation built-ins —
        // primitives, containers, `Interval`, ISO 8601, terminology). Both are
        // needed so ancestor flattening and type resolution work; the emitter
        // decides which to actually emit.
        let mut classes = BTreeMap::new();
        for section in ["primitive_types", "class_definitions"] {
            if let Some(defs) = doc.get(section).and_then(Value::as_object) {
                for node in defs.values() {
                    let class = parse_class(node);
                    classes.insert(class.name.clone(), class);
                }
            }
        }

        BmmSchema {
            schema_name: s("schema_name"),
            rm_release: s("rm_release"),
            bmm_version: s("bmm_version"),
            includes,
            packages,
            classes,
        }
    }

    /// Combine another schema's classes and packages into this one (set union;
    /// `self` wins on class-name collisions). openEHR ships some components
    /// across several vendored BMM files — LANG's model spans a persisted-BMM
    /// / `EXPR_*` file and a BMM-object-model / `EL_*` file — and a single crate
    /// must emit the union of both.
    #[must_use]
    pub fn combined(mut self, other: &BmmSchema) -> Self {
        for (name, class) in &other.classes {
            self.classes
                .entry(name.clone())
                .or_insert_with(|| class.clone());
        }
        self.packages.extend(other.packages.iter().cloned());
        self
    }

    /// A map from class name to its leaf package name (the innermost package
    /// that directly lists the class), for deriving the module layout.
    #[must_use]
    pub fn class_packages(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for p in &self.packages {
            collect_class_packages(p, &mut out);
        }
        out
    }
}

fn collect_class_packages(p: &BmmPackage, out: &mut BTreeMap<String, String>) {
    for c in &p.classes {
        out.insert(c.clone(), p.name.clone());
    }
    for sub in &p.packages {
        collect_class_packages(sub, out);
    }
}

fn str_vec(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_package(node: &Value) -> BmmPackage {
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let classes = str_vec(node.get("classes"));
    let packages = node
        .get("packages")
        .and_then(Value::as_object)
        .map(|m| m.values().map(parse_package).collect())
        .unwrap_or_default();
    BmmPackage {
        name,
        classes,
        packages,
    }
}

fn parse_class(node: &Value) -> BmmClass {
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let documentation = node
        .get("documentation")
        .and_then(Value::as_str)
        .map(str::to_string);
    let ancestors = str_vec(node.get("ancestors"));
    let is_abstract = node
        .get("is_abstract")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let generic_params = node
        .get("generic_parameter_defs")
        .and_then(Value::as_object)
        .map(|m| {
            m.values()
                .map(|n| BmmGenericParam {
                    name: n
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    conforms_to: n
                        .get("conforms_to_type")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();

    // Properties: preserve declaration order. serde_json with `preserve_order`
    // keeps object key order, so the map iterates in file order.
    let properties = node
        .get("properties")
        .and_then(Value::as_object)
        .map(|m| m.values().map(parse_property).collect())
        .unwrap_or_default();

    BmmClass {
        name,
        documentation,
        ancestors,
        is_abstract,
        generic_params,
        properties,
    }
}

fn parse_property(node: &Value) -> BmmProperty {
    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let documentation = node
        .get("documentation")
        .and_then(Value::as_str)
        .map(str::to_string);
    let is_mandatory = node
        .get("is_mandatory")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ptype = node.get("_type").and_then(Value::as_str).unwrap_or("");

    let kind = if ptype == "P_BMM_CONTAINER_PROPERTY" {
        let td = node.get("type_def");
        let container_type = td
            .and_then(|t| t.get("container_type"))
            .and_then(Value::as_str)
            .unwrap_or("List")
            .to_string();
        let item = td.map_or(BmmType::Simple("Any".into()), parse_container_item);
        let cardinality = node.get("cardinality").map(parse_cardinality);
        BmmPropKind::Container {
            container_type,
            item,
            cardinality,
        }
    } else if let Some(td) = node.get("type_def") {
        // P_BMM_GENERIC_PROPERTY
        BmmPropKind::Single(parse_type_def(td))
    } else {
        // P_BMM_SINGLE_PROPERTY / P_BMM_SINGLE_PROPERTY_OPEN
        let t = node
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("Any")
            .to_string();
        BmmPropKind::Single(BmmType::Simple(t))
    };

    BmmProperty {
        name,
        documentation,
        is_mandatory,
        kind,
    }
}

fn parse_cardinality(v: &Value) -> BmmCardinality {
    let lower = v
        .get("lower")
        .and_then(Value::as_u64)
        .and_then(|u| u32::try_from(u).ok())
        .unwrap_or(0);
    let upper = if v
        .get("upper_unbounded")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        None
    } else {
        v.get("upper")
            .and_then(Value::as_u64)
            .and_then(|u| u32::try_from(u).ok())
    };
    BmmCardinality { lower, upper }
}

/// The element type inside a container's `type_def`: either a simple `type`
/// (`List<LINK>`) or a nested generic `type_def` (`List<REFERENCE_RANGE<...>>`).
fn parse_container_item(td: &Value) -> BmmType {
    if let Some(inner) = td.get("type_def") {
        parse_type_def(inner)
    } else if let Some(t) = td.get("type").and_then(Value::as_str) {
        BmmType::Simple(t.to_string())
    } else {
        BmmType::Simple("Any".to_string())
    }
}

/// A `type_def` with `root_type` → a generic type; otherwise a simple `type`.
/// The generic arguments come either as `generic_parameters` (a list, whose
/// entries may be bare type names *or* nested `type_def`s) or as
/// `generic_parameter_defs` (an ordered map of named params, used for
/// `Hash<K,V>`; `serde_json`'s `preserve_order` keeps K before V).
fn parse_type_def(td: &Value) -> BmmType {
    if let Some(root) = td.get("root_type").and_then(Value::as_str) {
        let params: Vec<BmmType> =
            if let Some(arr) = td.get("generic_parameters").and_then(Value::as_array) {
                arr.iter().map(parse_generic_param).collect()
            } else if let Some(obj) = td.get("generic_parameter_defs").and_then(Value::as_object) {
                obj.values().map(parse_type_def).collect()
            } else {
                Vec::new()
            };
        BmmType::Generic {
            root: root.to_string(),
            params,
        }
    } else if let Some(t) = td.get("type").and_then(Value::as_str) {
        BmmType::Simple(t.to_string())
    } else {
        BmmType::Simple("Any".to_string())
    }
}

/// A single `generic_parameters` entry: a bare type name (`"String"`) or a
/// nested `type_def` (`{ "root_type": "Interval", "generic_parameters": [...] }`).
fn parse_generic_param(x: &Value) -> BmmType {
    match x.as_str() {
        Some(s) => BmmType::Simple(s.to_string()),
        None => parse_type_def(x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNIPPET: &str = r#"{
        "schema_name": "rm",
        "rm_release": "1.1.0",
        "bmm_version": "2.4",
        "includes": { "openehr_base_1.2.0": { "id": "openehr_base_1.2.0" } },
        "packages": {
            "org.openehr.rm.data_types": {
                "name": "org.openehr.rm.data_types",
                "packages": {
                    "quantity": { "name": "quantity", "classes": ["DV_QUANTITY", "DV_INTERVAL"] }
                }
            }
        },
        "class_definitions": {
            "DV_INTERVAL": {
                "name": "DV_INTERVAL",
                "ancestors": ["DATA_VALUE", "Interval"],
                "generic_parameter_defs": { "T": { "name": "T", "conforms_to_type": "DV_ORDERED" } }
            },
            "DATA_VALUE": { "name": "DATA_VALUE", "is_abstract": true },
            "DV_QUANTITY": {
                "name": "DV_QUANTITY",
                "documentation": "A quantity.",
                "ancestors": ["DV_AMOUNT"],
                "properties": {
                    "magnitude": { "_type": "P_BMM_SINGLE_PROPERTY", "name": "magnitude", "is_mandatory": true, "type": "Real" },
                    "precision": { "_type": "P_BMM_SINGLE_PROPERTY", "name": "precision", "type": "Integer" },
                    "normal_range": { "_type": "P_BMM_GENERIC_PROPERTY", "name": "normal_range", "type_def": { "root_type": "DV_INTERVAL", "generic_parameters": ["DV_QUANTITY"] } },
                    "other_reference_ranges": { "_type": "P_BMM_CONTAINER_PROPERTY", "name": "other_reference_ranges", "cardinality": { "lower": 0, "upper_unbounded": true }, "type_def": { "container_type": "List", "type_def": { "_type": "P_BMM_GENERIC_TYPE", "root_type": "REFERENCE_RANGE", "generic_parameters": ["DV_QUANTITY"] } } },
                    "links": { "_type": "P_BMM_CONTAINER_PROPERTY", "name": "links", "type_def": { "container_type": "List", "type": "LINK" } }
                }
            }
        }
    }"#;

    #[test]
    fn loads_schema_header_and_includes() {
        let s = BmmSchema::parse_json(SNIPPET).unwrap();
        assert_eq!(s.schema_name, "rm");
        assert_eq!(s.rm_release, "1.1.0");
        assert_eq!(s.includes, vec!["openehr_base_1.2.0"]);
        assert_eq!(s.classes.len(), 3);
    }

    #[test]
    fn packages_map_classes() {
        let s = BmmSchema::parse_json(SNIPPET).unwrap();
        let cp = s.class_packages();
        assert_eq!(cp.get("DV_QUANTITY").map(String::as_str), Some("quantity"));
        assert_eq!(cp.get("DV_INTERVAL").map(String::as_str), Some("quantity"));
    }

    #[test]
    fn generic_class_params_and_abstract() {
        let s = BmmSchema::parse_json(SNIPPET).unwrap();
        let iv = &s.classes["DV_INTERVAL"];
        assert_eq!(iv.generic_params.len(), 1);
        assert_eq!(iv.generic_params[0].name, "T");
        assert_eq!(
            iv.generic_params[0].conforms_to.as_deref(),
            Some("DV_ORDERED")
        );
        assert!(s.classes["DATA_VALUE"].is_abstract);
        assert!(!iv.is_abstract);
    }

    #[test]
    fn property_kinds_and_types() {
        let s = BmmSchema::parse_json(SNIPPET).unwrap();
        let q = &s.classes["DV_QUANTITY"];
        assert_eq!(q.ancestors, vec!["DV_AMOUNT"]);
        assert_eq!(q.documentation.as_deref(), Some("A quantity."));

        let by = |n: &str| q.properties.iter().find(|p| p.name == n).unwrap();

        let mag = by("magnitude");
        assert!(mag.is_mandatory);
        assert!(matches!(&mag.kind, BmmPropKind::Single(BmmType::Simple(t)) if t == "Real"));

        assert!(!by("precision").is_mandatory);

        let nr = by("normal_range");
        match &nr.kind {
            BmmPropKind::Single(BmmType::Generic { root, params }) => {
                assert_eq!(root, "DV_INTERVAL");
                assert_eq!(params, &[BmmType::Simple("DV_QUANTITY".into())]);
            }
            other => panic!("expected generic single, got {other:?}"),
        }

        let orr = by("other_reference_ranges");
        match &orr.kind {
            BmmPropKind::Container {
                container_type,
                item,
                cardinality,
            } => {
                assert_eq!(container_type, "List");
                assert_eq!(
                    cardinality,
                    &Some(BmmCardinality {
                        lower: 0,
                        upper: None
                    })
                );
                assert!(matches!(item, BmmType::Generic { root, .. } if root == "REFERENCE_RANGE"));
            }
            other @ BmmPropKind::Single(_) => panic!("expected container, got {other:?}"),
        }

        match &by("links").kind {
            BmmPropKind::Container { item, .. } => {
                assert_eq!(item, &BmmType::Simple("LINK".into()));
            }
            other @ BmmPropKind::Single(_) => panic!("expected container, got {other:?}"),
        }
    }

    #[test]
    fn declaration_order_preserved() {
        let s = BmmSchema::parse_json(SNIPPET).unwrap();
        let names: Vec<&str> = s.classes["DV_QUANTITY"]
            .properties
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "magnitude",
                "precision",
                "normal_range",
                "other_reference_ranges",
                "links"
            ]
        );
    }
}
