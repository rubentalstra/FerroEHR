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
//! - **Transparent newtypes** for enumeration classes that are just a
//!   primitive on the wire (`VALIDITY_KIND` → `String`).
//! - **Generics** only for classes the BMM declares generic (`Interval<T>`);
//!   the actual type argument is emitted at each use site.
//! - `_type` is handled by `#[derive(OpenEhrType)]` (`openehr-derive`), not a
//!   per-struct field.
//! - Foundation **primitives / containers / marker traits** are mapped to Rust
//!   (bool, i32, Vec, …) and never emitted (see [`SKIP`] and [`primitive`]).

use crate::naming;
use openehr_lang::bmm::{BmmClass, BmmPropKind, BmmSchema, BmmType};
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

/// Foundation classes that are **mapped to Rust and never emitted**: the
/// container types, marker/functional/service classes, and constant holders.
/// Scalar primitives are handled by [`primitive`]. Two interval classes need a
/// generic-ancestor binding the BMM does not carry (`Multiplicity_interval` =
/// `Interval<Integer>`), so they are skipped in this pass and become a
/// `codegen.toml` override later.
const SKIP: &[&str] = &[
    // containers → Vec / handled by container properties
    "Container",
    "List",
    "Set",
    "Array",
    "Hash",
    "Bag",
    // abstract marker / algebraic traits (no data)
    "Any",
    "Ordered",
    "Numeric",
    "Ordered_Numeric",
    "Comparable",
    "Temporal",
    // functional types
    "TUPLE",
    "TUPLE1",
    "TUPLE2",
    "ROUTINE",
    "FUNCTION",
    "PROCEDURE",
    // service interfaces (no data)
    "Env",
    "Locale",
    "Math",
    "Quantity_converter",
    "Statistical_evaluator",
    // constant-holder classes (no data; become assoc consts in *_impl.rs)
    "Time_Definitions",
    "BASIC_DEFINITIONS",
    "OPENEHR_DEFINITIONS",
    // interval classes needing a generic-ancestor binding not present in BMM
    "Multiplicity_interval",
    "Cardinality",
];

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

    /// Is `name` mapped to Rust rather than emitted (primitive or [`SKIP`])?
    fn is_mapped(name: &str) -> bool {
        primitive(name).is_some() || SKIP.contains(&name)
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

    /// Can `from` transitively reach `target` through **`Single`** (non-`Vec`)
    /// field types? Used to detect struct-sizing cycles that need boxing
    /// (`Vec`/`Box` already break a cycle; `Option<T>` and plain `T` do not).
    fn reaches(&self, from: &str, target: &str, seen: &mut BTreeSet<String>) -> bool {
        if from == target {
            return true;
        }
        if !seen.insert(from.to_string()) {
            return false;
        }
        let Some(class) = self.get(from) else {
            return false;
        };
        // An abstract class is emitted as an untagged enum; a cycle can run
        // through its variants (e.g. ARCHETYPE_CONSTRAINT ↔ ARCHETYPE_SLOT,
        // EXPR_ITEM ↔ EXPR_BINARY_OPERATOR). Traverse them too.
        if class.is_abstract {
            for d in self.enum_variants(from) {
                if d == target || self.reaches(&d, target, seen) {
                    return true;
                }
            }
            return false;
        }
        for rp in self.flattened_props(class) {
            if let BmmPropKind::Single(t) = &rp.prop.kind {
                let root = t.root_name();
                if Self::is_mapped(root) {
                    continue;
                }
                if root == target || self.reaches(root, target, seen) {
                    return true;
                }
            }
        }
        false
    }

    /// Concrete, emittable, non-generic descendants of `name` (for enum slots).
    fn enum_variants(&self, name: &str) -> Vec<String> {
        self.classes
            .values()
            .filter(|c| {
                !c.is_abstract
                    && c.name != name
                    && c.generic_params.is_empty()
                    && !Self::is_mapped(&c.name)
                    && self.inherits(&c.name, name)
            })
            .map(|c| c.name.clone())
            .collect()
    }

    /// Class names used anywhere as a property type — enum-slot candidates.
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

    /// Render a BMM type to Rust. `local` is the set of spec class names emitted
    /// in the current crate; a referenced class outside it (or a malformed
    /// container) degrades to `serde_json::Value` so the crate stays
    /// self-contained.
    fn render_type(&self, t: &BmmType, generics: &[String], local: &BTreeSet<String>) -> String {
        match t {
            BmmType::Simple(n) => {
                if let Some(p) = primitive(n) {
                    p.to_string()
                } else if generics.iter().any(|g| g == n) {
                    n.clone()
                } else if n == "Any" || !local.contains(n) {
                    "serde_json::Value".to_string()
                } else {
                    naming::type_name(n)
                }
            }
            BmmType::Generic { root, params } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| self.render_type(p, generics, local))
                    .collect();
                // Foundation container generics map to Rust collections; a
                // container with the wrong arity (e.g. the deeply-nested
                // free-form `Hash` in RESOURCE_ANNOTATIONS) is free-form JSON.
                match root.as_str() {
                    "Hash" if ps.len() == 2 => {
                        format!("std::collections::BTreeMap<{}>", ps.join(", "))
                    }
                    "Hash" => "serde_json::Value".to_string(),
                    "List" | "Array" if ps.len() == 1 => format!("Vec<{}>", ps[0]),
                    "Set" if ps.len() == 1 => format!("std::collections::BTreeSet<{}>", ps[0]),
                    "List" | "Array" | "Set" => "serde_json::Value".to_string(),
                    _ if !local.contains(root) => "serde_json::Value".to_string(),
                    _ => format!("{}<{}>", naming::type_name(root), ps.join(", ")),
                }
            }
        }
    }

    /// The emittable spec class names a class refers to in its (flattened)
    /// fields — for computing precise `use` imports. Excludes primitives,
    /// generic params, mapped/skip types, and `Any`.
    fn referenced_specs(&self, class: &BmmClass, generics: &[String]) -> BTreeSet<String> {
        let mut roots = BTreeSet::new();
        for rp in self.flattened_props(class) {
            match &rp.prop.kind {
                BmmPropKind::Single(t) => collect_roots(t, &mut roots),
                BmmPropKind::Container { item, .. } => collect_roots(item, &mut roots),
            }
        }
        roots
            .into_iter()
            .filter(|n| !Self::is_mapped(n) && n != "Any" && !generics.iter().any(|g| g == n))
            .collect()
    }
}

/// The set of primitive spec types → Rust types (ADR-004 type map).
fn primitive(name: &str) -> Option<&'static str> {
    Some(match name {
        "Boolean" => "bool",
        "Integer" => "i32",
        "Integer64" => "i64",
        "Real" | "Double" => "f64",
        // `Uri` is a plain string until the strong-newtype override lands.
        "String" | "Uri" => "String",
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

/// What to emit for a class.
enum Emission<'a> {
    Struct,
    Enum(Vec<String>),
    /// Transparent newtype over a Rust primitive (an enumeration-of-strings, …).
    Newtype(&'a str),
    Skip,
}

/// Decide how a class is emitted.
fn decide<'a>(model: &Model, class: &'a BmmClass, used: &BTreeSet<String>) -> Emission<'a> {
    if Model::is_mapped(&class.name) {
        return Emission::Skip;
    }
    if class.is_abstract {
        if used.contains(&class.name) {
            let variants = model.enum_variants(&class.name);
            if variants.is_empty() {
                // Abstract, referenced as a field type, but no concrete
                // descendants in this schema (e.g. `AUTHORED_RESOURCE` in BASE —
                // its concretes live in AM). Emit its own fields as a struct so
                // the reference resolves; a cross-schema pass can promote it to
                // an enum later.
                Emission::Struct
            } else {
                Emission::Enum(variants)
            }
        } else {
            Emission::Skip
        }
    } else {
        // Concrete: a 0-field class whose sole ancestor is a primitive is an
        // enumeration-style newtype (VALIDITY_KIND → String).
        let flattened = model.flattened_props(class);
        if flattened.is_empty()
            && class.ancestors.len() == 1
            && let Some(prim) = primitive(&class.ancestors[0])
        {
            return Emission::Newtype(prim);
        }
        Emission::Struct
    }
}

/// One emitted type and the module chain it lives in (for import + prelude).
struct Emitted {
    /// Module chain under the crate root, e.g. `["base_types","identification","uid"]`.
    chain: Vec<String>,
    /// Rust type identifier, e.g. `Uid`.
    ident: String,
}

/// The generated files for one schema version plus its top-level module names.
struct Version {
    files: Vec<GenFile>,
    /// Top-level module names of this version (under its prefix, if any).
    top: BTreeSet<String>,
}

/// Emit one schema version under `prefix` (empty for a single-version crate).
/// Produces the type files, the `mod.rs` tree, and a `prelude.rs`; the caller
/// assembles `lib.rs`.
fn emit_version(model: &Model, schema: &BmmSchema, prefix: &str) -> Version {
    let class_pkg = class_paths(schema);
    let used = model.used_as_type();

    struct Planned<'a> {
        class: &'a BmmClass,
        emission: Emission<'a>,
        chain: Vec<String>,
    }
    // Spec class names emitted in this version; anything referenced outside it
    // degrades to `serde_json::Value` so the crate stays self-contained.
    let mut local: BTreeSet<String> = BTreeSet::new();
    for (name, class) in &schema.classes {
        if !matches!(decide(model, class, &used), Emission::Skip) {
            local.insert(name.clone());
        }
    }

    let mut planned = Vec::new();
    let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new(); // ident → chain
    for (name, class) in &schema.classes {
        let emission = decide(model, class, &used);
        if matches!(emission, Emission::Skip) {
            continue;
        }
        let pkg = class_pkg.get(name).cloned().unwrap_or_default();
        let mut chain: Vec<String> = Vec::new();
        if !prefix.is_empty() {
            chain.push(prefix.to_string());
        }
        chain.extend(pkg.split('/').filter(|s| !s.is_empty()).map(str::to_string));
        chain.push(naming::field_ident(&to_snake(name)));
        index.insert(naming::type_name(name), chain.clone());
        planned.push(Planned {
            class,
            emission,
            chain,
        });
    }

    let mut files = Vec::new();
    for p in &planned {
        let body = match &p.emission {
            Emission::Struct => emit_struct(model, p.class, &index, &local),
            Emission::Enum(variants) => emit_enum(p.class, variants, &index),
            Emission::Newtype(prim) => emit_newtype(p.class, prim),
            Emission::Skip => unreachable!(),
        };
        files.push(GenFile {
            path: format!("{}.rs", p.chain.join("/")),
            body,
        });
    }

    let type_chains: Vec<Vec<String>> = planned.iter().map(|p| p.chain.clone()).collect();
    let emitted: Vec<Emitted> = index
        .into_iter()
        .map(|(ident, chain)| Emitted { chain, ident })
        .collect();

    // Module tree. For a prefixed version, also register `<prefix>/prelude` so
    // the prefix `mod.rs` declares it.
    let mut tree_chains = type_chains.clone();
    let prelude_path = if prefix.is_empty() {
        "prelude.rs".to_string()
    } else {
        tree_chains.push(vec![prefix.to_string(), "prelude".to_string()]);
        format!("{prefix}/prelude.rs")
    };
    files.extend(emit_module_tree(&tree_chains));
    files.push(emit_prelude(&emitted, &prelude_path));

    // Top modules: the prefix itself if prefixed, else the type roots.
    let top = if prefix.is_empty() {
        top_modules(&type_chains)
    } else {
        std::iter::once(prefix.to_string()).collect()
    };
    Version { files, top }
}

/// Emit a single-version crate (`openehr-base`): one schema, top-level modules,
/// crate `prelude`, and `lib.rs`.
#[must_use]
pub fn emit_crate(model: &Model, schema: &BmmSchema, crate_doc: &str) -> Vec<GenFile> {
    let v = emit_version(model, schema, "");
    let mut files = v.files;
    files.push(emit_lib(&v.top, true, crate_doc));
    files
}

/// Emit a multi-version crate (`openehr-am`): each `(prefix, model, schema)`
/// becomes a top-level version module (`am14`, `am24`) with its own prelude.
#[must_use]
pub fn emit_multi_crate(versions: &[(&str, &Model, &BmmSchema)], crate_doc: &str) -> Vec<GenFile> {
    let mut files = Vec::new();
    let mut top: BTreeSet<String> = BTreeSet::new();
    for (prefix, model, schema) in versions {
        let v = emit_version(model, schema, prefix);
        files.extend(v.files);
        top.extend(v.top);
    }
    files.push(emit_lib(&top, false, crate_doc));
    files
}

/// Build every `mod.rs` from the set of emitted module chains.
fn emit_module_tree(chains: &[Vec<String>]) -> Vec<GenFile> {
    // dir path ("" = root is handled by lib.rs) → set of child module idents.
    let mut dirs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for chain in chains {
        for i in 1..chain.len() {
            let dir = chain[..i].join("/");
            dirs.entry(dir).or_default().insert(chain[i].clone());
        }
    }
    dirs.into_iter()
        .map(|(dir, children)| {
            let mut b = String::from("// @generated by openehr-codegen — DO NOT EDIT.\n\n");
            for c in &children {
                b.push_str(&format!("pub mod {c};\n"));
            }
            GenFile {
                path: format!("{dir}/mod.rs"),
                body: b,
            }
        })
        .collect()
}

/// Top-level module names (first chain segment), deduped.
fn top_modules(chains: &[Vec<String>]) -> BTreeSet<String> {
    chains.iter().filter_map(|c| c.first().cloned()).collect()
}

fn emit_lib(top: &BTreeSet<String>, include_prelude: bool, crate_doc: &str) -> GenFile {
    let mut b = String::new();
    for line in crate_doc.lines() {
        b.push_str(&format!("//! {line}\n"));
    }
    b.push_str("//!\n//! @generated module tree by openehr-codegen (ADR-004). The type files\n");
    b.push_str("//! are generated; hand-written spec behaviour lives in sibling `*_impl.rs`.\n\n");
    // Lint exceptions inherent to faithful spec generation:
    //  - doc comments are verbatim openEHR spec text (bare URLs, un-backticked
    //    terms, tabs, quote-style links, loose list continuation);
    //  - some spec classes carry >3 boolean flags (e.g. `Interval` bounds);
    //  - the package tree can nest a module of the same name (module_inception);
    //  - closed-slot enums can have size-disparate variants.
    b.push_str(
        "#![allow(\n    \
         clippy::doc_markdown,\n    \
         clippy::doc_link_with_quotes,\n    \
         clippy::tabs_in_doc_comments,\n    \
         clippy::doc_lazy_continuation,\n    \
         clippy::struct_excessive_bools,\n    \
         clippy::module_inception,\n    \
         clippy::large_enum_variant\n\
         )]\n\n",
    );
    for m in top {
        b.push_str(&format!("pub mod {m};\n"));
    }
    if include_prelude {
        b.push_str("pub mod prelude;\n");
    }
    GenFile {
        path: "lib.rs".to_string(),
        body: b,
    }
}

fn emit_prelude(emitted: &[Emitted], path: &str) -> GenFile {
    let mut b = String::from(
        "//! Prelude: re-exports every generated spec type of this version.\n\
         //!\n//! @generated by openehr-codegen (ADR-004). Per-file imports are precise;\n\
         //! downstream crates and hand-written code may `use <path>::*`.\n\n",
    );
    let mut lines: Vec<String> = emitted
        .iter()
        .map(|e| format!("pub use crate::{}::{};", e.chain.join("::"), e.ident))
        .collect();
    lines.sort();
    for l in lines {
        b.push_str(&l);
        b.push('\n');
    }
    GenFile {
        path: path.to_string(),
        body: b,
    }
}

/// Build a class → nested directory path map from the package tree, e.g.
/// `DV_QUANTITY` → `data_types/quantity`.
fn class_paths(schema: &BmmSchema) -> BTreeMap<String, String> {
    fn walk(p: &openehr_lang::bmm::BmmPackage, prefix: &str, out: &mut BTreeMap<String, String>) {
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

fn emit_struct(
    model: &Model,
    class: &BmmClass,
    index: &BTreeMap<String, Vec<String>>,
    local: &BTreeSet<String>,
) -> String {
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
    let imports = import_lines(model, class, &generics, &ty, index);
    struct_header(&mut b, &class.name, &imports);
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str("#[derive(Debug, Clone, PartialEq, OpenEhrType)]\n");
    b.push_str(&format!("#[openehr(type_name = \"{}\")]\n", class.name));
    b.push_str(&format!("pub struct {ty}{gen_decl} {{\n"));

    let props = model.flattened_props(class);
    let mut prev_owner: Option<&str> = None;
    for rp in &props {
        let p = rp.prop;
        if rp.owner != class.name && prev_owner != Some(rp.owner.as_str()) {
            b.push_str(&format!("\n    // inherited: {}\n", rp.owner));
        }
        prev_owner = Some(rp.owner.as_str());
        doc_block(&mut b, p.documentation.as_deref(), "    ");

        let ident = naming::field_ident(&p.name);
        if let Some(rename) = naming::serde_rename(&p.name, &ident) {
            b.push_str(&format!("    #[openehr(rename = \"{rename}\")]\n"));
        }
        let rust_ty = field_type(model, class, p, &generics, local);
        b.push_str(&format!("    pub {ident}: {rust_ty},\n"));
    }

    b.push_str("}\n");
    b
}

/// Field-level type overrides mapping a `(class, field)` to a proven Rust crate
/// type instead of the BMM primitive (ADR-004 override layer). Seeded here;
/// slated to move to `codegen.toml`. Only unambiguous mappings belong here —
/// where openEHR's semantics are broader than a crate (partial-precision ISO
/// 8601, plain-text URIs) the field stays `String` and the crate is used in the
/// hand-written `*_impl.rs` behavior instead.
fn type_override(class: &str, field: &str) -> Option<&'static str> {
    match (class, field) {
        // A UUID is an RFC-4122 canonical UUID — use the `uuid` crate directly.
        // (ISO_OID / INTERNET_ID / OBJECT_VERSION_ID are *not* plain UUIDs.)
        ("UUID", "value") => Some("uuid::Uuid"),
        _ => None,
    }
}

/// Compute a field's Rust type (`OpenEhrType` handles skip-if-none/empty, so no
/// serde attributes are needed on the field).
fn field_type(
    model: &Model,
    class: &BmmClass,
    p: &openehr_lang::bmm::BmmProperty,
    generics: &[String],
    local: &BTreeSet<String>,
) -> String {
    match &p.kind {
        BmmPropKind::Single(t) => {
            let overridden = type_override(&class.name, &p.name);
            let mut inner = match overridden {
                Some(rust) => rust.to_string(),
                None => model.render_type(t, generics, local),
            };
            // Box direct self-recursion, and mutual recursion that would make
            // the struct infinitely sized (e.g. RESOURCE_DESCRIPTION ↔
            // AUTHORED_RESOURCE). Skips overridden/mapped types.
            let root = t.root_name();
            let cyclic = overridden.is_none()
                && !Model::is_mapped(root)
                && local.contains(root)
                && (root == class.name || model.reaches(root, &class.name, &mut BTreeSet::new()));
            if cyclic {
                inner = format!("Box<{inner}>");
            }
            if p.is_mandatory {
                inner
            } else {
                format!("Option<{inner}>")
            }
        }
        BmmPropKind::Container { item, .. } => {
            format!("Vec<{}>", model.render_type(item, generics, local))
        }
    }
}

fn emit_enum(
    class: &BmmClass,
    variants: &[String],
    index: &BTreeMap<String, Vec<String>>,
) -> String {
    let ty = naming::type_name(&class.name);
    let mut b = String::new();

    let mut imports: BTreeSet<String> = BTreeSet::new();
    for v in variants {
        add_import(&mut imports, &naming::type_name(v), &ty, index);
    }
    enum_header(&mut b, &class.name, &imports);
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str(&format!(
        "/// Closed subtype set of `{}` (ADR-004): dispatched on each payload's `_type`.\n",
        class.name
    ));
    b.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
    b.push_str("#[serde(untagged)]\n");
    b.push_str(&format!("pub enum {ty} {{\n"));
    for d in variants {
        let variant = naming::type_name(d);
        b.push_str(&format!("    {variant}({variant}),\n"));
    }
    b.push_str("}\n");
    b
}

fn emit_newtype(class: &BmmClass, prim: &str) -> String {
    let ty = naming::type_name(&class.name);
    let mut b = String::new();
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{}`) — DO NOT EDIT.\n\n\
         use serde::{{Deserialize, Serialize}};\n\n",
        class.name
    ));
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str("#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n");
    b.push_str("#[serde(transparent)]\n");
    b.push_str(&format!("pub struct {ty}(pub {prim});\n"));
    b
}

// ── import + header helpers ──────────────────────────────────────────────────

/// Precise `use crate::...;` lines for a struct's referenced spec types.
fn import_lines(
    model: &Model,
    class: &BmmClass,
    generics: &[String],
    self_ident: &str,
    index: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for spec in model.referenced_specs(class, generics) {
        add_import(&mut imports, &naming::type_name(&spec), self_ident, index);
    }
    imports
}

fn add_import(
    imports: &mut BTreeSet<String>,
    ident: &str,
    self_ident: &str,
    index: &BTreeMap<String, Vec<String>>,
) {
    if ident == self_ident {
        return;
    }
    if let Some(chain) = index.get(ident) {
        imports.insert(format!("use crate::{}::{};", chain.join("::"), ident));
    }
}

fn struct_header(b: &mut String, class: &str, imports: &BTreeSet<String>) {
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{class}`) — DO NOT EDIT.\n\
         // Hand-written spec functions/invariants live in the sibling `*_impl.rs` (ADR-004).\n\n\
         use openehr_derive::OpenEhrType;\n"
    ));
    for imp in imports {
        b.push_str(imp);
        b.push('\n');
    }
    b.push('\n');
}

fn enum_header(b: &mut String, class: &str, imports: &BTreeSet<String>) {
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{class}`) — DO NOT EDIT.\n\n\
         use serde::{{Deserialize, Serialize}};\n"
    ));
    for imp in imports {
        b.push_str(imp);
        b.push('\n');
    }
    b.push('\n');
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

/// `DV_QUANTITY` → `dv_quantity`, `Iso8601_date` → `iso8601_date`.
fn to_snake(spec: &str) -> String {
    spec.to_lowercase()
}
