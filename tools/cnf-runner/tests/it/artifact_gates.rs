// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The production artifact tree (`artifacts/`) passes every machine gate —
//! the eight pilot encodings plus their `verified_by` targets, validated
//! against the vendored spec tree.

#![expect(
    clippy::expect_used,
    reason = "test-support helpers (not `#[test]` fns, so the clippy.toml in-tests scoping does not reach them) are panic-idiomatic: a broken fixture must abort the test loudly, Book ch11"
)]

use cnf_runner::artifacts::load_root;
use cnf_runner::validate::{Context, validate};

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_owned()
}

#[test]
fn pilot_world_is_clean_under_all_gates() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let specs = repo_root().join("docs/specs/openehr");
    assert!(
        specs.is_dir(),
        "vendored spec tree missing at {}",
        specs.display()
    );

    let loaded = load_root(&crate_dir.join("artifacts")).expect("schema compilation");
    let findings = validate(&Context {
        set: &loaded.set,
        load_errors: &loaded.errors,
        spec_root: Some(&specs),
    });
    assert!(
        findings.is_empty(),
        "the artifact tree must be clean, found:\n{}",
        findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );

    let ids: Vec<&str> = loaded
        .set
        .cases
        .iter()
        .map(|(_, c)| c.id.as_str())
        .collect();
    for pilot in [
        "I_EHR_SERVICE.create_ehr-main",
        "I_EHR_SERVICE.create_ehr-same_ehr_twice",
        "I_DEFINITION_ADL14.upload_opt-invalid_opt",
        "I_EHR_COMPOSITION.update_composition-event",
        "CONT-DV_QUANTITY-validate_property_units_mag",
        "SF-FLAT-commit_roundtrip_ctx_defaults",
        "I_QUERY_SERVICE.execute_ad_hoc_query-where_magnitude",
        "I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions",
        "I_EHR_STATUS.get_ehr_status-get_by_ehr_id",
        "I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts",
    ] {
        assert!(
            ids.contains(&pilot),
            "pilot {pilot} missing from the schedule"
        );
    }
    assert!(loaded.set.bindings.len() >= 10, "binding set shrank");
}

/// The two 1.1.0-dated header rules are scoped TOTALLY, by ONE mechanism
/// (issue #627): the affected set is DERIVED from the committed bindings —
/// never a hand-kept list that a new binding can quietly escape — and every
/// member carries the `applies: { its_rest: ">=1.1.0" }` floor.
///
/// Both rules are dated by the released text itself, in the one chapter
/// (`ITS-REST specifications/docs/overview/Requests_and_responses.md`):
///
/// * §"Deprecated headers" — "The `ETag` response header was used without a
///   weakness indicator `W/`. This is now deprecated, all `ETag` headers that
///   hold a resource identifier MUST include a weakness indicator `W/`" —
///   with §"`ETag` and Last-Modified" naming the release: "DEPRECATION: Prior to
///   Release 1.1.0, the `ETag` header was used without a weakness indicator
///   `W/`. This usage is now deprecated, but implementations MAY still support
///   it alongside the updated header format".
/// * §Location — "DEPRECATION: Prior to Release 1.1.0, the `Location` header
///   was used to indicate the canonical location of a representation in a
///   response. This usage is now deprecated. The `Location` header MUST ONLY
///   be used for resource creation (e.g., `201 Created`) or redirect
///   responses" — with §"Deprecated headers" naming the two response families
///   it was withdrawn from ("Some of the `GET` methods had a `Location`
///   response header … Similarly, the `Location` response header was
///   deprecated from responses of `DELETE` methods").
///
/// A party declaring an earlier ITS-REST release conforms to the text of that
/// release, so neither rule may be applied to it — and a party declaring
/// 1.1.0 or later must face BOTH on every binding that states them, or the
/// artifact applies one MUST to one product by accident of which case
/// happened to carry a floor for some unrelated reason.
#[test]
fn every_release_dated_header_rule_carries_the_same_floor() {
    use cnf_runner::model::binding::HeaderMatcher;

    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let loaded = load_root(&crate_dir.join("artifacts")).expect("schema compilation");

    let mut dated = 0_usize;
    let mut unscoped: Vec<String> = Vec::new();
    for (path, binding) in &loaded.set.bindings {
        // EXTENSION bindings are exempt: their routes are our own design
        // (no released text dates their headers — the W/ form there is a
        // followed convention, not the 1.1.0 deprecation), and party-scoped
        // selection already keeps them off any party that does not claim
        // the capability, so no 1.0.3 party can be misjudged through them.
        if binding.extension.is_some() {
            continue;
        }
        for (kind, expectation) in binding.outcomes.as_deref().unwrap_or_default() {
            for (header, declared) in expectation.headers.as_deref().unwrap_or_default() {
                // The affected set, derived from the matcher itself: the
                // weak-ETag FORM pin, and the `Location` absent-restriction.
                let release_dated = match &declared.matcher {
                    HeaderMatcher::Pattern(pattern) => pattern.starts_with("W/"),
                    HeaderMatcher::Absent => header.eq_ignore_ascii_case("Location"),
                    _ => false,
                };
                if !release_dated {
                    continue;
                }
                dated += 1;
                let floored = declared.applies.as_ref().is_some_and(|applies| {
                    applies
                        .its_rest
                        .as_ref()
                        .is_some_and(|range| range.raw() == ">=1.1.0")
                });
                if !floored {
                    unscoped.push(format!(
                        "{}: outcome {kind} header {header}",
                        path.display()
                    ));
                }
            }
        }
    }

    assert!(
        unscoped.is_empty(),
        "every 1.1.0-dated header rule declares `applies: {{ its_rest: \">=1.1.0\" }}`; \
         {} do not:\n{}",
        unscoped.len(),
        unscoped.join("\n")
    );
    // The derivation must keep finding the families it scopes: a refactor
    // that silently emptied the set would pass the loop above vacuously.
    assert!(
        dated >= 100,
        "the derived release-dated set collapsed to {dated} matchers"
    );
}
