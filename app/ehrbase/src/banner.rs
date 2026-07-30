//! The startup ASCII-art banner printed to stdout before the structured
//! startup logs, in the spirit of the reference implementation's Spring banner.
//!
//! No openEHR spec governs this — it is our own presentation. The wordmark is a
//! fixed, hand-committed static string (generated once with a `FIGlet`
//! "standard" font and vendored here), so the boot path carries **zero** runtime
//! dependency and no font-asset load for a fixed piece of art. The version is
//! substituted from `CARGO_PKG_VERSION` at build time; the load-bearing spec
//! pins are read from [`crate::telemetry::provenance`] — the derived
//! crate-version constants — never re-typed here.

use std::fmt::Write as _;

/// The `EHRbase-rs` ASCII wordmark (`FIGlet` "standard" font, five lines,
/// ≤ 55 columns). A raw string so the backslashes/backticks in the art are
/// literal.
const WORDMARK: &str = r"
 _____ _   _ ____  _
| ____| | | |  _ \| |__   __ _ ___  ___       _ __ ___
|  _| | |_| | |_) | '_ \ / _` / __|/ _ \_____| '__/ __|
| |___|  _  |  _ <| |_) | (_| \__ \  __/_____| |  \__ \
|_____|_| |_|_| \_\_.__/ \__,_|___/\___|     |_|  |___/";

/// The project's public repository.
const PROJECT_URL: &str = "https://github.com/rubentalstra/ehrbase-rs";

/// The load-bearing spec/platform pins, one per line — each version read
/// from the shared [`crate::telemetry::provenance`] constants (themselves
/// the `openehr-*` crate versions), so the banner can never drift from the
/// actual pins. `(label, version)` pairs, aligned when rendered.
const PINS: &[(&str, &str)] = &[
    ("openEHR RM", crate::telemetry::provenance::RM),
    ("ITS-REST", crate::telemetry::provenance::ITS_REST),
    ("AQL", crate::telemetry::provenance::AQL),
    ("PostgreSQL", crate::telemetry::provenance::PG_TARGET),
];

/// Render the full banner for the given product `version`.
///
/// Kept version-parameterized (rather than reading `CARGO_PKG_VERSION`
/// directly) so it is unit-testable; [`print()`] supplies the real version.
#[must_use]
pub fn render(version: &str) -> String {
    let mut out = format!(
        "{WORDMARK}\n\n  \
         openEHR-conformant Clinical Data Repository · v{version}\n  \
         Maintained by Ruben Talstra · {PROJECT_URL}\n\n"
    );
    for (label, pin) in PINS {
        // Left-pad the version column so the pins line up as a list.
        let _ = writeln!(out, "  {label:<12}{pin}");
    }
    out
}

/// Print the banner to stdout. Called from the binary before telemetry/log
/// initialisation so the structured formatter never mangles the art.
#[expect(
    clippy::print_stdout,
    reason = "the boot banner IS console output, and it prints before any \
              tracing subscriber exists"
)]
pub fn print() {
    println!("{}", render(env!("CARGO_PKG_VERSION")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_contains_version_maintainer_and_url() {
        let b = render("9.9.9");
        assert!(b.contains("v9.9.9"), "version must be substituted");
        assert!(b.contains("Ruben Talstra"), "maintainer credit must appear");
        assert!(
            b.contains("https://github.com/rubentalstra/ehrbase-rs"),
            "project URL must appear"
        );
        // The load-bearing pins (docs/VERSIONS.md), each on its own line.
        for (label, pin) in PINS {
            assert!(b.contains(label), "pin label {label:?} must appear");
            assert!(b.contains(pin), "pin version {pin:?} must appear");
        }
        assert!(b.contains("openEHR RM"));
        assert!(b.contains("PostgreSQL"));
    }

    #[test]
    fn banner_lines_are_within_100_chars() {
        for line in render(env!("CARGO_PKG_VERSION")).lines() {
            assert!(
                line.chars().count() <= 100,
                "banner line exceeds 100 chars ({}): {line:?}",
                line.chars().count()
            );
        }
    }

    #[test]
    fn banner_uses_real_package_version() {
        assert!(render(env!("CARGO_PKG_VERSION")).contains(env!("CARGO_PKG_VERSION")));
    }
}
