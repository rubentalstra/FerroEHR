//! The Rust emitter: walks a merged BMM [`Model`] and produces
//! idiomatic, strongly-typed Rust for the openEHR spec crates.
//!
//! Emission rules:
//! - **Flattened concrete structs**: a concrete class inlines all inherited
//!   fields (ancestor-first, `// inherited: X` banners); one hop to any field.
//! - **`Option<T>`** for non-mandatory single properties; **`Vec<T>`** for
//!   containers (optional containers get `default` + `skip_serializing_if`).
//! - **Enums** (plain closed subtype sets) for abstract classes used as a
//!   property type — the closed polymorphic slots (`DATA_VALUE`, `ITEM`, …).
//! - **Transparent newtypes** for enumeration classes that are just a
//!   primitive on the wire (`VALIDITY_KIND` → `String`).
//! - **Generics** only for classes the BMM declares generic (`Interval<T>`);
//!   the actual type argument is emitted at each use site.
//! - Canonical-JSON (de)serialization and the `_type` discriminator are NOT
//!   emitted here: the native `ToJson`/`FromJson` impls are generated into
//!   `openehr-its` by `emit-json`. These type files carry no serde/derive.
//! - Foundation **primitives / containers / marker traits** are mapped to Rust
//!   (bool, i32, Vec, …) and never emitted (see
//!   [`crate::plan::overrides::PRIMITIVES`] and
//!   [`crate::plan::overrides::MAPPED_CLASSES`]).
//! - **Every emitted public item carries documentation**: the BMM
//!   `documentation` where the vendored schema has it, else a deterministic
//!   synthesized line ([`synth_class_doc`], [`synth_field_doc`]) — `missing_docs`
//!   covers modules, structs, fields, enums and variants alike
//!   (<https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html>).
//!   Verbatim spec prose is sanitized for rustdoc ([`sanitize_doc_prose`]).
//!
//! Stage 4 — RENDER. The only stage that produces text: the per-shape emit
//! functions turn a planned class into deterministic, byte-stable Rust source.

use crate::analyze::{External, Model, class_paths};
use crate::load::bmm::{
    BmmClass, BmmConstant, BmmEnumValue, BmmEnumeration, BmmPropKind, BmmSchema, BmmType,
};
use crate::plan::composition::Composed;
use crate::plan::overrides::{back_reference, class_binding, type_override};
use crate::plan::{Emission, decide};
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
    /// The openEHR spec class the type realizes (`UID`) — the key the crate
    /// prelude's one-entry-per-name ownership is decided on.
    spec: String,
}

/// The generated files for one schema version plus its top-level module names.
struct Version {
    files: Vec<GenFile>,
    /// Top-level module names of this version (under its prefix, if any).
    top: BTreeSet<String>,
    /// The types this version emitted, for crate-prelude assembly (empty for a
    /// prefixed version, which emits its own prelude in-tree).
    emitted: Vec<Emitted>,
}

/// One BMM generation of a crate as the emitter consumes it.
///
/// A crate composed of several generations (LANG's stable v2.x BMM beside the
/// v3 development line — `LANG/docs/bmm3/master00-amendment_record.adoc`
/// SPECLANG-14) emits each one COMPLETELY from its own schema and its own
/// resolution model, so a class name declared by both yields two Rust types at
/// two source-package paths, each with its own shape and its own
/// intra-generation cross-references.
pub(crate) struct CrateGeneration<'a> {
    /// The resolution model this generation's classes resolve against.
    pub model: &'a Model,
    /// This generation's own schema (one vendored BMM file, verbatim).
    pub schema: &'a BmmSchema,
    /// The spec class names this generation contributes to the crate prelude,
    /// or `None` when it is the crate's sole generation (it then owns
    /// everything it emits).
    pub prelude_owned: Option<&'a BTreeSet<String>>,
}

/// The emission shapes that produce a file: [`Emission`] minus its `Skip`
/// variant. Narrowing at the planning step makes "a skipped class reached the
/// render loop" unrepresentable instead of a runtime check.
enum Shape<'a> {
    /// [`Emission::Struct`].
    Struct,
    /// [`Emission::Enum`].
    Enum(Vec<String>),
    /// [`Emission::PolyEnum`].
    PolyEnum(Vec<String>),
    /// [`Emission::EnumLiterals`].
    EnumLiterals(&'a BmmEnumeration),
    /// [`Emission::Newtype`].
    Newtype(&'a str),
}

impl<'a> Shape<'a> {
    /// The file-producing shape of an emission decision, or `None` for
    /// [`Emission::Skip`] (a mapped/primitive class emits no file).
    fn new(emission: Emission<'a>) -> Option<Self> {
        match emission {
            Emission::Struct => Some(Self::Struct),
            Emission::Enum(variants) => Some(Self::Enum(variants)),
            Emission::PolyEnum(variants) => Some(Self::PolyEnum(variants)),
            Emission::EnumLiterals(enumeration) => Some(Self::EnumLiterals(enumeration)),
            Emission::Newtype(prim) => Some(Self::Newtype(prim)),
            Emission::Skip => None,
        }
    }
}

/// Emit one schema version under `prefix` (empty for a single-version crate).
/// Produces the type files, the `mod.rs` tree, and a `prelude.rs`; the caller
/// assembles `lib.rs`.
fn emit_version(model: &Model, schema: &BmmSchema, prefix: &str, external: &External) -> Version {
    struct Planned<'a> {
        class: &'a BmmClass,
        shape: Shape<'a>,
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
    let mut emitted: Vec<Emitted> = Vec::new();
    for (name, class) in &schema.classes {
        let Some(shape) = Shape::new(decide(model, class, &used)) else {
            continue;
        };
        let pkg = class_pkg.get(name).cloned().unwrap_or_default();
        let mut chain: Vec<String> = Vec::new();
        if !prefix.is_empty() {
            chain.push(prefix.to_string());
        }
        chain.extend(pkg.split('/').filter(|s| !s.is_empty()).map(str::to_string));
        chain.push(naming::field_ident(&to_snake(name)));
        index.insert(naming::type_name(name), chain.clone());
        emitted.push(Emitted {
            chain: chain.clone(),
            ident: naming::type_name(name),
            spec: name.clone(),
        });
        // A polymorphic-concrete class emits a sibling `{Name}Data` struct in the
        // same file (the enum owns `{Name}`); export it from the prelude too so
        // downstream code (e.g. the generated XML impls) can name it.
        if matches!(shape, Shape::PolyEnum(_)) {
            index.insert(format!("{}Data", naming::type_name(name)), chain.clone());
            emitted.push(Emitted {
                chain: chain.clone(),
                ident: format!("{}Data", naming::type_name(name)),
                spec: name.clone(),
            });
        }
        planned.push(Planned {
            class,
            shape,
            chain,
        });
    }

    let mut files = Vec::new();
    for p in &planned {
        let body = match &p.shape {
            Shape::Struct => emit_struct(model, p.class, &index, &local, external),
            Shape::Enum(variants) => {
                emit_enum(model, p.class, variants, false, &index, &local, external)
            }
            Shape::PolyEnum(variants) => {
                emit_enum(model, p.class, variants, true, &index, &local, external)
            }
            Shape::EnumLiterals(enumeration) => emit_enum_literals(p.class, enumeration),
            Shape::Newtype(prim) => emit_newtype(p.class, prim),
        };
        files.push(GenFile {
            path: format!("{}.rs", p.chain.join("/")),
            body,
        });
    }

    let type_chains: Vec<Vec<String>> = planned.iter().map(|p| p.chain.clone()).collect();

    // Module tree. For a prefixed version, also register `<prefix>/prelude` so
    // the prefix `mod.rs` declares it.
    let mut tree_chains = type_chains.clone();
    if !prefix.is_empty() {
        tree_chains.push(vec![prefix.to_string(), "prelude".to_string()]);
    }
    files.extend(emit_module_tree(&tree_chains));
    // A prefixed version module carries its own spec-version constant, sourced
    // from the vendored BMM schema's `rm_release` (the crate-level constant in
    // `lib.rs` covers only the crate's primary generation).
    // A prefixed version module carries its own prelude + spec-version constant;
    // an unprefixed generation contributes to the crate-level prelude the caller
    // assembles (a crate may be composed of several BMM generations, and the
    // prelude carries exactly one entry per Rust type name).
    let emitted = if prefix.is_empty() {
        emitted
    } else {
        let mod_path = format!("{prefix}/mod.rs");
        for f in &mut files {
            if f.path == mod_path {
                f.body.push_str(&format!(
                    "\n/// The openEHR specification version this generation implements —\n\
                     /// the vendored BMM schema's `rm_release`.\n\
                     pub const SPEC_VERSION: &str = \"{}\";\n",
                    schema.rm_release
                ));
            }
        }
        files.push(emit_prelude(&emitted, &format!("{prefix}/prelude.rs")));
        Vec::new()
    };

    // Top modules: the prefix itself if prefixed, else the type roots.
    let top = if prefix.is_empty() {
        top_modules(&type_chains)
    } else {
        std::iter::once(prefix.to_string()).collect()
    };
    Version {
        files,
        top,
        emitted,
    }
}

/// Emit a single-generation crate (`openehr-base`): one schema, top-level
/// modules, crate `prelude`, and `lib.rs`. `external` resolves dependency-crate
/// types.
#[must_use]
pub(crate) fn emit_crate(
    model: &Model,
    schema: &BmmSchema,
    external: &External,
    crate_doc: &str,
) -> Vec<GenFile> {
    emit_generations(
        &[CrateGeneration {
            model,
            schema,
            prelude_owned: None,
        }],
        external,
        crate_doc,
    )
}

/// The crate-relative module path a class's emitted type lives at, e.g.
/// `bmm::core::bmm_class` — the same chain [`emit_version`] builds, so a
/// downstream emitter (`emit-json`) can name a type the crate prelude does not
/// export.
#[must_use]
pub(crate) fn type_module_path(schema: &BmmSchema, class: &str) -> String {
    let pkg = class_paths(schema).get(class).cloned().unwrap_or_default();
    let mut chain: Vec<String> = pkg
        .split('/')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    chain.push(naming::field_ident(&to_snake(class)));
    chain.join("::")
}

/// The per-generation render inputs of a resolved composition, in declaration
/// order — the bridge from [`crate::plan::composition::compose`] to
/// [`emit_generations`].
#[must_use]
pub(crate) fn crate_generations(composed: &Composed) -> Vec<CrateGeneration<'_>> {
    composed
        .generations
        .iter()
        .map(|g| CrateGeneration {
            model: &g.model,
            schema: &g.schema,
            prelude_owned: Some(&g.owned),
        })
        .collect()
}

/// Emit a crate composed of one or more BMM **generations** (`openehr-lang`):
/// every generation is rendered completely at its own source-package paths, and
/// the crate carries ONE prelude with one entry per Rust type name.
///
/// # Panics
/// Panics if two generations would write the same file (a silent shape pick —
/// one generation's output overwriting the other's), or if two generations both
/// claim the same prelude identifier. Both are emitter/table bugs: the
/// generations' source packages must be disjoint, and prelude ownership is
/// decided once in [`crate::plan::composition::Generation::owned`].
#[must_use]
pub(crate) fn emit_generations(
    generations: &[CrateGeneration<'_>],
    external: &External,
    crate_doc: &str,
) -> Vec<GenFile> {
    let mut files: Vec<GenFile> = Vec::new();
    let mut top: BTreeSet<String> = BTreeSet::new();
    let mut emitted: Vec<Emitted> = Vec::new();
    let mut paths: BTreeSet<String> = BTreeSet::new();
    let mut idents: BTreeSet<String> = BTreeSet::new();
    for g in generations {
        let v = emit_version(g.model, g.schema, "", external);
        for f in &v.files {
            assert!(
                paths.insert(f.path.clone()),
                "openehr-codegen: two BMM generations of one crate both emit {:?} — their \
                 source packages must be disjoint so each generation lands at its own path. \
                 Emitting one over the other would silently pick a single shape for a \
                 colliding class.",
                f.path,
            );
        }
        files.extend(v.files);
        top.extend(v.top);
        for e in v.emitted {
            if g.prelude_owned
                .is_some_and(|owned| !owned.contains(&e.spec))
            {
                continue;
            }
            assert!(
                idents.insert(e.ident.clone()),
                "openehr-codegen: two BMM generations both export {:?} from the crate prelude \
                 — prelude ownership must name exactly one generation per Rust type name.",
                e.ident,
            );
            emitted.push(e);
        }
    }
    files.push(emit_prelude(&emitted, "prelude.rs"));
    files.push(emit_lib(&top, true, crate_doc));
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
        // Walk the chain left to right, growing `dir` to the join of every
        // segment already passed — the index-free form of `chain[..i]`/`chain[i]`.
        let mut dir = String::new();
        for (i, segment) in chain.iter().enumerate() {
            if i > 0 {
                dirs.entry(dir.clone()).or_default().insert(segment.clone());
                dir.push('/');
            }
            dir.push_str(segment);
        }
    }
    dirs.into_iter()
        .map(|(dir, children)| {
            let mut b = String::from("// @generated by openehr-codegen — DO NOT EDIT.\n\n");
            // A module is a public item too, so it needs its own docs
            // (`missing_docs`, rustc lint listing).
            b.push_str(&format!(
                "//! The openEHR spec package `{dir}` — generated module tree.\n\n"
            ));
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
    //    terms, tabs, quote-style links, loose/overindented list continuation);
    //  - some spec classes carry >3 boolean flags (e.g. `Interval` bounds);
    //  - the package tree can nest a module of the same name (module_inception);
    //  - closed-slot enums can have size-disparate variants;
    //  - the spec owns the subtype names, so a closed set can share a prefix or
    //    postfix (`OBJECT_ID` ⊇ `TEMPLATE_ID`, `TERMINOLOGY_ID`, …) — renaming
    //    a variant would fork the spec model;
    //  - the spec likewise owns the ATTRIBUTE names, so a class's fields can
    //    share the class's own stem (`EHR.ehr_id`/`ehr_status`/`ehr_access`,
    //    `ARCHETYPE.archetype_id`, `C_TEMPORAL.valid_*`) — the BMM attribute
    //    name is the wire name, so `struct_field_names` cannot be satisfied
    //    without forking the model.
    // `reason` is mandatory (`clippy::allow_attributes_without_reason` is deny
    // workspace-wide); `expect` is wrong here because a given crate need not
    // trigger every listed lint.
    b.push_str(
        "#![allow(\n    \
         clippy::doc_markdown,\n    \
         clippy::doc_link_with_quotes,\n    \
         clippy::tabs_in_doc_comments,\n    \
         clippy::doc_lazy_continuation,\n    \
         clippy::doc_overindented_list_items,\n    \
         clippy::struct_excessive_bools,\n    \
         clippy::struct_field_names,\n    \
         clippy::module_inception,\n    \
         clippy::large_enum_variant,\n    \
         clippy::enum_variant_names,\n    \
         reason = \"inherent to faithful openEHR spec generation: verbatim spec \
         prose in doc comments, and spec-owned class/variant/field names (a field \
         name IS the normative BMM attribute name)\"\n\
         )]\n\
         // A vendored BMM model is a deep, mutually-recursive type graph (the LANG \
         // BMM-3 expression/statement families reach several hundred levels), so \
         // auto-trait inference — `Send`/`Sync`/`RefUnwindSafe`, which rustdoc \
         // evaluates for every item — overflows the default limit of 128. Raising \
         // the limit is exactly what rustc prescribes for that overflow \
         // (<https://doc.rust-lang.org/reference/attributes/limits.html>); it \
         // changes no emitted type.\n\
         #![recursion_limit = \"512\"]\n\n",
    );
    for m in top {
        b.push_str(&format!("pub mod {m};\n"));
    }
    if include_prelude {
        b.push_str("pub mod prelude;\n");
    }
    b.push_str(
        "\n/// The openEHR specification version this crate implements — the crate\n\
         /// version itself: the spec crates are versioned by the specification they\n\
         /// implement (`docs/VERSIONS.md` §Product and crate versioning), so\n\
         /// consumers read the pin from the package, never from a hand-typed literal.\n\
         pub const SPEC_VERSION: &str = env!(\"CARGO_PKG_VERSION\");\n",
    );
    GenFile {
        path: "lib.rs".to_string(),
        body: b,
    }
}

fn emit_prelude(emitted: &[Emitted], path: &str) -> GenFile {
    // The prelude carries ONE entry per Rust type name. A crate composed of two
    // BMM generations (openEHR publishes the stable v2.x BMM,
    // `LANG/docs/bmm/master01-preface.adoc` §History, beside the v3 development
    // line, `LANG/docs/bmm3/master01-preface.adoc` §Previous Versions) emits both
    // generations completely at their own source-package paths; where a class
    // name exists in both, the prelude exports the LAST-declared generation's
    // type and the other twin is reachable by its full module path only.
    let mut b = String::from(
        "//! Prelude: re-exports every generated spec type of this version.\n\
         //!\n//! @generated by openehr-codegen. Per-file imports are precise;\n\
         //! downstream crates and hand-written code may `use <path>::*`.\n\
         //!\n\
         //! ONE ENTRY PER RUST TYPE NAME. Where a crate is composed of several BMM\n\
         //! generations (openEHR publishes the stable v2.x BMM —\n\
         //! `LANG/docs/bmm/master01-preface.adoc` §History — beside the v3\n\
         //! development line — `LANG/docs/bmm3/master01-preface.adoc` §Previous\n\
         //! Versions), EVERY generation is emitted completely at its own\n\
         //! source-package path, and a class name declared by more than one is\n\
         //! exported here from the LAST-declared generation; the other\n\
         //! generation's twin is reachable by its full module path only.\n\n",
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
    b.push_str(&render_constants(class, &ty));
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
    doc_block_or(
        &mut b,
        class.documentation.as_deref(),
        "",
        &synth_class_doc(&class.name),
    );
    push_spec_alias(&mut b, &class.name, struct_ty, "");
    // No serde/`_type` derive: canonical-JSON (de)serialization is provided by
    // the emitted `ToJson`/`FromJson` impls in `openehr-its` (`emit-json`), not by
    // a per-struct derive. The type is a plain data record.
    b.push_str("#[derive(Debug, Clone, PartialEq)]\n");
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
        doc_block_or(
            &mut b,
            p.documentation.as_deref(),
            "    ",
            &synth_field_doc(&rp.owner, &p.name),
        );

        // The wire name (rename) and literal default are consumed by the JSON
        // codec emitter (`emit-json`, which reads them from the BMM), not from a
        // struct attribute — so no serde/`openehr` field attribute is emitted here.
        let ident = naming::field_ident(&p.name);
        let rust_ty = field_type(model, class, p, generics, subst, local, external);
        b.push_str(&format!("    pub {ident}: {rust_ty},\n"));
    }

    b.push_str("}\n");
    b
}

/// Emit the class's `BMM_CLASS.constants` as an `impl {ty} { pub const … }`
/// block (empty when the class has none). `ty` is the struct the constants hang
/// on — `type_name(class)` for a plain struct, `{Name}Data` for a polymorphic
/// slot. Each constant carries a doc line citing its verbatim BMM name (the BMM
/// is the authority); the literal is decoded from the raw BMM `value`.
fn render_constants(class: &BmmClass, ty: &str) -> String {
    if class.constants.is_empty() {
        return String::new();
    }
    let siblings: BTreeSet<&str> = class.constants.iter().map(|c| c.name.as_str()).collect();
    let mut b = String::new();
    b.push_str(&format!("\nimpl {ty} {{\n"));
    for (i, c) in class.constants.iter().enumerate() {
        if i > 0 {
            b.push('\n');
        }
        doc_block(&mut b, c.documentation.as_deref(), "    ");
        b.push_str(&format!("    /// BMM constant `{}`.\n", c.name));
        let (rust_ty, lit) = const_literal(c, &siblings);
        b.push_str(&format!(
            "    pub const {}: {rust_ty} = {lit};\n",
            naming::const_ident(&c.name)
        ));
    }
    b.push_str("}\n");
    b
}

/// Decode a BMM constant's raw `value` to a Rust `(type, literal)` pair. A JSON
/// number keys off the BMM `type` (`Real`/`Double` → `f64`, else `i64`); a JSON
/// string carries a quoted `"…"` (→ `&'static str`) or `'…'` (→ `char`) literal,
/// a bareword cross-reference to a sibling constant (→ `Self::OTHER`), or a
/// boolean keyword. Numeric character references (`&#42;`) and Eiffel octal
/// escapes (`\015`) inside literals are decoded.
fn const_literal(c: &BmmConstant, siblings: &BTreeSet<&str>) -> (String, String) {
    let is_real = matches!(c.type_name.as_str(), "Real" | "Double");
    match &c.value {
        serde_json::Value::Number(n) if is_real => (
            "f64".to_string(),
            format!("{:?}", n.as_f64().unwrap_or(0.0)),
        ),
        serde_json::Value::Number(n) => ("i64".to_string(), format!("{}", n.as_i64().unwrap_or(0))),
        serde_json::Value::Bool(b) => ("bool".to_string(), format!("{b}")),
        serde_json::Value::String(s) => {
            let t = s.trim();
            if let Some(inner) = strip_delims(t, '"') {
                (
                    "&'static str".to_string(),
                    format!("{:?}", decode_entities(inner)),
                )
            } else if let Some(inner) = strip_delims(t, '\'') {
                ("char".to_string(), format!("{:?}", decode_char(inner)))
            } else if siblings.contains(t) {
                let rust_ty = if is_real { "f64" } else { "i64" };
                (
                    rust_ty.to_string(),
                    format!("Self::{}", naming::const_ident(t)),
                )
            } else if c.type_name == "Boolean" {
                (
                    "bool".to_string(),
                    format!("{}", t.eq_ignore_ascii_case("true")),
                )
            } else {
                // A bareword that is neither a sibling nor a boolean: emit as a
                // string literal (verbatim), the safest total decoding.
                ("&'static str".to_string(), format!("{t:?}"))
            }
        }
        serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            ("&'static str".to_string(), "\"\"".to_string())
        }
    }
}

/// Strip a matching pair of delimiter characters (`"…"` or `'…'`) from `s`,
/// returning the inner text; `None` if `s` is not so delimited.
fn strip_delims(s: &str, delim: char) -> Option<&str> {
    s.strip_prefix(delim).and_then(|r| r.strip_suffix(delim))
}

/// Decode numeric character references (`&#42;` → `*`) in a BMM literal, without
/// byte slicing (clinical-text-safe). Non-references pass through verbatim.
fn decode_entities(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' && chars.peek() == Some(&'#') {
            chars.next();
            let mut num = String::new();
            while let Some(&d) = chars.peek() {
                if d == ';' {
                    chars.next();
                    break;
                }
                if d.is_ascii_digit() {
                    num.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if let Some(ch) = num.parse::<u32>().ok().and_then(char::from_u32) {
                out.push(ch);
            } else {
                out.push('&');
                out.push('#');
                out.push_str(&num);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Decode a single-character BMM literal body: numeric references first, then an
/// Eiffel octal escape (`\015` → CR); falls back to the first decoded char.
fn decode_char(inner: &str) -> char {
    let decoded = decode_entities(inner);
    if let Some(rest) = decoded.strip_prefix('\\')
        && !rest.is_empty()
        && rest.bytes().all(|b| (b'0'..=b'7').contains(&b))
        && let Some(ch) = u32::from_str_radix(rest, 8).ok().and_then(char::from_u32)
    {
        return ch;
    }
    decoded.chars().next().unwrap_or('\u{0}')
}

/// Compute a field's Rust type (the JSON codec handles `None`/empty omission at
/// its field call sites, so no attribute is needed on the field).
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
    // Each entry is `(variant ident, payload type, doc line)` — a variant is a
    // public item `missing_docs` checks, and the BMM has no per-subtype text for
    // a closed slot, so the line is synthesized from the subtype's spec name.
    let payloads: Vec<(String, String, String)> = variants
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
            let doc = format!("The `{d}` subtype of `{}`.", class.name);
            (variant, payload, doc)
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
        payloads.push((
            ty.clone(),
            data_payload,
            format!(
                "An instance of `{}` itself (its own, least-rich form).",
                class.name
            ),
        ));
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

    // No serde: the canonical-JSON `_type` dispatch (abstract slots require
    // `_type`; concrete polymorphic slots default a `_type`-less value to the
    // base type) is emitted as a native `FromJson` impl in `openehr-its`
    // (`emit-json`), and serialization as a native `ToJson` impl there. This
    // enum is a plain closed subtype set with no derive/serde attributes.
    file_header(&mut b, &class.name, self_data);
    write_uses(&mut b, &[], &imports);

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
        b.push_str(&render_constants(class, &data_ty));
        b.push('\n');
    }

    let slot = if self_data {
        "Polymorphic slot"
    } else {
        "Closed subtype set"
    };
    doc_summary_then(
        &mut b,
        &format!(
            "{slot} of `{}`, dispatched on each payload's `_type`.",
            class.name
        ),
        class.documentation.as_deref(),
        "",
    );
    push_spec_alias(&mut b, &class.name, &ty, "");
    b.push_str("#[derive(Debug, Clone, PartialEq)]\n");
    b.push_str(&format!("pub enum {ty}{gen_decl} {{\n"));
    for (variant, payload, doc) in &payloads {
        b.push_str(&format!("    /// {doc}\n"));
        b.push_str(&format!("    {variant}({payload}),\n"));
    }
    b.push_str("}\n");
    b
}

fn emit_newtype(class: &BmmClass, prim: &str) -> String {
    let ty = naming::type_name(&class.name);
    let mut b = String::new();
    file_header(&mut b, &class.name, false);
    doc_block_or(
        &mut b,
        class.documentation.as_deref(),
        "",
        &synth_class_doc(&class.name),
    );
    push_spec_alias(&mut b, &class.name, &ty, "");
    // A transparent primitive newtype; canonical-JSON (de)serialization is the
    // emitted `ToJson`/`FromJson` impl in `openehr-its` (`emit-json`), which
    // delegates through the inner primitive.
    b.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
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
    file_header(&mut b, spec, false);
    doc_block_or(
        &mut b,
        class.documentation.as_deref(),
        "",
        &synth_class_doc(spec),
    );
    push_spec_alias(&mut b, spec, &ty, "");
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

    // Canonical-JSON (de)serialization is the emitted `ToJson`/`FromJson` impl in
    // `openehr-its` (`emit-json`): `ToJson` writes `as_str`/`value` (the constant
    // token or verbatim `Other` payload) and `FromJson` maps the bare primitive
    // through the total `from_wire`/`from_value`, byte-identical to the primitive
    // it replaces. No serde impl is emitted here.

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
    file_header(b, class, true);
    write_uses(b, &[], imports);
}

/// The generated file's banner + its module documentation. Every generated type
/// file IS a module, and `missing_docs` checks modules, so the file carries an
/// inner `//!` summary naming the spec class it realizes (an out-of-line
/// module's inner docs satisfy the lint at the `pub mod` declaration site).
/// `impl_note` adds the sibling-`*_impl.rs` banner line.
fn file_header(b: &mut String, class: &str, impl_note: bool) {
    b.push_str(&format!(
        "// @generated by openehr-codegen from BMM (`{class}`) — DO NOT EDIT.\n"
    ));
    if impl_note {
        b.push_str("// Hand-written spec functions/invariants live in the sibling `*_impl.rs`.\n");
    }
    b.push_str(&format!(
        "\n//! The openEHR `{class}` spec class, generated from the vendored BMM\n\
         //! meta-model.\n\n"
    ));
}

/// A rustdoc search alias carrying the verbatim openEHR spelling, so
/// `EHR_STATUS` finds `EhrStatus`. Skipped when the Rust name already equals the
/// spec name — rustc rejects an alias identical to the item name.
fn push_spec_alias(b: &mut String, spec: &str, rust_ty: &str, indent: &str) {
    if spec != rust_ty {
        b.push_str(&format!("{indent}#[doc(alias = \"{spec}\")]\n"));
    }
}

/// The synthesized class-doc line for a BMM class the vendored schema carries no
/// `documentation` for (10 such classes across the pinned schemas as of the
/// current pins) — `missing_docs` admits no exceptions, and an honest
/// synthesized line beats a silent gap.
fn synth_class_doc(spec: &str) -> String {
    format!("The openEHR `{spec}` class (the vendored BMM carries no documentation for it).")
}

/// The synthesized attribute-doc line for a BMM property with no
/// `documentation`, naming the attribute and the class that declares it.
fn synth_field_doc(owner: &str, prop: &str) -> String {
    format!(
        "The `{prop}` attribute of openEHR `{owner}` (the vendored BMM carries no \
         documentation for it)."
    )
}

/// Emit a crate's `use` block as a single lexicographically-sorted list (so the
/// output matches `rustfmt`'s default import ordering — `crate::…` before
/// `openehr_base::…`), followed by a blank line. `fixed` holds always-present
/// uses (none now that the type files carry no derive/serde); `imports` holds
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

/// Emit `doc` as `///` lines, falling back to `fallback` when the vendored BMM
/// carries no documentation for the item. Every public item needs docs
/// (`missing_docs`), so there is no "no docs" branch.
fn doc_block_or(b: &mut String, doc: Option<&str>, indent: &str, fallback: &str) {
    if doc.is_some_and(|d| !d.trim().is_empty()) {
        doc_block(b, doc, indent);
    } else {
        b.push_str(&format!("{indent}/// {fallback}\n"));
    }
}

/// Emit a one-sentence `summary` line, then the BMM prose as the detail
/// paragraph after a blank `///` — the documented rustdoc shape (the first
/// paragraph is what search results and module indexes show:
/// <https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html>).
fn doc_summary_then(b: &mut String, summary: &str, doc: Option<&str>, indent: &str) {
    b.push_str(&format!("{indent}/// {summary}\n"));
    if doc.is_some_and(|d| !d.trim().is_empty()) {
        b.push_str(&format!("{indent}///\n"));
        doc_block(b, doc, indent);
    }
}

fn doc_block(b: &mut String, doc: Option<&str>, indent: &str) {
    let Some(doc) = doc else { return };
    // Spec prose carries example blocks (ODIN snippets, `YYYY-MM-DDTHH:MM:SS`
    // date formats) that rustdoc would compile as Rust doctests and choke on.
    // Neutralize both forms it recognizes so the docs render as text, never run:
    //   - a bare ``` fence → tag the opening as ```text (closing stays bare);
    //   - a run of 4-space-indented lines → wrap it in a ```text fence.
    // Prose OUTSIDE those blocks additionally goes through `sanitize_doc_prose`
    // (bare URLs, stray brackets/angle brackets — the rustdoc deny-lints).
    let mut out: Vec<String> = Vec::new();
    // Pending prose lines, sanitized as one segment so a code span may span
    // lines (the BMM has such spans, e.g. `BMM_SCHEMA_DESCRIPTOR.schema_id`).
    let mut prose: Vec<&str> = Vec::new();
    let flush = |prose: &mut Vec<&str>, out: &mut Vec<String>| {
        if prose.is_empty() {
            return;
        }
        let sanitized = sanitize_doc_prose(&prose.join("\n"));
        out.extend(sanitized.split('\n').map(str::to_string));
        prose.clear();
    };

    let mut in_fence = false; // inside an explicit ``` fence
    let mut in_indent = false; // inside an auto-wrapped indented block
    for line in doc.lines() {
        let line = line.trim_end();
        let stripped = line.trim_start();
        let lead = line.len() - stripped.len();

        if stripped.starts_with("```") && !in_indent {
            flush(&mut prose, &mut out);
            if in_fence {
                in_fence = false;
                out.push(line.to_string());
            } else {
                in_fence = true;
                out.push(if stripped == "```" {
                    line.replacen("```", "```text", 1)
                } else {
                    line.to_string()
                });
            }
            continue;
        }
        if in_fence {
            out.push(line.to_string());
            continue;
        }

        let is_indent_line = lead >= 4 && !stripped.is_empty();
        if is_indent_line && !in_indent {
            flush(&mut prose, &mut out);
            out.push("```text".to_string());
            in_indent = true;
        } else if in_indent && !is_indent_line && !stripped.is_empty() {
            out.push("```".to_string());
            in_indent = false;
        }
        if in_indent {
            out.push(line.to_string());
        } else {
            prose.push(line);
        }
    }
    flush(&mut prose, &mut out);
    if in_indent {
        out.push("```".to_string());
    }
    if in_fence {
        out.push("```".to_string());
    }

    for line in &out {
        if line.is_empty() {
            b.push_str(&format!("{indent}///\n"));
        } else {
            b.push_str(&format!("{indent}/// {line}\n"));
        }
    }
}

/// Make verbatim openEHR spec prose safe for the workspace's deny-level rustdoc
/// lints, leaving code spans untouched (rustdoc never lints inside them):
///
/// - a bare URL becomes an autolink `<url>`, and the asciidoc `url[label]` form
///   becomes a Markdown link (`rustdoc::bare_urls`);
/// - a literal `[…]` is escaped `\[…\]`, so prose is not read as a broken
///   intra-doc link (`rustdoc::broken_intra_doc_links`);
/// - a literal `<…>` is escaped `\<…\>`, so `"name <email>"` / `VERSION<T>` in
///   prose is not read as an HTML tag (`rustdoc::invalid_html_tags`);
/// - an unpaired backtick run is escaped, so it opens no dangling code span
///   (`rustdoc::unescaped_backticks`).
fn sanitize_doc_prose(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        match c {
            '`' => {
                let open = backtick_run(rest);
                // A matched pair delimits a code span: copy it verbatim, closing
                // run included. An unmatched run is escaped instead.
                if let Some(offset) = find_backtick_run(after(rest, open), open) {
                    let end = open + offset + open;
                    out.push_str(upto(rest, end));
                    rest = after(rest, end);
                } else {
                    for _ in 0..open {
                        out.push_str("\\`");
                    }
                    rest = after(rest, open);
                }
            }
            'h' if rest.starts_with("http://") || rest.starts_with("https://") => {
                let url = read_url(rest);
                let tail = after(rest, url.len());
                // asciidoc link form `https://host/path[label]`. A parenthesis in
                // the URL would end the Markdown destination early, so such a
                // link stays an autolink with escaped brackets.
                let plain_dest = !url.contains(['(', ')']);
                if let Some((consumed, label)) = read_link_label(tail).filter(|_| plain_dest) {
                    out.push_str(&format!("[{label}]({url})"));
                    rest = after(tail, consumed);
                } else {
                    out.push_str(&format!("<{url}>"));
                    rest = tail;
                }
            }
            '[' | ']' | '<' | '>' => {
                out.push('\\');
                out.push(c);
                rest = after(rest, c.len_utf8());
            }
            c => {
                out.push(c);
                rest = after(rest, c.len_utf8());
            }
        }
    }
    out
}

/// `s` after its first `n` bytes — total (empty when `n` is out of range or not
/// on a char boundary), so no slicing can panic.
fn after(s: &str, n: usize) -> &str {
    s.get(n..).unwrap_or_default()
}

/// The first `n` bytes of `s` — total, like [`after`].
fn upto(s: &str, n: usize) -> &str {
    s.get(..n).unwrap_or_default()
}

/// The length of the backtick run at the start of `s`.
fn backtick_run(s: &str) -> usize {
    s.chars().take_while(|&c| c == '`').count()
}

/// The byte offset in `s` of the next backtick run of exactly `n`, if any.
fn find_backtick_run(s: &str, n: usize) -> Option<usize> {
    let mut rest = s;
    let mut base = 0;
    while let Some(idx) = rest.find('`') {
        let run = backtick_run(after(rest, idx));
        if run == n {
            return Some(base + idx);
        }
        base += idx + run;
        rest = after(rest, idx + run);
    }
    None
}

/// The URL at the start of `s`: up to whitespace or a delimiter, with trailing
/// sentence punctuation left outside the link.
fn read_url(s: &str) -> &str {
    let len = s
        .chars()
        .take_while(|&c| {
            !c.is_whitespace() && !matches!(c, '`' | '[' | ']' | '<' | '>' | '"' | '\'')
        })
        .map(char::len_utf8)
        .sum();
    let mut url = upto(s, len);
    while let Some(trimmed) = url.strip_suffix(['.', ',', ';', ':', ')']) {
        url = trimmed;
    }
    url
}

/// An asciidoc link label `[label]` at the start of `s`, if it is usable as
/// Markdown link text (no nested bracket or code span). Returns
/// `(bytes consumed incl. both brackets, label)`.
fn read_link_label(s: &str) -> Option<(usize, &str)> {
    let body = s.strip_prefix('[')?;
    let close = body.find(']')?;
    let label = upto(body, close);
    if label.is_empty() || label.contains(['[', '`']) {
        return None;
    }
    Some((close + 2, label))
}

/// `DV_QUANTITY` → `dv_quantity`, `Iso8601_date` → `iso8601_date`.
fn to_snake(spec: &str) -> String {
    spec.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{doc_block, sanitize_doc_prose};

    #[test]
    fn bare_url_becomes_an_autolink() {
        assert_eq!(
            sanitize_doc_prose("specified by https://www.rfc-editor.org/rfc/rfc1034."),
            "specified by <https://www.rfc-editor.org/rfc/rfc1034>.",
        );
    }

    #[test]
    fn asciidoc_link_becomes_a_markdown_link() {
        assert_eq!(
            sanitize_doc_prose("Coded using https://example.org/t.xml[openEHR vocabulary]."),
            "Coded using [openEHR vocabulary](https://example.org/t.xml).",
        );
    }

    #[test]
    fn prose_brackets_and_angle_brackets_are_escaped() {
        assert_eq!(
            sanitize_doc_prose(r#"in "name <email>" form, table [ [ String ] ]"#),
            r#"in "name \<email\>" form, table \[ \[ String \] \]"#,
        );
    }

    #[test]
    fn code_spans_are_copied_verbatim() {
        // rustdoc lints nothing inside a code span, so `Interval<T>` and a URL in
        // backticks must survive untouched — including a span crossing lines.
        assert_eq!(
            sanitize_doc_prose("substitutable for `Interval<T>` where needed"),
            "substitutable for `Interval<T>` where needed",
        );
        assert_eq!(
            sanitize_doc_prose("formed by\n\n`create_schema_id(\n  publisher)`\n\ne.g. x"),
            "formed by\n\n`create_schema_id(\n  publisher)`\n\ne.g. x",
        );
    }

    #[test]
    fn an_unpaired_backtick_is_escaped() {
        assert_eq!(
            sanitize_doc_prose("the property `tuple_constraint', and comes"),
            "the property \\`tuple_constraint', and comes",
        );
    }

    #[test]
    fn fenced_and_indented_blocks_are_left_alone() {
        let mut out = String::new();
        doc_block(
            &mut out,
            Some("Values:\n\n```\n\"YYYY-MM-DD\" -- [full]\n```"),
            "",
        );
        assert_eq!(
            out,
            "/// Values:\n///\n/// ```text\n/// \"YYYY-MM-DD\" -- [full]\n/// ```\n",
        );
    }
}
