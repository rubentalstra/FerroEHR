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

mod common;

mod canonical_contract;
mod cnf_vitals_template;
mod coded_names;
mod constraint_binding_capture;
mod content_constraint_capture;
mod content_existence_capture;
mod corpus;
mod example_rm_validity;
mod example_stub;
mod fidelity;
mod flat;
mod json_codec_parity;
mod master05_tables;
mod opt14_am14_divergence;
mod opt14_corpus;
mod rest_contract;
mod rm_validation;
mod spec_vectors;
mod structured;
mod tdd;
mod untagged_nodes;
mod validation;
mod validation_checklist;
mod validation_rules;
mod webtemplate;
mod webtemplate_am24;
mod xml_abstract_root;
mod xml_c14n;
mod xml_ehrbase;
mod xml_hash;
mod xml_locatable_attr;
mod xml_namespace;
mod xml_roundtrip;
mod xml_smoke;
