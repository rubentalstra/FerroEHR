//! The **crate → generation table**: declarative data recording, per emitted
//! spec crate, the BMM GENERATIONS it carries — vendored input file, emitted
//! generation-module name, per-generation spec version, which generation is
//! CURRENT, and the dependency GENERATIONS each one resolves against. Each
//! entry carries the `includes` citation that justifies its composition.
//!
//! This is the single source of truth for what the generator emits. [`compose`]
//! resolves an entry into per-generation loaded schemas + resolution models +
//! the [`External`] full-path index the render stage consumes, so `cli.rs`
//! never hand-merges schemas — the membership is data, not control flow.
//!
//! **One generation = one vendored BMM file, emitted completely** at its own
//! version-named top module (`v1_2`, `v2_4`, …) mirroring its source package
//! structure. Generations are never merged into one class map — a merge
//! silently picks one shape per colliding name and discards the other's
//! attributes. The crate prelude re-exports the CURRENT generation only; an
//! older generation's types are reached by full module path.
//!
//! # NOTE: the five RM/BASE twin classes are spec-mandated, not accidental
//!
//! `AUTHORED_RESOURCE`, `RESOURCE_DESCRIPTION`, `RESOURCE_DESCRIPTION_ITEM`,
//! `TRANSLATION_DETAILS` and `CODE_PHRASE` are declared by BOTH the RM 1.2.0 and
//! the BASE 1.3.0 BMM, with materially different shapes, and BOTH generations
//! are emitted (the RM twin into `openehr-rm`, the BASE twin into
//! `openehr-base`). That is what the vendored components state, first-hand:
//!
//! - **The resource package.** RM `docs/common/master08-resource_package.adoc`
//!   opens with the normative note that "the version of the Resource package
//!   described below is used only in ADL 1.4 archetypes, i.e. via the AOM 1.4
//!   archetype model. A newer version of this package is defined in the openEHR
//!   Resource Specification in the BASE component, and is used in ADL 2
//!   archetypes … with the older form here retained only while needed by AOM 1.4
//!   based archetypes and tools." Two versions of one package, kept side by side
//!   on purpose — the AM 1.4/2.4 situation, in the components that own
//!   them. The RM twin is therefore NOT a stale copy to retire, and the two
//!   member-level differences that look like defects are the older generation's
//!   real shape: `TRANSLATION_DETAILS.accreditaton` (RM
//!   `docs/UML/classes/org.openehr.rm.common.translation_details.adoc`) and
//!   `copyright` on `RESOURCE_DESCRIPTION_ITEM` rather than
//!   `RESOURCE_DESCRIPTION` (RM
//!   `docs/UML/classes/org.openehr.rm.common.resource_description_item.adoc`).
//!   The published ADL-1.4 schema agrees on the second one — `Resource.xsd`
//!   in the vendored `AM/Release-1.4` bundle declares `copyright` inside
//!   `RESOURCE_DESCRIPTION_ITEM` — so only the `accreditaton` spelling is a
//!   genuine upstream defect: BASE `docs/resource/master00-amendment_record.adoc`
//!   records SPECPUB-6, "Correct spelling error in
//!   `TRANSLATION_DETAILS._accreditation_`", against the BASE copy alone, and
//!   every published `Resource.xsd` in BOTH ITS-XML lineages spells the element
//!   `accreditation`, leaving the RM component's retained copy the only artifact
//!   still carrying the typo. The emitter reproduces its input; correcting the
//!   RM spelling is an upstream matter, not an override.
//! - **`CODE_PHRASE`.** BASE `docs/foundation_types/master00-amendment_record.adoc`
//!   records SPECAM-82 as "Add **legacy** `CODE_PHRASE` class to Foundation Types
//!   to support AOM 1.4 model", and the vendored BASE 1.3.0 BMM's own class
//!   documentation says "Retain for LEGACY only, while ADL1.4 requires
//!   `CODE_PHRASE`" (that sentence is propagated into the generated
//!   `openehr_base` type). It is an ADDITION for AM 1.4's benefit — AM's BMM
//!   includes BASE, not RM — not a relocation of the RM class, so
//!   `openehr_rm::…::text::CODE_PHRASE` stays the live domain type and
//!   there is no move to finish.

use crate::analyze::{External, Model, emittable_specs};
use crate::load::bmm::BmmSchema;
use std::path::Path;

/// The vendored BMM root. Paths below mirror the upstream ITS-BMM layout
/// (`components/<COMPONENT>/json/…`); the JSON forms are the codegen input for
/// our pinned versions (each schema's pin is recorded in its `PROVENANCE.md`).
const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/bmm");

pub(crate) const BASE_BMM: &str = "components/BASE/json/openehr_base_1.3.0.bmm.json";
/// BASE's latest RELEASED generation (1.2.0, 09-Apr-2021) — the `stable`
/// profile's BASE pairing (#1936: RM 1.1.0 is modelled against BASE 1.2.0).
pub(crate) const BASE12_BMM: &str = "components/BASE/json/openehr_base_1.2.0.bmm.json";
pub(crate) const RM_BMM: &str = "components/RM/json/openehr_rm_1.2.0.bmm.json";
/// RM's latest RELEASED generation (1.1.0, 29-Sep-2020); its BMM `includes`
/// names `openehr_base_1.2.0` — the released pairing, first-hand.
pub(crate) const RM11_BMM: &str = "components/RM/json/openehr_rm_1.1.0.bmm.json";
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
/// side (the AM precedent —
/// `BASE/docs/architecture_overview/master05-package_structure.adoc` §AM
/// Component, "Both versions are maintained side by side").
pub(crate) const LANG_BMM3: &str = "components/LANG/json/openehr_lang_1.1.0-bmm3.bmm.json";

const BASE_DOC: &str = "openEHR BASE (foundation + base types), generated from the BMM meta-model.";
const RM_DOC: &str = "openEHR RM (Reference Model), generated from the BMM meta-model.";
const LANG_DOC: &str = "openEHR LANG: the BMM object model in BOTH its extant generations, \
    generated from the BMM meta-model — the stable v2.x model (`v2`: the `bmm` object model, \
    its `bmm_persistence` P_BMM form and the `beom` expression model) and the v3 development \
    line (`v3`: `bmm3`, with the `EL_*` expression and `BMM_STATEMENT*` families). Each \
    generation is emitted completely under its own version module; the crate prelude re-exports \
    the current generation (`v3`) only. The generator's own BMM reader lives in openehr-codegen \
    (tooling, not spec); the hand-written ODIN reader and BEL parser live beside this generated \
    tree.";
const AM_DOC: &str = "openEHR AM (Archetype Model): `v1_4` (AM 1.4.0, for ADL 1.4) and `v2_4` \
    (AM 2.4.0, for ADL 2) — both generated from BMM. Both ADL versions are in use.";
const TERM_DOC: &str = "openEHR TERM (Terminology) data model, generated from the BMM \
    meta-model. The vendored terminology XML content lives in `assets/` (data, not \
    generated); an XML→model loader is added when composition validation needs it.";

/// One dependency **generation** a generation resolves against: the dependency
/// crate's composition key plus the generation module inside it.
pub(crate) struct DepGeneration {
    /// Composition key of the dependency crate (`base`, `lang`, …).
    pub key: &'static str,
    /// The generation module inside that crate (`v1_3`, `v2`, …).
    pub generation: &'static str,
}

/// One BMM generation of an emitted crate: exactly one vendored file, emitted
/// completely under its own version-named top module.
pub(crate) struct GenerationSpec {
    /// The emitted generation-module name (`v1_2`, `v2_4`; LANG's `v2`/`v3`
    /// are the BMM meta-model majors — both its files carry the same LANG
    /// release, so the spec-version-derived name cannot distinguish them).
    pub module: &'static str,
    /// The openEHR specification version this generation implements (the
    /// vendored file's pin) — emitted as the generation module's
    /// `SPEC_VERSION` constant and the [`Generation`] enum's
    /// `spec_version()` value.
    pub spec_version: &'static str,
    /// The vendored BMM file (relative to the vendor root).
    pub file: &'static str,
    /// Whether this is the crate's CURRENT generation: the one the crate
    /// prelude re-exports and the crate-level `Generation::CURRENT` names.
    /// Exactly one generation per crate is current ([`compose`] asserts it).
    pub current: bool,
    /// Dependency generations merged (in order, before this generation's own
    /// schema) into the resolution model — last-wins on name collision.
    pub model_deps: &'static [DepGeneration],
    /// Dependency generations whose exported names resolve this generation's
    /// cross-crate references, consulted in order (FIRST match wins — listing
    /// two generations of one dependency decides collisions by list order).
    pub prelude_deps: &'static [DepGeneration],
}

/// One emitted crate and the BMM generations that compose it.
pub(crate) struct CrateComposition {
    /// Unique key (`base`, `rm`, `lang`, `am`, `term`).
    pub key: &'static str,
    /// Emitted crate directory.
    pub crate_name: &'static str,
    /// The crate-level implemented-spec pin, emitted as the crate's
    /// `SPEC_VERSION` constant — deliberately independent of the crates.io
    /// package version. Usually the current generation's version; LANG
    /// deviates (its crate pin is the latest LANG release, 1.0.0, while both
    /// vendored files are 1.1.0-line snapshots — `docs/VERSIONS.md` §openEHR
    /// specification matrix).
    pub spec_version: &'static str,
    /// The crate's BMM generations, oldest first. Exactly one is `current`.
    pub generations: &'static [GenerationSpec],
    /// Crate doc comment (emitted into `lib.rs`).
    pub doc: &'static str,
    /// The `includes` citation that justifies the composition.
    pub citation: &'static str,
    /// One-line reason.
    pub reason: &'static str,
}

/// The declarative crate → generation table.
pub(crate) const COMPOSITIONS: &[CrateComposition] = &[
    CrateComposition {
        key: "base",
        crate_name: "openehr-base",
        spec_version: "1.3.0",
        generations: &[
            GenerationSpec {
                module: "v1_2",
                spec_version: "1.2.0",
                file: BASE12_BMM,
                current: false,
                model_deps: &[],
                prelude_deps: &[],
            },
            GenerationSpec {
                module: "v1_3",
                spec_version: "1.3.0",
                file: BASE_BMM,
                current: true,
                model_deps: &[],
                prelude_deps: &[],
            },
        ],
        doc: BASE_DOC,
        citation: "BASE BMMs (openehr_base_1.2.0 released + openehr_base_1.3.0 development) — no \
                   includes; the foundation crate. Both generations emitted side by side \
                   (#1936: the released generation stays selectable).",
        reason: "Foundation types; nothing below it.",
    },
    CrateComposition {
        key: "rm",
        crate_name: "openehr-rm",
        spec_version: "1.2.0",
        generations: &[
            GenerationSpec {
                module: "v1_1",
                spec_version: "1.1.0",
                file: RM11_BMM,
                current: false,
                model_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_2",
                }],
                prelude_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_2",
                }],
            },
            GenerationSpec {
                module: "v1_2",
                spec_version: "1.2.0",
                file: RM_BMM,
                current: true,
                model_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_3",
                }],
                prelude_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_3",
                }],
            },
        ],
        doc: RM_DOC,
        citation: "RM 1.2.0 BMM includes openehr_base_1.3.0; RM 1.1.0 BMM includes \
                   openehr_base_1.2.0 — each generation resolves against its OWN released \
                   pairing, first-hand from the files' `includes`. Five class names are \
                   declared by both an RM and its paired BASE file and the RM declaration \
                   wins the merge, which is correct in every case — see the module NOTE on \
                   the RM/BASE twin classes.",
        reason: "The domain model; RM 1.2.0 pairs with BASE 1.3.0, RM 1.1.0 with BASE 1.2.0.",
    },
    CrateComposition {
        key: "lang",
        crate_name: "openehr-lang",
        spec_version: "1.0.0",
        generations: &[
            GenerationSpec {
                module: "v2",
                spec_version: "1.1.0",
                file: LANG_BMM,
                current: false,
                model_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_3",
                }],
                prelude_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_3",
                }],
            },
            GenerationSpec {
                module: "v3",
                spec_version: "1.1.0",
                file: LANG_BMM3,
                current: true,
                model_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_3",
                }],
                prelude_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_3",
                }],
            },
        ],
        doc: LANG_DOC,
        citation: "LANG 1.1.0 BMM includes openehr_base_1.3.0. Two GENERATIONS of the same \
                   meta-model compose the crate and both are emitted completely, each under its \
                   own version module: the stable v2.x BMM + P_BMM + beom \
                   (LANG/docs/bmm/master01-preface.adoc §History — \"the normative, \
                   tool-implemented version\") as `v2`, and the v3 development line \
                   (LANG/docs/bmm3/master01-preface.adoc §Previous Versions; \
                   master00-amendment_record.adoc SPECLANG-14 \"Formalise the BMM v2/v3 \
                   split\") as `v3`. The module names are the BMM meta-model majors: both \
                   files carry the same LANG release, so a spec-version-derived name cannot \
                   distinguish them. `v3` is current (the crate prelude), preserving the \
                   pre-table prelude semantics where the v3 twin won every colliding name.",
        reason: "The BMM/P_BMM object model, both extant generations; depends on BASE.",
    },
    CrateComposition {
        key: "am",
        crate_name: "openehr-am",
        spec_version: "2.4.0",
        generations: &[
            GenerationSpec {
                module: "v1_4",
                spec_version: "1.4.0",
                file: AM14_BMM,
                current: false,
                model_deps: &[DepGeneration {
                    key: "base",
                    generation: "v1_3",
                }],
                prelude_deps: &[
                    DepGeneration {
                        key: "base",
                        generation: "v1_3",
                    },
                    DepGeneration {
                        key: "lang",
                        generation: "v3",
                    },
                    DepGeneration {
                        key: "lang",
                        generation: "v2",
                    },
                ],
            },
            GenerationSpec {
                module: "v2_4",
                spec_version: "2.4.0",
                file: AM24_BMM,
                current: true,
                model_deps: &[
                    DepGeneration {
                        key: "base",
                        generation: "v1_3",
                    },
                    DepGeneration {
                        key: "lang",
                        generation: "v2",
                    },
                    DepGeneration {
                        key: "lang",
                        generation: "v3",
                    },
                ],
                prelude_deps: &[
                    DepGeneration {
                        key: "base",
                        generation: "v1_3",
                    },
                    DepGeneration {
                        key: "lang",
                        generation: "v3",
                    },
                    DepGeneration {
                        key: "lang",
                        generation: "v2",
                    },
                ],
            },
        ],
        doc: AM_DOC,
        citation: "AM 1.4.0 BMM includes openehr_base_1.3.0; ARCHETYPE extends BASE's \
                   AUTHORED_RESOURCE, whose Rust form widens downstream, so the \
                   AUTHORED_RESOURCE + RESOURCE_DESCRIPTION closure re-emits crate-locally \
                   (#1699; `augmented_schema`). AM 2.4.0 BMM includes openehr_lang_1.1.0 + \
                   openehr_base_1.3.0; its rules package declares subtypes of LANG's beom \
                   expression classes (EXPR_ARCHETYPE_REF ⊂ EXPR_VALUE_REF, EXPR_CONSTRAINT ⊂ \
                   EXPR_LEAF), so the downstream re-emission closure is non-empty. It reaches \
                   classes of BOTH LANG generations (v2 beom + v3 BMM_*/EL_*), so its deps \
                   list both — model merge last-wins (v2 then v3), reference lookup first-wins \
                   (v3 then v2), preserving the pre-table one-name-per-type resolution.",
        reason: "Both extant ADL generations, side by side (BASE \
                 architecture_overview master05 §AM Component).",
    },
    CrateComposition {
        key: "term",
        crate_name: "openehr-term",
        spec_version: "3.1.0",
        generations: &[GenerationSpec {
            module: "v3_1",
            spec_version: "3.1.0",
            file: TERM_BMM,
            current: true,
            model_deps: &[DepGeneration {
                key: "base",
                generation: "v1_3",
            }],
            prelude_deps: &[DepGeneration {
                key: "base",
                generation: "v1_3",
            }],
        }],
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

/// Load and parse one vendored BMM file (relative to [`VENDOR`]).
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub(crate) fn load_bmm(file: &str) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(Path::new(VENDOR).join(file))?;
    Ok(BmmSchema::parse_json(&src)?)
}

/// One resolved BMM generation of a composed crate: its loaded schema, the
/// resolution model (paired dependency generations below, this generation on
/// top), and the [`External`] index resolving its cross-crate references to
/// full generation-module paths.
pub(crate) struct ComposedGeneration {
    /// The table row this generation was resolved from.
    pub spec: &'static GenerationSpec,
    /// This generation's own schema — one vendored file, verbatim.
    pub schema: BmmSchema,
    /// The paired dependency generations' schemas (`model_deps`, in order).
    pub dep_schemas: Vec<BmmSchema>,
    /// The merged resolution model (`dep_schemas` then [`Self::schema`]) — a
    /// class of this generation never resolves an ancestor, field type or
    /// subtype against another generation's definitions.
    pub model: Model,
    /// The full-path index resolving `prelude_deps` cross-crate references.
    pub external: External,
}

/// A composition entry resolved into its per-generation schemas + models.
pub(crate) struct Composed {
    /// The composition entry this was resolved from.
    pub comp: &'static CrateComposition,
    /// The crate's generations, in table (oldest-first) order.
    pub generations: Vec<ComposedGeneration>,
}

impl Composed {
    /// The crate's CURRENT generation — the prelude/default surface.
    ///
    /// # Panics
    /// Panics if the table declares no current generation for the crate
    /// ([`compose`] validates exactly-one-current first, so this is
    /// unreachable on a composed value).
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "compose() rejects a table row without exactly one current generation before \
                  constructing Composed, so a composed value always has one"
    )]
    pub(crate) fn current(&self) -> &ComposedGeneration {
        self.generations
            .iter()
            .find(|g| g.spec.current)
            .expect("a composed crate should carry exactly one current generation")
    }
}

/// The full Rust path of the module a dependency generation emits `spec` at
/// (`openehr_base::v1_3::base_types::identification::uid`) — the path the
/// [`External`] index hands the render stage.
fn generation_module_path(
    comp: &CrateComposition,
    generation: &str,
    schema: &BmmSchema,
    spec: &str,
) -> String {
    format!(
        "{}::{}::{}",
        comp.crate_name.replace('-', "_"),
        generation,
        crate::render::emit::type_module_path(schema, spec)
    )
}

/// Find a generation row inside a composition entry.
///
/// # Errors
/// Returns an error if `generation` names no row of `comp` (a table bug).
fn generation_spec(
    comp: &'static CrateComposition,
    generation: &str,
) -> Result<&'static GenerationSpec, Box<dyn std::error::Error>> {
    comp.generations
        .iter()
        .find(|g| g.module == generation)
        .ok_or_else(|| {
            format!(
                "composition {:?} has no generation {generation:?} (table bug: a DepGeneration \
                 names a generation module its dependency crate does not declare)",
                comp.key
            )
            .into()
        })
}

/// Load one generation's schema plus its merged resolution model (paired
/// dependency generations below, the generation's own schema on top).
///
/// # Errors
/// Returns an error if any involved BMM file cannot be loaded or a dependency
/// reference names a missing key/generation.
fn generation_model(
    spec: &'static GenerationSpec,
) -> Result<(BmmSchema, Vec<BmmSchema>, Model), Box<dyn std::error::Error>> {
    let schema = load_bmm(spec.file)?;
    let mut dep_schemas = Vec::with_capacity(spec.model_deps.len());
    for dep in spec.model_deps {
        let dep_comp = lookup(dep.key)?;
        let dep_spec = generation_spec(dep_comp, dep.generation)?;
        dep_schemas.push(load_bmm(dep_spec.file)?);
    }
    let mut refs: Vec<&BmmSchema> = dep_schemas.iter().collect();
    refs.push(&schema);
    let model = Model::merged(&refs);
    Ok((schema, dep_schemas, model))
}

/// Resolve a composition entry into its per-generation schemas, models, and
/// full-path [`External`] indexes.
///
/// # Errors
/// Returns an error if the table row is malformed (zero or several `current`
/// generations, duplicate module names), any BMM file cannot be loaded, or a
/// dependency reference names a missing key/generation.
pub(crate) fn compose(key: &str) -> Result<Composed, Box<dyn std::error::Error>> {
    let comp = lookup(key)?;
    if comp.generations.is_empty() {
        return Err(format!("composition {key:?} lists no generation").into());
    }
    let current_n = comp.generations.iter().filter(|g| g.current).count();
    if current_n != 1 {
        return Err(format!(
            "composition {key:?} declares {current_n} current generations (exactly one required)"
        )
        .into());
    }
    let mut modules: Vec<&str> = comp.generations.iter().map(|g| g.module).collect();
    modules.sort_unstable();
    modules.dedup();
    if modules.len() != comp.generations.len() {
        return Err(format!("composition {key:?} declares duplicate generation modules").into());
    }

    let mut generations = Vec::with_capacity(comp.generations.len());
    for spec in comp.generations {
        let (schema, dep_schemas, model) = generation_model(spec)?;

        // The External index over prelude_deps: each dependency generation's
        // emittable specs mapped to full generation-module paths, consulted
        // first-wins in table order.
        let mut external = External::default().in_crate(comp.crate_name);
        for dep in spec.prelude_deps {
            let dep_comp = lookup(dep.key)?;
            let dep_spec = generation_spec(dep_comp, dep.generation)?;
            let (dep_schema, _, dep_model) = generation_model(dep_spec)?;
            let modules = emittable_specs(&dep_model, &dep_schema)
                .into_iter()
                .map(|s| {
                    let path = generation_module_path(dep_comp, dep.generation, &dep_schema, &s);
                    (s, path)
                })
                .collect();
            external = external.with(modules);
        }

        generations.push(ComposedGeneration {
            spec,
            schema,
            dep_schemas,
            model,
            external,
        });
    }

    Ok(Composed { comp, generations })
}

/// The Rust identifier of a generation's [`Generation`]-enum variant
/// (`v1_2` → `V1_2`) — the module token upper-cased with its underscores
/// KEPT: collapsing them (`V12`) would make `v1_2` and a future `v12`
/// indistinguishable.
pub(crate) fn generation_variant(module: &str) -> String {
    module.to_uppercase()
}
