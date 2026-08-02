//! The **crate → schema-merge table**: declarative data recording which vendored
//! BMM component files compose each emitted spec crate, and which dependency
//! crates' preludes resolve its cross-crate references. Each entry carries the
//! `includes` citation that justifies the merge.
//!
//! This is the single source of truth for schema composition. [`compose`]
//! resolves an entry into the loaded model + per-generation schemas + the
//! `External` prelude index the render stage consumes, so `cli.rs` never
//! hand-merges schemas — the membership is data, not control flow.
//!
//! **One `own` file = one BMM generation, emitted completely.** Where a crate
//! lists several `own` files they are two generations of the same meta-model
//! (LANG's stable v2.x BMM beside the v3 development line), and each is emitted
//! in full from its OWN schema at its OWN source-package path — never merged
//! into one class map, because a merge silently picks one shape per colliding
//! name and discards the other's attributes. The merged
//! [`BmmSchema::dependency_view`] is built only as the crate's *naming* view
//! (one type per Rust name for the prelude and for downstream crates).

use crate::analyze::{External, Model, emittable_specs};
use crate::load::bmm::BmmSchema;
use std::collections::BTreeSet;
use std::path::Path;

/// The vendored BMM root. Paths below mirror the upstream ITS-BMM layout
/// (`components/<COMPONENT>/json/…`); the JSON forms are the codegen input for
/// our pinned versions (see `docs/VERSIONS.md`).
const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/bmm");

pub(crate) const BASE_BMM: &str = "components/BASE/json/openehr_base_1.3.0.bmm.json";
pub(crate) const RM_BMM: &str = "components/RM/json/openehr_rm_1.2.0.bmm.json";
pub(crate) const TERM_BMM: &str = "components/TERM/json/openehr_term_3.1.0.bmm.json";
pub(crate) const AM14_BMM: &str = "components/AM/json/openehr_am_1.4.0.bmm.json";
pub(crate) const AM24_BMM: &str = "components/AM/json/openehr_am_2.4.0.bmm.json";
/// LANG's **v2.x generation**: the stable BMM object model (`org.openehr.lang.bmm`),
/// its `P_BMM` persistence form (`…bmm_persistence`) and the beom expression model
/// (`…beom`, with `EXPR_*` and `STATEMENT_SET`/`ASSERTION`, which AM's rules/slots
/// reference). `LANG/docs/bmm/master01-preface.adoc` §History calls this "the
/// normative, tool-implemented version".
pub(crate) const LANG_BMM: &str = "components/LANG/json/openehr_lang_1.1.0.bmm.json";
/// LANG's **v3 generation** (`org.openehr.lang.bmm3`): the evolved `BMM_*` object
/// model with the `EL_*` expression language and the `BMM_STATEMENT*` family,
/// specified separately per `LANG/docs/bmm3/master00-amendment_record.adoc`
/// (SPECLANG-14, "Formalise the BMM v2/v3 split"). It is a second GENERATION of
/// the same meta-model, not a second part of one model: 18 class names exist in
/// both files with materially different shapes, so the two are emitted side by
/// side (the AM `am14`/`am24` precedent —
/// `BASE/docs/architecture_overview/master05-package_structure.adoc` §AM
/// Component, "Both versions are maintained side by side").
pub(crate) const LANG_BMM3: &str = "components/LANG/json/openehr_lang_1.1.0-bmm3.bmm.json";

const BASE_DOC: &str = "openEHR BASE (foundation + base types), generated from the BMM meta-model.";
const RM_DOC: &str = "openEHR RM (Reference Model), generated from the BMM meta-model.";
const LANG_DOC: &str = "openEHR LANG: the BMM object model in BOTH its extant generations, \
    generated from the BMM meta-model — the stable v2.x model (`bmm`, its `bmm_persistence` \
    P_BMM form and the `beom` expression model) and the v3 development line (`bmm3`, with the \
    `EL_*` expression and `BMM_STATEMENT*` families). Each generation is emitted completely at \
    its own source-package path; the prelude exports one type per Rust name (the v3 twin where \
    both declare a name). The generator's own BMM reader lives in openehr-codegen (tooling, \
    not spec); the hand-written ODIN reader and BEL parser live beside this generated tree.";
const AM_DOC: &str = "openEHR AM (Archetype Model): am14 (AM 1.4.0, for ADL 1.4) and am24 \
    (AM 2.4.0, for ADL 2) — both generated from BMM. Both ADL versions are in use.";
const TERM_DOC: &str = "openEHR TERM (Terminology) data model, generated from the BMM \
    meta-model. The vendored terminology XML content lives in `assets/` (data, not \
    generated); an XML→model loader is added when composition validation needs it.";

/// One emitted crate (or one version of a multi-version crate) and the BMM
/// files that compose it.
pub(crate) struct CrateComposition {
    /// Unique key (`base`, `rm`, `lang`, `am14`, `am24`, `term`).
    pub key: &'static str,
    /// Emitted crate directory (`am14` and `am24` share `openehr-am`).
    pub crate_name: &'static str,
    /// Version-module prefix for a multi-version crate (`am14`/`am24`), else
    /// `None` (single-version crate).
    pub variant: Option<&'static str>,
    /// The crate's own BMM file(s) — **one file = one generation**, each emitted
    /// completely at its own source-package path, in declaration order. Where a
    /// class name occurs in more than one, the LAST one owns the crate prelude
    /// entry (see [`Generation::owned`]).
    pub own: &'static [&'static str],
    /// Composition keys merged (in order, before `own`) into the resolution
    /// model — last-wins on name collision (BASE first).
    pub model_deps: &'static [&'static str],
    /// Composition keys whose prelude the `External` index offers, in order.
    pub prelude_deps: &'static [&'static str],
    /// Crate doc comment (emitted into `lib.rs`).
    pub doc: &'static str,
    /// The `includes` citation that justifies the merge.
    pub citation: &'static str,
    /// One-line reason.
    pub reason: &'static str,
}

/// The declarative crate → schema-merge table.
pub(crate) const COMPOSITIONS: &[CrateComposition] = &[
    CrateComposition {
        key: "base",
        crate_name: "openehr-base",
        variant: None,
        own: &[BASE_BMM],
        model_deps: &[],
        prelude_deps: &[],
        doc: BASE_DOC,
        citation: "BASE 1.3.0 BMM (openehr_base_1.3.0) — no includes; the foundation crate.",
        reason: "Foundation types; nothing below it.",
    },
    CrateComposition {
        key: "rm",
        crate_name: "openehr-rm",
        variant: None,
        own: &[RM_BMM],
        model_deps: &["base"],
        prelude_deps: &["base"],
        doc: RM_DOC,
        citation: "RM 1.2.0 BMM includes openehr_base_1.3.0 (ancestors resolve to BASE).",
        reason: "The domain model; depends on BASE.",
    },
    CrateComposition {
        key: "lang",
        crate_name: "openehr-lang",
        variant: None,
        own: &[LANG_BMM, LANG_BMM3],
        model_deps: &["base"],
        prelude_deps: &["base"],
        doc: LANG_DOC,
        citation: "LANG 1.1.0 BMM includes openehr_base_1.3.0. Two GENERATIONS of the same \
                   meta-model compose the crate and both are emitted completely, each at its \
                   own source-package path: the stable v2.x BMM + P_BMM + beom \
                   (LANG/docs/bmm/master01-preface.adoc §History — \"the normative, \
                   tool-implemented version\") under bmm/, bmm_persistence/, beom/, and the v3 \
                   development line (LANG/docs/bmm3/master01-preface.adoc §Previous Versions; \
                   master00-amendment_record.adoc SPECLANG-14 \"Formalise the BMM v2/v3 \
                   split\") under bmm3/. The AM am14/am24 precedent applies \
                   (BASE/docs/architecture_overview/master05-package_structure.adoc §AM \
                   Component: \"Both versions are maintained side by side\"). The crate \
                   prelude and downstream crates see the LAST generation for a colliding name \
                   (bmm3), the v2 twin by full module path.",
        reason: "The BMM/P_BMM object model, both extant generations; depends on BASE.",
    },
    CrateComposition {
        key: "am14",
        crate_name: "openehr-am",
        variant: Some("am14"),
        own: &[AM14_BMM],
        model_deps: &["base"],
        prelude_deps: &["base", "lang"],
        doc: AM_DOC,
        citation: "AM 1.4.0 BMM includes openehr_base_1.3.0; declares no cross-includes subtypes \
                   (empty re-emission closure).",
        reason: "ADL 1.4 archetype model; ancestors resolve to BASE.",
    },
    CrateComposition {
        key: "am24",
        crate_name: "openehr-am",
        variant: Some("am24"),
        own: &[AM24_BMM],
        model_deps: &["base", "lang"],
        prelude_deps: &["base", "lang"],
        doc: AM_DOC,
        citation: "AM 2.4.0 BMM includes openehr_lang_1.1.0 + openehr_base_1.3.0; its rules \
                   package declares subtypes of LANG's beom expression classes \
                   (EXPR_ARCHETYPE_REF ⊂ EXPR_VALUE_REF, EXPR_CONSTRAINT ⊂ EXPR_LEAF), so the \
                   downstream re-emission closure is non-empty. It reaches classes of BOTH \
                   LANG generations (v2 beom + v3 BMM_*/EL_*), so it merges LANG's dependency \
                   view — the same one-type-per-name view openehr_lang's prelude exports.",
        reason: "ADL 2 archetype model; merges BASE + the full LANG include-closure.",
    },
    CrateComposition {
        key: "term",
        crate_name: "openehr-term",
        variant: None,
        own: &[TERM_BMM],
        model_deps: &["base"],
        prelude_deps: &["base"],
        doc: TERM_DOC,
        citation: "TERM 3.1.0 BMM includes openehr_base_1.3.0 (TERMINOLOGY.date : Iso8601_date).",
        reason: "Terminology data model; depends on BASE.",
    },
];

/// Look up a composition entry by key.
///
/// # Errors
/// Returns an error if `key` names no composition entry (a codegen bug).
pub(crate) fn lookup(key: &str) -> Result<&'static CrateComposition, Box<dyn std::error::Error>> {
    COMPOSITIONS
        .iter()
        .find(|c| c.key == key)
        .ok_or_else(|| format!("unknown composition key {key:?}").into())
}

/// The Rust prelude path a composition's crate exports its types from.
pub(crate) fn prelude_path(comp: &CrateComposition) -> String {
    format!("{}::prelude", comp.crate_name.replace('-', "_"))
}

/// Load and parse one vendored BMM file (relative to [`VENDOR`]).
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub(crate) fn load_bmm(file: &str) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(Path::new(VENDOR).join(file))?;
    Ok(BmmSchema::parse_json(&src)?)
}

/// One BMM **generation** composing a crate: exactly one vendored file, emitted
/// completely at its own source-package paths, resolved against its own model.
///
/// A single-file composition has exactly one of these; LANG has two (the stable
/// v2.x BMM and the v3 development line). Never merge two generations into one
/// class map — that is precisely what discards a colliding class's attributes.
pub(crate) struct Generation {
    /// The vendored BMM file this generation loads (relative to [`VENDOR`]).
    pub file: &'static str,
    /// This generation's own schema — one vendored file, verbatim.
    pub schema: BmmSchema,
    /// The resolution model this generation's classes resolve against: the
    /// composition's dependency schemas, then this generation alone. A class of
    /// one generation therefore never resolves an ancestor, field type or
    /// subtype against the other generation's definitions.
    pub model: Model,
    /// The class names this generation contributes to the crate prelude: those
    /// it declares and no LATER generation redeclares. The prelude carries one
    /// entry per Rust type name, so a colliding name is exported from the last
    /// generation and the earlier twin is reachable by full module path only.
    pub owned: BTreeSet<String>,
}

/// A resolver's per-generation schemas plus the crate-level views, and the
/// loaded dependency schemas (kept so the caller can compute the re-emission
/// closure over the same source schemas).
pub(crate) struct Composed {
    /// The crate's BMM generations, in declaration order — what the emitter
    /// renders.
    pub generations: Vec<Generation>,
    /// The crate's **dependency view**: every generation folded last-wins (see
    /// [`BmmSchema::dependency_view`]). Naming only — never an emission input.
    pub own_schema: BmmSchema,
    /// The dependency schemas (`model_deps`, in order) that merge below it.
    pub dep_schemas: Vec<BmmSchema>,
    /// The merged resolution model (`dep_schemas` then [`Self::own_schema`]) —
    /// the crate-level view a downstream crate resolves against.
    pub model: Model,
    /// The prelude index resolving `prelude_deps` cross-crate references.
    pub external: External,
    /// The crate doc comment.
    pub doc: &'static str,
}

/// Load a composition's own BMM files, one loaded schema per generation, in
/// declaration order.
///
/// # Errors
/// Returns an error if any member BMM file cannot be loaded, or `key` names no
/// composition entry, or the entry lists no own file.
fn generation_schemas(
    key: &str,
) -> Result<Vec<(&'static str, BmmSchema)>, Box<dyn std::error::Error>> {
    let comp = lookup(key)?;
    if comp.own.is_empty() {
        return Err(format!("composition {key:?} lists no own BMM file").into());
    }
    comp.own
        .iter()
        .map(|f| load_bmm(f).map(|s| (*f, s)))
        .collect()
}

/// Fold a composition's generations into its crate-level dependency view (one
/// type per Rust name; see [`BmmSchema::dependency_view`]).
///
/// # Errors
/// Returns an error if `generations` is empty (a schema-less crate, which
/// [`generation_schemas`] already rejects).
fn fold_dependency_view(
    generations: &[(&'static str, BmmSchema)],
) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    let (first, rest) = generations
        .split_first()
        .ok_or("composition lists no own BMM file")?;
    let mut view = first.1.clone();
    for (_, s) in rest {
        view = view.dependency_view(s);
    }
    Ok(view)
}

/// Resolve a composition's crate-level dependency view (its own BMM files folded
/// last-wins).
///
/// # Errors
/// Returns an error if any member BMM file cannot be loaded.
fn dependency_view_schema(key: &str) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    fold_dependency_view(&generation_schemas(key)?)
}

/// Resolve a composition entry into its per-generation schemas + models, the
/// crate-level dependency view, and the prelude index the render stage consumes.
///
/// # Errors
/// Returns an error if any member/dependency BMM file cannot be loaded.
pub(crate) fn compose(key: &str) -> Result<Composed, Box<dyn std::error::Error>> {
    let comp = lookup(key)?;
    let loaded = generation_schemas(key)?;
    let own = fold_dependency_view(&loaded)?;

    let dep_schemas: Vec<BmmSchema> = comp
        .model_deps
        .iter()
        .map(|d| dependency_view_schema(d))
        .collect::<Result<_, _>>()?;

    // model = merged(dep_schemas.. , own) — BASE first, last-wins on collision.
    let mut merge_refs: Vec<&BmmSchema> = dep_schemas.iter().collect();
    merge_refs.push(&own);
    let model = Model::merged(&merge_refs);

    // One model per generation: dependency schemas below, this generation alone
    // on top. Ownership of a colliding name goes to the LAST generation that
    // declares it (the crate prelude's one-entry-per-name rule).
    let mut generations = Vec::with_capacity(loaded.len());
    for (i, (file, schema)) in loaded.iter().enumerate() {
        let mut refs: Vec<&BmmSchema> = dep_schemas.iter().collect();
        refs.push(schema);
        let owned = schema
            .classes
            .keys()
            .filter(|n| {
                !loaded
                    .iter()
                    .skip(i + 1)
                    .any(|(_, later)| later.classes.contains_key(*n))
            })
            .cloned()
            .collect();
        generations.push(Generation {
            file,
            schema: schema.clone(),
            model: Model::merged(&refs),
            owned,
        });
    }

    // external = the prelude index over prelude_deps (each dep's emittable specs
    // under its prelude path), in declaration order.
    let mut external = External::default().in_crate(comp.crate_name);
    for dep_key in comp.prelude_deps {
        let dep_comp = lookup(dep_key)?;
        let dep_own = dependency_view_schema(dep_key)?;
        let dep_dep_schemas: Vec<BmmSchema> = dep_comp
            .model_deps
            .iter()
            .map(|d| dependency_view_schema(d))
            .collect::<Result<_, _>>()?;
        let mut dep_merge: Vec<&BmmSchema> = dep_dep_schemas.iter().collect();
        dep_merge.push(&dep_own);
        let dep_model = Model::merged(&dep_merge);
        external = external.with(
            emittable_specs(&dep_model, &dep_own),
            &prelude_path(dep_comp),
        );
    }

    Ok(Composed {
        generations,
        own_schema: own,
        dep_schemas,
        model,
        external,
        doc: comp.doc,
    })
}
