//! BMM object model + loader (openEHR **LANG 1.0.0**, `P_BMM` persisted form).
//!
//! Consumes the [`crate::odin`] tree of a vendored `*.bmm` schema file into a
//! typed [`BmmSchema`]: packages (the module tree), classes (with ancestors,
//! abstractness, generic parameters), and each class's ordered properties
//! (single / generic / container, with cardinality and mandatory-ness).
//!
//! This is the deterministic model that `openehr-codegen` walks to emit the
//! openEHR spec crates (ADR-004). It is also the runtime BMM model the ADL/AOM
//! phases (P9) will use, hence its home in `openehr-lang` rather than the
//! generator.
//!
//! What BMM captures (and we model): structure. What it does not (function
//! bodies, invariant logic) stays hand-written per ADR-003 — but invariant EL
//! text and function signatures are available here for scaffolding.

use crate::odin::{self, Node};
use std::collections::BTreeMap;

/// A loaded BMM schema (one vendored `*.bmm` file).
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
    /// Whether the class is abstract (`is_abstract = <True>`).
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
    /// `is_mandatory = <True>` → an obligatory attribute (else optional/`Option`).
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
        /// Raw cardinality/occurrence text (`>=0`, `0..1`, …), if present.
        cardinality: Option<String>,
    },
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
    /// Parse a BMM schema from ODIN source text.
    ///
    /// # Errors
    /// Returns [`odin::OdinError`] if the ODIN does not parse.
    pub fn parse(src: &str) -> Result<Self, odin::OdinError> {
        let doc = odin::parse(src)?;
        Ok(Self::from_odin(&doc))
    }

    /// Build a schema from an already-parsed ODIN document.
    #[must_use]
    pub fn from_odin(doc: &Node) -> Self {
        let get_str = |k: &str| {
            doc.get(k)
                .and_then(Node::as_str)
                .unwrap_or_default()
                .to_string()
        };

        let includes = doc
            .get("includes")
            .map(|inc| {
                inc.entries()
                    .iter()
                    .map(|(_, n)| n.get("id").and_then(Node::as_str).unwrap_or("").to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let packages = doc
            .get("packages")
            .map(|p| p.entries().iter().map(|(_, n)| parse_package(n)).collect())
            .unwrap_or_default();

        let mut classes = BTreeMap::new();
        if let Some(defs) = doc.get("class_definitions") {
            for (_, node) in defs.entries() {
                let class = parse_class(node);
                classes.insert(class.name.clone(), class);
            }
        }

        BmmSchema {
            schema_name: get_str("schema_name"),
            rm_release: get_str("rm_release"),
            bmm_version: get_str("bmm_version"),
            includes,
            packages,
            classes,
        }
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

fn parse_package(node: &Node) -> BmmPackage {
    let name = node
        .get("name")
        .and_then(Node::as_str)
        .unwrap_or_default()
        .to_string();
    let classes = node
        .get("classes")
        .map(|c| c.str_list().iter().map(|s| (*s).to_string()).collect())
        .unwrap_or_default();
    let packages = node
        .get("packages")
        .map(|p| p.entries().iter().map(|(_, n)| parse_package(n)).collect())
        .unwrap_or_default();
    BmmPackage {
        name,
        classes,
        packages,
    }
}

fn parse_class(node: &Node) -> BmmClass {
    let name = node
        .get("name")
        .and_then(Node::as_str)
        .unwrap_or_default()
        .to_string();
    let documentation = node
        .get("documentation")
        .and_then(Node::as_str)
        .map(str::to_string);
    let ancestors = node
        .get("ancestors")
        .map(|a| a.str_list().iter().map(|s| (*s).to_string()).collect())
        .unwrap_or_default();
    let is_abstract = node
        .get("is_abstract")
        .and_then(Node::as_bool)
        .unwrap_or(false);

    let generic_params = node
        .get("generic_parameter_defs")
        .map(|g| {
            g.entries()
                .iter()
                .map(|(_, n)| BmmGenericParam {
                    name: n
                        .get("name")
                        .and_then(Node::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    conforms_to: n
                        .get("conforms_to_type")
                        .and_then(Node::as_str)
                        .map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();

    let properties = node
        .get("properties")
        .map(|p| p.entries().iter().map(|(_, n)| parse_property(n)).collect())
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

fn parse_property(node: &Node) -> BmmProperty {
    let name = node
        .get("name")
        .and_then(Node::as_str)
        .unwrap_or_default()
        .to_string();
    let documentation = node
        .get("documentation")
        .and_then(Node::as_str)
        .map(str::to_string);
    let is_mandatory = node
        .get("is_mandatory")
        .and_then(Node::as_bool)
        .unwrap_or(false);

    let kind = if node.type_name.as_deref() == Some("P_BMM_CONTAINER_PROPERTY") {
        let td = node.get("type_def");
        let container_type = td
            .and_then(|t| t.get("container_type"))
            .and_then(Node::as_str)
            .unwrap_or("List")
            .to_string();
        let item = td.map_or(BmmType::Simple("Any".into()), parse_container_item);
        let cardinality = node
            .get("cardinality")
            .and_then(Node::as_interval)
            .map(str::to_string);
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
            .and_then(Node::as_str)
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

/// The element type inside a container's `type_def`: either a simple `type`
/// (`List<LINK>`) or a nested generic `type_def` (`List<REFERENCE_RANGE<...>>`).
fn parse_container_item(td: &Node) -> BmmType {
    if let Some(inner) = td.get("type_def") {
        parse_type_def(inner)
    } else if let Some(t) = td.get("type").and_then(Node::as_str) {
        BmmType::Simple(t.to_string())
    } else {
        BmmType::Simple("Any".to_string())
    }
}

/// A `type_def` with `root_type` + `generic_parameters` → a generic type;
/// otherwise a simple `type`.
fn parse_type_def(td: &Node) -> BmmType {
    if let Some(root) = td.get("root_type").and_then(Node::as_str) {
        let params = td
            .get("generic_parameters")
            .map(|g| {
                g.str_list()
                    .iter()
                    .map(|s| BmmType::Simple((*s).to_string()))
                    .collect()
            })
            .unwrap_or_default();
        BmmType::Generic {
            root: root.to_string(),
            params,
        }
    } else if let Some(t) = td.get("type").and_then(Node::as_str) {
        BmmType::Simple(t.to_string())
    } else {
        BmmType::Simple("Any".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNIPPET: &str = r#"
        schema_name = <"rm">
        rm_release = <"1.1.0">
        bmm_version = <"2.4">
        includes = <
            ["openehr_base_1.2.0"] = < id = <"openehr_base_1.2.0"> >
        >
        packages = <
            ["org.openehr.rm.data_types"] = <
                name = <"org.openehr.rm.data_types">
                packages = <
                    ["quantity"] = <
                        name = <"quantity">
                        classes = <"DV_QUANTITY", "DV_INTERVAL">
                    >
                >
            >
        >
        class_definitions = <
            ["DV_INTERVAL"] = <
                name = <"DV_INTERVAL">
                is_abstract = <False>
                ancestors = <"DATA_VALUE", "Interval">
                generic_parameter_defs = <
                    ["T"] = < name = <"T"> conforms_to_type = <"DV_ORDERED"> >
                >
                properties = < >
            >
            ["DV_QUANTITY"] = <
                name = <"DV_QUANTITY">
                documentation = <"A quantity.">
                ancestors = <"DV_AMOUNT">
                properties = <
                    ["magnitude"] = (P_BMM_SINGLE_PROPERTY) <
                        name = <"magnitude"> is_mandatory = <True> type = <"Real">
                    >
                    ["precision"] = (P_BMM_SINGLE_PROPERTY) <
                        name = <"precision"> type = <"Integer">
                    >
                    ["normal_range"] = (P_BMM_GENERIC_PROPERTY) <
                        name = <"normal_range">
                        type_def = < root_type = <"DV_INTERVAL"> generic_parameters = <"DV_QUANTITY", ...> >
                    >
                    ["other_reference_ranges"] = (P_BMM_CONTAINER_PROPERTY) <
                        name = <"other_reference_ranges">
                        cardinality = <|>=0|>
                        type_def = <
                            container_type = <"List">
                            type_def = (P_BMM_GENERIC_TYPE) <
                                root_type = <"REFERENCE_RANGE"> generic_parameters = <"DV_QUANTITY", ...>
                            >
                        >
                    >
                    ["links"] = (P_BMM_CONTAINER_PROPERTY) <
                        name = <"links">
                        type_def = < container_type = <"List"> type = <"LINK"> >
                    >
                >
            >
        >
    "#;

    #[test]
    fn loads_schema_header_and_includes() {
        let s = BmmSchema::parse(SNIPPET).unwrap();
        assert_eq!(s.schema_name, "rm");
        assert_eq!(s.rm_release, "1.1.0");
        assert_eq!(s.includes, vec!["openehr_base_1.2.0"]);
        assert_eq!(s.classes.len(), 2);
    }

    #[test]
    fn packages_map_classes() {
        let s = BmmSchema::parse(SNIPPET).unwrap();
        let cp = s.class_packages();
        assert_eq!(cp.get("DV_QUANTITY").map(String::as_str), Some("quantity"));
        assert_eq!(cp.get("DV_INTERVAL").map(String::as_str), Some("quantity"));
    }

    #[test]
    fn generic_class_params() {
        let s = BmmSchema::parse(SNIPPET).unwrap();
        let iv = &s.classes["DV_INTERVAL"];
        assert_eq!(iv.generic_params.len(), 1);
        assert_eq!(iv.generic_params[0].name, "T");
        assert_eq!(
            iv.generic_params[0].conforms_to.as_deref(),
            Some("DV_ORDERED")
        );
    }

    #[test]
    fn property_kinds_and_types() {
        let s = BmmSchema::parse(SNIPPET).unwrap();
        let q = &s.classes["DV_QUANTITY"];
        assert_eq!(q.ancestors, vec!["DV_AMOUNT"]);
        assert_eq!(q.documentation.as_deref(), Some("A quantity."));

        let by = |n: &str| q.properties.iter().find(|p| p.name == n).unwrap();

        let mag = by("magnitude");
        assert!(mag.is_mandatory);
        assert!(matches!(&mag.kind, BmmPropKind::Single(BmmType::Simple(t)) if t == "Real"));

        let prec = by("precision");
        assert!(!prec.is_mandatory); // optional

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
                assert_eq!(cardinality.as_deref(), Some(">=0"));
                assert!(matches!(item, BmmType::Generic { root, .. } if root == "REFERENCE_RANGE"));
            }
            other => panic!("expected container, got {other:?}"),
        }

        let links = by("links");
        match &links.kind {
            BmmPropKind::Container { item, .. } => {
                assert_eq!(item, &BmmType::Simple("LINK".into()));
            }
            other => panic!("expected container, got {other:?}"),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: openEHR LANG 1.0.0 BMM / P_BMM persisted meta-model (specifications-ITS-BMM)
//   source_loc: n/a
//   confidence: medium
//   todos: 0
//   note: structural model for codegen (ADR-004); invariants/functions captured later.
// ─────────────────────────────────────────────
