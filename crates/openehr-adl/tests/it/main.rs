//! Integration tests for `openehr-adl`: the ADL 2.4 engine — the ADL2/cADL/ODIN
//! parser, AOM2 validation (phase 1/2 + RM conformance), the flattener, OPT2
//! generation, ADL 1.4 reading and 1.4→2 conversion — driven by the vendored
//! reference corpora under `tests/corpus/**` and their coverage gate.
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate).

mod adl14_assertions;
mod adl14_cadl_gates;
mod adl14_cnf_fixture_slot;
mod adl14_conversion;
mod adl14_dadl_breadth;
mod adl14_header_sections;
mod assembly;
mod corpus_coverage;
mod corpus_definition_parse;
mod corpus_lex;
mod corpus_outer_parse;
mod corpus_roundtrip;
mod corpus_rules_parse;
mod corpus_validity_phase1;
mod corpus_validity_phase2;
mod corpus_validity_rm;
mod flattener_spec;
mod legacy14_corpus;
mod opt_spec;
mod rules_parse;
mod templates_corpus;
mod validation_adl14;
mod validation_phase1_cases;
mod vetdf_terminology;
