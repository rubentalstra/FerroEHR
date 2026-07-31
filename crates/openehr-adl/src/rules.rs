//! The ADL2 `rules` section + structured slot-assertion parser.
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
//!   `EXPR_CONSTRAINT` wrapping the cADL primitive (reusing the cADL primitive
//!   parser, `crate::parse::parse_inline_primitive_text`);
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
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
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

use crate::error::{SyntaxError, SyntaxErrorCode};
use crate::parse::{parse_contained_regexp_text, parse_inline_primitive_text};
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
///
/// NOTE: `EXPR_ARCHETYPE_REF.item : ARCHETYPE_CONSTRAINT` is mandatory in the
/// generated AOM model but is the *resolved* target of the path, unknown at
/// parse time (`AOM2` master05 — the path is the runtime-value proxy). We emit
/// an empty `C_COMPLEX_OBJECT` as the target during the standalone rules parse;
/// [`resolve_archetype_refs`] replaces it with the resolved definition node once
/// the whole archetype is assembled (an unresolvable path keeps this placeholder
/// and surfaces as a VRRLP validation finding). No openEHR spec governs this
/// parse-time placeholder shape — our own design/extension.
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
            operator: OperatorKind::ForAll,
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

    fn constant_declaration(
        &mut self,
        name: &str,
        type_id: &str,
        value: Option<Expression>,
    ) -> Statement {
        // Same beom-normative bound as `variable_declaration`: no constant
        // class exists (`LANG/docs/BEL/master04-expression_object_model.adoc`
        // §Core Package), so the model's one declaration shape carries it and
        // the value is discarded like a variable initialiser. Archetype rules
        // sections do not use constants in practice.
        drop(value);
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

/// Parse the `rules` span of an already-parsed [`SourceArtefact`] (the rules
/// counterpart of driving [`crate::parse::parse_definition_body`] over the
/// definition span).
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

/// Parse a slot include/exclude assertion block (`master04.3` §Archetype Slots;
/// the cADL grammar `c_includes : SYM_INCLUDE assertion+`) into one or more
/// real AM-level [`Assertion`]s — each `archetype_id_path matches { /regex/ }` →
/// `EXPR_ARCHETYPE_REF matches EXPR_ARCHETYPE_ID_CONSTRAINT` (`master05`). The
/// verbatim block `text` is preserved in every assertion's `string_expression`
/// so the slot stays usable even for a form that is not structurally modelled.
///
/// The grammar admits more than one assertion after a single `include`/`exclude`
/// keyword, so every parsed [`Statement::Assertion`] in the block is returned in
/// source order.
///
/// # Errors
/// Returns the `S*` errors on a malformed slot assertion (regex compile `SCSRE`,
/// or `SINVS`/`SEXPT` for the expression shape).
pub fn parse_slot_assertions(text: &str) -> Result<Vec<Assertion>, Vec<SyntaxError>> {
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
    let raw = text.trim().to_owned();
    let assertions: Vec<Assertion> = stmts
        .into_iter()
        .filter_map(|s| match s {
            Statement::Assertion(mut a) => {
                a.string_expression = Some(raw.clone());
                Some(a)
            }
            _ => None,
        })
        .collect();
    if assertions.is_empty() {
        return Err(vec![SyntaxError::at(
            SyntaxErrorCode::Sccog,
            "expecting an assertion after 'include'/'exclude'",
            0..text.len(),
            text,
        )]);
    }
    Ok(assertions)
}

/// Resolve every `EXPR_ARCHETYPE_REF` proxy in a parsed `rules` [`StatementSet`]
/// against the assembled archetype `definition`, replacing the parse-time
/// placeholder `item` (see `unresolved_ref_target`) with the target node the
/// reference path addresses (`AOM2` master05 — the path is the runtime-value
/// proxy, `item` its resolved `ARCHETYPE_CONSTRAINT` target). A path that does
/// not resolve within the archetype keeps the placeholder (the unresolved path
/// is a VRRLP finding in validation, not an assembly error).
pub fn resolve_archetype_refs(rules: &mut StatementSet, definition: &CComplexObject) {
    for stmt in &mut rules.statement {
        match stmt {
            Statement::Assertion(a) => resolve_in_expr(&mut a.expression, definition),
            Statement::Assignment(a) => resolve_in_expr_value(&mut a.source, definition),
            Statement::VariableDeclaration(_) => {}
        }
    }
}

/// Resolve archetype-ref proxies inside an [`Expression`] tree.
fn resolve_in_expr(expr: &mut Expression, definition: &CComplexObject) {
    match expr {
        Expression::ExprBinaryOperator(b) => {
            resolve_in_expr(&mut b.left_operand, definition);
            resolve_in_expr(&mut b.right_operand, definition);
        }
        Expression::ExprUnaryOperator(u) => resolve_in_expr(&mut u.operand, definition),
        Expression::ExprForAll(f) => {
            resolve_in_expr(&mut f.condition.expression, definition);
            resolve_in_value_ref(&mut f.operand, definition);
        }
        Expression::ExprFunctionCall(fc) => {
            for arg in &mut fc.arguments {
                resolve_in_expr(arg, definition);
            }
        }
        Expression::ExprValueRef(r) => resolve_in_value_ref(r, definition),
        Expression::ExprConstraint(_)
        | Expression::ExprLiteral(_)
        | Expression::ExprVariableRef(_) => {}
    }
}

/// Resolve archetype-ref proxies inside an [`ExprValue`] (an assignment source).
fn resolve_in_expr_value(value: &mut ExprValue, definition: &CComplexObject) {
    match value {
        ExprValue::ExprBinaryOperator(b) => {
            resolve_in_expr(&mut b.left_operand, definition);
            resolve_in_expr(&mut b.right_operand, definition);
        }
        ExprValue::ExprUnaryOperator(u) => resolve_in_expr(&mut u.operand, definition),
        ExprValue::ExprForAll(f) => {
            resolve_in_expr(&mut f.condition.expression, definition);
            resolve_in_value_ref(&mut f.operand, definition);
        }
        ExprValue::ExprFunctionCall(fc) => {
            for arg in &mut fc.arguments {
                resolve_in_expr(arg, definition);
            }
        }
        ExprValue::ExprValueRef(r) => resolve_in_value_ref(r, definition),
        ExprValue::ExprConstraint(_)
        | ExprValue::ExprLiteral(_)
        | ExprValue::ExprVariableRef(_)
        | ExprValue::ExternalQuery(_) => {}
    }
}

/// Resolve an [`ExprValueRef`]: if it is an `EXPR_ARCHETYPE_REF` whose path
/// resolves within the archetype, set its `item` to the resolved target node.
fn resolve_in_value_ref(value_ref: &mut ExprValueRef, definition: &CComplexObject) {
    if let ExprValueRef::ExprArchetypeRef(ar) = value_ref
        && let Some(node) = crate::paths::locate(definition, &ar.path)
    {
        ar.item = to_archetype_constraint(node);
    }
}

/// Convert a resolved definition [`CObject`] into the `ARCHETYPE_CONSTRAINT`
/// union used by `EXPR_ARCHETYPE_REF.item` (a total 1:1 variant mapping).
fn to_archetype_constraint(node: &CObject) -> ArchetypeConstraint {
    match node {
        CObject::ArchetypeSlot(x) => ArchetypeConstraint::ArchetypeSlot(Box::new(x.clone())),
        CObject::CComplexObject(x) => ArchetypeConstraint::CComplexObject(Box::new(x.clone())),
        CObject::CComplexObjectProxy(x) => {
            ArchetypeConstraint::CComplexObjectProxy(Box::new(x.clone()))
        }
        CObject::CBoolean(x) => ArchetypeConstraint::CBoolean(Box::new(x.clone())),
        CObject::CInteger(x) => ArchetypeConstraint::CInteger(Box::new(x.clone())),
        CObject::CReal(x) => ArchetypeConstraint::CReal(Box::new(x.clone())),
        CObject::CString(x) => ArchetypeConstraint::CString(Box::new(x.clone())),
        CObject::CTerminologyCode(x) => ArchetypeConstraint::CTerminologyCode(Box::new(x.clone())),
        CObject::CDate(x) => ArchetypeConstraint::CDate(Box::new(x.clone())),
        CObject::CTime(x) => ArchetypeConstraint::CTime(Box::new(x.clone())),
        CObject::CDateTime(x) => ArchetypeConstraint::CDateTime(Box::new(x.clone())),
        CObject::CDuration(x) => ArchetypeConstraint::CDuration(Box::new(x.clone())),
    }
}
