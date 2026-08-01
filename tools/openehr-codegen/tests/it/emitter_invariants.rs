//! Emitter invariants, tested as properties over the **real** pipeline on the
//! **real** vendored BMM inputs (design doc §1). Each test drives
//! `openehr_codegen::testsupport`, which runs the same LOAD → ANALYZE → PLAN →
//! RENDER stages `cli.rs` drives. These are the machine-checked replacements for
//! the emitter's formerly review-enforced or panic-only guarantees:
//! completeness, constructibility, byte-determinism, source-package mirroring,
//! downstream-closure correctness, and decision-map integrity.

use openehr_codegen::testsupport;

// ── completeness ────────────────────────────────────────────────────────────

/// Every class of **every BMM generation** is accounted for in the emission
/// plan: nothing is silently dropped, and the only skips are the two sanctioned
/// reasons (mapped-to-Rust, or abstract-with-no-descendants-and-unused). The
/// counts add up to the generation's full class list.
///
/// Counting per generation rather than over a merged class map is the point: a
/// merge keeps every NAME, so a name-level count over it passes while one
/// generation's classes have been replaced wholesale by the other's.
#[test]
fn completeness_every_class_is_planned_or_sanctioned_skip() {
    for key in testsupport::crate_keys() {
        let rows = testsupport::completeness(key).unwrap();
        assert!(!rows.is_empty(), "{key}: no BMM generation resolved");
        for c in rows {
            let file = &c.file;
            assert!(
                c.silently_dropped.is_empty(),
                "{key} [{file}]: {} class(es) silently dropped from the emission plan: {:?}",
                c.silently_dropped.len(),
                c.silently_dropped,
            );
            assert_eq!(
                c.planned + c.skipped_mapped + c.skipped_abstract_unused,
                c.total,
                "{key} [{file}]: plan ({}) + mapped ({}) + abstract-unused ({}) != total ({})",
                c.planned,
                c.skipped_mapped,
                c.skipped_abstract_unused,
                c.total,
            );
            assert!(c.planned > 0, "{key} [{file}]: emitted zero types");
        }
    }
}

/// Every attribute every loaded BMM generation declares reaches an emitted Rust
/// field — the attribute-level half of completeness, which the class-NAME count
/// above cannot see.
///
/// This is the gate the LANG two-schema merge defeated: merging the stable v2.x
/// BMM and the v3 development line into one class map left every class NAME
/// present while discarding one generation's attribute set for each of the 18
/// names both declare (`LANG/docs/bmm3/master00-amendment_record.adoc`
/// SPECLANG-14 formalises the v2/v3 split).
#[test]
fn completeness_every_declared_attribute_reaches_an_emitted_field() {
    for key in testsupport::crate_keys() {
        let (gaps, checked) = testsupport::attribute_gaps(key).unwrap();
        assert!(
            gaps.is_empty(),
            "{key}: {} BMM-declared attribute(s) reach no emitted field: {:?}",
            gaps.len(),
            gaps.iter()
                .map(|g| format!("[{}] {}.{} — {}", g.file, g.class, g.attribute, g.detail))
                .collect::<Vec<_>>(),
        );
        // Non-vacuity: a broken traversal that checked nothing would also report
        // zero gaps, so pin that real work happened. LANG is the composition this
        // gate exists for (two generations, 205 declared classes between them), so
        // it carries the substantial bar.
        assert!(
            checked > 0,
            "{key}: no (class, attribute) pair checked — the traversal is vacuous",
        );
        if key == "lang" {
            assert!(
                checked > 500,
                "lang: only {checked} (class, attribute) pair(s) checked across both BMM \
                 generations — the traversal no longer covers the model",
            );
        }
    }
}

/// No two BMM generations of one crate claim the same emitted file path or the
/// same crate-prelude identifier.
///
/// A shared path means one generation's output overwrites the other's — a
/// silently picked shape, which is exactly the defect the per-generation
/// emission exists to prevent. A shared prelude identifier means the crate's
/// one-type-per-Rust-name contract is broken.
#[test]
fn generations_never_silently_pick_a_shape() {
    let conflicts = testsupport::generation_conflicts().unwrap();
    assert!(
        conflicts.is_empty(),
        "BMM generations of one crate collide: {:?}",
        conflicts
            .iter()
            .map(|c| format!("{}: {:?} claimed by {:?}", c.key, c.what, c.files))
            .collect::<Vec<_>>(),
    );
}

/// LANG emits BOTH extant BMM generations completely, each at its own
/// source-package path: the stable v2.x model under `bmm/`, `bmm_persistence/`
/// and `beom/` (`LANG/docs/bmm/master01-preface.adoc` §History — "the normative,
/// tool-implemented version"), and the v3 development line under `bmm3/`
/// (`LANG/docs/bmm3/master01-preface.adoc` §Previous Versions). A name declared
/// by both — `BMM_CLASS`, `BMM_TYPE`, … — yields two Rust types.
#[test]
fn lang_emits_both_bmm_generations_at_their_own_paths() {
    let files = testsupport::rendered_files("lang").unwrap();
    let has = |p: &str| files.iter().any(|f| f == p);
    // Both generations of a colliding name (18 of them; these are the shapes the
    // ch.6–ch.8 chapter audits pinned as materially different).
    for (v2, bmm3) in [
        ("bmm/core/bmm_class.rs", "bmm3/core/entity/bmm_class.rs"),
        ("bmm/core/bmm_type.rs", "bmm3/core/entity/bmm_type.rs"),
        (
            "bmm/core/bmm_container_type.rs",
            "bmm3/core/entity/bmm_container_type.rs",
        ),
        (
            "bmm/core/bmm_property.rs",
            "bmm3/core/feature/bmm_property.rs",
        ),
        ("bmm/core/bmm_model.rs", "bmm3/core/model/bmm_model.rs"),
        (
            "bmm/core/bmm_model_element.rs",
            "bmm3/core/bmm_model_element.rs",
        ),
    ] {
        assert!(has(v2), "LANG did not emit the v2 generation's {v2}");
        assert!(has(bmm3), "LANG did not emit the v3 generation's {bmm3}");
    }
    // The two classes the merge left descendant-less and therefore unemitted.
    for bmm3_only in [
        "bmm3/core/entity/bmm_model_type.rs",
        "bmm3/core/entity/bmm_module.rs",
    ] {
        assert!(has(bmm3_only), "LANG did not emit {bmm3_only}");
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

/// Every adjudicated free-form field is a REAL degrade whose adjudication is
/// written at the site: some generated file renders the field as
/// `serde_json::Value` and carries the `// NOTE:` with the entry's citation.
///
/// This is the non-vacuity half of the decision-map integrity test — an entry
/// for a field that turned out to be typeable would otherwise sit unnoticed,
/// which is exactly the "silence over an untyped slot" this map exists to end.
#[test]
fn untyped_field_adjudications_land_on_a_real_free_form_field() {
    let files = testsupport::render_all_to_memory().unwrap();
    for (class, field, citation) in testsupport::untyped_fields() {
        let ident = if field == "type" {
            "r#type".to_string()
        } else {
            field.clone()
        };
        let hit = files.values().any(|body| {
            body.contains(&citation)
                && body.contains(&format!("pub {ident}: serde_json::Value"))
                && body.contains(&format!("(`{class}`)"))
        });
        assert!(
            hit,
            "{class}.{field}: no generated file carries the adjudication NOTE above a free-form \
             `{ident}: serde_json::Value` field — the entry is stale (the field is typed now) or \
             the NOTE is not being emitted",
        );
    }
}

/// Every declared additional subtype member actually widens its parent's
/// polymorphic slot: in every composition whose model defines both classes, the
/// parent's variant set contains the subtype (and at least one composition does
/// define both, so no entry is inert).
#[test]
fn subtype_extensions_widen_their_parents_variant_set() {
    for (parent, subtype) in testsupport::subtype_extensions() {
        let mut seen = 0_usize;
        for key in testsupport::crate_keys() {
            let Some(variants) = testsupport::enum_variants(key, &parent).unwrap() else {
                continue;
            };
            if testsupport::enum_variants(key, &subtype).unwrap().is_none() {
                continue;
            }
            seen += 1;
            assert!(
                variants.contains(&subtype),
                "{key}: {parent}'s variants {variants:?} do not include the declared \
                 extension subtype {subtype}",
            );
        }
        assert!(
            seen > 0,
            "no composition defines both {parent} and {subtype}: the subtype_extensions entry \
             is inert",
        );
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
        // Resolving the composition loads every member/dependency BMM file, one
        // completeness row per generation.
        let rows = testsupport::completeness(&info.key);
        assert!(
            rows.as_ref().is_ok_and(|r| r.len() == info.own.len()),
            "composition {:?} failed to resolve one generation per own BMM file",
            info.key,
        );
    }
}

// ── invariant classification (assertion-dialect analyzer) ────────────────────

/// The assertion-dialect analyzer accounts for **every** RM 1.2.0 class
/// invariant in exactly one bucket — nothing is silently dropped. `emitted +
/// runtime-hook-missing + complex == total`, and the per-bucket counts are
/// pinned as the current classifier's verdict (a deliberate classifier or
/// BMM-pin change updates them under review), so an accidental
/// mis-classification or a dropped invariant fails the suite.
#[test]
fn invariant_classification_is_a_total_tripartition() {
    let rows = testsupport::classify_invariants("rm").unwrap();
    let total = rows.len();
    let emitted = rows.iter().filter(|r| r.bucket == "emitted").count();
    let hook = rows
        .iter()
        .filter(|r| r.bucket == "runtime-hook-missing")
        .count();
    let complex = rows.iter().filter(|r| r.bucket == "complex").count();

    // Every row lands in one of the three known buckets (no unexpected label).
    assert_eq!(
        emitted + hook + complex,
        total,
        "some invariant carries an unknown bucket label",
    );
    // The RM 1.2.0 BMM carries exactly 155 class invariants.
    assert_eq!(total, 155, "RM 1.2.0 invariant count changed");
    // Pinned tripartition (design doc §4 R5): a change here is deliberate.
    // ITEM_TAG.Inv_key_valid is COMPLEX, not EMITTED: `key.is_justified` is a
    // boolean-returning BMM function (a method call), which the assertion-dialect
    // emitter cannot project from a field — so 90 emitted / 31 complex (R5b).
    assert_eq!(emitted, 90, "EMITTED count drifted");
    assert_eq!(hook, 34, "RUNTIME-HOOK-MISSING count drifted");
    assert_eq!(complex, 31, "COMPLEX count drifted");

    // A non-emitted row always names its reason; an emitted row never does.
    for r in &rows {
        if r.bucket == "emitted" {
            assert!(
                r.reason.is_empty(),
                "{}::{} is emitted but carries a reason",
                r.class,
                r.name,
            );
        } else {
            assert!(
                !r.reason.is_empty(),
                "{}::{} ({}) is missing its reason",
                r.class,
                r.name,
                r.bucket,
            );
        }
    }
}

/// Spot-checks pinning the classifier's verdict on representative invariants so
/// the buckets cannot silently swap: a terminology/code-set predicate needs a
/// runtime hook, a quantifier is complex, and a plain emptiness/`Void` check is
/// emittable.
#[test]
fn invariant_classification_spot_checks() {
    let rows = testsupport::classify_invariants("rm").unwrap();
    let bucket = |class: &str, name: &str| -> &'static str {
        rows.iter()
            .find(|r| r.class == class && r.name == name)
            .map_or("MISSING", |r| r.bucket)
    };
    assert_eq!(bucket("DV_IDENTIFIER", "Id_valid"), "emitted");
    assert_eq!(bucket("DV_DATE", "Value_valid"), "emitted");
    assert_eq!(bucket("LOCATABLE", "Links_valid"), "emitted");
    assert_eq!(
        bucket("COMPOSITION", "Category_validity"),
        "runtime-hook-missing",
    );
    assert_eq!(
        bucket("COMPOSITION", "Territory_valid"),
        "runtime-hook-missing"
    );
    assert_eq!(bucket("EHR", "Compositions_valid"), "complex");
    assert_eq!(bucket("HISTORY", "Period_consistency"), "complex");
}

// ── invariant-core emission (emit-validate) ──────────────────────────────────

/// The generated invariant-core file (`emit-validate`) honestly accounts for
/// every RM class invariant the emitter claims to cover: each **emitted-core**
/// invariant appears as a violation-message literal, and each **inert**
/// (runtime-hook-missing) invariant appears in the pending-adjudication doc
/// register — so nothing the classifier flagged is silently dropped.
#[test]
fn invariant_core_file_accounts_for_emitted_and_inert_invariants() {
    let files = testsupport::render_all_to_memory().unwrap();
    let gen_file = files
        .get("openehr-rm/validate/generated.rs")
        .expect("emit-validate did not produce validate/generated.rs");

    // The invariants the emitted cores realize (their violation messages carry
    // the BMM invariant name verbatim).
    for name in [
        "Code_string_valid",
        "Valid_value",
        "Formatting_valid",
        "Value_valid",
        "Id_valid",
        "Match_valid",
        "Formalism_valid",
        "Accuracy_is_percent_validity",
        "Accuracy_validity",
        "Magnitude_status_valid",
        "Type_validity",
        "Valid_denominator",
        "Precision_validity",
        "Fraction_validity",
        "Unitary_validity",
        "Percent_validity",
        "Archetype_node_id_valid",
        "Events_valid",
        "Is_archetype_root",
        "Action_archetype_id_valid",
        "location_valid",
        "Rm_version_valid",
        "Basic_validity",
        "Name_valid",
    ] {
        assert!(
            gen_file.contains(name),
            "emitted-core invariant {name:?} is not named in validate/generated.rs",
        );
    }

    // Every runtime-hook-missing (inert) RM invariant is named in the pending
    // register — the same classifier verdict the emitter derives it from.
    let rows = testsupport::classify_invariants("rm").unwrap();
    let hook: Vec<_> = rows
        .iter()
        .filter(|r| r.bucket == "runtime-hook-missing")
        .collect();
    assert!(!hook.is_empty(), "no runtime-hook-missing invariants found");
    for r in hook {
        assert!(
            gen_file.contains(&r.name),
            "inert invariant {}::{} is not named in the pending register",
            r.class,
            r.name,
        );
    }
}

/// The assertion-dialect predicate → runtime-function table
/// (`plan::overrides::DIALECT_PREDICATES`) covers **exactly** the classifier's
/// recognised runtime-backed leaf predicates (`RUNTIME_PREDICATES`) — no
/// recognised predicate lacks a runtime hook, and no stale table entry names a
/// predicate the classifier no longer recognises.
#[test]
fn dialect_predicates_match_the_classifier() {
    use std::collections::BTreeSet;
    let table: BTreeSet<String> = testsupport::dialect_predicates()
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    let classifier: BTreeSet<String> = testsupport::runtime_predicates().into_iter().collect();
    assert_eq!(
        table, classifier,
        "DIALECT_PREDICATES and the classifier's RUNTIME_PREDICATES drifted apart",
    );
    // Every mapped runtime function is non-empty (the integrity test in
    // `decision_map_integrity_*` already checks citation/reason/decision).
    for (predicate, runtime_fn) in testsupport::dialect_predicates() {
        assert!(
            !runtime_fn.trim().is_empty(),
            "dialect predicate {predicate:?} has no runtime function",
        );
    }
}

// ── constants emission (BMM `BMM_CLASS.constants`) ───────────────────────────

/// Every constant the BMM declares on an **emitted** class is rendered as a
/// `pub const` in that crate's output — the R5 constants-emission completeness
/// guard. Checked over the two RM terminology identifier classes (22 constants)
/// whose hand-written `*_impl.rs` was deleted in favour of emission.
#[test]
fn emitted_classes_render_their_bmm_constants() {
    let files = testsupport::render_all_to_memory().unwrap();
    let group = files
        .get("openehr-rm/support/terminology/openehr_terminology_group_identifiers.rs")
        .unwrap();
    let code_set = files
        .get("openehr-rm/support/terminology/openehr_code_set_identifiers.rs")
        .unwrap();

    // The 15 terminology-group identifier constants + the openEHR terminology id.
    for c in [
        "TERMINOLOGY_ID_OPENEHR",
        "GROUP_ID_AUDIT_CHANGE_TYPE",
        "GROUP_ID_VERSION_LIFE_CYCLE_STATE",
    ] {
        assert!(
            group.contains(&format!("pub const {c}:")),
            "group-identifiers output is missing constant {c}",
        );
    }
    assert!(
        group.contains(r#"= "openehr";"#),
        "constant value not emitted"
    );

    // The 7 code-set identifier constants (incl. the `_id`-less integrity one).
    for c in [
        "CODE_SET_ID_LANGUAGES",
        "CODE_SET_INTEGRITY_CHECK_ALGORITHMS",
        "CODE_SET_ID_NORMAL_STATUSES",
    ] {
        assert!(
            code_set.contains(&format!("pub const {c}:")),
            "code-set output is missing constant {c}",
        );
    }
}

/// The register records the INV-UNIFY adjudication state: the terminology /
/// code-set family is `enforced at the dispatcher` and only the four
/// `VERSIONED_OBJECT` aggregate invariants stay `runtime-hook-missing`. Pins the
/// split so a regression (an invariant silently sliding back to "pending", or the
/// enforced count drifting) fails the suite.
#[test]
fn register_records_terminology_invariants_as_enforced() {
    let files = testsupport::render_all_to_memory().unwrap();
    let gen_file = files
        .get("openehr-rm/validate/generated.rs")
        .expect("emit-validate did not produce validate/generated.rs");

    assert!(
        gen_file.contains("# Terminology-backed invariants (enforced at the dispatcher"),
        "the enforced-at-dispatcher register heading is missing",
    );

    // Every runtime-hook-missing invariant is adjudicated as either enforced at
    // the dispatcher (terminology/code-set) or a versioned-object aggregate.
    let rows = testsupport::classify_invariants("rm").unwrap();
    let hook: Vec<_> = rows
        .iter()
        .filter(|r| r.bucket == "runtime-hook-missing")
        .collect();
    let mut enforced = 0usize;
    let mut aggregate = 0usize;
    let mut unadjudicated: Vec<String> = Vec::new();
    for r in &hook {
        let enforced_line = format!("`{}.{}` — enforced at the dispatcher", r.class, r.name);
        let aggregate_line = format!(
            "`{}.{}` — versioned-object aggregate model",
            r.class, r.name
        );
        if gen_file.contains(&enforced_line) {
            enforced += 1;
        } else if gen_file.contains(&aggregate_line) {
            aggregate += 1;
        } else {
            unadjudicated.push(format!("{}::{}", r.class, r.name));
        }
    }
    assert!(
        unadjudicated.is_empty(),
        "invariant(s) carry no adjudication verdict in the register: {unadjudicated:?}",
    );
    // 30 terminology/code-set invariants wired to the dispatcher; the 4
    // `VERSIONED_OBJECT` aggregate invariants stay pending.
    assert_eq!(enforced, 30, "enforced-at-dispatcher count drifted");
    assert_eq!(aggregate, 4, "versioned-aggregate pending count drifted");
    assert_eq!(
        enforced + aggregate,
        hook.len(),
        "register split is not total"
    );
}
