//! Integration tests for `openehr-lang`: the shared lexical layer, the ODIN +
//! BMM reader and the Basic Expression Language parser — the per-language
//! lexer-surface battery, the vendored-fixture batteries (`tests/vendor/**`),
//! their 100%-coverage gate, and BEL parsing.
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate).

mod bel_parse;
mod escape_validation;
mod lexer_equivalence;
mod odin_spec_examples;
mod vendor_bmm_odin;
mod vendor_coverage;
mod vendor_odin;
