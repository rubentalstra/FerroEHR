//! The CNF 2.0 reference runner CLI.
//!
//! ```text
//! cnf-runner emit-schemas --out DIR     write the published JSON-Schema set
//! cnf-runner validate --root DIR [--specs DIR]
//! cnf-runner compare-ecc --root DIR --ecc-catalog TSV --map YAML --out REPORT.md
//! cnf-runner run --root DIR --ixit FILE --out DIR [--sut-name N] [--sut-version V]
//!                                       validate an artifact tree (all gates);
//!                                       --specs enables the SM/spec-ref
//!                                       resolution checks against the vendored
//!                                       spec tree (docs/specs/openehr)
//! cnf-runner verdicts --statement F --results F --root DIR --out DIR
//!                                       compute the verdicts (pure pipeline)
//!                                       and write the report/statement/
//!                                       certificate + verdicts.json
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
use cnf_runner::compare;
use cnf_runner::load::compile_schema;
use cnf_runner::party::{Results, Statement};
use cnf_runner::render::{render_certificate, render_report, render_statement};
use cnf_runner::schema::{emit_all, render, results_schema, statement_schema};
use cnf_runner::validate::{Context, validate};
use cnf_runner::verdict::compute;

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
    /// Generate the committed ECC↔CNF comparison report from the
    /// hand-adjudicated map; exit 1 while the gate is open.
    CompareEcc {
        /// The artifact root.
        #[arg(long)]
        root: PathBuf,
        /// The old harness's catalogue TSV.
        #[arg(long)]
        ecc_catalog: PathBuf,
        /// The hand-adjudicated ECC→CNF map (YAML).
        #[arg(long)]
        map: PathBuf,
        /// Where to write the generated report (Markdown).
        #[arg(long)]
        out: PathBuf,
    },
    /// Execute the catalogue against a live SUT (from its ixit topology)
    /// and emit results.json + the run report.
    Run {
        /// The artifact root.
        #[arg(long)]
        root: PathBuf,
        /// The ixit topology file (JSON).
        #[arg(long)]
        ixit: PathBuf,
        /// Output directory for results.json + the run summary.
        #[arg(long)]
        out: PathBuf,
        /// SUT display name.
        #[arg(long, default_value = "ehrbase-rs")]
        sut_name: String,
        /// SUT version label.
        #[arg(long, default_value = "dev")]
        sut_version: String,
        /// Only run cases whose id contains this substring.
        #[arg(long)]
        filter: Option<String>,
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
    /// Compute the verdicts from a statement + results against an artifact
    /// tree (the pure pipeline) and write the rendered submission documents.
    Verdicts {
        /// The party statement (`statement.json`).
        #[arg(long)]
        statement: PathBuf,
        /// The party results (`results.json`).
        #[arg(long)]
        results: PathBuf,
        /// The artifact root (schedule/, vocab/, registers/).
        #[arg(long)]
        root: PathBuf,
        /// Output directory for the rendered documents + verdicts.json.
        #[arg(long)]
        out: PathBuf,
    },
}

/// Load one JSON party artifact, validating it against its emitted schema
/// before typed parsing.
fn load_party_json<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
    schema: &serde_json::Value,
    schema_name: &str,
) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: JSON: {e}", path.display()))?;
    let validator = compile_schema(schema, schema_name).map_err(|e| e.to_string())?;
    let violations: Vec<String> = validator
        .iter_errors(&value)
        .map(|e| format!("{}: {e}", e.instance_path()))
        .collect();
    if !violations.is_empty() {
        return Err(format!(
            "{}: schema: {}",
            path.display(),
            violations.join("; ")
        ));
    }
    serde_json::from_value(value).map_err(|e| format!("{}: model: {e}", path.display()))
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
        Command::CompareEcc {
            root,
            ecc_catalog,
            map,
            out,
        } => run_compare_ecc(&root, &ecc_catalog, &map, &out),
        Command::Run {
            root,
            ixit,
            out,
            sut_name,
            sut_version,
            filter,
        } => run_command(
            &root,
            &ixit,
            &out,
            &sut_name,
            &sut_version,
            filter.as_deref(),
        ),
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
        Command::Verdicts {
            statement,
            results,
            root,
            out,
        } => run_verdicts(&statement, &results, &root, &out),
    }
}

fn run_compare_ecc(
    root: &std::path::Path,
    ecc_catalog: &std::path::Path,
    map: &std::path::Path,
    out: &std::path::Path,
) -> ExitCode {
    let loaded = match load_root(root) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("runner defect: {e}");
            return ExitCode::from(2);
        }
    };
    if !loaded.errors.is_empty() {
        for e in &loaded.errors {
            eprintln!("{e}");
        }
        return ExitCode::from(2);
    }
    match compare::run(ecc_catalog, map, &loaded.set) {
        Ok((cmp, report)) => {
            if let Err(e) = std::fs::write(out, report) {
                eprintln!("cannot write {}: {e}", out.display());
                return ExitCode::from(2);
            }
            println!(
                "wrote {} — mapped {} · unmapped {} · gate {}",
                out.display(),
                cmp.mapped.len(),
                cmp.unmapped.len(),
                if cmp.gate_clean() { "clean" } else { "OPEN" }
            );
            if cmp.gate_clean() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("comparison failed: {e}");
            ExitCode::from(2)
        }
    }
}

fn run_verdicts(
    statement_path: &std::path::Path,
    results_path: &std::path::Path,
    root: &std::path::Path,
    out: &std::path::Path,
) -> ExitCode {
    let statement: Statement =
        match load_party_json(statement_path, &statement_schema(), "statement.schema.json") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
    let results: Results =
        match load_party_json(results_path, &results_schema(), "results.schema.json") {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
    if let Err(errors) = results.check_invariants() {
        for e in &errors {
            eprintln!("{e}");
        }
        return ExitCode::from(2);
    }

    let loaded = match load_root(root) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("runner defect: {e}");
            return ExitCode::from(2);
        }
    };
    if !loaded.errors.is_empty() {
        for e in &loaded.errors {
            eprintln!("{e}");
        }
        return ExitCode::from(2);
    }
    let Some((_, matrix)) = &loaded.set.matrix else {
        eprintln!("artifact tree carries no capability matrix");
        return ExitCode::from(2);
    };
    let Some((_, register)) = &loaded.set.register else {
        eprintln!("artifact tree carries no ambiguity register");
        return ExitCode::from(2);
    };
    let cases: Vec<_> = loaded.set.cases.iter().map(|(_, c)| c.clone()).collect();

    let report = compute(&statement, &results, &cases, matrix, register);

    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("cannot create {}: {e}", out.display());
        return ExitCode::from(2);
    }
    let artifacts: [(&str, String); 4] = [
        (
            "verdicts.json",
            match serde_json::to_string_pretty(&report) {
                Ok(mut json) => {
                    json.push('\n');
                    json
                }
                Err(e) => {
                    eprintln!("cannot serialize verdicts: {e}");
                    return ExitCode::from(2);
                }
            },
        ),
        ("CONFORMANCE_REPORT.md", render_report(&results, &report)),
        (
            "CONFORMANCE_STATEMENT.md",
            render_statement(&statement, &report),
        ),
        (
            "CONFORMANCE_CERTIFICATE.md",
            render_certificate(&statement, &results, &report, matrix),
        ),
    ];
    for (name, body) in &artifacts {
        let path = out.join(name);
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("cannot write {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", path.display());
    }

    for finding in &report.review {
        println!("static-review: {}", finding.message);
    }
    println!(
        "{} capability verdict(s), {} of {} cases driven, {} review finding(s)",
        report.capabilities.len(),
        report.coverage.driven,
        report.coverage.selected,
        report.review.len(),
    );
    if report.review.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// The live-run handler: load, execute, emit results.json + summary.
#[allow(clippy::too_many_lines)] // the one-shot orchestration seam
fn run_command(
    root: &std::path::Path,
    ixit_path: &std::path::Path,
    out: &std::path::Path,
    sut_name: &str,
    sut_version: &str,
    filter: Option<&str>,
) -> ExitCode {
    let loaded = match load_root(root) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("runner defect: {e}");
            return ExitCode::from(2);
        }
    };
    if !loaded.errors.is_empty() {
        for e in &loaded.errors {
            eprintln!("{e}");
        }
        return ExitCode::from(2);
    }
    let ixit_text = match std::fs::read_to_string(ixit_path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("cannot read {}: {e}", ixit_path.display());
            return ExitCode::from(2);
        }
    };
    let ixit: cnf_runner::ixit::Ixit = match serde_json::from_str(&ixit_text) {
        Ok(ixit) => ixit,
        Err(e) => {
            eprintln!("ixit: {e}");
            return ExitCode::from(2);
        }
    };
    let mut set = loaded.set;
    if let Some(needle) = filter {
        set.cases.retain(|(_, c)| c.id.as_str().contains(needle));
    }
    let report = match cnf_runner::run::execute(&set, &ixit) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("execution defect: {e}");
            return ExitCode::from(2);
        }
    };
    let outcomes: Vec<cnf_runner::party::OutcomeRecord> = report
        .records
        .iter()
        .map(cnf_runner::party::OutcomeRecord::from)
        .collect();
    let (passed, failed, errored, na) = outcomes.iter().fold((0, 0, 0, 0), |acc, o| {
        use cnf_runner::party::OutcomeStatus;
        match o.status {
            OutcomeStatus::Passed => (acc.0 + 1, acc.1, acc.2, acc.3),
            OutcomeStatus::Failed => (acc.0, acc.1 + 1, acc.2, acc.3),
            OutcomeStatus::Errored => (acc.0, acc.1, acc.2 + 1, acc.3),
            _ => (acc.0, acc.1, acc.2, acc.3 + 1),
        }
    });
    let ixit_digest = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ixit_text.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };
    let results = cnf_runner::party::Results {
        sut: cnf_runner::party::Sut {
            name: sut_name.to_owned(),
            version: sut_version.to_owned(),
        },
        runner: cnf_runner::party::Runner {
            name: "cnf-runner".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            verification_pack_status: cnf_runner::party::VerificationPackStatus::Passed,
        },
        schedule_release: "cnf-2.0-w2".to_owned(),
        tech_profile: cnf_runner::party::TechProfile {
            its: cnf_runner::vocab::ItsName::ItsRest,
            formats: vec![cnf_runner::vocab::FormatName::CanonicalJson],
        },
        ixit_digest,
        outcomes,
        measurements: Vec::new(),
        ambiguity_dispositions: Vec::new(),
    };
    if let Err(errors) = results.check_invariants() {
        for e in errors {
            eprintln!("results invariant: {e}");
        }
        return ExitCode::from(2);
    }
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("cannot create {}: {e}", out.display());
        return ExitCode::from(2);
    }
    let results_path = out.join("results.json");
    match serde_json::to_string_pretty(&results) {
        Ok(mut text) => {
            text.push('\n');
            if let Err(e) = std::fs::write(&results_path, text) {
                eprintln!("cannot write {}: {e}", results_path.display());
                return ExitCode::from(2);
            }
        }
        Err(e) => {
            eprintln!("serialize: {e}");
            return ExitCode::from(2);
        }
    }
    let exceptions_path = out.join("run-exceptions.json");
    if let Ok(mut text) = serde_json::to_string_pretty(
        &report
            .exceptions
            .iter()
            .map(|(case, e)| serde_json::json!({ "case": case.to_string(), "exception": e }))
            .collect::<Vec<_>>(),
    ) {
        text.push('\n');
        let _write = std::fs::write(&exceptions_path, text);
    }
    println!(
        "{} case-records: {passed} passed / {failed} failed / {errored} errored / {na} n-a; interpreter coverage {:.1}% ({} exceptions); wrote {}",
        report.records.len(),
        report.interpreter_coverage() * 100.0,
        report.exceptions.len(),
        results_path.display()
    );
    if failed == 0 && errored == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
