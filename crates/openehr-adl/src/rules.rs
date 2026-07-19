//! The ADL2 `rules` section + structured slot-assertion parser (phase A3b).
//!
//! The core BEL grammar is `openehr_lang::bel` (a LANG spec — statements,
//! assertions, assignments, operators, literals → the generated `beom` model).
//! This module supplies the **AOM composition**: a [`BelBuilder`] that produces
//! the AM-level expression model (`openehr_am::am24::beom` — the extender enums
//! composing the beom variants + the AOM leaves), with the two AOM-specific leaf
//! productions the spec adds (`AOM2` master05-rules_package):
//!
//! * an archetype/data **path** leaf becomes an `EXPR_ARCHETYPE_REF`
//!   (`master05`; the path is a runtime-value proxy);
//! * a `matches { c_primitive_object }` right-hand side becomes an
//!   `EXPR_CONSTRAINT` wrapping the cADL primitive (reusing the A3a primitive
//!   parser, [`crate::cadl::parse_inline_primitive_text`]);
//! * inside a **slot** assertion (`master04.3` §Archetype Slots), a
//!   `matches { /regex/ }` right-hand side becomes an
//!   `EXPR_ARCHETYPE_ID_CONSTRAINT` (an archetype-id regex matcher).
//!
//! No expression grammar is duplicated: this drives
//! [`openehr_lang::bel::parse_statements_with`] with the AOM builder.

use openehr_am::am24::aom2::constraint_model::archetype_constraint::ArchetypeConstraint;
use openehr_am::am24::aom2::constraint_model::c_complex_object::{
    CComplexObject, CComplexObjectData,
};
use openehr_am::am24::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
use openehr_am::am24::aom2::rules::expr_archetype_id_constraint::ExprArchetypeIdConstraint;
use openehr_am::am24::aom2::rules::expr_archetype_ref::ExprArchetypeRef;
use openehr_am::am24::aom2::rules::expr_constraint::{ExprConstraint, ExprConstraintData};
use openehr_am::am24::beom::core::assertion::Assertion;
use openehr_am::am24::beom::core::assignment::Assignment;
use openehr_am::am24::beom::core::expr_binary_operator::ExprBinaryOperator;
use openehr_am::am24::beom::core::expr_for_all::ExprForAll;
use openehr_am::am24::beom::core::expr_function_call::ExprFunctionCall;
use openehr_am::am24::beom::core::expr_unary_operator::ExprUnaryOperator;
use openehr_am::am24::beom::core::expr_value::ExprValue;
use openehr_am::am24::beom::core::expr_value_ref::{ExprValueRef, ExprValueRefData};
use openehr_am::am24::beom::core::expression::Expression;
use openehr_am::am24::beom::core::statement::Statement;
use openehr_am::am24::beom::core::statement_set::StatementSet;
use openehr_lang::bel::{BelBuilder, BelError, BelLiteral, parse_statements_with};
use openehr_lang::prelude::{ExprLiteral, ExprVariableRef, OperatorKind, VariableDeclaration};
use openehr_lang::prelude::{ExprTypeDef, TypeDefObjectRef};

use crate::cadl::{parse_contained_regexp_text, parse_inline_primitive_text};
use crate::error::{SyntaxError, SyntaxErrorCode};
use crate::source::SourceArtefact;

/// Which AOM leaf a `matches { … }` right-hand side yields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConstraintMode {
    /// `rules` section: a `matches { c_primitive_object }` → `EXPR_CONSTRAINT`.
    Rule,
    /// `ARCHETYPE_SLOT` include/exclude: a `matches { /regex/ }` →
    /// `EXPR_ARCHETYPE_ID_CONSTRAINT` (`master04.3` §Archetype Slots).
    Slot,
}

/// The AOM [`BelBuilder`]: drives `openehr_lang::bel` to construct the AM-level
/// expression model with the AOM leaf productions spliced in.
struct AmBuilder {
    mode: ConstraintMode,
    /// cADL primitive-constraint parse errors (with their real `S*` codes),
    /// collected so they survive the [`BelError`] boundary.
    errors: Vec<SyntaxError>,
}

impl AmBuilder {
    fn new(mode: ConstraintMode) -> Self {
        Self {
            mode,
            errors: Vec::new(),
        }
    }
}

/// A parse-time placeholder for `EXPR_ARCHETYPE_REF.item` (the referenced node).
/// TODO: resolve it against the archetype definition (currently a placeholder).
///
/// NOTE: `EXPR_ARCHETYPE_REF.item : ARCHETYPE_CONSTRAINT` is mandatory in the
/// generated AOM model but is the *resolved* target of the path, unknown at
/// parse time (`AOM2` master05 — the path is the runtime-value proxy). We emit
/// an empty `C_COMPLEX_OBJECT` as the unresolved target; path resolution (a
/// later ADL2 phase) replaces it. No openEHR spec governs this parse-time
/// placeholder — our own design/extension.
fn unresolved_ref_target() -> ArchetypeConstraint {
    ArchetypeConstraint::CComplexObject(Box::new(CComplexObject::CComplexObject(
        CComplexObjectData {
            parent: None,
            soc_parent: None,
            rm_type_name: String::new(),
            occurrences: None,
            node_id: String::new(),
            alternative_ids: Vec::new(),
            is_deprecated: None,
            sibling_order: None,
            default_value: None,
            attributes: Vec::new(),
            attribute_tuples: Vec::new(),
        },
    )))
}

/// True if a captured `matches` right-hand side is a delimited regex
/// (`{ /re/ }` / `{ ^re^ }`) rather than a primitive-value constraint.
fn is_contained_regexp(raw: &str) -> bool {
    let after_brace = raw.trim_start().trim_start_matches('{').trim_start();
    after_brace.starts_with('/') || after_brace.starts_with('^')
}

/// An object-reference [`ExprTypeDef`] naming `name` (a parser does no value
/// anchoring; the declared/undeclared type carries a JSON-null anchor).
fn object_ref_type(name: &str) -> ExprTypeDef {
    ExprTypeDef::TypeDefObjectRef(TypeDefObjectRef {
        type_name: name.to_owned(),
        type_anchor: serde_json::Value::Null,
    })
}

impl BelBuilder for AmBuilder {
    type Expr = Expression;
    type Stmt = Statement;

    fn literal(&mut self, lit: BelLiteral) -> Expression {
        Expression::ExprLiteral(ExprLiteral {
            item: lit.to_json(),
        })
    }

    fn variable_ref(&mut self, name: &str) -> Expression {
        Expression::ExprVariableRef(ExprVariableRef {
            item: VariableDeclaration {
                name: name.to_owned(),
                r#type: object_ref_type(name),
            },
        })
    }

    fn path_ref(&mut self, path: &str) -> Result<Expression, BelError> {
        // `master05`: an archetype/data path is an EXPR_ARCHETYPE_REF proxy.
        Ok(Expression::ExprValueRef(ExprValueRef::ExprArchetypeRef(
            ExprArchetypeRef {
                path: path.to_owned(),
                item: unresolved_ref_target(),
            },
        )))
    }

    fn function_call(&mut self, name: &str, args: Vec<Expression>) -> Expression {
        Expression::ExprFunctionCall(ExprFunctionCall {
            item: Some(serde_json::Value::String(name.to_owned())),
            arguments: args,
        })
    }

    fn binary(
        &mut self,
        op: OperatorKind,
        symbol: &str,
        left: Expression,
        right: Expression,
    ) -> Expression {
        Expression::ExprBinaryOperator(Box::new(ExprBinaryOperator {
            precedence_overridden: None,
            operator: op,
            symbol: Some(symbol.to_owned()),
            left_operand: Box::new(left),
            right_operand: Box::new(right),
        }))
    }

    fn unary(&mut self, op: OperatorKind, symbol: &str, operand: Expression) -> Expression {
        Expression::ExprUnaryOperator(Box::new(ExprUnaryOperator {
            precedence_overridden: None,
            operator: op,
            symbol: Some(symbol.to_owned()),
            operand: Box::new(operand),
        }))
    }

    fn constraint(&mut self, raw: &str, at: usize) -> Result<Expression, BelError> {
        let inner = if is_contained_regexp(raw) {
            let cstring = self.record(parse_contained_regexp_text(raw), at)?;
            match self.mode {
                ConstraintMode::Slot => {
                    ExprConstraint::ExprArchetypeIdConstraint(ExprArchetypeIdConstraint {
                        item: cstring,
                    })
                }
                ConstraintMode::Rule => ExprConstraint::ExprConstraint(ExprConstraintData {
                    item: CPrimitiveObject::CString(cstring),
                }),
            }
        } else {
            let prim = self.record(parse_inline_primitive_text(raw), at)?;
            ExprConstraint::ExprConstraint(ExprConstraintData { item: prim })
        };
        Ok(Expression::ExprConstraint(inner))
    }

    fn for_all(
        &mut self,
        _variable: &str,
        collection: Expression,
        condition: Expression,
    ) -> Result<Expression, BelError> {
        let operand = match collection {
            Expression::ExprValueRef(r) => r,
            _ => ExprValueRef::ExprValueRef(ExprValueRefData { item: None }),
        };
        Ok(Expression::ExprForAll(Box::new(ExprForAll {
            precedence_overridden: None,
            operator: OperatorKind("for_all".to_owned()),
            symbol: Some("for_all".to_owned()),
            condition: Box::new(Assertion {
                tag: None,
                string_expression: None,
                expression: Box::new(condition),
            }),
            operand,
        })))
    }

    fn assertion(&mut self, tag: Option<String>, expr: Expression) -> Statement {
        Statement::Assertion(Assertion {
            tag,
            string_expression: None,
            expression: Box::new(expr),
        })
    }

    fn assignment(&mut self, target: &str, source: Expression) -> Statement {
        Statement::Assignment(Assignment {
            target: VariableDeclaration {
                name: target.to_owned(),
                r#type: object_ref_type(target),
            },
            source: to_expr_value(source),
        })
    }

    fn variable_declaration(
        &mut self,
        name: &str,
        type_id: &str,
        init: Option<Expression>,
    ) -> Statement {
        drop(init); // no initialiser slot on VARIABLE_DECLARATION (see beom).
        Statement::VariableDeclaration(VariableDeclaration {
            name: name.to_owned(),
            r#type: object_ref_type(type_id),
        })
    }
}

impl AmBuilder {
    /// Record cADL constraint-parse errors (offset by `at`) and turn the failure
    /// into a sentinel [`BelError`]; the caller reads [`AmBuilder::errors`].
    fn record<T>(&mut self, result: Result<T, Vec<SyntaxError>>, at: usize) -> Result<T, BelError> {
        match result {
            Ok(v) => Ok(v),
            Err(errs) => {
                for e in errs {
                    self.errors.push(SyntaxError {
                        code: e.code,
                        message: e.message,
                        line: e.line,
                        column: e.column,
                        span: (e.span.start + at)..(e.span.end + at),
                    });
                }
                Err(BelError::Parse {
                    at,
                    message: "invalid matches-constraint right-hand side".to_owned(),
                })
            }
        }
    }
}

/// Lift an [`Expression`] into the `EXPR_VALUE` union used by assignment sources.
fn to_expr_value(e: Expression) -> ExprValue {
    match e {
        Expression::ExprBinaryOperator(b) => ExprValue::ExprBinaryOperator(*b),
        Expression::ExprConstraint(c) => ExprValue::ExprConstraint(c),
        Expression::ExprForAll(f) => ExprValue::ExprForAll(*f),
        Expression::ExprFunctionCall(f) => ExprValue::ExprFunctionCall(f),
        Expression::ExprLiteral(l) => ExprValue::ExprLiteral(l),
        Expression::ExprUnaryOperator(u) => ExprValue::ExprUnaryOperator(*u),
        Expression::ExprValueRef(r) => ExprValue::ExprValueRef(r),
        Expression::ExprVariableRef(v) => ExprValue::ExprVariableRef(v),
    }
}

/// Map a `openehr_lang::bel` error to the ADL2 syntax catalogue. Rules
/// expressions are AOM invariants: a malformed one is `SINVS`, a missing path
/// after `exists` is `SEXPT` (`master04.6`).
fn bel_to_syntax(err: &BelError, offset: usize, src: &str) -> SyntaxError {
    let (code, at, message) = match err {
        BelError::Lex { at, text } => (
            SyntaxErrorCode::Sinvs,
            *at,
            format!("illegal rule expression: unrecognised input {text:?}"),
        ),
        BelError::Parse { at, message } => {
            let code = if message.contains("exists") {
                SyntaxErrorCode::Sexpt
            } else {
                SyntaxErrorCode::Sinvs
            };
            (code, *at, format!("illegal rule expression: {message}"))
        }
        BelError::Unsupported { at, message } => (SyntaxErrorCode::Sinvs, *at, message.clone()),
    };
    SyntaxError::at(code, message, (at + offset)..(at + offset), src)
}

/// Parse a raw `rules`-section body into the AM-level statement set.
///
/// Byte offsets in returned errors are relative to `body`.
///
/// # Errors
/// Returns the `S*` catalogue errors (`SINVS`/`SEXPT`/`SAIV` and the cADL codes
/// raised while parsing embedded `matches { … }` primitive constraints).
pub fn parse_rules_body(body: &str) -> Result<StatementSet, Vec<SyntaxError>> {
    let mut builder = AmBuilder::new(ConstraintMode::Rule);
    match parse_statements_with(body, &mut builder) {
        Ok(statements) if builder.errors.is_empty() => Ok(StatementSet {
            statement: statements,
            name: None,
        }),
        Ok(_) => Err(builder.errors),
        Err(e) => {
            if builder.errors.is_empty() {
                Err(vec![bel_to_syntax(&e, 0, body)])
            } else {
                Err(builder.errors)
            }
        }
    }
}

/// Parse the `rules` section of a whole ADL2 source, span-offsetting errors and
/// the requested statement set back to the file. Returns `Ok(None)` when the
/// artefact has no `rules` section.
///
/// # Errors
/// Returns the outer-parse errors if `src` does not parse, or the rule
/// (`SINVS`/`SEXPT` + cADL) errors offset to the whole file.
pub fn parse_rules(src: &str) -> Result<Option<StatementSet>, Vec<SyntaxError>> {
    let artefact = crate::source::parse_source(src)?;
    parse_artefact_rules(&artefact, src)
}

/// Parse the `rules` span of an already-parsed [`SourceArtefact`] (mirrors how
/// [`crate::cadl::parse_definition`] re-lexes the definition span).
///
/// # Errors
/// Returns the rule errors offset to the whole file.
pub fn parse_artefact_rules(
    artefact: &SourceArtefact,
    src: &str,
) -> Result<Option<StatementSet>, Vec<SyntaxError>> {
    let Some(rules) = artefact.rules.as_ref() else {
        return Ok(None);
    };
    let body = src.get(rules.bytes.clone()).unwrap_or_default();
    let offset = rules.bytes.start;
    match parse_rules_body(body) {
        Ok(set) => Ok(Some(set)),
        Err(errs) => Err(errs
            .into_iter()
            .map(|e| {
                SyntaxError::at(
                    e.code,
                    e.message,
                    (e.span.start + offset)..(e.span.end + offset),
                    src,
                )
            })
            .collect()),
    }
}

/// Parse one slot include/exclude assertion (`master04.3` §Archetype Slots) into
/// a real AM-level [`Assertion`] — `archetype_id_path matches { /regex/ }` →
/// `EXPR_ARCHETYPE_REF matches EXPR_ARCHETYPE_ID_CONSTRAINT` (`master05`). The
/// verbatim `text` is preserved in `string_expression` so the slot stays usable
/// even for a form that is not structurally modelled.
///
/// # Errors
/// Returns the `S*` errors on a malformed slot assertion (regex compile `SCSRE`,
/// or `SINVS`/`SEXPT` for the expression shape).
pub fn parse_slot_assertion(text: &str) -> Result<Assertion, Vec<SyntaxError>> {
    let mut builder = AmBuilder::new(ConstraintMode::Slot);
    let stmts = match parse_statements_with(text, &mut builder) {
        Ok(stmts) if builder.errors.is_empty() => stmts,
        Ok(_) => return Err(builder.errors),
        Err(e) => {
            return Err(if builder.errors.is_empty() {
                vec![bel_to_syntax(&e, 0, text)]
            } else {
                builder.errors
            });
        }
    };
    let Some(Statement::Assertion(mut assertion)) = stmts.into_iter().next() else {
        return Err(vec![SyntaxError::at(
            SyntaxErrorCode::Sccog,
            "expecting an assertion after 'include'/'exclude'",
            0..text.len(),
            text,
        )]);
    };
    assertion.string_expression = Some(text.trim().to_owned());
    Ok(assertion)
}
