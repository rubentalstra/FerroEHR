// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The printer's own documented invariant, asserted over every boolean shape:
//! for any AST the parser produces, `parse(to_aql(ast)) == ast`.
//!
//! The shapes here were found by the `aql_query` fuzz harness (`fuzz/`), which
//! parses arbitrary text and asserts the same invariant. Two defects surfaced,
//! both about parentheses the printer dropped:
//!
//! - `AqlParser.g4` states `AND`/`OR` as binary alternatives of one recursive
//!   rule, which ANTLR4 resolves LEFT-associatively, so a same-precedence right
//!   operand must keep its parentheses to survive a re-parse;
//! - `containsExpr: classExprOperand (NOT? CONTAINS containsExpr)?` makes the
//!   `CONTAINS` operand a whole `containsExpr`, so an unparenthesised
//!   `A CONTAINS B` used as a boolean operand absorbs the operator that follows
//!   it — which moves the operator INTO the CONTAINS scope and changes what the
//!   query means.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    reason = "integration-test assertions and fixture plumbing"
)]

/// Every AQL source below must satisfy `parse(to_aql(parse(src))) == parse(src)`.
fn assert_round_trips(src: &str) {
    let parsed = openehr_query::parser::parse_str(src)
        .unwrap_or_else(|e| panic!("fixture must parse: {src}\n  {e}"));
    let printed = openehr_query::printer::to_aql(&parsed);
    let reparsed = openehr_query::parser::parse_str(&printed)
        .unwrap_or_else(|e| panic!("printed AQL must reparse: {printed}\n  {e}"));
    assert_eq!(
        reparsed, parsed,
        "printer round-trip drifted the AST\n  source:  {src}\n  printed: {printed}"
    );
}

/// The WHERE tree: both associativity sides of both operators, plus `NOT`.
#[test]
fn where_boolean_shapes_round_trip() {
    const HEAD: &str = "SELECT e/ehr_id/value FROM EHR e WHERE ";
    for tail in [
        // Left-nested — the parser's own default shape, parenthesis-free.
        "(e/a = 1 AND e/b = 1) AND e/c = 1",
        "(e/a = 1 OR e/b = 1) OR e/c = 1",
        // Right-nested at the same precedence — the associativity finding.
        "e/a = 1 AND (e/b = 1 AND e/c = 1)",
        "e/a = 1 OR (e/b = 1 OR e/c = 1)",
        // Both sides right-nested, the shape the fuzzer actually reported.
        "(e/a = 1 AND e/b = 1) AND (e/c = 1 AND e/d = 1)",
        // Precedence: OR below AND, in every position.
        "(e/a = 1 OR e/b = 1) AND e/c = 1",
        "e/a = 1 AND (e/b = 1 OR e/c = 1)",
        "e/a = 1 OR e/b = 1 AND e/c = 1",
        "e/a = 1 AND e/b = 1 OR e/c = 1",
        // NOT binds tighter than AND, so a boolean operand needs parens.
        "NOT (e/a = 1 AND e/b = 1)",
        "NOT (e/a = 1 OR e/b = 1)",
        "NOT e/a = 1 AND e/b = 1",
        "NOT NOT e/a = 1",
    ] {
        assert_round_trips(&format!("{HEAD}{tail}"));
    }
}

/// The FROM tree: the same associativity rule, plus the greedy `CONTAINS`
/// operand.
#[test]
fn contains_boolean_shapes_round_trip() {
    const HEAD: &str = "SELECT c/uid/value FROM ";
    for tail in [
        "(COMPOSITION c AND OBSERVATION o) AND CLUSTER l",
        "COMPOSITION c AND (OBSERVATION o AND CLUSTER l)",
        "COMPOSITION c OR (OBSERVATION o OR CLUSTER l)",
        "(COMPOSITION c OR OBSERVATION o) AND CLUSTER l",
        "COMPOSITION c AND (OBSERVATION o OR CLUSTER l)",
        // The greedy-CONTAINS finding: the parenthesised form keeps the `AND`
        // OUTSIDE the CONTAINS scope, and printing must not move it in.
        "(EHR e CONTAINS COMPOSITION c) AND OBSERVATION o",
        "(EHR e CONTAINS COMPOSITION c) OR OBSERVATION o",
        "OBSERVATION o AND (EHR e CONTAINS COMPOSITION c)",
        // Its unparenthesised twin, where the operator belongs INSIDE.
        "EHR e CONTAINS COMPOSITION c AND OBSERVATION o",
        "EHR e CONTAINS (COMPOSITION c AND OBSERVATION o)",
        "EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o",
    ] {
        assert_round_trips(&format!("{HEAD}{tail}"));
    }
}

/// The two shapes are genuinely DIFFERENT queries, so the printer moving an
/// operator across a `CONTAINS` boundary would be a silent wrong answer, not a
/// cosmetic drift.
#[test]
fn a_parenthesised_contains_is_not_its_unparenthesised_twin() {
    let grouped = openehr_query::parser::parse_str(
        "SELECT c/uid/value FROM (EHR e CONTAINS COMPOSITION c) AND OBSERVATION o",
    )
    .unwrap();
    let greedy = openehr_query::parser::parse_str(
        "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c AND OBSERVATION o",
    )
    .unwrap();
    assert_ne!(
        grouped, greedy,
        "the CONTAINS operand is greedy, so these two sources must not parse alike"
    );
}
