//! The ECC conformance CLI (W-10 redesign).
//!
//! ```text
//! conformance run     --base-url URL [--sut ehrbase-rs|ehrbase-java|byo] [--sut-name NAME]
//!                     [--sut-version VER] [--auth SPEC] [--admin-auth SPEC] [--admin-base-url URL]
//!                     [--edition auto|development|1.0.3] [--filter S]
//!                     [--profile core|standard|options] [--format json|xml|both] [--out DIR]
//!                     [--fairness-register FILE] [--own-adjudications FILE] [--tx-server-url URL]
//! conformance report  --from results.json [--out DIR]
//! conformance compare --from a.json --from b.json [--from …] [--out FILE]
//! conformance catalog [--from results.json] [--out FILE]
//! ```
//!
//! `run` builds a [`SutDescriptor`] (a built-in first-class target or a
//! bring-your-own endpoint), probes it, executes the case universe, and writes
//! the per-SUT artefact set (`results.json`, `CONFORMANCE_REPORT.md`,
//! `CONFORMANCE_STATEMENT.md`, `CONFORMANCE_CERTIFICATE.md` for our own SUT
//! only, badges) into `--out/<sut-name>/`. Exit codes: `0` all pass · `1`
//! failures (artefacts still written) · `2` runner/SUT error.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use conformance::adjudication::OwnRegister;
use conformance::case::{Format, Profile};
use conformance::catalog::{Area, Catalog};
use conformance::edition::{Edition, EditionPolicy};
use conformance::fairness::AdjudicationRegister;
use conformance::profile::required_capabilities;
use conformance::registry::registry;
use conformance::reporting::results::{RunResults, TerminologyRun};
use conformance::reporting::{compare, report};
use conformance::run::RunConfig;
use conformance::sut::builtin;
use conformance::sut::descriptor::{SutDescriptor, SutKind};
use conformance::transport::{Credential, SutClient};
use conformance::ts::{FhirTxFixture, TxServer};
use conformance::versions::SpecVersions;
use conformance::{edition, run};

/// The ehrbase-rs Conformance Catalogue (ECC) runner.
#[derive(Debug, Parser)]
#[command(
    name = "conformance",
    about = "openEHR CNF conformance runner (any ITS-REST CDR)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
// `RunArgs` is the large variant (a full CLI arg set); boxing it fights clap's
// derive, and the command enum is constructed once, so the asymmetry is moot.
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Run the selected cases against a SUT and write its artefact set.
    Run(RunArgs),
    /// Regenerate the artefact set from an existing results.json.
    Report(ReportArgs),
    /// Render the cross-SUT comparison matrix from two or more results.json.
    Compare(CompareArgs),
    /// Render the ECC catalogue (optionally annotated with a run's outcomes).
    Catalog(CatalogArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
    /// The SUT's ITS-REST base URL, e.g.
    /// `http://localhost:8080/ehrbase/rest/openehr/v1`.
    #[arg(long)]
    base_url: Option<String>,
    /// The target class: a built-in first-class target or a bring-your-own
    /// endpoint. `ehrbase-rs` pins the development edition; the others default
    /// to the auto ladder.
    #[arg(long, value_enum, default_value_t = SutArg::EhrbaseRs)]
    sut: SutArg,
    /// Override the output/lookup name (defaults: the built-in name, or `byo`).
    #[arg(long)]
    sut_name: Option<String>,
    /// The product version label recorded in the identity (used for the
    /// `ehrbase-java` product label; defaults per target).
    #[arg(long)]
    sut_version: Option<String>,
    /// Regular-user credential (`basic:<user>:<pass>` or `bearer:<token>`).
    #[arg(long)]
    auth: Option<String>,
    /// ADMIN-role credential (same forms as `--auth`).
    #[arg(long)]
    admin_auth: Option<String>,
    /// Sibling admin-API base URL (upstream `EHRbase` serves admin at a sibling
    /// mount of `/rest/openehr`); overrides the target default.
    #[arg(long)]
    admin_base_url: Option<String>,
    /// The spec-edition policy: `auto` (ladder), `development`, or `1.0.3`.
    /// Unset keeps the target's default (pinned development for ehrbase-rs).
    #[arg(long, value_enum)]
    edition: Option<EditionArg>,
    /// Only run cases whose id contains this substring.
    #[arg(long)]
    filter: Option<String>,
    /// Only run cases required by this profile.
    #[arg(long, value_enum)]
    profile: Option<ProfileArg>,
    /// The wire format(s) to run.
    #[arg(long, value_enum, default_value_t = FormatArg::Json)]
    format: FormatArg,
    /// Where to write the artefact set (the SUT name is appended).
    #[arg(long, default_value = "docs/conformance")]
    out: PathBuf,
    /// The upstream fairness register (TOML) for a foreign SUT. Unset:
    /// auto-resolve `adjudications/<sut-name>*.toml`. Ignored for ehrbase-rs.
    #[arg(long)]
    fairness_register: Option<PathBuf>,
    /// The own-corpus adjudication register (vendored-data defects).
    #[arg(long, default_value = "adjudications/ecc-own.toml")]
    own_adjudications: PathBuf,
    /// Real FHIR R4 terminology-server base URL for the `TS` cases. Unset:
    /// spin up a hermetic wiremock FHIR-tx fixture (the CI default).
    #[arg(long)]
    tx_server_url: Option<String>,
    /// The assessor attribution on the Conformance Certificate. Unset: the
    /// default self-assessment-via-ECC line.
    #[arg(long)]
    assessor: Option<String>,
}

#[derive(Debug, Parser)]
struct ReportArgs {
    /// The results.json to regenerate artefacts from.
    #[arg(long)]
    from: PathBuf,
    /// Where to write the artefact set.
    #[arg(long, default_value = "docs/conformance")]
    out: PathBuf,
}

#[derive(Debug, Parser)]
struct CompareArgs {
    /// A results.json to include (repeat for each SUT; at least two).
    #[arg(long = "from", required = true)]
    from: Vec<PathBuf>,
    /// Where to write the comparison matrix.
    #[arg(long, default_value = "docs/conformance/COMPARISON.md")]
    out: PathBuf,
}

#[derive(Debug, Parser)]
struct CatalogArgs {
    /// Optionally annotate each case with this run's outcome.
    #[arg(long)]
    from: Option<PathBuf>,
    /// Where to write the catalogue.
    #[arg(long, default_value = "docs/conformance/CATALOG.md")]
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
    Core,
    Standard,
    Options,
}

impl From<ProfileArg> for Profile {
    fn from(p: ProfileArg) -> Self {
        match p {
            ProfileArg::Core => Profile::Core,
            ProfileArg::Standard => Profile::Standard,
            ProfileArg::Options => Profile::Options,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Json,
    Xml,
    Both,
}

impl FormatArg {
    fn formats(self) -> Vec<Format> {
        match self {
            FormatArg::Json => vec![Format::Json],
            FormatArg::Xml => vec![Format::Xml],
            FormatArg::Both => vec![Format::Json, Format::Xml],
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum EditionArg {
    Auto,
    Development,
    #[value(name = "1.0.3", alias = "release-1.0.3")]
    Release103,
}

impl EditionArg {
    fn policy(self) -> EditionPolicy {
        match self {
            EditionArg::Auto => EditionPolicy::Auto,
            EditionArg::Development => EditionPolicy::Pinned(Edition::Development),
            EditionArg::Release103 => EditionPolicy::Pinned(Edition::Release103),
        }
    }
}

#[tokio::main]
async fn main() {
    let code = match Cli::parse().command {
        Command::Run(args) => cmd_run(args).await,
        Command::Report(args) => cmd_report(&args),
        Command::Compare(args) => cmd_compare(&args),
        Command::Catalog(args) => cmd_catalog(&args),
    };
    std::process::exit(code);
}

/// Build the SUT descriptor from the CLI args.
fn build_descriptor(args: &RunArgs) -> Result<SutDescriptor, String> {
    let base_url = args
        .base_url
        .clone()
        .ok_or_else(|| "--base-url is required (see scripts/conformance.sh)".to_owned())?;

    let mut descriptor = match args.sut {
        SutArg::EhrbaseRs => {
            builtin::ehrbase_rs(base_url, args.auth.clone(), args.admin_auth.clone())
        }
        SutArg::EhrbaseJava => {
            let version = args.sut_version.as_deref().unwrap_or("upstream");
            builtin::ehrbase_java(
                base_url,
                args.auth.clone(),
                args.admin_auth.clone(),
                version,
            )
        }
        SutArg::Byo => {
            let mut d = SutDescriptor::byo(args.sut_name.clone(), base_url);
            d.auth.clone_from(&args.auth);
            d.admin_auth.clone_from(&args.admin_auth);
            d.admin_base_url.clone_from(&args.admin_base_url);
            d
        }
    };

    // A `--sut-name` override applies to any target (the output subdir + the
    // fairness-register lookup key).
    if let Some(name) = &args.sut_name {
        descriptor.name.clone_from(name);
    }
    // A `--admin-base-url` override wins over the target default.
    if args.admin_base_url.is_some() {
        descriptor.admin_base_url.clone_from(&args.admin_base_url);
    }
    // A `--edition` override wins over the target default.
    if let Some(edition) = args.edition {
        descriptor.edition_policy = edition.policy();
    }
    Ok(descriptor)
}

/// The auth-mode label recorded in the Statement (the credential scheme).
fn auth_mode_label(descriptor: &SutDescriptor) -> String {
    descriptor
        .auth
        .as_deref()
        .and_then(|s| s.split_once(':').map(|(scheme, _)| scheme.to_owned()))
        .unwrap_or_else(|| "none".to_owned())
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
    let client = SutClient::new(descriptor.base_url.clone(), regular, admin)
        .map_err(|e| e.to_string())?
        .with_admin_base_url(descriptor.admin_base_url.clone());
    Ok(client)
}

/// Whether any registered `TS`-area case is selected (drives whether the
/// hermetic FHIR-tx fixture is worth spinning up). Mirrors the executor's
/// selection: id substring + profile→required-capabilities membership.
fn ts_cases_selected(filter: Option<&str>, profile: Option<Profile>) -> bool {
    registry().entries().iter().any(|e| {
        e.meta.area == Area::Ts
            && filter.is_none_or(|f| e.meta.id.contains(f))
            && profile.is_none_or(|p| required_capabilities(p).contains(&e.meta.capability))
    })
}

/// Establish the terminology server for the `TS` cases: a real server when
/// `--tx-server-url` is given, else — when `TS` cases are in scope — the
/// hermetic wiremock FHIR-tx fixture. Returns the descriptor threaded to cases
/// plus the live fixture (kept alive for the run).
async fn establish_tx(args: &RunArgs) -> (Option<TxServer>, Option<FhirTxFixture>) {
    if let Some(url) = &args.tx_server_url {
        return (Some(TxServer::real(url.clone())), None);
    }
    let profile = args.profile.map(Profile::from);
    if !ts_cases_selected(args.filter.as_deref(), profile) {
        return (None, None);
    }
    let fixture = FhirTxFixture::start_canned().await;
    let base = fixture.base_url();
    if let Err(e) = fixture.self_check().await {
        eprintln!("warning: FHIR-tx fixture self-check failed, running without it: {e}");
        return (None, None);
    }
    (Some(TxServer::fixture(base)), Some(fixture))
}

/// Resolve the fairness register for a foreign SUT: the explicit flag, else the
/// first `adjudications/<name>*.toml` file.
fn resolve_fairness_register(
    explicit: Option<&Path>,
    name: &str,
) -> Result<Option<AdjudicationRegister>, String> {
    let path = match explicit {
        Some(p) => Some(p.to_path_buf()),
        None => auto_fairness_path(name),
    };
    match path {
        Some(p) => {
            let reg = AdjudicationRegister::load(&p).map_err(|e| e.to_string())?;
            println!(
                "conformance: fairness register {} ({} rule(s))",
                p.display(),
                reg.len()
            );
            Ok(Some(reg))
        }
        None => Ok(None),
    }
}

/// Find `adjudications/<name>*.toml`, returning the lexicographically first
/// match (deterministic).
fn auto_fairness_path(name: &str) -> Option<PathBuf> {
    let dir = Path::new("adjudications");
    let mut matches: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(name))
        })
        .collect();
    matches.sort();
    matches.into_iter().next()
}

async fn cmd_run(args: RunArgs) -> i32 {
    let descriptor = match build_descriptor(&args) {
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

    // Pre-run discovery probe (`OPTIONS /`). It never gates a case; it enriches
    // the operator log with what the SUT advertises.
    let probe = edition::probe::probe(&transport).await;
    if let Some(solution) = &probe.solution {
        println!("conformance: SUT advertises solution `{solution}` on OPTIONS /");
    }

    let (tx, fixture) = establish_tx(&args).await;

    // The fairness register applies to foreign SUTs only; run() ignores it for
    // ehrbase-rs, but we also skip loading it so a stray flag is a no-op log.
    let fairness = match descriptor.kind {
        SutKind::Foreign => {
            match resolve_fairness_register(args.fairness_register.as_deref(), &descriptor.name) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("error loading fairness register: {e}");
                    return 2;
                }
            }
        }
        SutKind::Ours => {
            if args.fairness_register.is_some() {
                eprintln!(
                    "conformance: --fairness-register given but the SUT is ehrbase-rs — ignored \
                     (our baseline is never reclassified)"
                );
            }
            None
        }
    };

    let own_adjudications = match OwnRegister::load(&args.own_adjudications) {
        Ok(reg) => reg,
        Err(e) => {
            eprintln!("error loading own-adjudication register: {e}");
            return 2;
        }
    };

    let config = RunConfig {
        filter: args.filter.clone(),
        profile: args.profile.map(Profile::from),
        formats: args.format.formats(),
        versions: SpecVersions::latest(),
        auth_mode: auth_mode_label(&descriptor),
        fairness,
        own_adjudications,
        tx: tx.clone(),
    };

    let mut results = match run::run(&transport, &descriptor, &config).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    // Record the terminology run + the FHIR-tx exchange (the fixture's own
    // self-check plus anything a SUT wired to it sent).
    if let Some(server) = &tx {
        let exchanges = match &fixture {
            Some(fx) => fx.exchanges().await,
            None => Vec::new(),
        };
        results.terminology = Some(TerminologyRun {
            base_url: server.base_url.clone(),
            mode: server.mode.label().to_owned(),
            exchanges,
        });
    }

    let out_dir = args.out.join(&descriptor.name);
    if let Err(e) = report::write_all(&results, &out_dir, args.assessor.as_deref()) {
        eprintln!("error writing artefacts: {e}");
        return 2;
    }
    println!(
        "conformance: {} executed · {} passed · {} failed · {} not-applicable → {}",
        results.executed(),
        results.passed(),
        results.failed(),
        results.not_applicable(),
        out_dir.display()
    );
    results.exit_code()
}

fn cmd_report(args: &ReportArgs) -> i32 {
    let results = match report::from_results_file(&args.from) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error reading {}: {e}", args.from.display());
            return 2;
        }
    };
    if let Err(e) = report::write_all(&results, &args.out, None) {
        eprintln!("error writing artefacts: {e}");
        return 2;
    }
    println!(
        "regenerated {} from {}",
        args.out.display(),
        args.from.display()
    );
    0
}

fn cmd_compare(args: &CompareArgs) -> i32 {
    if args.from.len() < 2 {
        eprintln!("error: compare needs at least two --from results.json inputs");
        return 2;
    }
    let mut runs: Vec<RunResults> = Vec::with_capacity(args.from.len());
    for path in &args.from {
        match report::from_results_file(path) {
            Ok(r) => runs.push(r),
            Err(e) => {
                eprintln!("error reading {}: {e}", path.display());
                return 2;
            }
        }
    }
    let md = compare::render_comparison_md(&runs);
    if let Some(parent) = args.out.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("error creating {}: {e}", parent.display());
        return 2;
    }
    if let Err(e) = std::fs::write(&args.out, md) {
        eprintln!("error writing {}: {e}", args.out.display());
        return 2;
    }
    println!(
        "wrote comparison of {} SUTs → {}",
        runs.len(),
        args.out.display()
    );
    0
}

fn cmd_catalog(args: &CatalogArgs) -> i32 {
    let catalog = match Catalog::load_default() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error loading catalogue: {e}");
            return 2;
        }
    };
    let results = match &args.from {
        Some(path) => match report::from_results_file(path) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("error reading {}: {e}", path.display());
                return 2;
            }
        },
        None => None,
    };
    let md = report::render_catalog_md(results.as_ref(), &catalog);
    if let Some(parent) = args.out.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!("error creating {}: {e}", parent.display());
        return 2;
    }
    if let Err(e) = std::fs::write(&args.out, md) {
        eprintln!("error writing {}: {e}", args.out.display());
        return 2;
    }
    println!("wrote catalogue → {}", args.out.display());
    0
}
