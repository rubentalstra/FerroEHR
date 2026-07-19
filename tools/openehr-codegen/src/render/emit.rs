//! The Rust emitter: walks a merged BMM [`Model`] and produces
//! idiomatic, strongly-typed Rust for the openEHR spec crates.
//!
//! Emission rules:
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
//!
//! Stage 4 — RENDER. The only stage that produces text: the per-shape emit
//! functions turn a planned class into deterministic, byte-stable Rust source.

use crate::analyze::{External, Model, class_paths};
use crate::load::bmm::{BmmClass, BmmEnumValue, BmmEnumeration, BmmPropKind, BmmSchema, BmmType};
use crate::plan::{Emission, back_reference, class_binding, decide, field_default, type_override};
use crate::render::naming;
use std::collections::{BTreeMap, BTreeSet};

/// A generated Rust source file (path relative to the crate `src/`, plus body).
pub(crate) struct GenFile {
    /// Relative path under the crate `src/`, e.g. `data_types/quantity/dv_quantity.rs`.
    pub path: String,
    /// The Rust source.
    pub body: String,
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
fn emit_version(model: &Model, schema: &BmmSchema, prefix: &str, external: &External) -> Version {
    struct Planned<'a> {
        class: &'a BmmClass,
        emission: Emission<'a>,
        chain: Vec<String>,
    }

    // Safeguard (owner ruling 2026-07-19): never emit a non-constructible type.
    // A mandatory single-valued construction cycle must be broken at a designated
    // owner/parent back-reference edge (`back_reference`); this fails loudly if a
    // cycle is left unbroken (e.g. a future BMM addition), pointing at the fix.
    model.assert_constructible(schema);

    let class_pkg = class_paths(schema);
    let used = model.used_as_type();

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
        // A polymorphic-concrete class emits a sibling `{Name}Data` struct in the
        // same file (the enum owns `{Name}`); export it from the prelude too so
        // downstream code (e.g. the generated XML impls) can name it.
        if matches!(emission, Emission::PolyEnum(_)) {
            index.insert(format!("{}Data", naming::type_name(name)), chain.clone());
        }
        planned.push(Planned {
            class,
            emission,
            chain,
        });
    }

    let mut files = Vec::new();
    for p in &planned {
        let body = match &p.emission {
            Emission::Struct => emit_struct(model, p.class, &index, &local, external),
            Emission::Enum(variants) => {
                emit_enum(model, p.class, variants, false, &index, &local, external)
            }
            Emission::PolyEnum(variants) => {
                emit_enum(model, p.class, variants, true, &index, &local, external)
            }
            Emission::EnumLiterals(enumeration) => emit_enum_literals(p.class, enumeration),
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
/// crate `prelude`, and `lib.rs`. `external` resolves dependency-crate types.
#[must_use]
pub(crate) fn emit_crate(
    model: &Model,
    schema: &BmmSchema,
    external: &External,
    crate_doc: &str,
) -> Vec<GenFile> {
    let v = emit_version(model, schema, "", external);
    let mut files = v.files;
    files.push(emit_lib(&v.top, true, crate_doc));
    files
}

/// Emit a multi-version crate (`openehr-am`): each `(prefix, model, schema)`
/// becomes a top-level version module (`am14`, `am24`) with its own prelude.
#[must_use]
pub(crate) fn emit_multi_crate(
    versions: &[(&str, &Model, &BmmSchema)],
    external: &External,
    crate_doc: &str,
) -> Vec<GenFile> {
    let mut files = Vec::new();
    let mut top: BTreeSet<String> = BTreeSet::new();
    for (prefix, model, schema) in versions {
        let v = emit_version(model, schema, prefix, external);
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
    b.push_str("//!\n//! @generated module tree by openehr-codegen. The type files\n");
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
         //!\n//! @generated by openehr-codegen. Per-file imports are precise;\n\
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

fn emit_struct(
    model: &Model,
    class: &BmmClass,
    index: &BTreeMap<String, Vec<String>>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    let ty = naming::type_name(&class.name);
    let generics = struct_generics(model, class);
    let subst = class_binding(&class.name);

    let mut b = String::new();
    let imports = import_lines(model, class, &generics, &subst, &ty, index, external);
    struct_header(&mut b, &class.name, &imports);
    b.push_str(&render_struct_def(
        model, class, &ty, &generics, &subst, local, external,
    ));
    b
}

/// The params a struct is generic over (see `used_generic_params`).
fn struct_generics(model: &Model, class: &BmmClass) -> Vec<String> {
    model
        .used_generic_params(&class.name)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// The struct definition (doc, derive, fields) under the name `struct_ty`,
/// without the file header. `struct_ty` is normally `type_name(class)`, but a
/// polymorphic-concrete class emits its own instances as `{Name}Data` (the
/// enum owns `{Name}`). The canonical `_type` stays the class name either way.
fn render_struct_def(
    model: &Model,
    class: &BmmClass,
    struct_ty: &str,
    generics: &[String],
    subst: &BTreeMap<String, String>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    let gen_decl = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };
    let mut b = String::new();
    doc_block(&mut b, class.documentation.as_deref(), "");
    b.push_str("#[derive(Debug, Clone, PartialEq, OpenEhrType)]\n");
    b.push_str(&format!("#[openehr(type_name = \"{}\")]\n", class.name));
    b.push_str(&format!("pub struct {struct_ty}{gen_decl} {{\n"));

    let props = model.flattened_props(class);
    let mut prev_owner: Option<&str> = None;
    let mut first = true;
    for rp in &props {
        let p = rp.prop;
        // A designated owner/parent back-reference is a non-data navigational
        // association, never forward-owned data and never on the canonical wire;
        // emitting it as an owning field makes the type a non-constructible
        // infinite value (see `back_reference`). Omit it from the struct + serde;
        // behavioural access, if ever needed, belongs in a hand-written
        // `*_impl.rs`. This is the only sanctioned way to break a mandatory
        // construction cycle — a forward composition is never relaxed.
        if let Some(citation) = back_reference(&rp.owner, &p.name) {
            b.push_str(&format!(
                "    // NOTE: `{}` (BMM-mandatory back-reference) omitted — {}. \
                 A back-reference is not forward-owned data and never appears on \
                 the canonical wire; emitting it as an owning field would make \
                 this type non-constructible.\n",
                p.name, citation
            ));
            continue;
        }
        if rp.owner != class.name && prev_owner != Some(rp.owner.as_str()) {
            // Blank line before a new `// inherited:` group, but not as the very
            // first line inside the braces (rustfmt strips a leading blank line).
            let sep = if first { "" } else { "\n" };
            b.push_str(&format!("{sep}    // inherited: {}\n", rp.owner));
        }
        prev_owner = Some(rp.owner.as_str());
        first = false;
        doc_block(&mut b, p.documentation.as_deref(), "    ");

        let ident = naming::field_ident(&p.name);
        if let Some(rename) = naming::serde_rename(&p.name, &ident) {
            b.push_str(&format!("    #[openehr(rename = \"{rename}\")]\n"));
        }
        if let Some(default) = field_default(&rp.owner, &p.name) {
            b.push_str(&format!("    #[openehr(default = \"{default}\")]\n"));
        }
        let rust_ty = field_type(model, class, p, generics, subst, local, external);
        b.push_str(&format!("    pub {ident}: {rust_ty},\n"));
    }

    b.push_str("}\n");
    b
}

/// Compute a field's Rust type (`OpenEhrType` handles skip-if-none/empty, so no
/// serde attributes are needed on the field).
fn field_type(
    model: &Model,
    class: &BmmClass,
    p: &crate::load::bmm::BmmProperty,
    generics: &[String],
    subst: &BTreeMap<String, String>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    match &p.kind {
        BmmPropKind::Single(t) => {
            let overridden = type_override(&class.name, &p.name);
            let mut inner = match overridden {
                Some(rust) => rust.to_string(),
                None => model.render_type(t, generics, subst, local, external),
            };
            // Box a field that would make the struct infinitely sized: direct
            // self-recursion, mutual recursion (RESOURCE_DESCRIPTION ↔
            // AUTHORED_RESOURCE), and F-bounded recursion through an auto-filled
            // generic arg (DV_QUANTITY → normal_range: DvInterval<DvOrdered>,
            // and DvOrdered's variants include DV_QUANTITY). We check every spec
            // name the rendered type embeds by value, not just its head.
            // A type already behind an indirection (`Vec`, `BTreeMap`,
            // `BTreeSet`) breaks the cycle on its own — boxing it is redundant.
            let already_indirect =
                inner.starts_with("Vec<") || inner.starts_with("std::collections::");
            let cyclic = overridden.is_none() && !already_indirect && {
                let mut roots = BTreeSet::new();
                model.effective_roots(t, &mut roots);
                roots.iter().any(|r| {
                    !Model::is_mapped(r)
                        && (r == &class.name || model.reaches(r, &class.name, &mut BTreeSet::new()))
                })
            };
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
            // A byte buffer (`Array<Octet>` / `List<Octet>`, e.g.
            // `DV_MULTIMEDIA.data`) is inline base64 *text* on the canonical
            // wire, not a JSON array — carry the base64 verbatim as a `String`
            // (decoding is a behaviour-layer concern), like other broader-than-a-
            // crate openEHR types. Optionality follows the property.
            if item.root_name() == "Octet" {
                return if p.is_mandatory {
                    "String".to_string()
                } else {
                    "Option<String>".to_string()
                };
            }
            format!(
                "Vec<{}>",
                model.render_type(item, generics, subst, local, external)
            )
        }
    }
}

fn emit_enum(
    model: &Model,
    class: &BmmClass,
    variants: &[String],
    self_data: bool,
    index: &BTreeMap<String, Vec<String>>,
    local: &BTreeSet<String>,
    external: &External,
) -> String {
    let ty = naming::type_name(&class.name);
    // The enum is generic over the abstract class's declared params that any
    // concrete variant uses (`VERSION<T>` exposes `T` only through
    // `ORIGINAL_VERSION.data: T`); `used_generic_params` resolves this uniformly
    // so a bare reference elsewhere renders the same arity.
    let enum_generics: Vec<String> = model
        .used_generic_params(&class.name)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    let gen_decl = if enum_generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", enum_generics.join(", "))
    };
    let mut b = String::new();
    let no_subst = BTreeMap::new();

    // Compute payloads first (so imports can be derived from what they touch).
    let payloads: Vec<(String, String)> = variants
        .iter()
        .map(|d| {
            let variant = naming::type_name(d);
            let d_generic = !model.used_generic_params(d).is_empty();
            let payload = if d_generic && !enum_generics.is_empty() {
                // Same subtype family: thread the enum's own params (`Event<T>`
                // → `PointEvent(PointEvent<T>)`).
                format!("{variant}<{}>", enum_generics.join(", "))
            } else {
                // Non-generic enum (e.g. `DataValue`) with a generic variant:
                // bound-fill the variant (`DvInterval(DvInterval<DvOrdered>)`).
                model.render_type(
                    &BmmType::Simple(d.clone()),
                    &enum_generics,
                    &no_subst,
                    local,
                    external,
                )
            };
            // Box a variant that would make the enum infinitely sized: either
            // the payload embeds the enum type by value via a bound-filled arg
            // (`EL_TERMINAL` ⊇ `EL_CASE_TABLE<EL_TERMINAL>`), or the variant's
            // own fields reach back to the enum (`BMM_TYPE` ⊇ `BMM_CONTAINER_TYPE`
            // whose `base_type` is a `BMM_TYPE`). A `Vec`/map payload already
            // breaks the cycle.
            let already_indirect =
                payload.starts_with("Vec<") || payload.starts_with("std::collections::");
            let cyclic = !already_indirect && {
                let mut roots = BTreeSet::new();
                model.effective_roots(&BmmType::Simple(d.clone()), &mut roots);
                roots.contains(&class.name) || model.reaches(d, &class.name, &mut BTreeSet::new())
            };
            let payload = if cyclic {
                format!("Box<{payload}>")
            } else {
                payload
            };
            (variant, payload)
        })
        .collect();

    // A polymorphic *concrete* class also carries its own instances: append a
    // `{Name}({Name}Data)` variant last (least-rich, so richer subtypes match
    // first on the untagged wire), and emit the `{Name}Data` struct in-file.
    let data_ty = format!("{ty}Data");
    let data_generics = struct_generics(model, class);
    let data_subst = class_binding(&class.name);
    let mut payloads = payloads;
    if self_data {
        let data_payload = if data_generics.is_empty() {
            data_ty.clone()
        } else {
            format!("{data_ty}<{}>", data_generics.join(", "))
        };
        payloads.push((ty.clone(), data_payload));
    }

    // Imports: every emittable spec type each payload embeds. For a variant
    // threaded over the enum's own params (`IntervalEvent<T>`) that is just the
    // variant type; for a bound-filled variant (`DvInterval<DvOrdered>`) it also
    // includes the auto-filled bound args. Mirror the payload decision so we do
    // not import a bound type the payload never names.
    let mut imports: BTreeSet<String> = BTreeSet::new();
    for d in variants {
        let mut roots = BTreeSet::new();
        let d_generic = !model.used_generic_params(d).is_empty();
        if d_generic && !enum_generics.is_empty() {
            roots.insert(d.clone());
        } else {
            model.effective_roots(&BmmType::Simple(d.clone()), &mut roots);
        }
        for r in roots {
            add_import(&mut imports, &r, &ty, index, external);
        }
    }
    // The in-file `{Name}Data` struct pulls in imports for the class's own fields.
    if self_data {
        imports.extend(
            model
                .referenced_specs(class, &data_generics, &data_subst)
                .iter()
                .filter_map(|spec| {
                    let ident = naming::type_name(spec);
                    if ident == ty {
                        return None;
                    }
                    if let Some(chain) = index.get(&ident) {
                        Some(format!("use crate::{}::{};", chain.join("::"), ident))
                    } else {
                        external
                            .prelude_of(spec)
                            .map(|path| format!("use {path}::{ident};"))
                    }
                }),
        );
    }

    // ── `_type` dispatch (mirrors the XML xsi:type runtime) ─────────────────
    // The ITS-JSON schema requires `_type` on an *abstract* polymorphic slot
    // (`DATA_VALUE`, `UID`, `VERSION`, …) and rejects a `_type`-less value, while
    // a *concrete* polymorphic slot (`DV_TEXT`, holding a plain DV_TEXT or a
    // DV_CODED_TEXT) makes `_type` optional and defaults a `_type`-less value to
    // the base concrete type. We emit a hand-rolled `Deserialize` that dispatches
    // on `_type` (deep descendants routed to their direct variant, which
    // recurses) instead of `#[serde(untagged)]`, whose structural guessing
    // silently mis-types a `_type`-less value. Serialize keeps
    // `#[serde(untagged)]` — its output is byte-identical (variant payload only).
    let dispatch = model.xsi_dispatch(&class.name, variants);
    // `_type` dispatch is valid only when every concrete target actually carries
    // a `_type` on the wire (a Struct or PolyEnum, not a transparent enumeration
    // Newtype); otherwise keep the structural `#[serde(untagged)]` reader.
    let type_dispatch = !dispatch.is_empty()
        && dispatch
            .iter()
            .all(|(spec, _)| model.concrete_carries_type(spec));
    // The variant a `_type`-less value defaults to: `Some` for a concrete
    // polymorphic slot (its own `{Name}Data`), `None` for an abstract slot (a
    // `_type`-less value is rejected, per the schema).
    let self_ident = dispatch
        .iter()
        .find(|(spec, _)| *spec == class.name)
        .map(|(_, id)| id.clone());

    // Header: an untagged enum uses serde derives; a polymorphic-concrete file
    // also emits an `OpenEhrType` struct, so it needs that import too.
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{}`) — DO NOT EDIT.\n",
        class.name
    ));
    if self_data {
        b.push_str("// Hand-written spec functions/invariants live in the sibling `*_impl.rs`.\n");
    }
    b.push('\n');
    // When we hand-roll `Deserialize`, only `Serialize` is derived; `Deserialize`
    // is referenced by full path in the emitted impl, so drop its import.
    let fixed: &[&str] = match (self_data, type_dispatch) {
        (true, true) => &["use serde::Serialize;", "use openehr_derive::OpenEhrType;"],
        (true, false) => &[
            "use serde::{Deserialize, Serialize};",
            "use openehr_derive::OpenEhrType;",
        ],
        (false, true) => &["use serde::Serialize;"],
        (false, false) => &["use serde::{Deserialize, Serialize};"],
    };
    write_uses(&mut b, fixed, &imports);

    // The `{Name}Data` struct (the class's own instances) precedes the enum.
    if self_data {
        b.push_str(&render_struct_def(
            model,
            class,
            &data_ty,
            &data_generics,
            &data_subst,
            local,
            external,
        ));
        b.push('\n');
    }

    doc_block(&mut b, class.documentation.as_deref(), "");
    let slot = if self_data {
        "Polymorphic slot"
    } else {
        "Closed subtype set"
    };
    b.push_str(&format!(
        "/// {slot} of `{}`: a closed subtype set dispatched on each payload's `_type`.\n",
        class.name
    ));
    if type_dispatch {
        b.push_str("#[derive(Debug, Clone, PartialEq, Serialize)]\n");
    } else {
        b.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
    }
    b.push_str("#[serde(untagged)]\n");
    b.push_str(&format!("pub enum {ty}{gen_decl} {{\n"));
    for (variant, payload) in &payloads {
        b.push_str(&format!("    {variant}({payload}),\n"));
    }
    b.push_str("}\n");

    if type_dispatch {
        b.push('\n');
        b.push_str(&emit_type_dispatch_deser(
            &ty,
            &enum_generics,
            &class.name,
            &dispatch,
            self_ident.as_deref(),
        ));
    }
    b
}

/// Emit a hand-rolled `Deserialize` for an abstract/polymorphic enum that
/// dispatches on the canonical-JSON `_type` discriminator instead of
/// `#[serde(untagged)]`'s structural fallback.
///
/// The value is buffered into a `serde_json::Value` (these types are
/// canonical-JSON-only for serde; XML has its own `FromXml` path), its `_type`
/// read, and the whole value re-deserialized into the one matching variant via
/// `serde_json::from_value` — which preserves that variant's precise inner error
/// and re-checks `_type` + unknown keys in the inner `OpenEhrType`
/// reader. A deep descendant (`DV_CODED_TEXT` in a `DATA_VALUE` slot) routes to
/// its direct variant (`DvText`), whose own dispatcher recurses.
///
/// `self_ident` is `Some(variant)` for a concrete polymorphic slot — a
/// `_type`-less value defaults to the base concrete type, matching the schema's
/// `if not required _type then <base>` construction — and `None` for an abstract
/// slot, where a `_type`-less value is rejected (schema `required: [_type]`).
fn emit_type_dispatch_deser(
    ty: &str,
    generics: &[String],
    spec_name: &str,
    dispatch: &[(String, String)],
    self_ident: Option<&str>,
) -> String {
    let expected = dispatch
        .iter()
        .map(|(s, _)| s.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let (impl_hdr, ty_ref, where_cl) = if generics.is_empty() {
        ("impl<'de>".to_string(), ty.to_string(), String::new())
    } else {
        let ps = generics.join(", ");
        // `from_value` deserializes an owned `Value`, so each parameter must be
        // `DeserializeOwned` (satisfied at every call site — the RM types are all
        // owned, and canonical JSON is parsed from owned input).
        let wc = generics
            .iter()
            .map(|p| format!("{p}: ::serde::de::DeserializeOwned"))
            .collect::<Vec<_>>()
            .join(", ");
        (
            format!("impl<'de, {ps}>"),
            format!("{ty}<{ps}>"),
            format!("\nwhere\n{wc},"),
        )
    };

    let mut b = String::new();
    b.push_str(&format!(
        "{impl_hdr} ::serde::Deserialize<'de> for {ty_ref}{where_cl} {{\n"
    ));
    // `too_many_lines`: enums with many concrete descendants generate a long
    // match. `match_same_arms`: several `_type`s can route to one direct variant
    // (deep descendants collapse), yielding intentionally-identical arms.
    b.push_str("    #[allow(clippy::too_many_lines, clippy::match_same_arms)]\n");
    b.push_str(
        "    fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>\n",
    );
    b.push_str("    where\n        D: ::serde::Deserializer<'de>,\n    {\n");
    b.push_str(
        "        let __value = <::serde_json::Value as ::serde::Deserialize>::deserialize(deserializer)?;\n",
    );
    b.push_str("        match __value.get(\"_type\").and_then(::serde_json::Value::as_str) {\n");
    for (spec, ident) in dispatch {
        b.push_str(&format!(
            "            ::core::option::Option::Some({spec:?}) => ::core::result::Result::Ok(\n                Self::{ident}(::serde_json::from_value(__value).map_err(::serde::de::Error::custom)?),\n            ),\n"
        ));
    }
    if let Some(ident) = self_ident {
        b.push_str(&format!(
            "            ::core::option::Option::None => ::core::result::Result::Ok(\n                Self::{ident}(::serde_json::from_value(__value).map_err(::serde::de::Error::custom)?),\n            ),\n"
        ));
    } else {
        let msg = format!(
            "{spec_name}: missing required `_type` on polymorphic slot (expected one of: {expected})"
        );
        b.push_str(&format!(
            "            ::core::option::Option::None => ::core::result::Result::Err(::serde::de::Error::custom(\n                {msg:?},\n            )),\n"
        ));
    }
    // Inline the binding (`{__other:?}`) so the generated `format!` is
    // clippy-clean (`uninlined_format_args`).
    let fmt =
        format!("{spec_name}: unexpected `_type` {{__other:?}} (expected one of: {expected})");
    b.push_str(&format!(
        "            ::core::option::Option::Some(__other) => ::core::result::Result::Err(::serde::de::Error::custom(\n                ::std::format!({fmt:?}),\n            )),\n"
    ));
    b.push_str("        }\n    }\n}\n");
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

/// One enumeration constant resolved for emission: the BMM constant name, its
/// Rust variant identifier, and its wire value (the string token or the integer).
struct EnumLit {
    /// The verbatim BMM constant name (`release_candidate`) — for doc lines.
    name: String,
    /// The Rust variant identifier (`ReleaseCandidate`).
    ident: String,
    wire: EnumLitWire,
}

enum EnumLitWire {
    Str(String),
    Int(i32),
}

/// Resolve an enumeration's constants to `(ident, wire)` pairs, applying the same
/// value rule as the RM-model emitter (`BMM_ENUMERATION`: explicit `item_values`
/// win; an INTEGER with none assumes 0,1,2,…; a STRING with none takes each
/// constant name as its own value).
fn enum_literals(enumeration: &BmmEnumeration) -> Vec<EnumLit> {
    let is_int = enumeration.underlying_type == "INTEGER";
    enumeration
        .item_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let ident = naming::type_name(name);
            let wire = match enumeration.item_values.as_ref().and_then(|v| v.get(i)) {
                Some(BmmEnumValue::Int(v)) => {
                    EnumLitWire::Int(i32::try_from(*v).unwrap_or_default())
                }
                Some(BmmEnumValue::Str(s)) => EnumLitWire::Str(s.clone()),
                None if is_int => EnumLitWire::Int(i32::try_from(i).unwrap_or_default()),
                None => EnumLitWire::Str(name.clone()),
            };
            EnumLit {
                name: name.clone(),
                ident,
                wire,
            }
        })
        .collect()
}

/// Emit a BMM enumeration class as a real Rust enum: one variant per named
/// constant, plus a tolerance-preserving `Other(String|i32)` catch-all.
///
/// The hand-written serde is provably byte-identical to the transparent
/// `String`/`i32` newtype it replaces: `serialize` writes `as_str`/`value` (the
/// constant token for a known variant, the verbatim payload for `Other`), and
/// `deserialize` reads the bare primitive then maps it through the total
/// `from_wire`/`from_value`. Because `as_str ∘ from_wire` (and `value ∘
/// from_value`) is the identity for every input, the round-trip preserves every
/// byte. A strict `TryFrom` seam alongside rejects out-of-set values with a
/// per-enum typed error and never yields `Other`.
fn emit_enum_literals(class: &BmmClass, enumeration: &BmmEnumeration) -> String {
    let ty = naming::type_name(&class.name);
    let spec = &class.name;
    let is_int = enumeration.underlying_type == "INTEGER";
    let lits = enum_literals(enumeration);
    let err_ty = format!("Unknown{ty}");
    let (payload, err_inner): (&str, &str) = if is_int {
        ("i32", "i64")
    } else {
        ("String", "String")
    };

    let mut b = String::new();
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{spec}`) — DO NOT EDIT.\n\n"
    ));
    doc_block(&mut b, class.documentation.as_deref(), "");
    let derive = if is_int {
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n"
    } else {
        "#[derive(Debug, Clone, PartialEq, Eq, Hash)]\n"
    };
    b.push_str(derive);
    b.push_str(&format!("pub enum {ty} {{\n"));
    for lit in &lits {
        match &lit.wire {
            EnumLitWire::Str(s) => b.push_str(&format!("    /// `{s}`\n")),
            EnumLitWire::Int(v) => b.push_str(&format!("    /// `{}` = {v}\n", lit.name)),
        }
        b.push_str(&format!("    {},\n", lit.ident));
    }
    b.push_str(&format!(
        "    /// A value outside the `{spec}` constant set.\n    ///\n    \
         /// NOTE: no openEHR spec governs an out-of-set value — our own\n    \
         /// tolerance-preserving design (`BMM_ENUMERATION` defines only the listed\n    \
         /// constants), retained so this enum's wire form stays byte-identical to\n    \
         /// the bare `{payload}` it replaces.\n    \
         Other({payload}),\n}}\n\n"
    ));

    // Inherent conversions.
    b.push_str(&format!("impl {ty} {{\n"));
    if is_int {
        b.push_str(
            "    /// The `i32` wire value of this constant (the verbatim payload for\n    \
             /// [`Self::Other`]).\n    #[must_use]\n    pub fn value(self) -> i32 {\n        match self {\n",
        );
        for lit in &lits {
            if let EnumLitWire::Int(v) = &lit.wire {
                b.push_str(&format!("            Self::{} => {v},\n", lit.ident));
            }
        }
        b.push_str("            Self::Other(__v) => __v,\n        }\n    }\n\n");
        b.push_str(
            "    /// This constant for an `i32` wire value, tolerating an unknown\n    \
             /// value as [`Self::Other`] (total — never fails).\n    #[must_use]\n    \
             pub fn from_value(__v: i32) -> Self {\n        match __v {\n",
        );
        for lit in &lits {
            if let EnumLitWire::Int(v) = &lit.wire {
                b.push_str(&format!("            {v} => Self::{},\n", lit.ident));
            }
        }
        b.push_str("            _ => Self::Other(__v),\n        }\n    }\n}\n\n");
    } else {
        b.push_str(
            "    /// The wire string of this constant (the verbatim token for\n    \
             /// [`Self::Other`]).\n    #[must_use]\n    pub fn as_str(&self) -> &str {\n        match self {\n",
        );
        for lit in &lits {
            if let EnumLitWire::Str(s) = &lit.wire {
                b.push_str(&format!("            Self::{} => {s:?},\n", lit.ident));
            }
        }
        b.push_str("            Self::Other(__s) => __s.as_str(),\n        }\n    }\n\n");
        b.push_str(
            "    /// This constant for a wire string, tolerating an unknown token\n    \
             /// as [`Self::Other`] (total — never fails).\n    #[must_use]\n    \
             pub fn from_wire(__s: &str) -> Self {\n        match __s {\n",
        );
        for lit in &lits {
            if let EnumLitWire::Str(s) = &lit.wire {
                b.push_str(&format!("            {s:?} => Self::{},\n", lit.ident));
            }
        }
        b.push_str("            _ => Self::Other(__s.to_owned()),\n        }\n    }\n}\n\n");
    }

    // Strict `TryFrom` seam (never yields `Other`).
    if is_int {
        b.push_str(&format!(
            "impl ::core::convert::TryFrom<i64> for {ty} {{\n    type Error = {err_ty};\n\n    \
             /// # Errors\n    /// Returns [`{err_ty}`] when `__v` is not a `{spec}` value\n    \
             /// (unlike [`Self::from_value`], which is total).\n    \
             fn try_from(__v: i64) -> ::core::result::Result<Self, Self::Error> {{\n        match __v {{\n"
        ));
        for lit in &lits {
            if let EnumLitWire::Int(v) = &lit.wire {
                b.push_str(&format!(
                    "            {v} => ::core::result::Result::Ok(Self::{}),\n",
                    lit.ident
                ));
            }
        }
        b.push_str(&format!(
            "            _ => ::core::result::Result::Err({err_ty}(__v)),\n        }}\n    }}\n}}\n\n"
        ));
    } else {
        b.push_str(&format!(
            "impl ::core::convert::TryFrom<&str> for {ty} {{\n    type Error = {err_ty};\n\n    \
             /// # Errors\n    /// Returns [`{err_ty}`] when `__s` is not a `{spec}` value\n    \
             /// (unlike [`Self::from_wire`], which is total).\n    \
             fn try_from(__s: &str) -> ::core::result::Result<Self, Self::Error> {{\n        match __s {{\n"
        ));
        for lit in &lits {
            if let EnumLitWire::Str(s) = &lit.wire {
                b.push_str(&format!(
                    "            {s:?} => ::core::result::Result::Ok(Self::{}),\n",
                    lit.ident
                ));
            }
        }
        b.push_str(&format!(
            "            _ => ::core::result::Result::Err({err_ty}(__s.to_owned())),\n        }}\n    }}\n}}\n\n"
        ));
    }

    // Byte-identical serde (see the fn doc for the proof).
    b.push_str(&format!(
        "impl ::serde::Serialize for {ty} {{\n    \
         fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>\n    \
         where\n        S: ::serde::Serializer,\n    {{\n"
    ));
    if is_int {
        b.push_str("        serializer.serialize_i32(self.value())\n    }\n}\n\n");
    } else {
        b.push_str("        serializer.serialize_str(self.as_str())\n    }\n}\n\n");
    }
    b.push_str(&format!(
        "impl<'de> ::serde::Deserialize<'de> for {ty} {{\n    \
         fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>\n    \
         where\n        D: ::serde::Deserializer<'de>,\n    {{\n"
    ));
    if is_int {
        b.push_str(
            "        let __v = <i32 as ::serde::Deserialize>::deserialize(deserializer)?;\n        \
             ::core::result::Result::Ok(Self::from_value(__v))\n    }\n}\n\n",
        );
    } else {
        b.push_str(
            "        let __s = <::std::string::String as ::serde::Deserialize>::deserialize(deserializer)?;\n        \
             ::core::result::Result::Ok(Self::from_wire(&__s))\n    }\n}\n\n",
        );
    }

    // The strict-seam error type (hand-rolled Display + Error, no `thiserror`).
    b.push_str(&format!(
        "/// The error returned by [`{ty}::try_from`] for a value outside the `{spec}`\n\
         /// constant set.\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {err_ty}(pub {err_inner});\n\n"
    ));
    let fmt = if is_int {
        format!("unknown {spec} value: {{}}")
    } else {
        format!("unknown {spec} value: {{:?}}")
    };
    b.push_str(&format!(
        "impl ::core::fmt::Display for {err_ty} {{\n    \
         fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{\n        \
         ::core::write!(f, {fmt:?}, self.0)\n    }}\n}}\n\n"
    ));
    b.push_str(&format!("impl ::std::error::Error for {err_ty} {{}}\n"));
    b
}

// ── import + header helpers ──────────────────────────────────────────────────

/// Precise `use` lines for a struct's referenced spec types: `crate::…` for
/// types emitted in this crate, `<dep>::prelude::…` for dependency types.
fn import_lines(
    model: &Model,
    class: &BmmClass,
    generics: &[String],
    subst: &BTreeMap<String, String>,
    self_ident: &str,
    index: &BTreeMap<String, Vec<String>>,
    external: &External,
) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for spec in model.referenced_specs(class, generics, subst) {
        add_import(&mut imports, &spec, self_ident, index, external);
    }
    imports
}

/// Resolve a referenced spec type to a `use` line: local (`crate::…`) wins,
/// then a dependency prelude; an unresolved type needs no import (it rendered as
/// `serde_json::Value`).
fn add_import(
    imports: &mut BTreeSet<String>,
    spec: &str,
    self_ident: &str,
    index: &BTreeMap<String, Vec<String>>,
    external: &External,
) {
    let ident = naming::type_name(spec);
    if ident == self_ident {
        return;
    }
    if let Some(chain) = index.get(&ident) {
        imports.insert(format!("use crate::{}::{};", chain.join("::"), ident));
    } else if let Some(path) = external.prelude_of(spec) {
        imports.insert(format!("use {path}::{ident};"));
    }
}

fn struct_header(b: &mut String, class: &str, imports: &BTreeSet<String>) {
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{class}`) — DO NOT EDIT.\n\
         // Hand-written spec functions/invariants live in the sibling `*_impl.rs`.\n\n"
    ));
    write_uses(b, &["use openehr_derive::OpenEhrType;"], imports);
}

/// Emit a crate's `use` block as a single lexicographically-sorted list (so the
/// output matches `rustfmt`'s default import ordering — `crate::…` before
/// `openehr_base::…` before `openehr_derive::…`/`serde::…`), followed by a blank
/// line. `fixed` holds always-present uses (the derive / serde); `imports` holds
/// the per-file resolved spec imports.
fn write_uses(b: &mut String, fixed: &[&str], imports: &BTreeSet<String>) {
    let mut all: BTreeSet<String> = imports.clone();
    for f in fixed {
        all.insert((*f).to_string());
    }
    for u in &all {
        b.push_str(u);
        b.push('\n');
    }
    b.push('\n');
}

fn doc_block(b: &mut String, doc: Option<&str>, indent: &str) {
    let Some(doc) = doc else { return };
    // Spec prose carries example blocks (ODIN snippets, `YYYY-MM-DDTHH:MM:SS`
    // date formats) that rustdoc would compile as Rust doctests and choke on.
    // Neutralize both forms it recognizes so the docs render as text, never run:
    //   - a bare ``` fence → tag the opening as ```text (closing stays bare);
    //   - a run of 4-space-indented lines → wrap it in a ```text fence.
    let mut push = |line: &str| {
        if line.is_empty() {
            b.push_str(&format!("{indent}///\n"));
        } else {
            b.push_str(&format!("{indent}/// {line}\n"));
        }
    };
    let mut in_fence = false; // inside an explicit ``` fence
    let mut in_indent = false; // inside an auto-wrapped indented block
    for line in doc.lines() {
        let line = line.trim_end();
        let stripped = line.trim_start();
        let lead = line.len() - stripped.len();

        if stripped.starts_with("```") && !in_indent {
            if in_fence {
                in_fence = false;
                push(line);
            } else {
                in_fence = true;
                push(&if stripped == "```" {
                    line.replacen("```", "```text", 1)
                } else {
                    line.to_string()
                });
            }
            continue;
        }
        if in_fence {
            push(line);
            continue;
        }

        let is_indent_line = lead >= 4 && !stripped.is_empty();
        if is_indent_line && !in_indent {
            push("```text");
            in_indent = true;
        } else if in_indent && !is_indent_line && !stripped.is_empty() {
            push("```");
            in_indent = false;
        }
        push(line);
    }
    if in_indent {
        push("```");
    }
    if in_fence {
        push("```");
    }
}

/// `DV_QUANTITY` → `dv_quantity`, `Iso8601_date` → `iso8601_date`.
fn to_snake(spec: &str) -> String {
    spec.to_lowercase()
}
