// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The runner-verification pack, part 1 (verdict conformance): replaying
//! the committed transcript reproduces every adjudicated verdict — and a
//! deliberately-broken runner (tampered adjudications) is REJECTED, which
//! is the pack's own falsifiability requirement.

#![expect(
    clippy::expect_used,
    reason = "test-support helpers (not `#[test]` fns, so the clippy.toml in-tests scoping does not reach them) are panic-idiomatic: a broken fixture must abort the test loudly, Book ch11"
)]

use cnf_runner::artifacts::load_root;
use cnf_runner::exec::player::{ExpectedVerdict, Transcript, replay_entry, verdict_matches};

fn load() -> (cnf_runner::artifacts::ArtifactSet, Transcript) {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let loaded = load_root(&crate_dir.join("artifacts")).expect("schema compilation");
    assert!(loaded.errors.is_empty(), "artifact tree must load cleanly");
    let text = std::fs::read_to_string(crate_dir.join("verification-pack/transcript.json"))
        .expect("committed transcript");
    let transcript: Transcript = serde_json::from_str(&text).expect("transcript parses");
    (loaded.set, transcript)
}

#[test]
fn every_adjudicated_verdict_is_reproduced() {
    let (set, transcript) = load();
    assert!(!transcript.entries.is_empty());
    for entry in &transcript.entries {
        let (expected, produced) =
            replay_entry(&set, entry).unwrap_or_else(|e| panic!("{}: {e}", entry.case));
        assert!(
            verdict_matches(expected, &produced),
            "{} row {}: adjudicated {:?}, runner produced {:?} ({})",
            entry.case,
            entry.row,
            expected,
            produced,
            entry.adjudication_ref
        );
    }
}

#[test]
fn the_pack_rejects_a_broken_runner() {
    // Tampering with the adjudication (the moral equivalent of a runner
    // that mis-classifies) MUST be detected: flip the errored entry to
    // `passed` and require the reproduction check to fail.
    let (set, mut transcript) = load();
    let entry = transcript
        .entries
        .iter_mut()
        .find(|e| matches!(e.expected_verdict, ExpectedVerdict::Errored))
        .expect("an errored adjudication exists");
    entry.expected_verdict = ExpectedVerdict::Passed;
    let (expected, produced) = replay_entry(&set, entry).expect("replay");
    assert!(
        !verdict_matches(expected, &produced),
        "a mis-adjudicated verdict must NOT be reproduced silently"
    );
}

#[test]
fn transcript_verdict_vocabulary_is_exercised() {
    let (_, transcript) = load();
    let has = |v: ExpectedVerdict| {
        transcript
            .entries
            .iter()
            .any(|e| std::mem::discriminant(&e.expected_verdict) == std::mem::discriminant(&v))
    };
    assert!(
        has(ExpectedVerdict::Passed),
        "pack must exercise a passing row"
    );
    assert!(
        has(ExpectedVerdict::Failed),
        "pack must exercise a failing row"
    );
    assert!(
        has(ExpectedVerdict::Errored),
        "pack must exercise an inconclusive row"
    );
}
