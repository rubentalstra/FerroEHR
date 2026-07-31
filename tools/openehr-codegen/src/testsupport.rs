//! A curated public surface over the pipeline for the emitter-invariant tests
//! (`tests/emitter_invariants.rs`). It runs the **real** pipeline on the **real**
//! vendored inputs and returns plain data — the stage-2/3/4 facts the invariants
//! assert over — so the tests never reach into crate internals.
//!
//! This module is test scaffolding, not part of the generator's output path; it
//! only reads the same tables and functions `cli.rs` drives.

use crate::analyze::invariants::{self, Bucket};
use crate::analyze::{Model, augment_with_reemit, class_paths, cross_schema_reemit};
use crate::load::bmm::BmmSchema;
use crate::plan::composition::{self, compose};
use crate::plan::overrides;
use crate::plan::{Emission, decide};
use crate::render::emit::{GenFile, emit_crate, emit_multi_crate};
use crate::render::{emit_rm_model, emit_validate};
use std::collections::{BTreeMap, BTreeSet};

type Error = Box<dyn std::error::Error>;

/// The composition keys, in emission order.
#[must_use]
pub fn crate_keys() -> Vec<&'static str> {
    composition::COMPOSITIONS.iter().map(|c| c.key).collect()
}

/// One crate → schema-merge composition entry, flattened for the integrity
/// invariant (the merge table is itself declarative decision data).
#[derive(Debug, Clone)]
pub struct CompositionInfo {
    /// The composition key.
    pub key: String,
    /// The emitted crate directory.
    pub crate_name: String,
    /// The version-module prefix for a multi-version crate, else `None`.
    pub variant: Option<String>,
    /// The crate's own BMM member files.
    pub own: Vec<String>,
    /// Dependency composition keys merged into the model.
    pub model_deps: Vec<String>,
    /// Dependency composition keys whose prelude the `External` index offers.
    pub prelude_deps: Vec<String>,
    /// The `includes` citation behind the merge.
    pub citation: String,
    /// The one-line reason.
    pub reason: String,
}

/// Every crate → schema-merge composition entry.
#[must_use]
pub fn composition_infos() -> Vec<CompositionInfo> {
    composition::COMPOSITIONS
        .iter()
        .map(|c| CompositionInfo {
            key: c.key.to_string(),
            crate_name: c.crate_name.to_string(),
            variant: c.variant.map(str::to_string),
            own: c.own.iter().map(|s| (*s).to_string()).collect(),
            model_deps: c.model_deps.iter().map(|s| (*s).to_string()).collect(),
            prelude_deps: c.prelude_deps.iter().map(|s| (*s).to_string()).collect(),
            citation: c.citation.to_string(),
            reason: c.reason.to_string(),
        })
        .collect()
}

// ── completeness ────────────────────────────────────────────────────────────

/// Per-crate class-count breakdown for the completeness invariant.
#[derive(Debug, Clone)]
pub struct Completeness {
    /// The composition key.
    pub key: String,
    /// Total classes in the crate's own schema.
    pub total: usize,
    /// Classes planned (emitted, `decide` ≠ `Skip`).
    pub planned: usize,
    /// Classes skipped because they are mapped to Rust (primitive / container /
    /// marker / functional / service / constant-holder).
    pub skipped_mapped: usize,
    /// Classes skipped because they are abstract with no concrete descendants
    /// and are never referenced as a field type (the only other sanctioned
    /// skip).
    pub skipped_abstract_unused: usize,
    /// Classes that vanished for **neither** sanctioned reason — a silent drop.
    /// The completeness invariant requires this to be empty.
    pub silently_dropped: Vec<String>,
}

/// Compute the completeness breakdown for one crate's own schema.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn completeness(key: &str) -> Result<Completeness, Error> {
    let c = compose(key)?;
    let used = c.model.used_as_type();
    let mut planned = 0;
    let mut skipped_mapped = 0;
    let mut skipped_abstract_unused = 0;
    let mut silently_dropped = Vec::new();
    for (name, class) in &c.own_schema.classes {
        match decide(&c.model, class, &used) {
            Emission::Skip => {
                if Model::is_mapped(name) {
                    skipped_mapped += 1;
                } else if class.is_abstract
                    && c.model.enum_variants(name).is_empty()
                    && !used.contains(name)
                {
                    skipped_abstract_unused += 1;
                } else {
                    silently_dropped.push(name.clone());
                }
            }
            _ => planned += 1,
        }
    }
    silently_dropped.sort();
    Ok(Completeness {
        key: key.to_string(),
        total: c.own_schema.classes.len(),
        planned,
        skipped_mapped,
        skipped_abstract_unused,
        silently_dropped,
    })
}

// ── invariant classification (assertion-dialect analyzer) ────────────────────

/// One classified BMM invariant: the owning class, the invariant name, its
/// verbatim assertion expression, and the R5 bucket the assertion-dialect
/// analyzer assigns it.
#[derive(Debug, Clone)]
pub struct ClassifiedInvariant {
    /// The owning BMM class name.
    pub class: String,
    /// The BMM invariant name.
    pub name: String,
    /// The verbatim assertion expression.
    pub expr: String,
    /// The bucket: `"emitted"`, `"runtime-hook-missing"`, or `"complex"`.
    pub bucket: &'static str,
    /// The hook/complex reason (empty for `"emitted"`).
    pub reason: String,
}

/// Classify every `BMM_CLASS.invariants` expression in a crate's own schema
/// with the assertion-dialect analyzer, returning one row per invariant, sorted
/// by (class, name) for determinism.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn classify_invariants(key: &str) -> Result<Vec<ClassifiedInvariant>, Error> {
    let c = compose(key)?;
    let mut out = Vec::new();
    for (class, def) in &c.own_schema.classes {
        for (name, expr) in &def.invariants {
            let (bucket, reason) = match invariants::classify(expr) {
                Bucket::Emitted => ("emitted", String::new()),
                Bucket::RuntimeHookMissing(r) => ("runtime-hook-missing", r.to_string()),
                Bucket::Complex(r) => ("complex", r.to_string()),
            };
            out.push(ClassifiedInvariant {
                class: class.clone(),
                name: name.clone(),
                expr: expr.clone(),
                bucket,
                reason,
            });
        }
    }
    out.sort_by(|a, b| (&a.class, &a.name).cmp(&(&b.class, &b.name)));
    Ok(out)
}

// ── constructibility ────────────────────────────────────────────────────────

/// The non-constructible concrete classes of a crate's own schema (an unbroken
/// mandatory single-valued construction cycle). The invariant requires empty.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn constructibility_offenders(key: &str) -> Result<Vec<String>, Error> {
    let c = compose(key)?;
    Ok(c.model.constructibility_violations(&c.own_schema))
}

// ── determinism ─────────────────────────────────────────────────────────────

/// The decided Rust shape (`Struct` / `Enum` / `PolyEnum` / `EnumLiterals` /
/// `Newtype` / `Skip`) of every class in a crate's own schema, keyed by class
/// name — the stage-3 plan, as comparable data.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn plan_shapes(key: &str) -> Result<BTreeMap<String, String>, Error> {
    let c = compose(key)?;
    let used = c.model.used_as_type();
    let mut out = BTreeMap::new();
    for (name, class) in &c.own_schema.classes {
        out.insert(
            name.clone(),
            shape_name(&decide(&c.model, class, &used)).to_string(),
        );
    }
    Ok(out)
}

fn shape_name(e: &Emission) -> &'static str {
    match e {
        Emission::Struct => "Struct",
        Emission::Enum(_) => "Enum",
        Emission::PolyEnum(_) => "PolyEnum",
        Emission::EnumLiterals(_) => "EnumLiterals",
        Emission::Newtype(_) => "Newtype",
        Emission::Skip => "Skip",
    }
}

/// Render every emit-crate's spec files to an in-memory map keyed
/// `"<crate>/<path>"` (pre-rustfmt bodies). Reproduces `cli::cmd_emit`'s emission
/// half without touching the filesystem, so a double call proves the emitter is
/// byte-deterministic.
///
/// # Errors
/// Returns an error if any crate's BMM files cannot be loaded.
pub fn render_all_to_memory() -> Result<BTreeMap<String, String>, Error> {
    let mut out = BTreeMap::new();
    let mut add = |crate_name: &str, files: &[GenFile]| {
        for f in files {
            out.insert(format!("{crate_name}/{}", f.path), f.body.clone());
        }
    };

    let base = compose("base")?;
    add(
        "openehr-base",
        &emit_crate(&base.model, &base.own_schema, &base.external, base.doc),
    );

    let rm = compose("rm")?;
    let mut rm_files = emit_crate(&rm.model, &rm.own_schema, &rm.external, rm.doc);
    inject_rm_model(&mut rm_files, emit_rm_model::emit_files(&rm.model));
    inject_validate(
        &mut rm_files,
        emit_validate::emit_files(&rm.model, &rm.own_schema),
    );
    add("openehr-rm", &rm_files);

    let lang = compose("lang")?;
    add(
        "openehr-lang",
        &emit_crate(&lang.model, &lang.own_schema, &lang.external, lang.doc),
    );

    let am14 = compose("am14")?;
    let am24 = compose("am24")?;
    let reemit = cross_schema_reemit(&am24.model, &am24.own_schema);
    let dep_refs: Vec<&BmmSchema> = am24.dep_schemas.iter().collect();
    let am24_aug = augment_with_reemit(&am24.own_schema, &am24.model, &reemit, &dep_refs);
    add(
        "openehr-am",
        &emit_multi_crate(
            &[
                ("am14", &am14.model, &am14.own_schema),
                ("am24", &am24.model, &am24_aug),
            ],
            &am24.external,
            am24.doc,
        ),
    );

    let term = compose("term")?;
    add(
        "openehr-term",
        &emit_crate(&term.model, &term.own_schema, &term.external, term.doc),
    );
    Ok(out)
}

/// Mirror of `cli::inject_validate` (kept private there): append the generated
/// invariant-core file (the module is declared by a hand edit in `validate.rs`).
fn inject_validate(files: &mut Vec<GenFile>, mut validate_files: Vec<GenFile>) {
    files.append(&mut validate_files);
}

/// Mirror of `cli::inject_rm_model` (kept private there): append the RM-model
/// files and declare `pub mod model;` in the crate's `lib.rs`.
fn inject_rm_model(files: &mut Vec<GenFile>, mut model_files: Vec<GenFile>) {
    for f in files.iter_mut() {
        if f.path == "lib.rs" && !f.body.contains("pub mod model;") {
            f.body.push_str("pub mod model;\n");
        }
    }
    files.append(&mut model_files);
}

// ── cross-schema re-emission (source-package mirroring + downstream closure) ──

/// One re-emitted cross-schema class: where its package sits in the upstream
/// source schema vs where it is grafted in the downstream (AM 2.4) schema.
#[derive(Debug, Clone)]
pub struct Mirror {
    /// The re-emitted class name.
    pub class: String,
    /// Its package path in the upstream source schema (BASE/LANG).
    pub source_path: Option<String>,
    /// Its package path in the augmented downstream (AM 2.4) schema.
    pub augmented_path: Option<String>,
}

/// The AM 2.4 downstream re-emission closure (the upstream classes whose Rust
/// form widens downstream) with the source vs augmented package paths of each.
///
/// # Errors
/// Returns an error if the AM/BASE/LANG BMM files cannot be loaded.
pub fn am24_reemit_mirrors() -> Result<Vec<Mirror>, Error> {
    let am24 = compose("am24")?;
    let reemit = cross_schema_reemit(&am24.model, &am24.own_schema);
    let dep_refs: Vec<&BmmSchema> = am24.dep_schemas.iter().collect();
    let aug = augment_with_reemit(&am24.own_schema, &am24.model, &reemit, &dep_refs);
    let aug_paths = class_paths(&aug);

    // Source package path per class, first-wins across the dependency schemas
    // (BASE before LANG — the same order `augment_with_reemit` grafts).
    let mut source_paths: BTreeMap<String, String> = BTreeMap::new();
    for dep in &am24.dep_schemas {
        for (cls, path) in class_paths(dep) {
            source_paths.entry(cls).or_insert(path);
        }
    }

    Ok(reemit
        .iter()
        .map(|class| Mirror {
            class: class.clone(),
            source_path: source_paths.get(class).cloned(),
            augmented_path: aug_paths.get(class).cloned(),
        })
        .collect())
}

/// The AM 2.4 downstream re-emission closure (class-name set).
///
/// # Errors
/// Returns an error if the AM/BASE/LANG BMM files cannot be loaded.
pub fn am24_reemit_closure() -> Result<BTreeSet<String>, Error> {
    let am24 = compose("am24")?;
    Ok(cross_schema_reemit(&am24.model, &am24.own_schema))
}

/// The set of type files (`"<crate>/<path>"`) emitted for one composition
/// variant — the crate `key`'s rendered output alone (not shared crate mates).
/// Used to prove upstream (LANG) output is untouched by downstream (AM) analysis.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn rendered_files(key: &str) -> Result<Vec<String>, Error> {
    let c = compose(key)?;
    let files = emit_crate(&c.model, &c.own_schema, &c.external, c.doc);
    Ok(files.into_iter().map(|f| f.path).collect())
}

// ── decision-map integrity ──────────────────────────────────────────────────

/// One declarative decision entry, flattened for integrity checks.
#[derive(Debug, Clone)]
pub struct DeclEntry {
    /// The lookup key (`CLASS`, `CLASS.field`, or a primitive/class name).
    pub key: String,
    /// The decision the entry encodes.
    pub decision: String,
    /// The spec citation, or the explicit our-own-design flag.
    pub citation: String,
    /// The one-line reason.
    pub reason: String,
}

/// Every declarative decision entry, tagged by which map it came from, for the
/// integrity invariant (non-empty citation; the keyed class/field exists).
#[derive(Debug, Clone)]
pub struct DeclMap {
    /// The map's name (`back_reference`, `class_binding`, …).
    pub map: &'static str,
    /// Whether the integrity test must verify the entry's `(class[, field])`
    /// exists in a loaded schema (true for binding maps; false for the
    /// mapped-class denylist and the primitive type map, whose extra entries
    /// are inert).
    pub check_existence: bool,
    /// The entries.
    pub entries: Vec<DeclEntry>,
}

/// Every declarative decision map, flattened for the integrity invariant.
#[must_use]
pub fn decision_maps() -> Vec<DeclMap> {
    vec![
        DeclMap {
            map: "back_reference",
            check_existence: true,
            entries: overrides::BACK_REFERENCES
                .iter()
                .map(|e| DeclEntry {
                    key: format!("{}.{}", e.class, e.field),
                    decision: "owner/parent back-reference (omitted from struct)".to_string(),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "class_binding",
            check_existence: true,
            entries: overrides::CLASS_BINDINGS
                .iter()
                .map(|e| DeclEntry {
                    key: e.class.to_string(),
                    decision: format!("{} → {}", e.param, e.concrete),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "type_override",
            check_existence: true,
            entries: overrides::TYPE_OVERRIDES
                .iter()
                .map(|e| DeclEntry {
                    key: format!("{}.{}", e.class, e.field),
                    decision: e.rust_type.to_string(),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "field_default",
            check_existence: true,
            entries: overrides::FIELD_DEFAULTS
                .iter()
                .map(|e| DeclEntry {
                    key: format!("{}.{}", e.owner, e.field),
                    decision: format!("default = {}", e.default),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "xml_bmm_only_allowlist",
            check_existence: true,
            entries: overrides::XML_BMM_ONLY_ALLOWLIST
                .iter()
                .map(|e| DeclEntry {
                    key: format!("{}.{}", e.spec, e.wire_name),
                    decision: "append as trailing canonical-XML element".to_string(),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "mapped_classes",
            check_existence: false,
            entries: overrides::MAPPED_CLASSES
                .iter()
                .map(|e| DeclEntry {
                    key: e.name.to_string(),
                    decision: "mapped to Rust / never emitted".to_string(),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "primitives",
            check_existence: false,
            entries: overrides::PRIMITIVES
                .iter()
                .map(|e| DeclEntry {
                    key: e.spec.to_string(),
                    decision: e.rust.to_string(),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "subtype_extensions",
            check_existence: true,
            entries: overrides::SUBTYPE_EXTENSIONS
                .iter()
                .map(|e| DeclEntry {
                    key: e.subtype.to_string(),
                    decision: format!("additional variant of {}", e.parent),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "dialect_predicates",
            // The keys are BMM assertion-dialect predicate spellings, not
            // class/field names, so existence is checked against the classifier
            // (see `dialect_predicates_match_the_classifier`), not the schema.
            check_existence: false,
            entries: overrides::DIALECT_PREDICATES
                .iter()
                .map(|e| DeclEntry {
                    key: e.predicate.to_string(),
                    decision: format!("→ {}", e.runtime_fn),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
    ]
}

/// The assertion-dialect predicate → runtime-function map (predicate spelling,
/// runtime function), for the lockstep test against the classifier.
#[must_use]
pub fn dialect_predicates() -> Vec<(String, String)> {
    overrides::DIALECT_PREDICATES
        .iter()
        .map(|e| (e.predicate.to_string(), e.runtime_fn.to_string()))
        .collect()
}

/// The declared additional polymorphic subtype members
/// (`analyze`-level inheritance edges the vendored BMM under-declares), as
/// `(parent, subtype)` pairs.
#[must_use]
pub fn subtype_extensions() -> Vec<(String, String)> {
    overrides::SUBTYPE_EXTENSIONS
        .iter()
        .map(|e| (e.parent.to_string(), e.subtype.to_string()))
        .collect()
}

/// The immediate concrete variants the emitter gives `class`'s polymorphic slot
/// in the composition `key`, or `None` when that composition's model does not
/// define `class`.
///
/// # Errors
/// Returns an error if the composition's BMM files cannot be loaded.
pub fn enum_variants(key: &str, class: &str) -> Result<Option<Vec<String>>, Error> {
    let c = compose(key)?;
    Ok(c.model
        .get(class)
        .map(|_| c.model.enum_variants(class))
        .map(|mut variants| {
            variants.sort();
            variants
        }))
}

/// The classifier's recognised runtime-backed leaf predicates
/// (`analyze::invariants::RUNTIME_PREDICATES`).
#[must_use]
pub fn runtime_predicates() -> Vec<String> {
    invariants::runtime_predicates()
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Does `class` exist as a class in any loaded composition model?
///
/// # Errors
/// Returns an error if any composition's BMM files cannot be loaded.
pub fn class_exists(class: &str) -> Result<bool, Error> {
    for key in crate_keys() {
        if compose(key)?.model.get(class).is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Does `class` carry a (flattened) field named `field` in any loaded
/// composition model?
///
/// # Errors
/// Returns an error if any composition's BMM files cannot be loaded.
pub fn field_exists(class: &str, field: &str) -> Result<bool, Error> {
    for key in crate_keys() {
        let c = compose(key)?;
        if let Some(cls) = c.model.get(class)
            && c.model
                .flattened_props(cls)
                .iter()
                .any(|rp| rp.prop.name == field)
        {
            return Ok(true);
        }
    }
    Ok(false)
}
