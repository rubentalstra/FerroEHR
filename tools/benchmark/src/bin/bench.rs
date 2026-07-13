//! `bench` CLI — the hospital-day stress instrument (register 01 §8).
//!
//! ```text
//! bench run    --sut ehrbase-rs|ehrbase-java|byo [--base-url URL] [--auth SPEC]
//!              [--admin-auth SPEC] --profile smoke|hour|day
//!              --scale empty|10k|100k|1m [--ward-size N] [--load-factor L]
//!              [--seed U64] [--app-container NAME] [--db-container NAME]
//!              [--db-user U] [--db-name D] [--no-seed] [--out DIR]
//! bench knee   --sut … --scale … [--ward-size N] [--seed U64]
//!              [--steps "1,2,4,8,16,32"] [--step-window 120] [--warmup 15]
//!              [--app-container NAME] [--db-container NAME] [--no-seed] [--out DIR]
//! bench seed   --sut … --scale … [--seed U64]
//! bench report --from results.json [--out DIR]
//! bench compare --from a.json --from b.json [--knee-from a-knee.json …] [--out DIR]
//! ```
//!
//! `run` provisions the workload's templates, (optionally) seeds the scale rung,
//! drives the open-loop schedule against the SUT (the conformance transport —
//! the fairness guarantee), samples container CPU/RSS + storage, and writes the
//! per-SUT artefact set into `--out/<sut-name>/`. Exit codes: `0` ok · `1` the
//! 0.1% error-rate flag was breached · `2` runner/SUT failure.
//!
//! `knee` provisions + seeds once, then drives the `hour` rate shape at an
//! ascending load-factor ladder on short fixed windows (register 01 §3), stops
//! at the first step past the SLO (p99 > 1 s) or the 0.1% error flag, and writes
//! `knee.json` + `KNEE.md` + `charts/knee.svg`. Exit: `0` ok · `2` failure.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};

use benchmark::drive::{self, DriveOutcome};
use benchmark::measure::Recorder;
use benchmark::model::{self, WorkloadSpec};
use benchmark::report::json::{
    ClassRecord, ContainerSummary, EnvironmentBlock, ResourcesBlock, Results, StorageBlock,
    SutBlock, ThroughputBlock, WorkloadBlock,
};
use benchmark::report::knee::{KneeResults, KneeStep};
use benchmark::sample::{self, ContainerSeries, DbAccess, ResourceSampler};
use benchmark::{Profile, Scale, report, seed};

use conformance::sut::builtin;
use conformance::sut::descriptor::{SutDescriptor, SutKind};
use conformance::transport::{Credential, SutClient};

/// A fixed default seed so an unqualified run is reproducible run-to-run.
const DEFAULT_SEED: u64 = 0x_B0_11_CA_FE;
/// The 0.1% error-rate flag (register 01 §1).
const ERROR_RATE_FLAG: f64 = 0.001;

#[derive(Debug, Parser)]
#[command(
    name = "bench",
    about = "The openEHR CDR hospital-day stress instrument"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Drive the workload against a SUT and write its artefact set.
    Run(RunArgs),
    /// Drive the knee/saturation ladder and write the knee artefact set.
    Knee(KneeArgs),
    /// Deterministically seed a SUT to a scale rung.
    Seed(SeedArgs),
    /// Re-render the artefact set from an existing results.json.
    Report(ReportArgs),
    /// Render the cross-SUT comparison (Markdown + charts) from two runs.
    Compare(CompareArgs),
}

#[derive(Debug, Parser)]
struct CompareArgs {
    /// A results.json to include (repeat; the first two are compared).
    #[arg(long = "from", required = true, num_args = 1..)]
    from: Vec<PathBuf>,
    /// An optional knee.json to include (repeat; the first two add the knee
    /// section). When fewer than two are supplied the knee section is omitted.
    #[arg(long = "knee-from", num_args = 1..)]
    knee_from: Vec<PathBuf>,
    /// Where to write COMPARISON.md (+ charts/ beside it).
    #[arg(long, default_value = "docs/benchmarks")]
    out: PathBuf,
}

#[derive(Debug, Parser)]
struct KneeArgs {
    /// The target class (mirrors `bench run`).
    #[arg(long, value_enum, default_value_t = SutArg::EhrbaseRs)]
    sut: SutArg,
    /// The SUT's ITS-REST base URL (default per target).
    #[arg(long)]
    base_url: Option<String>,
    /// Override the output/lookup name (default: the target's name, or `byo`).
    #[arg(long)]
    sut_name: Option<String>,
    /// Product version label (used for the `ehrbase-java` product label).
    #[arg(long)]
    sut_version: Option<String>,
    /// Regular credential (`basic:<u>:<p>` or `bearer:<t>`).
    #[arg(long)]
    auth: Option<String>,
    /// ADMIN-role credential (same forms).
    #[arg(long)]
    admin_auth: Option<String>,
    /// The pre-seeded scale rung (metadata + the seed target when seeding).
    #[arg(long, value_enum)]
    scale: ScaleArg,
    /// The ward size (admitted patients).
    #[arg(long, default_value_t = 20)]
    ward_size: usize,
    /// The deterministic generator seed.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
    /// The app container name (compose-managed; accepted for CLI symmetry).
    #[arg(long)]
    app_container: Option<String>,
    /// The db container name (compose-managed; accepted for CLI symmetry).
    #[arg(long)]
    db_container: Option<String>,
    /// The db psql role (accepted for CLI symmetry with `bench run`).
    #[arg(long, default_value = "ehrbase")]
    db_user: String,
    /// The db name (accepted for CLI symmetry with `bench run`).
    #[arg(long, default_value = "ehrbase")]
    db_name: String,
    /// Skip seeding (the DB is already at the scale rung).
    #[arg(long)]
    no_seed: bool,
    /// The ascending load-factor ladder (comma list).
    #[arg(long, default_value = "1,2,4,8,16,32", value_delimiter = ',')]
    steps: Vec<f64>,
    /// The fixed per-step measurement window (seconds).
    #[arg(long, default_value_t = 120)]
    step_window: u64,
    /// The per-step warmup floor (seconds).
    #[arg(long, default_value_t = 15)]
    warmup: u64,
    /// Where to write the knee artefact set (the SUT name is appended).
    #[arg(long, default_value = "docs/benchmarks")]
    out: PathBuf,
}

#[derive(Debug, Parser)]
struct RunArgs {
    /// The target class (mirrors `conformance run`).
    #[arg(long, value_enum, default_value_t = SutArg::EhrbaseRs)]
    sut: SutArg,
    /// The SUT's ITS-REST base URL (default per target).
    #[arg(long)]
    base_url: Option<String>,
    /// Override the output/lookup name (default: the target's name, or `byo`).
    #[arg(long)]
    sut_name: Option<String>,
    /// Product version label (used for the `ehrbase-java` product label).
    #[arg(long)]
    sut_version: Option<String>,
    /// Regular credential (`basic:<u>:<p>` or `bearer:<t>`).
    #[arg(long)]
    auth: Option<String>,
    /// ADMIN-role credential (same forms).
    #[arg(long)]
    admin_auth: Option<String>,
    /// The run profile.
    #[arg(long, value_enum)]
    profile: ProfileArg,
    /// The pre-seeded scale rung (metadata + the seed target when seeding).
    #[arg(long, value_enum)]
    scale: ScaleArg,
    /// The ward size (admitted patients).
    #[arg(long, default_value_t = 20)]
    ward_size: usize,
    /// The arrival-rate load factor `L`.
    #[arg(long, default_value_t = 1.0)]
    load_factor: f64,
    /// The deterministic generator seed.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
    /// The app container name (compose-managed; enables CPU/RSS sampling).
    #[arg(long)]
    app_container: Option<String>,
    /// The db container name (compose-managed; enables CPU/RSS + storage probe).
    #[arg(long)]
    db_container: Option<String>,
    /// The db psql role for the storage probe.
    #[arg(long, default_value = "ehrbase")]
    db_user: String,
    /// The db name for the storage probe.
    #[arg(long, default_value = "ehrbase")]
    db_name: String,
    /// Skip seeding (the DB is already at the scale rung).
    #[arg(long)]
    no_seed: bool,
    /// Compose-up → first-answer duration, supplied by `scripts/benchmark.sh`.
    #[arg(long, hide = true)]
    cold_start_ms: Option<u64>,
    /// Where to write the artefact set (the SUT name is appended).
    #[arg(long, default_value = "docs/benchmarks")]
    out: PathBuf,
}

#[derive(Debug, Parser)]
struct SeedArgs {
    #[arg(long, value_enum, default_value_t = SutArg::EhrbaseRs)]
    sut: SutArg,
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long)]
    sut_name: Option<String>,
    #[arg(long)]
    sut_version: Option<String>,
    #[arg(long)]
    auth: Option<String>,
    #[arg(long)]
    admin_auth: Option<String>,
    #[arg(long, value_enum)]
    scale: ScaleArg,
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
}

#[derive(Debug, Parser)]
struct ReportArgs {
    /// The results.json to regenerate artefacts from.
    #[arg(long)]
    from: PathBuf,
    /// Where to write the artefact set.
    #[arg(long, default_value = "docs/benchmarks")]
    out: PathBuf,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum SutArg {
    #[default]
    EhrbaseRs,
    EhrbaseJava,
    Byo,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProfileArg {
    Smoke,
    Hour,
    Day,
}

impl From<ProfileArg> for Profile {
    fn from(p: ProfileArg) -> Self {
        match p {
            ProfileArg::Smoke => Profile::Smoke,
            ProfileArg::Hour => Profile::Hour,
            ProfileArg::Day => Profile::Day,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScaleArg {
    Empty,
    #[value(name = "10k")]
    TenK,
    #[value(name = "100k")]
    HundredK,
    #[value(name = "1m")]
    OneM,
}

impl From<ScaleArg> for Scale {
    fn from(s: ScaleArg) -> Self {
        match s {
            ScaleArg::Empty => Scale::Empty,
            ScaleArg::TenK => Scale::TenK,
            ScaleArg::HundredK => Scale::HundredK,
            ScaleArg::OneM => Scale::OneM,
        }
    }
}

#[tokio::main]
async fn main() {
    let code = match Cli::parse().command {
        Command::Run(args) => cmd_run(args).await,
        Command::Knee(args) => cmd_knee(args).await,
        Command::Seed(args) => cmd_seed(args).await,
        Command::Report(args) => cmd_report(&args),
        Command::Compare(args) => cmd_compare(&args),
    };
    std::process::exit(code);
}

/// `bench compare` — render the cross-SUT comparison from two results.json
/// (and, optionally, two knee.json for the knee section).
fn cmd_compare(args: &CompareArgs) -> i32 {
    let mut runs = Vec::new();
    for path in &args.from {
        match report::from_results_file(path) {
            Ok(r) => runs.push(r),
            Err(e) => {
                eprintln!("error reading {}: {e}", path.display());
                return 2;
            }
        }
    }
    let mut knees = Vec::new();
    for path in &args.knee_from {
        match report::knee::from_file(path) {
            Ok(k) => knees.push(k),
            Err(e) => {
                eprintln!("error reading {}: {e}", path.display());
                return 2;
            }
        }
    }
    let rendered = report::compare::render(&runs, &knees);
    let chart_dir = args.out.join("charts");
    if let Err(e) = std::fs::create_dir_all(&chart_dir) {
        eprintln!("error creating {}: {e}", chart_dir.display());
        return 2;
    }
    for (name, svg) in &rendered.charts {
        if let Err(e) = std::fs::write(chart_dir.join(name), svg) {
            eprintln!("error writing chart {name}: {e}");
            return 2;
        }
    }
    let md_path = args.out.join("COMPARISON.md");
    if let Err(e) = std::fs::write(&md_path, rendered.markdown) {
        eprintln!("error writing {}: {e}", md_path.display());
        return 2;
    }
    eprintln!(
        "bench: comparison of {} run(s) → {}",
        runs.len(),
        md_path.display()
    );
    0
}

/// Default base URL per target (matches `scripts/benchmark.sh` / the compose
/// port mappings).
fn default_base_url(sut: SutArg) -> Option<&'static str> {
    match sut {
        SutArg::EhrbaseRs => Some("http://localhost:8080/ehrbase/rest/openehr/v1"),
        SutArg::EhrbaseJava => Some("http://localhost:8091/ehrbase/rest/openehr/v1"),
        SutArg::Byo => None,
    }
}

/// Build the SUT descriptor (mirrors `conformance run`).
fn build_descriptor(
    sut: SutArg,
    base_url: Option<String>,
    sut_name: Option<String>,
    sut_version: Option<&str>,
    auth: Option<String>,
    admin_auth: Option<String>,
) -> Result<SutDescriptor, String> {
    let base_url = base_url
        .or_else(|| default_base_url(sut).map(ToOwned::to_owned))
        .ok_or_else(|| "--base-url is required for --sut byo".to_owned())?;

    let mut descriptor = match sut {
        SutArg::EhrbaseRs => builtin::ehrbase_rs(base_url, auth, admin_auth),
        SutArg::EhrbaseJava => {
            let version = sut_version.unwrap_or("upstream");
            builtin::ehrbase_java(base_url, auth, admin_auth, version)
        }
        SutArg::Byo => {
            let mut d = SutDescriptor::byo(sut_name.clone(), base_url);
            d.auth = auth;
            d.admin_auth = admin_auth;
            d
        }
    };
    if let Some(name) = sut_name {
        descriptor.name = name;
    }
    Ok(descriptor)
}

/// Build the reqwest transport for a descriptor.
fn build_transport(descriptor: &SutDescriptor) -> Result<SutClient, String> {
    let regular = descriptor
        .auth
        .as_deref()
        .map(Credential::parse)
        .transpose()?;
    let admin = descriptor
        .admin_auth
        .as_deref()
        .map(Credential::parse)
        .transpose()?;
    SutClient::new(descriptor.base_url.clone(), regular, admin)
        .map(|c| c.with_admin_base_url(descriptor.admin_base_url.clone()))
        .map_err(|e| e.to_string())
}

fn sut_kind_label(kind: SutKind) -> &'static str {
    match kind {
        SutKind::Ours => "ours",
        SutKind::Foreign => "foreign",
    }
}

/// The idle-baseline sampling duration (register 01 §2: 30 s, ~3 s for smoke).
fn baseline_duration(profile: Profile) -> Duration {
    match profile {
        Profile::Smoke => Duration::from_secs(3),
        _ => Duration::from_secs(30),
    }
}

// The run command is a linear pipeline (descriptor → transport → seed → build →
// sample → drive → probe → write) whose steps read best in sequence; splitting
// it would scatter the shared locals across helpers.
#[allow(clippy::too_many_lines)]
async fn cmd_run(args: RunArgs) -> i32 {
    let descriptor = match build_descriptor(
        args.sut,
        args.base_url.clone(),
        args.sut_name.clone(),
        args.sut_version.as_deref(),
        args.auth.clone(),
        args.admin_auth.clone(),
    ) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let transport = match build_transport(&descriptor) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    let profile = Profile::from(args.profile);
    let scale = Scale::from(args.scale);

    // Seed the scale rung first (unless suppressed / empty).
    let mut seeded_compositions: Option<u64> = None;
    if !args.no_seed && scale.compositions() > 0 {
        eprintln!("bench: seeding scale rung {} …", scale.key());
        match seed::seed_scale(&transport, scale, args.seed).await {
            Ok(n) => seeded_compositions = Some(n),
            Err(e) => {
                eprintln!("error seeding: {e}");
                return 2;
            }
        }
    }

    // Build the workload.
    let spec = WorkloadSpec {
        profile,
        ward_size: args.ward_size,
        load_factor: args.load_factor,
        seed: args.seed,
    };
    let workload = match model::build(&spec) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error building workload: {e}");
            return 2;
        }
    };

    // Resource sampling: idle baseline, then the run series.
    let containers: Vec<String> = [args.app_container.clone(), args.db_container.clone()]
        .into_iter()
        .flatten()
        .collect();
    let idle = if containers.is_empty() {
        Vec::new()
    } else {
        eprintln!("bench: idle baseline ({:?}) …", baseline_duration(profile));
        match sample::idle_baseline(containers.clone(), baseline_duration(profile)).await {
            Ok(series) => series,
            Err(e) => {
                eprintln!("bench: idle baseline sampling failed: {e}");
                Vec::new()
            }
        }
    };
    let sampler = ResourceSampler::start(containers);

    eprintln!(
        "bench: driving profile={} scale={} ward={} L={} against {}",
        profile.key(),
        scale.key(),
        args.ward_size,
        args.load_factor,
        descriptor.base_url
    );
    let outcome = match drive::drive(&transport, &workload, Recorder::new()).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error driving workload: {e}");
            let _ = sampler.stop().await;
            return 2;
        }
    };

    let run_series = sampler.stop().await.unwrap_or_default();

    // Storage probe (compose-managed db only).
    let storage = if let Some(container) = &args.db_container {
        let db = DbAccess {
            container: container.clone(),
            user: args.db_user.clone(),
            db: args.db_name.clone(),
        };
        sample::probe_storage(&db).await.map(|bytes| {
            let comps = seeded_compositions.unwrap_or_else(|| scale.compositions());
            StorageBlock::new(bytes, comps)
        })
    } else {
        None
    };

    let results = build_results(
        &descriptor,
        &spec,
        scale,
        &workload.lock,
        &outcome,
        &args,
        &idle,
        &run_series,
        storage,
    );

    let out_dir = args.out.join(&descriptor.name);
    if let Err(e) = report::write_all(&results, &out_dir) {
        eprintln!("error writing artefacts: {e}");
        return 2;
    }
    eprintln!(
        "bench: {} requests · {} errors · {:.1} req/s · error rate {:.3}% → {}",
        outcome.requests,
        outcome.errors,
        outcome.rps,
        outcome.error_rate * 100.0,
        out_dir.display()
    );

    if outcome.error_rate > ERROR_RATE_FLAG {
        eprintln!(
            "bench: error rate {:.3}% exceeds the {:.1}% flag",
            outcome.error_rate * 100.0,
            ERROR_RATE_FLAG * 100.0
        );
        return 1;
    }
    0
}

#[allow(clippy::too_many_arguments)]
fn build_results(
    descriptor: &SutDescriptor,
    spec: &WorkloadSpec,
    scale: Scale,
    lock: &str,
    outcome: &DriveOutcome,
    args: &RunArgs,
    idle: &[ContainerSeries],
    run_series: &[ContainerSeries],
    storage: Option<StorageBlock>,
) -> Results {
    let classes: BTreeMap<String, ClassRecord> = outcome
        .recorder
        .summaries()
        .iter()
        .map(|(key, summary)| ((*key).to_owned(), ClassRecord::from_summary(summary)))
        .collect();

    let app = args
        .app_container
        .as_deref()
        .and_then(|name| summarize(name, idle, run_series));
    let db = args
        .db_container
        .as_deref()
        .and_then(|name| summarize(name, idle, run_series));

    Results {
        sut: SutBlock {
            name: descriptor.name.clone(),
            kind: sut_kind_label(descriptor.kind).to_owned(),
            base_url: descriptor.base_url.clone(),
            product_label: descriptor.product_label.clone(),
            image_digests: BTreeMap::new(),
            versions: BTreeMap::new(),
        },
        workload: WorkloadBlock {
            lock: lock.to_owned(),
            profile: spec.profile.key().to_owned(),
            scale: scale.key().to_owned(),
            ward_size: spec.ward_size,
            load_factor: spec.load_factor,
            seed: spec.seed,
        },
        environment: EnvironmentBlock::capture(),
        classes,
        throughput: ThroughputBlock {
            window_s: outcome.window_s,
            requests: outcome.requests,
            rps: outcome.rps,
            error_rate: outcome.error_rate,
        },
        resources: ResourcesBlock {
            app,
            db,
            cold_start_ms: args.cold_start_ms,
        },
        storage,
        reproduce: reproduce_command(descriptor, spec, scale),
        excluded_templates: outcome.excluded_templates.clone(),
    }
}

/// Summarize a container's run series with its idle baseline.
fn summarize(
    name: &str,
    idle: &[ContainerSeries],
    run: &[ContainerSeries],
) -> Option<ContainerSummary> {
    let run_series = run.iter().find(|s| s.name == name)?;
    let idle_series = idle.iter().find(|s| s.name == name);
    Some(ContainerSummary::from_series(run_series, idle_series))
}

/// The exact command that reproduces this run (register 01 §5 reproduce-it).
fn reproduce_command(descriptor: &SutDescriptor, spec: &WorkloadSpec, scale: Scale) -> String {
    let sut = match descriptor.kind {
        SutKind::Ours => "ehrbase-rs",
        SutKind::Foreign => &descriptor.name,
    };
    format!(
        "cargo run -q -p benchmark --bin bench -- run --sut {sut} --base-url {} \
         --profile {} --scale {} --ward-size {} --load-factor {} --seed {}",
        descriptor.base_url,
        spec.profile.key(),
        scale.key(),
        spec.ward_size,
        spec.load_factor,
        spec.seed,
    )
}

// The knee command is a linear pipeline (descriptor → transport → seed once →
// ladder of build_capacity + drive → write) whose steps read best in sequence.
#[allow(clippy::too_many_lines)]
async fn cmd_knee(args: KneeArgs) -> i32 {
    let descriptor = match build_descriptor(
        args.sut,
        args.base_url.clone(),
        args.sut_name.clone(),
        args.sut_version.as_deref(),
        args.auth.clone(),
        args.admin_auth.clone(),
    ) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let transport = match build_transport(&descriptor) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let scale = Scale::from(args.scale);

    // Normalize the ladder: positive load factors, ascending, deduplicated.
    let mut ladder: Vec<f64> = args.steps.iter().copied().filter(|l| *l > 0.0).collect();
    ladder.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ladder.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);
    if ladder.is_empty() {
        eprintln!("error: --steps must list at least one positive load factor");
        return 2;
    }

    // Provision + seed happen ONCE: seeding here, provisioning on the first
    // drive (re-applied idempotently at each later step).
    if !args.no_seed && scale.compositions() > 0 {
        eprintln!("bench: seeding scale rung {} …", scale.key());
        if let Err(e) = seed::seed_scale(&transport, scale, args.seed).await {
            eprintln!("error seeding: {e}");
            return 2;
        }
    }

    let step_window = Duration::from_secs(args.step_window);
    let warmup = Duration::from_secs(args.warmup);

    let mut steps_out: Vec<KneeStep> = Vec::new();
    let mut knee: Option<KneeStep> = None;
    for (step_index, load_factor) in ladder.into_iter().enumerate() {
        let spec = WorkloadSpec {
            profile: Profile::Hour,
            ward_size: args.ward_size,
            load_factor,
            // Every step admits a FRESH ward: subject ids derive from the
            // seed, and both first-class SUTs enforce one EHR per subject
            // (RM ehr master04 §EHR Status) — re-running the same subjects
            // 409s every admission and cascades the whole step.
            seed: args.seed.wrapping_add(step_index as u64),
        };
        let workload = match model::build_capacity(&spec, step_window, warmup) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("error building capacity workload: {e}");
                return 2;
            }
        };
        eprintln!(
            "bench: capacity step L={load_factor} (window {}s, warmup {}s) against {}",
            args.step_window, args.warmup, descriptor.base_url
        );
        let outcome = match drive::drive(&transport, &workload, Recorder::new()).await {
            Ok(o) => o,
            Err(e) => {
                eprintln!("error driving workload: {e}");
                return 2;
            }
        };
        let p99_us = outcome.recorder.overall_p99_us();
        let step = KneeStep {
            load_factor,
            rps: outcome.rps,
            error_rate: outcome.error_rate,
            p99_us,
            requests: outcome.requests,
            max_dispatch_lag_ms: outcome.max_dispatch_lag_ms,
        };
        eprintln!(
            "bench: L={load_factor} → {:.1} req/s · p99 {p99_us} µs · error rate {:.3}% · {} requests · dispatch lag {} ms{}",
            step.rps,
            step.error_rate * 100.0,
            step.requests,
            step.max_dispatch_lag_ms,
            if step.generator_bound() {
                " — GENERATOR-BOUND"
            } else {
                ""
            }
        );
        let saturated = report::knee::ladder_should_stop(p99_us, outcome.error_rate);
        steps_out.push(step.clone());
        if saturated {
            eprintln!(
                "bench: SLO breached at L={load_factor} (p99 {p99_us} µs / error {:.3}%) — ladder stops",
                outcome.error_rate * 100.0
            );
            break;
        }
        knee = Some(step);
    }

    let results = KneeResults {
        sut: SutBlock {
            name: descriptor.name.clone(),
            kind: sut_kind_label(descriptor.kind).to_owned(),
            base_url: descriptor.base_url.clone(),
            product_label: descriptor.product_label.clone(),
            image_digests: BTreeMap::new(),
            versions: BTreeMap::new(),
        },
        scale: scale.key().to_owned(),
        steps: steps_out,
        knee,
    };

    let out_dir = args.out.join(&descriptor.name);
    if let Err(e) = report::knee::write_all(&results, &out_dir) {
        eprintln!("error writing artefacts: {e}");
        return 2;
    }
    match &results.knee {
        Some(step) => eprintln!(
            "bench: knee = L {} → {:.1} req/s at p99 {} µs → {}",
            step.load_factor,
            step.rps,
            step.p99_us,
            out_dir.display()
        ),
        None => eprintln!(
            "bench: no sustainable step (first ladder step saturated) → {}",
            out_dir.display()
        ),
    }
    0
}

async fn cmd_seed(args: SeedArgs) -> i32 {
    let descriptor = match build_descriptor(
        args.sut,
        args.base_url,
        args.sut_name,
        args.sut_version.as_deref(),
        args.auth,
        args.admin_auth,
    ) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let transport = match build_transport(&descriptor) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let scale = Scale::from(args.scale);
    match seed::seed_scale(&transport, scale, args.seed).await {
        Ok(n) => {
            eprintln!("bench: seeded {n} compositions (scale {})", scale.key());
            0
        }
        Err(e) => {
            eprintln!("seed error: {e}");
            2
        }
    }
}

fn cmd_report(args: &ReportArgs) -> i32 {
    let results = match report::from_results_file(&args.from) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error reading {}: {e}", args.from.display());
            return 2;
        }
    };
    let out_dir = args.out.join(&results.sut.name);
    if let Err(e) = report::write_all(&results, &out_dir) {
        eprintln!("error writing artefacts: {e}");
        return 2;
    }
    eprintln!(
        "regenerated {} from {}",
        out_dir.display(),
        args.from.display()
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_args_parse_with_defaults() {
        let cli = Cli::try_parse_from([
            "bench",
            "run",
            "--sut",
            "ehrbase-rs",
            "--profile",
            "smoke",
            "--scale",
            "empty",
        ])
        .expect("parse");
        let Command::Run(args) = cli.command else {
            panic!("expected run");
        };
        assert!(matches!(args.sut, SutArg::EhrbaseRs));
        assert!(matches!(args.profile, ProfileArg::Smoke));
        assert!(matches!(args.scale, ScaleArg::Empty));
        assert_eq!(args.ward_size, 20);
        assert!((args.load_factor - 1.0).abs() < f64::EPSILON);
        assert_eq!(args.seed, DEFAULT_SEED);
    }

    #[test]
    fn scale_values_accept_the_ladder_spellings() {
        for (arg, expect) in [
            ("empty", Scale::Empty),
            ("10k", Scale::TenK),
            ("100k", Scale::HundredK),
            ("1m", Scale::OneM),
        ] {
            let cli = Cli::try_parse_from([
                "bench",
                "run",
                "--sut",
                "byo",
                "--base-url",
                "http://x/v1",
                "--profile",
                "hour",
                "--scale",
                arg,
            ])
            .expect("parse");
            let Command::Run(args) = cli.command else {
                panic!("expected run");
            };
            assert_eq!(Scale::from(args.scale), expect);
        }
    }

    #[test]
    fn byo_without_base_url_is_rejected_at_descriptor_build() {
        let err = build_descriptor(SutArg::Byo, None, None, None, None, None).unwrap_err();
        assert!(err.contains("--base-url"));
    }

    #[test]
    fn ehrbase_rs_defaults_its_base_url() {
        let d =
            build_descriptor(SutArg::EhrbaseRs, None, None, None, None, None).expect("descriptor");
        assert!(d.base_url.contains("localhost:8080"));
        assert_eq!(d.name, "ehrbase-rs");
    }

    #[test]
    fn knee_args_parse_with_defaults() {
        let cli = Cli::try_parse_from(["bench", "knee", "--sut", "ehrbase-rs", "--scale", "10k"])
            .expect("parse");
        let Command::Knee(args) = cli.command else {
            panic!("expected knee");
        };
        assert_eq!(Scale::from(args.scale), Scale::TenK);
        assert_eq!(args.steps, vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0]);
        assert_eq!(args.step_window, 120);
        assert_eq!(args.warmup, 15);
        assert_eq!(args.ward_size, 20);
        assert_eq!(args.seed, DEFAULT_SEED);
    }

    #[test]
    fn knee_steps_override_parses_the_comma_ladder() {
        let cli = Cli::try_parse_from([
            "bench",
            "knee",
            "--sut",
            "byo",
            "--base-url",
            "http://x/v1",
            "--scale",
            "empty",
            "--steps",
            "1,3,9",
            "--step-window",
            "60",
            "--warmup",
            "10",
        ])
        .expect("parse");
        let Command::Knee(args) = cli.command else {
            panic!("expected knee");
        };
        assert_eq!(args.steps, vec![1.0, 3.0, 9.0]);
        assert_eq!(args.step_window, 60);
        assert_eq!(args.warmup, 10);
    }

    #[test]
    fn compare_accepts_knee_from() {
        let cli = Cli::try_parse_from([
            "bench",
            "compare",
            "--from",
            "a.json",
            "--from",
            "b.json",
            "--knee-from",
            "a-knee.json",
            "--knee-from",
            "b-knee.json",
        ])
        .expect("parse");
        let Command::Compare(args) = cli.command else {
            panic!("expected compare");
        };
        assert_eq!(args.from.len(), 2);
        assert_eq!(args.knee_from.len(), 2);
    }

    #[test]
    fn seed_args_parse() {
        let cli = Cli::try_parse_from(["bench", "seed", "--sut", "ehrbase-rs", "--scale", "10k"])
            .expect("parse");
        let Command::Seed(args) = cli.command else {
            panic!("expected seed");
        };
        assert_eq!(Scale::from(args.scale), Scale::TenK);
    }
}
