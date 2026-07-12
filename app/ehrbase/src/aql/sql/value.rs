//! Value-expression lowering: the design's **path split** for a data leaf, the
//! typed coercions (QUERY master03 §Comparison operators + §ORDER BY), VERSION /
//! EHR metadata fields, ORDER BY, and paging.
//!
//! No openEHR spec governs the extraction mechanics — this is our own design
//! (`docs/design/aql-engine.md` §path split): a leaf whose anchor is empty reads
//! its `data` fragment off the source node alias; a leaf with structure hops
//! reads through a **correlated scalar subquery** walking the anchor chain
//! (interval containment + promoted-column filters per step). The extracted
//! fragment is coerced per the resolved [`Coercion`]; subqueries return the value
//! or `NULL`, so a missing path compares false and never multiplies rows —
//! keeping `OR`/`NOT`/`EXISTS` correct.

use std::fmt::Write as _;

use sea_query::{Alias, Expr, ExprTrait as _, Order, Query};

use crate::aql::error::{AqlError, SqlError};
use crate::aql::ir::{Coercion, EhrField, LeafPath, OrderKey, PathTarget, VersionField};
use crate::db::iden::Node;

use super::expr::{as_text, call, cast, col, extract_base, order_coercion};
use super::{Builder, ValueMode};

impl Builder<'_> {
    // ── value expressions (the path split) ──────────────────────────────────

    /// Lower an identified path to a value expression, discriminated by what it
    /// addresses (QUERY master03 §Identified Paths / §Identified expression).
    pub(super) fn value_expr(
        &mut self,
        target: &PathTarget,
        mode: ValueMode,
    ) -> Result<Expr, AqlError> {
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
            // A scalar leaf under `e/ehr_status` (e.g. `.../subject/...`,
            // `.../is_queryable`): join the EHR_STATUS root, then extract from
            // its node subtree via the same path split as any data leaf. RM 1.2.0
            // `EHR.ehr_status` (`docs/specs/openehr/RM/docs/ehr/`).
            PathTarget::EhrStatus(leaf) => {
                self.ensure_ehr_status_root(leaf.source.0)?;
                self.data_leaf_expr(leaf, mode)
            }
        }
    }

    /// The design's path split for a data leaf: empty anchor → read the source
    /// node's `data` fragment inline; non-empty anchor → a correlated scalar
    /// subquery walking the anchor chain and extracting the fragment.
    pub(super) fn data_leaf_expr(
        &mut self,
        leaf: &LeafPath,
        mode: ValueMode,
    ) -> Result<Expr, AqlError> {
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
            if let Some(cond) = super::expr::type_cond(&alias, &step.node_types) {
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

    /// Resolve a leaf's source to a node alias present in the FROM.
    pub(super) fn source_node(&self, sid: usize) -> Result<String, AqlError> {
        self.node_alias.get(&sid).cloned().ok_or_else(|| {
            SqlError::Unsupported("data path on a non-node source".to_owned()).into()
        })
    }

    // ── ORDER BY / paging ─────────────────────────────────────────────────────

    /// ORDER BY (QUERY master03 §ORDER BY): multi-key, ASC/DESC, Ordered types
    /// compared by ordered-magnitude via the key's own [`Coercion`].
    pub(super) fn build_order_by(&mut self) -> Result<(), AqlError> {
        for key in self.ir.order_by.clone() {
            let OrderKey { path, ascending } = key;
            let coercion = order_coercion(&path);
            let expr = self.value_expr(&path, ValueMode::Value(coercion))?;
            let order = if ascending { Order::Asc } else { Order::Desc };
            self.q.order_by_expr(expr, order);
        }
        Ok(())
    }

    /// LIMIT/OFFSET (QUERY master03 §LIMIT): the effective window is pre-composed
    /// by the service (AQL clause vs REST `fetch`/`offset`); bounds were checked
    /// at lowering.
    pub(super) fn build_paging(&mut self) {
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
}

// ── coercions ─────────────────────────────────────────────────────────────────

/// Apply the value coercion to a jsonb extraction base (QUERY master03
/// §Comparison operators; DV_ORDERED ordered-magnitude via `ext.openehr_magnitude`).
pub(super) fn coerce_value(base: Expr, mode: ValueMode, leaf: &LeafPath) -> Expr {
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
        // (`2019`, `12:00`) are a documented gap (QUERY master03 §Built-in
        // Types/Dates and Times).
        ValueMode::Value(Coercion::Temporal) => cast(as_text(base), "timestamptz"),
        ValueMode::Value(Coercion::Text | Coercion::Raw) => as_text(base),
        // G-12: a mixed-type (`Raw`) leaf being compared/matched against a
        // numeric literal — extract numerically, but guard on the stored jsonb
        // type so a non-number occurrence yields NULL (comparison false) instead
        // of a cast error. "numeric for numbers, text otherwise" (QUERY master03
        // §Comparison operators).
        ValueMode::RawNumeric => raw_numeric(base),
    }
}

/// Cast a bound right-hand-side value to match the comparison coercion
/// (QUERY master03 §Comparison operators).
pub(super) fn coerce_rhs(value: sea_query::Value, coercion: Coercion) -> Expr {
    match coercion {
        Coercion::Magnitude => cast(Expr::val(value), "numeric"),
        Coercion::Boolean => cast(Expr::val(value), "boolean"),
        Coercion::Temporal => cast(Expr::val(value), "timestamptz"),
        Coercion::Text | Coercion::Raw => cast(Expr::val(value), "text"),
    }
}

/// Guarded numeric extraction for a mixed-type (`Raw`) leaf: number-typed jsonb
/// occurrences extract as `numeric`; every non-number occurrence yields `NULL`,
/// so a numeric comparison against them is false — never a cast error and never a
/// silent lexical miscompare (QUERY master03 §Comparison operators). The `$1`/`$2`
/// placeholders both carry the same extraction expression.
fn raw_numeric(base: Expr) -> Expr {
    Expr::cust_with_exprs(
        "CASE WHEN jsonb_typeof($1) = 'number' THEN ($2 #>> '{}'::text[])::numeric END",
        [base.clone(), base],
    )
}

// ── VERSION / EHR fields ────────────────────────────────────────────────────

/// The typed SQL for a VERSION metadata field, off the `vo_version`/`audit`
/// aliases. The `uid` is synthesized as
/// `vo_id::creating_system_id::version_tree_id` from the STORED per-version
/// identity columns (never the live config `system_id` — the creating system
/// is immutable per version, RM common master06 §Distributed versioning) via
/// the typed `PgExpr::concatenate` `||` operator; the tree id renders
/// `trunk[.branch.version]`.
pub(super) fn version_field_expr(
    voa: &str,
    aud: &str,
    field: VersionField,
    _system_id: &str,
) -> Expr {
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
            col(voa, "creating_system_id"),
            Expr::val("::"),
            cast(col(voa, "trunk_version"), "text"),
            Expr::cust_with_exprs(
                "CASE WHEN $1 > 0 THEN '.' || $2 || '.' || $3 ELSE '' END",
                [
                    col(voa, "branch_number"),
                    cast(col(voa, "branch_number"), "text"),
                    cast(col(voa, "branch_version"), "text"),
                ],
            ),
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

/// The typed SQL for an EHR attribute (`ehr_id/value`, `time_created`,
/// `system_id`). RM 1.2.0 `EHR` (`docs/specs/openehr/RM/docs/ehr/`).
pub(super) fn ehr_field_expr(alias: &str, field: EhrField, system_id: &str) -> Expr {
    match field {
        EhrField::EhrId | EhrField::Whole => cast(col(alias, "id"), "text"),
        EhrField::TimeCreated => col(alias, "time_created"),
        EhrField::SystemId => Expr::val(system_id.to_owned()),
    }
}

// ── jsonpaths ───────────────────────────────────────────────────────────────

/// Build the fragment jsonpath (`$.a.b`) for a leaf, or `None` when the leaf
/// addresses the whole anchor node.
pub(super) fn fragment_jsonpath(leaf: &LeafPath) -> Option<String> {
    if leaf.fragment.is_empty() {
        return None;
    }
    let mut jp = String::from("$");
    for step in &leaf.fragment {
        let _ = write!(jp, ".{}", step.name);
    }
    Some(jp)
}

/// Build a jsonpath from a relative object path (`[a, b]` → `$.a.b`) — a node
/// standard-predicate sub-path (QUERY master03 §Standard predicate).
pub(super) fn jsonpath(parts: &[String]) -> String {
    let mut jp = String::from("$");
    for p in parts {
        let _ = write!(jp, ".{p}");
    }
    jp
}
