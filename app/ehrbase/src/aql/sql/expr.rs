//! Typed `sea-query` building blocks shared across the SQL submodules, plus the
//! two lexical translations the AQL language pins exactly — the `LIKE` pattern
//! conversion (QUERY master03 §Operators/LIKE) and the `archetype_node_id`
//! predicate (QUERY master03 §Archetype predicate, re-grounded on BASE/AM
//! subsumption — see [`archetype_predicate`]).
//!
//! No openEHR spec governs the lowering itself — this is our own design
//! (`docs/design/aql-engine.md`): every reference is a typed [`Expr::col`], every
//! literal binds through [`Expr::val`], and the PostgreSQL-specific pieces use
//! only the sanctioned typed escape hatches ([`Func::cust`] for functions
//! sea-query does not model; [`BinOper::Custom`] for `#>>` / `->>` which have no
//! typed variant). Runtime functions resolve unqualified (`search_path = ehr,
//! ext, public`).

use std::fmt::Write as _;

use sea_query::{Alias, BinOper, Expr, ExprTrait as _, Func, Value};

use openehr_base::prelude::ArchetypeId;
use openehr_query::lexer::CompOp;

use crate::aql::ir::{Coercion, EhrField, LeafPath, PathTarget, TypeSet, TypedLit, VersionField};

/// A typed `"alias"."column"` reference.
pub(super) fn col(alias: &str, column: &str) -> Expr {
    Expr::col((Alias::new(alias), Alias::new(column)))
}

/// A typed custom-function call `name(args...)`.
pub(super) fn call(name: &str, args: Vec<Expr>) -> Expr {
    let mut f = Func::cust(Alias::new(name));
    for a in args {
        f = f.arg(a);
    }
    Expr::from(f)
}

/// `to_jsonb(x)` — normalizes any scalar into a canonical-JSON cell.
pub(super) fn to_jsonb(e: Expr) -> Expr {
    call("to_jsonb", vec![e])
}

/// `jsonb_path_query_first(data, '<jp>'::jsonpath)`.
pub(super) fn jsonb_path(data: Expr, jp: &str) -> Expr {
    call(
        "jsonb_path_query_first",
        vec![data, cast(Expr::val(jp.to_owned()), "jsonpath")],
    )
}

/// `<jsonb> #>> '{}'` — the scalar's text at the empty path.
pub(super) fn as_text(e: Expr) -> Expr {
    e.binary(BinOper::Custom("#>>"), cast(Expr::val("{}"), "text[]"))
}

/// A typed cast `<e>::<ty>`.
pub(super) fn cast(e: Expr, ty: &str) -> Expr {
    e.cast_as(Alias::new(ty))
}

/// The jsonb extraction base: `jsonb_path_query_first(<data>, jp)` when a
/// fragment path is present, else the raw `<data>` expression.
pub(super) fn extract_base(data: Expr, jp: Option<&str>) -> Expr {
    match jp {
        Some(jp) => jsonb_path(data, jp),
        None => data,
    }
}

/// The AQL comparison operators (QUERY master03 §Comparison operators) → the
/// typed `sea-query` binary operators.
pub(super) fn binoper(op: CompOp) -> BinOper {
    match op {
        CompOp::Eq => BinOper::Equal,
        CompOp::Ne => BinOper::NotEqual,
        CompOp::Lt => BinOper::SmallerThan,
        CompOp::Le => BinOper::SmallerThanOrEqual,
        CompOp::Gt => BinOper::GreaterThan,
        CompOp::Ge => BinOper::GreaterThanOrEqual,
    }
}

/// A typed AQL literal → a bound `sea-query` [`Value`] (QUERY master03
/// §Literals + §Built-in Types).
pub(super) fn literal_value(lit: &TypedLit) -> Value {
    match lit {
        TypedLit::Integer(i) => Value::from(*i),
        TypedLit::Real(r) => Value::from(*r),
        TypedLit::Boolean(b) => Value::from(*b),
        TypedLit::String(s) | TypedLit::Temporal(s) => Value::from(s.clone()),
        TypedLit::Null => Value::from(Option::<String>::None),
    }
}

/// A typed `rm_type IN (...)` condition, or `None` for an unresolved type set.
/// The concrete RM types a source/step may bind (QUERY master03 §Class
/// expressions; the abstract→concrete expansion is done in analysis).
pub(super) fn type_cond(node: &str, types: &TypeSet) -> Option<Expr> {
    if types.is_empty() {
        return None;
    }
    let members: Vec<Expr> = types.names().iter().map(|t| Expr::val(t.clone())).collect();
    Some(col(node, "rm_type").is_in(members))
}

/// The coercion an ORDER BY key uses (QUERY master03 §ORDER BY — Ordered types
/// compare by ordered-magnitude; mirrors the analyzer's leaf typing).
pub(super) fn order_coercion(target: &PathTarget) -> Coercion {
    match target {
        PathTarget::Data(leaf) | PathTarget::EhrStatus(leaf) => leaf.coercion,
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

/// A best-effort AQL path string for a `RESULT_SET` column's `path`
/// (ITS-REST 1.0.3 `RESULT_SET_COLUMN.path` — the RM path "as specified in
/// query"). `"/"` for a bare variable / whole-object leaf.
pub(super) fn leaf_path_string(leaf: &LeafPath) -> String {
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

// ── LIKE ──────────────────────────────────────────────────────────────────────

/// Translate an AQL `LIKE` pattern (`*` = any run, `?` = one char) to a SQL
/// `LIKE` pattern, escaping literal `%`/`_`/`\`. The AQL escapes `\*`/`\?` are
/// the LITERAL characters (QUERY master03 §Operators/LIKE); `\\` is a literal
/// backslash (SQL-escaped).
pub(super) fn aql_like_to_sql(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' => out.push('%'),
            '?' => out.push('_'),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            '\\' => match chars.next() {
                Some('*') => out.push('*'),
                Some('?') => out.push('?'),
                Some(other) => {
                    out.push_str("\\\\");
                    if other != '\\' {
                        out.push(other);
                    }
                }
                None => out.push_str("\\\\"),
            },
            other => out.push(other),
        }
    }
    out
}

// ── archetype_node_id predicate ──────────────────────────────────────

/// Build the `archetype_node_id` predicate condition for a node alias.
///
/// When `value` parses as a full archetype id (BASE `base_types` master05
/// §Archetype Identifiers, form `qualified_rm_entity.domain_concept.vN`), the
/// predicate implements **query subsumption**: BASE `architecture_overview`
/// master10 §Design-time Relationships — "the data created with any specialised
/// archetype will always be matched by queries based on the parent archetype" —
/// and AM master07 §Querying / §Supporting Archetype-based Querying, where the
/// matching set for a query naming X is X, its older minor/patch variants, its
/// specialisation parents and their older variants, bounded to one major version
/// (the interface-reference major boundary is hard). A specialisation child is
/// identified by extending the parent's `domain_concept` with a `-`-separated
/// segment (master10 §Design-time Relationships), so a query naming a parent
/// matches its own node plus every child whose `arch_concept` begins with
/// `concept-`, scoped to the same `qualified_rm_entity` + major. All parts
/// compare lowercased (master05 §"Composite Identifiers and Case"), served by
/// `idx_node_arch_subsume`.
///
/// Otherwise (at/id-codes, arbitrary strings, params resolving to non-HRIDs) the
/// predicate is the case-folded equality on `archetype`, served by the
/// `idx_node_archetype_lower` functional index — unchanged.
//
// NOTE: QUERY master03 §Archetype predicate literally equates the predicate
// to `archetype_node_id = '<literal>'` string equality; we implement the BASE/AM
// subsumption + interface-reference semantics instead, because a query naming a
// parent archetype MUST retrieve data created with its specialisation children
// (master10 §Design-time Relationships) — plain string equality never would.
// AOM2-era identifiers carry no lineage semantics in the `-` separator (AM
// master03 §"Legacy ADL 1.4 Semantics"), so full template-derived lineage
// matching (specialisation parents obtainable only from the operational template
// per master07 §Supporting Archetype-based Querying) is deferred to the ADL2
// phase; the `-`-prefix rule here is exact for the ADL 1.4-form ids this store
// holds (major-only `.vN`, lineage encoded directly in the concept).
pub(super) fn archetype_predicate(node: &str, value: &str) -> Expr {
    if let Ok(id) = value.parse::<ArchetypeId>()
        && let Ok(major) = id.major_version().parse::<i32>()
    {
        let entity = id.qualified_rm_entity().to_ascii_lowercase();
        let concept = id.domain_concept().to_ascii_lowercase();
        let child_prefix = format!("{}-%", like_escape(&concept));
        let concept_match =
            col(node, "arch_concept")
                .eq(Expr::val(concept))
                .or(col(node, "arch_concept").like(child_prefix));
        col(node, "arch_entity")
            .eq(Expr::val(entity))
            .and(col(node, "arch_major").eq(Expr::val(major)))
            .and(concept_match)
    } else {
        archetype_equality(node, value)
    }
}

/// Case-folded equality on the verbatim `archetype` column (the non-HRID
/// fallback: at/id-codes and arbitrary strings). BASE `base_types` master05
/// §"Composite Identifiers and Case".
fn archetype_equality(node: &str, value: &str) -> Expr {
    Expr::expr(Func::lower(col(node, "archetype"))).eq(Func::lower(Expr::val(value.to_owned())))
}

/// Escape the SQL `LIKE` metacharacters (`%`, `_`, `\`) in a literal prefix.
/// `domain_concept` segments are alphanumeric + `-` per the master05 grammar, so
/// this is defensive; the default `PostgreSQL` `LIKE` escape character `\` matches.
fn like_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

// ── test-only inline renderers ─────────────────────────────────────────────

/// Render the `archetype_node_id` predicate for `value` to inline `PostgreSQL`
/// text (values folded in, not parameterized) so tests can assert the emitted
/// condition.
#[cfg(test)]
pub(super) fn archetype_predicate_sql(value: &str) -> String {
    use sea_query::{PostgresQueryBuilder, Query};

    let cond = archetype_predicate("n", value);
    let mut q = Query::select();
    q.expr(Expr::val(1)).and_where(cond);
    q.to_string(PostgresQueryBuilder)
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::{aql_like_to_sql, archetype_predicate_sql};

    /// `\*` / `\?` in a LIKE pattern are the literal characters (§Operators/LIKE).
    #[test]
    fn like_escapes_are_literal() {
        assert_eq!(aql_like_to_sql("a*b"), "a%b");
        assert_eq!(aql_like_to_sql("a?b"), "a_b");
        assert_eq!(aql_like_to_sql(r"a\*b"), "a*b");
        assert_eq!(aql_like_to_sql(r"a\?b"), "a?b");
        assert_eq!(aql_like_to_sql("100%"), r"100\%");
    }

    /// The `archetype_node_id` predicate: a full archetype HRID lowers to the
    /// subsumption condition (parent + specialisation children, same entity +
    /// major); an at-code keeps the case-folded equality. BASE
    /// `architecture_overview` master10 §Design-time Relationships; AM master07
    /// §Querying.
    #[test]
    fn archetype_predicate_subsumption_sql() {
        // Parent archetype: matches its own concept OR any `laboratory-` child,
        // scoped to the same qualified RM entity + major.
        let parent = archetype_predicate_sql("openEHR-EHR-OBSERVATION.laboratory.v1");
        assert!(
            parent.contains(r#""n"."arch_entity" = 'openehr-ehr-observation'"#),
            "entity match (lowercased): {parent}"
        );
        assert!(
            parent.contains(r#""n"."arch_major" = 1"#),
            "major bound: {parent}"
        );
        assert!(
            parent.contains(r#""n"."arch_concept" = 'laboratory'"#),
            "exact concept match: {parent}"
        );
        assert!(
            parent.contains(r#""n"."arch_concept" LIKE 'laboratory-%'"#),
            "specialisation-child prefix match: {parent}"
        );
        assert!(
            !parent.contains("LOWER"),
            "the HRID path does not fall back to case-folded equality: {parent}"
        );

        // A specialisation child carries its full concept, so it prefix-matches its
        // own further children too.
        let child = archetype_predicate_sql("openEHR-EHR-OBSERVATION.laboratory-glucose.v1");
        assert!(
            child.contains(r#""n"."arch_concept" = 'laboratory-glucose'"#),
            "exact specialised concept: {child}"
        );
        assert!(
            child.contains(r#""n"."arch_concept" LIKE 'laboratory-glucose-%'"#),
            "further-specialisation prefix: {child}"
        );

        // An at-code is not a full HRID → case-folded equality on `archetype`, no
        // subsumption columns.
        let at_code = archetype_predicate_sql("at0001");
        assert!(
            at_code.contains("LOWER") && at_code.contains(r#""n"."archetype""#),
            "at-code keeps case-folded equality: {at_code}"
        );
        assert!(
            !at_code.contains("arch_entity"),
            "at-code does not use the subsumption columns: {at_code}"
        );
    }
}
