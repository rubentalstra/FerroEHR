//! Structure tests for the AOM rules / slot-assertion composition
//! (`openehr_adl::rules`). Deep-structure assertions hand-derived from the
//! `AOM2` master05 / `master04.3` shapes.

#![allow(clippy::unwrap_used, clippy::panic)]

use openehr_adl::rules::{parse_rules_body, parse_slot_assertion};
use openehr_am::am24::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
use openehr_am::am24::aom2::rules::expr_constraint::ExprConstraint;
use openehr_am::am24::beom::core::expr_value_ref::ExprValueRef;
use openehr_am::am24::beom::core::expression::Expression;
use openehr_am::am24::beom::core::statement::Statement;

fn only_assertion(body: &str) -> Expression {
    let set = parse_rules_body(body).unwrap_or_else(|e| panic!("parse {body:?}: {e:?}"));
    assert_eq!(set.statement.len(), 1, "expected one statement in {body:?}");
    match set.statement.into_iter().next().unwrap() {
        Statement::Assertion(a) => *a.expression,
        other => panic!("expected assertion, got {other:?}"),
    }
}

fn binop(e: &Expression) -> (&str, &Expression, &Expression) {
    match e {
        Expression::ExprBinaryOperator(b) => {
            (b.operator.as_str(), &b.left_operand, &b.right_operand)
        }
        other => panic!("expected binary operator, got {other:?}"),
    }
}

fn archetype_ref_path(e: &Expression) -> &str {
    match e {
        Expression::ExprValueRef(ExprValueRef::ExprArchetypeRef(r)) => r.path.as_str(),
        other => panic!("expected EXPR_ARCHETYPE_REF, got {other:?}"),
    }
}

#[test]
fn path_leaf_becomes_expr_archetype_ref() {
    // A bare rule path is an EXPR_ARCHETYPE_REF (master05).
    let e = only_assertion("/data[id2]/items[id5]/value/magnitude = 10");
    let (op, l, _) = binop(&e);
    assert_eq!(op, "eq");
    assert_eq!(
        archetype_ref_path(l),
        "/data[id2]/items[id5]/value/magnitude"
    );
}

#[test]
fn dependency_rule_matches_implies_exists() {
    // Hand-derived from features/aom_structures/rules/…dependency_rule…: an
    // implication whose antecedent is a `matches` terminology constraint and
    // whose consequent is an `exists` over a path.
    let e = only_assertion(
        "/data[id2]/items[id21]/items[id15]/value[id50]/defining_code matches {[at19]} \
         implies exists /data[id2]/items[id21]/items[id20]",
    );
    let (op, lhs, rhs) = binop(&e);
    assert_eq!(op, "implies");

    // antecedent: EXPR_ARCHETYPE_REF matches EXPR_CONSTRAINT(C_TERMINOLOGY_CODE).
    let (mop, mlhs, mrhs) = binop(lhs);
    assert_eq!(mop, "matches");
    assert_eq!(
        archetype_ref_path(mlhs),
        "/data[id2]/items[id21]/items[id15]/value[id50]/defining_code"
    );
    match mrhs {
        Expression::ExprConstraint(ExprConstraint::ExprConstraint(c)) => {
            assert!(matches!(c.item, CPrimitiveObject::CTerminologyCode(_)));
        }
        other => panic!("expected EXPR_CONSTRAINT, got {other:?}"),
    }

    // consequent: exists <path> (a unary operator).
    match rhs {
        Expression::ExprUnaryOperator(u) => {
            assert_eq!(u.operator.as_str(), "exists");
            assert_eq!(
                archetype_ref_path(&u.operand),
                "/data[id2]/items[id21]/items[id20]"
            );
        }
        other => panic!("expected unary exists, got {other:?}"),
    }
}

#[test]
fn tagged_sum_assertion() {
    // The rules_sum shape: `tag: /path = /a + /b`.
    let set = parse_rules_body("score: /data[id3]/x = /data[id3]/a + /data[id3]/b").expect("parse");
    let Statement::Assertion(a) = set.statement.into_iter().next().unwrap() else {
        panic!("expected assertion");
    };
    assert_eq!(a.tag.as_deref(), Some("score"));
    let (op, _, r) = binop(&a.expression);
    assert_eq!(op, "eq");
    assert_eq!(binop(r).0, "plus");
}

#[test]
fn real_literal_and_precedence_in_formula() {
    // rules_formulae shape: 0.33 * ( … - … ) inside a sum.
    let e = only_assertion("/data/x = /data/a + 0.33 * (/data/b - /data/c)");
    let (op, _, rhs) = binop(&e);
    assert_eq!(op, "eq");
    let (plus, _, prod) = binop(rhs);
    assert_eq!(plus, "plus");
    assert_eq!(binop(prod).0, "multiply");
}

#[test]
fn slot_assertion_is_archetype_ref_matches_id_constraint() {
    // master04.3 §Archetype Slots: `archetype_id/value matches { /regex/ }`
    // parses to EXPR_ARCHETYPE_REF matches EXPR_ARCHETYPE_ID_CONSTRAINT.
    let text = "archetype_id/value matches {/openEHR-EHR-OBSERVATION\\..*\\.v1/}";
    let assertion = parse_slot_assertion(text).unwrap_or_else(|e| panic!("slot parse: {e:?}"));
    assert_eq!(assertion.string_expression.as_deref(), Some(text));
    let (op, lhs, rhs) = binop(&assertion.expression);
    assert_eq!(op, "matches");
    assert_eq!(archetype_ref_path(lhs), "archetype_id/value");
    assert!(
        matches!(
            rhs,
            Expression::ExprConstraint(ExprConstraint::ExprArchetypeIdConstraint(_))
        ),
        "slot RHS must be EXPR_ARCHETYPE_ID_CONSTRAINT, got {rhs:?}"
    );
}

#[test]
fn invalid_rule_expression_is_typed_error() {
    // An unparsable rule expression surfaces a typed error, never a panic.
    let err = parse_rules_body("exists").unwrap_err();
    assert!(!err.is_empty());
}
