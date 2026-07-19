//! Emitter invariants, tested as properties over the **real** pipeline on the
//! **real** vendored BMM inputs (design doc §1). Each test drives
//! `openehr_codegen::testsupport`, which runs the same LOAD → ANALYZE → PLAN →
//! RENDER stages `cli.rs` drives. These are the machine-checked replacements for
//! the emitter's formerly review-enforced or panic-only guarantees:
//! completeness, constructibility, byte-determinism, source-package mirroring,
//! downstream-closure correctness, and decision-map integrity.

use openehr_codegen::testsupport;

// ── completeness ────────────────────────────────────────────────────────────

/// Every class in every loaded schema is accounted for in the emission plan:
/// nothing is silently dropped, and the only skips are the two sanctioned
/// reasons (mapped-to-Rust, or abstract-with-no-descendants-and-unused). The
/// counts add up to the full schema.
#[test]
fn completeness_every_class_is_planned_or_sanctioned_skip() {
    for key in testsupport::crate_keys() {
        let c = testsupport::completeness(key).unwrap();
        assert!(
            c.silently_dropped.is_empty(),
            "{key}: {} class(es) silently dropped from the emission plan: {:?}",
            c.silently_dropped.len(),
            c.silently_dropped,
        );
        assert_eq!(
            c.planned + c.skipped_mapped + c.skipped_abstract_unused,
            c.total,
            "{key}: plan ({}) + mapped ({}) + abstract-unused ({}) != total ({})",
            c.planned,
            c.skipped_mapped,
            c.skipped_abstract_unused,
            c.total,
        );
        assert!(c.planned > 0, "{key}: emitted zero types");
    }
}

// ── constructibility ────────────────────────────────────────────────────────

/// No emitted concrete class is a non-constructible infinite value (an unbroken
/// mandatory single-valued construction cycle). Formerly a `panic!` mid-emit;
/// now a test that FAILS the suite listing every offender.
#[test]
fn constructibility_no_non_constructible_type_is_emitted() {
    for key in testsupport::crate_keys() {
        let offenders = testsupport::constructibility_offenders(key).unwrap();
        assert!(
            offenders.is_empty(),
            "{key}: non-constructible type(s) would be emitted (unbroken construction cycle): \
             {offenders:?}. Break each cycle at a spec-cited back_reference edge.",
        );
    }
}

// ── determinism ─────────────────────────────────────────────────────────────

/// Planning the workspace twice yields identical plans (no nondeterministic
/// map/iteration order leaks into the decided shapes).
#[test]
fn determinism_plan_is_stable_across_runs() {
    for key in testsupport::crate_keys() {
        let a = testsupport::plan_shapes(key).unwrap();
        let b = testsupport::plan_shapes(key).unwrap();
        assert_eq!(a, b, "{key}: plan differs between two runs");
        assert!(!a.is_empty(), "{key}: empty plan");
    }
}

/// Rendering the workspace to an in-memory map twice yields byte-identical
/// output (the emitter is a deterministic function of the vendored inputs).
#[test]
fn determinism_render_is_byte_identical_across_runs() {
    let a = testsupport::render_all_to_memory().unwrap();
    let b = testsupport::render_all_to_memory().unwrap();
    assert!(!a.is_empty(), "rendered nothing");
    assert_eq!(
        a.len(),
        b.len(),
        "file count differs between two renders ({} vs {})",
        a.len(),
        b.len(),
    );
    for (path, body_a) in &a {
        assert!(b.contains_key(path), "{path} missing on 2nd render");
        let body_b = b.get(path).unwrap();
        assert_eq!(body_a, body_b, "{path}: body differs between two renders");
    }
}

// ── source-package mirroring + downstream closure ───────────────────────────

/// Every re-emitted cross-schema class (the AM 2.4 downstream closure) lands at
/// the SAME package path it occupies in its upstream source schema (BASE/LANG).
#[test]
fn mirroring_reemitted_classes_land_at_source_package_path() {
    let mirrors = testsupport::am24_reemit_mirrors().unwrap();
    assert!(!mirrors.is_empty(), "empty re-emission closure");
    for m in &mirrors {
        assert!(
            m.source_path.is_some(),
            "{}: re-emitted class has no upstream source package path",
            m.class,
        );
        assert!(
            m.augmented_path.is_some(),
            "{}: re-emitted class not grafted into the downstream schema",
            m.class,
        );
        assert_eq!(
            m.source_path, m.augmented_path,
            "{}: re-emitted at a different package path than its source",
            m.class,
        );
    }
}

/// The AM ⊃ LANG downstream closure contains the expected cross-`includes`
/// re-emission set — the beom expression/statement subtree, the BMM-3 + EL
/// object model, and the BASE resource metatype — and every member is grafted
/// into the AM 2.4 output.
#[test]
fn downstream_closure_contains_the_beom_bmm3_resource_set() {
    let closure = testsupport::am24_reemit_closure().unwrap();
    // beom expression/statement subtree (LANG-origin, extended by AM's rules):
    for expected in [
        "EXPRESSION",
        "EXPR_LEAF",
        "EXPR_VALUE_REF",
        "STATEMENT",
        "STATEMENT_SET",
        "ASSERTION",
    ] {
        assert!(closure.contains(expected), "closure missing {expected}");
    }
    // the BMM-3 + EL object model (LANG-origin):
    assert!(
        closure.iter().any(|c| c.starts_with("BMM_")),
        "closure missing the BMM-3 object model",
    );
    assert!(
        closure.iter().any(|c| c.starts_with("EL_")),
        "closure missing the EL expression language",
    );
    // the BASE resource metatype (extended by AM's AUTHORED_ARCHETYPE etc.):
    for expected in ["AUTHORED_RESOURCE", "RESOURCE_DESCRIPTION"] {
        assert!(closure.contains(expected), "closure missing {expected}");
    }
}

/// Upstream (LANG) output is untouched by AM analysis: LANG emits its own
/// expression classes independently, the AM-only rules leaves never leak into
/// LANG, and those leaves ARE emitted into the AM crate.
#[test]
fn downstream_closure_leaves_upstream_output_untouched() {
    let lang_files = testsupport::rendered_files("lang").unwrap();
    // LANG emits its own EXPR_LEAF (a shared closure member) independently:
    assert!(
        lang_files.iter().any(|p| p.ends_with("expr_leaf.rs")),
        "LANG did not emit its own EXPR_LEAF",
    );
    // The AM-only rules leaves must NOT appear in LANG's output:
    for leaf in ["expr_archetype_ref.rs", "expr_constraint.rs"] {
        assert!(
            !lang_files.iter().any(|p| p.ends_with(leaf)),
            "AM-only leaf {leaf} leaked into LANG output",
        );
    }
    // …and they ARE emitted into the AM crate, while the shared member is
    // re-emitted downstream too.
    let rendered = testsupport::render_all_to_memory().unwrap();
    let am_paths: Vec<&String> = rendered
        .keys()
        .filter(|k| k.starts_with("openehr-am/"))
        .collect();
    for leaf in ["expr_archetype_ref.rs", "expr_constraint.rs"] {
        assert!(
            am_paths.iter().any(|p| p.ends_with(leaf)),
            "AM did not emit its own leaf {leaf}",
        );
    }
    assert!(
        am_paths
            .iter()
            .any(|p| p.contains("am24/") && p.ends_with("expr_leaf.rs")),
        "AM did not re-emit the shared EXPR_LEAF downstream",
    );
}

// ── decision-map integrity ──────────────────────────────────────────────────

/// Every declarative decision entry carries a non-empty citation/flag, and —
/// for the binding maps — refers to a class/field that EXISTS in the loaded
/// schemas (no stale entries). The mapped-class denylist and primitive type map
/// are exempt from existence (extra entries there are inert), but still must be
/// cited.
#[test]
fn decision_map_integrity_entries_are_cited_and_reference_real_classes() {
    for map in testsupport::decision_maps() {
        assert!(!map.entries.is_empty(), "{}: empty decision map", map.map);
        for e in &map.entries {
            assert!(
                !e.citation.trim().is_empty(),
                "{}: entry {:?} has an empty citation/flag",
                map.map,
                e.key,
            );
            assert!(
                !e.reason.trim().is_empty(),
                "{}: entry {:?} has an empty reason",
                map.map,
                e.key,
            );
            assert!(
                !e.decision.trim().is_empty(),
                "{}: entry {:?} has an empty decision",
                map.map,
                e.key,
            );
            if !map.check_existence {
                continue;
            }
            if let Some((class, field)) = e.key.split_once('.') {
                assert!(
                    testsupport::field_exists(class, field).unwrap(),
                    "{}: stale entry — {class}.{field} exists in no loaded schema",
                    map.map,
                );
            } else {
                assert!(
                    testsupport::class_exists(&e.key).unwrap(),
                    "{}: stale entry — class {:?} exists in no loaded schema",
                    map.map,
                    e.key,
                );
            }
        }
    }
}

/// The crate → schema-merge table is itself declarative decision data: every
/// composition entry carries a non-empty citation, lists at least one own BMM
/// file, resolves (its member/dependency BMM files load), and references only
/// known dependency keys.
#[test]
fn composition_table_integrity() {
    let keys: Vec<String> = testsupport::composition_infos()
        .iter()
        .map(|c| c.key.clone())
        .collect();
    for info in testsupport::composition_infos() {
        assert!(
            !info.citation.trim().is_empty(),
            "composition {:?} has an empty citation",
            info.key,
        );
        assert!(
            !info.reason.trim().is_empty(),
            "composition {:?} has an empty reason",
            info.key,
        );
        assert!(
            !info.own.is_empty(),
            "composition {:?} lists no own BMM file",
            info.key,
        );
        assert!(
            !info.crate_name.is_empty(),
            "composition {:?} has no crate name",
            info.key,
        );
        // `variant` is Some only for the multi-version crate (am14/am24).
        if info.variant.is_some() {
            assert_eq!(
                info.crate_name, "openehr-am",
                "only openehr-am is a multi-version crate; {:?} has a variant",
                info.key,
            );
        }
        for dep in info.model_deps.iter().chain(info.prelude_deps.iter()) {
            assert!(
                keys.iter().any(|k| k == dep),
                "composition {:?} references unknown dependency key {dep:?}",
                info.key,
            );
        }
        // Resolving the composition loads every member/dependency BMM file.
        assert!(
            testsupport::completeness(&info.key).is_ok(),
            "composition {:?} failed to resolve its member/dependency BMM files",
            info.key,
        );
    }
}
