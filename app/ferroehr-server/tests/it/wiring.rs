// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The binary's command-line seam: `--set` override parsing, the subcommand
//! shapes, and the one dispatch branch of `run` that touches nothing external.
//!
//! `Cli`'s fields are private by design (visibility is deliberate — the type is
//! a `clap` parse target, not a record), so a parse result is observed through
//! its derived `Debug` rendering, the surface the type actually offers.

#![expect(
    clippy::panic_in_result_fn,
    reason = "the blessed test shape (the Rust Book ch11-01): `?` propagates \
              plumbing failures — here the clap parse — while the assertion \
              carries the behaviour under test and is meant to panic"
)]

use clap::Parser as _;

use ferroehr_server::{Cli, run};

/// A `--set key=value` pair parses into the override list.
#[test]
fn set_override_accepts_a_key_value_pair() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from(["ferroehr", "--set", "db.max_connections=40"])?;
    assert!(
        format!("{cli:?}").contains(r#"("db.max_connections", "40")"#),
        "parsed override missing: {cli:?}"
    );
    Ok(())
}

/// `--set` is repeatable and keeps every pair, in the order given (the loader
/// applies them in sequence, so order is behaviour).
#[test]
fn set_override_is_repeatable_and_ordered() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from([
        "ferroehr",
        "--set",
        "db.max_connections=40",
        "--set",
        "server.bind=0.0.0.0:9000",
    ])?;
    let rendered = format!("{cli:?}");
    let first = rendered.find(r#"("db.max_connections", "40")"#);
    let second = rendered.find(r#"("server.bind", "0.0.0.0:9000")"#);
    assert!(
        matches!((first, second), (Some(a), Some(b)) if a < b),
        "overrides lost or reordered: {rendered}"
    );
    Ok(())
}

/// The key is trimmed and only the FIRST `=` separates key from value, so a
/// value may itself contain `=` (a DSN query string, a base64 tail).
#[test]
fn set_override_splits_on_the_first_equals_and_trims_the_key() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from(["ferroehr", "--set", "  db.url =postgres://h/db?a=b"])?;
    assert!(
        format!("{cli:?}").contains(r#"("db.url", "postgres://h/db?a=b")"#),
        "unexpected split: {cli:?}"
    );
    Ok(())
}

/// A `--set` argument without `=` is rejected, naming the expected form.
#[test]
fn set_override_rejects_a_pair_without_an_equals() {
    let err = Cli::try_parse_from(["ferroehr", "--set", "db.max_connections"])
        .expect_err("a bare key must not parse");
    let rendered = err.to_string();
    assert!(
        rendered.contains("expected key=value"),
        "unhelpful rejection: {rendered}"
    );
}

/// `ferroehr` with no subcommand is the serve path (`command: None`).
#[test]
fn no_subcommand_selects_the_serve_path() -> Result<(), clap::Error> {
    let cli = Cli::try_parse_from(["ferroehr"])?;
    assert!(
        format!("{cli:?}").contains("command: None"),
        "expected no subcommand: {cli:?}"
    );
    Ok(())
}

/// `--config` is global: accepted before or after the subcommand.
#[test]
fn config_path_is_global() -> Result<(), clap::Error> {
    const PATH: &str = "/etc/ferroehr/ferroehr.toml";
    for args in [
        ["ferroehr", "--config", PATH, "config", "check"],
        ["ferroehr", "config", "check", "--config", PATH],
    ] {
        let cli = Cli::try_parse_from(args)?;
        assert!(
            format!("{cli:?}").contains(PATH),
            "config path lost for {args:?}: {cli:?}"
        );
    }
    Ok(())
}

/// Both `config` utilities parse to their own variant.
#[test]
fn config_subcommands_parse() -> Result<(), clap::Error> {
    let default = Cli::try_parse_from(["ferroehr", "config", "default"])?;
    assert!(
        format!("{default:?}").contains("Default"),
        "not the Default utility: {default:?}"
    );
    let check = Cli::try_parse_from(["ferroehr", "config", "check"])?;
    assert!(
        format!("{check:?}").contains("Check"),
        "not the Check utility: {check:?}"
    );
    Ok(())
}

/// `config` without a utility is rejected (the inner subcommand is required).
#[test]
fn config_without_a_utility_is_rejected() {
    assert!(
        Cli::try_parse_from(["ferroehr", "config"]).is_err(),
        "`config` must require a utility"
    );
}

/// `healthcheck` takes an explicit URL and otherwise defaults to the local
/// status endpoint.
#[test]
fn healthcheck_url_is_optional_with_a_default() -> Result<(), clap::Error> {
    let explicit = Cli::try_parse_from(["ferroehr", "healthcheck", "--url", "http://h:8080/x"])?;
    assert!(
        format!("{explicit:?}").contains("http://h:8080/x"),
        "explicit URL lost: {explicit:?}"
    );
    let defaulted = Cli::try_parse_from(["ferroehr", "healthcheck"])?;
    assert!(
        format!("{defaulted:?}").contains("/ferroehr/rest/status"),
        "default URL missing: {defaulted:?}"
    );
    Ok(())
}

/// An unknown subcommand is rejected rather than silently falling through to
/// the serve path.
#[test]
fn unknown_subcommand_is_rejected() {
    assert!(
        Cli::try_parse_from(["ferroehr", "migrate"]).is_err(),
        "an unknown subcommand must not parse"
    );
}

/// `ferroehr config default` runs end to end through the real dispatch: it only
/// writes the annotated template to stdout, so it needs no database, listener,
/// or network.
#[tokio::test]
async fn run_config_default_is_a_pure_stdout_path() -> anyhow::Result<()> {
    let cli = Cli::try_parse_from(["ferroehr", "config", "default"])?;
    run(cli).await
}
