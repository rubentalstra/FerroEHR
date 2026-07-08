//! The ECC catalogue guard (design v4): the framework's own coverage
//! discipline.
//!
//! Every registered case must have an allocated `ECC-<AREA>-<NNN>` number in
//! the committed catalogue (`inventory/ecc-catalog.tsv`); numbers are never
//! reused; every `active` line maps back to a live registry entry. Allocate
//! numbers for newly registered cases with
//! `REGEN_CATALOG=1 cargo test -p ehrbase-conformance --test coverage` and
//! review the appended lines before committing. Lines are never deleted — a
//! case removed from the registry flips to `retired`.
#![allow(clippy::expect_used)]

use ehrbase_conformance::catalog::{CATALOG_PATH, Catalog, EccStatus, area_of};
use ehrbase_conformance::registry::registry;

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
            let area = area_of(&entry.meta);
            catalog
                .allocate(area, entry.meta.id, entry.meta.id)
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

    // Area stability: the catalogue's area matches the registry's derivation
    // (numbers are permanent; an area remap means retire + reallocate).
    for entry in reg.entries() {
        let line = catalog
            .by_primary_ref(entry.meta.id)
            .expect("guarded above (or REGEN just allocated)");
        assert_eq!(
            line.area,
            area_of(&entry.meta),
            "{}: catalogue area diverges from the registry's derived area",
            line.ecc_id
        );
    }
}
