//! XSD reader for the XML codegen (ADR-005).
//!
//! Parses the vendored openEHR ITS-XML schemas into a small structural model
//! the XML emitter needs and BMM does not encode: for each `xs:complexType`,
//! which properties are XML **attributes** vs child **elements**, the child
//! **order** (canonical XML is order-sensitive), the inheritance `base`, and the
//! `abstract` flag. From these it derives a subtype index for `xsi:type`
//! polymorphic dispatch.
//!
//! Only the RM *instance* schemas are read (`BaseTypes`, Structure, Content,
//! Composition, …) — not the OPT/AOM constraint schemas (`CompositionTemplate`,
//! Archetype, …), which redefine some type names (`ELEMENT`, `CODE_PHRASE`) for
//! the archetype world and would collide.

use std::collections::BTreeMap;
use std::path::Path;

/// A parsed XSD type model: openEHR complexType name → [`XsdType`].
pub struct XsdModel {
    /// Target XML namespace (`http://schemas.openehr.org/v1` or `…/v2`).
    pub namespace: String,
    pub types: BTreeMap<String, XsdType>,
}

/// One `xs:complexType` (its *local* attributes/elements — inheritance is via
/// [`XsdType::base`], resolved by [`XsdModel::flattened`]).
pub struct XsdType {
    pub name: String,
    pub is_abstract: bool,
    /// The `xs:extension`/`xs:restriction` base type, if any.
    pub base: Option<String>,
    /// Local attributes, declaration order.
    pub attributes: Vec<XsdAttr>,
    /// Local child elements, sequence order.
    pub elements: Vec<XsdElem>,
}

#[derive(Clone)]
#[allow(dead_code)] // type_name/required consumed by the emit-xml emitter (landing next)
pub struct XsdAttr {
    pub name: String,
    pub type_name: String,
    pub required: bool,
}

#[derive(Clone)]
#[allow(dead_code)] // type_name/optional/multiple consumed by the emit-xml emitter (landing next)
pub struct XsdElem {
    pub name: String,
    pub type_name: String,
    pub optional: bool,
    pub multiple: bool,
}

impl XsdModel {
    /// Parse a curated set of XSD files into one merged model. Later files do
    /// not override types already seen (the caller curates a conflict-free set).
    ///
    /// # Errors
    /// Returns an error if a file cannot be read or parsed as XML.
    pub fn parse_files(paths: &[std::path::PathBuf]) -> Result<Self, String> {
        let mut types: BTreeMap<String, XsdType> = BTreeMap::new();
        let mut namespace = String::new();
        for path in paths {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("read {}: {e}", path.display()))?;
            let doc = roxmltree::Document::parse(&text)
                .map_err(|e| format!("parse {}: {e}", path.display()))?;
            let root = doc.root_element();
            if namespace.is_empty()
                && let Some(ns) = root.attribute("targetNamespace")
            {
                namespace = ns.to_string();
            }
            for node in root.children().filter(|n| local(n) == "complexType") {
                if let Some(t) = parse_complex_type(node) {
                    types.entry(t.name.clone()).or_insert(t);
                }
            }
        }
        Ok(Self { namespace, types })
    }

    /// The flattened (ancestor-first) attributes + elements for a concrete type,
    /// walking the `base` chain. Attributes precede elements on the wire; within
    /// each, ancestors come first (matching the flattened generated struct
    /// field order).
    #[must_use]
    pub fn flattened(&self, name: &str) -> (Vec<XsdAttr>, Vec<XsdElem>) {
        // Collect the chain root→…→self, then concatenate in that order.
        let mut chain: Vec<&XsdType> = Vec::new();
        let mut cur = self.types.get(name);
        while let Some(t) = cur {
            chain.push(t);
            cur = t.base.as_deref().and_then(|b| self.types.get(b));
        }
        chain.reverse(); // ancestor-first
        let mut attrs = Vec::new();
        let mut elems = Vec::new();
        for t in chain {
            attrs.extend(t.attributes.iter().cloned());
            elems.extend(t.elements.iter().cloned());
        }
        (attrs, elems)
    }

    /// The concrete descendants of `name` (types whose `base` chain reaches
    /// `name`), i.e. the valid `xsi:type` values for a slot declared as `name`.
    #[must_use]
    pub fn descendants(&self, name: &str) -> Vec<String> {
        self.types
            .values()
            .filter(|t| !t.is_abstract && self.is_a(&t.name, name))
            .map(|t| t.name.clone())
            .collect()
    }

    /// Whether `sub` is `sup` or transitively extends it.
    #[must_use]
    pub fn is_a(&self, sub: &str, sup: &str) -> bool {
        let mut cur = Some(sub.to_string());
        while let Some(n) = cur {
            if n == sup {
                return true;
            }
            cur = self.types.get(&n).and_then(|t| t.base.clone());
        }
        false
    }
}

fn local<'input>(n: &roxmltree::Node<'_, 'input>) -> &'input str {
    n.tag_name().name()
}

fn parse_complex_type(node: roxmltree::Node) -> Option<XsdType> {
    let name = node.attribute("name")?.to_string();
    let is_abstract = node.attribute("abstract") == Some("true");
    let mut ty = XsdType {
        name,
        is_abstract,
        base: None,
        attributes: Vec::new(),
        elements: Vec::new(),
    };
    // Content is either wrapped in complexContent/simpleContent > extension|restriction,
    // or a direct sequence/choice/all + attributes on the complexType itself.
    for child in node.children().filter(roxmltree::Node::is_element) {
        match local(&child) {
            "complexContent" | "simpleContent" => {
                for deriv in child.children().filter(roxmltree::Node::is_element) {
                    if matches!(local(&deriv), "extension" | "restriction") {
                        ty.base = deriv.attribute("base").map(str::to_string);
                        collect_content(deriv, &mut ty);
                    }
                }
            }
            "sequence" | "choice" | "all" => collect_particle(child, &mut ty.elements),
            "attribute" => push_attr(child, &mut ty.attributes),
            _ => {}
        }
    }
    Some(ty)
}

/// Collect the element/attribute content directly under an extension/restriction
/// (or the complexType itself).
fn collect_content(container: roxmltree::Node, ty: &mut XsdType) {
    for child in container.children().filter(roxmltree::Node::is_element) {
        match local(&child) {
            "sequence" | "choice" | "all" => collect_particle(child, &mut ty.elements),
            "attribute" => push_attr(child, &mut ty.attributes),
            _ => {}
        }
    }
}

/// Recurse a particle (sequence/choice/all), collecting `xs:element`s in order.
/// Does not descend into an element's own inline `complexType`.
fn collect_particle(particle: roxmltree::Node, out: &mut Vec<XsdElem>) {
    for child in particle.children().filter(roxmltree::Node::is_element) {
        match local(&child) {
            "element" => {
                let Some(name) = child.attribute("name") else {
                    continue; // ref-based element: not used by the RM instance schemas
                };
                let type_name = child.attribute("type").unwrap_or("").to_string();
                let min = child.attribute("minOccurs").unwrap_or("1");
                let max = child.attribute("maxOccurs").unwrap_or("1");
                out.push(XsdElem {
                    name: name.to_string(),
                    type_name,
                    optional: min == "0",
                    multiple: max == "unbounded" || max.parse::<u32>().unwrap_or(1) > 1,
                });
            }
            "sequence" | "choice" | "all" => collect_particle(child, out),
            _ => {}
        }
    }
}

fn push_attr(node: roxmltree::Node, out: &mut Vec<XsdAttr>) {
    let Some(name) = node.attribute("name") else {
        return;
    };
    out.push(XsdAttr {
        name: name.to_string(),
        type_name: node.attribute("type").unwrap_or("").to_string(),
        required: node.attribute("use") == Some("required"),
    });
}

/// The RM *instance* XSD file basenames per lineage (order = merge order).
/// Excludes the OPT/AOM constraint schemas that redefine RM type names.
pub const RM_FILES_V1: &[&str] = &[
    "BaseTypes.xsd",
    "Structure.xsd",
    "Content.xsd",
    "Composition.xsd",
    "Version.xsd",
    "Extract.xsd",
    "Resource.xsd",
];

/// Resolve the v1 RM-instance file paths under the `ALL/` bundle dir.
#[must_use]
pub fn v1_files(all_dir: &Path) -> Vec<std::path::PathBuf> {
    RM_FILES_V1.iter().map(|f| all_dir.join(f)).collect()
}

/// The v1 `ALL/` files that supply the **served** RM-instance closure. Merged
/// first so shared/served types (COMPOSITION, SECTION, LOCATABLE, ENTRY, …) keep
/// their v1 shape via [`XsdModel::parse_files`]'s first-wins `.or_insert`.
///
/// `Extract.xsd` is intentionally **excluded**: the v1 `ALL/` bundle carries the
/// stale RM-1.0.2 EXTRACT model, whose `EXTRACT_ITEM` does **not** extend
/// `LOCATABLE` — contradicting the RM-1.2.0 BMM (where `EXTRACT_ITEM` is a
/// `LOCATABLE` subtype). The extract family is drawn from the v2 `EhrExtract.xsd`
/// (see [`RM_FILES_V2_SUPPLEMENT`]) so its LOCATABLE ancestry — and the
/// `archetype_node_id` **attribute** — resolves correctly.
pub const RM_FILES_V1_SERVED: &[&str] = &[
    "BaseTypes.xsd",
    "Structure.xsd",
    "Content.xsd",
    "Composition.xsd",
    "Version.xsd",
    "Resource.xsd",
];

/// The v2 (RM 1.1.0) schemas that supply the RM-instance types the v1 `ALL/`
/// bundle **lacks** (EHR + demographic) or carries **stale** (extract). Merged
/// *after* the v1 served core so already-present served types keep their v1
/// definition; only the missing types are added (via first-wins `.or_insert`).
///
/// The RM-instance wire shape is identical across the v1/v2 lineages bar the root
/// `xmlns` (ADR-005, `main.rs` rationale), so the v1 `LOCATABLE` — base of all
/// these types via the flatten walk — supplies the `archetype_node_id` attribute
/// and canonical element order for `EHR_STATUS`/`EHR_ACCESS`, the demographic
/// PARTY hierarchy, and the extract LOCATABLE subtypes. This closes F-05-01.
pub const RM_FILES_V2_SUPPLEMENT: &[&str] = &[
    "RM/Release-1.1.0/Ehr.xsd",
    "RM/Release-1.1.0/Demographic.xsd",
    "RM/Release-1.1.0/EhrExtract.xsd",
];

/// Resolve the merged emit-xml XSD input: the v1 served core under `v1_all_dir`
/// (the `its-xml-1.0.2-nsv1/ALL` bundle) followed by the v2 EHR/demographic/
/// extract supplement under `v2_root` (the `its-xml-2.0.0-nsv2` root). Order is
/// load-bearing — v1 first so served types win (`.or_insert`).
#[must_use]
pub fn xml_emit_files(v1_all_dir: &Path, v2_root: &Path) -> Vec<std::path::PathBuf> {
    RM_FILES_V1_SERVED
        .iter()
        .map(|f| v1_all_dir.join(f))
        .chain(RM_FILES_V2_SUPPLEMENT.iter().map(|f| v2_root.join(f)))
        .collect()
}

/// The AM/OPT 1.4 constraint-schema closure (`Template.xsd` and its `xs:include`
/// chain), for the `emit-opt` OPT generator (ADR-005). Order = merge order;
/// `Template.xsd` first so the OPT-specific types (`OPERATIONAL_TEMPLATE`,
/// `C_ARCHETYPE_ROOT`, …) win. `Resource.xsd` + `BaseTypes.xsd` overlap the
/// RM-instance set — those types resolve to the already-generated `openehr-rm`/
/// `openehr-base` XML impls; the AOM/OPT constraint types are generated fresh.
pub const AM_FILES_V1: &[&str] = &[
    "Template.xsd",
    "OpenehrProfile.xsd",
    "Archetype.xsd",
    "Resource.xsd",
    "BaseTypes.xsd",
];

/// Resolve the v1 AM/OPT constraint-schema file paths under the `ALL/` bundle dir.
#[must_use]
pub fn am_files_v1(all_dir: &Path) -> Vec<std::path::PathBuf> {
    AM_FILES_V1.iter().map(|f| all_dir.join(f)).collect()
}

/// The v2 RM-instance XSDs, as (component-relative) paths. v2 splits the schemas
/// per component (RM 1.1.0 + BASE 1.2.0) rather than one flat `ALL/` bundle.
/// Reserved for a future v2-specific trait (ADR-005); the v1 shape currently
/// serves both lineages (they differ only by root `xmlns`).
#[allow(dead_code)]
pub const RM_FILES_V2: &[&str] = &[
    "BASE/Release-1.2.0/BaseTypes.xsd",
    "BASE/Release-1.2.0/Resource.xsd",
    "RM/Release-1.1.0/Common.xsd",
    "RM/Release-1.1.0/DataTypes.xsd",
    "RM/Release-1.1.0/DataStructures.xsd",
    "RM/Release-1.1.0/Ehr.xsd",
    "RM/Release-1.1.0/Demographic.xsd",
    "RM/Release-1.1.0/EhrExtract.xsd",
];

/// Resolve the v2 RM-instance file paths under the `its-xml-2.0.0-nsv2/` root.
#[must_use]
#[allow(dead_code)] // reserved for a future v2-specific trait (ADR-005)
pub fn v2_files(root: &Path) -> Vec<std::path::PathBuf> {
    RM_FILES_V2.iter().map(|f| root.join(f)).collect()
}
