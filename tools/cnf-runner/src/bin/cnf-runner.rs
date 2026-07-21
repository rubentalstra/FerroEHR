//! The CNF 2.0 reference runner CLI.
//!
//! ```text
//! cnf-runner emit-schemas --out DIR     write the published JSON-Schema set
//! cnf-runner validate --root DIR [--specs DIR]
//!                                       validate an artifact tree (all gates);
//!                                       --specs enables the SM/spec-ref
//!                                       resolution checks against the vendored
//!                                       spec tree (docs/specs/openehr)
//! ```
//!
//! Exit codes: `0` clean · `1` findings · `2` runner error.
// Verification CLI: progress/diagnostics on the console ARE this tool's user
// interface — the reliability deny-tier for shipped code deliberately relaxes
// stdio here (.claude/rules/reliability.md §tools).
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use cnf_runner::artifacts::load_root;
use cnf_runner::schema::{emit_all, render};
use cnf_runner::validate::{Context, validate};

#[derive(Parser)]
#[command(name = "cnf-runner", about = "CNF 2.0 reference runner", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write the published JSON-Schema set (byte-deterministic).
    EmitSchemas {
        /// Output directory (created if missing).
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate one artifact tree through every machine gate.
    Validate {
        /// The artifact root (schedule/, bindings/, vocab/, corpus/, registers/).
        #[arg(long)]
        root: PathBuf,
        /// The vendored openEHR spec tree; enables SM-operation and spec-ref
        /// resolution.
        #[arg(long)]
        specs: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::EmitSchemas { out } => {
            if let Err(e) = std::fs::create_dir_all(&out) {
                eprintln!("cannot create {}: {e}", out.display());
                return ExitCode::from(2);
            }
            for (name, schema) in emit_all() {
                let path = out.join(name);
                if let Err(e) = std::fs::write(&path, render(&schema)) {
                    eprintln!("cannot write {}: {e}", path.display());
                    return ExitCode::from(2);
                }
                println!("wrote {}", path.display());
            }
            ExitCode::SUCCESS
        }
        Command::Validate { root, specs } => {
            let loaded = match load_root(&root) {
                Ok(loaded) => loaded,
                Err(e) => {
                    eprintln!("runner defect: {e}");
                    return ExitCode::from(2);
                }
            };
            let findings = validate(&Context {
                set: &loaded.set,
                load_errors: &loaded.errors,
                spec_root: specs.as_deref(),
            });
            for finding in &findings {
                println!("{finding}");
            }
            println!(
                "{} case(s), {} binding(s), {} finding(s)",
                loaded.set.cases.len(),
                loaded.set.bindings.len(),
                findings.len()
            );
            if findings.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
    }
}
