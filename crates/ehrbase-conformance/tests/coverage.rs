//! The total-coverage guard (design §4.2, the house pattern's third use).
//!
//! Parses the vendored schedule, classifies every identified case through the
//! registry, and asserts:
//!
//! 1. every case is classified (no unclassifiable id);
//! 2. the full inventory of classification keys matches the committed snapshot
//!    `inventory/schedule-cases.txt` — so an upstream/re-vendor change fails the
//!    build until triaged;
//! 3. every registry id is in the inventory (no phantom implemented cases).
//!
//! Regenerate the snapshot after an intentional inventory change with
//! `REGEN_INVENTORY=1 cargo test -p ehrbase-conformance --test coverage` and
//! review the diff before committing.
#![allow(clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use ehrbase_conformance::case::Provenance;
use ehrbase_conformance::registry::{ExclusionReason, Registration, registry};
use ehrbase_conformance::schedule::parse_default;

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("inventory/schedule-cases.txt")
}

#[test]
fn every_schedule_case_is_classified_and_matches_the_snapshot() {
    let schedule = parse_default().expect("parse vendored schedule");
    let inventory = schedule.inventory().expect("classify inventory");
    let reg = registry();

    // (1) Every case classifies; tally the structural reasons for a sanity check.
    let mut implemented = 0usize;
    let mut placeholder = 0usize;
    let mut duplicate = 0usize;
    let mut adl2 = 0usize;
    let mut not_yet = 0usize;
    let mut other = 0usize;
    for item in &inventory {
        match reg.classify(item) {
            Registration::Implemented(entry) => {
                assert_eq!(entry.meta.id, item.key, "implemented entry keyed by id");
                implemented += 1;
            }
            Registration::Excluded(ExclusionReason::UpstreamPlaceholder) => placeholder += 1,
            Registration::Excluded(ExclusionReason::UpstreamDuplicate) => duplicate += 1,
            Registration::Excluded(ExclusionReason::Adl2Returns501) => adl2 += 1,
            Registration::Excluded(ExclusionReason::NotYetTranscribed) => not_yet += 1,
            Registration::Excluded(_) => other += 1,
        }
    }
    assert_eq!(inventory.len(), 324, "the 324-case identified inventory");
    assert_eq!(placeholder, 57, "aaaa (28) + bbbb (29) placeholders");
    assert_eq!(duplicate, 1, "the one CONT-DV_TEXT-validate_open duplicate");
    assert_eq!(
        adl2, 0,
        "no I_DEFINITION_ADL2 cases in the current vendored schedule"
    );
    // Only Schedule-provenance entries appear in the 322 inventory; the
    // supplementary FixtureDerived / RunnerDefined cases (design §3.4, §4.6) sit
    // outside it by design.
    let schedule_impl = reg
        .entries()
        .iter()
        .filter(|e| e.meta.provenance == Provenance::Schedule)
        .count();
    assert_eq!(
        implemented, schedule_impl,
        "implemented (in-inventory) count equals the schedule-provenance registry size"
    );
    assert_eq!(
        implemented + placeholder + duplicate + adl2 + not_yet + other,
        324,
        "classification is total"
    );

    // (2) The inventory of classification keys matches the committed snapshot.
    let keys: BTreeSet<&str> = inventory.iter().map(|i| i.key.as_str()).collect();
    assert_eq!(keys.len(), inventory.len(), "keys are unique");
    let rendered = {
        let mut s: String = keys.iter().fold(String::new(), |mut acc, k| {
            acc.push_str(k);
            acc.push('\n');
            acc
        });
        s.shrink_to_fit();
        s
    };
    let path = snapshot_path();
    if std::env::var_os("REGEN_INVENTORY").is_some() {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir inventory");
        std::fs::write(&path, &rendered).expect("write snapshot");
    } else {
        let committed = std::fs::read_to_string(&path).expect(
            "read inventory/schedule-cases.txt — regenerate with REGEN_INVENTORY=1 if intended",
        );
        assert_eq!(
            rendered, committed,
            "the schedule inventory changed; review and regenerate the snapshot with REGEN_INVENTORY=1"
        );
    }

    // (3) No phantom schedule cases: every Schedule-provenance registry id is in
    // the inventory. FixtureDerived / RunnerDefined ids are intentionally not.
    for entry in reg.entries() {
        if entry.meta.provenance == Provenance::Schedule {
            assert!(
                keys.contains(entry.meta.id),
                "schedule-provenance registry id {:?} is not in the inventory (phantom case)",
                entry.meta.id
            );
        }
    }
}

