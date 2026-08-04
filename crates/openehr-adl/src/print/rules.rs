//! The `rules` section (`ADL2/master07.11`) and the BEL expression printers
//! that also serve slot include/exclude assertions (`master04.6`). Expressions
//! are rendered with full parenthesization so they re-parse to the identical
//! tree.

#![expect(
    clippy::disallowed_types,
    reason = "ODIN-to-JSON conversion targets the JSON data model by specification (LANG odin \
              spec) (#1694)"
)]

use openehr_am::am24::aom2::rules::expr_constraint::ExprConstraint;
use openehr_am::am24::beom::core::expr_value::ExprValue;
use openehr_am::am24::beom::core::expr_value_ref::ExprValueRef;
use openehr_am::am24::beom::core::expression::Expression;
use openehr_am::am24::beom::core::statement::Statement;
use openehr_am::am24::beom::core::statement_set::StatementSet;

use crate::print::Printer;
use crate::print::definition::{bool_str, primitive_inline};
use crate::print::odin::quoted;

impl Printer {
    // ── rules (master07.11; BEL) ───────────────────────────────────────────
    pub(super) fn rules(&mut self, set: &StatementSet) {
        for stmt in set.statement.iter().flatten() {
            match stmt {
                Statement::Assertion(a) => {
                    let expr = expression_str(&a.expression);
                    match &a.tag {
                        Some(tag) => self.line(1, &format!("{tag}: {expr}")),
                        None => self.line(1, &expr),
                    }
                }
                Statement::Assignment(a) => {
                    self.line(
                        1,
                        &format!("${} = {}", a.target.name, expr_value_str(&a.source)),
                    );
                }
                Statement::VariableDeclaration(v) => {
                    self.line(1, &format!("${} : {}", v.name, type_def_name(&v.r#type)));
                }
            }
        }
    }
}

// ── rules expression printing (full parenthesization) ──────────────────────

/// A single slot include/exclude assertion — the verbatim `string_expression`
/// is the round-trip carrier (`master04.6`); otherwise the expression tree.
pub(super) fn assertion_str(a: &openehr_am::am24::beom::core::assertion::Assertion) -> String {
    if let Some(s) = &a.string_expression {
        return s.clone();
    }
    expression_str(&a.expression)
}

/// Render an [`Expression`] with full parenthesization so it re-parses to the
/// identical tree regardless of operator precedence (the BEL parser drops
/// redundant parentheses — `bel::parser::parse_primary` — so extra parens never
/// change the built tree).
fn expression_str(e: &Expression) -> String {
    match e {
        Expression::ExprLiteral(l) => literal_str(&l.item),
        Expression::ExprVariableRef(v) => format!("${}", v.item.name),
        Expression::ExprValueRef(r) => value_ref_str(r),
        Expression::ExprBinaryOperator(b) => {
            let sym = b.symbol.as_deref().unwrap_or(b.operator.as_str());
            let left = expression_str(&b.left_operand);
            if sym == "matches" {
                format!("({left} matches {})", constraint_rhs(&b.right_operand))
            } else {
                format!("({left} {sym} {})", expression_str(&b.right_operand))
            }
        }
        Expression::ExprUnaryOperator(u) => {
            let sym = u.symbol.as_deref().unwrap_or(u.operator.as_str());
            if sym == "exists" {
                // `exists` binds a bare reference leaf (no parentheses; the BEL
                // grammar's `parse_ref_leaf`).
                format!("exists {}", ref_leaf_str(&u.operand))
            } else {
                format!("{sym} ({})", expression_str(&u.operand))
            }
        }
        Expression::ExprFunctionCall(f) => {
            let name = f.item.as_ref().and_then(|v| v.as_str()).unwrap_or_default();
            let args = f
                .arguments
                .iter()
                .flatten()
                .map(expression_str)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}({args})")
        }
        Expression::ExprForAll(fa) => {
            let collection = value_ref_str(&fa.operand);
            let cond = expression_str(&fa.condition.expression);
            format!("for_all $x : {collection} | {cond}")
        }
        Expression::ExprConstraint(c) => constraint_leaf_str(c),
    }
}

/// The RHS of a `matches` operator: the cADL primitive/regex, wrapped in braces.
fn constraint_rhs(e: &Expression) -> String {
    match e {
        Expression::ExprConstraint(c) => constraint_leaf_str(c),
        other => expression_str(other),
    }
}

/// A brace-wrapped constraint leaf: an inline primitive, or the archetype-id
/// regex matcher of a slot assertion.
fn constraint_leaf_str(c: &ExprConstraint) -> String {
    match c {
        ExprConstraint::ExprConstraint(d) => format!("{{{}}}", primitive_inline(&d.item)),
        ExprConstraint::ExprArchetypeIdConstraint(a) => {
            // A C_STRING regex matcher (`master04.3` §Archetype Slots).
            format!(
                "{{{}}}",
                a.item
                    .constraint
                    .iter()
                    .flatten()
                    .next()
                    .cloned()
                    .unwrap_or_default()
            )
        }
    }
}

/// The path text of a value reference (an archetype path or a named leaf).
fn value_ref_str(r: &ExprValueRef) -> String {
    match r {
        ExprValueRef::ExprArchetypeRef(a) => a.path.clone(),
        ExprValueRef::ExprValueRef(v) => v
            .item
            .as_ref()
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_owned(),
    }
}

/// The bare (unparenthesized) reference leaf an `exists` operator binds.
fn ref_leaf_str(e: &Expression) -> String {
    match e {
        Expression::ExprValueRef(r) => value_ref_str(r),
        Expression::ExprVariableRef(v) => format!("${}", v.item.name),
        other => expression_str(other),
    }
}

/// The RHS of an assignment statement, over the `EXPR_VALUE` union.
fn expr_value_str(v: &ExprValue) -> String {
    match v {
        ExprValue::ExprBinaryOperator(b) => {
            expression_str(&Expression::ExprBinaryOperator(Box::new(b.clone())))
        }
        ExprValue::ExprUnaryOperator(u) => {
            expression_str(&Expression::ExprUnaryOperator(Box::new(u.clone())))
        }
        ExprValue::ExprForAll(f) => expression_str(&Expression::ExprForAll(Box::new(f.clone()))),
        ExprValue::ExprFunctionCall(f) => expression_str(&Expression::ExprFunctionCall(f.clone())),
        ExprValue::ExprLiteral(l) => literal_str(&l.item),
        ExprValue::ExprValueRef(r) => value_ref_str(r),
        ExprValue::ExprVariableRef(v) => format!("${}", v.item.name),
        ExprValue::ExprConstraint(c) => constraint_leaf_str(c),
        ExprValue::ExternalQuery(_) => String::new(),
    }
}

/// A BEL literal in its source spelling.
fn literal_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Bool(b) => bool_str(*b).to_owned(),
        serde_json::Value::String(s) => quoted(s),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// The declared type name of a variable declaration (`Any` when untyped).
fn type_def_name(t: &openehr_lang::prelude::ExprTypeDef) -> String {
    match t {
        openehr_lang::prelude::ExprTypeDef::TypeDefObjectRef(r) => r.type_name.clone(),
        _ => "Any".to_owned(),
    }
}
