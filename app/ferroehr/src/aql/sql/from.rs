// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! FROM-clause + containment lowering (QUERY master03 §Class expressions,
//! §Containment, §VERSION sources) and the population / scope gates.
//!
//! No openEHR spec governs the join mechanics — this is our own design: the FROM containment tree becomes a **cross
//! join of table aliases + typed WHERE conditions** (the planner folds
//! cross-join+filter into joins). Each RM source that roots a versioned object
//! gets a `node` + `vo_version` (+ `audit`) alias; content sources contained
//! within it share the `vo_version` and interval-join
//! (`num BETWEEN a.num AND a.num_cap`, same `(vo_id, sys_version)`). `EHR`
//! sources join VO roots via `ehr_id`; `VERSION` sources share the contained
//! VO's `vo_version`. `AND` containment is a join; `OR` and `NOT CONTAINS` are
//! disjunctive / anti-join correlated `EXISTS` filters (QUERY master03
//! §Containment — boolean `AND`/`OR`, `NOT`).

use sea_query::extension::postgres::PgExpr as _;
use sea_query::{Alias, Asterisk, Expr, ExprTrait as _, JoinType, Query, SelectStatement};
use uuid::Uuid;

use crate::aql::error::{AqlError, SqlError};
use crate::aql::ir::{
    Contained, ContainsTree, EhrField, EhrPredicate, LeafPath, Link, Operand, PathTarget, QueryIr,
    RmSource, SelectValue, Source, VersionField, VersionScope,
};
use crate::db::iden::{Audit, Ehr, Node, VoVersion};

use super::expr::{call, cast, col, type_cond};
use super::value::version_field_expr;
use super::{Builder, VoGroup};

/// The correlation anchor for an `OR` / `NOT CONTAINS` `EXISTS` subquery.
enum ExistsAnchor {
    /// Interval-anchored inside a node subtree of a shared versioned object:
    /// `num BETWEEN parent.num AND parent.num_cap`, same `(vo_id, sys_version)`.
    Vo(String),
    /// Contained in an EHR as its own versioned object: `ehr_id` join + version
    /// scope.
    Ehr(String),
}

/// The streaming-shape plan: the linear containment chain eligible for the
/// LIMIT-streaming FROM (see [`Builder::build_from_streaming`]).
#[derive(Debug)]
pub(super) struct StreamPlan {
    /// The enclosing `EHR` source, when present.
    ehr: Option<usize>,
    /// The single versioned-object-root RM source.
    root: usize,
    /// Contained content sources as `(source id, parent source id)`,
    /// pre-order.
    contained: Vec<(usize, usize)>,
}

/// Extract the streaming-eligible containment chain, or `None` when the
/// tree needs the general flat shape: `OR`/`NOT CONTAINS` branches,
/// `VERSION` sources, a bare `EHR` with no versioned-object root, nested
/// versioned-object roots (a second version group), or an untyped root.
pub(super) fn streaming_plan(ir: &QueryIr) -> Option<StreamPlan> {
    let (ehr, root_tree) = match &ir.contains {
        ContainsTree::Operand { source, contained } => match ir.sources.get(source.0) {
            Some(Source::Ehr(_)) => match contained.as_deref() {
                Some(Contained {
                    link: Link::Contains,
                    tree,
                }) => (Some(source.0), tree),
                _ => return None,
            },
            Some(Source::Rm(_)) => (None, &ir.contains),
            Some(Source::Version(_)) | None => return None,
        },
        _ => return None,
    };
    let ContainsTree::Operand { source, contained } = root_tree else {
        return None;
    };
    let root = source.0;
    let Some(Source::Rm(r)) = ir.sources.get(root) else {
        return None;
    };
    if r.rm_type.is_empty() || !r.rm_type.names().iter().all(|t| is_vo_root_type(t)) {
        return None;
    }
    let mut plan = StreamPlan {
        ehr,
        root,
        contained: Vec::new(),
    };
    if let Some(c) = contained.as_deref() {
        if !matches!(c.link, Link::Contains) {
            return None;
        }
        if !collect_contained(ir, &c.tree, root, &mut plan.contained) {
            return None;
        }
    }
    Some(plan)
}

/// Collect the contained content sources of a streaming chain (pre-order),
/// or `false` when the subtree is ineligible.
fn collect_contained(
    ir: &QueryIr,
    tree: &ContainsTree,
    parent: usize,
    out: &mut Vec<(usize, usize)>,
) -> bool {
    match tree {
        ContainsTree::And(a, b) => {
            collect_contained(ir, a, parent, out) && collect_contained(ir, b, parent, out)
        }
        ContainsTree::Or(..) => false,
        ContainsTree::Operand { source, contained } => {
            let sid = source.0;
            let Some(Source::Rm(r)) = ir.sources.get(sid) else {
                return false;
            };
            // A nested versioned-object root would open its own version
            // group — the flat shape owns that case.
            if r.rm_type.names().iter().any(|t| is_vo_root_type(t)) {
                return false;
            }
            out.push((sid, parent));
            match contained.as_deref() {
                None => true,
                Some(Contained {
                    link: Link::Contains,
                    tree,
                }) => collect_contained(ir, tree, sid, out),
                Some(_) => false,
            }
        }
    }
}

/// Whether an identified path must read the streaming ROOT source's node row.
/// The one root-sourced shape the version spine serves without the node row is
/// the direct `uid/value` synthesis (`version_uid_expr`: no structure hop, no
/// predicates — the `OBJECT_VERSION_ID` is composed from `vo_version` columns per
/// RM common master06 §Version Identification). Everything else — promoted
/// columns, fragment reads, whole-object projection (empty fragment), any
/// predicate along the path — reads `node`. The bare-`uid` object projection is
/// conservatively counted as needing the node row (its non-projection uses fall
/// through to the fragment).
fn leaf_needs_root_node(leaf: &LeafPath, root: usize) -> bool {
    if leaf.source.0 != root {
        return false;
    }
    if leaf.anchor.is_empty()
        && leaf.root_predicate.is_none()
        && leaf.fragment.iter().all(|s| s.predicate.is_none())
    {
        let names: Vec<&str> = leaf.fragment.iter().map(|s| s.name.as_str()).collect();
        if names.as_slice() == ["uid", "value"] {
            return false;
        }
    }
    true
}

fn path_needs_root_node(target: &PathTarget, root: usize) -> bool {
    match target {
        PathTarget::Data(leaf) => leaf_needs_root_node(leaf, root),
        // Version metadata reads `vo_version`; EHR fields read `ehr`; an
        // `e/ehr_status/...` path joins its OWN EHR_STATUS root node (its
        // leaf is rooted at the EHR source, never the streaming root).
        PathTarget::Version { .. } | PathTarget::Ehr { .. } | PathTarget::EhrStatus(_) => false,
    }
}

fn operand_needs_root_node(operand: &Operand, root: usize) -> bool {
    match operand {
        Operand::Path(t) => path_needs_root_node(t, root),
        Operand::Function { args, .. } => args.iter().any(|a| operand_needs_root_node(a, root)),
        Operand::Literal(_) | Operand::Param(_) => false,
    }
}

fn filter_needs_root_node(expr: &crate::aql::ir::Expr, root: usize) -> bool {
    match expr {
        crate::aql::ir::Expr::Compare { lhs, rhs, .. } => {
            operand_needs_root_node(lhs, root) || operand_needs_root_node(rhs, root)
        }
        crate::aql::ir::Expr::Exists(t)
        | crate::aql::ir::Expr::Like { path: t, .. }
        | crate::aql::ir::Expr::Matches { path: t, .. } => path_needs_root_node(t, root),
        crate::aql::ir::Expr::Const(_) => false,
        crate::aql::ir::Expr::And(a, b) | crate::aql::ir::Expr::Or(a, b) => {
            filter_needs_root_node(a, root) || filter_needs_root_node(b, root)
        }
        crate::aql::ir::Expr::Not(a) => filter_needs_root_node(a, root),
    }
}

/// Whether ANY identified path in the query (SELECT, WHERE, ORDER BY) reads
/// the streaming root's node row — the projection/predicate half of the
/// dead-root-lateral test (`build_from_streaming`).
fn root_node_referenced(ir: &QueryIr, root: usize) -> bool {
    ir.select.iter().any(|c| match &c.value {
        SelectValue::Path(t) => path_needs_root_node(t, root),
        SelectValue::Aggregate { arg, .. } => {
            arg.as_ref().is_some_and(|t| path_needs_root_node(t, root))
        }
        SelectValue::Function { args, .. } => args.iter().any(|a| operand_needs_root_node(a, root)),
        SelectValue::Literal(_) => false,
    }) || ir
        .filter
        .as_ref()
        .is_some_and(|f| filter_needs_root_node(f, root))
        || ir
            .order_by
            .iter()
            .any(|k| path_needs_root_node(&k.path, root))
}

impl Builder<'_> {
    // ── FROM / containment ────────────────────────────────────────────────────

    pub(super) fn build_from(&mut self, tree: &ContainsTree) -> Result<(), AqlError> {
        self.walk(tree, None, None)?;
        Ok(())
    }

    /// Emit the STREAMING FROM shape for a LIMIT-bearing, unordered query:
    /// the version spine (`vo_version`) is the single FROM item and every
    /// node source hangs off it as a `LATERAL` subquery (`OFFSET 0` as the
    /// planner's documented pull-up fence), so `PostgreSQL` walks current
    /// versions lazily, probes each through the nested-set indexes, and
    /// stops at the LIMIT — instead of materializing an archetype-anchor
    /// bitmap over the whole corpus first (the measured 1M-entry/1 s
    /// failure on non-selective anchors).
    ///
    /// Semantics are identical to the flat shape: each `LATERAL` returns
    /// ALL matching nodes per outer row, so row-per-binding multiplicity
    /// is preserved (QUERY master03 §DISTINCT — duplicates exist by
    /// default), and QUERY master03 §LIMIT makes WHICH rows return
    /// explicitly non-deterministic without `ORDER BY`, so the changed
    /// visit order is conformant. No openEHR spec governs the join
    /// mechanics — the shape selection is our own design.
    pub(super) fn build_from_streaming(&mut self, plan: &StreamPlan) -> Result<(), AqlError> {
        self.streaming = true;
        let Some(Source::Rm(root)) = self.ir.sources.get(plan.root).cloned() else {
            return Err(SqlError::Unsupported(
                "streaming plan root is not an RM source".to_owned(),
            )
            .into());
        };

        // The version spine: the one FROM item everything joins onto.
        let v = format!("v{}", plan.root);
        self.q.from_as(VoVersion::Table, Alias::new(v.as_str()));
        let kinds: Vec<String> = root.rm_type.names().to_vec();
        self.q.and_where(col(&v, "kind").is_in(kinds));
        self.push_scope(&v, &root.scope)?;

        if let Some(esid) = plan.ehr {
            let e = format!("e{esid}");
            self.q.join_as(
                JoinType::Join,
                Ehr::Table,
                Alias::new(e.as_str()),
                col(&e, "id").eq(col(&v, "ehr_id")),
            );
            let Some(Source::Ehr(src)) = self.ir.sources.get(esid).cloned() else {
                return Err(SqlError::Unsupported(
                    "streaming plan EHR is not an EHR source".to_owned(),
                )
                .into());
            };
            for p in &src.predicates {
                self.push_ehr_predicate(&e, p)?;
            }
            self.ehr_alias.insert(esid, e);
        }

        let group_root = self.stream_root(plan, &root, &v)?;
        self.vo_alias.insert(plan.root, v.clone());
        self.group_roots.push(group_root.clone());
        self.group_vos.push(v.clone());
        if plan.ehr.is_some() {
            self.roots_linked_to_ehr.insert(group_root);
        }

        // Contained content sources: one LATERAL per source, interval-bound
        // into its parent's subtree. A first-level source under a dead root
        // (no node alias) binds directly on the spine's version instead.
        for (sid, parent) in &plan.contained {
            let parent_alias = self.node_alias.get(parent).cloned();
            if parent_alias.is_none() && *parent != plan.root {
                return Err(SqlError::Unsupported(
                    "streaming plan parent without an alias".to_owned(),
                )
                .into());
            }
            let Some(Source::Rm(r)) = self.ir.sources.get(*sid).cloned() else {
                return Err(SqlError::Unsupported(
                    "streaming plan content is not an RM source".to_owned(),
                )
                .into());
            };
            let alias = format!("n{sid}");
            let mut sub = Query::select();
            sub.column(Asterisk).from(Node::Table).offset(0);
            if let Some(p) = &parent_alias {
                sub.and_where(Expr::col(Alias::new("vo_id")).eq(col(p, "vo_id")))
                    .and_where(Expr::col(Alias::new("sys_version")).eq(col(p, "sys_version")))
                    .and_where(
                        Expr::col(Alias::new("num")).between(col(p, "num"), col(p, "num_cap")),
                    );
            } else {
                // Dead-root binding: the root's subtree is the whole
                // versioned object, so containment in it IS version
                // membership — same `(vo_id, sys_version)` as the spine.
                sub.and_where(Expr::col(Alias::new("vo_id")).eq(col(&v, "vo_id")))
                    .and_where(Expr::col(Alias::new("sys_version")).eq(col(&v, "sys_version")));
            }
            self.q.join_lateral(
                JoinType::Join,
                sub,
                Alias::new(alias.as_str()),
                Expr::val(true),
            );
            for cond in self.rm_conds(&alias, &r)? {
                self.q.and_where(cond);
            }
            self.node_alias.insert(*sid, alias);
        }
        Ok(())
    }

    /// Emit the streaming root's node lateral — or elide it. The lateral is
    /// DEAD WEIGHT when (a) the root source carries no node-level condition
    /// beyond its RM type — which the spine's `kind` filter already pins: a
    /// versioned object's `num = 0` row has `rm_type = kind` by construction —
    /// and (b) no identified path reads the root's node row
    /// ([`root_node_referenced`]). It is then one wasted `pk_node` probe per
    /// spine row: it yields exactly one row per version (every stored version
    /// has its root node), so dropping it preserves row multiplicity, and the
    /// root's subtree interval spans the whole versioned object, so
    /// first-level containment reduces to sharing the version. Measured as
    /// the post-streaming ladder's rung 1. No openEHR spec governs the join
    /// mechanics — our own design. Returns the alias the scope/gate machinery
    /// group-tracks (the node alias, or the spine when elided — it carries
    /// the `ehr_id` those gates filter on).
    fn stream_root(
        &mut self,
        plan: &StreamPlan,
        root: &RmSource,
        v: &str,
    ) -> Result<String, AqlError> {
        let root_node_needed = root.archetype.is_some()
            || root.name.is_some()
            || !root.standard.is_empty()
            || root_node_referenced(self.ir, plan.root);
        if !root_node_needed {
            return Ok(v.to_owned());
        }
        let root_alias = format!("n{}", plan.root);
        let mut sub = Query::select();
        sub.column(Asterisk)
            .from(Node::Table)
            .and_where(Expr::col(Alias::new("vo_id")).eq(col(v, "vo_id")))
            .and_where(Expr::col(Alias::new("sys_version")).eq(col(v, "sys_version")))
            .and_where(Expr::col(Alias::new("num")).eq(Expr::val(0)))
            .offset(0);
        self.q.join_lateral(
            JoinType::Join,
            sub,
            Alias::new(root_alias.as_str()),
            Expr::val(true),
        );
        for cond in self.rm_conds(&root_alias, root)? {
            self.q.and_where(cond);
        }
        self.node_alias.insert(plan.root, root_alias.clone());
        Ok(root_alias)
    }

    /// The single-source node conditions of an RM source (type, archetype,
    /// name, standard predicates) — the one implementation the flat walk,
    /// the `EXISTS` builder, and the streaming laterals all share.
    fn rm_conds(&self, alias: &str, r: &RmSource) -> Result<Vec<Expr>, AqlError> {
        let mut out = Vec::new();
        if let Some(cond) = type_cond(alias, &r.rm_type) {
            out.push(cond);
        }
        if let Some(a) = &r.archetype {
            out.push(self.archetype_cond(alias, a)?);
        }
        if let Some(n) = &r.name {
            out.push(self.name_cond(alias, n)?);
        }
        for sp in &r.standard {
            out.push(self.std_cond(alias, sp)?);
        }
        Ok(out)
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
            // OR-containment (QUERY master03 §Containment — "Logical
            // operators AND and OR"). The enclosing object satisfies either
            // branch's containment; each branch lowers to a correlated `EXISTS`
            // over the shared anchor, combined with `OR`. A top-level `OR` with
            // no enclosing object is rejected (there is no row scope to
            // correlate to). Variables bound only inside an `OR` branch are
            // containment-filter-only — see the module NOTE.
            ContainsTree::Or(a, b) => {
                let anchor = match (&vo, ehr) {
                    (Some(g), _) => ExistsAnchor::Vo(g.node.clone()),
                    (None, Some(e)) => ExistsAnchor::Ehr(e.to_owned()),
                    (None, None) => {
                        return Err(SqlError::Unsupported(
                            "OR at the top of the FROM tree without a containing object".to_owned(),
                        )
                        .into());
                    }
                };
                let left = self.contained_exists(&anchor, a)?;
                let right = self.contained_exists(&anchor, b)?;
                self.q.and_where(left.or(right));
                Ok(None)
            }
            ContainsTree::Operand { source, contained } => {
                let sid = source.0;
                match self.ir.sources.get(sid) {
                    Some(Source::Version(_)) => {
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
                    Some(Source::Ehr(e)) => {
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
                    Some(Source::Rm(r)) => {
                        let r = r.clone();
                        let group = self.emit_rm(sid, &r, ehr, vo.as_ref())?;
                        if let Some(c) = contained {
                            self.contained_edge(c, ehr, Some(group.clone()))?;
                        }
                        Ok(Some(group))
                    }
                    None => Err(SqlError::Unsupported(
                        "containment operand names an unknown source".to_owned(),
                    )
                    .into()),
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
                // the exclusion is the negation of "parent contains
                // <tree>" — the same disjunction-capable `EXISTS` builder,
                // negated. This generalises NOT CONTAINS to compound (AND/OR)
                // and further-nested operands (QUERY master03 §Containment,
                // §NOT).
                let inner =
                    self.contained_exists(&ExistsAnchor::Vo(parent.node.clone()), &c.tree)?;
                self.q.and_where(inner.not());
                Ok(())
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
        for cond in self.rm_conds(&node, r)? {
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
            // The audit join is summoned lazily (`ensure_audit`) by the
            // version-field expressions that actually read it — a query that
            // never projects/filters audit fields pays no join to the
            // ever-growing audit table.
            self.push_scope(&voa, &r.scope)?;
            self.group_roots.push(node.clone());
            self.group_vos.push(voa.clone());
            self.vo_alias.insert(sid, voa.clone());
            if let Some(e) = ehr {
                // The EHR link binds on the version SPINE only: vo_version.
                // ehr_id equals the node rows' by construction (one owning
                // EHR per versioned object — RM ehr master04 §EHR), the node
                // group already joins the spine on (vo_id, sys_version), and
                // the spine predicate drives idx_vo_version_ehr. A node-side
                // twin predicate bought no plan this route does not serve and
                // cost a per-node-row index at every write (#2698).
                self.q.and_where(col(&voa, "ehr_id").eq(col(e, "id")));
                self.roots_linked_to_ehr.insert(node.clone());
            }
            Ok(VoGroup { node, vo: voa })
        }
    }

    /// Build the correlated `EXISTS` boolean for "the `anchor` object contains
    /// `tree`" (QUERY master03 §Containment). Recurses through `AND`/`OR`
    /// subtrees (each a conjunction / disjunction of `EXISTS`) and nested
    /// `[NOT] CONTAINS` within an operand. Used by both OR-containment and
    /// the generalised NOT-CONTAINS anti-join.
    fn contained_exists(
        &mut self,
        anchor: &ExistsAnchor,
        tree: &ContainsTree,
    ) -> Result<Expr, AqlError> {
        match tree {
            ContainsTree::And(a, b) => Ok(self
                .contained_exists(anchor, a)?
                .and(self.contained_exists(anchor, b)?)),
            ContainsTree::Or(a, b) => Ok(self
                .contained_exists(anchor, a)?
                .or(self.contained_exists(anchor, b)?)),
            ContainsTree::Operand { source, contained } => {
                let Some(Source::Rm(r)) = self.ir.sources.get(source.0) else {
                    return Err(SqlError::Unsupported(
                        "OR/NOT CONTAINS of a non-structure operand".to_owned(),
                    )
                    .into());
                };
                let r = r.clone();
                let alias = format!("x{}", self.next_ctr());
                let mut sub = Query::select();
                sub.expr(Expr::val(1));
                sub.from_as(Node::Table, Alias::new(alias.as_str()));
                self.anchor_correlation(&mut sub, anchor, &alias, &r.scope)?;
                for cond in self.rm_conds(&alias, &r)? {
                    sub.and_where(cond);
                }
                if let Some(c) = contained {
                    let inner = self.contained_exists(&ExistsAnchor::Vo(alias.clone()), &c.tree)?;
                    match c.link {
                        Link::Contains => sub.and_where(inner),
                        Link::NotContains => sub.and_where(inner.not()),
                    };
                }
                Ok(Expr::exists(sub))
            }
        }
    }

    /// Correlate an `EXISTS` subquery's operand node (`alias`) to its anchor:
    /// interval containment for a shared VO, or an `ehr_id` join + version scope
    /// for a VO contained in an EHR.
    fn anchor_correlation(
        &mut self,
        sub: &mut SelectStatement,
        anchor: &ExistsAnchor,
        alias: &str,
        scope: &VersionScope,
    ) -> Result<(), AqlError> {
        match anchor {
            ExistsAnchor::Vo(parent) => {
                sub.and_where(col(alias, "vo_id").eq(col(parent, "vo_id")));
                sub.and_where(col(alias, "sys_version").eq(col(parent, "sys_version")));
                sub.and_where(
                    col(alias, "num").between(col(parent, "num"), col(parent, "num_cap")),
                );
                Ok(())
            }
            ExistsAnchor::Ehr(e) => {
                let voa = format!("xv{}", self.next_ctr());
                sub.from_as(VoVersion::Table, Alias::new(voa.as_str()));
                sub.and_where(col(alias, "vo_id").eq(col(&voa, "vo_id")));
                sub.and_where(col(alias, "sys_version").eq(col(&voa, "sys_version")));
                sub.and_where(col(alias, "ehr_id").eq(col(e, "id")));
                // Same version-spine mirror as the FROM path (see above):
                // lets the planner bound the EXISTS by the EHR instead of the
                // corpus. Identical semantics by construction.
                sub.and_where(col(&voa, "ehr_id").eq(col(e, "id")));
                match scope {
                    VersionScope::Latest => {
                        sub.and_where(call("upper_inf", vec![col(&voa, "sys_period")]));
                        sub.and_where(col(&voa, "branch_number").eq(Expr::val(0)));
                    }
                    VersionScope::All => {}
                    VersionScope::Predicate(_) => {
                        return Err(SqlError::Unsupported(
                            "a version predicate on an OR/NOT-CONTAINS branch VO".to_owned(),
                        )
                        .into());
                    }
                }
                Ok(())
            }
        }
    }

    pub(super) fn apply_ehr_scope(&mut self) {
        // Multi-EHR scoping (`ehr_ids: List<UUID>`): restrict every VO root to
        // the id set with `ehr_id IN (…)` (equivalently `= ANY($ids)`). The
        // single-`ehr_id` REST case is just the one-element set. Empty = no
        // explicit scope (the population gate takes over).
        if !self.ctx.ehr_ids.is_empty() {
            // sea-query bind boundary: the `ehr_id` column is a plain `uuid`.
            let ids: Vec<Uuid> = self.ctx.ehr_ids.iter().map(|id| id.0).collect();
            for root in self.group_roots.clone() {
                self.q.and_where(col(&root, "ehr_id").is_in(ids.clone()));
            }
            // A bare `FROM EHR e` (no CONTAINS) has no VO root — the scope
            // must bind the EHR source itself, or a scoped query would run
            // over the whole population (ITS-REST query Request.md: `ehr_id`
            // "used to execute the query within a single EHR context").
            for alias in self.ehr_alias.values().cloned().collect::<Vec<_>>() {
                self.q.and_where(col(&alias, "id").is_in(ids.clone()));
            }
        }
        // ABAC patient scope (no openEHR spec governs this, our own
        // access-control extension): restrict
        // every VO root to the caller's patient EHRs. Rows outside are never
        // fetched — regardless of the query's projection (the v1 defect-#1 fix).
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

    /// The query-population gate (SM `I_QUERY_SERVICE.execute_ad_hoc_query` /
    /// `execute_stored_query`). When no explicit `ehr_ids` are supplied, the
    /// `ehr_ids` parameter doc mandates a "full population query [...] on all
    /// EHRs whose status has the `is_queryable` flag set to `True`"
    /// (`docs/specs/openehr/SM/docs/UML/classes/i_query_service.adoc`). We
    /// therefore restrict every EHR root — bare `EHR` sources (`ehr.id`) and VO
    /// roots (`node.ehr_id`) alike — to that set. A scoped query (a non-empty
    /// `ehr_ids` set) targets specific EHRs and is not gated.
    pub(super) fn apply_population_gate(&mut self) {
        if !self.ctx.ehr_ids.is_empty() {
            return;
        }
        // One gate per join-connected component: a VO root linked to
        // an EHR alias (`node.ehr_id = e.id`) is covered by that alias's gate —
        // gating both would filter the same EHR twice.
        for root in self.group_roots.clone() {
            if self.roots_linked_to_ehr.contains(&root) {
                continue;
            }
            self.gate_vo_root(&root);
        }
        // An EHR source already has its `ehr` row in the FROM — filter its
        // promoted `is_queryable` column directly.
        for alias in self.ehr_alias.values().cloned().collect::<Vec<_>>() {
            self.q.and_where(col(&alias, "is_queryable").eq(true));
        }
    }

    /// Gate an unlinked versioned-object root (no bound EHR alias covers it):
    /// join the owning `ehr` row on the node's `ehr_id` and filter the promoted
    /// [`is_queryable`](crate::db::iden::Ehr::IsQueryable) column. The flag is a
    /// scalar attribute of the current `EHR_STATUS` (RM ehr master04 §EHR Status,
    /// `EHR_STATUS.is_queryable` 1..1 Boolean), promoted onto the `ehr` row and
    /// kept in lockstep by the status write path, so the gate is a boolean-column
    /// filter over a PK join instead of a per-query `EXISTS` that index-scanned
    /// every current `EHR_STATUS` root. SM `I_QUERY_SERVICE`: a
    /// full-population query runs over "all EHRs whose status has the
    /// `is_queryable` flag set to `True`" (`i_query_service.adoc`). No openEHR
    /// spec governs the join mechanics — our own storage design.
    fn gate_vo_root(&mut self, root: &str) {
        let alias = format!("qg{}", self.next_ctr());
        let link = col(&alias, "id").eq(col(root, "ehr_id"));
        if self.streaming {
            self.q
                .join_as(JoinType::Join, Ehr::Table, Alias::new(alias.as_str()), link);
        } else {
            self.q.from_as(Ehr::Table, Alias::new(alias.as_str()));
            self.q.and_where(link);
        }
        self.q.and_where(col(&alias, "is_queryable").eq(true));
    }

    /// Join the EHR's current `EHR_STATUS` versioned-object root node for an EHR
    /// source, once, and register it as that source's node so the whole-object /
    /// anchor-walk / fragment machinery resolves paths under `ehr_status`.
    ///
    /// `EHR` is not a `node` and `EHR_STATUS` is a *separate* VO (RM 1.2.0
    /// `EHR.ehr_status`), so this is an engine-level join on the store, not a
    /// node-tree walk: `vo_version.ehr_id = ehr.id`, `kind = 'EHR_STATUS'`,
    /// latest version (`upper_inf(sys_period)`), root node (`num = 0`). Every
    /// EHR has exactly one current `EHR_STATUS`, so the inner join is 1:1. The
    /// population/`ehr_id` gates already constrain the `ehr` row, so the joined
    /// status inherits that scope transitively (no separate gating).
    pub(super) fn ensure_ehr_status_root(&mut self, ehr_sid: usize) -> Result<String, AqlError> {
        if let Some(a) = self.ehr_status_node.get(&ehr_sid) {
            return Ok(a.clone());
        }
        let ehr = self.ehr_alias.get(&ehr_sid).cloned().ok_or_else(|| {
            SqlError::Unsupported("ehr_status path on a non-EHR source".to_owned())
        })?;
        let vo = format!("esv{}", self.next_ctr());
        let node = format!("esn{}", self.next_ctr());
        let vo_link = col(&vo, "ehr_id").eq(col(&ehr, "id"));
        let node_link = col(&node, "vo_id")
            .eq(col(&vo, "vo_id"))
            .and(col(&node, "sys_version").eq(col(&vo, "sys_version")));
        if self.streaming {
            self.q.join_as(
                JoinType::Join,
                VoVersion::Table,
                Alias::new(vo.as_str()),
                vo_link,
            );
            self.q.join_as(
                JoinType::Join,
                Node::Table,
                Alias::new(node.as_str()),
                node_link,
            );
        } else {
            self.q.from_as(VoVersion::Table, Alias::new(vo.as_str()));
            self.q.from_as(Node::Table, Alias::new(node.as_str()));
            self.q.and_where(vo_link);
            self.q.and_where(node_link);
        }
        self.q
            .and_where(col(&vo, "kind").eq(Expr::val("EHR_STATUS")));
        self.q
            .and_where(call("upper_inf", vec![col(&vo, "sys_period")]));
        // Current = latest trunk (master06 latest_trunk_version).
        self.q.and_where(col(&vo, "branch_number").eq(Expr::val(0)));
        self.q.and_where(col(&node, "num").eq(Expr::val(0)));
        self.ehr_status_node.insert(ehr_sid, node.clone());
        // Register as the source node so `source_node`/`whole_object_alias`/
        // `data_leaf_expr` (which start from the leaf's source node) resolve the
        // EHR_STATUS root; the EHR source has no other node entry.
        self.node_alias.insert(ehr_sid, node.clone());
        Ok(node)
    }

    pub(super) fn ensure_audit(&mut self, voa: &str) -> String {
        if let Some(a) = self.audit_alias.get(voa) {
            return a.clone();
        }
        let alias = format!("a_{voa}");
        let cond = col(&alias, "id").eq(col(voa, "audit_id"));
        if self.streaming {
            self.q.join_as(
                JoinType::Join,
                Audit::Table,
                Alias::new(alias.as_str()),
                cond,
            );
        } else {
            self.q.from_as(Audit::Table, Alias::new(alias.as_str()));
            self.q.and_where(cond);
        }
        self.audit_alias.insert(voa.to_owned(), alias.clone());
        alias
    }

    fn push_scope(&mut self, voa: &str, scope: &VersionScope) -> Result<(), AqlError> {
        match scope {
            VersionScope::Latest => {
                // LATEST_VERSION = the latest TRUNK version (RM common master06
                // latest_trunk_version; open branch tips coexist and are not
                // "the latest version" of the container).
                self.q
                    .and_where(call("upper_inf", vec![col(voa, "sys_period")]));
                self.q.and_where(col(voa, "branch_number").eq(Expr::val(0)));
            }
            VersionScope::All => {}
            VersionScope::Predicate(p) if p.field == VersionField::TimeCommitted => {
                // Version-at-time: the TRUNK version whose validity contains the
                // instant (`sys_period @> $t`); a branch open at that instant
                // coexists by design and must not duplicate the row.
                let value = self.bind_value(&p.value)?;
                self.q.and_where(
                    col(voa, "sys_period").contains(cast(Expr::val(value), "timestamptz")),
                );
                self.q.and_where(col(voa, "branch_number").eq(Expr::val(0)));
            }
            VersionScope::Predicate(p) => {
                let system_id = self.ctx.system_id.clone();
                let rhs = cast(Expr::val(self.bind_value(&p.value)?), "text");
                let lhs = version_field_expr(voa, || self.ensure_audit(voa), p.field, &system_id);
                self.q
                    .and_where(lhs.binary(super::expr::binoper(p.op), rhs));
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
        self.q
            .and_where(lhs.binary(super::expr::binoper(p.op), rhs));
        Ok(())
    }
}

/// The VO-root RM types the store versions independently (RM common master06
/// versioned objects), decided from the storage domain itself
/// ([`crate::versioning::Kind`], the `vo_version.kind` discriminants) rather
/// than a restated list — a kind added there is a VO root here.
///
/// The EHR-scoped subset only: the demographic kinds (the five party roots and
/// `PARTY_RELATIONSHIP`) are versioned objects but are **unreachable** as an
/// AQL source, because a FROM class must have a concrete descendant the node
/// store addresses (`openehr_rm::v1_2::model::is_structure_root`, whose set excludes
/// the demographic LOCATABLE hierarchy) — `crate::aql::lower` refuses the rest
/// as [`crate::aql::error::AqlFeatureError::UnsupportedSourceClass`] before any
/// SQL is built. So the exclusion is that gate's consequence, stated here
/// explicitly, not a second opinion about what a versioned object is.
pub(super) fn is_vo_root_type(t: &str) -> bool {
    crate::versioning::Kind::from_type(t).is_some_and(|kind| !kind.is_demographic())
}
