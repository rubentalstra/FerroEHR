//! The ADL 1.4 cADL regression corpus (`tests/corpus/adl14-cadl/`).
//!
//! Sibling of `adl14-dadl/` (the dADL leaf/structure breadth tree): this one
//! covers the **cADL** half of an ADL 1.4 text — `master05-cadl.adoc` — where the
//! vendored `adl2-reference` library gives no coverage because it is an ADL2
//! corpus. Like its sibling it is **hand-written, not vendored**
//! (`tests/corpus/PROVENANCE.md` §`adl14-cadl/`), and every expectation is
//! derived first-hand from the spec text, cited in the fixture beside the
//! construct it exercises.
//!
//! Four families live here:
//!
//! 1. **Dialect gates.** ADL 1.4's cADL keyword set is CLOSED
//!    (`ADL1.4/master05-cadl.adoc` §Keywords L48-53), so a construct ADL 2
//!    introduced is a syntax error in a 1.4 text, not a tolerated superset. Each
//!    such construct gets a refusal fixture named for the `S*` code it raises.
//!    The ACCEPTING twin of every one of them is the vendored ADL2 corpus, which
//!    exercises the same construct in its own dialect — a parser that stopped
//!    accepting it there would fail `corpus_definition_parse.rs`.
//! 2. **Refusals with their accepting twin in-tree** — the inline dADL domain
//!    lowering (`ADL1.4/master09-customising_adl.adoc`) and the 1.4 term-constraint
//!    definedness rules (`ADL1.4/master08-adl.adoc` §Validity Rules VATDF/VACDF),
//!    where the accepted form is itself 1.4-only and so has no ADL2 twin.
//! 3. **Positive fixtures** for behaviour a lenient OR an over-strict reader would
//!    get wrong: `before`/`after` sibling order (a 1.4 keyword — L53 — that must
//!    NOT be gated), the effective occurrences default `{1..1}` (L316), and the
//!    assumed-value lowering.
//! 4. **The breadth trio** — `cadl_breadth_{structure,primitives,datetime}` —
//!    which exercises every construct `master05-cadl.adoc` defines (existence
//!    and occurrences and cardinality spellings incl. two modifiers and the bare
//!    star, the whole interval shape set incl. the `infinity` endpoints and both
//!    exclusive-lower spellings, every string/boolean/character/term-code form,
//!    every date/time/date-time/duration pattern family incl. the
//!    literal-substituted and ASCII-timezone variants and the mixed
//!    `pattern/interval` form, assumed values on every primitive, `use_node`
//!    with and without occurrences, generic type names, and the bare and
//!    identified slot forms) plus `cadl_keyword_case`, which repeats a whole
//!    definition in upper/mixed case because the chapter's own lexical
//!    specification is case-insensitive (§Symbols L1326-1354).
//!
//! Both twins are kept for every refusal (`.claude/rules/testing.md`).

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "integration-test assertions, diagnostics and fixture plumbing outside #[test] fns, which the clippy.toml allow-*-in-tests scoping does not reach"
)]

use std::path::{Path, PathBuf};

use openehr_adl::assemble::parse_artefact_adl14;
use openehr_adl::error::SyntaxErrorCode;
use openehr_adl::validate::{Severity, ValidationCode, validate_source_phase1_adl14};

/// What a fixture must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// Parses in the 1.4 dialect and validates clean under the 1.4 phase-1 subset.
    Pass,
    /// Refused at parse with this syntax code.
    Refuse(SyntaxErrorCode),
    /// Parses, but the 1.4 phase-1 subset raises this validation code as an error.
    Invalid(ValidationCode),
}

/// Every fixture of the tree, with its expected outcome. The coverage gate
/// (`corpus_coverage.rs`) cross-checks this list against the filesystem.
const FIXTURES: &[(&str, Expect)] = &[
    // ── dialect gates (ADL2-only constructs in a 1.4 text) ────────────────
    (
        "openEHR-EHR-OBSERVATION.SCOAT_adl2_default_value.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Scoat),
    ),
    (
        "openEHR-EHR-OBSERVATION.SCOAT_adl2_attribute_tuple.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Scoat),
    ),
    (
        "openEHR-EHR-CLUSTER.SCCOG_adl2_use_archetype.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Sccog),
    ),
    (
        "openEHR-EHR-CLUSTER.SCCOG_adl2_slot_closed.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Sccog),
    ),
    (
        "openEHR-EHR-CLUSTER.STCCP_adl2_constraint_strength.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Stccp),
    ),
    (
        "openEHR-EHR-CLUSTER.STCCP_adl2_terminology_binding.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Stccp),
    ),
    // ── inline dADL domain lowering (master09) ────────────────────────────
    (
        "openEHR-EHR-CLUSTER.SDINV_unsupported_domain_type.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Sdinv),
    ),
    (
        "openEHR-EHR-CLUSTER.SDINV_assumed_value_unmatched.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Sdinv),
    ),
    (
        "openEHR-EHR-CLUSTER.domain_assumed_value.v1.adl",
        Expect::Pass,
    ),
    (
        "openEHR-EHR-CLUSTER.code_phrase_dadl_block.v1.adl",
        Expect::Pass,
    ),
    // ── the deprecated 1.4 pipe-ordinal shorthand (master04.4) ────────────
    (
        "openEHR-EHR-CLUSTER.ordinal_pipe_shorthand.v1.adl",
        Expect::Pass,
    ),
    // ── 1.4 term-constraint definedness (master08 VATDF/VACDF) ────────────
    (
        "openEHR-EHR-CLUSTER.VATDF_undefined_listed_code.v1.adl",
        Expect::Invalid(ValidationCode::Vatdf),
    ),
    (
        "openEHR-EHR-CLUSTER.VATDF_undefined_code_with_assumed.v1.adl",
        Expect::Invalid(ValidationCode::Vatdf),
    ),
    (
        "openEHR-EHR-CLUSTER.VACDF_undefined_constraint_code.v1.adl",
        Expect::Invalid(ValidationCode::Vacdf),
    ),
    (
        "openEHR-EHR-CLUSTER.term_constraint_codes_defined.v1.adl",
        Expect::Pass,
    ),
    // ── 1.4 term-constraint LIST integrity (master04.6 STCDC/STCAC) ───────
    (
        "openEHR-EHR-CLUSTER.STCDC_duplicate_code_in_list.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Stcdc),
    ),
    (
        "openEHR-EHR-CLUSTER.STCAC_assumed_code_not_in_list.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Stcac),
    ),
    // ── operators the chapter names but no grammar defines ────────────────
    (
        "openEHR-EHR-CLUSTER.SCCOG_negated_matches.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Sccog),
    ),
    (
        "openEHR-EHR-CLUSTER.SCOAT_negated_is_in.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Scoat),
    ),
    (
        "openEHR-EHR-CLUSTER.SCOAT_not_in_symbol.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Scoat),
    ),
    (
        "openEHR-EHR-CLUSTER.SCCOG_regex_match_operator.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Sccog),
    ),
    // ── the date/time interval timezone-symmetry rule (master05 L932) ─────
    (
        "openEHR-EHR-CLUSTER.SCDTAV_interval_timezone_asymmetry.v1.adl",
        Expect::Refuse(SyntaxErrorCode::Scdtav),
    ),
    // ── cardinality/occurrences (master05 VCOC + the {1..1} defaults) ─────
    (
        "openEHR-EHR-CLUSTER.VCOC_occurrences_exceed_cardinality.v1.adl",
        Expect::Invalid(ValidationCode::Vcoc),
    ),
    (
        "openEHR-EHR-CLUSTER.VCOC_occurrences_below_cardinality.v1.adl",
        Expect::Invalid(ValidationCode::Vcoc),
    ),
    (
        "openEHR-EHR-CLUSTER.default_occurrences.v1.adl",
        Expect::Pass,
    ),
    // ── 1.4 keywords that must stay accepted ──────────────────────────────
    ("openEHR-EHR-CLUSTER.sibling_order.v1.adl", Expect::Pass),
    ("openEHR-EHR-CLUSTER.cadl_keyword_case.v1.adl", Expect::Pass),
    // ── the accepting twins of the two vendored concept-less 1.4 fixtures ─
    // `ADL1.4/master08-adl.adoc` §Syntax Specification makes `arch_concept`
    // mandatory and §Validity Rules VARCN requires its term, so
    // `adl2-reference/validity/legacy_adl_1.4/openehr-test_pkg-SOME_TYPE.
    // {c_dv_quantity,code_phrase}.v1.adl` are refused (SACO — pinned as such in
    // `legacy14_corpus.rs`). These are their concept-carrying twins, so the
    // constructs they exercise (1.4 inline dADL `C_DV_QUANTITY` blocks; the
    // vanilla and 1.4-qualified `CODE_PHRASE` constraint spellings) keep their
    // accepted-and-validates-clean coverage.
    (
        "openehr-TEST_PKG-SOME_TYPE.c_dv_quantity_concept.v1.adl",
        Expect::Pass,
    ),
    (
        "openehr-TEST_PKG-SOME_TYPE.code_phrase_concept.v1.adl",
        Expect::Pass,
    ),
    // A slot assertion targeting domain_concept: its literal regex is NOT an
    // archetype id, so VDFAI must not fire (master05 §Archetype Slots; the
    // VDFAI subject is the archetype identifier).
    (
        "openEHR-EHR-SECTION.slot_domain_concept_regex.v1.adl",
        Expect::Pass,
    ),
    // ── the master05 breadth trio (every construct of the chapter) ────────
    (
        "openEHR-EHR-OBSERVATION.cadl_breadth_structure.v1.adl",
        Expect::Pass,
    ),
    (
        "openEHR-EHR-OBSERVATION.cadl_breadth_primitives.v1.adl",
        Expect::Pass,
    ),
    (
        "openEHR-EHR-OBSERVATION.cadl_breadth_datetime.v1.adl",
        Expect::Pass,
    ),
];

fn tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus/adl14-cadl")
}

fn read(name: &str) -> String {
    let path = tree().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn error_codes(src: &str, name: &str) -> Vec<ValidationCode> {
    validate_source_phase1_adl14(src)
        .unwrap_or_else(|e| panic!("{name}: must parse, got {e:?}"))
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.code)
        .collect()
}

#[test]
fn every_fixture_meets_its_declared_outcome() {
    for (name, expect) in FIXTURES {
        let src = read(name);
        match expect {
            Expect::Pass => {
                parse_artefact_adl14(&src)
                    .unwrap_or_else(|e| panic!("{name} must parse, got {e:?}"));
                let codes = error_codes(&src, name);
                assert!(
                    codes.is_empty(),
                    "{name}: expected clean 1.4 validation, got {:?}",
                    codes.iter().map(|c| c.mnemonic()).collect::<Vec<_>>()
                );
            }
            Expect::Refuse(code) => {
                let errs = parse_artefact_adl14(&src)
                    .err()
                    .unwrap_or_else(|| panic!("{name} must be refused at parse"));
                assert!(
                    errs.iter().any(|e| e.code == *code),
                    "{name}: expected {code}, got {:?}",
                    errs.iter().map(|e| e.code).collect::<Vec<_>>()
                );
            }
            Expect::Invalid(code) => {
                let codes = error_codes(&src, name);
                assert!(
                    codes.contains(code),
                    "{name}: expected {code}, got {:?}",
                    codes.iter().map(|c| c.mnemonic()).collect::<Vec<_>>()
                );
            }
        }
    }
}

/// The whole tree is claimed by the table above — no fixture may sit unexercised.
#[test]
fn every_fixture_file_is_in_the_table() {
    let mut on_disk: Vec<String> = std::fs::read_dir(tree())
        .expect("read the adl14-cadl tree")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| Path::new(n).extension().is_some_and(|e| e == "adl"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = FIXTURES.iter().map(|(n, _)| (*n).to_owned()).collect();
    listed.sort();
    assert_eq!(on_disk, listed, "the fixture table and the tree disagree");
}
