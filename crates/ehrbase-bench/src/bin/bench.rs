//! `bench` CLI — the benchmark harness entry point (docs/design/benchmarking.md).
//!
//! `bench run` drives the pre-registered workload against a SUT (external URL or,
//! under `--features self-host`, an in-process ehrbase-rs) and writes the
//! generated report to `--out`. `bench seed` bulk-loads a SUT for the scale
//! ladder. `bench report --from results.json` re-renders the markdown.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use ehrbase_bench::driver::{DriverConfig, ScenarioResult, run_latency};
use ehrbase_bench::report::{BenchReport, EnvBlock};
use ehrbase_bench::target::{Implementation, Target};
use ehrbase_bench::workload::{Scenario, workload_lock};
use ehrbase_conformance::client::Credential;

#[derive(Parser)]
#[command(
    name = "bench",
    about = "Honest ehrbase-rs vs. EHRbase benchmark harness"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the workload against a SUT and write the report.
    Run(RunArgs),
    /// Bulk-seed a SUT (scale ladder).
    Seed(SeedArgs),
    /// Re-render REPORT.md from a results.json.
    Report(ReportArgs),
}

#[derive(Parser)]
struct RunArgs {
    /// SUT base URL (…/ehrbase/rest/openehr/v1). Omit with --self-host.
    #[arg(long)]
    base_url: Option<String>,
    /// Boot an in-process ehrbase-rs (requires --features self-host).
    #[arg(long)]
    self_host: bool,
    /// Which implementation the SUT is (labels the report).
    #[arg(long, default_value = "ehrbase-rs")]
    implementation: String,
    /// Auth spec: basic:<user>:<pass> or bearer:<token>.
    #[arg(long)]
    auth: Option<String>,
    /// Only run this scenario id (W1/W2/W4/W8); default all.
    #[arg(long)]
    scenario: Option<String>,
    /// Fast smoke config (proves the harness, not publishable numbers).
    #[arg(long)]
    smoke: bool,
    /// Merge this run's results into an existing `out/results.json` (keeping the
    /// other target's rows) so a two-server comparison lands in one report.
    #[arg(long)]
    merge: bool,
    /// Output directory.
    #[arg(long, default_value = "docs/benchmarks")]
    out: PathBuf,
    /// Override the ISO-8601 run timestamp (default: now). The host machine is
    /// always auto-captured regardless.
    #[arg(long)]
    run_date: Option<String>,
}

#[derive(Parser)]
struct SeedArgs {
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long)]
    self_host: bool,
    #[arg(long)]
    auth: Option<String>,
    #[arg(long, default_value_t = 10)]
    ehrs: u32,
    #[arg(long, default_value_t = 1)]
    comps: u32,
}

#[derive(Parser)]
struct ReportArgs {
    /// Path to a results.json produced by `bench run`.
    #[arg(long)]
    from: PathBuf,
    /// Output directory.
    #[arg(long, default_value = "docs/benchmarks")]
    out: PathBuf,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Run(args) => cmd_run(args).await,
        Command::Seed(args) => cmd_seed(args).await,
        Command::Report(args) => cmd_report(&args),
    };
    std::process::exit(code);
}

/// Resolve a target (external or self-hosted). Returns the target and an
/// optional keep-alive guard (the self-hosted SUT, whose Drop stops the server).
// `async` is load-bearing under `--features self-host` (awaits container boot);
// without the feature there is no await, which is expected.
#[cfg_attr(not(feature = "self-host"), allow(clippy::unused_async))]
async fn resolve_target(
    base_url: Option<String>,
    self_host: bool,
    implementation: Implementation,
    auth: Option<String>,
) -> Result<(Target, Option<Box<dyn std::any::Any>>), String> {
    if self_host {
        #[cfg(feature = "self-host")]
        {
            let sut = ehrbase_conformance::sut::Sut::self_hosted()
                .await
                .map_err(|e| format!("self-host boot: {e}"))?;
            let base = sut.base_url();
            // The dev Basic user the self-hosted app is configured with.
            let cred = Credential::Basic {
                user: "ehrbase".to_owned(),
                pass: "ehrbase".to_owned(),
            };
            let target = Target::new(implementation, base, Some(cred.clone()), Some(cred))
                .map_err(|e| e.to_string())?;
            return Ok((target, Some(Box::new(sut) as Box<dyn std::any::Any>)));
        }
        #[cfg(not(feature = "self-host"))]
        {
            let _ = implementation;
            return Err("--self-host requires building with `--features self-host`".to_owned());
        }
    }
    let base = base_url.ok_or_else(|| "one of --base-url or --self-host is required".to_owned())?;
    let cred = auth.as_deref().map(Credential::parse).transpose()?;
    let target =
        Target::new(implementation, base, cred.clone(), cred).map_err(|e| e.to_string())?;
    Ok((target, None))
}

async fn cmd_run(args: RunArgs) -> i32 {
    let implementation = match args.implementation.parse::<Implementation>() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let (target, _keep_alive) =
        match resolve_target(args.base_url, args.self_host, implementation, args.auth).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("error: {e}");
                return 2;
            }
        };

    let cfg = if args.smoke {
        DriverConfig::smoke()
    } else {
        DriverConfig::default()
    };

    let scenarios: Vec<Scenario> = match &args.scenario {
        Some(id) => {
            if let Some(s) = Scenario::ALL
                .iter()
                .find(|s| s.id().eq_ignore_ascii_case(id))
            {
                vec![*s]
            } else {
                eprintln!("error: unknown scenario {id:?} (expected W1/W2/W4/W8)");
                return 2;
            }
        }
        None => Scenario::ALL.to_vec(),
    };

    let mut results: Vec<ScenarioResult> = Vec::new();

    // Merge: keep the other target's rows from a prior run so a two-server
    // comparison lands in one report (this run's label overwrites its own).
    if args.merge {
        let existing = args.out.join("results.json");
        if let Ok(text) = std::fs::read_to_string(&existing)
            && let Ok(prior) = serde_json::from_str::<BenchReport>(&text)
        {
            let this = target.label();
            results.extend(prior.results.into_iter().filter(|r| r.target != this));
        }
    }

    for s in scenarios {
        eprintln!(
            "running {} ({}) against {}",
            s.id(),
            s.description(),
            target.label()
        );
        match run_latency(&target, s, cfg).await {
            Ok(r) => results.push(r),
            Err(e) => {
                eprintln!("error running {}: {e}", s.id());
                return 1;
            }
        }
    }

    let report = BenchReport {
        env: EnvBlock {
            run_date: args.run_date.unwrap_or_else(now_iso),
            // Auto-captured — every report states the machine that produced it.
            host: ehrbase_bench::host::HostInfo::capture(),
            payload: ehrbase_bench::workload::payload_description(),
            workload_lock: workload_lock(),
            harness_revision: option_env!("GIT_REV").unwrap_or("unknown").to_owned(),
            warmup_iters: cfg.warmup_iters,
            measure_iters: cfg.measure_iters,
            runs: cfg.runs,
        },
        results,
    };

    write_report(&report, &args.out)
}

async fn cmd_seed(args: SeedArgs) -> i32 {
    let (target, _keep_alive) = match resolve_target(
        args.base_url,
        args.self_host,
        Implementation::EhrbaseRs,
        args.auth,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    match ehrbase_bench::seed::seed(&target, args.ehrs, args.comps).await {
        Ok(n) => {
            eprintln!("seeded {n} compositions across {} EHRs", args.ehrs);
            0
        }
        Err(e) => {
            eprintln!("seed error: {e}");
            1
        }
    }
}

fn cmd_report(args: &ReportArgs) -> i32 {
    let json = match std::fs::read_to_string(&args.from) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {}: {e}", args.from.display());
            return 2;
        }
    };
    let report: BenchReport = match serde_json::from_str(&json) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error parsing results.json: {e}");
            return 2;
        }
    };
    write_report(&report, &args.out)
}

fn write_report(report: &BenchReport, out: &std::path::Path) -> i32 {
    if let Err(e) = std::fs::create_dir_all(out) {
        eprintln!("error creating {}: {e}", out.display());
        return 1;
    }
    let json = match report.to_json() {
        Ok(j) => j,
        Err(e) => {
            eprintln!("error serializing results: {e}");
            return 1;
        }
    };
    let md = report.to_markdown();
    if let Err(e) = std::fs::write(out.join("results.json"), json) {
        eprintln!("error writing results.json: {e}");
        return 1;
    }
    if let Err(e) = std::fs::write(out.join("REPORT.md"), md) {
        eprintln!("error writing REPORT.md: {e}");
        return 1;
    }
    eprintln!("wrote {}/REPORT.md + results.json", out.display());
    0
}

fn now_iso() -> String {
    jiff::Timestamp::now().to_string()
}
