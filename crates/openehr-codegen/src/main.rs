#![allow(clippy::format_push_string, clippy::too_many_lines)]

//! `openehr-codegen` — generates the openEHR spec crates from the vendored BMM
//! meta-model (ADR-004).
//!
//! Usage:
//!   `openehr-codegen check`          — load + validate the vendored BMM schemas.
//!   `openehr-codegen emit [OUTDIR]`  — emit Rust into OUTDIR (default:
//!                                       `target/codegen-preview`).

mod emit;
mod naming;

use emit::Model;
use openehr_lang::bmm::BmmSchema;
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

fn cmd_emit(_outdir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;

    // openehr-base: single version.
    let base_model = Model::merged(&[&base, &rm]);
    write_crate(
        "openehr-base",
        &emit::emit_crate(&base_model, &base, BASE_DOC),
    )?;

    // openehr-am: two versions in one crate. Each version merges BASE so its
    // ancestors (e.g. ARCHETYPE ← AUTHORED_RESOURCE) resolve; they are kept in
    // separate models because AM 1.4 and 2.4 share class names.
    let am14 = load(AM14_BMM)?;
    let am24 = load(AM24_BMM)?;
    let m14 = Model::merged(&[&base, &am14]);
    let m24 = Model::merged(&[&base, &am24]);
    let am_files = emit::emit_multi_crate(&[("am14", &m14, &am14), ("am24", &m24, &am24)], AM_DOC);
    write_crate("openehr-am", &am_files)?;
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
    for f in files {
        let full = src.join(&f.path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, &f.body)?;
    }
    println!("emitted {} files into {}", files.len(), src.display());
    Ok(())
}
