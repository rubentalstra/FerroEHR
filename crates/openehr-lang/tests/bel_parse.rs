//! Public-API tests for the pure-BEL parser (`openehr_lang::bel`, the
//! [`BeomBuilder`] path). Structural assertions over the generated `beom` tree;
//! the AOM-extended path is exercised in `openehr-adl`.

#![allow(clippy::unwrap_used, clippy::panic)]

use openehr_lang::bel::{BelError, parse_statements};
use openehr_lang::prelude::{Expression, Statement};

fn one_assertion(src: &str) -> Expression {
    let stmts = parse_statements(src).unwrap_or_else(|e| panic!("parse {src:?}: {e}"));
    assert_eq!(stmts.len(), 1, "expected one statement in {src:?}");
    match stmts.into_iter().next().unwrap() {
        Statement::Assertion(a) => *a.expression,
        other => panic!("expected an assertion, got {other:?}"),
    }
}

fn binary(e: &Expression) -> (&str, &Expression, &Expression) {
    match e {
        Expression::ExprBinaryOperator(b) => {
            (b.operator.0.as_str(), &b.left_operand, &b.right_operand)
        }
        other => panic!("expected a binary operator, got {other:?}"),
    }
}

fn is_path(e: &Expression, path: &str) -> bool {
    matches!(e, Expression::ExprValueRef(r)
        if r.item.as_ref().and_then(|v| v.as_str()) == Some(path))
}

fn is_int(e: &Expression, n: i64) -> bool {
    matches!(e, Expression::ExprLiteral(l) if l.item.as_i64() == Some(n))
}

#[test]
fn arithmetic_precedence_multiply_binds_tighter_than_plus() {
    // 1 + 2 * 3  ==  1 + (2 * 3)
    let e = one_assertion("1 + 2 * 3");
    let (op, l, r) = binary(&e);
    assert_eq!(op, "plus");
    assert!(is_int(l, 1));
    let (op2, l2, r2) = binary(r);
    assert_eq!(op2, "multiply");
    assert!(is_int(l2, 2) && is_int(r2, 3));
}

#[test]
fn exponent_is_right_associative() {
    // 2 ^ 3 ^ 2  ==  2 ^ (3 ^ 2)
    let e = one_assertion("2 ^ 3 ^ 2");
    let (op, l, r) = binary(&e);
    assert_eq!(op, "exponent");
    assert!(is_int(l, 2));
    assert_eq!(binary(r).0, "exponent");
}

#[test]
fn equality_over_arithmetic_sum() {
    // tagged assertion: total: /data/x = /data/a + /data/b
    let stmts = parse_statements("total: /data/x = /data/a + /data/b").expect("parse");
    let Statement::Assertion(a) = stmts.into_iter().next().unwrap() else {
        panic!("expected assertion");
    };
    assert_eq!(a.tag.as_deref(), Some("total"));
    let (op, l, r) = binary(&a.expression);
    assert_eq!(op, "eq");
    assert!(is_path(l, "/data/x"));
    let (op2, l2, _) = binary(r);
    assert_eq!(op2, "plus");
    assert!(is_path(l2, "/data/a"));
}

#[test]
fn implies_is_lowest_precedence() {
    // a and b implies c  ==  (a and b) implies c
    let e = one_assertion("a and b implies c");
    let (op, l, r) = binary(&e);
    assert_eq!(op, "implies");
    assert!(is_path(r, "c"));
    assert_eq!(binary(l).0, "and");
}

#[test]
fn symbol_and_text_operator_forms_are_equivalent() {
    assert_eq!(binary(&one_assertion("a and b")).0, "and");
    assert_eq!(binary(&one_assertion("a \u{2227} b")).0, "and"); // ∧
    assert_eq!(binary(&one_assertion("a \u{2228} b")).0, "or"); // ∨
    assert_eq!(binary(&one_assertion("a -> b")).0, "implies");
}

#[test]
fn not_unary_both_forms() {
    for src in ["not a", "~a", "\u{00AC}a"] {
        match one_assertion(src) {
            Expression::ExprUnaryOperator(u) => assert_eq!(u.operator.0, "not"),
            other => panic!("{src:?}: expected unary not, got {other:?}"),
        }
    }
}

#[test]
fn exists_path_is_unary() {
    match one_assertion("exists /data/events") {
        Expression::ExprUnaryOperator(u) => {
            assert_eq!(u.operator.0, "exists");
            assert!(is_path(&u.operand, "/data/events"));
        }
        other => panic!("expected unary exists, got {other:?}"),
    }
}

#[test]
fn assignment_and_declaration() {
    let stmts = parse_statements("$x : Integer\n$x := /data/count").expect("parse");
    assert_eq!(stmts.len(), 2);
    assert!(matches!(&stmts[0], Statement::VariableDeclaration(v) if v.name == "x"));
    match &stmts[1] {
        Statement::Assignment(a) => assert_eq!(a.target.name, "x"),
        other => panic!("expected assignment, got {other:?}"),
    }
}

#[test]
fn parenthesised_overrides_precedence() {
    // (1 + 2) * 3
    let e = one_assertion("(1 + 2) * 3");
    let (op, l, r) = binary(&e);
    assert_eq!(op, "multiply");
    assert!(is_int(r, 3));
    assert_eq!(binary(l).0, "plus");
}

#[test]
fn matches_constraint_rejected_by_pure_builder() {
    // The AOM `matches { … }` leaf is EXPR_CONSTRAINT (not in beom); the pure
    // builder surfaces a typed Unsupported error rather than mis-parsing.
    let err = parse_statements("/data/x matches {|140..160|}").unwrap_err();
    assert!(matches!(err, BelError::Unsupported { .. }), "got {err:?}");
}

#[test]
fn lex_error_is_typed_and_located() {
    let err = parse_statements("a \u{00a7} b").unwrap_err(); // § is not a BEL token
    assert!(matches!(err, BelError::Lex { .. }), "got {err:?}");
}
