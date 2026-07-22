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
        /// Exploratory smoke run: a tiny corpus and a seconds-long window to
        /// prove the wiring; NEVER persisted (the record would not realize
        /// the case's workload).
        #[arg(long)]
        smoke: bool,
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
            smoke,
        } => perf_command(
            &root,
            &ixit,
            &results,
            &class,
            skip_seed,
            seed_workers,
            smoke,
        ),
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

/// The measured-run handler (`perf`): seed the scale corpus, drive the
/// open-loop workload, merge the measurement record into results.json.
#[allow(clippy::too_many_lines, clippy::fn_params_excessive_bools)] // one-shot orchestration seam
fn perf_command(
    root: &std::path::Path,
    ixit_path: &std::path::Path,
    results_path: &std::path::Path,
    class_token: &str,
    skip_seed: bool,
    seed_workers: usize,
    smoke: bool,
) -> ExitCode {
    use cnf_runner::perf::PerfClass;
    use cnf_runner::perf_run;

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
    let (instance, environment) = match perf_run::measured_run_context(&ixit) {
        Ok(context) => context,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let client = match perf_run::PerfClient::from_instance(instance) {
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
    let opt_xml = {
        let Some(corpus_dir) = loaded.set.corpus_dir.as_deref() else {
            eprintln!("artifact set has no corpus directory");
            return ExitCode::from(2);
        };
        let key = match cnf_runner::ids::CorpusKey::parse("cnf.opt.blood_pressure") {
            Ok(key) => key,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        let source = loaded
            .set
            .corpus
            .as_ref()
            .and_then(|(_, m)| m.get(&key))
            .and_then(|entry| entry.source.clone());
        let Some(source) = source else {
            eprintln!("corpus manifest has no cnf.opt.blood_pressure fixture");
            return ExitCode::from(2);
        };
        match std::fs::read_to_string(corpus_dir.join(&source)) {
            Ok(xml) => xml,
            Err(e) => {
                eprintln!("cannot read OPT fixture {source}: {e}");
                return ExitCode::from(2);
            }
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
        let (ehrs, versions) = if smoke {
            (25, 4)
        } else {
            match perf_run::scale_shape(case.corpus.as_str()) {
                Ok(shape) => shape,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            }
        };
        let index_path = results_path.with_file_name(format!(
            "perf-corpus-{}.json",
            case.corpus.as_str().replace('.', "-")
        ));
        let corpus = if skip_seed && !smoke {
            match std::fs::read_to_string(&index_path)
                .map_err(|e| format!("cannot read corpus index {}: {e}", index_path.display()))
                .and_then(|text| {
                    serde_json::from_str::<perf_run::SeededCorpus>(&text)
                        .map_err(|e| format!("corpus index: {e}"))
                }) {
                Ok(corpus) => corpus,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::from(2);
                }
            }
        } else {
            match perf_run::seed_scale_ladder(
                &client,
                case.corpus.as_str(),
                &opt_xml,
                ehrs,
                versions,
                seed_workers,
                &progress,
            ) {
                Ok(corpus) => corpus,
                Err(e) => {
                    eprintln!("seeding failed: {e}");
                    return ExitCode::from(2);
                }
            }
        };
        if !smoke && !skip_seed {
            match serde_json::to_string(&corpus) {
                Ok(text) => {
                    if let Err(e) = std::fs::write(&index_path, text) {
                        eprintln!("cannot write corpus index {}: {e}", index_path.display());
                        return ExitCode::from(2);
                    }
                }
                Err(e) => {
                    eprintln!("serialize corpus index: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        let (warmup_s, duration_s) = if smoke {
            (2, 20)
        } else {
            (case.workload.warmup.0, case.workload.duration.0)
        };
        let measurement = match perf_run::drive_case(
            case,
            &client,
            &corpus,
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
        if smoke {
            println!("  smoke run — record NOT persisted (workload not realized as specified)");
            continue;
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
