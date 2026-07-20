//! Shields.io endpoint-schema badges (`badge.json` + one per profile), written
//! per SUT under the run's out-dir. Colours and messages come from the machine
//! verdicts only — a profile badge can never read PASS unless
//! [`crate::reporting::report::profile_verdict`] does.

use std::path::Path;

use crate::model::case::Profile;
use crate::model::catalog::{Catalog, EccStatus};
use crate::model::profile::{CapabilityEvidence, ProfileVerdict};
use crate::reporting::report::{ReportError, profile_lines, profile_verdict};
use crate::reporting::results::RunResults;

/// Write the badge set (`badge.json` + `badge-core/standard/options.json`)
/// into `out_dir`.
///
/// # Errors
/// [`ReportError::Io`] on write failure.
pub fn write_badges(
    results: &RunResults,
    catalog: &Catalog,
    out_dir: &Path,
) -> Result<(), ReportError> {
    write_file(&out_dir.join("badge.json"), &render_badge(results, catalog))?;
    for profile in [Profile::Core, Profile::Standard, Profile::Options] {
        let name = format!("badge-{}.json", format!("{profile:?}").to_lowercase());
        write_file(&out_dir.join(name), &render_profile_badge(profile, results))?;
    }
    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<(), ReportError> {
    std::fs::write(path, content).map_err(|source| ReportError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// The top-level executed/passed badge (`passed/active`).
#[must_use]
pub fn render_badge(results: &RunResults, catalog: &Catalog) -> String {
    let active = catalog
        .entries()
        .iter()
        .filter(|e| e.status == EccStatus::Active)
        .count();
    let passed = results.passed();
    let failed = results.failed();
    let color = if failed > 0 {
        "red"
    } else if active > 0 && passed >= active {
        "brightgreen"
    } else {
        "yellow"
    };
    let badge = serde_json::json!({
        "schemaVersion": 1,
        "label": "ECC conformance",
        "message": format!("{passed}/{active}"),
        "color": color,
    });
    serde_json::to_string_pretty(&badge).unwrap_or_else(|_| "{}".to_owned())
}

/// One per-profile badge. The message and colour come from the machine
/// all-of/any-of verdict.
#[must_use]
pub fn render_profile_badge(profile: Profile, results: &RunResults) -> String {
    let lines = profile_lines(results, profile);
    let total = lines.len();
    let passing = lines
        .iter()
        .filter(|l| l.evidence == CapabilityEvidence::Passed)
        .count();
    let any_broken = lines
        .iter()
        .any(|l| l.evidence == CapabilityEvidence::Failed);
    let verdict = profile_verdict(results, profile);
    let (message, color) = if verdict == ProfileVerdict::Pass {
        let word = if profile == Profile::Options {
            "OBTAINED"
        } else {
            "PASS"
        };
        (
            format!("{word} ({passing}/{total} capabilities)"),
            "brightgreen",
        )
    } else if any_broken {
        (format!("{passing}/{total} capabilities"), "red")
    } else {
        // Nothing failing, but the claim is not yet evidenced.
        (format!("{passing}/{total} capabilities"), "yellow")
    };
    let badge = serde_json::json!({
        "schemaVersion": 1,
        "label": format!("ECC {}", format!("{profile:?}").to_uppercase()),
        "message": message,
        "color": color,
    });
    serde_json::to_string_pretty(&badge).unwrap_or_else(|_| "{}".to_owned())
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;
    use crate::edition::{Edition, EditionPolicy};
    use crate::model::versions::SpecVersions;
    use crate::reporting::results::CaseStatus;
    use crate::reporting::results::{
        CaseOutcome, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity,
    };
    use crate::sut::descriptor::SutKind;

    fn results(cases: Vec<CaseOutcome>) -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "http://sut".to_owned(),
                product: ProductIdentity::default(),
                kind: SutKind::Ours,
                edition_policy: EditionPolicy::Pinned(Edition::Development),
                versions: SpecVersions::latest(),
                auth_mode: "none".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: "2026-07-13T00:00:00Z".to_owned(),
            selection: SelectionInfo::default(),
            terminology: None,
            cases,
        }
    }

    fn outcome(capability: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: "ECC-EHR-001".to_owned(),
            id: "slug".to_owned(),
            title: "t".to_owned(),
            capability: capability.to_owned(),
            format: "json".to_owned(),
            status,
            passed_data_sets: 1,
            total_data_sets: 1,
            schedule_rows: None,
            message: None,
            citation: String::new(),
            schedule_ref: None,
            ecc_original: None,
            binding: String::new(),
            edition_level: None,
            edition_findings: Vec::new(),
            duration_ms: 0,
        }
    }

    #[test]
    fn zero_state_badge_is_yellow() {
        let badge = render_badge(&results(Vec::new()), &Catalog::default());
        assert!(badge.contains("\"message\": \"0/0\""));
        assert!(badge.contains("yellow"));
    }

    #[test]
    fn options_badge_obtained_when_one_optional_passes() {
        let r = results(vec![outcome("AdminPhysicalDeletion", CaseStatus::Passed)]);
        let badge = render_profile_badge(Profile::Options, &r);
        assert!(badge.contains("OBTAINED"));
        assert!(badge.contains("brightgreen"));
    }
}
