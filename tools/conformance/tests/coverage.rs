//! The ECC catalogue + case-metadata guard (the CI `cnf coverage guard`).
//!
//! Every registered case must have an allocated `ECC-<AREA>-<NNN>` number in
//! the committed catalogue (`inventory/ecc-catalog.tsv`); numbers are never
//! reused; every `active` line maps back to a live registry entry. Allocate
//! numbers for newly registered cases with
//! `REGEN_CATALOG=1 cargo test -p conformance --test coverage` and review
//! the appended lines before committing. Lines are never deleted — a case
//! removed from the registry flips to `retired`.
//!
//! W-10 additions: the derivation square is machine-checked — every case
//! carries a non-empty spec citation, a schedule trace (or a stated
//! ECC-original reason), and a binding string; the fixture manifest parses.
#![allow(clippy::expect_used)]

use conformance::case::{Binding, ScheduleTrace};
use conformance::catalog::{CATALOG_PATH, Catalog, EccStatus};
use conformance::registry::registry;

#[test]
fn every_registry_case_has_an_ecc_number() {
    let reg = registry();
    let mut catalog = Catalog::load_default().expect("load inventory/ecc-catalog.tsv");

    let missing: Vec<_> = reg
        .entries()
        .iter()
        .filter(|e| catalog.by_primary_ref(e.meta.id).is_none())
        .collect();

    if std::env::var_os("REGEN_CATALOG").is_some() {
        for entry in &missing {
            catalog
                .allocate(entry.meta.area, entry.meta.id, entry.meta.title)
                .expect("allocate ECC number");
        }
        catalog
            .save(std::path::Path::new(CATALOG_PATH))
            .expect("save catalogue");
    } else {
        assert!(
            missing.is_empty(),
            "{} registered case(s) have no ECC number — allocate with REGEN_CATALOG=1 \
             and review the appended lines: {:?}",
            missing.len(),
            missing.iter().map(|e| e.meta.id).collect::<Vec<_>>()
        );
    }

    // Every active line maps to a live registry entry; retired/planned may not.
    for line in catalog.entries() {
        if line.status == EccStatus::Active {
            assert!(
                reg.entries().iter().any(|e| e.meta.id == line.primary_ref),
                "catalogue line {} is active but its primary_ref {:?} has no registry entry — \
                 retire it (never delete)",
                line.ecc_id,
                line.primary_ref
            );
        }
    }

    // Area stability: the catalogue's area matches the case's declared area
    // (numbers are permanent; an area remap means retire + reallocate).
    for entry in reg.entries() {
        let line = catalog
            .by_primary_ref(entry.meta.id)
            .expect("guarded above (or REGEN just allocated)");
        assert_eq!(
            line.area, entry.meta.area,
            "{}: catalogue area diverges from the case's declared area",
            line.ecc_id
        );
    }
}

#[test]
fn every_case_carries_the_derivation_square() {
    for entry in registry().entries() {
        let meta = &entry.meta;
        assert!(
            !meta.citation.trim().is_empty(),
            "{}: empty spec citation",
            meta.id
        );
        match meta.schedule {
            ScheduleTrace::Schedule(s) => assert!(
                s.contains("master"),
                "{}: schedule trace {s:?} names no schedule chapter",
                meta.id
            ),
            ScheduleTrace::EccOriginal(reason) => assert!(
                !reason.trim().is_empty(),
                "{}: ECC-original marker without a stated reason",
                meta.id
            ),
        }
        match meta.binding {
            Binding::Rest(b) => assert!(!b.trim().is_empty(), "{}: empty REST binding", meta.id),
            Binding::NoRestBinding(sm_op) | Binding::NativeApiOnly(sm_op) => assert!(
                !sm_op.trim().is_empty(),
                "{}: no-binding marker without the SM operation named",
                meta.id
            ),
        }
        assert!(
            !meta.formats.is_empty(),
            "{}: no wire formats declared",
            meta.id
        );
    }
}

#[test]
fn the_committed_fixture_manifest_parses() {
    let manifest = conformance::testdata::manifest::Manifest::load_default()
        .expect("testdata/MANIFEST.tsv parses");
    assert!(
        !manifest.entries().is_empty(),
        "the fixture manifest must not be empty"
    );
}
