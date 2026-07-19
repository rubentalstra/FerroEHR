#![allow(clippy::format_push_string, clippy::too_many_lines)]
// Build-time codegen CLI (never ships in the server): the console IS its
// user interface and a malformed vendored spec must abort loudly, so the
// reliability deny-tier for shipped code is deliberately relaxed here
// (.claude/rules/reliability.md §tools). `let _ = writeln!(String)` is the
// infallible in-memory emit idiom.
#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::let_underscore_must_use
)]

//! `openehr-codegen` — generates the openEHR spec crates from the vendored BMM
//! meta-model.
//!
//! Usage:
//!   `openehr-codegen check`          — load + validate the vendored BMM schemas.
//!   `openehr-codegen emit [OUTDIR]`  — emit Rust into OUTDIR (default:
//!                                       `target/codegen-preview`).

mod bmm;
mod emit;
mod emit_opt;
mod emit_rest;
mod emit_rm_model;
mod emit_xml;
mod naming;
mod oas;
mod xsd;

use bmm::BmmSchema;
use emit::{External, Model};
use std::path::{Path, PathBuf};

const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/bmm");
/// The `openehr-its` crate root (holds the vendored XSDs/OAS and receives the
/// generated XML/REST code).
#[allow(dead_code)] // used by the emit-xml/emit-rest writers (landing incrementally)
const ITS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../openehr-its");
/// v1 (namespace `.../v1`) RM-instance XSD bundle dir — the Stage-1 parity target.
const XSD_V1_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../openehr-its/schemas/xml/its-xml-1.0.2-nsv1/ALL"
);
/// v2 (namespace `.../v2`) XSD root (per-component release folders). Supplies the
/// RM-instance types the v1 `ALL/` bundle lacks (EHR + demographic) or carries
/// stale (extract) to the emit-xml input.
const XSD_V2_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../openehr-its/schemas/xml/its-xml-2.0.0-nsv2"
);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map_or("check", String::as_str);
    let result = match cmd {
        "check" => cmd_check(),
        "emit" => cmd_emit(args.get(1).map(PathBuf::from)),
        "check-xsd" => cmd_check_xsd(),
        "emit-xml" => cmd_emit_xml(),
        "emit-rest" => cmd_emit_rest(),
        "emit-opt" => cmd_emit_opt(),
        "emit-rm-model" => cmd_emit_rm_model(),
        other => {
            eprintln!(
                "unknown command {other:?}; use `check`, `emit [OUTDIR]`, `check-xsd`, `emit-xml`, `emit-rest`, `emit-opt`, or `emit-rm-model`"
            );
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

// Paths are relative to `VENDOR` and mirror the upstream ITS-BMM layout
// (`components/<COMPONENT>/json/…`) — the full meta-model is vendored verbatim
// (json + odin + yaml, all released versions); the JSON forms below are the
// codegen input for our pinned versions (see `docs/VERSIONS.md`).
const BASE_BMM: &str = "components/BASE/json/openehr_base_1.3.0.bmm.json";
const RM_BMM: &str = "components/RM/json/openehr_rm_1.2.0.bmm.json";
const TERM_BMM: &str = "components/TERM/json/openehr_term_3.1.0.bmm.json";
const AM14_BMM: &str = "components/AM/json/openehr_am_1.4.0.bmm.json";
const AM24_BMM: &str = "components/AM/json/openehr_am_2.4.0.bmm.json";
const LANG_BMM: &str = "components/LANG/json/openehr_lang_1.1.0.bmm.json";
/// LANG's model spans two vendored files: the primary one above (persisted BMM
/// with `EXPR_*` and `STATEMENT_SET`/`ASSERTION`, which AM's rules/slots
/// reference) and this BMM-3 file (the full `BMM_*` object model with the
/// `EL_*` expression language, which AM's persisted-archetype rules reference).
/// Both are merged into the `openehr-lang` crate.
const LANG_BMM3: &str = "components/LANG/json/openehr_lang_1.1.0-bmm3.bmm.json";

fn load(file: &str) -> Result<BmmSchema, Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(Path::new(VENDOR).join(file))?;
    Ok(BmmSchema::parse_json(&src)?)
}

fn cmd_check() -> Result<(), Box<dyn std::error::Error>> {
    for file in [BASE_BMM, RM_BMM, TERM_BMM] {
        let s = load(file)?;
        let abstract_n = s.classes.values().filter(|c| c.is_abstract).count();
        let generic_n = s
            .classes
            .values()
            .filter(|c| !c.generic_params.is_empty())
            .count();
        println!(
            "✓ {file}: schema={} release={} classes={} (abstract={abstract_n}, generic={generic_n})",
            s.schema_name,
            s.rm_release,
            s.classes.len(),
        );
    }
    Ok(())
}

/// Emit the ITS-REST contract (DTOs, param structs, server trait, route table)
/// for each API group into `openehr-its/src/rest/generated/`.
fn cmd_emit_rest() -> Result<(), Box<dyn std::error::Error>> {
    // Groups with operations (overview is an index, system has none).
    const GROUPS: &[&str] = &["admin", "definition", "demographic", "ehr", "query"];
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let base_model = Model::merged(&[&base]);
    let rm_model = Model::merged(&[&base, &rm]);
    // OAS $ref names are PascalCase (`EhrStatus`) — the same as the emitted Rust
    // type names — so map each crate's emittable spec names through `type_name`.
    let names = emit_rest::RmNames {
        base: emit::emittable_specs(&base_model, &base)
            .iter()
            .map(|s| naming::type_name(s))
            .collect(),
        rm: emit::emittable_specs(&rm_model, &rm)
            .iter()
            .map(|s| naming::type_name(s))
            .collect(),
    };

    let oas_dir = Path::new(ITS_ROOT).join("vendor/rest-oas");
    let gen_dir = Path::new(ITS_ROOT).join("src/rest/generated");
    std::fs::create_dir_all(&gen_dir)?;

    let mut written = Vec::new();
    for group in GROUPS {
        let oas = oas::Oas::parse_file(&oas_dir.join(format!("{group}-codegen.openapi.yaml")))?;
        let body = emit_rest::emit_group(&oas, group, &names);
        let path = gen_dir.join(format!("{group}.rs"));
        std::fs::write(&path, &body)?;
        written.push(path);
    }
    let mod_rs = {
        let mut s = String::from(
            "// @generated by openehr-codegen (emit-rest) — DO NOT EDIT.\n\
             //! ITS-REST contract, one module per API group.\n\n",
        );
        for g in GROUPS {
            s.push_str(&format!("pub mod {g};\n"));
        }
        s
    };
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&mod_path, mod_rs)?;
    written.push(mod_path);

    rustfmt(&written)?;
    println!("emitted {} files into {}", written.len(), gen_dir.display());
    Ok(())
}

/// Emit canonical-XML `ToXml`/`FromXml` impls for the RM/BASE spec types into
/// `openehr-its/src/xml/generated/`. Generates both wire lineages: v1
/// (`.../v1`, parity target) and v2 (`.../v2`, latest).
fn cmd_emit_xml() -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let base_model = Model::merged(&[&base]);
    let rm_model = Model::merged(&[&base, &rm]);

    // The RM-instance wire shape (element names, order, xsi:type, attributes) is
    // identical across the two ITS-XML lineages; they differ only by the root
    // `xmlns` string, which the `Namespace` serialize-time param selects. So a
    // single `ToXml` impl set — generated from the v1 (parity) XSD — serves both
    // (one impl per type; a second set would be a duplicate-impl conflict).
    //
    // The v1 `ALL/` bundle is not a complete RM closure: it has no `Ehr.xsd` and
    // no demographic schema, and its `Extract.xsd` is the stale RM-1.0.2 model.
    // So the emit-xml input is the v1 *served* core (which wins for shared types
    // via first-wins `.or_insert`) followed by the v2 RM-1.1.0 EHR/demographic/
    // extract schemas, which supply the LOCATABLE subtypes the v1 bundle lacks —
    // resolving `archetype_node_id` as the required XML **attribute** for
    // EHR_STATUS/EHR_ACCESS, the demographic PARTY hierarchy, and the extract
    // LOCATABLE subtypes. Same wire shape bar the root `xmlns`.
    let v1 = xsd::XsdModel::parse_files(&xsd::xml_emit_files(
        Path::new(XSD_V1_DIR),
        Path::new(XSD_V2_DIR),
    ))?;

    let gen_dir = Path::new(ITS_ROOT).join("src/xml/generated");
    std::fs::create_dir_all(&gen_dir)?;

    let schemas = [
        emit_xml::XmlSchema {
            model: &base_model,
            schema: &base,
            prelude: "openehr_base::prelude",
        },
        emit_xml::XmlSchema {
            model: &rm_model,
            schema: &rm,
            prelude: "openehr_rm::prelude",
        },
    ];
    let mut unmatched = Vec::new();
    let body = emit_xml::emit_file(&schemas, &v1, &mut unmatched)?;
    let impls_path = gen_dir.join("impls.rs");
    std::fs::write(&impls_path, &body)?;

    let mod_rs = "// @generated by openehr-codegen (emit-xml) — DO NOT EDIT.\n\
        //! Canonical-XML `ToXml`/`FromXml` impls for the RM/BASE spec types.\n\n\
        mod impls;\n";
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&mod_path, mod_rs)?;

    let written = vec![impls_path, mod_path];
    rustfmt(&written)?;
    println!(
        "emitted {} files into {} ({} XSD-only elements without a BMM field skipped)",
        written.len(),
        gen_dir.display(),
        unmatched.len()
    );
    Ok(())
}

/// Emit the OPT 1.4 model (`opt14`): typed Rust types + canonical-XML
/// `ToXml`/`FromXml` for `OPERATIONAL_TEMPLATE`, generated from the AM/OPT
/// constraint XSD closure (`Template.xsd` + includes). RM instance types
/// resolve to the already-generated `openehr-base`/`openehr-rm` impls.
fn cmd_emit_opt() -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let base_model = Model::merged(&[&base]);
    let rm_model = Model::merged(&[&base, &rm]);
    let base_specs = emit::emittable_specs(&base_model, &base);
    let rm_specs = emit::emittable_specs(&rm_model, &rm);

    // The AM/OPT constraint schemas share `Resource.xsd`+`BaseTypes.xsd` with the
    // RM-instance set; those shared types resolve to the RM/BASE XML impls.
    let xsd = xsd::XsdModel::parse_files(&xsd::am_files_v1(Path::new(XSD_V1_DIR)))?;
    let model = emit_opt::OptModel::new(&xsd, &base_specs, &rm_specs);

    let gen_dir = Path::new(ITS_ROOT).join("src/opt14");
    std::fs::create_dir_all(&gen_dir)?;

    let mut unmatched = Vec::new();
    let types_path = gen_dir.join("types.rs");
    let impls_path = gen_dir.join("impls.rs");
    let mod_path = gen_dir.join("mod.rs");
    std::fs::write(&types_path, model.emit_types())?;
    std::fs::write(&impls_path, model.emit_impls(&mut unmatched))?;
    std::fs::write(&mod_path, emit_opt::OptModel::emit_mod())?;

    let written = vec![types_path, impls_path, mod_path];
    rustfmt(&written)?;
    println!(
        "emitted {} files into {} ({} XSD-only elements without a matching field skipped)",
        written.len(),
        gen_dir.display(),
        unmatched.len()
    );
    Ok(())
}

/// Emit the static RM attribute/type model (`openehr-rm/src/model/`) — the AQL
/// planner's spec-pinned oracle. Generated from the same
/// BASE + RM BMM `emit` consumes. Writes the `model/` subtree in place (does not
/// touch the generated spec files) and declares `pub mod model;` in `lib.rs` if
/// absent, so it is correct run standalone; `emit` produces the identical output.
fn cmd_emit_rm_model() -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let rm_model = Model::merged(&[&base, &rm]);
    let files = emit_rm_model::emit_files(&rm_model);

    let src = crates_root().join("openehr-rm").join("src");
    let mut written = Vec::new();
    for f in &files {
        let full = src.join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, &f.body)?;
        written.push(full);
    }
    // `emit` is the usual authority for lib.rs; ensure the module is declared so
    // the standalone target is correct + byte-identical to `emit`'s output.
    let lib = src.join("lib.rs");
    if lib.exists() {
        let mut body = std::fs::read_to_string(&lib)?;
        if !body.contains("pub mod model;") {
            body.push_str("pub mod model;\n");
            std::fs::write(&lib, &body)?;
            written.push(lib);
        }
    }
    rustfmt(&written)?;
    println!(
        "emitted {} files into {}",
        files.len(),
        src.join("model").display()
    );
    Ok(())
}

/// Append the generated RM-model files to `openehr-rm`'s file set and declare the
/// module in its `lib.rs` (the authority for the crate layout).
fn inject_rm_model(files: &mut Vec<emit::GenFile>, mut model_files: Vec<emit::GenFile>) {
    for f in files.iter_mut() {
        if f.path == "lib.rs" && !f.body.contains("pub mod model;") {
            f.body.push_str("pub mod model;\n");
        }
    }
    files.append(&mut model_files);
}

/// Diagnostic: parse the vendored v1 RM-instance XSDs and print a summary +
/// a couple of flattened views, to validate the XSD reader.
fn cmd_check_xsd() -> Result<(), Box<dyn std::error::Error>> {
    let files = xsd::v1_files(Path::new(XSD_V1_DIR));
    let model = xsd::XsdModel::parse_files(&files)?;
    println!(
        "✓ v1 XSDs: namespace={} types={}",
        model.namespace,
        model.types.len()
    );
    let abstract_n = model.types.values().filter(|t| t.is_abstract).count();
    println!("  abstract={abstract_n}");
    for probe in ["ELEMENT", "DV_CODED_TEXT", "COMPOSITION"] {
        if let Some(t) = model.types.get(probe) {
            let (attrs, elems) = model.flattened(probe);
            println!(
                "  {probe}: base={:?} attrs=[{}] elems=[{}]",
                t.base,
                attrs
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
                elems
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
    }
    let dv = model.descendants("DATA_VALUE");
    println!(
        "  DATA_VALUE concrete descendants ({}): {}",
        dv.len(),
        dv.join(",")
    );
    Ok(())
}

const BASE_DOC: &str = "openEHR BASE (foundation + base types), generated from the BMM meta-model.";

fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("crates")
}

const AM_DOC: &str = "openEHR AM (Archetype Model): am14 (AM 1.4.0, for ADL 1.4) and am24 \
    (AM 2.4.0, for ADL 2) — both generated from BMM. Both ADL versions are in use.";

const RM_DOC: &str = "openEHR RM (Reference Model), generated from the BMM meta-model.";
const TERM_DOC: &str = "openEHR TERM (Terminology) data model, generated from the BMM \
    meta-model. The vendored terminology XML content lives in `assets/` (data, not \
    generated); an XML→model loader is added when composition validation needs it.";
const LANG_DOC: &str = "openEHR LANG: the BMM / P_BMM object model, generated from the BMM \
    meta-model. The generator's own BMM reader lives in openehr-codegen (tooling, not spec); \
    the runtime ODIN and EL parsers are future hand-written work (P8/P9).";

fn cmd_emit(_outdir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    let term = load(TERM_BMM)?;

    // openehr-base: single version, no dependency crates.
    let base_model = Model::merged(&[&base]);
    let no_ext = External::default();
    write_crate(
        "openehr-base",
        &emit::emit_crate(&base_model, &base, &no_ext, BASE_DOC),
    )?;

    // Types exported by openehr-base — downstream crates resolve references to
    // these against `openehr_base::prelude` instead of degrading to Value.
    let base_specs = emit::emittable_specs(&base_model, &base);
    let ext_base = External::default().with(base_specs, "openehr_base::prelude");

    // openehr-rm: single version, depends on openehr-base. Also carries the
    // static RM attribute/type model, emitted
    // here too so a plain `emit` keeps the crate self-consistent (lib.rs declares
    // `model`, and a later `emit` regenerates it byte-identically to the
    // standalone `emit-rm-model` target).
    let rm_model = Model::merged(&[&base, &rm]);
    let mut rm_files = emit::emit_crate(&rm_model, &rm, &ext_base, RM_DOC);
    inject_rm_model(&mut rm_files, emit_rm_model::emit_files(&rm_model));
    write_crate("openehr-rm", &rm_files)?;

    // openehr-lang: the BMM/P_BMM object model (86 classes), fully generated.
    // The generator's own reader lives here in `openehr-codegen`, so there is no
    // bootstrap cycle. The runtime ODIN/EL parsers are future hand-written work.
    // Emitted before AM because AM's rule model references LANG types
    // (`ARCHETYPE.rules : List<STATEMENT_SET>`, `ARCHETYPE_SLOT.includes :
    // List<ASSERTION>`), so AM resolves them against `openehr_lang::prelude`.
    let lang = load(LANG_BMM)?.combined(&load(LANG_BMM3)?);
    let lang_model = Model::merged(&[&base, &lang]);
    write_crate(
        "openehr-lang",
        &emit::emit_crate(&lang_model, &lang, &ext_base, LANG_DOC),
    )?;
    let lang_specs = emit::emittable_specs(&lang_model, &lang);
    let ext_base_lang = External::default()
        .with(
            emit::emittable_specs(&base_model, &base),
            "openehr_base::prelude",
        )
        .with(lang_specs, "openehr_lang::prelude");

    // openehr-am: two versions in one crate, each depending on openehr-base and
    // openehr-lang. Each version merges BASE so its ancestors (e.g. ARCHETYPE ←
    // AUTHORED_RESOURCE) resolve; the two are kept in separate models because
    // AM 1.4 and 2.4 share class names.
    // AM 2.4's `rules` package declares subtypes of LANG's beom expression
    // classes (EXPR_ARCHETYPE_REF ⊂ EXPR_VALUE_REF, EXPR_CONSTRAINT ⊂ EXPR_LEAF).
    // Per `.claude/rules/codegen.md`, that cross-`includes` extension is re-opened
    // at the DOWNSTREAM crate: the reachable beom expression/statement closure is
    // re-emitted into `openehr-am` as crate-local types (an extender-level enum
    // set composing the LANG variants + the AM leaves), so `ARCHETYPE.rules` /
    // `ARCHETYPE_SLOT.includes` resolve against the AM-level types. openehr-lang
    // stays byte-identical (the closure is emitted only here, never upstream).
    let am14 = load(AM14_BMM)?;
    let am24 = load(AM24_BMM)?;
    let m14 = Model::merged(&[&base, &am14]);
    // The AM 2.4 model merges the full LANG include-closure (`lang` = the beom
    // expression/statement object model + BMM3/EL), matching the AM BMM's
    // `includes: openehr_lang_1.1.0`, so every ancestor/descendant of the AM
    // `rules` leaves resolves. `cross_schema_reemit` then computes the COMPLETE
    // set of upstream classes whose Rust form widens downstream and grafts them
    // into the AM schema at their source package paths — a full, non-minimal
    // re-emission (owner ruling 2026-07-19). AM 1.4 declares no cross-`include`
    // subtypes → empty closure, unchanged emission.
    let m24 = Model::merged(&[&base, &lang, &am24]);
    let reemit24 = emit::cross_schema_reemit(&m24, &am24);
    let am24_aug = emit::augment_with_reemit(&am24, &m24, &reemit24, &[&base, &lang]);
    let am_files = emit::emit_multi_crate(
        &[("am14", &m14, &am14), ("am24", &m24, &am24_aug)],
        &ext_base_lang,
        AM_DOC,
    );
    write_crate("openehr-am", &am_files)?;

    // openehr-term: the TERM data model (CODE_SET, TERMINOLOGY, …), depends on
    // openehr-base (TERMINOLOGY.date : Iso8601_date). The vendored terminology
    // XML in `assets/` is data (outside `src/`, survives regen).
    let term_model = Model::merged(&[&base, &term]);
    write_crate(
        "openehr-term",
        &emit::emit_crate(&term_model, &term, &ext_base, TERM_DOC),
    )?;
    Ok(())
}

/// Write a generated crate's `src/` in place. Wipes the crate's `src/` first
/// (there is no hand-written `*_impl.rs` yet; when there is, this must preserve
/// it).
fn write_crate(
    crate_name: &str,
    files: &[emit::GenFile],
) -> Result<(), Box<dyn std::error::Error>> {
    let src = crates_root().join(crate_name).join("src");
    // Preserve hand-written code: delete only previously-`@generated`
    // files, never the hand-written `*_impl.rs` / spec-behaviour modules beside
    // them. (A stale generated file no longer emitted this run is `@generated` →
    // removed; a hand-written sibling is kept.)
    if src.exists() {
        remove_generated_files(&src)?;
    }
    std::fs::create_dir_all(&src)?;
    let mut written = Vec::with_capacity(files.len());
    for f in files {
        let full = src.join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, &f.body)?;
        written.push(full);
    }
    // Weave hand-written modules into the generated tree: any hand-written `.rs`
    // beside a generated `mod.rs` (or at the crate root beside `lib.rs`) is
    // declared `pub mod <name>;` so the hand-written `*_impl.rs` files compile without the
    // generator owning them. Deterministic (sorted scan) → drift-check-stable.
    declare_hand_written_modules(&src, &mut written)?;
    rustfmt(&written)?;
    println!("emitted {} files into {}", files.len(), src.display());
    Ok(())
}

/// Whether a file's first line marks it as generated (`// @generated …`).
fn is_generated_file(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .is_ok_and(|s| s.lines().next().is_some_and(|l| l.contains("@generated")))
}

/// Recursively delete every `@generated` file under `dir` (leaving hand-written
/// files + their directories in place), then prune directories left empty.
fn remove_generated_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            remove_generated_files(&path)?;
            if std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
            }
        } else if is_generated_file(&path) {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// For each directory whose module anchor (`lib.rs` at the crate root, else
/// `mod.rs`) is **generated**, append `pub mod <name>;` for every hand-written
/// child module not already declared — both hand-written `.rs` files
/// (`foo_impl.rs`) and hand-written module directories (a subdir whose own
/// `mod.rs` is hand-written, e.g. `odin/`). A hand-written anchor is left
/// untouched: it manages its own submodules (so we never duplicate an `odin/`
/// mod.rs's private `mod lexer;`). Modified anchors are added to `written` for
/// rustfmt.
fn declare_hand_written_modules(
    src: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // (dir, module-anchor file) pairs: the root anchors on lib.rs, each subdir on
    // its mod.rs. Collect all dirs first (sorted) for deterministic output.
    let mut dirs = vec![src.to_path_buf()];
    collect_dirs(src, &mut dirs)?;
    dirs.sort();
    for dir in dirs {
        let anchor = if dir == src {
            dir.join("lib.rs")
        } else {
            dir.join("mod.rs")
        };
        // Only extend a GENERATED anchor. A hand-written `mod.rs` (a hand-written
        // module directory) owns its own `mod`/`pub mod` declarations, so
        // appending here would duplicate them. The crate-root `lib.rs` is always
        // (re)generated by `emit_lib` — its `@generated` marker sits on a later
        // line (line 1 is the crate doc), so recognise it as generated directly.
        let anchor_generated = dir == src || is_generated_file(&anchor);
        if !anchor.exists() || !anchor_generated {
            continue;
        }
        let mut hand_written: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let p = entry?.path();
            let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if p.is_dir() {
                // A hand-written module directory: it has a `mod.rs` that is not
                // generated. A fully-generated subpackage's `mod.rs` is already
                // declared by the parent's generated module tree, so skip it.
                let child_mod = p.join("mod.rs");
                if child_mod.exists() && !is_generated_file(&child_mod) {
                    hand_written.push(stem.to_owned());
                }
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs")
                && !matches!(stem, "mod" | "lib" | "prelude")
                && !is_generated_file(&p)
            {
                hand_written.push(stem.to_owned());
            }
        }
        if hand_written.is_empty() {
            continue;
        }
        hand_written.sort();
        let mut body = std::fs::read_to_string(&anchor)?;
        let mut appended = false;
        for m in hand_written {
            let decl = format!("pub mod {m};");
            if !body.contains(&decl) {
                if !appended {
                    body.push_str("\n// hand-written modules (spec behaviour), auto-declared:\n");
                    appended = true;
                }
                body.push_str(&decl);
                body.push('\n');
            }
        }
        if appended {
            std::fs::write(&anchor, &body)?;
            written.push(anchor);
        }
    }
    Ok(())
}

/// Recursively collect subdirectories of `dir` into `out`.
fn collect_dirs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            out.push(path.clone());
            collect_dirs(&path, out)?;
        }
    }
    Ok(())
}

/// Run `rustfmt` over the generated files so the emitted output is exactly what
/// `cargo fmt --all --check` expects (line wrapping, empty `{}`, import order,
/// …), instead of the emitter having to reproduce every rustfmt rule by hand.
fn rustfmt(files: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    if files.is_empty() {
        return Ok(());
    }
    let status = std::process::Command::new("rustfmt")
        .args(["--edition", "2024", "--quiet"])
        .args(files)
        .status()?;
    if !status.success() {
        return Err(format!("rustfmt failed with status {status}").into());
    }
    Ok(())
}
