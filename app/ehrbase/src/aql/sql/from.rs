//! FROM-clause + containment lowering (QUERY master03 §Class expressions,
//! §Containment, §VERSION sources) and the population / scope gates.
//!
//! No openEHR spec governs the join mechanics — this is our own design
//! (`docs/design/aql-engine.md`): the FROM containment tree becomes a **cross
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
use sea_query::{Alias, BinOper, Expr, ExprTrait as _, Query, SelectStatement};

use crate::aql::error::{AqlError, SqlError};
use crate::aql::ir::{
    Contained, ContainsTree, EhrField, EhrPredicate, Link, RmSource, Source, VersionField,
    VersionScope,
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

impl Builder<'_> {
    // ── FROM / containment ────────────────────────────────────────────────────

    pub(super) fn build_from(&mut self, tree: &ContainsTree) -> Result<(), AqlError> {
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
            // G-01: OR-containment (QUERY master03 §Containment — "Logical
            // operators AND and OR"). The enclosing object satisfies either
            // branch's containment; each branch lowers to a correlated `EXISTS`
            // over the shared anchor, combined with `OR`. A top-level `OR` with
            // no enclosing object is rejected (there is no row scope to
            // correlate to). Variables bound only inside an `OR` branch are
            // containment-filter-only — see the module PORT NOTE.
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
                // G-08: the exclusion is the negation of "parent contains
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

    /// Build the correlated `EXISTS` boolean for "the `anchor` object contains
    /// `tree`" (QUERY master03 §Containment). Recurses through `AND`/`OR`
    /// subtrees (each a conjunction / disjunction of `EXISTS`) and nested
    /// `[NOT] CONTAINS` within an operand. Used by both OR-containment (G-01) and
    /// the generalised NOT-CONTAINS anti-join (G-08).
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
                let Source::Rm(r) = &self.ir.sources[source.0] else {
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
                if let Some(cond) = type_cond(&alias, &r.rm_type) {
                    sub.and_where(cond);
                }
                if let Some(a) = &r.archetype {
                    let cond = self.archetype_cond(&alias, a)?;
                    sub.and_where(cond);
                }
                if let Some(n) = &r.name {
                    let cond = self.name_cond(&alias, n)?;
                    sub.and_where(cond);
                }
                for sp in &r.standard {
                    let cond = self.std_cond(&alias, sp)?;
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
            let ids = self.ctx.ehr_ids.clone();
            for root in self.group_roots.clone() {
                self.q.and_where(col(&root, "ehr_id").is_in(ids.clone()));
            }
        }
        // ABAC patient scope (`docs/enterprise/access-control.md` §6.4 — no
        // openEHR spec governs this, our own access-control extension): restrict
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
        for root in self.group_roots.clone() {
            let sub = self.queryable_ehr_subquery();
            self.q.and_where(col(&root, "ehr_id").in_subquery(sub));
        }
        for alias in self.ehr_alias.values().cloned().collect::<Vec<_>>() {
            let sub = self.queryable_ehr_subquery();
            self.q.and_where(col(&alias, "id").in_subquery(sub));
        }
    }

    /// `SELECT ehr_id FROM vo_version JOIN node(num = 0) …` — the EHR ids whose
    /// current (`upper_inf`) `EHR_STATUS` has `is_queryable = true`.
    /// `is_queryable` is a scalar attribute of `EHR_STATUS`, so it lives inline in
    /// the `EHR_STATUS` **root** node's verbatim canonical `data` fragment
    /// (`num = 0`; children are pruned but scalars stay).
    fn queryable_ehr_subquery(&mut self) -> SelectStatement {
        let sv = format!("qgv{}", self.next_ctr());
        let sn = format!("qgn{}", self.next_ctr());
        let mut sub = Query::select();
        sub.expr(col(&sv, "ehr_id"));
        sub.from_as(VoVersion::Table, Alias::new(sv.as_str()));
        sub.from_as(Node::Table, Alias::new(sn.as_str()));
        sub.and_where(col(&sn, "vo_id").eq(col(&sv, "vo_id")));
        sub.and_where(col(&sn, "sys_version").eq(col(&sv, "sys_version")));
        sub.and_where(col(&sn, "num").eq(Expr::val(0)));
        sub.and_where(col(&sv, "kind").eq(Expr::val("EHR_STATUS")));
        // Current = the latest TRUNK version (branches coexist with the trunk;
        // RM common master06 latest_trunk_version).
        sub.and_where(call("upper_inf", vec![col(&sv, "sys_period")]));
        sub.and_where(col(&sv, "branch_number").eq(Expr::val(0)));
        sub.and_where(
            col(&sn, "data")
                .binary(BinOper::Custom("->>"), Expr::val("is_queryable"))
                .eq(Expr::val("true")),
        );
        sub
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
        self.q.from_as(VoVersion::Table, Alias::new(vo.as_str()));
        self.q.from_as(Node::Table, Alias::new(node.as_str()));
        self.q.and_where(col(&vo, "ehr_id").eq(col(&ehr, "id")));
        self.q
            .and_where(col(&vo, "kind").eq(Expr::val("EHR_STATUS")));
        self.q
            .and_where(call("upper_inf", vec![col(&vo, "sys_period")]));
        // Current = latest trunk (master06 latest_trunk_version).
        self.q.and_where(col(&vo, "branch_number").eq(Expr::val(0)));
        self.q.and_where(col(&node, "vo_id").eq(col(&vo, "vo_id")));
        self.q
            .and_where(col(&node, "sys_version").eq(col(&vo, "sys_version")));
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
        self.q.from_as(Audit::Table, Alias::new(alias.as_str()));
        self.q.and_where(col(&alias, "id").eq(col(voa, "audit_id")));
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
                let aud = self.ensure_audit(voa);
                let lhs = version_field_expr(voa, &aud, p.field, &self.ctx.system_id);
                let rhs = cast(Expr::val(self.bind_value(&p.value)?), "text");
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
/// versioned objects; the store's `vo_version.kind` discriminants).
fn is_vo_root_type(t: &str) -> bool {
    matches!(t, "COMPOSITION" | "EHR_STATUS" | "EHR_ACCESS" | "FOLDER")
}
