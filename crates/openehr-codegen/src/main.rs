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
use std::collections::BTreeMap;
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

const BASE_BMM: &str = "openehr_base_1.2.0.bmm.json";
const RM_BMM: &str = "openehr_rm_1.1.0.bmm.json";
const TERM_BMM: &str = "openehr_term_3.0.0.bmm.json";

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

fn cmd_emit(outdir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let out = outdir.unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/codegen-preview")
            .clone()
    });

    // Start clean so stale files from a previous layout never linger.
    if out.exists() {
        std::fs::remove_dir_all(&out)?;
    }

    let base = load(BASE_BMM)?;
    let rm = load(RM_BMM)?;
    // BASE first so RM overrides on any name collision.
    let model = Model::merged(&[&base, &rm]);

    let mut total = 0;
    let mut per_crate: BTreeMap<&str, usize> = BTreeMap::new();
    for (crate_name, schema) in [("openehr-base", &base), ("openehr-rm", &rm)] {
        let files = emit::emit_schema(&model, schema);
        for f in &files {
            let full = out.join(crate_name).join("src").join(&f.path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&full, &f.body)?;
        }
        *per_crate.entry(crate_name).or_default() += files.len();
        total += files.len();
    }

    println!("emitted {total} files to {}", out.display());
    for (k, v) in &per_crate {
        println!("  {k}: {v} files");
    }
    // Dump two representative files so quality is visible in the log.
    for sample in [
        "openehr-rm/src/quantity/dv_quantity.rs",
        "openehr-rm/src/data_types/data_value.rs",
    ] {
        let p = out.join(sample);
        if let Ok(txt) = std::fs::read_to_string(&p) {
            println!("\n──────── {sample} ────────\n{txt}");
        }
    }
    Ok(())
}
