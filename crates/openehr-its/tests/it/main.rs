//! Integration tests for `openehr-its`: the openEHR Implementation
//! Technology Specification surfaces — canonical JSON (fidelity, contract,
//! codec parity, ITS-JSON schema validation), canonical XML (round-trip,
//! namespaces, C14N/hash, locatable attributes), the generated ITS-REST
//! contract, RM/template validation, and the Simplified Formats
//! (Web Template / FLAT / STRUCTURED / TDD).
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate);
//! `common` is the shared corpus plumbing, never a test binary of its own.

#![expect(
    clippy::disallowed_types,
    reason = "test fixtures and wire assertions are raw JSON by the testing rule \
              (.claude/rules/testing.md §Test-fixture construction)"
)]

mod common;

mod aom2_model_xml;
mod aom2_xml;
mod attested_version;
mod canonical_contract;
mod ckm_archetype_xml;
mod ckm_full_pack;
mod cnf_vitals_template;
mod coded_names;
mod constraint_binding_capture;
mod content_constraint_capture;
mod content_existence_capture;
mod corpus;
mod dispatch_lockstep;
mod example_rm_validity;
mod example_stub;
mod fidelity;
mod fixture_twins;
mod flat;
mod format_parity;
mod its_json_delta;
mod json_codec_parity;
mod master05_tables;
mod model_walkgen;
mod nonempty_wire;
mod oas_update_version_sync;
mod opt14_corpus;
mod opt14_v1_4_divergence;
mod rest_contract;
mod rm_validation;
mod shape_tolerance;
mod spec_vectors;
mod structured;
mod tdd;
mod untagged_nodes;
mod validation;
mod validation_checklist;
mod validation_rules;
mod webtemplate;
mod webtemplate_v2_4;
mod xml_abstract_root;
mod xml_c14n;
mod xml_ehrbase;
mod xml_hash;
mod xml_hostile_input;
mod xml_locatable_attr;
mod xml_namespace;
mod xml_roundtrip;
mod xml_smoke;
mod xml_xsd_validity;
