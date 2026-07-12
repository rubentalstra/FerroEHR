//! WHERE-clause lowering (QUERY master03 §WHERE, §Operators, §Logical
//! operators), the whitelisted scalar functions (§Functions), and the node/
//! source predicate conditions (§Standard predicate, §Node predicate).
//!
//! No openEHR spec governs the SQL shapes — this is our own design
//! (`docs/design/aql-engine.md`). Every operator maps to a typed `sea-query`
//! binary op; string functions extract as text and numeric functions through the
//! magnitude coercion, following each function's declared signature.

use sea_query::{Expr, ExprTrait as _};

use crate::aql::error::{AqlError, SqlError};
use crate::aql::ir::{
    ArchetypeConstraint, Bind, Coercion, Expr as IrExpr, LikePattern, NameConstraint,
    NodeConstraint, Operand, ScalarFn, StdPredicate, TypedLit,
};

use super::expr::{aql_like_to_sql, archetype_predicate, as_text, binoper, cast, col, jsonb_path};
use super::value::{coerce_rhs, jsonpath};
use super::{Builder, ValueMode};

impl Builder<'_> {
    // ── WHERE ──────────────────────────────────────────────────────────────────

    pub(super) fn build_where(&mut self) -> Result<(), AqlError> {
        let Some(filter) = self.ir.filter.clone() else {
            return Ok(());
        };
        let cond = self.where_expr(&filter)?;
        self.q.and_where(cond);
        Ok(())
    }

    pub(super) fn where_expr(&mut self, expr: &IrExpr) -> Result<Expr, AqlError> {
        match expr {
            IrExpr::And(a, b) => Ok(self.where_expr(a)?.and(self.where_expr(b)?)),
            IrExpr::Or(a, b) => Ok(self.where_expr(a)?.or(self.where_expr(b)?)),
            IrExpr::Not(a) => Ok(self.where_expr(a)?.not()),
            IrExpr::Compare {
                lhs,
                op,
                rhs,
                coercion,
            } => {
                // G-12: a mixed-type (`Raw`) leaf compared to a numeric literal
                // is compared numerically (with a NULL-guard); otherwise the Raw
                // set falls through to text, exactly as every other Text leaf
                // (QUERY master03 §Comparison operators).
                let (l, r) = if *coercion == Coercion::Raw && raw_numeric_wanted(&[lhs, rhs]) {
                    (
                        self.operand_value_raw_numeric(lhs)?,
                        self.operand_value_raw_numeric(rhs)?,
                    )
                } else {
                    (
                        self.operand_value(lhs, *coercion)?,
                        self.operand_value(rhs, *coercion)?,
                    )
                };
                Ok(l.binary(binoper(*op), r))
            }
            IrExpr::Exists(target) => Ok(self
                .value_expr(target, ValueMode::Projection)?
                .is_not_null()),
            IrExpr::Like { path, pattern } => {
                let lhs = self.value_expr(path, ValueMode::Value(Coercion::Text))?;
                let pat = match pattern {
                    LikePattern::Literal(s) => aql_like_to_sql(s),
                    LikePattern::Param(p) => aql_like_to_sql(&self.param_str(p)?),
                };
                Ok(lhs.like(pat))
            }
            IrExpr::Const(b) => Ok(Expr::val(*b)),
            IrExpr::Matches {
                path,
                values,
                coercion,
            } => {
                // G-12 (matches item 1, QUERY master03 §matches): a mixed-type
                // leaf matched against numeric literals compares numerically.
                let numeric = *coercion == Coercion::Raw
                    && values.iter().any(|b| {
                        matches!(b, Bind::Literal(TypedLit::Integer(_) | TypedLit::Real(_)))
                    });
                let lhs = if numeric {
                    self.value_expr(path, ValueMode::RawNumeric)?
                } else {
                    self.value_expr(path, ValueMode::Value(*coercion))?
                };
                let mut members = Vec::with_capacity(values.len());
                for b in values {
                    let v = self.bind_value(b)?;
                    members.push(if numeric {
                        cast(Expr::val(v), "numeric")
                    } else {
                        coerce_rhs(v, *coercion)
                    });
                }
                Ok(lhs.is_in(members))
            }
        }
    }

    /// Render a whitelisted scalar function call (QUERY master03 §Functions) to
    /// `PostgreSQL`. Arity was validated at lowering; argument typing follows each
    /// function's declared signature (string args extract as text, numeric args
    /// through the magnitude coercion).
    pub(super) fn scalar_fn_expr(
        &mut self,
        func: ScalarFn,
        args: &[Operand],
    ) -> Result<Expr, AqlError> {
        let text = |this: &mut Self, i: usize| this.operand_value(&args[i], Coercion::Text);
        let num = |this: &mut Self, i: usize| this.operand_value(&args[i], Coercion::Magnitude);
        Ok(match func {
            ScalarFn::Length => Expr::cust_with_exprs("length($1)", [text(self, 0)?]),
            // SUBSTRING(expression, position[, length]) — 1-based positions,
            // omitted length extracts to end-of-string (PG substr matches).
            ScalarFn::Substring => match args.len() {
                2 => {
                    Expr::cust_with_exprs("substr($1, ($2)::int4)", [text(self, 0)?, num(self, 1)?])
                }
                _ => Expr::cust_with_exprs(
                    "substr($1, ($2)::int4, ($3)::int4)",
                    [text(self, 0)?, num(self, 1)?, num(self, 2)?],
                ),
            },
            // POSITION(substring, expression): 1-based index of the first
            // occurrence, 0 when absent — exactly PG strpos(expression, sub).
            ScalarFn::Position => {
                Expr::cust_with_exprs("strpos($2, $1)", [text(self, 0)?, text(self, 1)?])
            }
            // The string function CONTAINS(expression, substring) → Boolean.
            ScalarFn::StrContains => {
                Expr::cust_with_exprs("(strpos($1, $2) > 0)", [text(self, 0)?, text(self, 1)?])
            }
            ScalarFn::Concat | ScalarFn::ConcatWs => {
                let name = if func == ScalarFn::Concat {
                    "concat"
                } else {
                    "concat_ws"
                };
                let rendered = (0..args.len())
                    .map(|i| text(self, i))
                    .collect::<Result<Vec<_>, _>>()?;
                let placeholders = (1..=rendered.len())
                    .map(|n| format!("${n}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                Expr::cust_with_exprs(format!("{name}({placeholders})"), rendered)
            }
            ScalarFn::Abs => Expr::cust_with_exprs("abs($1)", [num(self, 0)?]),
            ScalarFn::Mod => Expr::cust_with_exprs(
                "mod(($1)::numeric, ($2)::numeric)",
                [num(self, 0)?, num(self, 1)?],
            ),
            // CEIL/FLOOR return Integer (QUERY master03 §Numeric functions).
            ScalarFn::Ceil => Expr::cust_with_exprs("(ceil($1))::int8", [num(self, 0)?]),
            ScalarFn::Floor => Expr::cust_with_exprs("(floor($1))::int8", [num(self, 0)?]),
            // ROUND(expression[, decimal]) — decimal defaults to 0.
            ScalarFn::Round => match args.len() {
                1 => Expr::cust_with_exprs("round(($1)::numeric, 0)", [num(self, 0)?]),
                _ => Expr::cust_with_exprs(
                    "round(($1)::numeric, ($2)::int4)",
                    [num(self, 0)?, num(self, 1)?],
                ),
            },
            // Date/time functions: the exact string formats of QUERY master03
            // §Date and time functions.
            ScalarFn::CurrentDate => Expr::cust("to_char(now(), 'YYYY-MM-DD')"),
            ScalarFn::CurrentTime => Expr::cust("to_char(now(), 'HH24:MI:SS')"),
            ScalarFn::CurrentDateTime | ScalarFn::Now => {
                Expr::cust("to_char(now(), 'YYYY-MM-DD\"T\"HH24:MI:SS.MSTZH:TZM')")
            }
            ScalarFn::CurrentTimezone => Expr::cust("to_char(now(), 'TZH:TZM')"),
        })
    }

    pub(super) fn operand_value(
        &mut self,
        op: &Operand,
        coercion: Coercion,
    ) -> Result<Expr, AqlError> {
        match op {
            Operand::Path(t) => self.value_expr(t, ValueMode::Value(coercion)),
            Operand::Literal(lit) => Ok(coerce_rhs(super::expr::literal_value(lit), coercion)),
            Operand::Param(p) => Ok(coerce_rhs(self.param_value(p)?, coercion)),
            Operand::Function { func, args } => self.scalar_fn_expr(*func, args),
        }
    }

    /// Lower a comparison operand for the numeric branch of a mixed-type (`Raw`)
    /// comparison (G-12): the path side extracts through the NULL-guarded numeric
    /// mode, literals/params cast to `numeric` (QUERY master03 §Comparison operators).
    fn operand_value_raw_numeric(&mut self, op: &Operand) -> Result<Expr, AqlError> {
        match op {
            Operand::Path(t) => self.value_expr(t, ValueMode::RawNumeric),
            Operand::Literal(lit) => {
                Ok(cast(Expr::val(super::expr::literal_value(lit)), "numeric"))
            }
            Operand::Param(p) => Ok(cast(Expr::val(self.param_value(p)?), "numeric")),
            Operand::Function { func, args } => self.scalar_fn_expr(*func, args),
        }
    }

    // ── source / node predicates ───────────────────────────────────────────────

    pub(super) fn archetype_cond(
        &self,
        node: &str,
        a: &ArchetypeConstraint,
    ) -> Result<Expr, AqlError> {
        let value = match a {
            ArchetypeConstraint::NodeCode(c) | ArchetypeConstraint::Archetype(c) => c.clone(),
            ArchetypeConstraint::Param(p) => self.param_str(p)?,
        };
        Ok(archetype_predicate(node, &value))
    }

    pub(super) fn name_cond(&self, node: &str, n: &NameConstraint) -> Result<Expr, AqlError> {
        match n {
            NameConstraint::Value(s) => Ok(col(node, "name").eq(Expr::val(s.clone()))),
            NameConstraint::Param(p) => Ok(col(node, "name").eq(Expr::val(self.param_str(p)?))),
            NameConstraint::TermCode(c) => {
                let extract = as_text(jsonb_path(
                    col(node, "data"),
                    "$.name.defining_code.code_string",
                ));
                Ok(extract.eq(Expr::val(c.clone())))
            }
        }
    }

    pub(super) fn std_cond(&self, node: &str, sp: &StdPredicate) -> Result<Expr, AqlError> {
        let jp = jsonpath(&sp.path);
        let lhs = as_text(jsonb_path(col(node, "data"), &jp));
        let rhs = cast(Expr::val(self.bind_value(&sp.value)?), "text");
        Ok(lhs.binary(binoper(sp.op), rhs))
    }

    pub(super) fn node_constraint_conds(
        &self,
        node: &str,
        c: &NodeConstraint,
    ) -> Result<Vec<Expr>, AqlError> {
        let mut conds = Vec::new();
        if let Some(a) = &c.archetype {
            conds.push(self.archetype_cond(node, a)?);
        }
        if let Some(n) = &c.name {
            conds.push(self.name_cond(node, n)?);
        }
        for sp in &c.standard {
            conds.push(self.std_cond(node, sp)?);
        }
        Ok(conds)
    }
}

/// Whether a `Raw` (mixed-type) comparison should be evaluated numerically: true
/// when a comparison partner is a numeric literal — "numeric for numbers, text
/// otherwise" (QUERY master03 §Comparison operators). Parameters are untyped at
/// plan time and keep the text path.
fn raw_numeric_wanted(operands: &[&Operand]) -> bool {
    operands.iter().any(|o| {
        matches!(
            o,
            Operand::Literal(TypedLit::Integer(_) | TypedLit::Real(_))
        )
    })
}
