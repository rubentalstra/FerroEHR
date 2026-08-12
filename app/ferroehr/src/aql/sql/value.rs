// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Value-expression lowering: the design's **path split** for a data leaf, the
//! typed coercions (QUERY master03 §Comparison operators + §ORDER BY), VERSION /
//! EHR metadata fields, ORDER BY, and paging.
//!
//! No openEHR spec governs the extraction mechanics — this is our own design: a leaf whose anchor is empty reads
//! its `data` fragment off the source node alias; a leaf with structure hops
//! reads through a **correlated scalar subquery** walking the anchor chain
//! (interval containment + promoted-column filters per step). The extracted
//! fragment is coerced per the resolved [`Coercion`]; subqueries return the value
//! or `NULL`, so a missing path compares false and never multiplies rows —
//! keeping `OR`/`NOT`/`EXISTS` correct.

use std::fmt::Write as _;

use sea_query::{Alias, Expr, ExprTrait as _, Order, Query};

use crate::aql::error::{AnalysisError, AqlError, SqlError};
use crate::aql::ir::{Coercion, EhrField, LeafPath, OrderKey, PathTarget, Source, VersionField};
use crate::db::iden::Node;
use crate::storage::promoted::{PROMOTED_LEAVES, PromotedKind};

use super::expr::{as_text, call, cast, col, extract_base, order_coercion};
use super::from::is_vo_root_type;
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
                let system_id = self.ctx.system_id.clone();
                Ok(version_field_expr(
                    &voa,
                    || self.ensure_audit(&voa),
                    *field,
                    &system_id,
                ))
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
        // The server-assigned OBJECT_VERSION_ID (`uid[/value]`) is synthesized
        // from the joined `vo_version`, not stored in the fragment (uid synthesis).
        if let Some(expr) = self.version_uid_expr(leaf, mode) {
            return Ok(expr);
        }
        // A registered promoted leaf reads its indexed `node` column instead of
        // the correlated subtree extraction (fast path).
        if let Some(expr) = self.promoted_leaf_expr(leaf, mode) {
            return Ok(expr);
        }
        let src = self.source_node(leaf.source.0)?;
        let jp = fragment_jsonpath(leaf);

        if leaf.anchor.is_empty() {
            let base = extract_base(col(&src, "data"), jp.as_deref());
            return Ok(coerce_value(base, mode, leaf));
        }

        let (mut sub, last) = self.anchored_walk(leaf, &src)?;
        let base = extract_base(col(&last, "data"), jp.as_deref());
        sub.expr(coerce_value(base, mode, leaf));
        sub.limit(1);
        Ok(Expr::from(sub))
    }

    /// The anchored containment walk shared by the scalar extraction (above)
    /// and the existential predicate lowering
    /// ([`data_leaf_exists`](Self::data_leaf_exists)): a correlated
    /// `SELECT`-less subquery joining one `node` alias per anchor step, with
    /// the nested-set containment + type/constraint conditions. Returns the
    /// statement (FROM/WHERE only) and the innermost alias.
    fn anchored_walk(
        &mut self,
        leaf: &LeafPath,
        src: &str,
    ) -> Result<(sea_query::SelectStatement, String), AqlError> {
        let mut sub = Query::select();
        let mut prev = src.to_owned();
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
        Ok((sub, last))
    }

    /// The EXISTENTIAL lowering of a predicate on an anchored data leaf:
    /// `EXISTS (SELECT 1 <walk> WHERE <cond(extracted value)>)` — the
    /// predicate holds when ANY node the path matches satisfies it.
    ///
    /// NOTE: QUERY master03 §WHERE is silent on predicates over multi-valued
    /// paths — any-match is our own design decision (also the prior-art
    /// convention): it is deterministic where a scalar `LIMIT 1` pick without
    /// an ordering is not, and it lets the planner cost the filter as a
    /// semi-join instead of an opaque scalar subquery (which the measured
    /// cross-EHR profile showed collapsing cardinality estimates and
    /// materializing bitmap plans under `LIMIT`).
    ///
    /// Returns `Ok(None)` when the leaf is not an anchored-walk extraction
    /// (uid synthesis, a promoted column, or an inline fragment read) — the
    /// caller falls back to the scalar comparison, which is exact there.
    pub(super) fn data_leaf_exists(
        &mut self,
        leaf: &LeafPath,
        mode: ValueMode,
        cond: impl FnOnce(Expr) -> Expr,
    ) -> Result<Option<Expr>, AqlError> {
        if leaf.anchor.is_empty()
            || self.version_uid_expr(leaf, mode).is_some()
            || self.promoted_leaf_expr(leaf, mode).is_some()
        {
            return Ok(None);
        }
        let src = self.source_node(leaf.source.0)?;
        let jp = fragment_jsonpath(leaf);
        let (mut sub, last) = self.anchored_walk(leaf, &src)?;
        let base = extract_base(col(&last, "data"), jp.as_deref());
        sub.expr(Expr::val(1));
        sub.and_where(cond(coerce_value(base, mode, leaf)));
        if self.streaming {
            // STREAMING shape: the EXISTS must stay CORRELATED. As a bare WHERE
            // sublink the planner may pull it up into a semi-join and
            // DECORRELATE its inner side into a corpus-wide Materialize, which
            // costs seconds per execution where the correlated probe runs in
            // milliseconds. Hosting the EXISTS inside a LATERAL subquery behind
            // the `OFFSET 0` fence pins the correlated per-row SubPlan by
            // construction (a `LIMIT 1` inside the sublink is NOT a fence).
            // Identical semantics: one boolean per outer row.
            let probe = format!("p{}", self.next_ctr());
            let mut wrapper = Query::select();
            wrapper.expr_as(Expr::exists(sub), Alias::new("hit"));
            wrapper.offset(0);
            self.q.join_lateral(
                sea_query::JoinType::Join,
                wrapper,
                Alias::new(probe.as_str()),
                Expr::val(true),
            );
            return Ok(Some(col(&probe, "hit")));
        }
        Ok(Some(Expr::exists(sub)))
    }

    /// The server-assigned `OBJECT_VERSION_ID` for a `uid` / `uid/value` path on
    /// a versioned-object-root variable (uid synthesis). The uid is assigned at commit and
    /// is **not** persisted in the canonical fragment (the REST read path injects
    /// it — `service::ehr::meta::with_uid`), so an AQL jsonb extraction finds
    /// nothing; QUERY master03 §Identified paths lists `COMPOSITION.uid.value` as
    /// normative, so it is synthesized here from the already-joined `vo_version`
    /// via the same law as the wire id (RM common master06 §Version
    /// Identification): `vo_id::creating_system_id::version_tree_id`. `None`
    /// (fall through to the stored fragment) for a contained object's own `uid`,
    /// any predicate on the path, or a source with no versioned-object root.
    fn version_uid_expr(&mut self, leaf: &LeafPath, mode: ValueMode) -> Option<Expr> {
        // Only a direct `uid[/value]` — no structure hop, no path predicate.
        if !leaf.anchor.is_empty()
            || leaf.root_predicate.is_some()
            || leaf.fragment.iter().any(|s| s.predicate.is_some())
        {
            return None;
        }
        // The source must be a versioned-object root: only there does the server
        // assign the OBJECT_VERSION_ID (a contained OBSERVATION's `uid` is its
        // own stored value, if any).
        let Some(Source::Rm(r)) = self.ir.sources.get(leaf.source.0) else {
            return None;
        };
        if r.rm_type.is_empty() || !r.rm_type.names().iter().all(|t| is_vo_root_type(t)) {
            return None;
        }
        let voa = self.vo_alias.get(&leaf.source.0).cloned()?;
        let names: Vec<&str> = leaf.fragment.iter().map(|s| s.name.as_str()).collect();
        let system_id = self.ctx.system_id.clone();
        let uid = version_field_expr(
            &voa,
            || self.ensure_audit(&voa),
            VersionField::Uid,
            &system_id,
        );
        match names.as_slice() {
            // `uid/value` → the OBJECT_VERSION_ID string (projected → JSON string
            // via the caller's `to_jsonb`; compared/ordered as text).
            ["uid", "value"] => Some(uid),
            // `uid` → the whole OBJECT_VERSION_ID object, only as a projected
            // cell; a non-projection use falls through (not a normative shape).
            ["uid"] if matches!(mode, ValueMode::Projection) => Some(call(
                "jsonb_build_object",
                vec![
                    Expr::val("_type"),
                    Expr::val("OBJECT_VERSION_ID"),
                    Expr::val("value"),
                    uid,
                ],
            )),
            _ => None,
        }
    }

    /// The promoted-leaf fast path:
    /// when `leaf` addresses a registered promoted leaf on its versioned-object
    /// root and the requested `mode` matches the column's kind, read
    /// `node.<column>` (indexed) instead of lowering the correlated subtree
    /// extraction + coercion. `None` (fall through to the general lowering) for
    /// every non-promoted leaf, a projection / raw-numeric read, any predicate
    /// along the path (a column cannot honor one), or a coercion mismatch. No
    /// openEHR spec governs the lowering — our own design
    /// (`crate::storage::promoted`).
    fn promoted_leaf_expr(&self, leaf: &LeafPath, mode: ValueMode) -> Option<Expr> {
        let ValueMode::Value(coercion) = mode else {
            return None;
        };
        // The source must be an RM versioned-object root bound to a single
        // rm_type, so its node rows are exactly the `num = 0` root the column
        // is populated on.
        let Some(Source::Rm(r)) = self.ir.sources.get(leaf.source.0) else {
            return None;
        };
        if !r.rm_type.is_singleton() {
            return None;
        }
        let rm_type = r.rm_type.names().first()?.as_str();
        // A promoted column carries no predicate context, so any predicate on the
        // path rules the substitution out.
        if leaf.root_predicate.is_some()
            || leaf.anchor.iter().any(|s| s.predicate.is_some())
            || leaf.fragment.iter().any(|s| s.predicate.is_some())
        {
            return None;
        }
        // Flattened attribute path: anchor hops then fragment names.
        let path: Vec<&str> = leaf
            .anchor
            .iter()
            .map(|s| s.attribute.as_str())
            .chain(leaf.fragment.iter().map(|s| s.name.as_str()))
            .collect();
        let entry = PROMOTED_LEAVES
            .iter()
            .find(|p| p.rm_type == rm_type && p.path == path.as_slice())?;
        let compatible = match entry.kind {
            PromotedKind::Timestamp => coercion == Coercion::Temporal,
        };
        if !compatible {
            return None;
        }
        let alias = self.node_alias.get(&leaf.source.0)?;
        Some(col(alias, entry.column))
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
            let order = if ascending { Order::Asc } else { Order::Desc };
            // SELECT DISTINCT: PostgreSQL requires every ORDER BY expression
            // to appear in the select list, so a sort key that IS a selected
            // column orders by that output column (jsonb ordering of the
            // projected cell — numbers numeric, strings lexical), and an
            // unselected sort key is a typed reject: QUERY master03 §DISTINCT
            // defines no semantics for sorting a de-duplicated projection by
            // an expression outside it.
            if self.ir.distinct {
                let selected = self.ir.select.iter().position(
                    |c| matches!(&c.value, crate::aql::ir::SelectValue::Path(p) if *p == path),
                );
                match selected {
                    Some(i) => {
                        self.q.order_by(Alias::new(format!("col{i}")), order);
                        continue;
                    }
                    None => {
                        return Err(AnalysisError::DistinctOrderByUnselected.into());
                    }
                }
            }
            // ORDER BY `e/ehr_id[/value]` sorts by the raw `ehr.id` uuid column,
            // not the `CAST(id AS text)` the projection reads: a UUID's canonical
            // text form is fixed-length lowercase hex (BASE base_types master05
            // §Basic Types — Uuid), so lexical text order and the uuid binary
            // order coincide — the row sequence is byte-identical while the raw
            // column is index-served instead of forcing a per-row cast. The text
            // cast stays in the projection path.
            if let PathTarget::Ehr {
                source,
                field: EhrField::EhrId | EhrField::Whole,
            } = &path
                && let Some(alias) = self.ehr_alias.get(&source.0).cloned()
            {
                self.q.order_by_expr(col(&alias, "id"), order);
                continue;
            }
            let coercion = order_coercion(&path);
            let expr = self.value_expr(&path, ValueMode::Value(coercion))?;
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
/// §Comparison operators; `DV_ORDERED` ordered-magnitude via `ext.openehr_magnitude`).
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
        // ISO 8601 permits a COMMA decimal sign on the fractional second (BASE
        // foundation_types master06); PostgreSQL's timestamptz input does not,
        // and a comma cannot occur elsewhere in a valid ISO timestamp, so it
        // normalizes to the dot before the cast.
        // NOTE: temporal comparison casts the ISO-8601 leaf text to timestamptz
        // — precise for full timestamps; partial-precision values (`2019`,
        // `12:00`) are a documented gap (QUERY master03 §Dates and Times).
        ValueMode::Value(Coercion::Temporal) => cast(
            Expr::cust_with_exprs("replace($1, ',', '.')", [as_text(base)]),
            "timestamptz",
        ),
        ValueMode::Value(Coercion::Text | Coercion::Raw) => as_text(base),
        // a mixed-type (`Raw`) leaf being compared/matched against a
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
        // Comma-fraction normalization as on the leaf side (ISO 8601 permits
        // the comma decimal sign; PostgreSQL's timestamptz input does not).
        Coercion::Temporal => cast(
            Expr::cust_with_exprs("replace($1, ',', '.')", [cast(Expr::val(value), "text")]),
            "timestamptz",
        ),
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
///
/// `audit` summons the audit-table join LAZILY: only the commit-audit
/// fields read it, so a `uid`/`contribution_id`/`lifecycle_state` path
/// never pays the join (the same pay-per-use rule as `ensure_audit`'s
/// doc — the first live plan autopsy found the ward query joining `audit`
/// it never projected).
pub(super) fn version_field_expr(
    voa: &str,
    audit: impl FnOnce() -> String,
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
        VersionField::TimeCommitted => col(&audit(), "time_committed"),
        VersionField::SystemId => col(&audit(), "system_id"),
        VersionField::ChangeType => col(&audit(), "change_type"),
        // The rubric renders from the openEHR terminology group at SQL-build
        // time (the bundle is the authority, never a hardcoded rubric — the
        // same rule as the versioning render edge); the terminology id of
        // both coded version fields is the constant `openehr` (#976).
        VersionField::ChangeTypeRubric => {
            coded_rubric_case(&col(&audit(), "change_type"), "audit_change_type")
        }
        VersionField::Committer => col(&audit(), "committer"),
        // The stored description is the whole canonical DV_TEXT fragment, so
        // the addressed representation is a jsonb extraction: the bare
        // attribute is the object (as `committer` is), each scalar sub-path a
        // `->>` text read that is NULL when the description is uncoded.
        VersionField::Description => col(&audit(), "description"),
        VersionField::DescriptionValue => sea_query::extension::postgres::PgExpr::cast_json_field(
            col(&audit(), "description"),
            Expr::val("value"),
        ),
        VersionField::DescriptionCode => sea_query::extension::postgres::PgExpr::cast_json_field(
            sea_query::extension::postgres::PgExpr::get_json_field(
                col(&audit(), "description"),
                Expr::val("defining_code"),
            ),
            Expr::val("code_string"),
        ),
        VersionField::DescriptionTerminology => {
            sea_query::extension::postgres::PgExpr::cast_json_field(
                sea_query::extension::postgres::PgExpr::get_json_field(
                    sea_query::extension::postgres::PgExpr::get_json_field(
                        col(&audit(), "description"),
                        Expr::val("defining_code"),
                    ),
                    Expr::val("terminology_id"),
                ),
                Expr::val("value"),
            )
        }
        VersionField::ContributionId => cast(col(voa, "contribution_id"), "text"),
        VersionField::LifecycleState => col(voa, "lifecycle_state"),
        VersionField::LifecycleStateRubric => {
            coded_rubric_case(&col(voa, "lifecycle_state"), "version_lifecycle_state")
        }
        // Both coded version fields belong to the constant `openehr`
        // terminology (#976).
        VersionField::ChangeTypeTerminology | VersionField::LifecycleStateTerminology => {
            Expr::val("openehr")
        }
    }
}

/// `CASE <code_col> WHEN '<code>' THEN '<rubric>' … END` over an openEHR
/// terminology group, generated from the vendored TERM bundle (TERM 3.1.0
/// `openehr_terminology.xml`) — the coded version fields store the numeric
/// group code; their `…/value` sub-path compares against the rubric.
fn coded_rubric_case(code_col: &Expr, group_id: &str) -> Expr {
    let mut case = sea_query::CaseStatement::new();
    for concept in openehr_term::bundle::openehr().concepts_in_group(group_id) {
        case = case.case(
            code_col.clone().eq(Expr::val(concept.id.clone())),
            Expr::val(concept.rubric.clone()),
        );
    }
    case.into()
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
