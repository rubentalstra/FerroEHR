//! A curated public surface over the pipeline for the emitter-invariant tests
//! (`tests/emitter_invariants.rs`).
//!
//! It runs the **real** pipeline on the **real** vendored inputs and returns
//! plain data — the stage-2/3/4 facts the invariants assert over — so the
//! tests never reach into crate internals.
//!
//! This module is test scaffolding, not part of the generator's output path; it
//! only reads the same tables and functions `cli.rs` drives.

#![expect(
    clippy::disallowed_types,
    reason = "dev tooling over JSON artifacts (vendored BMM/OAS bundles, emitter reports) — not the \
              application (#1694)"
)]
use crate::analyze::invariants::{self, Bucket};
use crate::analyze::{Model, augment_with_reemit, class_paths, cross_schema_reemit};
use crate::load::bmm::BmmSchema;
use crate::load::impls::SiblingImpls;
use crate::load::oas;
use crate::plan::composition::{self, compose};
use crate::plan::overrides;
use crate::plan::{Emission, decide};
use crate::render::emit::{CrateGeneration, GenFile, RenderUnit, emit_composed};
use crate::render::{emit_rest, emit_rm_model, emit_validate, model_query, naming};
use std::collections::{BTreeMap, BTreeSet};

type Error = Box<dyn std::error::Error>;

/// The composition keys, in emission order.
#[must_use]
pub fn crate_keys() -> Vec<&'static str> {
    composition::COMPOSITIONS.iter().map(|c| c.key).collect()
}

/// One generation row of a composition entry, flattened for the integrity
/// invariant.
#[derive(Debug, Clone)]
pub struct GenerationInfo {
    /// The emitted generation-module name (`v1_2`).
    pub module: String,
    /// The generation's implemented spec version.
    pub spec_version: String,
    /// The vendored spec files composing the generation, each with its
    /// prelude membership.
    pub units: Vec<(String, bool)>,
    /// Whether this is the crate's current generation.
    pub current: bool,
    /// Paired dependency generations merged into the model, as
    /// `(key, generation)` pairs in merge order.
    pub model_deps: Vec<(String, String)>,
    /// Paired dependency generations resolving cross-crate references, as
    /// `(key, generation)` pairs in lookup order.
    pub prelude_deps: Vec<(String, String)>,
}

/// One crate → generation composition entry, flattened for the integrity
/// invariant (the table is itself declarative decision data).
#[derive(Debug, Clone)]
pub struct CompositionInfo {
    /// The composition key.
    pub key: String,
    /// The emitted crate directory.
    pub crate_name: String,
    /// The crate's BMM generations, oldest first.
    pub generations: Vec<GenerationInfo>,
    /// The `includes` citation behind the composition.
    pub citation: String,
    /// The one-line reason.
    pub reason: String,
}

/// Every crate → generation composition entry.
#[must_use]
pub fn composition_infos() -> Vec<CompositionInfo> {
    let deps = |d: &[composition::DepGeneration]| {
        d.iter()
            .map(|d| (d.key.to_string(), d.generation.to_string()))
            .collect()
    };
    composition::COMPOSITIONS
        .iter()
        .map(|c| CompositionInfo {
            key: c.key.to_string(),
            crate_name: c.crate_name.to_string(),
            generations: c
                .generations
                .iter()
                .map(|g| GenerationInfo {
                    module: g.module.to_string(),
                    spec_version: g.spec_version.to_string(),
                    units: g
                        .units
                        .iter()
                        .map(|u| (u.file.to_string(), u.in_prelude))
                        .collect(),
                    current: g.current,
                    model_deps: deps(g.model_deps),
                    prelude_deps: deps(g.prelude_deps),
                })
                .collect(),
            citation: c.citation.to_string(),
            reason: c.reason.to_string(),
        })
        .collect()
}

// ── completeness ────────────────────────────────────────────────────────────

/// Per-**generation** class-count breakdown for the completeness invariant.
///
/// One row per vendored BMM file composing the crate. Counting per generation
/// (not over a merged class map) is load-bearing: a merged map hides a
/// cross-generation name collision entirely — every name is still present, so a
/// name-level count over the merge passes while one generation's shape and
/// attributes have been discarded.
#[derive(Debug, Clone)]
pub struct Completeness {
    /// The composition key.
    pub key: String,
    /// The vendored BMM file this generation loads.
    pub file: String,
    /// Total classes this generation declares.
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

/// Compute the completeness breakdown of every BMM generation composing one
/// crate, in declaration order.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn completeness(key: &str) -> Result<Vec<Completeness>, Error> {
    let c = compose(key)?;
    let mut out = Vec::new();
    for g in &c.generations {
        for u in &g.units {
            let used = u.model.used_as_type();
            let mut planned = 0;
            let mut skipped_mapped = 0;
            let mut skipped_abstract_unused = 0;
            let mut silently_dropped = Vec::new();
            for (name, class) in &u.schema.classes {
                match decide(&u.model, class, &used) {
                    Emission::Skip => {
                        if Model::is_mapped(name) {
                            skipped_mapped += 1;
                        } else if class.is_abstract
                            && u.model.enum_variants(name).is_empty()
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
            out.push(Completeness {
                key: key.to_string(),
                file: u.spec.file.to_string(),
                total: u.schema.classes.len(),
                planned,
                skipped_mapped,
                skipped_abstract_unused,
                silently_dropped,
            });
        }
    }
    Ok(out)
}

/// One BMM-declared attribute that reaches no emitted Rust field — the
/// attribute-level half of the completeness invariant.
#[derive(Debug, Clone)]
pub struct AttributeGap {
    /// The vendored BMM file declaring it.
    pub file: String,
    /// The declaring class.
    pub class: String,
    /// The declared attribute (BMM property name).
    pub attribute: String,
    /// Why it is a gap (which emitted type was expected to carry it).
    pub detail: String,
}

/// Every BMM-declared attribute of a crate's generations that reaches no
/// emitted struct field.
///
/// A class emitted as a struct (or a polymorphic slot's `{Name}Data`) must carry
/// every attribute its OWN generation declares on it; a class emitted as a
/// closed subtype set carries its attributes through each concrete variant's
/// flattened struct. A designated owner/parent back-reference is deliberately
/// omitted from the struct
/// (the `back_reference` decision map) and is not a gap.
///
/// This is the check the class-NAME-level count cannot make: a merged
/// two-generation class map keeps every name while silently dropping one
/// generation's attribute set.
///
/// The `usize` beside the gap list is how many `(class, attribute)` pairs were
/// actually checked, so the invariant can assert the check is not vacuous.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn attribute_gaps(key: &str) -> Result<(Vec<AttributeGap>, usize), Error> {
    let c = compose(key)?;
    let mut gaps = Vec::new();
    let mut checked = 0_usize;
    for g in &c.generations {
        for u in &g.units {
            let used = u.model.used_as_type();
            // Emitted field names per class (flattened, back-references omitted —
            // exactly what `render_struct_def` writes).
            let fields = |class_name: &str| -> BTreeSet<String> {
                u.model.get(class_name).map_or_else(BTreeSet::new, |cls| {
                    u.model
                        .flattened_props(cls)
                        .iter()
                        .filter(|rp| overrides::back_reference(&rp.owner, &rp.prop.name).is_none())
                        .map(|rp| rp.prop.name.clone())
                        .collect()
                })
            };
            for (name, class) in &u.schema.classes {
                let carriers: Vec<String> = match decide(&u.model, class, &used) {
                    Emission::Struct | Emission::PolyEnum(_) => vec![name.clone()],
                    Emission::Enum(variants) => variants,
                    // A literal enumeration and a transparent newtype are scalars on
                    // the wire and declare no attributes of their own; a mapped or
                    // unused-abstract class emits nothing (the name-level
                    // completeness check accounts for it).
                    Emission::EnumLiterals(_) | Emission::Newtype(_) | Emission::Skip => Vec::new(),
                };
                for p in &class.properties {
                    if overrides::back_reference(name, &p.name).is_some() {
                        continue;
                    }
                    for carrier in &carriers {
                        checked += 1;
                        if !fields(carrier).contains(&p.name) {
                            gaps.push(AttributeGap {
                                file: u.spec.file.to_string(),
                                class: name.clone(),
                                attribute: p.name.clone(),
                                detail: format!("missing from the emitted `{carrier}` fields"),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok((gaps, checked))
}

/// A file path or prelude identifier claimed by more than one BMM generation of
/// the same crate — the silent-shape-pick hazard, which must never occur.
#[derive(Debug, Clone)]
pub struct GenerationConflict {
    /// The composition key.
    pub key: String,
    /// The conflicting artifact (an emitted `src/` path, or a prelude ident).
    pub what: String,
    /// The vendored BMM files that both claim it.
    pub files: Vec<String>,
}

/// Emitted-path conflicts between the BMM generations composing each crate.
///
/// Must be empty: every generation renders under its own version-named top
/// module, so two generations sharing an emitted path would mean one
/// overwrites the other (a silently picked shape). Cross-generation prelude
/// collisions no longer exist by construction — the crate prelude re-exports
/// the CURRENT generation only, and each generation carries its own in-tree
/// prelude.
///
/// # Errors
/// Returns an error if any composition's BMM files cannot be loaded.
pub fn generation_conflicts() -> Result<Vec<GenerationConflict>, Error> {
    let mut out = Vec::new();
    for comp in composition::COMPOSITIONS {
        let c = compose(comp.key)?;
        let mut paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let gens: Vec<CrateGeneration<'_>> = c
            .generations
            .iter()
            .map(|g| CrateGeneration {
                spec: g.spec,
                units: g
                    .units
                    .iter()
                    .map(|u| RenderUnit {
                        spec: u.spec,
                        model: &u.model,
                        schema: &u.schema,
                    })
                    .collect(),
                external: &g.external,
            })
            .collect();
        // Path conflicts are independent of the banner input, so an empty
        // sibling set is the right (and cheapest) view here. Attribute each
        // emitted path to the generation whose module prefixes it; the
        // crate-level artifacts (`lib.rs`, `prelude.rs`) are assembled once.
        for f in emit_composed(comp, &gens, &SiblingImpls::default()) {
            let file = c
                .generations
                .iter()
                .find(|g| f.path.starts_with(&format!("{}/", g.spec.module)))
                .map_or("(crate-level)", |g| g.spec.module);
            paths.entry(f.path).or_default().push(file.to_string());
        }
        for (what, files) in paths {
            if files.len() > 1 {
                out.push(GenerationConflict {
                    key: comp.key.to_string(),
                    what,
                    files,
                });
            }
        }
    }
    Ok(out)
}

// ── invariant classification (assertion-dialect analyzer) ────────────────────

/// One classified BMM invariant: the owning class, the invariant name, its
/// verbatim assertion expression, and the R5 bucket the assertion-dialect
/// analyzer assigns it.
#[derive(Debug, Clone)]
pub struct ClassifiedInvariant {
    /// The generation module (`v1_2`) whose schema declares it.
    pub generation: String,
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
    for g in &c.generations {
        for u in &g.units {
            for (class, def) in &u.schema.classes {
                for (name, expr) in &def.invariants {
                    let (bucket, reason) = match invariants::classify(expr) {
                        Bucket::Emitted => ("emitted", String::new()),
                        Bucket::RuntimeHookMissing(r) => ("runtime-hook-missing", r.to_string()),
                        Bucket::Complex(r) => ("complex", r.to_string()),
                    };
                    out.push(ClassifiedInvariant {
                        generation: g.spec.module.to_string(),
                        class: class.clone(),
                        name: name.clone(),
                        expr: expr.clone(),
                        bucket,
                        reason,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| (&a.generation, &a.class, &a.name).cmp(&(&b.generation, &b.class, &b.name)));
    Ok(out)
}

// ── constructibility ────────────────────────────────────────────────────────

/// The non-constructible concrete classes of every BMM generation composing a
/// crate (an unbroken mandatory single-valued construction cycle). The invariant
/// requires empty.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn constructibility_offenders(key: &str) -> Result<Vec<String>, Error> {
    let c = compose(key)?;
    let mut out: Vec<String> = c
        .generations
        .iter()
        .flat_map(|g| &g.units)
        .flat_map(|u| u.model.constructibility_violations(&u.schema))
        .collect();
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

// ── determinism ─────────────────────────────────────────────────────────────

/// The stage-3 plan for a crate, as comparable data.
///
/// The decided Rust shape (`Struct` / `Enum` / `PolyEnum` / `EnumLiterals` /
/// `Newtype` / `Skip`) of every class of every BMM generation composing the
/// crate, keyed `"<bmm file>::<CLASS>"`.
/// The key carries the generation because a class name may be declared by more
/// than one, with a different decided shape in each.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn plan_shapes(key: &str) -> Result<BTreeMap<String, String>, Error> {
    let c = compose(key)?;
    let mut out = BTreeMap::new();
    for g in &c.generations {
        for u in &g.units {
            let used = u.model.used_as_type();
            for (name, class) in &u.schema.classes {
                out.insert(
                    format!("{}::{name}", u.spec.file),
                    shape_name(&decide(&u.model, class, &used)).to_string(),
                );
            }
        }
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
/// `"<crate>/<path>"` (pre-rustfmt bodies).
///
/// Reproduces `cli::cmd_emit`'s emission half without WRITING to the
/// filesystem (it reads the same inputs `cmd_emit` does, the vendored BMM
/// plus each crate's hand-written `*_impl.rs` siblings), so a double call
/// proves the emitter is byte-deterministic.
///
/// # Errors
/// Returns an error if any crate's BMM files cannot be loaded.
pub fn render_all_to_memory() -> Result<BTreeMap<String, String>, Error> {
    let mut out = BTreeMap::new();
    for comp in composition::COMPOSITIONS {
        let c = compose(comp.key)?;
        let impls = sibling_impls(comp.crate_name);
        let augmented: Vec<Vec<BmmSchema>> = c
            .generations
            .iter()
            .map(|g| {
                g.units
                    .iter()
                    .map(|u| {
                        let reemit = cross_schema_reemit(&u.model, &u.schema);
                        let deps: Vec<&BmmSchema> = g.dep_schemas.iter().collect();
                        augment_with_reemit(&u.schema, &u.model, &reemit, &deps)
                    })
                    .collect()
            })
            .collect();
        let gens: Vec<CrateGeneration<'_>> = c
            .generations
            .iter()
            .zip(&augmented)
            .map(|(g, augs)| CrateGeneration {
                spec: g.spec,
                units: g
                    .units
                    .iter()
                    .zip(augs)
                    .map(|(u, aug)| RenderUnit {
                        spec: u.spec,
                        model: &u.model,
                        schema: aug,
                    })
                    .collect(),
                external: &g.external,
            })
            .collect();
        let mut files = emit_composed(comp, &gens, &impls);
        if comp.key == "rm" {
            // Mirror of `cli::cmd_emit`: every RM generation carries its own
            // attribute model + invariant cores.
            for (g, augs) in c.generations.iter().zip(&augmented) {
                let module = g.spec.module;
                let unit = g.unit()?;
                let aug = augs
                    .first()
                    .ok_or("an RM generation carries no specification unit")?;
                inject_rm_model(
                    &mut files,
                    prefix_gen_files(emit_rm_model::emit_files(&unit.model), module),
                    module,
                );
                files.extend(prefix_gen_files(
                    emit_validate::emit_files(&unit.model, aug),
                    module,
                ));
            }
        }
        for f in files {
            out.insert(format!("{}/{}", comp.crate_name, f.path), f.body);
        }
    }
    Ok(out)
}

/// The hand-written `*_impl.rs` siblings of a generated crate — the same
/// emitter input `cli::sibling_impls` reads, so the in-memory render matches
/// the written tree byte for byte.
fn sibling_impls(crate_name: &str) -> SiblingImpls {
    SiblingImpls::scan(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates")
            .join(crate_name)
            .join("src"),
    )
}

/// Mirror of `cli::prefix_gen_files` (kept private there): prefix generated
/// file paths with the generation module directory.
fn prefix_gen_files(files: Vec<GenFile>, module: &str) -> Vec<GenFile> {
    files
        .into_iter()
        .map(|f| GenFile {
            path: format!("{module}/{}", f.path),
            body: f.body,
        })
        .collect()
}

/// Mirror of `cli::inject_rm_model` (kept private there): append the RM-model
/// files (already generation-prefixed) and declare `pub mod model;` in the
/// generation `mod.rs`.
fn inject_rm_model(files: &mut Vec<GenFile>, mut model_files: Vec<GenFile>, module: &str) {
    let gen_mod = format!("{module}/mod.rs");
    for f in files.iter_mut() {
        if f.path == gen_mod && !f.body.contains("pub mod model;") {
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

/// The model delta between two generations of one crate, in BOTH directions.
#[derive(Debug, Clone)]
pub struct GenerationDelta {
    /// Class names the newer generation declares and the older does not.
    pub classes_added: Vec<String>,
    /// `CLASS.attribute` pairs the newer generation declares on classes both
    /// generations share, absent from the older.
    pub attributes_added: Vec<String>,
    /// Class names the older generation declares and the newer does not.
    pub classes_removed: Vec<String>,
    /// `CLASS.attribute` pairs the older generation declares on classes both
    /// generations share, absent from the newer.
    pub attributes_removed: Vec<String>,
}

/// Compute the model delta from generation `older` to `newer` of crate
/// `key` — the acceptance-boundary ledger's input (#1943; the REMOVED
/// direction #1961).
///
/// Multi-unit generations fold their units' class maps last-wins before
/// comparison (the crate-level naming view).
///
/// # Errors
/// Returns an error if the composition or either generation fails to load.
pub fn generation_attribute_delta(
    key: &str,
    older: &str,
    newer: &str,
) -> Result<GenerationDelta, Error> {
    let c = compose(key)?;
    let classes = |module: &str| -> Result<BTreeMap<String, BTreeSet<String>>, Error> {
        let g = find_generation(&c, module)?;
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for u in &g.units {
            for (name, class) in &u.schema.classes {
                out.insert(
                    name.clone(),
                    class.properties.iter().map(|p| p.name.clone()).collect(),
                );
            }
        }
        Ok(out)
    };
    let old_map = classes(older)?;
    let new_map = classes(newer)?;
    let classes_added: Vec<String> = new_map
        .keys()
        .filter(|k| !old_map.contains_key(*k))
        .cloned()
        .collect();
    let classes_removed: Vec<String> = old_map
        .keys()
        .filter(|k| !new_map.contains_key(*k))
        .cloned()
        .collect();
    let mut attributes_added = Vec::new();
    let mut attributes_removed = Vec::new();
    for (class, new_attrs) in &new_map {
        if let Some(old_attrs) = old_map.get(class) {
            for a in new_attrs.difference(old_attrs) {
                attributes_added.push(format!("{class}.{a}"));
            }
            for a in old_attrs.difference(new_attrs) {
                attributes_removed.push(format!("{class}.{a}"));
            }
        }
    }
    Ok(GenerationDelta {
        classes_added,
        attributes_added,
        classes_removed,
        attributes_removed,
    })
}

/// Find one generation of a composed crate by its module name.
fn find_generation<'a>(
    c: &'a composition::Composed,
    module: &str,
) -> Result<&'a composition::ComposedGeneration, Error> {
    c.generations
        .iter()
        .find(|g| g.spec.module == module)
        .ok_or_else(|| format!("composition {:?} has no generation {module:?}", c.comp.key).into())
}

/// The AM 2.4 downstream re-emission closure (the upstream classes whose Rust
/// form widens downstream) with the source vs augmented package paths of each.
///
/// # Errors
/// Returns an error if the AM/BASE/LANG BMM files cannot be loaded.
pub fn v2_4_reemit_mirrors() -> Result<Vec<Mirror>, Error> {
    let am = compose("am")?;
    let v2_4 = find_generation(&am, "v2_4")?;
    let unit = v2_4.unit()?;
    let reemit = cross_schema_reemit(&unit.model, &unit.schema);
    let dep_refs: Vec<&BmmSchema> = v2_4.dep_schemas.iter().collect();
    let aug = augment_with_reemit(&unit.schema, &unit.model, &reemit, &dep_refs);
    let aug_paths = class_paths(&aug);

    // Source package path per class, first-wins across the dependency schemas
    // (BASE before LANG — the same order `augment_with_reemit` grafts).
    let mut source_paths: BTreeMap<String, String> = BTreeMap::new();
    for dep in &v2_4.dep_schemas {
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
pub fn v2_4_reemit_closure() -> Result<BTreeSet<String>, Error> {
    reemit_closure("am", "v2_4")
}

/// One generation's raw downstream re-emission closure — the upstream classes
/// `analyze::cross_schema_reemit` (crate-private) reports for generation
/// `module` of crate `key`.
///
/// Queryable per generation (not just AM 2.4) so the "which generations have a
/// non-empty closure, and does the emitter act on it" question is answered from
/// the analysis itself rather than from a citation that can go stale.
///
/// # Errors
/// Returns an error if the composition's BMM files cannot be loaded or
/// `module` names no generation of `key`.
pub fn reemit_closure(key: &str, module: &str) -> Result<BTreeSet<String>, Error> {
    let c = compose(key)?;
    let g = find_generation(&c, module)?;
    Ok(g.units
        .iter()
        .flat_map(|u| cross_schema_reemit(&u.model, &u.schema))
        .collect())
}

/// The set of type files (`"<crate>/<path>"`) emitted for one crate — the
/// crate `key`'s rendered output alone.
///
/// Used to prove upstream (LANG) output is untouched by downstream (AM)
/// analysis.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn rendered_files(key: &str) -> Result<Vec<String>, Error> {
    let c = compose(key)?;
    let gens: Vec<CrateGeneration<'_>> = c
        .generations
        .iter()
        .map(|g| CrateGeneration {
            spec: g.spec,
            units: g
                .units
                .iter()
                .map(|u| RenderUnit {
                    spec: u.spec,
                    model: &u.model,
                    schema: &u.schema,
                })
                .collect(),
            external: &g.external,
        })
        .collect();
    let files = emit_composed(c.comp, &gens, &SiblingImpls::default());
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
            map: "untyped_field",
            check_existence: true,
            entries: overrides::UNTYPED_FIELDS
                .iter()
                .map(|e| DeclEntry {
                    key: format!("{}.{}", e.class, e.field),
                    decision: "adjudicated free-form JSON (serde_json::Value)".to_string(),
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
            // The keys are OAS component-schema names, not BMM classes, so
            // existence is checked against the vendored bundles by
            // `oas_monomorphizations`, not by the BMM class/field scan.
            map: "oas_monomorphization",
            check_existence: false,
            entries: overrides::OAS_MONOMORPHIZATIONS
                .iter()
                .map(|e| DeclEntry {
                    key: e.schema.to_string(),
                    decision: format!("{} → {}", e.title, e.rust_type),
                    citation: e.citation.to_string(),
                    reason: e.reason.to_string(),
                })
                .collect(),
        },
        DeclMap {
            map: "unrenderable_default",
            check_existence: true,
            entries: overrides::UNRENDERABLE_DEFAULTS
                .iter()
                .map(|e| DeclEntry {
                    key: format!("{}.{}", e.owner, e.field),
                    decision: "vendored `default` facet deliberately not realized".to_string(),
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

/// One accounted assertion-dialect-**emittable** class invariant.
///
/// Records which venue the realization register
/// (`plan::overrides::INVARIANT_REALIZATIONS`) says realizes it, flattened for
/// the accounting invariant.
#[derive(Debug, Clone)]
pub struct AccountedInvariant {
    /// The owning BMM class name.
    pub class: String,
    /// The BMM invariant name.
    pub name: String,
    /// The venue name (`"Core"`, `"Impl"`, `"Wire"`, `"Excluded"`,
    /// `"Unrealized"`), or `"UNACCOUNTED"` when the register has no row —
    /// an emittable invariant no venue claims.
    pub venue: &'static str,
    /// The realizing site (core function name, or repo-relative file); empty
    /// for the non-realizing venues and for an unaccounted invariant.
    pub site: String,
    /// The class's vendored spec file, repo-relative; empty when unaccounted.
    pub citation: String,
    /// The one-line reason; empty when unaccounted.
    pub reason: String,
}

/// Account a crate's own emittable class invariants against the realization
/// register: one row per invariant the classifier buckets `emitted`, sorted by
/// `(class, name)`.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn accounted_emitted_invariants(key: &str) -> Result<Vec<AccountedInvariant>, Error> {
    let c = compose(key)?;
    let triples: Vec<(String, String, String)> = c
        .generations
        .iter()
        .flat_map(|g| &g.units)
        .flat_map(|u| &u.schema.classes)
        .flat_map(|(class, def)| {
            def.invariants
                .iter()
                .map(move |(name, expr)| (class.clone(), name.clone(), expr.clone()))
        })
        .collect();
    Ok(account(
        triples
            .iter()
            .map(|(c, n, e)| (c.as_str(), n.as_str(), e.as_str())),
    ))
}

/// Accounts an explicit `(class, invariant, assertion-expression)` set.
///
/// The set is checked against the realization register — the seam the accounting
/// invariant's negative case uses to prove an unrealized emit is caught (a
/// synthetic emittable invariant has no register row, so it accounts as
/// `"UNACCOUNTED"`).
#[must_use]
pub fn account_invariants(triples: &[(&str, &str, &str)]) -> Vec<AccountedInvariant> {
    account(triples.iter().copied())
}

/// Shared body of [`accounted_emitted_invariants`] and [`account_invariants`].
fn account<'a>(
    triples: impl Iterator<Item = (&'a str, &'a str, &'a str)>,
) -> Vec<AccountedInvariant> {
    overrides::account_emitted(triples)
        .into_iter()
        .map(|a| {
            let venue = a.realization.map_or("UNACCOUNTED", |r| match r.venue {
                overrides::InvariantVenue::Core => "Core",
                overrides::InvariantVenue::Impl => "Impl",
                overrides::InvariantVenue::Wire => "Wire",
                overrides::InvariantVenue::App => "App",
                overrides::InvariantVenue::Excluded => "Excluded",
                overrides::InvariantVenue::Unrealized => "Unrealized",
            });
            AccountedInvariant {
                class: a.class,
                name: a.name,
                venue,
                site: a.realization.map(|r| r.site.to_owned()).unwrap_or_default(),
                citation: a.realization.map_or_else(String::new, |r| {
                    format!("{}/{}", overrides::RM_CLASS_DOCS, r.spec_file)
                }),
                reason: a
                    .realization
                    .map(|r| r.reason.to_owned())
                    .unwrap_or_default(),
            }
        })
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

/// The adjudicated free-form (`serde_json::Value`) fields, as
/// `(class, field, citation)` triples.
#[must_use]
pub fn untyped_fields() -> Vec<(String, String, String)> {
    overrides::UNTYPED_FIELDS
        .iter()
        .map(|e| {
            (
                e.class.to_string(),
                e.field.to_string(),
                e.citation.to_string(),
            )
        })
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
    // Newest generation first (mirroring the retired merged-view semantics
    // where the last generation won a colliding name).
    Ok(c.generations
        .iter()
        .rev()
        .flat_map(|g| g.units.iter().rev())
        .find(|u| u.model.get(class).is_some())
        .map(|u| {
            let mut variants = u.model.enum_variants(class);
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
        if compose(key)?
            .generations
            .iter()
            .flat_map(|g| &g.units)
            .any(|u| u.model.get(class).is_some())
        {
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
        for u in compose(key)?.generations.iter().flat_map(|g| &g.units) {
            if let Some(cls) = u.model.get(class)
                && u.model
                    .flattened_props(cls)
                    .iter()
                    .any(|rp| rp.prop.name == field)
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// The attribute names composition `key`'s OWN schema declares on `class`
/// (declared, not inherited), or `None` if that schema declares no such class.
///
/// Used to pin the RM/BASE twin classes: five class names are declared by both
/// components with materially different member sets, and both generations are
/// emitted deliberately (see `plan::composition`'s module note).
///
/// # Errors
/// Returns an error if the composition's BMM files cannot be loaded.
pub fn declared_attributes(key: &str, class: &str) -> Result<Option<BTreeSet<String>>, Error> {
    let c = compose(key)?;
    // Newest generation first (the retired merged-view semantics).
    Ok(c.generations
        .iter()
        .rev()
        .flat_map(|g| g.units.iter().rev())
        .find_map(|u| u.schema.classes.get(class))
        .map(|cls| cls.properties.iter().map(|p| p.name.clone()).collect()))
}

/// One vendored `default` facet, with the emitter's disposition of it.
#[derive(Debug, Clone)]
pub struct VendoredDefault {
    /// Composition key the facet was read from.
    pub key: String,
    /// The DECLARING class.
    pub owner: String,
    /// The property carrying the facet.
    pub field: String,
    /// The facet text, verbatim from the schema.
    pub facet: String,
    /// The literal Rust expression the emitter derives, or `None` when the
    /// facet is not renderable in the property's declared type.
    pub rendered: Option<String>,
}

/// Every `default` facet the vendored schemas carry, across every composition
/// generation, with what the emitter makes of it.
///
/// This is the reconciliation surface for the hand-written `field_default`
/// table: the vendored facet is the source, the table is the residue, and the
/// two must not overlap.
///
/// # Errors
/// Returns an error if a vendored BMM file cannot be loaded.
pub fn vendored_defaults() -> Result<Vec<VendoredDefault>, Error> {
    let mut out = Vec::new();
    for key in crate_keys() {
        for u in compose(key)?.generations.iter().flat_map(|g| &g.units) {
            for (owner, class) in &u.schema.classes {
                for prop in &class.properties {
                    if let Some(facet) = &prop.default {
                        out.push(VendoredDefault {
                            key: key.to_string(),
                            owner: owner.clone(),
                            field: prop.name.clone(),
                            facet: facet.clone(),
                            rendered: overrides::vendored_default(prop),
                        });
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Whether `(owner, field)` is an adjudicated un-renderable `default` facet.
#[must_use]
pub fn default_unrenderable(owner: &str, field: &str) -> bool {
    overrides::default_unrenderable(owner, field)
}

/// The hand-written `field_default` residue as `(owner, field)` pairs.
#[must_use]
pub fn hand_written_defaults() -> Vec<(String, String)> {
    overrides::FIELD_DEFAULTS
        .iter()
        .map(|d| (d.owner.to_string(), d.field.to_string()))
        .collect()
}

/// One adjudicated OAS monomorphization, paired with what the VENDORED bundles
/// actually declare for that schema key.
#[derive(Debug, Clone)]
pub struct OasMonomorphizationCheck {
    /// The OAS `components/schemas` key.
    pub schema: String,
    /// The `title` the decision map claims the schema declares.
    pub declared_title: String,
    /// The generated Rust type the map resolves it to.
    pub rust_type: String,
    /// The `title` values the vendored bundles really declare for that key
    /// (empty when no bundle declares the key at all).
    pub vendored_titles: BTreeSet<String>,
}

/// Every `OAS_MONOMORPHIZATIONS` entry checked against the vendored bundles.
///
/// The mapping is only legitimate because each ITS-REST schema declares its real
/// spec name in `title`, so the entry must still match the vendored text.
///
/// # Errors
/// Returns an error if a vendored OAS bundle cannot be read or parsed.
pub fn oas_monomorphizations() -> Result<Vec<OasMonomorphizationCheck>, Error> {
    let oas_dir = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/openehr-its/vendor/rest-oas"
    ))
    .to_path_buf();
    let mut bundles = Vec::new();
    for entry in std::fs::read_dir(&oas_dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|x| x == "yaml") {
            bundles.push(oas::Oas::parse_file(&path)?);
        }
    }
    Ok(overrides::OAS_MONOMORPHIZATIONS
        .iter()
        .map(|m| OasMonomorphizationCheck {
            schema: m.schema.to_string(),
            declared_title: m.title.to_string(),
            rust_type: m.rust_type.to_string(),
            vendored_titles: bundles
                .iter()
                .flat_map(oas::Oas::schemas)
                .filter(|(name, _)| name == m.schema)
                .filter_map(|(_, s)| {
                    s.get("title")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .collect(),
        })
        .collect())
}

/// The schema names the shared-module fallback document carries, paired with
/// the `allOf` base names the hoisted schemas reach.
///
/// The second set must be a subset of the first, or `emit_common` flattens
/// `allOf` compositions against a document that cannot resolve their bases.
///
/// # Errors
/// Returns an error if a vendored OAS bundle or a BMM file cannot be loaded.
pub fn merged_fallback_schema_names() -> Result<(BTreeSet<String>, BTreeSet<String>), Error> {
    let oas_dir = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/openehr-its/vendor/rest-oas"
    ))
    .to_path_buf();
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(&oas_dir)
        .map_err(|e| e.to_string())?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    paths.retain(|p| p.extension().is_some_and(|x| x == "yaml"));
    paths.sort();
    let mut bundles: Vec<(&str, oas::Oas)> = Vec::new();
    for path in &paths {
        bundles.push(("", oas::Oas::parse_file(path)?));
    }

    let base = compose("base")?;
    let rm = compose("rm")?;
    let type_paths = |c: &composition::Composed| -> Result<BTreeMap<String, String>, Error> {
        let krate = c.comp.crate_name.replace('-', "_");
        let g = c
            .generations
            .iter()
            .find(|g| g.spec.current)
            .ok_or("composition has no current generation")?;
        let mut out = BTreeMap::new();
        for u in &g.units {
            for spec in crate::analyze::emittable_specs(&u.model, &u.schema) {
                let ident = naming::type_name(&spec);
                let path = format!(
                    "{krate}::{}::{}::{ident}",
                    g.spec.module,
                    crate::render::emit::type_module_path(&u.schema, &spec)
                );
                out.insert(ident, path);
            }
        }
        Ok(out)
    };
    let names = emit_rest::RmNames {
        base: type_paths(&base)?,
        rm: type_paths(&rm)?,
    };
    let hoisted = emit_rest::hoist_set(&bundles, &names);
    let merged = oas::Oas::merged_schemas(&bundles, &hoisted);
    let carried: BTreeSet<String> = merged.schemas().iter().map(|(n, _)| n.clone()).collect();

    // The `allOf` bases the hoisted schemas reach, from the source bundles.
    let mut bases = BTreeSet::new();
    for (_, o) in &bundles {
        for (name, schema) in o.schemas() {
            if !hoisted.contains(&name) {
                continue;
            }
            if let Some(members) = schema
                .get("allOf")
                .and_then(serde_json::Value::as_array)
                .map(|m| m.iter().filter_map(oas::Oas::ref_name).collect::<Vec<_>>())
            {
                bases.extend(members);
            }
        }
    }
    Ok((carried, bases))
}

// ── model-query report ──────────────────────────────────────────────────────

/// Renders the `model-query` report over the real vendored BMM inputs.
///
/// It is the same projection the CLI subcommand prints (BMM-declared facts
/// beside the current field-shape decision), so a golden test pins the CLI's
/// actual output.
///
/// `component`/`class`/`attribute` are the optional filters; `format` is one of
/// `table`, `tsv`, `json`.
///
/// # Errors
/// Returns an error if a vendored BMM file cannot be loaded, if `format` is not
/// a valid format, or if a filter names a component/class/attribute the loaded
/// model does not have.
pub fn model_query(
    component: Option<&str>,
    class: Option<&str>,
    attribute: Option<&str>,
    format: &str,
) -> Result<String, Error> {
    model_query_view(component, class, attribute, format, false)
}

/// [`model_query()`] with the view selectable.
///
/// `flattened` reports one row per class × CARRIED attribute (inherited ones
/// included, each with its declaring class) instead of one row per class ×
/// declared attribute.
///
/// # Errors
/// Same as [`model_query()`].
pub fn model_query_view(
    component: Option<&str>,
    class: Option<&str>,
    attribute: Option<&str>,
    format: &str,
    flattened: bool,
) -> Result<String, Error> {
    model_query::render(
        &model_query::Query {
            component,
            class,
            attribute,
            flattened,
        },
        model_query::Format::parse(format)?,
    )
}

/// Hand-written generation-twin pairs of `key`'s crate that are byte-identical
/// modulo generation tokens — each MUST be a template (#1964).
///
/// Walks every non-current generation module against the current one; a
/// hand-written file (no `@generated` first line) at the same relative path
/// in both is normalized with the template substitution and compared. The
/// returned relative paths are the families the template mechanism must
/// absorb; the emitter-invariants suite asserts the list is EMPTY.
///
/// # Errors
/// Returns an error if the composition fails to load or a crate tree cannot
/// be read.
pub fn identical_hand_written_twins(key: &str) -> Result<Vec<String>, Error> {
    let comp = composition::COMPOSITIONS
        .iter()
        .find(|c| c.key == key)
        .ok_or_else(|| Error::from(format!("unknown composition key {key}")))?;
    let Some(current) = crate::render::emit_templates::current_generation(comp) else {
        return Err(Error::from(format!("no current generation for {key}")));
    };
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates")
        .join(comp.crate_name)
        .join("src");
    let current_files = hand_written_files(&src.join(current.module))?;
    let mut identical = Vec::new();
    for generation in comp.generations {
        if generation.module == current.module {
            continue;
        }
        for (rel, body) in hand_written_files(&src.join(generation.module))? {
            let Some(current_body) = current_files.get(&rel) else {
                continue;
            };
            let normalized = crate::render::emit_templates::substitute(&body, generation, current);
            if &normalized == current_body {
                identical.push(format!("{}/{rel}", generation.module));
            }
        }
    }
    identical.sort();
    Ok(identical)
}

/// Hand-written `.rs` files under `dir` (relative slash path → body), the
/// module anchors excluded.
fn hand_written_files(dir: &std::path::Path) -> Result<BTreeMap<String, String>, Error> {
    let mut out = BTreeMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).map_err(|e| Error::from(e.to_string()))? {
            let path = entry.map_err(|e| Error::from(e.to_string()))?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if path.extension().and_then(|e| e.to_str()) != Some("rs")
                || matches!(name, "mod.rs" | "prelude.rs")
            {
                continue;
            }
            let body = std::fs::read_to_string(&path).map_err(|e| Error::from(e.to_string()))?;
            if body
                .lines()
                .next()
                .is_some_and(|l| l.contains("@generated"))
            {
                continue;
            }
            let rel = path
                .strip_prefix(dir)
                .map_err(|e| Error::from(e.to_string()))?
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(rel, body);
        }
    }
    Ok(out)
}

/// BMM-declared functions whose realization DIFFERS between two generations of
/// one crate, as `<CLASS>.<function>: realized in <gen>, missing in <gen>`
/// (#2029).
///
/// The staleness this catches: the generation-twin templates give every
/// generation one source, so a class both generations declare must realize the
/// same accessors in both. A divergence means a per-generation override drifted,
/// or a re-vendor renamed a function and only one generation followed.
///
/// A function only the newer BMM declares is NOT a divergence when the older
/// generation realizes it anyway — a superset is permitted by the direction
/// contract (`docs/VERSIONS.md` §Spec version policy). The reverse, realized in
/// the older generation but not the newer, IS reported.
///
/// # Errors
/// Returns an error if the composition fails to load or a crate tree cannot be
/// read.
pub fn generation_function_divergence(key: &str) -> Result<Vec<String>, Error> {
    let comp = compose(key)?;
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates")
        .join(comp.comp.crate_name)
        .join("src");
    let mut realized: Vec<(&str, BTreeMap<String, BTreeSet<String>>)> = Vec::new();
    for generation in &comp.generations {
        let bodies = rust_bodies_by_stem(&src.join(generation.spec.module))?;
        let mut per_class: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for unit in &generation.units {
            for (name, class) in &unit.schema.classes {
                let stem = name.to_lowercase();
                let Some(sibling) = bodies.get(&format!("{stem}_impl")) else {
                    continue;
                };
                let own = bodies.get(&stem).map(String::as_str).unwrap_or_default();
                let found = class
                    .functions
                    .iter()
                    .filter(|f| {
                        let item = format!("fn {f}(");
                        sibling.contains(&item) || own.contains(&item)
                    })
                    .cloned()
                    .collect();
                per_class.insert(name.clone(), found);
            }
        }
        realized.push((generation.spec.module, per_class));
    }
    let mut divergent = Vec::new();
    for (i, (module, classes)) in realized.iter().enumerate() {
        for (other_module, other_classes) in realized.iter().skip(i + 1) {
            for (class, functions) in classes {
                let Some(other) = other_classes.get(class) else {
                    continue;
                };
                for f in functions.difference(other) {
                    divergent.push(format!(
                        "{class}.{f}: realized in {module}, missing in {other_module}"
                    ));
                }
            }
        }
    }
    divergent.sort();
    divergent.dedup();
    Ok(divergent)
}

/// Whether `body` applies items to `rust_type` — an `impl` block on it, or the
/// type passed to a macro that writes one.
///
/// Substring matching is not enough in either direction: `ImportedVersion`
/// contains `Version`, and a doc comment naming a type says nothing about what
/// the file implements. So the occurrence must be delimited, and it must sit
/// where an impl target sits.
fn targets_type(body: &str, rust_type: &str) -> bool {
    body.lines().any(|line| {
        let line = line.trim();
        // `ordered_limit!(\n    DvCount,` — the type as a bare macro argument.
        if line.strip_suffix(',').is_some_and(|arg| arg == rust_type) {
            return true;
        }
        // `impl DvCount {`, `impl<T> ImportedVersion<T> {`, `impl Validate for DvCount {`
        line.starts_with("impl") && delimited_mention(line, rust_type)
    })
}

/// Whether `haystack` contains `needle` bounded by non-identifier characters,
/// so `Version` does not match inside `ImportedVersion`.
fn delimited_mention(haystack: &str, needle: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    // Split on the needle rather than slicing by byte index: a `&str` index can
    // land inside a multi-byte character, and `string_slice` is denied for
    // exactly that reason. Each split boundary gives the neighbouring
    // characters directly.
    let mut rest = haystack;
    while let Some(at) = rest.find(needle) {
        let (before, from_match) = rest.split_at(at);
        let Some(after) = from_match.strip_prefix(needle) else {
            return false;
        };
        if before.chars().next_back().is_none_or(|c| !is_ident(c))
            && after.chars().next().is_none_or(|c| !is_ident(c))
        {
            return true;
        }
        rest = after;
    }
    false
}

/// BMM-declared functions of `key`'s crate that no Rust method realizes, as
/// `<generation>/<CLASS>.<function>` (#2029).
///
/// The BMM declares functions by name and result type only, so their bodies are
/// hand-written in a `*_impl.rs` sibling. This projection reports, per
/// generation, every declared function of a class that HAS such a sibling but
/// whose name appears as no `fn` item in that class's module files — the
/// staleness a re-vendor introduces when upstream renames or removes an
/// accessor.
///
/// Classes with no behaviour sibling are out of scope: a plain record realizes
/// its functions as struct fields, and the emitter has no body to write.
///
/// # Errors
/// Returns an error if the composition fails to load or a crate tree cannot be
/// read.
pub fn unrealized_bmm_functions(key: &str) -> Result<Vec<String>, Error> {
    let comp = compose(key)?;
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates")
        .join(comp.comp.crate_name)
        .join("src");
    let mut missing = Vec::new();
    for generation in &comp.generations {
        let bodies = rust_bodies_by_stem(&src.join(generation.spec.module))?;
        for unit in &generation.units {
            for (name, class) in &unit.schema.classes {
                if class.functions.is_empty() {
                    continue;
                }
                let stem = name.to_lowercase();
                let Some(sibling) = bodies.get(&format!("{stem}_impl")) else {
                    continue;
                };
                let own = bodies.get(&stem).map(String::as_str).unwrap_or_default();
                let rust_type = naming::type_name(name);
                for function in &class.functions {
                    let item = format!("fn {function}(");
                    if sibling.contains(&item) || own.contains(&item) {
                        continue;
                    }
                    // A method can be realized by a MACRO applied elsewhere in
                    // the generation: `ordered_limit!` in `dv_ordered_impl`
                    // gives every DV_ORDERED descendant its `less_than` and
                    // `is_strictly_comparable_to`. Reading only the class's own
                    // two files reported those as missing, and burning them
                    // down produces DUPLICATE DEFINITIONS.
                    //
                    // The witness must name the type as an impl TARGET, not
                    // merely mention it. Accepting a mention credited
                    // `VERSION.data` to `imported_version_impl.rs`, whose
                    // `pub fn data(` belongs to `ImportedVersion<T>` and whose
                    // only "Version" is inside that longer name — a false
                    // NEGATIVE, which silently drops a real gap and is worse
                    // than the false positives this replaced.
                    if bodies
                        .values()
                        .any(|body| body.contains(&item) && targets_type(body, &rust_type))
                    {
                        continue;
                    }
                    missing.push(format!("{}/{name}.{function}", generation.spec.module));
                }
            }
        }
    }
    missing.sort();
    missing.dedup();
    Ok(missing)
}

/// Every `.rs` file under `dir` as file-stem → body, generated and hand-written
/// alike (the realization check reads both: an accessor may be emitted on the
/// struct or written in its behaviour sibling).
fn rust_bodies_by_stem(dir: &std::path::Path) -> Result<BTreeMap<String, String>, Error> {
    let mut out = BTreeMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).map_err(|e| Error::from(e.to_string()))? {
            let path = entry.map_err(|e| Error::from(e.to_string()))?.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let body = std::fs::read_to_string(&path).map_err(|e| Error::from(e.to_string()))?;
            out.insert(stem.to_owned(), body);
        }
    }
    Ok(out)
}
