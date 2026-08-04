//! Integration tests for `openehr-rm`: the generated RM 1.2.0 type layer —
//! the static RM attribute/type model the AQL planner reads, and the
//! canonical-JSON `_type` dispatch of abstract/polymorphic RM slots.
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate).

mod enforcement_reach;
mod nonempty_rules;
mod rm_model;
mod type_dispatch;
