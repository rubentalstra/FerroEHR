//! The ECC conformance CLI (design v4).
//!
//! ```text
//! conformance run   [--base-url URL | --self-host] [--filter S] [--profile core|standard|options]
//!                   [--format json|xml|both] [--out DIR] [--auth SPEC] [--admin-auth SPEC]
//! conformance list  [--filter S]
//! conformance report --from results.json [--out DIR]
//! ```
//!
//! Exit codes: `0` all selected cases pass · `1` failures (report still written)
//! · `2` runner/SUT error.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use conformance::case::{Format, Profile};
use conformance::catalog::Catalog;
use conformance::client::Credential;
use conformance::registry::registry;
use conformance::run::RunConfig;
use conformance::sut::Sut;
use conformance::version::SpecVersions;
use conformance::{report, run};

/// The ehrbase-rs Conformance Catalogue (ECC) runner.
#[derive(Debug, Parser)]
#[command(
    name = "conformance",
    about = "ehrbase-rs conformance catalogue (ECC) runner"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the selected cases against a SUT and write the report set.
    Run(RunArgs),
    /// List the catalogue cases (with per-area totals).
    List(ListArgs),
    /// Regenerate the Markdown/badge artifacts from an existing results.json.
    Report(ReportArgs),
}

#[derive(Debug, Parser)]
struct RunArgs {
    /// External SUT base URL (ITS-REST base path), e.g.
    /// `http://localhost:8080/ehrbase/rest/openehr/v1`.
    #[arg(long)]
    base_url: Option<String>,
    /// Boot an in-process self-hosted SUT (requires the `self-host` feature).
    #[arg(long)]
    self_host: bool,
    /// Only run cases whose id contains this substring.
    #[arg(long)]
    filter: Option<String>,
    /// Only run cases required by this profile.
    #[arg(long)]
    profile: Option<ProfileArg>,
    /// The wire format(s) to run.
    #[arg(long, default_value = "json")]
    format: FormatArg,
    /// Where to write the report set.
    #[arg(long, default_value = "docs/conformance")]
    out: PathBuf,
    /// Regular-user credential (`basic:<user>:<pass>` or `bearer:<token>`).
    #[arg(long)]
    auth: Option<String>,
    /// ADMIN-role credential (same forms as `--auth`).
    #[arg(long)]
    admin_auth: Option<String>,
}

#[derive(Debug, Parser)]
struct ListArgs {
    /// Only list cases whose id contains this substring.
    #[arg(long)]
    filter: Option<String>,
}

#[derive(Debug, Parser)]
struct ReportArgs {
    /// The results.json to regenerate artifacts from.
    #[arg(long)]
    from: PathBuf,
    /// Where to write the report set.
    #[arg(long, default_value = "docs/conformance")]
    out: PathBuf,
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

#[tokio::main]
async fn main() {
    let code = match Cli::parse().command {
        Command::Run(args) => cmd_run(args).await,
        Command::List(args) => cmd_list(&args),
        Command::Report(args) => cmd_report(&args),
    };
    std::process::exit(code);
}

/// Build the SUT (external or self-hosted). Returns an auth-mode label for the
/// statement alongside the SUT.
// `async` is load-bearing under `--features self-host` (awaits container boot);
// without the feature the body has no await, which is expected.
#[cfg_attr(not(feature = "self-host"), allow(clippy::unused_async))]
async fn build_sut(args: &RunArgs) -> Result<(Sut, String), String> {
    if args.self_host {
        #[cfg(feature = "self-host")]
        {
            let sut = Sut::self_hosted().await.map_err(|e| e.to_string())?;
            return Ok((sut, "basic (self-host, RBAC off)".to_owned()));
        }
        #[cfg(not(feature = "self-host"))]
        {
            return Err("--self-host requires building with `--features self-host`".to_owned());
        }
    }
    let base_url = args
        .base_url
        .clone()
        .ok_or_else(|| "one of --base-url or --self-host is required".to_owned())?;
    let regular = args.auth.as_deref().map(Credential::parse).transpose()?;
    let admin = args
        .admin_auth
        .as_deref()
        .map(Credential::parse)
        .transpose()?;
    let auth_mode = args
        .auth
        .as_deref()
        .and_then(|s| s.split_once(':').map(|(scheme, _)| scheme.to_owned()))
        .unwrap_or_else(|| "none".to_owned());
    let sut = Sut::external(base_url, regular, admin).map_err(|e| e.to_string())?;
    Ok((sut, auth_mode))
}

async fn cmd_run(args: RunArgs) -> i32 {
    let (sut, auth_mode) = match build_sut(&args).await {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let config = RunConfig {
        filter: args.filter.clone(),
        profile: args.profile.map(Profile::from),
        formats: args.format.formats(),
        versions: SpecVersions::latest(),
        auth_mode,
    };
    let results = match run::run(sut.transport(), &config).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    if let Err(e) = report::write_all(&results, &args.out) {
        eprintln!("error writing report: {e}");
        return 2;
    }
    println!(
        "conformance: {} executed · {} passed · {} failed → {}",
        results.executed(),
        results.passed(),
        results.failed(),
        args.out.display()
    );
    results.exit_code()
}

fn cmd_list(args: &ListArgs) -> i32 {
    let reg = registry();
    let catalog = match Catalog::load_default() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error loading catalogue: {e}");
            return 2;
        }
    };
    let mut shown = 0;
    for entry in reg.entries() {
        let meta = &entry.meta;
        let ecc = catalog
            .by_primary_ref(meta.id)
            .map(|e| e.ecc_id.clone())
            .unwrap_or_default();
        if let Some(filter) = &args.filter
            && !meta.id.contains(filter.as_str())
            && !ecc.contains(filter.as_str())
        {
            continue;
        }
        println!(
            "{ecc:<14} {:<5} {:?} formats={:?} {} — {}",
            meta.area.tag(),
            meta.capability,
            meta.formats,
            meta.id,
            meta.title,
        );
        shown += 1;
    }
    println!(
        "\n{shown} shown · {} registered · {} catalogued",
        reg.entries().len(),
        catalog.entries().len()
    );
    0
}

fn cmd_report(args: &ReportArgs) -> i32 {
    match report::from_results_file(&args.from) {
        Ok(results) => {
            if let Err(e) = report::write_all(&results, &args.out) {
                eprintln!("error writing report: {e}");
                return 2;
            }
            println!(
                "regenerated {} from {}",
                args.out.display(),
                args.from.display()
            );
            0
        }
        Err(e) => {
            eprintln!("error reading {}: {e}", args.from.display());
            2
        }
    }
}
