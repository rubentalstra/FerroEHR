//! Capture build-time provenance for `/management/info` and the
//! `ehrbase_build_info` gauge: the git commit, the build timestamp, and the
//! `rustc` version. All are best-effort — a checkout without git, or a build
//! from a tarball, degrades to `unknown` rather than failing the build.

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Git short SHA: an explicit CI-provided value wins; otherwise ask git.
    let git_sha = std::env::var("EHRBASE_GIT_SHA")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=12", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=EHRBASE_GIT_SHA={git_sha}");

    // Build timestamp (epoch seconds), honouring SOURCE_DATE_EPOCH for
    // reproducible builds; rendered to an ISO-8601 string at runtime (jiff).
    let epoch = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|d| i64::try_from(d.as_secs()).ok())
                .unwrap_or(0)
        });
    println!("cargo:rustc-env=EHRBASE_BUILD_EPOCH={epoch}");

    // rustc version string.
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let rustc_version = Command::new(&rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or_else(
            || "unknown".to_owned(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_owned(),
        );
    println!("cargo:rustc-env=EHRBASE_RUSTC={rustc_version}");

    println!("cargo:rerun-if-env-changed=EHRBASE_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");
}
