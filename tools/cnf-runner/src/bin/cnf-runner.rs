//! The CNF 2.0 reference runner CLI.
//!
//! ```text
//! cnf-runner emit-schemas --out DIR     write the published JSON-Schema set
//! cnf-runner validate --root DIR [--specs DIR]
//! cnf-runner compare-ecc --root DIR --ecc-catalog TSV --map YAML --out REPORT.md
//! cnf-runner run --root DIR --ixit FILE --out DIR [--sut-name N] [--sut-version V] [--statement F]
//!                                       validate an artifact tree (all gates);
//!                                       --specs enables the SM/spec-ref
//!                                       resolution checks against the vendored
//!                                       spec tree (docs/specs/openehr)
//! cnf-runner verdicts --statement F --results F --root DIR --out DIR
//!                                       compute the verdicts (pure pipeline)
//!                                       and write the report/statement/
//!                                       certificate + verdicts.json
//! cnf-runner perf --root DIR --ixit FILE --results FILE --class POC|S|L|R
//!                 [--hours 1|2|4|6|8|12] [--skip-seed] [--seed-workers N]
//!                                       the measured class run (conformance-
//!                                       by-measurement): seed the scale
//!                                       corpus, hold the class's offered
//!                                       load for the sustained window, merge
//!                                       the record into results.json
//! cnf-runner stress --root DIR --ixit FILE --out FILE
//!                   [--corpus-class POC|S|L|R] [--skip-seed]
//!                   [--step-secs N] [--bisections N] [--max-rate R]
//!                                       the step-load stress ladder to the
//!                                       maximum sustainable throughput
//!                                       (exploration only — writes
//!                                       stress.json, never results.json)
//! cnf-runner perf-assets --root DIR --results FILE --out DIR
//!                        [--summary FILE] [--stress FILE]
//!                                       render the published SVGs + summary
//!                                       FROM committed artifacts
//! cnf-runner conformance-assets --root DIR --results FILE --verdicts FILE
//!                               --out DIR [--suffix=-java]
//!                                       render the capability heat grid +
//!                                       per-chapter outcome bars FROM the
//!                                       committed party artifacts
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
        /// The party statement (ICS) — when supplied, option-gated cases
        /// whose option the ICS does not declare are recorded N/A at drive
        /// time (ISO/IEC 9646 test selection) instead of driven.
        #[arg(long)]
        statement: Option<PathBuf>,
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
    /// Execute the performance schedule's open-loop measured run(s) against
    /// a live SUT and merge the §8.10 measurement records into an existing
    /// results.json (conformance-by-measurement).
    Perf {
        /// The artifact root.
        #[arg(long)]
        root: PathBuf,
        /// The ixit topology file (JSON) — its environment block is
        /// mandatory for a measured run.
        #[arg(long)]
        ixit: PathBuf,
        /// The results.json to merge the measurement records into (written
        /// by a prior `run`).
        #[arg(long)]
        results: PathBuf,
        /// Select the performance case(s) of this class (POC | S | L | R).
        #[arg(long)]
        class: String,
        /// Skip corpus seeding and load the sidecar corpus index written by
        /// a prior seeding pass.
        #[arg(long)]
        skip_seed: bool,
        /// Parallel seeding workers.
        #[arg(long, default_value_t = 16)]
        seed_workers: usize,
        /// The sustained-window ladder: hours to hold the offered load —
        /// 1 (default, the case's normative window) | 2 | 4 | 6 | 8 | 12.
        /// A longer window is a STRICTER demonstration and persists like
        /// any measured run; nothing shorter than the case exists.
        #[arg(long, default_value_t = 1)]
        hours: u64,
    },
    /// Run the step-load STRESS instrument: geometric load steps to the
    /// maximum sustainable throughput (exploration — never a conformance
    /// record; classes are earned exclusively by the hour-long class runs).
    Stress {
        /// The artifact root.
        #[arg(long)]
        root: PathBuf,
        /// The ixit topology file (JSON) — its environment block is
        /// mandatory (a throughput number without the deployment described
        /// is meaningless).
        #[arg(long)]
        ixit: PathBuf,
        /// Where to write the stress report (stress.json).
        #[arg(long)]
        out: PathBuf,
        /// The class whose corpus + workload mix the stress runs on
        /// (POC | S | L | R) — data volume context only, no class claim.
        #[arg(long, default_value = "POC")]
        corpus_class: String,
        /// Skip corpus seeding and load the sidecar corpus index written by
        /// a prior seeding pass (shared with `perf`).
        #[arg(long)]
        skip_seed: bool,
        /// Parallel seeding workers.
        #[arg(long, default_value_t = 16)]
        seed_workers: usize,
        /// Each load step's recorded hold, seconds (short + intense by
        /// design).
        #[arg(long, default_value_t = 120)]
        step_secs: u64,
        /// Post-breach bisection refinements.
        #[arg(long, default_value_t = 3)]
        bisections: u32,
        /// The climb cap (arrivals/s).
        #[arg(long, default_value_t = 4096.0)]
        max_rate: f64,
    },
    /// Render the published performance SVG assets FROM a committed
    /// results.json (deterministic; CI regenerates and diffs — hand-drawn
    /// numbers are a build failure).
    PerfAssets {
        /// The artifact root (for the class-ladder floors).
        #[arg(long)]
        root: PathBuf,
        /// The committed results.json carrying the measurement records.
        #[arg(long)]
        results: PathBuf,
        /// Output directory for the SVG files.
        #[arg(long)]
        out: PathBuf,
        /// Also write the generated Markdown summary (class ladder +
        /// measured detail) to this path — the book's build-time include.
        #[arg(long)]
        summary: Option<PathBuf>,
        /// A committed stress report (stress.json) to render the
        /// latency-throughput curve from, when one exists.
        #[arg(long)]
        stress: Option<PathBuf>,
    },
    /// Render the conformance visuals (the capability heat grid + the
    /// per-chapter outcome bars) deterministically FROM the committed party
    /// artifacts — the perf-assets pattern for functional conformance.
    ConformanceAssets {
        /// The artifact root (for the capability matrix).
        #[arg(long)]
        root: PathBuf,
        /// The committed results.json.
        #[arg(long)]
        results: PathBuf,
        /// The committed verdicts.json.
        #[arg(long)]
        verdicts: PathBuf,
        /// Output directory for the SVG files.
        #[arg(long)]
        out: PathBuf,
        /// A suffix appended to the SVG file stems (`-java` for the
        /// comparison SUT's copies).
        #[arg(long, default_value = "")]
        suffix: String,
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
        Command::EmitSchemas { out } => emit_schemas_command(&out),
        Command::CompareEcc {
            root,
            ecc_catalog,
            map,
            out,
        } => compare_ecc_command(&root, &ecc_catalog, &map, &out),
        Command::Run {
            root,
            ixit,
            out,
            sut_name,
            sut_version,
            filter,
            statement,
        } => run_command(
            &root,
            &ixit,
            &out,
            &sut_name,
            &sut_version,
            filter.as_deref(),
            statement.as_deref(),
        ),
        Command::Validate { root, specs } => validate_command(&root, specs.as_deref()),
        Command::Perf {
            root,
            ixit,
            results,
            class,
            skip_seed,
            seed_workers,
            hours,
        } => perf_command(
            &root,
            &ixit,
            &results,
            &class,
            skip_seed,
            seed_workers,
            hours,
        ),
        Command::Stress {
            root,
            ixit,
            out,
            corpus_class,
            skip_seed,
            seed_workers,
            step_secs,
            bisections,
            max_rate,
        } => stress_command(
            &root,
            &ixit,
            &out,
            &corpus_class,
            skip_seed,
            seed_workers,
            step_secs,
            bisections,
            max_rate,
        ),
        Command::PerfAssets {
            root,
            results,
            out,
            summary,
            stress,
        } => perf_assets_command(&root, &results, &out, summary.as_deref(), stress.as_deref()),
        Command::ConformanceAssets {
            root,
            results,
            verdicts,
            out,
            suffix,
        } => conformance_assets_command(&root, &results, &verdicts, &out, &suffix),
        Command::Verdicts {
            statement,
            results,
            root,
            out,
        } => run_verdicts(&statement, &results, &root, &out),
    }
}

fn emit_schemas_command(out: &std::path::Path) -> ExitCode {
    if let Err(e) = std::fs::create_dir_all(out) {
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

fn compare_ecc_command(
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

fn validate_command(root: &std::path::Path, specs: Option<&std::path::Path>) -> ExitCode {
    let loaded = match load_root(root) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("runner defect: {e}");
            return ExitCode::from(2);
        }
    };
    let findings = validate(&Context {
        set: &loaded.set,
        load_errors: &loaded.errors,
        spec_root: specs,
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

#[allow(clippy::too_many_lines)] // the one-shot orchestration seam
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
    let perf_cases: Vec<_> = loaded
        .set
        .performance
        .iter()
        .map(|(_, c)| c.clone())
        .collect();

    let report = compute(&statement, &results, &cases, &perf_cases, matrix, register);

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

/// The conformance-asset renderer (`conformance-assets`): the capability
/// heat grid + the per-chapter outcome bars, deterministic SVGs FROM the
/// committed party artifacts (regenerate-and-diff guarded in CI).
fn conformance_assets_command(
    root: &std::path::Path,
    results_path: &std::path::Path,
    verdicts_path: &std::path::Path,
    out: &std::path::Path,
    suffix: &str,
) -> ExitCode {
    // The committed verdicts.json — only the capability evidence list is
    // the render input.
    #[derive(serde::Deserialize)]
    struct VerdictSlice {
        capabilities: Vec<(String, cnf_runner::verdict::Evidence)>,
    }

    let loaded = match load_root(root) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("runner defect: {e}");
            return ExitCode::from(2);
        }
    };
    let Some((_, matrix)) = &loaded.set.matrix else {
        eprintln!("artifact set has no capability matrix");
        return ExitCode::from(2);
    };
    let results: Results =
        match load_party_json(results_path, &results_schema(), "results.schema.json") {
            Ok(results) => results,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
    let verdicts: VerdictSlice = match std::fs::read_to_string(verdicts_path)
        .map_err(|e| format!("cannot read {}: {e}", verdicts_path.display()))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("verdicts: {e}")))
    {
        Ok(verdicts) => verdicts,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("cannot create {}: {e}", out.display());
        return ExitCode::from(2);
    }
    let sut_label = format!("{} {}", results.sut.name, results.sut.version);
    let chapters = cnf_runner::conf_assets::chapter_counts(&results);
    let chapter_refs: Vec<(&str, cnf_runner::conf_assets::ChapterCounts)> = chapters
        .iter()
        .map(|(chapter, counts)| (*chapter, counts.clone()))
        .collect();
    let assets = [
        (
            format!("conformance-heat-grid{suffix}.svg"),
            cnf_runner::conf_assets::heat_grid_svg(&sut_label, matrix, &verdicts.capabilities),
        ),
        (
            format!("conformance-chapter-bars{suffix}.svg"),
            cnf_runner::conf_assets::chapter_bars_svg(&sut_label, &chapter_refs),
        ),
    ];
    for (name, body) in &assets {
        let path = out.join(name);
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("cannot write {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", path.display());
    }
    ExitCode::SUCCESS
}

/// The asset renderer (`perf-assets`): deterministic SVGs FROM the committed
/// measurement records (regenerate-and-diff guarded in CI).
fn perf_assets_command(
    root: &std::path::Path,
    results_path: &std::path::Path,
    out: &std::path::Path,
    summary: Option<&std::path::Path>,
    stress: Option<&std::path::Path>,
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
    let results: Results =
        match load_party_json(results_path, &results_schema(), "results.schema.json") {
            Ok(results) => results,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("cannot create {}: {e}", out.display());
        return ExitCode::from(2);
    }
    let perf_cases: Vec<_> = loaded
        .set
        .performance
        .iter()
        .map(|(_, c)| c.clone())
        .collect();
    let mut files: Vec<(String, String)> = vec![(
        "perf-class-ladder.svg".to_owned(),
        cnf_runner::perf_assets::class_ladder_svg(&perf_cases, &results.measurements),
    )];
    if let Some(stress_path) = stress {
        let report: cnf_runner::stress::StressReport = match std::fs::read_to_string(stress_path)
            .map_err(|e| format!("cannot read {}: {e}", stress_path.display()))
            .and_then(|text| {
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", stress_path.display()))
            }) {
            Ok(report) => report,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        match cnf_runner::perf_assets::stress_curve_svg(&report) {
            Ok(svg) => files.push(("perf-stress-curve.svg".to_owned(), svg)),
            Err(e) => {
                eprintln!("stress curve: {e}");
                return ExitCode::from(2);
            }
        }
    }
    for measurement in &results.measurements {
        match cnf_runner::perf_assets::latency_percentiles_svg(measurement) {
            Ok(svg) => files.push((
                format!("perf-latency-class-{}.svg", measurement.class.token()),
                svg,
            )),
            Err(e) => {
                eprintln!("{}: {e}", measurement.case);
                return ExitCode::from(2);
            }
        }
    }
    for (name, body) in &files {
        let path = out.join(name);
        if let Err(e) = std::fs::write(&path, body) {
            eprintln!("cannot write {}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", path.display());
    }
    if let Some(summary_path) = summary {
        let body =
            match cnf_runner::perf_assets::summary_markdown(&perf_cases, &results.measurements) {
                Ok(body) => body,
                Err(e) => {
                    eprintln!("summary: {e}");
                    return ExitCode::from(2);
                }
            };
        if let Some(parent) = summary_path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            eprintln!("cannot create {}: {e}", parent.display());
            return ExitCode::from(2);
        }
        if let Err(e) = std::fs::write(summary_path, body) {
            eprintln!("cannot write {}: {e}", summary_path.display());
            return ExitCode::from(2);
        }
        println!("wrote {}", summary_path.display());
    }
    ExitCode::SUCCESS
}

/// Resolve the blood-pressure OPT the scale corpora commit against.
fn scale_opt_xml(loaded: &cnf_runner::artifacts::Loaded) -> Result<String, String> {
    let corpus_dir = loaded
        .set
        .corpus_dir
        .as_deref()
        .ok_or_else(|| "artifact set has no corpus directory".to_owned())?;
    let key =
        cnf_runner::ids::CorpusKey::parse("cnf.opt.blood_pressure").map_err(|e| e.to_string())?;
    let source = loaded
        .set
        .corpus
        .as_ref()
        .and_then(|(_, m)| m.get(&key))
        .and_then(|entry| entry.source.clone())
        .ok_or_else(|| "corpus manifest has no cnf.opt.blood_pressure fixture".to_owned())?;
    std::fs::read_to_string(corpus_dir.join(&source))
        .map_err(|e| format!("cannot read OPT fixture {source}: {e}"))
}

/// The journey context every measured run needs: the catalogue (loaded
/// artifact) and the CKM template pack its stages name.
fn journey_context(
    loaded: &cnf_runner::artifacts::Loaded,
) -> Result<
    (
        cnf_runner::perf::JourneyCatalogue,
        cnf_runner::perf_run::pack::JourneyPack,
    ),
    String,
> {
    let catalogue = loaded
        .set
        .journeys
        .as_ref()
        .map(|(_, catalogue)| catalogue.clone())
        .ok_or_else(|| "artifact set has no vocab/journey_catalogue.yaml".to_owned())?;
    let corpus_dir = loaded
        .set
        .corpus_dir
        .as_deref()
        .ok_or_else(|| "artifact set has no corpus directory".to_owned())?;
    let manifest = loaded
        .set
        .corpus
        .as_ref()
        .map(|(_, manifest)| manifest)
        .ok_or_else(|| "artifact set has no corpus manifest".to_owned())?;
    let pack = cnf_runner::perf_run::pack::JourneyPack::load(corpus_dir, manifest, &catalogue)?;
    Ok((catalogue, pack))
}

/// Seed the scale corpus + the standing ward (or load the sidecar index a
/// prior seeding wrote — the index lives beside `artifact_path` so `perf`
/// and `stress` share it). Ward seeding is idempotent, so a pre-journey
/// sidecar upgrades in place on the next non-skip run.
#[allow(clippy::too_many_arguments)] // the one-shot seeding seam both handlers share
fn seed_or_load_corpus(
    client: &cnf_runner::perf_run::client::PerfClient,
    corpus_key: &str,
    opt_xml: &str,
    journey_pack: &cnf_runner::perf_run::pack::JourneyPack,
    artifact_path: &std::path::Path,
    skip_seed: bool,
    seed_workers: usize,
    progress: &(dyn Fn(String) + Sync),
) -> Result<cnf_runner::perf_run::corpus::SeededCorpus, String> {
    use cnf_runner::perf_run::corpus;
    let index_path =
        artifact_path.with_file_name(format!("perf-corpus-{}.json", corpus_key.replace('.', "-")));
    if skip_seed {
        return std::fs::read_to_string(&index_path)
            .map_err(|e| format!("cannot read corpus index {}: {e}", index_path.display()))
            .and_then(|text| {
                serde_json::from_str::<corpus::SeededCorpus>(&text)
                    .map_err(|e| format!("corpus index: {e}"))
            });
    }
    let (ehrs, versions) = corpus::scale_shape(corpus_key)?;
    let mut seeded = corpus::seed_scale_ladder(
        client,
        corpus_key,
        opt_xml,
        ehrs,
        versions,
        seed_workers,
        progress,
    )
    .map_err(|e| format!("seeding failed: {e}"))?;
    corpus::seed_ward(client, &mut seeded, journey_pack, seed_workers, progress)
        .map_err(|e| format!("ward seeding failed: {e}"))?;
    let text =
        serde_json::to_string(&seeded).map_err(|e| format!("serialize corpus index: {e}"))?;
    std::fs::write(&index_path, text)
        .map_err(|e| format!("cannot write corpus index {}: {e}", index_path.display()))?;
    Ok(seeded)
}

/// The stress handler: the step-load ladder to the maximum sustainable
/// throughput (exploration only — writes stress.json, never results.json).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one-shot orchestration seam
fn stress_command(
    root: &std::path::Path,
    ixit_path: &std::path::Path,
    out: &std::path::Path,
    corpus_class: &str,
    skip_seed: bool,
    seed_workers: usize,
    step_secs: u64,
    bisections: u32,
    max_rate: f64,
) -> ExitCode {
    use cnf_runner::perf::PerfClass;
    use cnf_runner::perf_run;

    let class = match PerfClass::parse(corpus_class) {
        Ok(class) => class,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
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
    let ixit: cnf_runner::ixit::Ixit = match std::fs::read_to_string(ixit_path)
        .map_err(|e| format!("cannot read {}: {e}", ixit_path.display()))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("ixit: {e}")))
    {
        Ok(ixit) => ixit,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let (instance, environment) = match perf_run::window::measured_run_context(&ixit) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let client = match perf_run::client::PerfClient::from_instance(instance) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    // The class supplies corpus + journey workload (data-volume context only).
    let Some((_, case)) = loaded
        .set
        .performance
        .iter()
        .find(|(_, c)| c.class == class)
    else {
        eprintln!("no performance case of class {corpus_class} in the catalogue");
        return ExitCode::from(2);
    };
    let opt_xml = match scale_opt_xml(&loaded) {
        Ok(xml) => xml,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let (catalogue, journey_pack) = match journey_context(&loaded) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let progress = |message: String| eprintln!("[stress] {message}");
    let corpus = match seed_or_load_corpus(
        &client,
        case.corpus.as_str(),
        &opt_xml,
        &journey_pack,
        out,
        skip_seed,
        seed_workers,
        &progress,
    ) {
        Ok(corpus) => corpus,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let options = cnf_runner::stress::StressOptions {
        step_hold_s: step_secs.max(10),
        bisections,
        max_rate,
        ..cnf_runner::stress::StressOptions::default()
    };
    let workload = cnf_runner::perf_run::schedule::JourneyWorkload {
        catalogue: &catalogue,
        shares: &case.workload.journeys,
        pack: &journey_pack,
        // Stress steps are short — the day curve has no meaning there.
        curve: cnf_runner::perf::ArrivalCurve::Uniform,
    };
    let report = match cnf_runner::stress::run_stress(
        &client,
        &corpus,
        &workload,
        environment,
        &options,
        &progress,
    ) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("stress run failed: {e}");
            return ExitCode::from(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(mut text) => {
            text.push('\n');
            if let Some(parent) = out.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                eprintln!("cannot create {}: {e}", parent.display());
                return ExitCode::from(2);
            }
            if let Err(e) = std::fs::write(out, text) {
                eprintln!("cannot write {}: {e}", out.display());
                return ExitCode::from(2);
            }
        }
        Err(e) => {
            eprintln!("serialize: {e}");
            return ExitCode::from(2);
        }
    }
    println!("{}", report.remark);
    println!(
        "wrote {} ({} steps, max sustainable {:.1}/s)",
        out.display(),
        report.steps.len(),
        report.max_sustainable_throughput_per_s
    );
    ExitCode::SUCCESS
}

/// The measured-run handler (`perf`): seed the scale corpus, drive the
/// open-loop workload, merge the measurement record into results.json.
#[allow(clippy::too_many_lines)] // one-shot orchestration seam
fn perf_command(
    root: &std::path::Path,
    ixit_path: &std::path::Path,
    results_path: &std::path::Path,
    class_token: &str,
    skip_seed: bool,
    seed_workers: usize,
    hours: u64,
) -> ExitCode {
    use cnf_runner::perf::PerfClass;
    use cnf_runner::perf_run;

    // The sustained-window ladder: the case's normative window (1 h) or an
    // officially extended one — never anything shorter.
    if ![1, 2, 4, 6, 8, 12].contains(&hours) {
        eprintln!("--hours must be one of 1 | 2 | 4 | 6 | 8 | 12 (got {hours})");
        return ExitCode::from(2);
    }

    let class = match PerfClass::parse(class_token) {
        Ok(class) => class,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
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
    let ixit: cnf_runner::ixit::Ixit = match std::fs::read_to_string(ixit_path)
        .map_err(|e| format!("cannot read {}: {e}", ixit_path.display()))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("ixit: {e}")))
    {
        Ok(ixit) => ixit,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let (instance, environment) = match perf_run::window::measured_run_context(&ixit) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let client = match perf_run::client::PerfClient::from_instance(instance) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let selected: Vec<_> = loaded
        .set
        .performance
        .iter()
        .filter(|(_, c)| c.class == class)
        .collect();
    if selected.is_empty() {
        eprintln!("no performance case of class {class_token} in the catalogue");
        return ExitCode::from(2);
    }
    // The blood-pressure OPT the scale corpus commits against.
    let opt_xml = match scale_opt_xml(&loaded) {
        Ok(xml) => xml,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let (catalogue, journey_pack) = match journey_context(&loaded) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let progress = |message: String| eprintln!("[perf] {message}");

    let mut earned_all = true;
    for (path, case) in selected {
        println!(
            "case {} (class {class_token}) from {}",
            case.id,
            path.display()
        );
        let corpus = match seed_or_load_corpus(
            &client,
            case.corpus.as_str(),
            &opt_xml,
            &journey_pack,
            results_path,
            skip_seed,
            seed_workers,
            &progress,
        ) {
            Ok(corpus) => corpus,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        // The case's normative warmup; the sustained window extends by the
        // hours ladder (a longer hold of the same offered load is a stricter
        // demonstration of the same class).
        let warmup_s = case.workload.warmup.0;
        let duration_s = case.workload.duration.0.max(hours.saturating_mul(3600));
        let measurement = match perf_run::window::drive_case(
            case,
            &client,
            &corpus,
            &journey_pack,
            &catalogue,
            environment,
            warmup_s,
            duration_s,
            &progress,
        ) {
            Ok(measurement) => measurement,
            Err(e) => {
                eprintln!("measured run failed: {e}");
                return ExitCode::from(2);
            }
        };
        for op in &measurement.operations {
            println!(
                "  {}: {} requests, {} errors, p50 {:.1}ms p90 {:.1}ms p99 {:.1}ms",
                op.operation,
                op.requests,
                op.errors,
                op.latency_ms_p50,
                op.latency_ms_p90,
                op.latency_ms_p99
            );
        }
        println!(
            "  offered load sustained {:.2}/s — verdict {:?}{}",
            measurement.offered_load_sustained,
            measurement.verdict,
            if measurement.violations.is_empty() {
                String::new()
            } else {
                format!(" ({})", measurement.violations.join("; "))
            }
        );
        if measurement.verdict != cnf_runner::perf::ClassVerdict::Earned {
            earned_all = false;
        }
        // Merge into results.json (replace any prior record for the case).
        let mut results: Results =
            match load_party_json(results_path, &results_schema(), "results.schema.json") {
                Ok(results) => results,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            };
        results.measurements.retain(|m| m.case != measurement.case);
        results.measurements.push(measurement);
        results
            .measurements
            .sort_by(|a, b| a.case.as_str().cmp(b.case.as_str()));
        match serde_json::to_string_pretty(&results) {
            Ok(mut text) => {
                text.push('\n');
                if let Err(e) = std::fs::write(results_path, text) {
                    eprintln!("cannot write {}: {e}", results_path.display());
                    return ExitCode::from(2);
                }
                println!("  measurement merged into {}", results_path.display());
            }
            Err(e) => {
                eprintln!("serialize: {e}");
                return ExitCode::from(2);
            }
        }
    }
    if earned_all {
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
    statement_path: Option<&std::path::Path>,
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
    let statement: Option<cnf_runner::party::Statement> = match statement_path {
        None => None,
        Some(path) => match std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))
            .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("statement: {e}")))
        {
            Ok(statement) => Some(statement),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        },
    };
    let report = match cnf_runner::run::execute(&set, &ixit, statement.as_ref()) {
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
    // A functional run never re-measures: carry the measurement records of a
    // prior results.json at the same path forward (same SUT name only; a
    // version change gets a loud warning — the §8.10 version-binding rule
    // wants fresh evidence or an unchanged-surface attestation).
    let carried_measurements: Vec<cnf_runner::perf::Measurement> = {
        let prior_path = out.join("results.json");
        std::fs::read_to_string(&prior_path)
            .ok()
            .and_then(|text| serde_json::from_str::<cnf_runner::party::Results>(&text).ok())
            .filter(|prior| prior.sut.name == sut_name)
            .map(|prior| {
                if prior.sut.version != sut_version && !prior.measurements.is_empty() {
                    eprintln!(
                        "warning: carrying {} measurement record(s) taken at SUT version {} into a run at {sut_version} — re-measure or attest the surface unchanged",
                        prior.measurements.len(),
                        prior.sut.version
                    );
                }
                prior.measurements
            })
            .unwrap_or_default()
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
        measurements: carried_measurements,
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
