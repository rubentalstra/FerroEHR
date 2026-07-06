//! IR → SQL lowering (ADR-008, P16; mapping table in `docs/design/aql-engine.md`
//! §SQL mapping). Turns a typed [`QueryIr`] into one `SELECT` over the greenfield
//! `node`/`vo_version`/`ehr`/`audit` store, built with `sea-query` +
//! `sea-query-sqlx` (never string-concatenated whole queries; parameters bind as
//! `sea_query::Value`s through [`sea_query::Expr::cust_with_values`]).
//!
//! ## Strategy
//!
//! The FROM containment tree is lowered to a **cross join of table aliases plus
//! WHERE conditions** (semantically identical to inner joins; the planner folds
//! them). Each RM source that roots a versioned object gets a `node` alias +
//! `vo_version` alias (+ an `audit` alias); content sources contained within it
//! share that `vo_version` and interval-join into the group's node
//! (`num BETWEEN a.num AND a.num_cap`, same `(vo_id, sys_version)`). `EHR`
//! sources join VO roots via `ehr_id`; `VERSION` sources share the contained
//! VO's `vo_version` alias. `NOT CONTAINS` is a correlated `NOT EXISTS`.
//!
//! **Identified-path extraction** (the design's path split): a leaf whose anchor
//! is empty reads its `data` fragment directly off the source node alias; a leaf
//! with structure hops reads through a **correlated scalar subquery** that walks
//! the anchor chain (interval containment + promoted-column filters per step) and
//! extracts the fragment with `jsonb_path_query_first` (+ jsonpath item methods /
//! `ext.openehr_magnitude` per the resolved [`Coercion`]). Subqueries return the
//! value or `NULL`, so missing paths compare false and never multiply rows —
//! which keeps `OR`/`NOT`/`EXISTS` correct.

use std::collections::HashMap;
use std::fmt::Write as _;

use sea_query::{Alias, Expr, Order, PostgresQueryBuilder, Query, SelectStatement, Value};
use sea_query_sqlx::{SqlxBinder, SqlxValues};
use uuid::Uuid;

use crate::db::iden::{Audit, Ehr, Node, VoVersion};

use super::error::{AqlError, SqlError};
use super::ir::{
    AggFunc, ArchetypeConstraint, Bind, Coercion, Contained, ContainsTree, EhrField,
    Expr as IrExpr, LeafPath, Link, NameConstraint, NodeConstraint, Operand, OrderKey, ParamValue,
    Params, PathTarget, QueryIr, RmSource, SelectColumn, SelectValue, Source, StdPredicate,
    TypeSet, TypedLit, VersionField, VersionScope,
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

/// Lower a planned [`QueryIr`] to SQL.
///
/// # Errors
///
/// [`SqlError`] for a planner-accepted construct the SQL generator does not
/// render yet (e.g. `OR` in CONTAINS), or a REST/AQL paging conflict.
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
    vo_alias: HashMap<usize, String>,
    audit_alias: HashMap<String, String>,
    ehr_alias: HashMap<usize, String>,
    version_vo: HashMap<usize, String>,
    /// Node aliases that root a VO group (targets of the REST `ehr_id` filter).
    group_roots: Vec<String>,
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
            vo_alias: HashMap::new(),
            audit_alias: HashMap::new(),
            ehr_alias: HashMap::new(),
            version_vo: HashMap::new(),
            group_roots: Vec::new(),
            sub_ctr: 0,
        }
    }

    // ── FROM / containment ────────────────────────────────────────────────────

    fn build_from(&mut self, tree: &ContainsTree) -> Result<(), AqlError> {
        self.walk(tree, None, None)?;
        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
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
                        // A VERSION shares its contained VO's version alias.
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
                        let e = e.clone();
                        for p in &e.predicates {
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
                            let child_ehr = ehr;
                            self.contained_edge(c, child_ehr, Some(group.clone()))?;
                        }
                        Ok(Some(group))
                    }
                }
            }
        }
    }

    /// Handle a `[NOT] CONTAINS` edge below an already-emitted operand.
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
        self.push_type_filter(&node, &r.rm_type);
        if let Some(a) = &r.archetype {
            self.push_archetype(&node, a)?;
        }
        if let Some(n) = &r.name {
            self.push_name(&node, n)?;
        }
        for sp in &r.standard {
            self.push_std_predicate(&node, sp)?;
        }

        let is_vo_root =
            !r.rm_type.is_empty() && r.rm_type.names().iter().all(|t| is_vo_root_type(t));

        if is_vo_root || vo.is_none() {
            let voa = format!("v{sid}");
            self.q.from_as(VoVersion::Table, Alias::new(voa.as_str()));
            self.where_raw(format!(
                r#"{n}."vo_id" = {v}."vo_id" AND {n}."sys_version" = {v}."sys_version""#,
                n = tbl(&node),
                v = tbl(&voa),
            ));
            self.ensure_audit(&voa);
            self.push_scope(&voa, &r.scope)?;
            self.vo_alias.insert(sid, voa.clone());
            self.group_roots.push(node.clone());
            if let Some(e) = ehr {
                self.where_raw(format!(
                    r#"{n}."ehr_id" = {e}."id""#,
                    n = tbl(&node),
                    e = tbl(e)
                ));
            }
            Ok(VoGroup { node, vo: voa })
        } else {
            let parent = vo.expect("checked");
            self.where_raw(format!(
                r#"{n}."vo_id" = {p}."vo_id" AND {n}."sys_version" = {p}."sys_version" AND {n}."num" BETWEEN {p}."num" AND {p}."num_cap""#,
                n = tbl(&node),
                p = tbl(&parent.node)
            ));
            self.vo_alias.insert(sid, parent.vo.clone());
            Ok(VoGroup {
                node,
                vo: parent.vo.clone(),
            })
        }
    }

    /// `NOT CONTAINS`: a correlated `NOT EXISTS` over the (single) content
    /// operand contained, interval-anchored inside `parent`'s node subtree.
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
        let sub = format!("x{}", self.next_ctr());
        let mut frag = Frag::new();
        let mut cond = format!(
            r#"{s}."vo_id" = {p}."vo_id" AND {s}."sys_version" = {p}."sys_version" AND {s}."num" BETWEEN {p}."num" AND {p}."num_cap""#,
            s = tbl(&sub),
            p = tbl(&parent.node)
        );
        cond.push_str(&self.type_filter_sql(&mut frag, &sub, &r.rm_type));
        if let Some(a) = &r.archetype {
            cond.push_str(&self.archetype_sql(&mut frag, &sub, a)?);
        }
        if let Some(n) = &r.name {
            cond.push_str(&self.name_sql(&mut frag, &sub, n)?);
        }
        let sql = format!(
            r#"NOT EXISTS (SELECT 1 FROM "node" AS {s} WHERE {cond})"#,
            s = tbl(&sub)
        );
        self.q.and_where(frag.into_expr(sql));
        Ok(())
    }

    fn apply_ehr_scope(&mut self) {
        let Some(ehr_id) = self.ctx.ehr_id else {
            return;
        };
        let roots = self.group_roots.clone();
        for root in roots {
            self.where_valued(
                format!(r#"{n}."ehr_id" = $1::uuid"#, n = tbl(&root)),
                vec![Value::from(ehr_id)],
            );
        }
    }

    fn ensure_audit(&mut self, voa: &str) -> String {
        if let Some(a) = self.audit_alias.get(voa) {
            return a.clone();
        }
        let alias = format!("a_{voa}");
        self.q.from_as(Audit::Table, Alias::new(alias.as_str()));
        self.where_raw(format!(
            r#"{a}."id" = {v}."audit_id""#,
            a = tbl(&alias),
            v = tbl(voa)
        ));
        self.audit_alias.insert(voa.to_owned(), alias.clone());
        alias
    }

    // ── source predicates ──────────────────────────────────────────────────────

    fn push_scope(&mut self, voa: &str, scope: &VersionScope) -> Result<(), AqlError> {
        match scope {
            VersionScope::Latest => {
                self.where_raw(format!(r#"upper_inf({v}."sys_period")"#, v = tbl(voa)));
            }
            VersionScope::All => {}
            VersionScope::Predicate(p) if p.field == VersionField::TimeCommitted => {
                // Version-at-time: the version whose validity contains the instant.
                let value = self.bind_value(&p.value)?;
                self.where_valued(
                    format!(r#"{v}."sys_period" @> $1::timestamptz"#, v = tbl(voa)),
                    vec![value],
                );
            }
            VersionScope::Predicate(p) => {
                let aud = self.ensure_audit(voa);
                let col = version_field_sql(voa, &aud, p.field, &self.ctx.system_id);
                let value = self.bind_value(&p.value)?;
                self.where_valued(format!("({col}) {op} $1", op = cmp_op(p.op)), vec![value]);
            }
        }
        Ok(())
    }

    fn push_ehr_predicate(
        &mut self,
        alias: &str,
        p: &super::ir::EhrPredicate,
    ) -> Result<(), AqlError> {
        let value = self.bind_value(&p.value)?;
        let (col, cast) = match p.field {
            EhrField::EhrId => (format!(r#"{e}."id""#, e = tbl(alias)), "::uuid"),
            EhrField::TimeCreated => (
                format!(r#"{e}."time_created""#, e = tbl(alias)),
                "::timestamptz",
            ),
            EhrField::SystemId | EhrField::Whole => {
                return Err(SqlError::Unsupported(
                    "EHR system_id / whole-EHR predicate".to_owned(),
                )
                .into());
            }
        };
        self.where_valued(
            format!("{col} {op} $1{cast}", op = cmp_op(p.op)),
            vec![value],
        );
        Ok(())
    }

    fn push_type_filter(&mut self, node: &str, types: &TypeSet) {
        let mut frag = Frag::new();
        let sql = self.type_filter_sql(&mut frag, node, types);
        if sql.is_empty() {
            return;
        }
        // Strip the leading " AND ".
        let sql = sql.trim_start_matches(" AND ").to_owned();
        self.q.and_where(frag.into_expr(sql));
    }

    fn type_filter_sql(&self, frag: &mut Frag, node: &str, types: &TypeSet) -> String {
        if types.is_empty() {
            return String::new();
        }
        let placeholders: Vec<String> = types
            .names()
            .iter()
            .map(|t| frag.ph(Value::from(t.clone())))
            .collect();
        format!(
            r#" AND {n}."rm_type" IN ({ph})"#,
            n = tbl(node),
            ph = placeholders.join(", ")
        )
    }

    fn push_archetype(&mut self, node: &str, a: &ArchetypeConstraint) -> Result<(), AqlError> {
        let mut frag = Frag::new();
        let sql = self.archetype_sql(&mut frag, node, a)?;
        let sql = sql.trim_start_matches(" AND ").to_owned();
        self.q.and_where(frag.into_expr(sql));
        Ok(())
    }

    fn archetype_sql(
        &self,
        frag: &mut Frag,
        node: &str,
        a: &ArchetypeConstraint,
    ) -> Result<String, AqlError> {
        let value = match a {
            ArchetypeConstraint::NodeCode(c) | ArchetypeConstraint::Archetype(c) => c.clone(),
            ArchetypeConstraint::Param(p) => self.param_str(p)?,
        };
        let ph = frag.ph(Value::from(value));
        Ok(format!(r#" AND {n}."archetype" = {ph}"#, n = tbl(node)))
    }

    fn push_name(&mut self, node: &str, n: &NameConstraint) -> Result<(), AqlError> {
        let mut frag = Frag::new();
        let sql = self.name_sql(&mut frag, node, n)?;
        let sql = sql.trim_start_matches(" AND ").to_owned();
        self.q.and_where(frag.into_expr(sql));
        Ok(())
    }

    fn name_sql(
        &self,
        frag: &mut Frag,
        node: &str,
        n: &NameConstraint,
    ) -> Result<String, AqlError> {
        match n {
            NameConstraint::Value(s) => {
                let ph = frag.ph(Value::from(s.clone()));
                Ok(format!(r#" AND {nd}."name" = {ph}"#, nd = tbl(node)))
            }
            NameConstraint::Param(p) => {
                let ph = frag.ph(Value::from(self.param_str(p)?));
                Ok(format!(r#" AND {nd}."name" = {ph}"#, nd = tbl(node)))
            }
            NameConstraint::TermCode(c) => {
                let ph = frag.ph(Value::from(c.clone()));
                Ok(format!(
                    r#" AND (jsonb_path_query_first({nd}."data", '$.name.defining_code.code_string') #>> '{{}}') = {ph}"#,
                    nd = tbl(node)
                ))
            }
        }
    }

    fn push_std_predicate(&mut self, node: &str, sp: &StdPredicate) -> Result<(), AqlError> {
        let jp = jsonpath(&sp.path);
        let mut frag = Frag::new();
        let jp_ph = frag.ph(Value::from(jp));
        let value = self.bind_value(&sp.value)?;
        let v_ph = frag.ph(value);
        let sql = format!(
            r#"(jsonb_path_query_first({n}."data", {jp}::jsonpath) #>> '{{}}') {op} {v}::text"#,
            n = tbl(node),
            jp = jp_ph,
            op = cmp_op(sp.op),
            v = v_ph
        );
        self.q.and_where(frag.into_expr(sql));
        Ok(())
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
                self.emit_whole_object(i, name, leaf)
            }
            SelectValue::Path(target) => {
                let mut frag = Frag::new();
                let expr = self.value_expr(&mut frag, target, ValueMode::Projection)?;
                let sql_col = format!("col{i}");
                self.q.expr_as(
                    frag.into_expr(format!("to_jsonb({expr})")),
                    Alias::new(sql_col.as_str()),
                );
                Ok(ColumnSpec {
                    name,
                    path: target_path_string(target),
                    kind: CellKind::Scalar,
                    sql_cols: vec![sql_col],
                })
            }
            SelectValue::Literal(lit) => {
                let mut frag = Frag::new();
                let ph = frag.ph(literal_value(lit));
                let sql_col = format!("col{i}");
                self.q.expr_as(
                    frag.into_expr(format!("to_jsonb({ph})")),
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
                let mut frag = Frag::new();
                let inner = match arg {
                    None => "*".to_owned(),
                    Some(target) => {
                        let mode = if matches!(func, AggFunc::Count) {
                            ValueMode::Projection
                        } else {
                            ValueMode::Value(Coercion::Magnitude)
                        };
                        self.value_expr(&mut frag, target, mode)?
                    }
                };
                let agg = aggregate_sql(*func, &inner, *distinct);
                let sql_col = format!("col{i}");
                self.q.expr_as(
                    frag.into_expr(format!("to_jsonb({agg})")),
                    Alias::new(sql_col.as_str()),
                );
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
            self.q.expr_as(
                Expr::cust(qcol(&anchor, ncol)),
                Alias::new(sql_col.as_str()),
            );
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
    /// anchor → the source node itself; otherwise the anchor chain is joined in
    /// and its final node alias returned.
    fn whole_object_alias(&mut self, leaf: &LeafPath) -> Result<String, AqlError> {
        let src = self
            .node_alias
            .get(&leaf.source.0)
            .cloned()
            .ok_or_else(|| {
                SqlError::Unsupported("whole-object select of a non-node source".to_owned())
            })?;
        if leaf.anchor.is_empty() {
            return Ok(src);
        }
        // Join the anchor chain into the main query (interval containment).
        let mut prev = src;
        for step in &leaf.anchor {
            let alias = format!("w{}", self.next_ctr());
            self.q.from_as(Node::Table, Alias::new(alias.as_str()));
            self.where_raw(format!(
                r#"{a}."vo_id" = {p}."vo_id" AND {a}."sys_version" = {p}."sys_version" AND {a}."num" BETWEEN {p}."num" AND {p}."num_cap""#,
                a = tbl(&alias),
                p = tbl(&prev)
            ));
            self.push_type_filter(&alias, &step.node_types);
            if let Some(pred) = &step.predicate {
                self.push_node_constraint(&alias, pred)?;
            }
            prev = alias;
        }
        Ok(prev)
    }

    fn push_node_constraint(&mut self, node: &str, c: &NodeConstraint) -> Result<(), AqlError> {
        if let Some(a) = &c.archetype {
            self.push_archetype(node, a)?;
        }
        if let Some(n) = &c.name {
            self.push_name(node, n)?;
        }
        for sp in &c.standard {
            self.push_std_predicate(node, sp)?;
        }
        Ok(())
    }

    // ── WHERE ──────────────────────────────────────────────────────────────────

    fn build_where(&mut self) -> Result<(), AqlError> {
        let Some(filter) = self.ir.filter.clone() else {
            return Ok(());
        };
        let mut frag = Frag::new();
        let sql = self.where_expr(&mut frag, &filter)?;
        self.q.and_where(frag.into_expr(sql));
        Ok(())
    }

    fn where_expr(&mut self, frag: &mut Frag, expr: &IrExpr) -> Result<String, AqlError> {
        match expr {
            IrExpr::And(a, b) => {
                let l = self.where_expr(frag, a)?;
                let r = self.where_expr(frag, b)?;
                Ok(format!("({l} AND {r})"))
            }
            IrExpr::Or(a, b) => {
                let l = self.where_expr(frag, a)?;
                let r = self.where_expr(frag, b)?;
                Ok(format!("({l} OR {r})"))
            }
            IrExpr::Not(a) => {
                let inner = self.where_expr(frag, a)?;
                Ok(format!("(NOT ({inner}))"))
            }
            IrExpr::Compare {
                lhs,
                op,
                rhs,
                coercion,
            } => {
                let l = self.operand_value(frag, lhs, *coercion)?;
                let r = self.operand_value(frag, rhs, *coercion)?;
                Ok(format!("({l} {op} {r})", op = cmp_op(*op)))
            }
            IrExpr::Exists(target) => {
                let expr = self.value_expr(frag, target, ValueMode::Projection)?;
                Ok(format!("(({expr}) IS NOT NULL)"))
            }
            IrExpr::Like { path, pattern } => {
                let lhs = self.value_expr(frag, path, ValueMode::Value(Coercion::Text))?;
                let pat = match pattern {
                    super::ir::LikePattern::Literal(s) => aql_like_to_sql(s),
                    super::ir::LikePattern::Param(p) => aql_like_to_sql(&self.param_str(p)?),
                };
                let ph = frag.ph(Value::from(pat));
                Ok(format!("(({lhs}) LIKE {ph})"))
            }
            IrExpr::Matches {
                path,
                values,
                coercion,
            } => {
                let lhs = self.value_expr(frag, path, ValueMode::Value(*coercion))?;
                let mut members = Vec::with_capacity(values.len());
                for b in values {
                    let v = self.bind_value(b)?;
                    members.push(coerce_rhs(frag, v, *coercion));
                }
                Ok(format!("(({lhs}) IN ({}))", members.join(", ")))
            }
        }
    }

    fn operand_value(
        &mut self,
        frag: &mut Frag,
        op: &Operand,
        coercion: Coercion,
    ) -> Result<String, AqlError> {
        match op {
            Operand::Path(t) => self.value_expr(frag, t, ValueMode::Value(coercion)),
            Operand::Literal(lit) => Ok(coerce_rhs(frag, literal_value(lit), coercion)),
            Operand::Param(p) => {
                let v = self.param_value(p)?;
                Ok(coerce_rhs(frag, v, coercion))
            }
            Operand::Function { .. } => {
                Err(SqlError::Unsupported("scalar function operand".to_owned()).into())
            }
        }
    }

    // ── ORDER BY / paging ────────────────────────────────────────────────────────

    fn build_order_by(&mut self) -> Result<(), AqlError> {
        for key in self.ir.order_by.clone() {
            let OrderKey { path, ascending } = key;
            let mut frag = Frag::new();
            let coercion = order_coercion(&path);
            let expr = self.value_expr(&mut frag, &path, ValueMode::Value(coercion))?;
            let order = if ascending { Order::Asc } else { Order::Desc };
            self.q.order_by_expr(frag.into_expr(expr), order);
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

    fn value_expr(
        &mut self,
        frag: &mut Frag,
        target: &PathTarget,
        mode: ValueMode,
    ) -> Result<String, AqlError> {
        match target {
            PathTarget::Data(leaf) => self.data_leaf_expr(frag, leaf, mode),
            PathTarget::Version { source, field } => {
                let voa = self.version_vo.get(&source.0).cloned().ok_or_else(|| {
                    SqlError::Unsupported("VERSION path without a bound version".to_owned())
                })?;
                let aud = self.ensure_audit(&voa);
                Ok(version_field_sql(&voa, &aud, *field, &self.ctx.system_id))
            }
            PathTarget::Ehr { source, field } => {
                let alias = self.ehr_alias.get(&source.0).cloned().ok_or_else(|| {
                    SqlError::Unsupported("EHR path without a bound EHR".to_owned())
                })?;
                ehr_field_sql(&alias, *field, &self.ctx.system_id)
            }
        }
    }

    /// The design's path split for a data leaf: empty anchor → read the source
    /// node's `data` inline; non-empty anchor → a correlated scalar subquery
    /// walking the anchor chain and extracting the fragment.
    fn data_leaf_expr(
        &mut self,
        frag: &mut Frag,
        leaf: &LeafPath,
        mode: ValueMode,
    ) -> Result<String, AqlError> {
        let src = self
            .node_alias
            .get(&leaf.source.0)
            .cloned()
            .ok_or_else(|| SqlError::Unsupported("data path on a non-node source".to_owned()))?;
        let jp = fragment_jsonpath(leaf);

        if leaf.anchor.is_empty() {
            let base = self.extract_base(frag, &qcol_expr(&src, "data"), jp.as_deref());
            return Ok(coerce_value(&base, mode, leaf));
        }

        // Correlated subquery over the anchor chain.
        let mut aliases = Vec::with_capacity(leaf.anchor.len());
        let mut from_parts = Vec::with_capacity(leaf.anchor.len());
        let mut cond = String::new();
        let mut prev = src.clone();
        for step in &leaf.anchor {
            let alias = format!("s{}", self.next_ctr());
            from_parts.push(format!(r#""node" AS {}"#, tbl(&alias)));
            let _ = write!(
                cond,
                r#"{sep}{a}."vo_id" = {p}."vo_id" AND {a}."sys_version" = {p}."sys_version" AND {a}."num" BETWEEN {p}."num" AND {p}."num_cap""#,
                sep = if cond.is_empty() { "" } else { " AND " },
                a = tbl(&alias),
                p = tbl(&prev)
            );
            cond.push_str(&self.type_filter_sql(frag, &alias, &step.node_types));
            if let Some(pred) = &step.predicate {
                if let Some(a) = &pred.archetype {
                    cond.push_str(&self.archetype_sql(frag, &alias, a)?);
                }
                if let Some(n) = &pred.name {
                    cond.push_str(&self.name_sql(frag, &alias, n)?);
                }
                for sp in &pred.standard {
                    cond.push_str(&self.std_predicate_sql(frag, &alias, sp)?);
                }
            }
            prev = alias.clone();
            aliases.push(alias);
        }
        let last = aliases.last().cloned().unwrap_or(src);
        let base = self.extract_base(frag, &qcol_expr(&last, "data"), jp.as_deref());
        let select_expr = coerce_value(&base, mode, leaf);
        Ok(format!(
            "(SELECT {select_expr} FROM {from} WHERE {cond} LIMIT 1)",
            from = from_parts.join(", ")
        ))
    }

    /// The jsonb extraction base: `jsonb_path_query_first(<data>, $jp)` when a
    /// fragment path is present, else the raw `<data>` expression.
    fn extract_base(&self, frag: &mut Frag, data_expr: &str, jp: Option<&str>) -> String {
        match jp {
            Some(jp) => {
                let ph = frag.ph(Value::from(jp.to_owned()));
                format!("jsonb_path_query_first({data_expr}, {ph}::jsonpath)")
            }
            None => data_expr.to_owned(),
        }
    }

    fn std_predicate_sql(
        &self,
        frag: &mut Frag,
        node: &str,
        sp: &StdPredicate,
    ) -> Result<String, AqlError> {
        let jp = jsonpath(&sp.path);
        let jp_ph = frag.ph(Value::from(jp));
        let value = self.bind_value(&sp.value)?;
        let v_ph = frag.ph(value);
        Ok(format!(
            r#" AND (jsonb_path_query_first({n}."data", {jp}::jsonpath) #>> '{{}}') {op} {v}::text"#,
            n = tbl(node),
            jp = jp_ph,
            op = cmp_op(sp.op),
            v = v_ph
        ))
    }

    // ── helpers ──────────────────────────────────────────────────────────────────

    fn next_ctr(&mut self) -> usize {
        let n = self.sub_ctr;
        self.sub_ctr += 1;
        n
    }

    fn where_raw(&mut self, sql: String) {
        self.q.and_where(Expr::cust(sql));
    }

    fn where_valued(&mut self, sql: String, vals: Vec<Value>) {
        self.q.and_where(Expr::cust_with_values(sql, vals));
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

/// A single custom-SQL fragment under construction: an SQL string with local
/// `$1..$N` placeholders and their bound values (renumbered globally by
/// `sea-query` at build time).
struct Frag {
    vals: Vec<Value>,
}

impl Frag {
    fn new() -> Self {
        Self { vals: Vec::new() }
    }

    /// Bind `v`, returning its local placeholder (`$N`, 1-based into this frag).
    fn ph(&mut self, v: Value) -> String {
        self.vals.push(v);
        format!("${}", self.vals.len())
    }

    fn into_expr(self, sql: String) -> Expr {
        if self.vals.is_empty() {
            Expr::cust(sql)
        } else {
            Expr::cust_with_values(sql, self.vals)
        }
    }
}

// ── free helpers ────────────────────────────────────────────────────────────────

fn is_vo_root_type(t: &str) -> bool {
    matches!(t, "COMPOSITION" | "EHR_STATUS" | "EHR_ACCESS" | "FOLDER")
}

/// A double-quoted alias reference, e.g. `n0` → `"n0"`.
fn tbl(alias: &str) -> String {
    format!("\"{alias}\"")
}

/// A double-quoted `alias.column`, e.g. (`n0`, `data`) → `"n0"."data"`.
fn qcol(alias: &str, col: &str) -> String {
    format!("\"{alias}\".\"{col}\"")
}

/// The same as [`qcol`] as an owned expression string (readability alias).
fn qcol_expr(alias: &str, col: &str) -> String {
    qcol(alias, col)
}

fn cmp_op(op: CompOp) -> &'static str {
    match op {
        CompOp::Eq => "=",
        CompOp::Ne => "<>",
        CompOp::Lt => "<",
        CompOp::Le => "<=",
        CompOp::Gt => ">",
        CompOp::Ge => ">=",
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

/// Cast a right-hand-side bound value to match the comparison coercion.
fn coerce_rhs(frag: &mut Frag, value: Value, coercion: Coercion) -> String {
    let ph = frag.ph(value);
    match coercion {
        Coercion::Magnitude => format!("{ph}::numeric"),
        Coercion::Boolean => format!("{ph}::boolean"),
        Coercion::Text | Coercion::Temporal | Coercion::Raw => format!("{ph}::text"),
    }
}

/// Apply the value coercion to a jsonb extraction base.
fn coerce_value(base: &str, mode: ValueMode, leaf: &LeafPath) -> String {
    match mode {
        ValueMode::Projection => base.to_owned(),
        ValueMode::Value(Coercion::Magnitude) => {
            if leaf.types.names().iter().any(|t| t.starts_with("DV_")) {
                format!("ext.openehr_magnitude({base})")
            } else {
                format!("({base} #>> '{{}}')::numeric")
            }
        }
        ValueMode::Value(Coercion::Boolean) => format!("({base} #>> '{{}}')::boolean"),
        ValueMode::Value(Coercion::Text | Coercion::Temporal | Coercion::Raw) => {
            format!("({base} #>> '{{}}')")
        }
    }
}

fn aggregate_sql(func: AggFunc, inner: &str, distinct: bool) -> String {
    let d = if distinct { "DISTINCT " } else { "" };
    match func {
        AggFunc::Count => format!("count({d}{inner})"),
        AggFunc::Min => format!("min({inner})"),
        AggFunc::Max => format!("max({inner})"),
        AggFunc::Sum => format!("sum({inner})"),
        AggFunc::Avg => format!("avg({inner})"),
    }
}

/// The SQL for a VERSION metadata field, off the `vo_version`/`audit` aliases.
fn version_field_sql(voa: &str, aud: &str, field: VersionField, system_id: &str) -> String {
    match field {
        VersionField::Uid => format!(
            "({vo}::text || '::' || '{sys}' || '::' || {sv}::text)",
            vo = qcol(voa, "vo_id"),
            sys = system_id.replace('\'', "''"),
            sv = qcol(voa, "sys_version")
        ),
        VersionField::TimeCommitted => qcol(aud, "time_committed"),
        VersionField::SystemId => qcol(aud, "system_id"),
        VersionField::ChangeType => qcol(aud, "change_type"),
        VersionField::Committer => qcol(aud, "committer"),
        VersionField::Description => qcol(aud, "description"),
        VersionField::ContributionId => format!("{}::text", qcol(voa, "contribution_id")),
        VersionField::LifecycleState => qcol(voa, "lifecycle_state"),
    }
}

fn ehr_field_sql(alias: &str, field: EhrField, system_id: &str) -> Result<String, AqlError> {
    Ok(match field {
        EhrField::EhrId | EhrField::Whole => format!("{}::text", qcol(alias, "id")),
        EhrField::TimeCreated => qcol(alias, "time_created"),
        EhrField::SystemId => format!("'{}'", system_id.replace('\'', "''")),
    })
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
