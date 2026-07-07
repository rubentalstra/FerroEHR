//! `emit-rm-model` (ADR-008 §3, P16): emit a **static RM attribute/type model**
//! into `openehr-rm` (`src/model/`), generated from the same BASE + RM BMM the
//! `emit` target consumes. This is the AQL planner's oracle — attribute typing,
//! multiplicity, abstract→concrete descendant sets, and structure classification
//! — compiled once at build time, with no reflection and no hand-maintained
//! tables (ADR-008).
//!
//! Two generated files under `openehr-rm/src/model/`:
//! - `mod.rs` — the public API (`RmClass`, `RmAttribute`, `Container`, and the
//!   `class`/`attribute`/`attributes`/`descendants`/`ancestors`/`is_a`/
//!   `is_structure_root` functions + a `LazyLock` name index).
//! - `data.rs` — the generated `static CLASSES: &[RmClass]` table.

use crate::bmm::{BmmClass, BmmPropKind, BmmType};
use crate::emit::{GenFile, Model};
use std::collections::BTreeSet;

/// Classes the node codec splits into their own `node` row — mirrored **verbatim**
/// from `ehrbase::storage::codec::STRUCTURE_TYPES` (ADR-008 decompose rule).
///
/// This is a hand-picked set, **not** a pure BMM-derivable predicate. It is the
/// set of LOCATABLE *content* subtypes that occur inside COMPOSITION / `EHR_STATUS`
/// / FOLDER trees, **plus** `EVENT_CONTEXT` and `FEEDER_AUDIT` — which AQL
/// addresses although the RM does not make them LOCATABLE. Concretely it differs
/// from "concrete LOCATABLE descendants" two ways:
/// - **added** (not LOCATABLE): `EVENT_CONTEXT` (a PATHABLE) and `FEEDER_AUDIT`.
/// - **excluded** LOCATABLE descendants: the demographic `PARTY` hierarchy
///   (`PARTY`, `ROLE`, `ACTOR`, `PERSON`, …) and `EHR`/`FOLDER`-external types,
///   which are never composition content.
///
/// Kept in lockstep with `codec.rs`; if either changes, both must change together
/// (the storage spike owns the canonical list).
const STRUCTURE_ROOTS: &[&str] = &[
    "COMPOSITION",
    "EHR_STATUS",
    "EHR_ACCESS",
    "FOLDER",
    "EVENT_CONTEXT",
    "SECTION",
    "GENERIC_ENTRY",
    "ADMIN_ENTRY",
    "OBSERVATION",
    "EVALUATION",
    "INSTRUCTION",
    "ACTION",
    "ACTIVITY",
    "HISTORY",
    "POINT_EVENT",
    "INTERVAL_EVENT",
    "ITEM_TREE",
    "ITEM_LIST",
    "ITEM_SINGLE",
    "ITEM_TABLE",
    "CLUSTER",
    "ELEMENT",
    "FEEDER_AUDIT",
];

/// One RM class row for the generated table.
struct ClassModel {
    name: String,
    is_abstract: bool,
    /// Transitive ancestors (spec names, mapped foundation types excluded), sorted.
    ancestors: Vec<String>,
    /// Transitive **concrete** descendants (incl. self if concrete), sorted.
    descendants: Vec<String>,
    attributes: Vec<AttrModel>,
    is_structure_root: bool,
}

/// One attribute row (own or inherited, flattened ancestor-first).
struct AttrModel {
    name: String,
    declared_type: String,
    container: &'static str,
    is_mandatory: bool,
}

/// Emit the `model/` files for `openehr-rm` from the merged BASE + RM model.
#[must_use]
pub fn emit_files(model: &Model) -> Vec<GenFile> {
    let classes = build(model);
    vec![
        GenFile {
            path: "model/mod.rs".to_string(),
            body: emit_mod(),
        },
        GenFile {
            path: "model/data.rs".to_string(),
            body: emit_data(&classes),
        },
    ]
}

/// Build the class table: every real spec class in the merged model (primitives
/// and foundation marker/container types excluded), in name order.
fn build(model: &Model) -> Vec<ClassModel> {
    // Concrete, real spec classes — the descendant-set universe.
    let concrete: Vec<&str> = model
        .class_iter()
        .filter(|(n, c)| !Model::is_mapped(n) && !c.is_abstract)
        .map(|(n, _)| n.as_str())
        .collect();

    let mut out = Vec::new();
    for (name, class) in model.class_iter() {
        if Model::is_mapped(name) {
            continue;
        }
        out.push(ClassModel {
            name: name.clone(),
            is_abstract: class.is_abstract,
            ancestors: ancestors_of(model, name),
            descendants: descendants_of(model, name, &concrete),
            attributes: attributes_of(model, class),
            is_structure_root: STRUCTURE_ROOTS.contains(&name.as_str()),
        });
    }
    out
}

/// Transitive ancestors of `name` (mapped foundation types excluded), sorted.
fn ancestors_of(model: &Model, name: &str) -> Vec<String> {
    let mut set = BTreeSet::new();
    collect_ancestors(model, name, &mut set);
    set.into_iter().collect()
}

fn collect_ancestors(model: &Model, name: &str, out: &mut BTreeSet<String>) {
    if let Some(c) = model.get(name) {
        for a in &c.ancestors {
            if !Model::is_mapped(a) {
                out.insert(a.clone());
            }
            // Recurse even through a mapped ancestor: a real class can sit above
            // one (rare, but keeps the closure honest).
            collect_ancestors(model, a, out);
        }
    }
}

/// Transitive concrete descendants of `name` (incl. `name` itself when concrete),
/// sorted. `concrete` is the precomputed concrete-class universe.
fn descendants_of(model: &Model, name: &str, concrete: &[&str]) -> Vec<String> {
    concrete
        .iter()
        .filter(|d| **d == name || model.inherits(d, name))
        .map(|d| (*d).to_string())
        .collect()
}

/// The flattened (own + inherited) attributes of `class`, ancestor-first.
fn attributes_of(model: &Model, class: &BmmClass) -> Vec<AttrModel> {
    model
        .flattened_props(class)
        .iter()
        .map(|rp| {
            let (declared_type, container) = match &rp.prop.kind {
                BmmPropKind::Single(t) => (declared_type(model, &class.name, t), "None"),
                BmmPropKind::Container {
                    container_type,
                    item,
                    ..
                } => (
                    declared_type(model, &class.name, item),
                    container_kind(container_type),
                ),
            };
            AttrModel {
                name: rp.prop.name.clone(),
                declared_type,
                container,
                is_mandatory: rp.prop.is_mandatory,
            }
        })
        .collect()
}

/// The declared spec-type name of an attribute value, resolving a bare generic
/// parameter (`T`) to its bound in the concrete class's scope (the same
/// bound-fill `emit` applies), and reducing generic instantiations to their root
/// (`DV_INTERVAL<T>` → `DV_INTERVAL`; the container column carries multiplicity).
fn declared_type(model: &Model, concrete: &str, t: &BmmType) -> String {
    let root = t.root_name();
    if model.is_generic_param(concrete, root) {
        model
            .resolved_param_bound(concrete, root)
            .unwrap_or_else(|| "Any".to_string())
    } else {
        root.to_string()
    }
}

/// Map a BMM container kind to a `Container` variant name.
fn container_kind(container_type: &str) -> &'static str {
    match container_type {
        "Set" => "Set",
        "Hash" => "Hash",
        // `List`, `Array`, and any unknown container are ordered lists.
        _ => "List",
    }
}

/// Render a `&["A", "B"]`-style array literal body (`["A", "B"]`, or `[]`).
fn str_slice(items: &[String]) -> String {
    let mut b = String::from("[");
    for (i, s) in items.iter().enumerate() {
        if i > 0 {
            b.push_str(", ");
        }
        b.push_str(&format!("{s:?}"));
    }
    b.push(']');
    b
}

/// The hand-authored-shaped, generated `model/mod.rs`: the public API over the
/// generated `data::CLASSES` table.
fn emit_mod() -> String {
    // Note: the body is a literal — no per-class data — so it is a constant
    // string. rustfmt runs over the written file afterwards.
    r#"// @generated by openehr-codegen (emit-rm-model, ADR-008) — DO NOT EDIT.
//! Static RM attribute/type model — the AQL planner's spec-pinned oracle
//! (ADR-008 §3), generated from the BASE + RM BMM meta-model (the same input as
//! the `emit` target). No reflection, no hand-maintained tables.
//!
//! Covers every real spec class of `openehr-base` + `openehr-rm` (foundation
//! primitives, containers, and marker types excluded). For each class it records
//! the flattened (own + inherited) attributes with their declared spec type,
//! container kind, and mandatory flag; the abstract flag; the transitive ancestor
//! set; the transitive **concrete** descendant set; and a structure-node flag.
//!
//! # `is_structure_root`
//!
//! `is_structure_root(class)` is true for exactly the RM types the node codec
//! splits into their own `node` row (`ehrbase::storage::codec`). That set is
//! **not** derivable from the BMM alone — it is a hand-picked list mirrored from
//! `codec::STRUCTURE_TYPES`: the LOCATABLE *content* subtypes used inside
//! COMPOSITION / EHR_STATUS / FOLDER trees, **plus** `EVENT_CONTEXT` and
//! `FEEDER_AUDIT` (which AQL addresses although the RM does not make them
//! LOCATABLE), and **without** the demographic LOCATABLE hierarchy (`PARTY`,
//! `ROLE`, `ACTOR`, …), which is never composition content. The emitter keeps
//! this in lockstep with `codec.rs`.

use std::collections::HashMap;
use std::sync::LazyLock;

mod data;

/// An RM class in the static model.
#[derive(Debug)]
pub struct RmClass {
    /// The spec class name, verbatim (e.g. `"OBSERVATION"`).
    pub name: &'static str,
    /// Whether the class is abstract (never instantiated directly).
    pub is_abstract: bool,
    /// Transitive ancestor spec names (foundation primitives/markers excluded).
    pub ancestors: &'static [&'static str],
    /// Transitive **concrete** descendant spec names, including this class when
    /// it is itself concrete.
    pub descendants: &'static [&'static str],
    /// The flattened (own + inherited) attributes, ancestor-first.
    pub attributes: &'static [RmAttribute],
    /// Whether the node codec splits this type into its own `node` row (see the
    /// module docs).
    pub is_structure_root: bool,
}

/// One attribute of an [`RmClass`].
#[derive(Debug)]
pub struct RmAttribute {
    /// The attribute name, verbatim (e.g. `"content"`).
    pub name: &'static str,
    /// The declared value spec-type name (e.g. `"HISTORY"`, `"DV_TEXT"`,
    /// `"CONTENT_ITEM"`; a generic parameter is resolved to its bound).
    pub declared_type: &'static str,
    /// The attribute's container shape.
    pub container: Container,
    /// Whether the attribute is mandatory (existence ≥ 1).
    pub is_mandatory: bool,
}

/// The container shape of an [`RmAttribute`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// Single-valued (`T` / `Option<T>`).
    None,
    /// Ordered list (`List`/`Array`).
    List,
    /// Unordered set (`Set`).
    Set,
    /// Keyed map (`Hash`).
    Hash,
}

/// Name → class index, built once from the generated table.
static INDEX: LazyLock<HashMap<&'static str, &'static RmClass>> =
    LazyLock::new(|| data::CLASSES.iter().map(|c| (c.name, c)).collect());

fn find(name: &str) -> Option<&'static RmClass> {
    INDEX.get(name).copied()
}

/// The class named `name`, if present in the model.
#[must_use]
pub fn class(name: &str) -> Option<&'static RmClass> {
    find(name)
}

/// Resolve attribute `attr` on `class`, through inheritance (the stored attribute
/// list is already flattened, so an inherited attribute like `LOCATABLE.name`
/// resolves for `OBSERVATION`).
#[must_use]
pub fn attribute(class: &str, attr: &str) -> Option<&'static RmAttribute> {
    find(class)?.attributes.iter().find(|a| a.name == attr)
}

/// Every attribute of `class`, own and inherited (empty for an unknown class).
pub fn attributes(class: &str) -> impl Iterator<Item = &'static RmAttribute> {
    const EMPTY: &[RmAttribute] = &[];
    find(class).map_or(EMPTY, |c| c.attributes).iter()
}

/// The transitive concrete descendants of `class` (empty for an unknown class).
#[must_use]
pub fn descendants(class: &str) -> &'static [&'static str] {
    const EMPTY: &[&str] = &[];
    find(class).map_or(EMPTY, |c| c.descendants)
}

/// The transitive ancestors of `class` (empty for an unknown class).
#[must_use]
pub fn ancestors(class: &str) -> &'static [&'static str] {
    const EMPTY: &[&str] = &[];
    find(class).map_or(EMPTY, |c| c.ancestors)
}

/// Whether `sub` is `sup` or a (transitive) subtype of it.
#[must_use]
pub fn is_a(sub: &str, sup: &str) -> bool {
    sub == sup || find(sub).is_some_and(|c| c.ancestors.contains(&sup))
}

/// Whether the node codec splits `class` into its own `node` row (see module docs).
#[must_use]
pub fn is_structure_root(class: &str) -> bool {
    find(class).is_some_and(|c| c.is_structure_root)
}
"#
    .to_string()
}

/// The generated `model/data.rs`: the `static CLASSES` table.
fn emit_data(classes: &[ClassModel]) -> String {
    let mut b = String::from(
        "// @generated by openehr-codegen (emit-rm-model, ADR-008) — DO NOT EDIT.\n\
         //! Static RM class/attribute table backing the `model` API.\n\n\
         use super::{Container, RmAttribute, RmClass};\n\n\
         pub(super) static CLASSES: &[RmClass] = &[\n",
    );
    for c in classes {
        b.push_str(&format!(
            "    RmClass {{\n        name: {:?},\n        is_abstract: {},\n",
            c.name, c.is_abstract
        ));
        b.push_str(&format!(
            "        ancestors: &{},\n",
            str_slice(&c.ancestors)
        ));
        b.push_str(&format!(
            "        descendants: &{},\n",
            str_slice(&c.descendants)
        ));
        b.push_str("        attributes: &[\n");
        for a in &c.attributes {
            b.push_str(&format!(
                "            RmAttribute {{ name: {:?}, declared_type: {:?}, container: Container::{}, is_mandatory: {} }},\n",
                a.name, a.declared_type, a.container, a.is_mandatory
            ));
        }
        b.push_str("        ],\n");
        b.push_str(&format!(
            "        is_structure_root: {},\n    }},\n",
            c.is_structure_root
        ));
    }
    b.push_str("];\n");
    b
}
