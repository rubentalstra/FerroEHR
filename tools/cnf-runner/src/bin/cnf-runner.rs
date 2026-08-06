//! The CNF 2.0 reference runner CLI.
//!
//! ```text
//! cnf-runner emit-schemas --out DIR     write the published JSON-Schema set
//! cnf-runner validate --root DIR [--specs DIR]
//! cnf-runner run --root DIR --ixit FILE --out DIR [--sut-name N] [--sut-version V] [--statement F]
//!                                       validate an artifact tree (all gates);
//!                                       --specs enables the SM/spec-ref
//!                                       resolution checks against the vendored
//!                                       spec tree (docs/specs/openehr). The
//!                                       committed party statements beside the
//!                                       root (<root>/../party/*/statement.json)
//!                                       are swept in for the claim gates.
//! cnf-runner verdicts --statement F --results F --root DIR --out DIR
//!                                       compute the verdicts (pure pipeline)
//!                                       and write the report/statement/
//!                                       certificate + verdicts.json
//! cnf-runner perf --root DIR --ixit FILE --results FILE --class POC|S|L|R
//!                 [--hours 1|2|4|6|8|12] [--seed-workers N]
//!                                       the measured class run (conformance-
//!                                       by-measurement): seed the scale
//!                                       corpus, hold the class's offered
//!                                       load for the sustained window, merge
//!                                       the record into results.json
//! cnf-runner stress --root DIR --ixit FILE --out FILE
//!                   [--corpus-class POC|S|L|R]
//!                   [--step-secs N] [--bisections N] [--max-rate R]
//!                                       the step-load stress ladder to the
//!                                       maximum sustainable throughput
//!                                       (exploration only — writes
//!                                       stress.json, never results.json)
//! cnf-runner aql-probe --root DIR --ixit FILE --out FILE
//!                      [--corpus-class POC|S|L|R] [--requests N]
//!                                       the seeded-corpus AQL optimization
//!                                       probe: wire percentiles + DB
//!                                       statement attribution (exploration
//!                                       only — never a conformance record)
//! cnf-runner perf-assets --root DIR --results FILE --out DIR
//!                        [--summary FILE] [--stress FILE]
//!                                       render the published SVGs + summary
//!                                       FROM committed artifacts
//! cnf-runner conformance-assets --root DIR --results FILE --verdicts FILE
//!                               --out DIR [--suffix=-ehrbase]
//!                                       render the capability heat grid +
//!                                       per-chapter outcome bars FROM the
//!                                       committed party artifacts
//! ```
//!
//! Exit codes: `0` clean · `1` findings · `2` runner error.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]
// Verification CLI: progress/diagnostics on the console ARE this tool's user
// interface, so stdio is the right channel here; only library crates are
// restricted to `tracing`.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "this IS the CLI: stdout carries the run report and stderr the \
              diagnostics; only library crates are restricted to `tracing`"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use cnf_runner::artifacts::load_root;
use cnf_runner::load::compile_schema;
use cnf_runner::party::{Results, Statement};
use cnf_runner::render::{render_certificate, render_report, render_statement};
use cnf_runner::schema::{emit_all, render, results_schema, statement_schema};
use cnf_runner::validate::{Context, render_coverage_report, validate};
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
        #[arg(long, default_value = "ferroehr")]
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
        /// Also refresh `docs/conformance/coverage-report.md` from `--specs`.
        ///
        /// OFF by default: `validate` is a check verb, and a check that
        /// mutates the working tree is a trap for read-only and fenced
        /// invocations. The pipeline scripts that publish the report pass
        /// this explicitly.
        #[arg(long)]
        write_report: bool,
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
    /// maximum sustainable throughput — where the system breaks
    /// (exploration only; never a conformance record, and class-free by
    /// design).
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
        /// The class-scale corpus the stress runs on (POC | S | L | R —
        /// the standardized corpus selector): data volume + workload mix
        /// only; no class floor enters the stress report or chart.
        #[arg(long, default_value = "POC")]
        corpus_class: String,
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
    /// Run the AQL optimization probe: fire the measurement machinery's
    /// AQL vocabulary against a live, freshly seeded SUT, record wire
    /// percentiles, and attribute the DB-side cost per probe via
    /// `pg_stat_statements` (exploration evidence for the optimization
    /// loop — never a conformance record).
    AqlProbe {
        /// The artifact root.
        #[arg(long)]
        root: PathBuf,
        /// The ixit topology file (JSON) — the `containers` block enables
        /// DB-side attribution and maintenance settling.
        #[arg(long)]
        ixit: PathBuf,
        /// Where to write the probe report (aql-probe.json).
        #[arg(long)]
        out: PathBuf,
        /// The class-scale corpus the probes run against (POC | S | L | R).
        #[arg(long, default_value = "POC")]
        corpus_class: String,
        /// Parallel seeding workers.
        #[arg(long, default_value_t = 16)]
        seed_workers: usize,
        /// Requests fired per probe.
        #[arg(long, default_value_t = 20)]
        requests: u32,
    },
    /// Render the cross-SUT stress overlay (both systems' latency-throughput
    /// curves on one canvas) FROM two committed stress reports —
    /// deterministic, both directions on equal footing.
    StressCompare {
        /// The primary SUT's committed stress.json.
        #[arg(long)]
        left: PathBuf,
        /// The primary SUT's display label.
        #[arg(long)]
        left_label: String,
        /// The comparison SUT's committed stress.json.
        #[arg(long)]
        right: PathBuf,
        /// The comparison SUT's display label.
        #[arg(long)]
        right_label: String,
        /// Where to write the overlay SVG.
        #[arg(long)]
        out: PathBuf,
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
        /// A suffix appended to the SVG file stems (`-ehrbase` for the
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
        Command::Validate {
            root,
            specs,
            write_report,
        } => validate_command(&root, specs.as_deref(), write_report),
        Command::Perf {
            root,
            ixit,
            results,
            class,
            seed_workers,
            hours,
        } => perf_command(&root, &ixit, &results, &class, seed_workers, hours),
        Command::Stress {
            root,
            ixit,
            out,
            corpus_class,
            seed_workers,
            step_secs,
            bisections,
            max_rate,
        } => stress_command(
            &root,
            &ixit,
            &out,
            &corpus_class,
            seed_workers,
            step_secs,
            bisections,
            max_rate,
        ),
        Command::AqlProbe {
            root,
            ixit,
            out,
            corpus_class,
            seed_workers,
            requests,
        } => probe_command(&root, &ixit, &out, &corpus_class, seed_workers, requests),
        Command::StressCompare {
            left,
            left_label,
            right,
            right_label,
            out,
        } => stress_compare_command(&left, &left_label, &right, &right_label, &out),
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

fn validate_command(
    root: &std::path::Path,
    specs: Option<&std::path::Path>,
    write_report: bool,
) -> ExitCode {
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
    // Refresh the deterministic wire-surface coverage report ONLY when asked
    // (`--write-report`) and the vendored spec tree is supplied (it feeds the
    // Axis-1 SM-operation enumeration). The report lives beside the committed
    // conformance artifacts (docs/conformance/), derived from the `--specs`
    // path; a write failure is a warning, never a gate failure. Default-off:
    // a `validate` verb that rewrites a committed file on every run surprises
    // read-only and fenced invocations.
    if write_report
        && let Some(specs) = specs
        && let Some(docs) = specs.parent().and_then(std::path::Path::parent)
    {
        let report_path = docs.join("conformance/coverage-report.md");
        let body = render_coverage_report(&loaded.set, Some(specs));
        match report_path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&report_path, body))
        {
            Ok(()) => println!("wrote {}", report_path.display()),
            Err(e) => eprintln!("warning: cannot write {}: {e}", report_path.display()),
        }
    }
    println!(
        "{} case(s), {} binding(s), {} party statement(s), {} finding(s)",
        loaded.set.cases.len(),
        loaded.set.bindings.len(),
        loaded.set.parties.len(),
        findings.len()
    );
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[expect(clippy::too_many_lines, reason = "the one-shot orchestration seam")]
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

    // The outward wire-surface axis (`vocab/wire_surface.yaml`
    // `served_extensions`): rendered into the statement as a declaration of the
    // non-openEHR surface, never an input to any verdict.
    let served_extensions = match &loaded.set.wire_surface {
        Some((_, wire_surface)) => wire_surface.served_extensions.as_slice(),
        None => &[],
    };

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
        (
            "CONFORMANCE_REPORT.md",
            match render_report(&results, &report) {
                Ok(markdown) => markdown,
                Err(e) => {
                    eprintln!("cannot render the report: {e}");
                    return ExitCode::from(2);
                }
            },
        ),
        (
            "CONFORMANCE_STATEMENT.md",
            render_statement(&statement, &report, served_extensions),
        ),
        (
            "CONFORMANCE_CERTIFICATE.md",
            render_certificate(&statement, &results, &report, matrix),
        ),
    ];
    // The shields.io endpoints, derived here rather than downstream so a
    // published count and the verdict beside it come from one rule.
    let mut artifacts: Vec<(String, String)> = artifacts
        .into_iter()
        .map(|(name, body)| (name.to_owned(), body))
        .collect();
    for named in cnf_runner::badges::badges(
        &report,
        matrix,
        cnf_runner::badges::CaseCounts::of(&results),
    ) {
        match serde_json::to_string_pretty(&named.badge) {
            Ok(mut json) => {
                json.push('\n');
                artifacts.push((named.file, json));
            }
            Err(e) => {
                eprintln!("cannot serialize the {} badge: {e}", named.file);
                return ExitCode::from(2);
            }
        }
    }

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
    // An unmapped case id is a taxonomy gap, not a chart to publish: the
    // renderer fails loudly and names the id.
    let chapters = match cnf_runner::conf_assets::chapter_counts(&results) {
        Ok(chapters) => chapters,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let assets = [
        (
            format!("conformance-heat-grid{suffix}.svg"),
            cnf_runner::conf_assets::heat_grid_svg(&sut_label, matrix, &verdicts.capabilities),
        ),
        (
            format!("conformance-chapter-bars{suffix}.svg"),
            cnf_runner::conf_assets::chapter_bars_svg(&sut_label, &chapters),
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
#[expect(clippy::too_many_lines, reason = "one-shot orchestration seam")]
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
        // The resource time-series renders only from a record that carries
        // one (sampling is optional by capability; nothing is fabricated).
        if let Some(svg) = cnf_runner::perf_assets::resources_timeseries_svg(measurement) {
            files.push((
                format!("perf-resources-class-{}.svg", measurement.class.token()),
                svg,
            ));
        }
    }
    if let Some(svg) = cnf_runner::perf_assets::disk_growth_svg(&results.measurements) {
        files.push(("perf-disk-growth.svg".to_owned(), svg));
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

/// The seeding milestones the disk anchors probe at (`perf` passes a
/// probing observer; `stress` observes nothing).
#[derive(Debug, Clone, Copy)]
enum SeedStage {
    BeforeScale,
    AfterScale,
    AfterWard,
}

/// Seed the scale corpus + the standing ward. The workflow ALWAYS seeds a
/// freshly composed, empty SUT and the stack is torn down afterwards —
/// there is no seed reuse (the retired `--skip-seed`/sidecar-index scheme
/// bred stale-state confusion). `stage` observes the seeding milestones
/// (the disk anchors).
fn seed_corpus(
    client: &cnf_runner::perf_run::client::PerfClient,
    corpus_key: &str,
    opt_xml: &str,
    journey_pack: &cnf_runner::perf_run::pack::JourneyPack,
    seed_workers: usize,
    progress: &(dyn Fn(String) + Sync),
    stage: &mut dyn FnMut(SeedStage),
) -> Result<cnf_runner::perf_run::corpus::SeededCorpus, String> {
    use cnf_runner::perf_run::corpus;
    let (ehrs, versions) = corpus::scale_shape(corpus_key)?;
    stage(SeedStage::BeforeScale);
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
    stage(SeedStage::AfterScale);
    corpus::seed_ward(client, &mut seeded, journey_pack, seed_workers, progress)
        .map_err(|e| format!("ward seeding failed: {e}"))?;
    stage(SeedStage::AfterWard);
    Ok(seeded)
}

/// The stress handler: the step-load ladder to the maximum sustainable
/// throughput (exploration only — writes stress.json, never results.json).
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one-shot orchestration seam"
)]
fn stress_command(
    root: &std::path::Path,
    ixit_path: &std::path::Path,
    out: &std::path::Path,
    corpus_class: &str,
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
    let mut ixit: cnf_runner::ixit::Ixit = match std::fs::read_to_string(ixit_path)
        .map_err(|e| format!("cannot read {}: {e}", ixit_path.display()))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("ixit: {e}")))
    {
        Ok(ixit) => ixit,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    // File references in the ixit (the SMART lane's signing key) are relative
    // to the ixit document, not to the runner's working directory — the same
    // rebase the `run` command applies (the measured client minted against an
    // unresolved relative path and died at seeding, 2026-07-29 POC run).
    ixit.rebase_paths(ixit_path.parent().unwrap_or(std::path::Path::new(".")));
    let (principals, environment) = match perf_run::window::measured_run_context(&ixit) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let client = principals.primary().clone();
    // The class token is the STANDARDIZED corpus selector (data volume +
    // workload mix); no class floor enters the stress report or chart.
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
    let corpus = match seed_corpus(
        &client,
        case.corpus.as_str(),
        &opt_xml,
        &journey_pack,
        seed_workers,
        &progress,
        // The stress instrument records no disk anchors (exploration only).
        &mut |_| {},
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
    let workload = perf_run::schedule::JourneyWorkload {
        catalogue: &catalogue,
        shares: &case.workload.journeys,
        pack: &journey_pack,
        // Stress steps are short — the day curve has no meaning there.
        curve: cnf_runner::perf::ArrivalCurve::Uniform,
        principals: &principals,
    };
    let report = match cnf_runner::stress::run_stress(
        &principals,
        &corpus,
        &workload,
        environment,
        ixit.containers.as_ref(),
        &options,
        &progress,
    ) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("stress run failed: {e}");
            return ExitCode::from(2);
        }
    };
    if perf_run::rate_limited_observed() {
        eprintln!("{}", perf_run::rate_limited_refusal("stress"));
        return ExitCode::from(2);
    }
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

/// The stress-overlay handler (`stress-compare`): render both systems'
/// latency-throughput curves from their committed stress reports.
fn stress_compare_command(
    left: &std::path::Path,
    left_label: &str,
    right: &std::path::Path,
    right_label: &str,
    out: &std::path::Path,
) -> ExitCode {
    let read = |path: &std::path::Path| -> Result<cnf_runner::stress::StressReport, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))
            .and_then(|text| {
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
            })
    };
    let (left_report, right_report) = match (read(left), read(right)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let svg = match cnf_runner::perf_assets::stress_compare_svg(
        (left_label, &left_report),
        (right_label, &right_report),
    ) {
        Ok(svg) => svg,
        Err(e) => {
            eprintln!("stress compare: {e}");
            return ExitCode::from(2);
        }
    };
    if let Some(parent) = out.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("cannot create {}: {e}", parent.display());
        return ExitCode::from(2);
    }
    if let Err(e) = std::fs::write(out, svg) {
        eprintln!("cannot write {}: {e}", out.display());
        return ExitCode::from(2);
    }
    println!("wrote {}", out.display());
    ExitCode::SUCCESS
}

/// The AQL-probe handler (`aql-probe`): seed the class corpus fresh, run
/// the probe set, write the report (exploration only — never touches
/// results.json).
#[expect(clippy::too_many_lines, reason = "one-shot orchestration seam")]
fn probe_command(
    root: &std::path::Path,
    ixit_path: &std::path::Path,
    out: &std::path::Path,
    corpus_class: &str,
    seed_workers: usize,
    requests: u32,
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
    let mut ixit: cnf_runner::ixit::Ixit = match std::fs::read_to_string(ixit_path)
        .map_err(|e| format!("cannot read {}: {e}", ixit_path.display()))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("ixit: {e}")))
    {
        Ok(ixit) => ixit,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    // File references in the ixit (the SMART lane's signing key) are relative
    // to the ixit document, not to the runner's working directory — the same
    // rebase the `run` command applies (the measured client minted against an
    // unresolved relative path and died at seeding, 2026-07-29 POC run).
    ixit.rebase_paths(ixit_path.parent().unwrap_or(std::path::Path::new(".")));
    let (principals, environment) = match perf_run::window::measured_run_context(&ixit) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let client = principals.primary().clone();
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
    let (_, journey_pack) = match journey_context(&loaded) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let progress = |message: String| eprintln!("[probe] {message}");
    let corpus = match seed_corpus(
        &client,
        case.corpus.as_str(),
        &opt_xml,
        &journey_pack,
        seed_workers,
        &progress,
        // The probe records no disk anchors (exploration only).
        &mut |_| {},
    ) {
        Ok(corpus) => corpus,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let options = cnf_runner::probe::ProbeOptions { requests };
    let report = match cnf_runner::probe::run_probe(
        &client,
        &corpus,
        environment,
        ixit.containers.as_ref(),
        &options,
        &progress,
    ) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("probe run failed: {e}");
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
    println!("wrote {} ({} probes)", out.display(), report.probes.len());
    ExitCode::SUCCESS
}

/// The measured-run handler (`perf`): seed the scale corpus, drive the
/// open-loop workload, merge the measurement record into results.json.
#[expect(clippy::too_many_lines, reason = "one-shot orchestration seam")]
fn perf_command(
    root: &std::path::Path,
    ixit_path: &std::path::Path,
    results_path: &std::path::Path,
    class_token: &str,
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
    let mut ixit: cnf_runner::ixit::Ixit = match std::fs::read_to_string(ixit_path)
        .map_err(|e| format!("cannot read {}: {e}", ixit_path.display()))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("ixit: {e}")))
    {
        Ok(ixit) => ixit,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    // File references in the ixit (the SMART lane's signing key) are relative
    // to the ixit document, not to the runner's working directory — the same
    // rebase the `run` command applies (the measured client minted against an
    // unresolved relative path and died at seeding, 2026-07-29 POC run).
    ixit.rebase_paths(ixit_path.parent().unwrap_or(std::path::Path::new(".")));
    let (principals, environment) = match perf_run::window::measured_run_context(&ixit) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let client = principals.primary().clone();
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
    // Resource sampling is optional by capability: no ixit `containers`
    // block → no `resources` record, never a failed run.
    let containers = ixit.containers.clone();
    if containers.is_none() {
        progress("resources: not sampled (ixit declares no `containers` block)".to_owned());
    }

    let mut earned_all = true;
    for (path, case) in selected {
        println!(
            "case {} (class {class_token}) from {}",
            case.id,
            path.display()
        );
        // The disk anchors bracket the seeding milestones; every probe
        // failure degrades to an absent anchor with the reason logged.
        let mut disk = cnf_runner::perf::DiskAnchors {
            before_scale_seed_bytes: None,
            after_scale_seed_bytes: None,
            after_ward_seed_bytes: None,
            after_window_bytes: None,
            seed_compositions: perf_run::corpus::scale_shape(case.corpus.as_str())
                .ok()
                .and_then(|(ehrs, versions)| u64::try_from(ehrs.saturating_mul(versions)).ok()),
        };
        let probe_volume = |label: &str| -> Option<u64> {
            let db = &containers.as_ref()?.db;
            match perf_run::resources::db_volume_bytes(db) {
                Ok(bytes) => {
                    progress(format!("disk anchor {label}: {bytes} bytes"));
                    Some(bytes)
                }
                Err(e) => {
                    progress(format!("disk anchor {label} unavailable: {e}"));
                    None
                }
            }
        };
        let corpus = match seed_corpus(
            &client,
            case.corpus.as_str(),
            &opt_xml,
            &journey_pack,
            seed_workers,
            &progress,
            &mut |milestone| match milestone {
                SeedStage::BeforeScale => {
                    disk.before_scale_seed_bytes = probe_volume("before scale seed");
                }
                SeedStage::AfterScale => {
                    disk.after_scale_seed_bytes = probe_volume("after scale seed");
                }
                SeedStage::AfterWard => {
                    disk.after_ward_seed_bytes = probe_volume("after preflight + ward seed");
                }
            },
        ) {
            Ok(corpus) => corpus,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        // Settle the seeding's maintenance debt before the window: a
        // mid-window autovacuum/analyze of the freshly seeded tables would
        // saturate the engine inside the measurement.
        if let Some(c) = &containers {
            progress(
                "settling maintenance before the measured window (vacuumdb --analyze)".to_owned(),
            );
            if let Err(e) = perf_run::resources::settle_maintenance(&c.db) {
                progress(format!("maintenance not settled: {e}"));
            }
        }
        // The case's normative warmup; the sustained window extends by the
        // hours ladder (a longer hold of the same offered load is a stricter
        // demonstration of the same class).
        let warmup_s = case.workload.warmup.0;
        let duration_s = case.workload.duration.0.max(hours.saturating_mul(3600));
        // The sampler brackets the whole window (warmup + sustained + the
        // completion drain) and stops after the dispatcher's last
        // completion lands — drive_case returns only then.
        let sampler = containers
            .as_ref()
            .map(|c| perf_run::resources::ResourceSampler::start(c, warmup_s, duration_s));
        let mut measurement = match perf_run::window::drive_case(
            case,
            &principals,
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
        if let Some(sampler) = sampler {
            let (series, notes) = sampler.stop();
            for note in notes {
                progress(note);
            }
            disk.after_window_bytes = probe_volume("after measured window");
            let sampled_any = series.iter().any(|s| !s.samples.is_empty());
            let anchored_any = disk.before_scale_seed_bytes.is_some()
                || disk.after_scale_seed_bytes.is_some()
                || disk.after_ward_seed_bytes.is_some()
                || disk.after_window_bytes.is_some();
            if sampled_any || anchored_any {
                measurement.resources = Some(cnf_runner::perf::ResourcesRecord {
                    sample_interval_s: perf_run::resources::SAMPLE_INTERVAL.as_secs(),
                    containers: series,
                    disk: Some(disk),
                });
            } else {
                progress(
                    "resources: not sampled (container runtime unreachable for the whole run)"
                        .to_owned(),
                );
            }
        }
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
        println!("  {}", cnf_runner::perf::verdict_evidence(&measurement));
        if measurement.verdict != cnf_runner::perf::ClassVerdict::Earned {
            earned_all = false;
        }
        // A limiter-shaped window is not a measurement of this server, so it
        // never reaches results.json (`crate::perf_run::rate_limited_observed`).
        if perf_run::rate_limited_observed() {
            eprintln!("{}", perf_run::rate_limited_refusal("perf"));
            return ExitCode::from(2);
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
        // A measurement whose case is no longer in the catalogue (a
        // renamed/retired case) is an orphan the verdict review would
        // flag — prune it here, visibly.
        results.measurements.retain(|m| {
            let known = loaded.set.performance.iter().any(|(_, c)| c.id == m.case);
            if !known {
                println!("  pruned orphaned measurement for retired case {}", m.case);
            }
            known
        });
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
#[expect(clippy::too_many_lines, reason = "the one-shot orchestration seam")]
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
    let mut ixit: cnf_runner::ixit::Ixit = match serde_json::from_str(&ixit_text) {
        Ok(ixit) => ixit,
        Err(e) => {
            eprintln!("ixit: {e}");
            return ExitCode::from(2);
        }
    };
    // File references in the ixit (the SMART lane's signing key) are relative
    // to the ixit document, not to the runner's working directory.
    ixit.rebase_paths(ixit_path.parent().unwrap_or(std::path::Path::new(".")));
    let mut set = loaded.set;
    if let Some(needle) = filter {
        set.cases.retain(|(_, c)| c.id.as_str().contains(needle));
    }
    let statement: Option<Statement> = match statement_path {
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
        // NOTE: no prior file is ABSENCE (the first run at this path); a file
        // that exists but will not read or parse is a DEFECT — carrying zero
        // measurements past it would silently drop the §8.10 evidence.
        let prior = match std::fs::read_to_string(&prior_path) {
            Ok(text) => match serde_json::from_str::<Results>(&text) {
                Ok(prior) => Some(prior),
                Err(e) => {
                    eprintln!(
                        "runner defect: {} exists but does not parse as results.json ({e}) — \
                         its measurement records cannot be carried forward",
                        prior_path.display()
                    );
                    return ExitCode::from(2);
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                eprintln!(
                    "runner defect: {} is unreadable ({e})",
                    prior_path.display()
                );
                return ExitCode::from(2);
            }
        };
        prior
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
    let results = Results {
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
        // The recorded technology profile IS the claim the verdict pipeline
        // selects gating records with (`verdict::rollup_results`): a narrow
        // hardcoded list here silently deselects every other format's failed
        // rows — the false-green shape that hid four red canonical-xml rows
        // behind a PASS badge (#288 convergence run, 2026-07-28). The profile
        // therefore comes from the party statement's its-rest claim; with no
        // statement, EVERY format is selected so nothing red can vanish.
        tech_profile: cnf_runner::party::TechProfile {
            its: cnf_runner::vocab::ItsName::ItsRest,
            formats: statement
                .as_ref()
                .and_then(|s| {
                    s.tech_profiles
                        .iter()
                        .find(|p| p.its == cnf_runner::vocab::ItsName::ItsRest)
                })
                .map_or_else(
                    || cnf_runner::vocab::FormatName::ALL.to_vec(),
                    |p| p.formats.clone(),
                ),
        },
        ixit_digest,
        restapi_specs_version: report.restapi_specs_version.clone(),
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
