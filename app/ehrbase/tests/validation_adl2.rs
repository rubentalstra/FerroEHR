//! Registration-side ADL2 validity tests: one spec-valid source, then one
//! targeted mutation per enforced AOM2 rule code.

#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
use ehrbase::validation::adl2::{Adl2Violation, validate_adl2_source};

/// A minimal spec-valid ADL2 source archetype: header, HRID, language,
/// definition, terminology (ADL2 master02 §Structure; the terminology carries
/// every code the definition uses).
const VALID: &str = r#"archetype (adl_version=2.0.6; rm_release=1.1.0)
    openEHR-EHR-OBSERVATION.bp.v1.0.0

language
    original_language = <[ISO_639-1::en]>

description
    lifecycle_state = <"unmanaged">

definition
    OBSERVATION[id1] matches {    -- Blood pressure
        data matches {
            HISTORY[id2] matches {
                events cardinality matches {0..*; unordered} matches {
                    POINT_EVENT[id3] matches {
                        data matches {
                            ITEM_TREE[id4] matches {
                                items matches {
                                    ELEMENT[id5] occurrences matches {0..1} matches {
                                        value matches {
                                            DV_CODED_TEXT[id6] matches {
                                                defining_code matches {[ac1; at5]}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

terminology
    term_definitions = <
        ["en"] = <
            ["id1"] = <
                text = <"Blood pressure">
                description = <"The measurement of blood pressure.">
            >
            ["id2"] = <text = <"History"> description = <"History.">>
            ["id3"] = <text = <"Any event"> description = <"Any event.">>
            ["id4"] = <text = <"Tree"> description = <"Tree.">>
            ["id5"] = <text = <"Cuff size"> description = <"Cuff size.">>
            ["id6"] = <text = <"Coded cuff"> description = <"Coded cuff.">>
            ["at5"] = <text = <"Adult"> description = <"Adult cuff.">>
            ["at6"] = <text = <"Child"> description = <"Child cuff.">>
            ["ac1"] = <text = <"Cuff sizes"> description = <"Any cuff size.">>
        >
    >
    value_sets = <
        ["ac1"] = <
            id = <"ac1">
            members = <"at5", "at6">
        >
    >
"#;

fn expect_code(src: &str, code: &str) {
    let Adl2Violation { code: got, detail } = validate_adl2_source(src)
        .err()
        .expect("expected a violation");
    assert_eq!(got, code, "expected {code}, got {got}: {detail}");
}

#[test]
fn valid_source_passes() {
    let meta = validate_adl2_source(VALID).expect("valid ADL2 source");
    assert_eq!(meta.kind, "archetype");
    assert_eq!(meta.hrid, "openEHR-EHR-OBSERVATION.bp.v1.0.0");
    assert_eq!(meta.depth, 0);
    assert!(meta.parent_hrid.is_none());
}

#[test]
fn stcnt_missing_terminology() {
    let src = VALID.split("terminology").next().unwrap();
    expect_code(src, "STCNT");
}

#[test]
fn stcnt_missing_definition() {
    let src = VALID.replace("\ndefinition\n", "\n-- definition removed\n");
    // The definition body lines remain but belong to no section keyword.
    expect_code(&src, "STCNT");
}

#[test]
fn varav_bad_adl_version() {
    let src = VALID.replace("adl_version=2.0.6", "adl_version=two");
    expect_code(&src, "VARAV");
}

#[test]
fn varrv_bad_rm_release() {
    let src = VALID.replace("rm_release=1.1.0", "rm_release=1.1.x");
    expect_code(&src, "VARRV");
}

#[test]
fn vardt_root_type_mismatch() {
    let src = VALID.replace("OBSERVATION[id1]", "EVALUATION[id1]");
    expect_code(&src, "VARDT");
}

#[test]
fn varcn_specialised_root_without_specialize_section() {
    let src = VALID.replace("OBSERVATION[id1]", "OBSERVATION[id1.1]");
    expect_code(&src, "VARCN");
}

#[test]
fn varcn_malformed_root_node_id() {
    let src = VALID.replace("OBSERVATION[id1]", "OBSERVATION[id7]");
    expect_code(&src, "VARCN");
}

#[test]
fn vatdf_undefined_id_code() {
    // Remove id5's definition: the definition still uses [id5].
    let src = VALID.replace(
        r#"            ["id5"] = <text = <"Cuff size"> description = <"Cuff size.">>"#,
        "",
    );
    expect_code(&src, "VATDF");
}

#[test]
fn vacdf_undefined_ac_code() {
    let src = VALID
        .replace(
            r#"            ["ac1"] = <text = <"Cuff sizes"> description = <"Any cuff size.">>"#,
            "",
        )
        // Keep the value_sets consistent so VACDF (definition use) fires first.
        .replace(
            r#"        ["ac1"] = <
            id = <"ac1">
            members = <"at5", "at6">
        >"#,
            "",
        );
    expect_code(&src, "VACDF");
}

#[test]
fn volt_original_language_not_in_terminology() {
    let src = VALID.replace("[ISO_639-1::en]", "[ISO_639-1::nl]");
    expect_code(&src, "VOLT");
}

#[test]
fn votm_translation_language_without_terminology() {
    let src = VALID.replace(
        "language\n    original_language = <[ISO_639-1::en]>",
        "language\n    original_language = <[ISO_639-1::en]>\n    translations = <\n        [\"de\"] = <\n            language = <[ISO_639-1::de]>\n        >\n    >",
    );
    expect_code(&src, "VOTM");
}

#[test]
fn vtlc_language_code_sets_differ() {
    // Add a German set missing every code but id1.
    let src = VALID.replace(
        "    value_sets = <",
        "        [\"de\"] = <\n            [\"id1\"] = <text = <\"Blutdruck\"> description = <\"Blutdruck.\">>\n        >\n    >\n    ignored = <\"x\">\n    value_sets_off = <",
    );
    // The mutation above restructures oddly; build the simpler direct form.
    let src2 = VALID.replace(
        r#"    term_definitions = <
        ["en"] = <"#,
        r#"    term_definitions = <
        ["de"] = <
            ["id1"] = <text = <"Blutdruck"> description = <"Blutdruck.">>
        >
        ["en"] = <"#,
    );
    let _ = src;
    expect_code(&src2, "VTLC");
}

#[test]
fn vtvsid_value_set_id_undefined() {
    let src = VALID.replace(
        r#"["ac1"] = <
            id = <"ac1">"#,
        r#"["ac9"] = <
            id = <"ac9">"#,
    );
    expect_code(&src, "VTVSID");
}

#[test]
fn vtvsmd_member_undefined() {
    let src = VALID.replace(r#"members = <"at5", "at6">"#, r#"members = <"at5", "at9">"#);
    expect_code(&src, "VTVSMD");
}

#[test]
fn vtvsuq_member_duplicated() {
    let src = VALID.replace(r#"members = <"at5", "at6">"#, r#"members = <"at5", "at5">"#);
    expect_code(&src, "VTVSUQ");
}

#[test]
fn vttbk_binding_key_undefined() {
    let src = VALID.replace(
        "    value_sets = <",
        "    term_bindings = <\n        [\"snomed_ct\"] = <\n            [\"at9\"] = <http://snomed.info/id/1234>\n        >\n    >\n    value_sets = <",
    );
    expect_code(&src, "VTTBK");
}

#[test]
fn specialised_artefact_meta() {
    let src = VALID
        .replace("OBSERVATION[id1]", "OBSERVATION[id1.1]")
        .replace(
            "\nlanguage\n",
            "\nspecialize\n    openEHR-EHR-OBSERVATION.parent.v1.0.0\n\nlanguage\n",
        )
        .replace(
            r#"            ["id6"] = <text = <"Coded cuff"> description = <"Coded cuff.">>"#,
            r#"            ["id6"] = <text = <"Coded cuff"> description = <"Coded cuff.">>
            ["id1.1"] = <text = <"BP child"> description = <"Specialised bp.">>"#,
        );
    let meta = validate_adl2_source(&src).expect("valid specialised source");
    assert_eq!(meta.depth, 1);
    assert_eq!(
        meta.parent_hrid.as_deref(),
        Some("openEHR-EHR-OBSERVATION.parent.v1.0.0")
    );
    // VACSD: parent depth 0 → child depth must be 1.
    ehrbase::validation::adl2::check_specialisation_depth(&meta, 0).expect("depth 1 = 0 + 1");
    assert_eq!(
        ehrbase::validation::adl2::check_specialisation_depth(&meta, 1)
            .err()
            .map(|v| v.code),
        Some("VACSD")
    );
}

#[test]
fn vdifv_differential_path_in_top_level_artefact() {
    let src = VALID.replace(
        "        data matches {",
        "        /data[id2]/events matches {",
    );
    expect_code(&src, "VDIFV");
}

#[test]
fn vtsd_code_deeper_than_artefact_depth() {
    // Define a level-1 code in a depth-0 archetype.
    let src = VALID.replace(
        r#"            ["at6"] = <text = <"Child"> description = <"Child cuff.">>"#,
        r#"            ["at6"] = <text = <"Child"> description = <"Child cuff.">>
            ["at6.1"] = <text = <"Deep"> description = <"Too deep.">>"#,
    );
    expect_code(&src, "VTSD");
}

#[test]
fn terminology_code_constraint_structure() {
    // `[at5; at6]` — an assumed value on an at-code is not a legal
    // C_TERMINOLOGY_CODE shape (single ac / single at / `[ac; at]` only).
    let src = VALID.replace("{[ac1; at5]}", "{[at5; at6]}");
    expect_code(&src, "C_TERMINOLOGY_CODE_validity");
}

#[test]
fn vatda_assumed_code_outside_value_set() {
    // at6 is in the ac1 value set; a foreign assumed at-code is not.
    let src = VALID.replace("{[ac1; at5]}", "{[ac1; at7]}").replace(
        r#"            ["at6"] = <text = <"Child"> description = <"Child cuff.">>"#,
        r#"            ["at6"] = <text = <"Child"> description = <"Child cuff.">>
            ["at7"] = <text = <"Neonatal"> description = <"Neonatal cuff.">>"#,
    );
    expect_code(&src, "VATDA");
}

#[test]
fn external_qualified_codes_are_not_checked_locally() {
    // `[snomed_ct::53829009]` in the definition is an external code (VETDF is
    // advisory: "warn only" where the terminology is inaccessible), never a
    // VATDF violation.
    let src = VALID.replace(
        "defining_code matches {[ac1; at5]}",
        "defining_code matches {[ac1; at5]}\n                                                ext matches {[snomed_ct::53829009]}",
    );
    validate_adl2_source(&src).expect("qualified external codes are not local-definedness checked");
}
