// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! IR → SQL lowering.
//!
//! Turns a typed [`QueryIr`] into one `SELECT` over the greenfield
//! `node`/`vo_version`/`ehr`/`audit` store, built entirely with `sea-query`'s
//! **typed** expression API + `sea-query-sqlx` — no string-concatenated SQL
//! (`.claude/rules/sqlx-conventions.md`). Every table/column reference is an
//! [`sea_query::Expr::col`], every literal binds through `Expr::val`
//! (parameterized on build), and the PostgreSQL-specific pieces use the
//! sanctioned typed escape hatches: [`sea_query::Func::cust`] for functions
//! sea-query does not model (`jsonb_path_query_first` / `to_jsonb` /
//! `upper_inf` / `openehr_magnitude`), the typed Postgres operators from
//! [`sea_query::extension::postgres::PgExpr`] (`@>` = `contains`, `||` =
//! `concatenate`), the built-in aggregates, and `cast_as` for casts. The only
//! string-operator escapes are the sanctioned [`sea_query::BinOper::Custom`]
//! set (`#>>`, `->>`). Runtime functions resolve unqualified (`search_path =
//! ehr, ext, public`).
//!
//! No openEHR spec governs the execution — openEHR defines the *language*, not
//! its lowering; the SQL shapes are our own design.
//! The construct-by-construct mapping to QUERY master03 lives in each submodule:
//! `from` (FROM/containment + scope gates), `select` (SELECT/aggregates),
//! `predicate` (WHERE/functions), `value` (the path split + coercions), and
//! `expr` (the typed building blocks + the `LIKE`/archetype translations).
//!
//! ## Coupling to the storage schema
//!
//! The builder references the `node`/`vo_version`/`ehr`/`audit` column
//! vocabulary directly and encodes the nested-set / `sys_period` /
//! `branch_number` semantics of the greenfield store;
//! `analyze::is_structure_root` must stay in lockstep with
//! `storage::codec::STRUCTURE_TYPES` (the `emit-rm-model` generator enforces
//! it). The `column_vocab` unit test (below) pins every column name the builder
//! emits against the `CREATE TABLE` definitions in
//! `migrations/ehr/0001_baseline.sql`, so a schema rename surfaces as a failing
//! test rather than a runtime SQL error.

mod expr;
mod from;
mod predicate;
mod select;
mod value;

use crate::ids::EhrId;
use std::collections::HashMap;
use std::sync::Arc;

use sea_query::{Alias, PostgresQueryBuilder, Query, SelectStatement, Value};
use sea_query_sqlx::{SqlxBinder as _, SqlxValues};

use super::error::{AqlError, SqlError};
use super::ir::{Bind, Coercion, ParamValue, Params, QueryIr};
use super::lineage::ArchetypeLineage;

use expr::{col, literal_value};

/// Static execution context the SQL generator needs beyond the IR: the CDR
/// system id (for `OBJECT_VERSION_ID` synthesis), the optional REST `ehr_id`
/// scope, and the composed effective paging window.
#[derive(Debug, Clone)]
pub struct SqlCtx {
    /// The openEHR system id stamped into synthesized `OBJECT_VERSION_ID`s.
    pub system_id: String,
    /// The set of EHRs the query is scoped to (`I_QUERY_SERVICE.execute_*`
    /// `ehr_ids: List<UUID> [0..1]`,
    /// `docs/specs/openehr/SM/docs/UML/classes/i_query_service.adoc`). The
    /// ITS-REST single `ehr_id` parameter is the one-element case. Empty = no
    /// explicit scope (the population gate over `is_queryable` EHRs applies).
    pub ehr_ids: Vec<EhrId>,
    /// The ABAC patient-scope subject id (no openEHR spec governs this, our own
    /// extension): when set, every
    /// VO root is restricted to EHRs whose `subject_id` equals it, so rows the
    /// caller may not see are never fetched — independent of what the query
    /// projects.
    pub subject_scope: Option<String>,
    /// The effective row limit (AQL `LIMIT`/`TOP` or REST `fetch`, pre-composed).
    pub limit: Option<i64>,
    /// The effective row offset (AQL `OFFSET` or REST `offset`, pre-composed).
    pub offset: Option<i64>,
    /// The stored archetype specialisation graph the `archetype_node_id`
    /// predicate resolves an AOM2-era identifier's descendants through
    /// (AM `Identification` master07 §Supporting Archetype-based Querying: for
    /// specialised archetypes the lineage "can only be obtained from the
    /// operational form of the archetype"). Resolved once per execution from
    /// the ADL2/OPT2 artefact store (the query service's cached
    /// `archetype_lineage` read); an empty index leaves the predicate at exact
    /// + ADL 1.4 concept-prefix matching.
    pub archetype_lineage: Arc<ArchetypeLineage>,
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

/// A built scope-collection query for the ABAC query post-check.
///
/// `SELECT DISTINCT` of every bound VO root's `ehr_id` + `template_id` over the
/// same containment/filter as the main query (no openEHR spec governs ABAC —
/// our own extension). Its rows carry the set of EHRs and templates the query
/// touches — across **all** bound variables and **independent of the
/// projection**.
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

/// A VO "group": the node alias that roots it and the `vo_version` alias its
/// nodes belong to. Content sources contained within it share the `vo` alias and
/// interval-join into `node`.
#[derive(Debug, Clone)]
struct VoGroup {
    node: String,
    vo: String,
    /// The resolved RM types of the operand that created this group — the
    /// containment-edge classifier's parent side (#2880).
    types: crate::aql::ir::TypeSet,
}

/// The extraction/coercion mode of a value expression.
#[derive(Debug, Clone, Copy)]
enum ValueMode {
    /// Raw jsonb value for a `RESULT_SET` cell.
    Projection,
    /// A coerced scalar for comparison / ordering / aggregation.
    Value(Coercion),
    /// A NULL-guarded numeric extraction for a mixed-type (`Raw`) leaf compared
    /// or matched against a numeric literal.
    RawNumeric,
}

/// The shared SQL-builder state; its `impl` blocks are fanned across the
/// submodules (`from`/`select`/`predicate`/`value`) — idiomatic, one struct.
struct Builder<'a> {
    ir: &'a QueryIr,
    params: &'a Params,
    ctx: &'a SqlCtx,
    q: SelectStatement,
    node_alias: HashMap<usize, String>,
    audit_alias: HashMap<String, String>,
    ehr_alias: HashMap<usize, String>,
    /// The `EHR_STATUS` root-node alias joined for an EHR source's `ehr_status`
    /// path (keyed by EHR source id; joined once, lazily, on first use).
    ehr_status_node: HashMap<usize, String>,
    /// The `vo_version` alias each versioned-object-root RM source opened
    /// (keyed by source id). Used to synthesize the server-assigned
    /// `OBJECT_VERSION_ID` for a `uid[/value]` path on a VO-root variable
    /// (RM common master06 §Version Identification), which is not stored in the
    /// canonical fragment.
    vo_alias: HashMap<usize, String>,
    version_vo: HashMap<usize, String>,
    /// Node aliases that root a VO group (targets of the REST `ehr_id` filter).
    group_roots: Vec<String>,
    /// The subset of `group_roots` whose rows are join-linked to a bound EHR
    /// alias (`node.ehr_id = e.id`) — the population gate on the EHR alias
    /// already covers them, so gating the root again would be a duplicate
    /// full-population subquery per query.
    roots_linked_to_ehr: std::collections::HashSet<String>,
    /// The `vo_version` alias for each entry in `group_roots` (parallel vec) —
    /// the source of the touched `template_id` for the ABAC scope collection.
    group_vos: Vec<String>,
    /// Fresh-alias counter for anchor subqueries / anti-joins.
    sub_ctr: usize,
    /// Whether the FROM was built in the STREAMING shape (one `vo_version`
    /// FROM item, everything else a join) — lazily summoned tables
    /// (`ensure_audit`, the population gate, the `EHR_STATUS` root) must
    /// then JOIN instead of adding comma-separated FROM items, or a later
    /// `LATERAL` could no longer reference them (SQL join-tree scoping).
    streaming: bool,
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
            ehr_status_node: HashMap::new(),
            vo_alias: HashMap::new(),
            version_vo: HashMap::new(),
            group_roots: Vec::new(),
            roots_linked_to_ehr: std::collections::HashSet::new(),
            group_vos: Vec::new(),
            sub_ctr: 0,
            streaming: false,
        }
    }

    pub(super) fn next_ctr(&mut self) -> usize {
        let n = self.sub_ctr;
        self.sub_ctr += 1;
        n
    }

    pub(super) fn bind_value(&self, b: &Bind) -> Result<Value, AqlError> {
        match b {
            Bind::Literal(lit) => Ok(literal_value(lit)),
            Bind::Param(p) => self.param_value(p),
        }
    }

    pub(super) fn param_value(&self, name: &str) -> Result<Value, AqlError> {
        match self.params.get(name) {
            Some(ParamValue::Int(i)) => Ok(Value::from(*i)),
            Some(ParamValue::Real(r)) => Ok(Value::from(*r)),
            Some(ParamValue::Bool(b)) => Ok(Value::from(*b)),
            Some(ParamValue::Str(s)) => Ok(Value::from(s.clone())),
            Some(ParamValue::Null) => Ok(Value::from(Option::<String>::None)),
            None => Err(SqlError::UnboundParameter(name.to_owned()).into()),
        }
    }

    pub(super) fn param_str(&self, name: &str) -> Result<String, AqlError> {
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

/// Whether the LIMIT-streaming FROM shape may apply. QUERY master03 §LIMIT:
/// without `ORDER BY`, "deterministic behavior" is explicitly not required —
/// which rows return is unconstrained, so a lazy streaming scan is
/// conformant. With `ORDER BY`, `DISTINCT`, or aggregates the full row set
/// is needed anyway; an EHR-scoped query (`ehr_ids`) is already bounded by
/// the `ehr_id` indexes and keeps the flat shape.
fn streaming_eligible(ir: &QueryIr, ctx: &SqlCtx) -> bool {
    ctx.limit.is_some()
        && ctx.ehr_ids.is_empty()
        && ir.order_by.is_empty()
        && !ir.distinct
        && !ir
            .select
            .iter()
            .any(|c| matches!(c.value, super::ir::SelectValue::Aggregate { .. }))
}

/// Lower a planned [`QueryIr`] to SQL.
///
/// # Errors
///
/// [`SqlError`] for a planner-accepted construct the SQL generator does not
/// render yet.
pub fn build(ir: &QueryIr, params: &Params, ctx: &SqlCtx) -> Result<PreparedQuery, AqlError> {
    let mut b = Builder::new(ir, params, ctx);
    match streaming_eligible(ir, ctx)
        .then(|| from::streaming_plan(ir))
        .flatten()
    {
        Some(plan) => b.build_from_streaming(&plan)?,
        None => b.build_from(&ir.contains)?,
    }
    b.apply_ehr_scope();
    b.apply_population_gate();
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
    b.apply_population_gate();
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

// ── test-only inline renderers ─────────────────────────────────────────────

#[cfg(test)]
mod column_vocab {
    //! Pin the builder's storage-column vocabulary to the schema. Every column
    //! name the IR→SQL lowering emits (collected here, one group per table)
    //! must be declared for that table in `migrations/ehr/0001_baseline.sql`,
    //! so a schema rename fails this test instead of failing at query runtime.

    /// The baseline migration — the authoritative schema (no openEHR spec
    /// governs the SQL — our own design).
    const BASELINE: &str = include_str!("../../../migrations/ehr/0001_baseline.sql");

    /// The columns the builder references, grouped by the table each `sea-query`
    /// alias resolves to. Keep in sync with the `col(..)` / `Expr::col(..)`
    /// call sites across `from`/`select`/`predicate`/`value`/`expr`.
    const VOCAB: &[(&str, &[&str])] = &[
        (
            "node",
            &[
                "vo_id",
                "sys_version",
                "num",
                "num_cap",
                "ehr_id",
                "rm_type",
                "archetype",
                "arch_entity",
                "arch_concept",
                "arch_major",
                "name",
                "data",
            ],
        ),
        (
            "vo_version",
            &[
                "vo_id",
                "kind",
                "ehr_id",
                "sys_version",
                "trunk_version",
                "branch_number",
                "branch_version",
                "sys_period",
                "lifecycle_state",
                "creating_system_id",
                "contribution_id",
                "audit_id",
                "template_id",
            ],
        ),
        (
            "ehr",
            &[
                "id",
                "system_id",
                "time_created",
                "subject_id",
                "is_queryable",
            ],
        ),
        (
            "audit",
            &[
                "id",
                "time_committed",
                "system_id",
                "change_type",
                "description",
                "committer",
            ],
        ),
    ];

    /// The `CREATE TABLE {table} ( … )` body from the baseline migration (the
    /// text between the opening paren and the balanced closing paren).
    fn create_table_body(table: &str) -> String {
        let head = format!("CREATE TABLE {table} (");
        let body = BASELINE
            .split_once(&head)
            .unwrap_or_else(|| panic!("no `{head}` in the baseline migration"))
            .1;
        let mut depth = 1usize;
        for (i, ch) in body.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return body.get(..i).unwrap_or_default().to_owned();
                    }
                }
                _ => {}
            }
        }
        panic!("unterminated CREATE TABLE {table} in the baseline migration");
    }

    /// Whether `body` declares a column named exactly `col` — a line whose
    /// trimmed text is `col` followed by a non-identifier char (so `num` does
    /// not match the `num_cap` declaration).
    fn declares_column(body: &str, col: &str) -> bool {
        body.lines().any(|line| {
            let t = line.trim_start();
            t.strip_prefix(col)
                .is_some_and(|rest| rest.starts_with([' ', '\t']))
        })
    }

    #[test]
    fn builder_columns_exist_in_baseline_schema() {
        for (table, columns) in VOCAB {
            let body = create_table_body(table);
            for col in *columns {
                assert!(
                    declares_column(&body, col),
                    "AQL SQL builder references column `{table}.{col}`, but it is not \
                     declared in `CREATE TABLE {table}` in 0001_baseline.sql — schema drift"
                );
            }
        }
    }
}
