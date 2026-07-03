//! The Rust emitter (ADR-004): walks a merged BMM [`Model`] and produces
//! idiomatic, strongly-typed Rust for the openEHR spec crates.
//!
//! Emission rules (ADR-004):
//! - **Flattened concrete structs**: a concrete class inlines all inherited
//!   fields (ancestor-first, `// inherited: X` banners); one hop to any field.
//! - **`Option<T>`** for non-mandatory single properties; **`Vec<T>`** for
//!   containers (optional containers get `default` + `skip_serializing_if`).
//! - **Enums** (`#[serde(untagged)]`) for abstract classes used as a property
//!   type — the closed polymorphic slots (`DATA_VALUE`, `ITEM`, …).
//! - **Generics** only for classes the BMM declares generic (`Interval<T>`,
//!   `DV_INTERVAL<T>`); the actual type argument is emitted at each use site.
//! - `_type` is handled by `#[derive(OpenEhrType)]` (the `openehr-derive`
//!   proc-macro), not a per-struct field.

use crate::naming;
use openehr_lang::bmm::{BmmClass, BmmPackage, BmmPropKind, BmmSchema, BmmType};
use std::collections::{BTreeMap, BTreeSet};

/// A merged BMM model (e.g. BASE + RM) used for ancestor flattening and type
/// resolution across schema boundaries.
pub struct Model {
    classes: BTreeMap<String, BmmClass>,
}

/// A property resolved onto a concrete class, tracking which class it came from
/// (for the `// inherited: X` banner).
struct ResolvedProp<'a> {
    owner: String,
    prop: &'a openehr_lang::bmm::BmmProperty,
}

impl Model {
    /// Merge several schemas into one class map (later schemas override earlier
    /// on name collision — pass BASE before RM).
    #[must_use]
    pub fn merged(schemas: &[&BmmSchema]) -> Self {
        let mut classes = BTreeMap::new();
        for s in schemas {
            for (name, class) in &s.classes {
                classes.insert(name.clone(), class.clone());
            }
        }
        Model { classes }
    }

    fn get(&self, name: &str) -> Option<&BmmClass> {
        self.classes.get(name)
    }

    /// Does `class` inherit from `target` (transitively)?
    fn inherits(&self, class: &str, target: &str) -> bool {
        let Some(c) = self.get(class) else {
            return false;
        };
        for a in &c.ancestors {
            if a == target || self.inherits(a, target) {
                return true;
            }
        }
        false
    }

    /// All concrete classes that inherit from `name` (excluding `name` itself),
    /// in name order.
    fn concrete_descendants(&self, name: &str) -> Vec<String> {
        self.classes
            .values()
            .filter(|c| !c.is_abstract && c.name != name && self.inherits(&c.name, name))
            .map(|c| c.name.clone())
            .collect()
    }

    /// Class names used anywhere as a property type (single, container item, or
    /// generic argument) — the candidates for closed-enum slots.
    fn used_as_type(&self) -> BTreeSet<String> {
        let mut used = BTreeSet::new();
        for c in self.classes.values() {
            for p in &c.properties {
                match &p.kind {
                    BmmPropKind::Single(t) => collect_roots(t, &mut used),
                    BmmPropKind::Container { item, .. } => collect_roots(item, &mut used),
                }
            }
        }
        used
    }

    /// Flatten a class's properties, ancestor-first, with child redefinitions
    /// overriding the inherited type in place.
    fn flattened_props(&self, class: &BmmClass) -> Vec<ResolvedProp<'_>> {
        let mut order: Vec<String> = Vec::new();
        let mut map: BTreeMap<String, ResolvedProp<'_>> = BTreeMap::new();
        self.gather(&class.name, &mut order, &mut map);
        order.into_iter().filter_map(|n| map.remove(&n)).collect()
    }

    fn gather<'a>(
        &'a self,
        class_name: &str,
        order: &mut Vec<String>,
        map: &mut BTreeMap<String, ResolvedProp<'a>>,
    ) {
        let Some(class) = self.get(class_name) else {
            return;
        };
        for anc in &class.ancestors {
            self.gather(anc, order, map);
        }
        for p in &class.properties {
            if !map.contains_key(&p.name) {
                order.push(p.name.clone());
            }
            map.insert(
                p.name.clone(),
                ResolvedProp {
                    owner: class.name.clone(),
                    prop: p,
                },
            );
        }
    }

    // ── type rendering ──────────────────────────────────────────────────────

    fn render_type(&self, t: &BmmType, generics: &[String]) -> String {
        match t {
            BmmType::Simple(n) => {
                if let Some(p) = primitive(n) {
                    p.to_string()
                } else if generics.iter().any(|g| g == n) {
                    n.clone()
                } else if n == "Any" {
                    "serde_json::Value".to_string()
                } else {
                    naming::type_name(n)
                }
            }
            BmmType::Generic { root, params } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| self.render_type(p, generics))
                    .collect();
                format!("{}<{}>", naming::type_name(root), ps.join(", "))
            }
        }
    }
}

/// The set of primitive spec types → Rust types (ADR-004 type map).
fn primitive(name: &str) -> Option<&'static str> {
    Some(match name {
        "Boolean" => "bool",
        "Integer" => "i32",
        "Integer64" => "i64",
        "Real" | "Double" => "f64",
        "String" | "Uri" | "Terminology_code" => "String",
        "Octet" => "u8",
        "Character" => "char",
        _ => return None,
    })
}

fn collect_roots(t: &BmmType, out: &mut BTreeSet<String>) {
    match t {
        BmmType::Simple(n) => {
            out.insert(n.clone());
        }
        BmmType::Generic { root, params } => {
            out.insert(root.clone());
            for p in params {
                collect_roots(p, out);
            }
        }
    }
}

/// A generated Rust source file (path relative to the crate `src/`, plus body).
pub struct GenFile {
    /// Relative path under the crate `src/`, e.g. `data_types/quantity/dv_quantity.rs`.
    pub path: String,
    /// The Rust source.
    pub body: String,
}

/// Emit every concrete struct and closed-slot enum for one schema, laid out by
/// the schema's package tree. `model` supplies cross-schema ancestors/types.
#[must_use]
pub fn emit_schema(model: &Model, schema: &BmmSchema) -> Vec<GenFile> {
    let class_pkg = class_paths(schema);
    let used = model.used_as_type();
    let mut files = Vec::new();

    for (name, class) in &schema.classes {
        let pkg = class_pkg
            .get(name)
            .cloned()
            .unwrap_or_else(|| "misc".to_string());
        let module = naming::field_ident(&to_snake(name));
        let path = format!("{pkg}/{module}.rs");

        let body = if class.is_abstract {
            if used.contains(name) {
                let descendants = model
                    .concrete_descendants(name)
                    .into_iter()
                    // skip generic descendants for now (need type-arg inference)
                    .filter(|d| model.get(d).is_some_and(|c| c.generic_params.is_empty()))
                    .collect::<Vec<_>>();
                if descendants.is_empty() {
                    continue;
                }
                emit_enum(class, &descendants)
            } else {
                // abstract, not a slot type → its fields flatten into concretes;
                // nothing to emit as a standalone type.
                continue;
            }
        } else {
            emit_struct(model, class)
        };

        files.push(GenFile { path, body });
    }

    files
}

/// Build a class → nested directory path map from the package tree, e.g.
/// `DV_QUANTITY` → `data_types/quantity`. Each level's segment is the last
/// dotted component of the package name (`org.openehr.rm.data_types` →
/// `data_types`).
fn class_paths(schema: &BmmSchema) -> BTreeMap<String, String> {
    fn walk(p: &BmmPackage, prefix: &str, out: &mut BTreeMap<String, String>) {
        let seg = p.name.rsplit('.').next().unwrap_or(&p.name);
        let path = if prefix.is_empty() {
            seg.to_string()
        } else {
            format!("{prefix}/{seg}")
        };
        for c in &p.classes {
            out.insert(c.clone(), path.clone());
        }
        for sub in &p.packages {
            walk(sub, &path, out);
        }
    }
    let mut out = BTreeMap::new();
    for p in &schema.packages {
        walk(p, "", &mut out);
    }
    out
}

fn emit_struct(model: &Model, class: &BmmClass) -> String {
    let ty = naming::type_name(&class.name);
    let generics: Vec<String> = class
        .generic_params
        .iter()
        .map(|g| g.name.clone())
        .collect();
    let gen_decl = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };

    let mut b = String::new();
    file_header(&mut b, &class.name);
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, OpenEhrType)]\n");
    b.push_str(&format!("#[openehr(type_name = \"{}\")]\n", class.name));
    b.push_str(&format!("pub struct {ty}{gen_decl} {{\n"));

    let props = model.flattened_props(class);
    let mut prev_owner: Option<&str> = None;
    for rp in &props {
        let p = rp.prop;
        // `// inherited: X` banner once per run of same-owner inherited fields
        if rp.owner != class.name && prev_owner != Some(rp.owner.as_str()) {
            b.push_str(&format!("\n    // inherited: {}\n", rp.owner));
        }
        prev_owner = Some(rp.owner.as_str());
        doc_block(&mut b, p.documentation.as_deref(), "    ");

        let ident = naming::field_ident(&p.name);
        if let Some(rename) = naming::serde_rename(&p.name, &ident) {
            b.push_str(&format!("    #[serde(rename = \"{rename}\")]\n"));
        }

        let (rust_ty, extra_serde) = field_type(model, class, p, &generics);
        for attr in extra_serde {
            b.push_str(&format!("    {attr}\n"));
        }
        b.push_str(&format!("    pub {ident}: {rust_ty},\n"));
    }

    b.push_str("}\n");
    trailer(&mut b, &class.name, "struct");
    b
}

/// Compute a field's Rust type and any serde attribute lines.
fn field_type(
    model: &Model,
    class: &BmmClass,
    p: &openehr_lang::bmm::BmmProperty,
    generics: &[String],
) -> (String, Vec<String>) {
    match &p.kind {
        BmmPropKind::Single(t) => {
            let mut inner = model.render_type(t, generics);
            // box direct self-recursion (e.g. DV_MULTIMEDIA.thumbnail)
            if t.root_name() == class.name {
                inner = format!("Box<{inner}>");
            }
            if p.is_mandatory {
                (inner, vec![])
            } else {
                (
                    format!("Option<{inner}>"),
                    vec!["#[serde(skip_serializing_if = \"Option::is_none\")]".to_string()],
                )
            }
        }
        BmmPropKind::Container { item, .. } => {
            let inner = model.render_type(item, generics);
            let ty = format!("Vec<{inner}>");
            if p.is_mandatory {
                (ty, vec![])
            } else {
                (
                    ty,
                    vec!["#[serde(default, skip_serializing_if = \"Vec::is_empty\")]".to_string()],
                )
            }
        }
    }
}

fn emit_enum(class: &BmmClass, descendants: &[String]) -> String {
    let ty = naming::type_name(&class.name);
    let mut b = String::new();
    file_header(&mut b, &class.name);
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str(&format!(
        "/// Closed subtype set of `{}` (ADR-004): dispatched on each payload's `_type`.\n",
        class.name
    ));
    b.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
    b.push_str("#[serde(untagged)]\n");
    b.push_str(&format!("pub enum {ty} {{\n"));
    for d in descendants {
        let variant = naming::type_name(d);
        b.push_str(&format!("    {variant}({variant}),\n"));
    }
    b.push_str("}\n");
    trailer(&mut b, &class.name, "enum");
    b
}

// ── formatting helpers ───────────────────────────────────────────────────────

fn file_header(b: &mut String, class: &str) {
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{class}`) — DO NOT EDIT.\n\
         // Hand-written spec functions/invariants live in the sibling `*_impl.rs` (ADR-004).\n\n\
         use crate::prelude::*;\n\
         use openehr_derive::OpenEhrType;\n\
         use serde::{{Deserialize, Serialize}};\n\n"
    ));
}

fn doc_block(b: &mut String, doc: Option<&str>, indent: &str) {
    let Some(doc) = doc else { return };
    for line in doc.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            b.push_str(&format!("{indent}///\n"));
        } else {
            b.push_str(&format!("{indent}/// {line}\n"));
        }
    }
}

fn trailer(b: &mut String, class: &str, kind: &str) {
    b.push_str(&format!(
        "\n// ─────────────────────────────────────────────\n\
         // PORT STATUS\n\
         //   source: openEHR BMM meta-model — class {class} ({kind})\n\
         //   source_loc: n/a\n\
         //   confidence: high\n\
         //   todos: 0\n\
         //   note: @generated by openehr-codegen (ADR-004); regenerate, do not hand-edit.\n\
         // ─────────────────────────────────────────────\n"
    ));
}

/// `DV_QUANTITY` → `dv_quantity`, `Iso8601_date` → `iso8601_date`.
fn to_snake(spec: &str) -> String {
    spec.to_lowercase()
}
