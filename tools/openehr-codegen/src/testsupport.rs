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
use crate::render::emit::{
    CrateGeneration, GenFile, crate_generations, emit_crate, emit_generations, emit_multi_crate,
};
use crate::render::{emit_rm_model, emit_validate, naming};
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
    let mut out = Vec::with_capacity(c.generations.len());
    for g in &c.generations {
        let used = g.model.used_as_type();
        let mut planned = 0;
        let mut skipped_mapped = 0;
        let mut skipped_abstract_unused = 0;
        let mut silently_dropped = Vec::new();
        for (name, class) in &g.schema.classes {
            match decide(&g.model, class, &used) {
                Emission::Skip => {
                    if Model::is_mapped(name) {
                        skipped_mapped += 1;
                    } else if class.is_abstract
                        && g.model.enum_variants(name).is_empty()
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
            file: g.file.to_string(),
            total: g.schema.classes.len(),
            planned,
            skipped_mapped,
            skipped_abstract_unused,
            silently_dropped,
        });
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
        let used = g.model.used_as_type();
        // Emitted field names per class (flattened, back-references omitted —
        // exactly what `render_struct_def` writes).
        let fields = |class_name: &str| -> BTreeSet<String> {
            g.model.get(class_name).map_or_else(BTreeSet::new, |cls| {
                g.model
                    .flattened_props(cls)
                    .iter()
                    .filter(|rp| overrides::back_reference(&rp.owner, &rp.prop.name).is_none())
                    .map(|rp| rp.prop.name.clone())
                    .collect()
            })
        };
        for (name, class) in &g.schema.classes {
            let carriers: Vec<String> = match decide(&g.model, class, &used) {
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
                            file: g.file.to_string(),
                            class: name.clone(),
                            attribute: p.name.clone(),
                            detail: format!("missing from the emitted `{carrier}` fields"),
                        });
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

/// Emitted-path and prelude-identifier conflicts between the BMM generations
/// composing each crate. Both must be empty: two generations sharing an emitted
/// path means one overwrites the other (a silently picked shape), and two
/// generations exporting one prelude name means the crate's one-type-per-name
/// contract is broken.
///
/// # Errors
/// Returns an error if any composition's BMM files cannot be loaded.
pub fn generation_conflicts() -> Result<Vec<GenerationConflict>, Error> {
    let mut out = Vec::new();
    for key in crate_keys() {
        let c = compose(key)?;
        let mut paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut idents: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for g in &c.generations {
            let gen_view = [CrateGeneration {
                model: &g.model,
                schema: &g.schema,
                prelude_owned: Some(&g.owned),
            }];
            for f in emit_generations(&gen_view, &c.external, c.doc) {
                paths.entry(f.path).or_default().push(g.file.to_string());
            }
            for spec in &g.owned {
                idents
                    .entry(naming::type_name(spec))
                    .or_default()
                    .push(g.file.to_string());
            }
        }
        // `prelude.rs` and `lib.rs` are crate-level artifacts assembled ONCE
        // from all generations, so a per-generation render naming them twice is
        // expected; every spec type file must be claimed exactly once.
        for (what, files) in paths {
            if files.len() > 1 && !matches!(what.as_str(), "prelude.rs" | "lib.rs") {
                out.push(GenerationConflict {
                    key: key.to_string(),
                    what,
                    files,
                });
            }
        }
        for (what, files) in idents {
            if files.len() > 1 {
                out.push(GenerationConflict {
                    key: key.to_string(),
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
    for (class, def) in c.generations.iter().flat_map(|g| &g.schema.classes) {
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
        .flat_map(|g| g.model.constructibility_violations(&g.schema))
        .collect();
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

// ── determinism ─────────────────────────────────────────────────────────────

/// The decided Rust shape (`Struct` / `Enum` / `PolyEnum` / `EnumLiterals` /
/// `Newtype` / `Skip`) of every class of every BMM generation composing a
/// crate, keyed `"<bmm file>::<CLASS>"` — the stage-3 plan, as comparable data.
/// The key carries the generation because a class name may be declared by more
/// than one, with a different decided shape in each.
///
/// # Errors
/// Returns an error if the crate's BMM files cannot be loaded.
pub fn plan_shapes(key: &str) -> Result<BTreeMap<String, String>, Error> {
    let c = compose(key)?;
    let mut out = BTreeMap::new();
    for g in &c.generations {
        let used = g.model.used_as_type();
        for (name, class) in &g.schema.classes {
            out.insert(
                format!("{}::{name}", g.file),
                shape_name(&decide(&g.model, class, &used)).to_string(),
            );
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
        &emit_generations(&crate_generations(&lang), &lang.external, lang.doc),
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
    let files = emit_generations(&crate_generations(&c), &c.external, c.doc);
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
        if compose(key)?
            .generations
            .iter()
            .any(|g| g.model.get(class).is_some())
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
        for g in &compose(key)?.generations {
            if let Some(cls) = g.model.get(class)
                && g.model
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
