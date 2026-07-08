//! IR → SQL lowering (ADR-008, P16; mapping table in `docs/design/aql-engine.md`
//! §SQL mapping). Turns a typed [`QueryIr`] into one `SELECT` over the greenfield
//! `node`/`vo_version`/`ehr`/`audit` store, built entirely with `sea-query`'s
//! **typed** expression API + `sea-query-sqlx` — no string-concatenated SQL
//! (`.claude/rules/sqlx-conventions.md`). Every table/column reference is an
//! [`Expr::col`], every literal binds through [`Expr::val`] (parameterized on
//! build), and the PostgreSQL-specific pieces use the sanctioned typed
//! escape hatches: [`Func::cust`] for the functions sea-query does not model
//! (`jsonb_path_query_first` / `to_jsonb` / `upper_inf` / `openehr_magnitude`),
//! the typed Postgres operators from [`sea_query::extension::postgres::PgExpr`]
//! (`@>` = `contains`, `||` = `concatenate`), the built-in [`Func`] aggregates
//! (`count`/`count_distinct`/`min`/`max`/`sum`/`avg`), and
//! [`sea_query::ExprTrait::cast_as`] for casts. Only the jsonb-scalar-as-text
//! operator `#>> '{}'` (which sea-query models no typed variant for) uses the
//! documented [`sea_query::BinOper::Custom`] escape hatch. All runtime functions
//! resolve unqualified because the pool's `search_path` is `ehr, ext, public`.
//!
//! ## Strategy
//!
//! The FROM containment tree becomes a **cross join of table aliases + typed
//! WHERE conditions** (the planner folds cross-join+filter into joins). Each RM
//! source that roots a versioned object gets a `node` + `vo_version` (+ `audit`)
//! alias; content sources contained within it share the `vo_version` and
//! interval-join (`num BETWEEN a.num AND a.num_cap`, same `(vo_id, sys_version)`).
//! `EHR` sources join VO roots via `ehr_id`; `VERSION` sources share the
//! contained VO's `vo_version`; `NOT CONTAINS` is a correlated `NOT EXISTS`.
//!
//! **Identified-path extraction** (the design's path split): a leaf whose anchor
//! is empty reads its `data` fragment off the source node alias; a leaf with
//! structure hops reads through a **correlated scalar subquery** walking the
//! anchor chain (interval containment + promoted-column filters per step),
//! extracting the fragment with `jsonb_path_query_first` and coercing per the
//! resolved [`Coercion`]. Subqueries return the value or `NULL`, so missing
//! paths compare false and never multiply rows — keeping `OR`/`NOT`/`EXISTS`
//! correct.

use std::collections::HashMap;
use std::fmt::Write as _;

use sea_query::extension::postgres::PgExpr as _;
use sea_query::{
    Alias, BinOper, Expr, ExprTrait as _, Func, Order, PostgresQueryBuilder, Query,
    SelectStatement, Value,
};
use sea_query_sqlx::{SqlxBinder, SqlxValues};
use uuid::Uuid;

use crate::db::iden::{Audit, Ehr, Node, VoVersion};

use super::error::{AqlError, SqlError};
use super::ir::{
    AggFunc, ArchetypeConstraint, Bind, Coercion, Contained, ContainsTree, EhrField, EhrPredicate,
    Expr as IrExpr, LeafPath, LikePattern, Link, NameConstraint, NodeConstraint, Operand, OrderKey,
    ParamValue, Params, PathTarget, QueryIr, RmSource, SelectColumn, SelectValue, Source,
    StdPredicate, TypeSet, TypedLit, VersionField, VersionScope,
};
use openehr_query::lexer::CompOp;

/// Static execution context the SQL generator needs beyond the IR: the CDR
/// system id (for `OBJECT_VERSION_ID` synthesis), the optional REST `ehr_id`
/// scope, and the composed effective paging window.
#[derive(Debug, Clone)]
pub struct SqlCtx {
    /// The openEHR system id stamped into synthesized `OBJECT_VERSION_ID`s.
    pub system_id: String,
    /// The REST `ehr_id` query parameter, constraining the query to one EHR.
    pub ehr_id: Option<Uuid>,
    /// The ABAC patient-scope subject id (`docs/enterprise/access-control.md`
    /// §6.4): when set, every VO root is restricted to EHRs whose
    /// `subject_id` equals it, so rows the caller may not see are never fetched —
    /// independent of what the query projects.
    pub subject_scope: Option<String>,
    /// The effective row limit (AQL `LIMIT`/`TOP` or REST `fetch`, pre-composed).
    pub limit: Option<i64>,
    /// The effective row offset (AQL `OFFSET` or REST `offset`, pre-composed).
    pub offset: Option<i64>,
}

/// How one `RESULT_SET` cell is read back from the query rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellKind {
    /// A single SQL column read as one canonical-JSON value.
    Scalar,
    /// A whole structure object: four locator columns (`vo_id`, `sys_version`,
    /// `num`, `num_cap`) the executor reassembles through the node codec.
    WholeObject,
}

/// The read-back plan + `RESULT_SET` metadata for one SELECT column.
#[derive(Debug, Clone)]
pub struct ColumnSpec {
    /// The `RESULT_SET` column name (the `AS` alias, else `#{index}`).
    pub name: String,
    /// The `RESULT_SET` column path (the AQL path expression), if reconstructable.
    pub path: Option<String>,
    /// How the executor reads this column.
    pub kind: CellKind,
    /// The generated SQL column alias(es): one for [`CellKind::Scalar`], four
    /// (`vo`, `sv`, `num`, `cap`) for [`CellKind::WholeObject`].
    pub sql_cols: Vec<String>,
}

/// A built, bindable query: the SQL text, its bound values, and the per-column
/// read-back plan.
pub struct PreparedQuery {
    /// The generated SQL.
    pub sql: String,
    /// The bound parameter values (`sea-query-sqlx` → `sqlx`).
    pub values: SqlxValues,
    /// The `RESULT_SET` column read-back plan, in SELECT order.
    pub columns: Vec<ColumnSpec>,
}

// `SqlxValues` is not `Debug`; project the bound-value count instead so the
// struct still satisfies the workspace `missing_debug_implementations` lint.
impl std::fmt::Debug for PreparedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedQuery")
            .field("sql", &self.sql)
            .field("value_count", &self.values.0.0.len())
            .field("columns", &self.columns)
            .finish()
    }
}

/// Lower a planned [`QueryIr`] to SQL.
///
/// # Errors
///
/// [`SqlError`] for a planner-accepted construct the SQL generator does not
/// render yet (e.g. `OR` in CONTAINS).
pub fn build(ir: &QueryIr, params: &Params, ctx: &SqlCtx) -> Result<PreparedQuery, AqlError> {
    let mut b = Builder::new(ir, params, ctx);
    b.build_from(&ir.contains)?;
    b.apply_ehr_scope();
    let columns = b.build_select()?;
    b.build_where()?;
    b.build_order_by()?;
    b.build_paging();
    if ir.distinct {
        b.q.distinct();
    }
    let (sql, values) = b.q.build_sqlx(PostgresQueryBuilder);
    Ok(PreparedQuery {
        sql,
        values,
        columns,
    })
}

/// A built scope-collection query: `SELECT DISTINCT` of every bound VO root's
/// `ehr_id` + `template_id` over the same containment/filter as the main query
/// (§6.4). Its rows carry the set of EHRs and templates the query touches —
/// across **all** bound variables and **independent of the projection** (fixes
/// v1 defect #1) — for the ABAC query post-check.
#[derive(Debug)]
pub struct ScopeQuery {
    /// The generated SQL.
    pub sql: String,
    /// The bound parameter values.
    pub values: SqlxValues,
    /// The `(ehr-id column, template-id column)` alias pairs, one per bound VO
    /// root, the executor reads each result row's ehr/template from.
    pub columns: Vec<(String, String)>,
}

/// Build the ABAC scope-collection query for `ir`. `None` when the query has no
/// VO root (nothing to collect).
///
/// # Errors
/// [`SqlError`] for a construct the SQL generator does not render.
pub fn build_scope(
    ir: &QueryIr,
    params: &Params,
    ctx: &SqlCtx,
) -> Result<Option<ScopeQuery>, AqlError> {
    let mut b = Builder::new(ir, params, ctx);
    b.build_from(&ir.contains)?;
    b.apply_ehr_scope();
    b.build_where()?;
    let roots: Vec<(String, String)> = b
        .group_roots
        .clone()
        .into_iter()
        .zip(b.group_vos.clone())
        .collect();
    if roots.is_empty() {
        return Ok(None);
    }
    // One (ehr_id, template_id) column pair per bound VO root — the row-wise
    // cartesian of the joined roots, DISTINCT-ed, so every touched EHR/template
    // across every bound variable is collected.
    let mut columns = Vec::with_capacity(roots.len());
    for (i, (node, vo)) in roots.iter().enumerate() {
        let ehr_col = format!("scope_ehr_{i}");
        let template_col = format!("scope_template_{i}");
        b.q.expr_as(col(node, "ehr_id"), Alias::new(ehr_col.as_str()));
        b.q.expr_as(col(vo, "template_id"), Alias::new(template_col.as_str()));
        columns.push((ehr_col, template_col));
    }
    b.q.distinct();
    let (sql, values) = b.q.build_sqlx(PostgresQueryBuilder);
    Ok(Some(ScopeQuery {
        sql,
        values,
        columns,
    }))
}

/// A VO "group": the node alias that roots it and the `vo_version` alias its
/// nodes belong to. Content sources contained within it share the `vo` alias and
/// interval-join into `node`.
#[derive(Debug, Clone)]
struct VoGroup {
    node: String,
    vo: String,
}

struct Builder<'a> {
    ir: &'a QueryIr,
    params: &'a Params,
    ctx: &'a SqlCtx,
    q: SelectStatement,
    node_alias: HashMap<usize, String>,
    audit_alias: HashMap<String, String>,
    ehr_alias: HashMap<usize, String>,
    version_vo: HashMap<usize, String>,
    /// Node aliases that root a VO group (targets of the REST `ehr_id` filter).
    group_roots: Vec<String>,
    /// The `vo_version` alias for each entry in `group_roots` (parallel vec) —
    /// the source of the touched `template_id` for the ABAC scope collection.
    group_vos: Vec<String>,
    /// Fresh-alias counter for anchor subqueries / anti-joins.
    sub_ctr: usize,
}

impl<'a> Builder<'a> {
    fn new(ir: &'a QueryIr, params: &'a Params, ctx: &'a SqlCtx) -> Self {
        Self {
            ir,
            params,
            ctx,
            q: Query::select(),
            node_alias: HashMap::new(),
            audit_alias: HashMap::new(),
            ehr_alias: HashMap::new(),
            version_vo: HashMap::new(),
            group_roots: Vec::new(),
            group_vos: Vec::new(),
            sub_ctr: 0,
        }
    }

    // ── FROM / containment ────────────────────────────────────────────────────

    fn build_from(&mut self, tree: &ContainsTree) -> Result<(), AqlError> {
        self.walk(tree, None, None)?;
        Ok(())
    }

    fn walk(
        &mut self,
        tree: &ContainsTree,
        ehr: Option<&str>,
        vo: Option<VoGroup>,
    ) -> Result<Option<VoGroup>, AqlError> {
        match tree {
            ContainsTree::And(a, b) => {
                let g = self.walk(a, ehr, vo.clone())?;
                self.walk(b, ehr, vo)?;
                Ok(g)
            }
            ContainsTree::Or(_, _) => {
                Err(SqlError::Unsupported("OR in the CONTAINS/FROM tree".to_owned()).into())
            }
            ContainsTree::Operand { source, contained } => {
                let sid = source.0;
                match &self.ir.sources[sid] {
                    Source::Version(_) => {
                        let child = match contained.as_deref() {
                            Some(Contained {
                                link: Link::Contains,
                                tree,
                            }) => self.walk(tree, ehr, vo)?,
                            Some(_) => {
                                return Err(SqlError::Unsupported(
                                    "VERSION NOT CONTAINS".to_owned(),
                                )
                                .into());
                            }
                            None => None,
                        };
                        if let Some(g) = &child {
                            self.version_vo.insert(sid, g.vo.clone());
                        }
                        Ok(child)
                    }
                    Source::Ehr(e) => {
                        let alias = format!("e{sid}");
                        self.q.from_as(Ehr::Table, Alias::new(alias.as_str()));
                        self.ehr_alias.insert(sid, alias.clone());
                        let preds = e.predicates.clone();
                        for p in &preds {
                            self.push_ehr_predicate(&alias, p)?;
                        }
                        if let Some(c) = contained {
                            self.contained_edge(c, Some(&alias), None)?;
                        }
                        Ok(None)
                    }
                    Source::Rm(r) => {
                        let r = r.clone();
                        let group = self.emit_rm(sid, &r, ehr, vo.as_ref())?;
                        if let Some(c) = contained {
                            self.contained_edge(c, ehr, Some(group.clone()))?;
                        }
                        Ok(Some(group))
                    }
                }
            }
        }
    }

    fn contained_edge(
        &mut self,
        c: &Contained,
        ehr: Option<&str>,
        vo: Option<VoGroup>,
    ) -> Result<(), AqlError> {
        match c.link {
            Link::Contains => {
                self.walk(&c.tree, ehr, vo)?;
                Ok(())
            }
            Link::NotContains => {
                let parent = vo.ok_or_else(|| {
                    SqlError::Unsupported("NOT CONTAINS without a containing object".to_owned())
                })?;
                self.emit_not_contains(&parent, &c.tree)
            }
        }
    }

    fn emit_rm(
        &mut self,
        sid: usize,
        r: &RmSource,
        ehr: Option<&str>,
        vo: Option<&VoGroup>,
    ) -> Result<VoGroup, AqlError> {
        let node = format!("n{sid}");
        self.q.from_as(Node::Table, Alias::new(node.as_str()));
        self.node_alias.insert(sid, node.clone());
        if let Some(cond) = type_cond(&node, &r.rm_type) {
            self.q.and_where(cond);
        }
        if let Some(a) = &r.archetype {
            let cond = self.archetype_cond(&node, a)?;
            self.q.and_where(cond);
        }
        if let Some(n) = &r.name {
            let cond = self.name_cond(&node, n)?;
            self.q.and_where(cond);
        }
        for sp in &r.standard {
            let cond = self.std_cond(&node, sp)?;
            self.q.and_where(cond);
        }

        let is_vo_root =
            !r.rm_type.is_empty() && r.rm_type.names().iter().all(|t| is_vo_root_type(t));

        // A VO root (or a top-level source with no enclosing group) opens its own
        // `vo_version` group; otherwise the source is content sharing the parent
        // group's version and interval-joining into its node subtree.
        if let (false, Some(parent)) = (is_vo_root, vo) {
            self.q
                .and_where(col(&node, "vo_id").eq(col(&parent.node, "vo_id")));
            self.q
                .and_where(col(&node, "sys_version").eq(col(&parent.node, "sys_version")));
            self.q.and_where(
                col(&node, "num").between(col(&parent.node, "num"), col(&parent.node, "num_cap")),
            );
            Ok(VoGroup {
                node,
                vo: parent.vo.clone(),
            })
        } else {
            let voa = format!("v{sid}");
            self.q.from_as(VoVersion::Table, Alias::new(voa.as_str()));
            self.q.and_where(col(&node, "vo_id").eq(col(&voa, "vo_id")));
            self.q
                .and_where(col(&node, "sys_version").eq(col(&voa, "sys_version")));
            self.ensure_audit(&voa);
            self.push_scope(&voa, &r.scope)?;
            self.group_roots.push(node.clone());
            self.group_vos.push(voa.clone());
            if let Some(e) = ehr {
                self.q.and_where(col(&node, "ehr_id").eq(col(e, "id")));
            }
            Ok(VoGroup { node, vo: voa })
        }
    }

    /// `NOT CONTAINS`: a correlated `NOT EXISTS` over the (single) content
    /// operand, interval-anchored inside `parent`'s node subtree.
    fn emit_not_contains(&mut self, parent: &VoGroup, tree: &ContainsTree) -> Result<(), AqlError> {
        let ContainsTree::Operand { source, contained } = tree else {
            return Err(SqlError::Unsupported(
                "NOT CONTAINS with a compound (AND/OR) operand".to_owned(),
            )
            .into());
        };
        if contained.is_some() {
            return Err(SqlError::Unsupported(
                "NOT CONTAINS with a further nested CONTAINS".to_owned(),
            )
            .into());
        }
        let Source::Rm(r) = &self.ir.sources[source.0] else {
            return Err(SqlError::Unsupported(
                "NOT CONTAINS of a non-structure operand".to_owned(),
            )
            .into());
        };
        let r = r.clone();
        let sub_alias = format!("x{}", self.next_ctr());
        let mut sub = Query::select();
        sub.expr(Expr::val(1));
        sub.from_as(Node::Table, Alias::new(sub_alias.as_str()));
        sub.and_where(col(&sub_alias, "vo_id").eq(col(&parent.node, "vo_id")));
        sub.and_where(col(&sub_alias, "sys_version").eq(col(&parent.node, "sys_version")));
        sub.and_where(
            col(&sub_alias, "num").between(col(&parent.node, "num"), col(&parent.node, "num_cap")),
        );
        if let Some(cond) = type_cond(&sub_alias, &r.rm_type) {
            sub.and_where(cond);
        }
        if let Some(a) = &r.archetype {
            let cond = self.archetype_cond(&sub_alias, a)?;
            sub.and_where(cond);
        }
        if let Some(n) = &r.name {
            let cond = self.name_cond(&sub_alias, n)?;
            sub.and_where(cond);
        }
        self.q.and_where(Expr::not_exists(sub));
        Ok(())
    }

    fn apply_ehr_scope(&mut self) {
        if let Some(ehr_id) = self.ctx.ehr_id {
            for root in self.group_roots.clone() {
                self.q.and_where(col(&root, "ehr_id").eq(Expr::val(ehr_id)));
            }
        }
        // ABAC patient scope (§6.4): restrict every VO root to the caller's
        // patient EHRs. Rows outside are never fetched — regardless of the
        // query's projection (the v1 defect-#1 fix).
        if let Some(subject) = self.ctx.subject_scope.clone() {
            for root in self.group_roots.clone() {
                let mut sub = Query::select();
                sub.column(Alias::new("id"))
                    .from(Ehr::Table)
                    .and_where(Expr::col(Alias::new("subject_id")).eq(Expr::val(subject.clone())));
                self.q.and_where(col(&root, "ehr_id").in_subquery(sub));
            }
        }
    }

    fn ensure_audit(&mut self, voa: &str) -> String {
        if let Some(a) = self.audit_alias.get(voa) {
            return a.clone();
        }
        let alias = format!("a_{voa}");
        self.q.from_as(Audit::Table, Alias::new(alias.as_str()));
        self.q.and_where(col(&alias, "id").eq(col(voa, "audit_id")));
        self.audit_alias.insert(voa.to_owned(), alias.clone());
        alias
    }

    // ── source predicates ──────────────────────────────────────────────────────

    fn push_scope(&mut self, voa: &str, scope: &VersionScope) -> Result<(), AqlError> {
        match scope {
            VersionScope::Latest => {
                self.q
                    .and_where(call("upper_inf", vec![col(voa, "sys_period")]));
            }
            VersionScope::All => {}
            VersionScope::Predicate(p) if p.field == VersionField::TimeCommitted => {
                // Version-at-time: the version whose validity contains the instant
                // (`sys_period @> $t` — the typed `PgExpr::contains` operator).
                let value = self.bind_value(&p.value)?;
                self.q.and_where(
                    col(voa, "sys_period").contains(cast(Expr::val(value), "timestamptz")),
                );
            }
            VersionScope::Predicate(p) => {
                let aud = self.ensure_audit(voa);
                let lhs = version_field_expr(voa, &aud, p.field, &self.ctx.system_id);
                let rhs = cast(Expr::val(self.bind_value(&p.value)?), "text");
                self.q.and_where(lhs.binary(binoper(p.op), rhs));
            }
        }
        Ok(())
    }

    fn push_ehr_predicate(&mut self, alias: &str, p: &EhrPredicate) -> Result<(), AqlError> {
        let value = self.bind_value(&p.value)?;
        let (lhs, rhs) = match p.field {
            EhrField::EhrId => (col(alias, "id"), cast(Expr::val(value), "uuid")),
            EhrField::TimeCreated => (
                col(alias, "time_created"),
                cast(Expr::val(value), "timestamptz"),
            ),
            EhrField::SystemId | EhrField::Whole => {
                return Err(SqlError::Unsupported(
                    "EHR system_id / whole-EHR predicate".to_owned(),
                )
                .into());
            }
        };
        self.q.and_where(lhs.binary(binoper(p.op), rhs));
        Ok(())
    }

    fn archetype_cond(&self, node: &str, a: &ArchetypeConstraint) -> Result<Expr, AqlError> {
        let value = match a {
            ArchetypeConstraint::NodeCode(c) | ArchetypeConstraint::Archetype(c) => c.clone(),
            ArchetypeConstraint::Param(p) => self.param_str(p)?,
        };
        Ok(col(node, "archetype").eq(Expr::val(value)))
    }

    fn name_cond(&self, node: &str, n: &NameConstraint) -> Result<Expr, AqlError> {
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

    fn std_cond(&self, node: &str, sp: &StdPredicate) -> Result<Expr, AqlError> {
        let jp = jsonpath(&sp.path);
        let lhs = as_text(jsonb_path(col(node, "data"), &jp));
        let rhs = cast(Expr::val(self.bind_value(&sp.value)?), "text");
        Ok(lhs.binary(binoper(sp.op), rhs))
    }

    fn node_constraint_conds(&self, node: &str, c: &NodeConstraint) -> Result<Vec<Expr>, AqlError> {
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

    // ── SELECT ──────────────────────────────────────────────────────────────────

    fn build_select(&mut self) -> Result<Vec<ColumnSpec>, AqlError> {
        let mut specs = Vec::with_capacity(self.ir.select.len());
        for (i, col) in self.ir.select.clone().iter().enumerate() {
            specs.push(self.emit_select_column(i, col)?);
        }
        Ok(specs)
    }

    fn emit_select_column(&mut self, i: usize, col: &SelectColumn) -> Result<ColumnSpec, AqlError> {
        let name = col.alias.clone().unwrap_or_else(|| format!("#{i}"));
        match &col.value {
            SelectValue::Path(PathTarget::Data(leaf)) if leaf.is_whole_object() => {
                let mut spec = self.emit_whole_object(i, name, leaf)?;
                // Prefer the query's own path text (the CNF goldens compare it
                // verbatim; `"/"` for a bare variable).
                if col.path.is_some() {
                    spec.path.clone_from(&col.path);
                }
                Ok(spec)
            }
            SelectValue::Path(target) => {
                let expr = self.value_expr(target, ValueMode::Projection)?;
                let sql_col = format!("col{i}");
                self.q.expr_as(to_jsonb(expr), Alias::new(sql_col.as_str()));
                Ok(ColumnSpec {
                    name,
                    path: col.path.clone().or_else(|| target_path_string(target)),
                    kind: CellKind::Scalar,
                    sql_cols: vec![sql_col],
                })
            }
            SelectValue::Literal(lit) => {
                let sql_col = format!("col{i}");
                self.q.expr_as(
                    to_jsonb(Expr::val(literal_value(lit))),
                    Alias::new(sql_col.as_str()),
                );
                Ok(ColumnSpec {
                    name,
                    path: None,
                    kind: CellKind::Scalar,
                    sql_cols: vec![sql_col],
                })
            }
            SelectValue::Aggregate {
                func,
                arg,
                distinct,
            } => {
                let inner = match arg {
                    None => None,
                    Some(target) => {
                        let mode = if matches!(func, AggFunc::Count) {
                            ValueMode::Projection
                        } else {
                            ValueMode::Value(Coercion::Magnitude)
                        };
                        Some(self.value_expr(target, mode)?)
                    }
                };
                let agg = aggregate_expr(*func, inner, *distinct);
                let sql_col = format!("col{i}");
                self.q.expr_as(to_jsonb(agg), Alias::new(sql_col.as_str()));
                Ok(ColumnSpec {
                    name,
                    path: None,
                    kind: CellKind::Scalar,
                    sql_cols: vec![sql_col],
                })
            }
            SelectValue::Function { .. } => {
                Err(SqlError::Unsupported("scalar function in SELECT".to_owned()).into())
            }
        }
    }

    fn emit_whole_object(
        &mut self,
        i: usize,
        name: String,
        leaf: &LeafPath,
    ) -> Result<ColumnSpec, AqlError> {
        let anchor = self.whole_object_alias(leaf)?;
        let cols = ["vo", "sv", "num", "cap"];
        let node_cols = ["vo_id", "sys_version", "num", "num_cap"];
        let mut sql_cols = Vec::with_capacity(4);
        for (suffix, ncol) in cols.iter().zip(node_cols) {
            let sql_col = format!("col{i}_{suffix}");
            self.q
                .expr_as(col(&anchor, ncol), Alias::new(sql_col.as_str()));
            sql_cols.push(sql_col);
        }
        Ok(ColumnSpec {
            name,
            path: Some(leaf_path_string(leaf)),
            kind: CellKind::WholeObject,
            sql_cols,
        })
    }

    /// Resolve a whole-object leaf to a node alias present in the FROM. Empty
    /// anchor → the source node; otherwise the anchor chain is joined in and its
    /// final node alias returned.
    fn whole_object_alias(&mut self, leaf: &LeafPath) -> Result<String, AqlError> {
        let src = self.source_node(leaf.source.0)?;
        if leaf.anchor.is_empty() {
            return Ok(src);
        }
        let mut prev = src;
        for step in &leaf.anchor {
            let alias = format!("w{}", self.next_ctr());
            self.q.from_as(Node::Table, Alias::new(alias.as_str()));
            self.q
                .and_where(col(&alias, "vo_id").eq(col(&prev, "vo_id")));
            self.q
                .and_where(col(&alias, "sys_version").eq(col(&prev, "sys_version")));
            self.q
                .and_where(col(&alias, "num").between(col(&prev, "num"), col(&prev, "num_cap")));
            if let Some(cond) = type_cond(&alias, &step.node_types) {
                self.q.and_where(cond);
            }
            if let Some(pred) = &step.predicate {
                for cond in self.node_constraint_conds(&alias, pred)? {
                    self.q.and_where(cond);
                }
            }
            prev = alias;
        }
        Ok(prev)
    }

    // ── WHERE ──────────────────────────────────────────────────────────────────

    fn build_where(&mut self) -> Result<(), AqlError> {
        let Some(filter) = self.ir.filter.clone() else {
            return Ok(());
        };
        let cond = self.where_expr(&filter)?;
        self.q.and_where(cond);
        Ok(())
    }

    fn where_expr(&mut self, expr: &IrExpr) -> Result<Expr, AqlError> {
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
                let l = self.operand_value(lhs, *coercion)?;
                let r = self.operand_value(rhs, *coercion)?;
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
            IrExpr::Matches {
                path,
                values,
                coercion,
            } => {
                let lhs = self.value_expr(path, ValueMode::Value(*coercion))?;
                let mut members = Vec::with_capacity(values.len());
                for b in values {
                    members.push(coerce_rhs(self.bind_value(b)?, *coercion));
                }
                Ok(lhs.is_in(members))
            }
        }
    }

    fn operand_value(&mut self, op: &Operand, coercion: Coercion) -> Result<Expr, AqlError> {
        match op {
            Operand::Path(t) => self.value_expr(t, ValueMode::Value(coercion)),
            Operand::Literal(lit) => Ok(coerce_rhs(literal_value(lit), coercion)),
            Operand::Param(p) => Ok(coerce_rhs(self.param_value(p)?, coercion)),
            Operand::Function { .. } => {
                Err(SqlError::Unsupported("scalar function operand".to_owned()).into())
            }
        }
    }

    // ── ORDER BY / paging ────────────────────────────────────────────────────────

    fn build_order_by(&mut self) -> Result<(), AqlError> {
        for key in self.ir.order_by.clone() {
            let OrderKey { path, ascending } = key;
            let coercion = order_coercion(&path);
            let expr = self.value_expr(&path, ValueMode::Value(coercion))?;
            let order = if ascending { Order::Asc } else { Order::Desc };
            self.q.order_by_expr(expr, order);
        }
        Ok(())
    }

    fn build_paging(&mut self) {
        if let Some(limit) = self.ctx.limit
            && limit >= 0
        {
            self.q.limit(u64::try_from(limit).unwrap_or(u64::MAX));
        }
        if let Some(offset) = self.ctx.offset
            && offset > 0
        {
            self.q.offset(u64::try_from(offset).unwrap_or(0));
        }
    }

    // ── value expressions (the path split) ──────────────────────────────────────

    fn value_expr(&mut self, target: &PathTarget, mode: ValueMode) -> Result<Expr, AqlError> {
        match target {
            PathTarget::Data(leaf) => self.data_leaf_expr(leaf, mode),
            PathTarget::Version { source, field } => {
                let voa = self.version_vo.get(&source.0).cloned().ok_or_else(|| {
                    SqlError::Unsupported("VERSION path without a bound version".to_owned())
                })?;
                let aud = self.ensure_audit(&voa);
                Ok(version_field_expr(&voa, &aud, *field, &self.ctx.system_id))
            }
            PathTarget::Ehr { source, field } => {
                let alias = self.ehr_alias.get(&source.0).cloned().ok_or_else(|| {
                    SqlError::Unsupported("EHR path without a bound EHR".to_owned())
                })?;
                Ok(ehr_field_expr(&alias, *field, &self.ctx.system_id))
            }
        }
    }

    /// The design's path split for a data leaf: empty anchor → read the source
    /// node's `data` fragment inline; non-empty anchor → a correlated scalar
    /// subquery walking the anchor chain and extracting the fragment.
    fn data_leaf_expr(&mut self, leaf: &LeafPath, mode: ValueMode) -> Result<Expr, AqlError> {
        let src = self.source_node(leaf.source.0)?;
        let jp = fragment_jsonpath(leaf);

        if leaf.anchor.is_empty() {
            let base = extract_base(col(&src, "data"), jp.as_deref());
            return Ok(coerce_value(base, mode, leaf));
        }

        let mut sub = Query::select();
        let mut prev = src;
        let mut last = String::new();
        for step in &leaf.anchor {
            let alias = format!("s{}", self.next_ctr());
            sub.from_as(Node::Table, Alias::new(alias.as_str()));
            sub.and_where(col(&alias, "vo_id").eq(col(&prev, "vo_id")));
            sub.and_where(col(&alias, "sys_version").eq(col(&prev, "sys_version")));
            sub.and_where(col(&alias, "num").between(col(&prev, "num"), col(&prev, "num_cap")));
            if let Some(cond) = type_cond(&alias, &step.node_types) {
                sub.and_where(cond);
            }
            if let Some(pred) = &step.predicate {
                for cond in self.node_constraint_conds(&alias, pred)? {
                    sub.and_where(cond);
                }
            }
            prev.clone_from(&alias);
            last = alias;
        }
        let base = extract_base(col(&last, "data"), jp.as_deref());
        sub.expr(coerce_value(base, mode, leaf));
        sub.limit(1);
        Ok(Expr::from(sub))
    }

    // ── helpers ──────────────────────────────────────────────────────────────────

    fn source_node(&self, sid: usize) -> Result<String, AqlError> {
        self.node_alias.get(&sid).cloned().ok_or_else(|| {
            SqlError::Unsupported("data path on a non-node source".to_owned()).into()
        })
    }

    fn next_ctr(&mut self) -> usize {
        let n = self.sub_ctr;
        self.sub_ctr += 1;
        n
    }

    fn bind_value(&self, b: &Bind) -> Result<Value, AqlError> {
        match b {
            Bind::Literal(lit) => Ok(literal_value(lit)),
            Bind::Param(p) => self.param_value(p),
        }
    }

    fn param_value(&self, name: &str) -> Result<Value, AqlError> {
        match self.params.get(name) {
            Some(ParamValue::Int(i)) => Ok(Value::from(*i)),
            Some(ParamValue::Real(r)) => Ok(Value::from(*r)),
            Some(ParamValue::Bool(b)) => Ok(Value::from(*b)),
            Some(ParamValue::Str(s)) => Ok(Value::from(s.clone())),
            Some(ParamValue::Null) => Ok(Value::from(Option::<String>::None)),
            None => Err(SqlError::UnboundParameter(name.to_owned()).into()),
        }
    }

    fn param_str(&self, name: &str) -> Result<String, AqlError> {
        match self.params.get(name) {
            Some(ParamValue::Str(s)) => Ok(s.clone()),
            Some(ParamValue::Int(i)) => Ok(i.to_string()),
            Some(ParamValue::Real(r)) => Ok(r.to_string()),
            Some(ParamValue::Bool(b)) => Ok(b.to_string()),
            Some(ParamValue::Null) | None => {
                Err(SqlError::UnboundParameter(name.to_owned()).into())
            }
        }
    }
}

/// The extraction/coercion mode of a value expression.
#[derive(Debug, Clone, Copy)]
enum ValueMode {
    /// Raw jsonb value for a `RESULT_SET` cell.
    Projection,
    /// A coerced scalar for comparison / ordering / aggregation.
    Value(Coercion),
}

// ── typed sea-query building blocks ───────────────────────────────────────────

fn is_vo_root_type(t: &str) -> bool {
    matches!(t, "COMPOSITION" | "EHR_STATUS" | "EHR_ACCESS" | "FOLDER")
}

/// A typed `"alias"."column"` reference.
fn col(alias: &str, column: &str) -> Expr {
    Expr::col((Alias::new(alias), Alias::new(column)))
}

/// A typed custom-function call `name(args...)`.
fn call(name: &str, args: Vec<Expr>) -> Expr {
    let mut f = Func::cust(Alias::new(name));
    for a in args {
        f = f.arg(a);
    }
    Expr::from(f)
}

/// `to_jsonb(x)` — normalizes any scalar into a canonical-JSON cell.
fn to_jsonb(e: Expr) -> Expr {
    call("to_jsonb", vec![e])
}

/// `jsonb_path_query_first(data, '<jp>'::jsonpath)`.
fn jsonb_path(data: Expr, jp: &str) -> Expr {
    call(
        "jsonb_path_query_first",
        vec![data, cast(Expr::val(jp.to_owned()), "jsonpath")],
    )
}

/// `<jsonb> #>> '{}'` — the scalar's text at the empty path.
fn as_text(e: Expr) -> Expr {
    e.binary(BinOper::Custom("#>>"), cast(Expr::val("{}"), "text[]"))
}

/// A typed cast `<e>::<ty>`.
fn cast(e: Expr, ty: &str) -> Expr {
    e.cast_as(Alias::new(ty))
}

/// The jsonb extraction base: `jsonb_path_query_first(<data>, jp)` when a
/// fragment path is present, else the raw `<data>` expression.
fn extract_base(data: Expr, jp: Option<&str>) -> Expr {
    match jp {
        Some(jp) => jsonb_path(data, jp),
        None => data,
    }
}

fn binoper(op: CompOp) -> BinOper {
    match op {
        CompOp::Eq => BinOper::Equal,
        CompOp::Ne => BinOper::NotEqual,
        CompOp::Lt => BinOper::SmallerThan,
        CompOp::Le => BinOper::SmallerThanOrEqual,
        CompOp::Gt => BinOper::GreaterThan,
        CompOp::Ge => BinOper::GreaterThanOrEqual,
    }
}

fn literal_value(lit: &TypedLit) -> Value {
    match lit {
        TypedLit::Integer(i) => Value::from(*i),
        TypedLit::Real(r) => Value::from(*r),
        TypedLit::Boolean(b) => Value::from(*b),
        TypedLit::String(s) | TypedLit::Temporal(s) => Value::from(s.clone()),
        TypedLit::Null => Value::from(Option::<String>::None),
    }
}

/// Cast a bound right-hand-side value to match the comparison coercion.
fn coerce_rhs(value: Value, coercion: Coercion) -> Expr {
    match coercion {
        Coercion::Magnitude => cast(Expr::val(value), "numeric"),
        Coercion::Boolean => cast(Expr::val(value), "boolean"),
        Coercion::Temporal => cast(Expr::val(value), "timestamptz"),
        Coercion::Text | Coercion::Raw => cast(Expr::val(value), "text"),
    }
}

/// Apply the value coercion to a jsonb extraction base.
fn coerce_value(base: Expr, mode: ValueMode, leaf: &LeafPath) -> Expr {
    match mode {
        ValueMode::Projection => base,
        ValueMode::Value(Coercion::Magnitude) => {
            if leaf.types.names().iter().any(|t| t.starts_with("DV_")) {
                call("openehr_magnitude", vec![base])
            } else {
                cast(as_text(base), "numeric")
            }
        }
        ValueMode::Value(Coercion::Boolean) => cast(as_text(base), "boolean"),
        // PORT NOTE: temporal comparison casts the ISO-8601 leaf text to
        // timestamptz — precise for full timestamps; partial-precision temporals
        // are a documented gap (QUERY §Built-in Types/Dates and Times).
        ValueMode::Value(Coercion::Temporal) => cast(as_text(base), "timestamptz"),
        ValueMode::Value(Coercion::Text | Coercion::Raw) => as_text(base),
    }
}

fn aggregate_expr(func: AggFunc, inner: Option<Expr>, distinct: bool) -> Expr {
    // The built-in `Func` aggregates (not `Func::cust`) render `COUNT/MIN/MAX/
    // SUM/AVG` and `COUNT(*)`/`COUNT(DISTINCT …)` via the typed API.
    match (func, inner) {
        // `COUNT(1)` is identical to `COUNT(*)` in Postgres (the sea-query
        // documented idiom for a count-all).
        (AggFunc::Count, None) => Expr::from(Func::count(Expr::val(1))),
        (AggFunc::Count, Some(e)) if distinct => Expr::from(Func::count_distinct(e)),
        (AggFunc::Count, Some(e)) => Expr::from(Func::count(e)),
        (AggFunc::Min, Some(e)) => Expr::from(Func::min(e)),
        (AggFunc::Max, Some(e)) => Expr::from(Func::max(e)),
        (AggFunc::Sum, Some(e)) => Expr::from(Func::sum(e)),
        (AggFunc::Avg, Some(e)) => Expr::from(Func::avg(e)),
        // MIN/MAX/SUM/AVG always carry an argument (grammar-enforced).
        (_, None) => Expr::val(Option::<i64>::None),
    }
}

/// The typed SQL for a VERSION metadata field, off the `vo_version`/`audit`
/// aliases (the `uid` is synthesized as `vo_id::system_id::sys_version` via the
/// typed `PgExpr::concatenate` `||` operator).
fn version_field_expr(voa: &str, aud: &str, field: VersionField, system_id: &str) -> Expr {
    let concat = |parts: Vec<Expr>| -> Expr {
        parts
            .into_iter()
            .reduce(sea_query::extension::postgres::PgExpr::concatenate)
            .unwrap_or_else(|| Expr::val(""))
    };
    match field {
        VersionField::Uid => concat(vec![
            cast(col(voa, "vo_id"), "text"),
            Expr::val("::"),
            Expr::val(system_id.to_owned()),
            Expr::val("::"),
            cast(col(voa, "sys_version"), "text"),
        ]),
        VersionField::TimeCommitted => col(aud, "time_committed"),
        VersionField::SystemId => col(aud, "system_id"),
        VersionField::ChangeType => col(aud, "change_type"),
        VersionField::Committer => col(aud, "committer"),
        VersionField::Description => col(aud, "description"),
        VersionField::ContributionId => cast(col(voa, "contribution_id"), "text"),
        VersionField::LifecycleState => col(voa, "lifecycle_state"),
    }
}

fn ehr_field_expr(alias: &str, field: EhrField, system_id: &str) -> Expr {
    match field {
        EhrField::EhrId | EhrField::Whole => cast(col(alias, "id"), "text"),
        EhrField::TimeCreated => col(alias, "time_created"),
        EhrField::SystemId => Expr::val(system_id.to_owned()),
    }
}

/// A typed `rm_type IN (...)` condition, or `None` for an unresolved type set.
fn type_cond(node: &str, types: &TypeSet) -> Option<Expr> {
    if types.is_empty() {
        return None;
    }
    let members: Vec<Expr> = types.names().iter().map(|t| Expr::val(t.clone())).collect();
    Some(col(node, "rm_type").is_in(members))
}

/// Build the fragment jsonpath (`$.a.b`) for a leaf, or `None` when the leaf
/// addresses the whole anchor node.
fn fragment_jsonpath(leaf: &LeafPath) -> Option<String> {
    if leaf.fragment.is_empty() {
        return None;
    }
    let mut jp = String::from("$");
    for step in &leaf.fragment {
        let _ = write!(jp, ".{}", step.name);
    }
    Some(jp)
}

/// Build a jsonpath from a relative object path (`[a, b]` → `$.a.b`).
fn jsonpath(parts: &[String]) -> String {
    let mut jp = String::from("$");
    for p in parts {
        let _ = write!(jp, ".{p}");
    }
    jp
}

/// Translate an AQL `LIKE` pattern (`*` any, `?` one) to a SQL `LIKE` pattern,
/// escaping literal `%`/`_`/`\`. QUERY §Operators/LIKE.
fn aql_like_to_sql(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    for ch in pattern.chars() {
        match ch {
            '*' => out.push('%'),
            '?' => out.push('_'),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

/// The coercion an ORDER BY key should use (mirrors the analyzer's leaf typing).
fn order_coercion(target: &PathTarget) -> Coercion {
    match target {
        PathTarget::Data(leaf) => leaf.coercion,
        PathTarget::Version { field, .. } => {
            if *field == VersionField::TimeCommitted {
                Coercion::Temporal
            } else {
                Coercion::Text
            }
        }
        PathTarget::Ehr { field, .. } => {
            if *field == EhrField::TimeCreated {
                Coercion::Temporal
            } else {
                Coercion::Text
            }
        }
    }
}

/// A best-effort AQL path string for a `RESULT_SET` column's `path`.
fn target_path_string(target: &PathTarget) -> Option<String> {
    match target {
        PathTarget::Data(leaf) => Some(leaf_path_string(leaf)),
        PathTarget::Version { .. } | PathTarget::Ehr { .. } => None,
    }
}

fn leaf_path_string(leaf: &LeafPath) -> String {
    let mut s = String::new();
    for step in &leaf.anchor {
        let _ = write!(s, "/{}", step.attribute);
    }
    for step in &leaf.fragment {
        let _ = write!(s, "/{}", step.name);
    }
    if s.is_empty() {
        s.push('/');
    }
    s
}
