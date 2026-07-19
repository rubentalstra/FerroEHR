//! The **crate → schema-merge table**: declarative data recording which vendored
//! BMM component files compose each emitted spec crate, and which dependency
//! crates' preludes resolve its cross-crate references. Each entry carries the
//! `includes` citation that justifies the merge.
//!
//! This is the single source of truth for schema composition. [`compose`]
//! resolves an entry into the loaded model + own schema + `External` prelude
//! index the render stage consumes, so `cli.rs` never hand-merges schemas — the
//! membership is data, not control flow. Pure re-representation: the resolver
//! reproduces, byte-for-byte, the models the R1 pipeline built inline (Model
//! merges are order-sensitive only on name collision, and this table preserves
//! the exact merge order; `own` files are combined first-wins, matching
//! [`BmmSchema::combined`]).

use crate::analyze::{External, Model, emittable_specs};
use crate::load::bmm::BmmSchema;
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
pub(crate) const LANG_BMM: &str = "components/LANG/json/openehr_lang_1.1.0.bmm.json";
/// LANG's model spans two vendored files: the primary one (persisted BMM with
/// `EXPR_*` and `STATEMENT_SET`/`ASSERTION`, which AM's rules/slots reference)
/// and this BMM-3 file (the full `BMM_*` object model with the `EL_*` expression
/// language, which AM's persisted-archetype rules reference). Both compose the
/// `openehr-lang` crate (combined first-wins).
pub(crate) const LANG_BMM3: &str = "components/LANG/json/openehr_lang_1.1.0-bmm3.bmm.json";

const BASE_DOC: &str = "openEHR BASE (foundation + base types), generated from the BMM meta-model.";
const RM_DOC: &str = "openEHR RM (Reference Model), generated from the BMM meta-model.";
const LANG_DOC: &str = "openEHR LANG: the BMM / P_BMM object model, generated from the BMM \
    meta-model. The generator's own BMM reader lives in openehr-codegen (tooling, not spec); \
    the runtime ODIN and EL parsers are future hand-written work (P8/P9).";
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
    /// The crate's own BMM file(s), combined first-wins into the own schema.
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
        citation: "LANG 1.1.0 BMM includes openehr_base_1.3.0; the persisted BMM + the BMM-3 \
                   object model compose one crate (combined first-wins).",
        reason: "The BMM/P_BMM object model; depends on BASE.",
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
                   downstream re-emission closure is non-empty.",
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

/// A resolver's own-schema plus the loaded dependency schemas (kept so the
/// caller can compute the re-emission closure over the same source schemas).
pub(crate) struct Composed {
    /// The crate's own schema (own BMM files, combined first-wins).
    pub own_schema: BmmSchema,
    /// The dependency schemas (`model_deps`, in order) that merge below it.
    pub dep_schemas: Vec<BmmSchema>,
    /// The merged resolution model (`dep_schemas` then `own_schema`).
    pub model: Model,
    /// The prelude index resolving `prelude_deps` cross-crate references.
    pub external: External,
    /// The crate doc comment.
    pub doc: &'static str,
}

/// Resolve a composition's own schema (its own BMM file(s), combined first-wins
/// — the same semantics [`BmmSchema::combined`] applies).
///
/// # Errors
/// Returns an error if any member BMM file cannot be loaded, or `key` names no
/// composition entry, or the entry lists no own file.
fn own_schema(key: &str) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    let comp = lookup(key)?;
    let (first, rest) = comp
        .own
        .split_first()
        .ok_or_else(|| format!("composition {key:?} lists no own BMM file"))?;
    let mut schema = load_bmm(first)?;
    for f in rest {
        schema = schema.combined(&load_bmm(f)?);
    }
    Ok(schema)
}

/// Resolve a composition entry into the loaded model, own schema, and prelude
/// index the render stage consumes.
///
/// # Errors
/// Returns an error if any member/dependency BMM file cannot be loaded.
pub(crate) fn compose(key: &str) -> Result<Composed, Box<dyn std::error::Error>> {
    let comp = lookup(key)?;
    let own = own_schema(key)?;

    let dep_schemas: Vec<BmmSchema> = comp
        .model_deps
        .iter()
        .map(|d| own_schema(d))
        .collect::<Result<_, _>>()?;

    // model = merged(dep_schemas.. , own) — BASE first, last-wins on collision.
    let mut merge_refs: Vec<&BmmSchema> = dep_schemas.iter().collect();
    merge_refs.push(&own);
    let model = Model::merged(&merge_refs);

    // external = the prelude index over prelude_deps (each dep's emittable specs
    // under its prelude path), in declaration order.
    let mut external = External::default();
    for dep_key in comp.prelude_deps {
        let dep_comp = lookup(dep_key)?;
        let dep_own = own_schema(dep_key)?;
        let dep_dep_schemas: Vec<BmmSchema> = dep_comp
            .model_deps
            .iter()
            .map(|d| own_schema(d))
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
        own_schema: own,
        dep_schemas,
        model,
        external,
        doc: comp.doc,
    })
}
