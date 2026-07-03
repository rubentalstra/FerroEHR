#![allow(clippy::format_push_string, clippy::too_many_lines)]

//! `openehr-codegen` — generates the openEHR spec crates from the vendored BMM
//! meta-model (ADR-004).
//!
//! Usage:
//!   `openehr-codegen check`          — load + validate the vendored BMM schemas.
//!   `openehr-codegen emit [OUTDIR]`  — emit Rust into OUTDIR (default:
//!                                       `target/codegen-preview`).

mod bmm;
mod emit;
mod naming;

use bmm::BmmSchema;
use emit::{External, Model};
use std::path::{Path, PathBuf};

const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/bmm");

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map_or("check", String::as_str);
    let result = match cmd {
        "check" => cmd_check(),
        "emit" => cmd_emit(args.get(1).map(PathBuf::from)),
        other => {
            eprintln!("unknown command {other:?}; use `check` or `emit [OUTDIR]`");
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

const BASE_BMM: &str = "openehr_base_1.3.0.bmm.json";
const RM_BMM: &str = "openehr_rm_1.2.0.bmm.json";
const TERM_BMM: &str = "openehr_term_3.1.0.bmm.json";
const AM14_BMM: &str = "openehr_am_1.4.0.bmm.json";
const AM24_BMM: &str = "openehr_am_2.4.0.bmm.json";
const LANG_BMM: &str = "openehr_lang_1.1.0.bmm.json";
/// LANG's model spans two vendored files: the primary one above (persisted BMM
/// with `EXPR_*` and `STATEMENT_SET`/`ASSERTION`, which AM's rules/slots
/// reference) and this BMM-3 file (the full `BMM_*` object model with the
/// `EL_*` expression language, which AM's persisted-archetype rules reference).
/// Both are merged into the `openehr-lang` crate.
const LANG_BMM3: &str = "openehr_lang_1.1.0-bmm3.bmm.json";

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

    // openehr-rm: single version, depends on openehr-base.
    let rm_model = Model::merged(&[&base, &rm]);
    write_crate(
        "openehr-rm",
        &emit::emit_crate(&rm_model, &rm, &ext_base, RM_DOC),
    )?;

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
    let am14 = load(AM14_BMM)?;
    let am24 = load(AM24_BMM)?;
    let m14 = Model::merged(&[&base, &am14]);
    let m24 = Model::merged(&[&base, &am24]);
    let am_files = emit::emit_multi_crate(
        &[("am14", &m14, &am14), ("am24", &m24, &am24)],
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
    if src.exists() {
        std::fs::remove_dir_all(&src)?;
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
    rustfmt(&written)?;
    println!("emitted {} files into {}", files.len(), src.display());
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
