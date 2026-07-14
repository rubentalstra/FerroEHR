//! Unit tests for the AQL planner (path analysis + IR lowering). No DB.
//!
//! Queries are parsed by `openehr_query` and planned by [`super::plan`]; each
//! test asserts either the lowered IR shape (accepted constructs) or the exact
//! [`AqlError`] variant (rejected constructs), covering the design doc's feature
//! envelope. Several queries are taken verbatim from the QUERY 1.1 worked
//! examples (`docs/specs/openehr/QUERY/docs/AQL/`).

use openehr_query::lexer::CompOp;
use openehr_query::parser::parse_str;

use super::error::{AnalysisError, AqlError, AqlFeatureError};
use super::ir::{
    AggFunc, Coercion, ContainsTree, EhrField, Expr, LeafPath, Link, Operand, PathTarget, QueryIr,
    ScalarFn, SelectValue, Source, TypedLit, VersionField, VersionScope,
};
use super::{ParamValue, Params, SqlCtx, plan};

// ── helpers ──────────────────────────────────────────────────────────────────

fn plan_ok(q: &str) -> QueryIr {
    let ast = parse_str(q).unwrap_or_else(|e| panic!("parse failed for {q:?}: {e}"));
    plan(&ast, &Params::new()).unwrap_or_else(|e| panic!("plan failed for {q:?}: {e}"))
}

/// Plan `q` (parameterless) and lower it to SQL, returning the generated SQL
/// text. Literals bind as `$n` placeholders; structural SQL (EXISTS, OR, CASE …)
/// is asserted directly against the text — the same technique as
/// [`super::sql::archetype_predicate_sql_for_tests`].
fn build_sql(q: &str) -> String {
    let ir = plan_ok(q);
    let ctx = SqlCtx {
        system_id: "sys.example.com".to_owned(),
        ehr_ids: Vec::new(),
        subject_scope: None,
        limit: None,
        offset: None,
    };
    super::sql::build(&ir, &Params::new(), &ctx)
        .unwrap_or_else(|e| panic!("SQL build failed for {q:?}: {e}"))
        .sql
}

fn plan_with(q: &str, params: &Params) -> Result<QueryIr, AqlError> {
    let ast = parse_str(q).unwrap_or_else(|e| panic!("parse failed for {q:?}: {e}"));
    plan(&ast, params)
}

fn plan_err(q: &str) -> AqlError {
    let ast = parse_str(q).unwrap_or_else(|e| panic!("parse failed for {q:?}: {e}"));
    plan(&ast, &Params::new()).expect_err(&format!("expected planning to fail for {q:?}"))
}

fn data_leaf(value: &SelectValue) -> &LeafPath {
    match value {
        SelectValue::Path(PathTarget::Data(l)) => l,
        other => panic!("expected a data-path select value, got {other:?}"),
    }
}

fn anchor_attrs(leaf: &LeafPath) -> Vec<&str> {
    leaf.anchor.iter().map(|s| s.attribute.as_str()).collect()
}

fn fragment_names(leaf: &LeafPath) -> Vec<&str> {
    leaf.fragment.iter().map(|s| s.name.as_str()).collect()
}

// ── path-split table ───────────────────────────────────────────────────────

/// The design doc's canonical bp.v1 split (rooted at the COMPOSITION var,
/// descending through `content`): anchor = the structure hops down to the
/// ELEMENT node, fragment = `value/magnitude`.
#[test]
fn path_split_bp_example() {
    let ir = plan_ok(
        "SELECT c/content[openEHR-EHR-OBSERVATION.bp.v1]/data[at0001]/events[at0006]\
         /data/items[at0004]/value/magnitude \
         FROM EHR e CONTAINS COMPOSITION c[openEHR-EHR-COMPOSITION.report.v1]",
    );
    let leaf = data_leaf(&ir.select[0].value);
    assert_eq!(
        anchor_attrs(leaf),
        ["content", "data", "events", "data", "items"],
        "structure hops down to the ELEMENT node"
    );
    assert_eq!(fragment_names(leaf), ["value", "magnitude"]);
    assert_eq!(leaf.coercion, Coercion::Magnitude, "magnitude is numeric");
    assert!(leaf.multi_valued, "content/events/items are list-valued");
    // The leaf resolves to the numeric primitive(s) behind `magnitude`.
    assert!(leaf.types.contains("Real") || leaf.types.contains("Integer"));
}

/// A path-split table: query path → (anchor attrs, fragment names, coercion).
#[test]
fn path_split_table() {
    struct Case {
        select: &'static str,
        anchor: &'static [&'static str],
        fragment: &'static [&'static str],
        coercion: Coercion,
    }
    let from = "FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1]";
    let cases = [
        // whole-object select of the source node: no hops, no fragment.
        Case {
            select: "o",
            anchor: &[],
            fragment: &[],
            coercion: Coercion::Raw,
        },
        // a fragment-only leaf: name/value lives in the OBSERVATION node itself.
        Case {
            select: "o/name/value",
            anchor: &[],
            fragment: &["name", "value"],
            coercion: Coercion::Text,
        },
        // numeric magnitude via structure hops then a numeric-primitive fragment.
        Case {
            select: "o/data[at0001]/events[at0006]/data/items[at0004]/value/magnitude",
            anchor: &["data", "events", "data", "items"],
            fragment: &["value", "magnitude"],
            coercion: Coercion::Magnitude,
        },
        // the whole polymorphic DV value: fragment starts at `value`, mixed
        // candidate types ⇒ guarded Raw.
        Case {
            select: "o/data[at0001]/events[at0006]/data/items[at0004]/value",
            anchor: &["data", "events", "data", "items"],
            fragment: &["value"],
            coercion: Coercion::Raw,
        },
    ];
    for c in cases {
        let ir = plan_ok(&format!("SELECT {} {from}", c.select));
        let leaf = data_leaf(&ir.select[0].value);
        assert_eq!(anchor_attrs(leaf), c.anchor, "anchor for {}", c.select);
        assert_eq!(
            fragment_names(leaf),
            c.fragment,
            "fragment for {}",
            c.select
        );
        assert_eq!(leaf.coercion, c.coercion, "coercion for {}", c.select);
    }
}

#[test]
fn temporal_leaf_coercion() {
    // POINT_EVENT.time is a DV_DATE_TIME → Temporal.
    let ir = plan_ok(
        "SELECT o/data[at0001]/events[at0006]/time/value \
         FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1]",
    );
    let leaf = data_leaf(&ir.select[0].value);
    assert_eq!(anchor_attrs(leaf), ["data", "events"]);
    assert_eq!(fragment_names(leaf), ["time", "value"]);
    assert_eq!(leaf.coercion, Coercion::Temporal);
}

// ── FROM / CONTAINS trees ─────────────────────────────────────────────────────

#[test]
fn simple_ehr_contains_composition() {
    let ir = plan_ok("SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c");
    assert_eq!(ir.sources.len(), 2);
    assert!(matches!(ir.sources[0], Source::Ehr(_)));
    assert!(matches!(ir.sources[1], Source::Rm(_)));
    match &ir.contains {
        ContainsTree::Operand {
            source,
            contained: Some(inner),
        } => {
            assert_eq!(source.0, 0);
            assert_eq!(inner.link, Link::Contains);
            assert!(matches!(inner.tree, ContainsTree::Operand { .. }));
        }
        other => panic!("unexpected contains tree: {other:?}"),
    }
}

#[test]
fn contains_or_tree() {
    let ir = plan_ok(
        "SELECT e/ehr_id/value FROM EHR e CONTAINS \
         (OBSERVATION o1[openEHR-EHR-OBSERVATION.lab.v1] \
          OR OBSERVATION o2[openEHR-EHR-OBSERVATION.glucose.v1])",
    );
    let ContainsTree::Operand {
        contained: Some(inner),
        ..
    } = &ir.contains
    else {
        panic!("expected EHR operand with a contained subtree");
    };
    assert!(matches!(inner.tree, ContainsTree::Or(_, _)));
}

#[test]
fn ehr_contains_two_vos_and() {
    let ir = plan_ok(
        "SELECT e/ehr_id/value FROM EHR e CONTAINS \
         (COMPOSITION c1[openEHR-EHR-COMPOSITION.referral.v1] \
          AND COMPOSITION c2[openEHR-EHR-COMPOSITION.report.v1])",
    );
    assert_eq!(ir.sources.len(), 3);
    let ContainsTree::Operand {
        contained: Some(inner),
        ..
    } = &ir.contains
    else {
        panic!("expected EHR operand");
    };
    assert!(matches!(inner.tree, ContainsTree::And(_, _)));
}

#[test]
fn not_contains_is_recorded() {
    let ir = plan_ok(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         COMPOSITION c[openEHR-EHR-COMPOSITION.referral.v1] \
         NOT CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.lab.v1]",
    );
    // e -> c, then c NOT CONTAINS o.
    let ContainsTree::Operand {
        contained: Some(ehr_child),
        ..
    } = &ir.contains
    else {
        panic!("expected EHR operand");
    };
    let ContainsTree::Operand {
        contained: Some(comp_child),
        ..
    } = &ehr_child.tree
    else {
        panic!("expected COMPOSITION operand");
    };
    assert_eq!(comp_child.link, Link::NotContains);
}

// ── version scoping ───────────────────────────────────────────────────────────

fn rm_scope(ir: &QueryIr, idx: usize) -> &VersionScope {
    match &ir.sources[idx] {
        Source::Rm(s) => &s.scope,
        other => panic!("source {idx} is not an RM source: {other:?}"),
    }
}

#[test]
fn version_latest_is_default() {
    let ir = plan_ok("SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c");
    assert_eq!(*rm_scope(&ir, 1), VersionScope::Latest);
}

#[test]
fn version_all_versions_scopes_contained_vo() {
    let ir = plan_ok(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         VERSION v[ALL_VERSIONS] CONTAINS COMPOSITION c",
    );
    // sources: EHR(0), VERSION(1), COMPOSITION(2); the COMPOSITION inherits ALL.
    assert!(matches!(&ir.sources[1], Source::Version(s) if s.scope == VersionScope::All));
    assert_eq!(*rm_scope(&ir, 2), VersionScope::All);
}

#[test]
fn version_latest_explicit() {
    let ir = plan_ok(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         VERSION v[LATEST_VERSION] CONTAINS COMPOSITION c",
    );
    assert_eq!(*rm_scope(&ir, 2), VersionScope::Latest);
}

#[test]
fn version_at_time_standard_predicate() {
    let ir = plan_ok(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         VERSION v[commit_audit/time_committed >= '2021-01-01T00:00:00'] \
         CONTAINS COMPOSITION c",
    );
    let Source::Version(v) = &ir.sources[1] else {
        panic!("expected a VERSION source");
    };
    match &v.scope {
        VersionScope::Predicate(p) => {
            assert_eq!(p.field, VersionField::TimeCommitted);
            assert_eq!(p.op, CompOp::Ge);
        }
        other => panic!("expected a version predicate scope, got {other:?}"),
    }
    assert!(v.scope.is_at_time(), "time_committed comparison is at-time");
    // and it propagates to the contained COMPOSITION.
    assert!(matches!(rm_scope(&ir, 2), VersionScope::Predicate(_)));
}

#[test]
fn version_field_in_select() {
    let ir = plan_ok(
        "SELECT v/commit_audit/time_committed FROM EHR e CONTAINS \
         VERSION v CONTAINS COMPOSITION c",
    );
    assert!(matches!(
        &ir.select[0].value,
        SelectValue::Path(PathTarget::Version {
            field: VersionField::TimeCommitted,
            ..
        })
    ));
}

// ── SELECT / DISTINCT / aggregates / TOP / LIMIT ──────────────────────────────

#[test]
fn distinct_and_alias() {
    let ir = plan_ok("SELECT DISTINCT c/name/value AS cname FROM EHR e CONTAINS COMPOSITION c");
    assert!(ir.distinct);
    assert_eq!(ir.select[0].alias.as_deref(), Some("cname"));
}

#[test]
fn aggregates() {
    let ir = plan_ok("SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c");
    assert!(matches!(
        &ir.select[0].value,
        SelectValue::Aggregate {
            func: AggFunc::Count,
            arg: None,
            distinct: false
        }
    ));

    let ir = plan_ok("SELECT COUNT(DISTINCT c/uid/value) FROM EHR e CONTAINS COMPOSITION c");
    assert!(matches!(
        &ir.select[0].value,
        SelectValue::Aggregate {
            func: AggFunc::Count,
            arg: Some(_),
            distinct: true
        }
    ));

    let ir = plan_ok(
        "SELECT MAX(o/data[at0001]/events[at0006]/data/items[at0004]/value/magnitude) \
         FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1]",
    );
    assert!(matches!(
        &ir.select[0].value,
        SelectValue::Aggregate {
            func: AggFunc::Max,
            ..
        }
    ));
}

#[test]
fn top_maps_to_limit() {
    let ir = plan_ok("SELECT TOP 5 c/uid/value FROM EHR e CONTAINS COMPOSITION c");
    assert_eq!(ir.limit, Some(5));
    assert_eq!(ir.offset, None);
}

#[test]
fn limit_and_offset() {
    let ir = plan_ok("SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c LIMIT 10 OFFSET 20");
    assert_eq!(ir.limit, Some(10));
    assert_eq!(ir.offset, Some(20));
}

#[test]
fn top_with_limit_rejected() {
    let err = plan_err("SELECT TOP 5 c/uid/value FROM EHR e CONTAINS COMPOSITION c LIMIT 10");
    assert!(matches!(
        err,
        AqlError::Feature(AqlFeatureError::TopWithLimit)
    ));
}

// ── WHERE ──────────────────────────────────────────────────────────────────

#[test]
fn where_comparison_magnitude() {
    let ir = plan_ok(
        "SELECT o/data[at0001]/events[at0006]/data/items[at0004]/value/magnitude \
         FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1] \
         WHERE o/data[at0001]/events[at0006]/data/items[at0004]/value/magnitude >= 140",
    );
    match ir.filter.as_ref().unwrap() {
        Expr::Compare {
            op,
            coercion,
            lhs,
            rhs,
        } => {
            assert_eq!(*op, CompOp::Ge);
            assert_eq!(*coercion, Coercion::Magnitude);
            assert!(matches!(lhs, Operand::Path(PathTarget::Data(_))));
            assert!(matches!(rhs, Operand::Literal(TypedLit::Integer(140))));
        }
        other => panic!("expected a comparison, got {other:?}"),
    }
}

#[test]
fn where_exists_like_matches_and_boolean_tree() {
    let ir = plan_ok(
        "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c \
         WHERE EXISTS c/context/other_context \
           AND c/name/value LIKE 'left *' \
           OR c/archetype_details/template_id/value MATCHES {'t1', 't2'}",
    );
    let filter = ir.filter.as_ref().unwrap();
    // top level is an OR (AND binds tighter in the parser).
    let Expr::Or(lhs, rhs) = filter else {
        panic!("expected top-level OR, got {filter:?}");
    };
    assert!(matches!(lhs.as_ref(), Expr::And(_, _)));
    let Expr::Matches { values, .. } = rhs.as_ref() else {
        panic!("expected MATCHES on the right");
    };
    assert_eq!(values.len(), 2);
}

#[test]
fn temporal_literal_retyped_from_context() {
    // POINT_EVENT.time/value is temporal → the quoted RHS is retyped Temporal.
    let ir = plan_ok(
        "SELECT o/name/value FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1] \
         WHERE o/data[at0001]/events[at0006]/time/value >= '2021-01-01T00:00:00'",
    );
    match ir.filter.as_ref().unwrap() {
        Expr::Compare { rhs, coercion, .. } => {
            assert_eq!(*coercion, Coercion::Temporal);
            assert!(matches!(rhs, Operand::Literal(TypedLit::Temporal(_))));
        }
        other => panic!("expected a comparison, got {other:?}"),
    }
}

// ── parameters ─────────────────────────────────────────────────────────────

#[test]
fn parameters_are_collected_and_validated() {
    let q = "SELECT c/uid/value FROM EHR e[ehr_id/value=$ehr] CONTAINS COMPOSITION c \
             WHERE c/name/value = $cname";
    // both bound → ok, params sorted+unique.
    let params = Params::new()
        .with("ehr", ParamValue::Str("1234".into()))
        .with("cname", ParamValue::Str("BP".into()));
    let ir = plan_with(q, &params).unwrap();
    assert_eq!(ir.params, vec!["cname".to_string(), "ehr".to_string()]);
}

#[test]
fn missing_parameter_is_rejected() {
    let q = "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c WHERE c/name/value = $cname";
    let err = plan_with(q, &Params::new()).unwrap_err();
    assert!(matches!(
        err,
        AqlError::Analysis(AnalysisError::UnboundParameter(p)) if p == "cname"
    ));
}

#[test]
fn ehr_id_predicate_lowers_to_field() {
    let ir = plan_with(
        "SELECT e/ehr_id/value FROM EHR e[ehr_id/value=$id] CONTAINS COMPOSITION c",
        &Params::new().with("id", ParamValue::Str("x".into())),
    )
    .unwrap();
    let Source::Ehr(ehr) = &ir.sources[0] else {
        panic!("expected EHR source");
    };
    assert_eq!(ehr.predicates.len(), 1);
    assert_eq!(ehr.predicates[0].field, EhrField::EhrId);
}

// ── rejections (exact variants) ───────────────────────────────────────────────

#[test]
fn terminology_function_rejected() {
    let err = plan_err(
        "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c \
         WHERE c/category/defining_code/code_string \
           MATCHES TERMINOLOGY('expand', 'hl7.org/fhir/4.0', 'http://snomed.info/sct')",
    );
    assert!(matches!(
        err,
        AqlError::Feature(AqlFeatureError::MatchesTerminology)
    ));
}

#[test]
fn matches_uri_rejected() {
    let err = plan_err(
        "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c \
         WHERE c/category/defining_code/code_string MATCHES {http://snomed.info/id/442031002}",
    );
    assert!(matches!(
        err,
        AqlError::Feature(AqlFeatureError::MatchesUri)
    ));
}

#[test]
fn demographic_source_rejected() {
    let err = plan_err("SELECT p/uid/value FROM EHR e CONTAINS PERSON p");
    assert!(matches!(
        err,
        AqlError::Feature(AqlFeatureError::UnsupportedSourceClass(c)) if c == "PERSON"
    ));
}

#[test]
fn branch_version_id_rejected() {
    let err = plan_err(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         VERSION v[uid/value='abc::sys.example.com::1.1.1'] CONTAINS COMPOSITION c",
    );
    assert!(matches!(
        err,
        AqlError::Feature(AqlFeatureError::BranchVersionAddressing)
    ));
}

#[test]
fn unknown_class_rejected() {
    let err = plan_err("SELECT x/uid/value FROM EHR e CONTAINS NOTATYPE x");
    assert!(matches!(
        err,
        AqlError::Analysis(AnalysisError::UnknownClass(c)) if c == "NOTATYPE"
    ));
}

#[test]
fn unknown_variable_rejected() {
    let err = plan_err("SELECT zzz/uid/value FROM EHR e CONTAINS COMPOSITION c");
    assert!(matches!(
        err,
        AqlError::Analysis(AnalysisError::UnknownVariable(v)) if v == "zzz"
    ));
}

#[test]
fn unresolvable_attribute_rejected() {
    let err = plan_err("SELECT c/not_an_attr FROM EHR e CONTAINS COMPOSITION c");
    assert!(matches!(
        err,
        AqlError::Analysis(AnalysisError::UnresolvableAttribute { attribute, .. })
            if attribute == "not_an_attr"
    ));
}

#[test]
fn unsupported_function_rejected() {
    let err = plan_err("SELECT frobnicate(c/uid/value) FROM EHR e CONTAINS COMPOSITION c");
    assert!(matches!(
        err,
        AqlError::Feature(AqlFeatureError::UnsupportedFunction(f)) if f == "frobnicate"
    ));
}

#[test]
fn whitelisted_function_accepted() {
    let ir = plan_ok("SELECT length(c/name/value) FROM EHR e CONTAINS COMPOSITION c");
    assert!(matches!(
        &ir.select[0].value,
        SelectValue::Function {
            func: ScalarFn::Length,
            ..
        }
    ));
}

// ── chapter-16 audit additions (QUERY master03) ──────────────────────────────

/// Variable names must be unique within an AQL statement (§Variables/Syntax).
#[test]
fn duplicate_variable_rejected() {
    let e = plan_err("SELECT c FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION c");
    assert!(matches!(
        e,
        AqlError::Analysis(AnalysisError::DuplicateVariable(v)) if v == "c"
    ));
}

/// Variable names are not case-sensitive (§Variables/Syntax) — both for the
/// uniqueness check and for reference resolution.
#[test]
fn variable_names_fold_case() {
    let e = plan_err("SELECT c FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION C");
    assert!(matches!(
        e,
        AqlError::Analysis(AnalysisError::DuplicateVariable(_))
    ));
    // A reference differing only by case resolves.
    plan_ok("SELECT C/name/value FROM EHR e CONTAINS COMPOSITION c");
}

/// `row_count` minimum is 1, `offset` minimum is 0 (§LIMIT/Syntax).
#[test]
fn limit_zero_rejected() {
    let e = plan_err("SELECT c FROM COMPOSITION c LIMIT 0");
    assert!(matches!(
        e,
        AqlError::Analysis(AnalysisError::PagingBounds {
            clause: "LIMIT",
            ..
        })
    ));
}

/// SUM/AVG accept Integer/Real input only (§Functions/SUM, AVG) — a textual
/// leaf is a typed reject.
#[test]
fn sum_over_textual_leaf_rejected() {
    let e = plan_err("SELECT SUM(c/name/value) FROM COMPOSITION c");
    assert!(matches!(
        e,
        AqlError::Analysis(AnalysisError::AggregateInputType { func: "SUM", .. })
    ));
}

/// Scalar-function arity is validated at lowering (§Functions).
#[test]
fn function_arity_enforced() {
    let e = plan_err("SELECT length(c/name/value, c/name/value) FROM COMPOSITION c");
    assert!(matches!(
        e,
        AqlError::Analysis(AnalysisError::FunctionArity { func: "LENGTH", .. })
    ));
    let e = plan_err("SELECT current_date(c/name/value) FROM COMPOSITION c");
    assert!(matches!(
        e,
        AqlError::Analysis(AnalysisError::FunctionArity {
            func: "CURRENT_DATE",
            ..
        })
    ));
}

/// The whole single-row function set plans (§Functions: string, numeric,
/// date/time incl. `CURRENT_TIMEZONE`, and the string function `CONTAINS`).
#[test]
fn full_scalar_function_set_plans() {
    for q in [
        "SELECT substring(c/name/value, 1, 3) FROM COMPOSITION c",
        "SELECT position('x', c/name/value) FROM COMPOSITION c",
        "SELECT contains(c/name/value, 'x') FROM COMPOSITION c",
        "SELECT concat(c/name/value, '!') FROM COMPOSITION c",
        "SELECT concat_ws('-', c/name/value, 'x') FROM COMPOSITION c",
        "SELECT abs(o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude) \
         FROM OBSERVATION o",
        "SELECT round(o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude) \
         FROM OBSERVATION o",
        "SELECT current_date() FROM COMPOSITION c",
        "SELECT current_timezone() FROM COMPOSITION c",
        "SELECT now() FROM COMPOSITION c",
    ] {
        plan_ok(q);
    }
}

/// `\*` / `\?` in a LIKE pattern are the literal characters (§Operators/LIKE).
#[test]
fn like_escapes_are_literal() {
    assert_eq!(super::sql::aql_like_to_sql_for_tests("a*b"), "a%b");
    assert_eq!(super::sql::aql_like_to_sql_for_tests("a?b"), "a_b");
    assert_eq!(super::sql::aql_like_to_sql_for_tests(r"a\*b"), "a*b");
    assert_eq!(super::sql::aql_like_to_sql_for_tests(r"a\?b"), "a?b");
    assert_eq!(super::sql::aql_like_to_sql_for_tests("100%"), r"100\%");
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
    let parent =
        super::sql::archetype_predicate_sql_for_tests("openEHR-EHR-OBSERVATION.laboratory.v1");
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
    let child = super::sql::archetype_predicate_sql_for_tests(
        "openEHR-EHR-OBSERVATION.laboratory-glucose.v1",
    );
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
    let at_code = super::sql::archetype_predicate_sql_for_tests("at0001");
    assert!(
        at_code.contains("LOWER") && at_code.contains(r#""n"."archetype""#),
        "at-code keeps case-folded equality: {at_code}"
    );
    assert!(
        !at_code.contains("arch_entity"),
        "at-code does not use the subsumption columns: {at_code}"
    );
}

// ── SQL lowering: the G-row fixes (QUERY master03) ────────────────────────────

/// G-01: OR-containment under an EHR lowers to a disjunction of correlated
/// `EXISTS` subqueries (QUERY master03 §Containment — "Logical operators AND and
/// OR"). Previously `sql.rs` returned `SqlError::Unsupported` for any `OR` in the
/// FROM tree; it must now build.
#[test]
fn or_contains_under_ehr_lowers_to_disjunctive_exists() {
    let sql = build_sql(
        "SELECT e/ehr_id/value FROM EHR e CONTAINS \
         (OBSERVATION o1[openEHR-EHR-OBSERVATION.lab.v1] \
          OR OBSERVATION o2[openEHR-EHR-OBSERVATION.glucose.v1])",
    );
    assert_eq!(
        sql.matches("EXISTS").count(),
        2,
        "one EXISTS per OR branch: {sql}"
    );
    assert!(sql.contains(" OR "), "the branches are OR-combined: {sql}");
}

/// G-01: OR-containment under a COMPOSITION shares the parent VO and
/// interval-anchors each branch inside its node subtree (`num BETWEEN`).
#[test]
fn or_contains_under_vo_interval_anchors_each_branch() {
    let sql = build_sql(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         COMPOSITION c[openEHR-EHR-COMPOSITION.report.v1] CONTAINS \
         (OBSERVATION o[openEHR-EHR-OBSERVATION.lab.v1] \
          OR OBSERVATION o1[openEHR-EHR-OBSERVATION.glucose.v1])",
    );
    assert_eq!(sql.matches("EXISTS").count(), 2, "{sql}");
    assert!(sql.contains(" OR "), "{sql}");
    assert!(
        sql.contains("BETWEEN"),
        "each branch is interval-anchored in the parent VO: {sql}"
    );
}

/// G-01: a nested AND/OR containment tree lowers to the matching boolean tree of
/// `EXISTS` filters (QUERY master03 §Containment).
#[test]
fn nested_and_or_contains_tree_builds() {
    let sql = build_sql(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         COMPOSITION c[openEHR-EHR-COMPOSITION.report.v1] CONTAINS \
         (OBSERVATION o1[openEHR-EHR-OBSERVATION.lab.v1] \
          OR (OBSERVATION o2[openEHR-EHR-OBSERVATION.glucose.v1] \
              AND OBSERVATION o3[openEHR-EHR-OBSERVATION.bp.v1]))",
    );
    assert_eq!(
        sql.matches("EXISTS").count(),
        3,
        "one EXISTS per operand: {sql}"
    );
    assert!(sql.contains(" OR ") && sql.contains(" AND "), "{sql}");
}

/// G-08: NOT CONTAINS generalises to a negated `EXISTS` over an arbitrary
/// operand tree — a compound (OR) operand now builds (previously
/// `SqlError::Unsupported`). QUERY master03 §Containment, §NOT.
#[test]
fn not_contains_compound_operand_builds() {
    let sql = build_sql(
        "SELECT c/uid/value FROM EHR e CONTAINS \
         COMPOSITION c[openEHR-EHR-COMPOSITION.referral.v1] NOT CONTAINS \
         (OBSERVATION o1[openEHR-EHR-OBSERVATION.lab.v1] \
          OR OBSERVATION o2[openEHR-EHR-OBSERVATION.glucose.v1])",
    );
    assert!(sql.contains("NOT"), "the exclusion is negated: {sql}");
    assert_eq!(sql.matches("EXISTS").count(), 2, "{sql}");
}

/// G-12: a mixed-type (`Raw`) leaf compared to a numeric literal extracts
/// numerically with a `jsonb_typeof` guard (non-number occurrences → NULL),
/// never a lexical text compare (QUERY master03 §Comparison operators).
#[test]
fn raw_leaf_numeric_comparison_dispatches_to_numeric() {
    let sql = build_sql(
        "SELECT o/name/value FROM EHR e CONTAINS \
         OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1] \
         WHERE o/data[at0001]/events[at0006]/data/items[at0004]/value >= 140",
    );
    assert!(
        sql.contains("jsonb_typeof") && sql.contains("'number'"),
        "the polymorphic value leaf is compared numerically under a type guard: {sql}"
    );
}

/// G-12: the same mixed-type leaf compared to a *string* literal stays on the
/// text path (no numeric guard).
#[test]
fn raw_leaf_text_comparison_stays_text() {
    let sql = build_sql(
        "SELECT o/name/value FROM EHR e CONTAINS \
         OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1] \
         WHERE o/data[at0001]/events[at0006]/data/items[at0004]/value = 'x'",
    );
    assert!(
        !sql.contains("jsonb_typeof"),
        "a text comparison must not use the numeric guard: {sql}"
    );
}

/// G-15: MIN/MAX over a String leaf compare textually, not by forced numeric
/// magnitude — "Input values type should be either String, Date, Time, Integer
/// or Real, and it will also determine the return type" (QUERY master03 §MAX).
#[test]
fn min_max_over_text_leaf_is_not_forced_numeric() {
    let sql = build_sql("SELECT MAX(c/name/value) FROM COMPOSITION c");
    assert!(
        !sql.contains("numeric") && !sql.contains("openehr_magnitude"),
        "MAX over a text leaf is not coerced numeric: {sql}"
    );
}

/// G-15: MIN/MAX over a numeric leaf still lowers numerically.
#[test]
fn min_max_over_numeric_leaf_is_numeric() {
    let sql = build_sql(
        "SELECT MAX(o/data[at0001]/events[at0006]/data/items[at0004]/value/magnitude) \
         FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1]",
    );
    assert!(
        sql.contains("numeric"),
        "MAX over a numeric leaf casts to numeric: {sql}"
    );
}

// ── P20: promoted context_start fast path + F6 uid synthesis ─────────────────

/// The patient-dashboard shape orders by the promoted `node.context_start`
/// column, not the correlated `EVENT_CONTEXT` extraction + `::timestamptz` cast
/// (P20; docs/plans/phase-20-optimization.md).
#[test]
fn dashboard_order_by_uses_context_start_column() {
    let sql = build_sql(
        "SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c \
         ORDER BY c/context/start_time/value DESC",
    );
    assert!(
        sql.contains(r#""context_start" DESC"#),
        "ORDER BY reads the promoted column: {sql}"
    );
    // The ordering no longer casts an extracted EVENT_CONTEXT value to timestamptz.
    assert!(
        !sql.contains("timestamptz"),
        "no per-row ::timestamptz cast for the ordering: {sql}"
    );
}

/// The same context start-time leaf in a temporal WHERE comparison also reads
/// the promoted column (the fast path lives in the shared value lowering, so it
/// covers ORDER BY, WHERE, and aggregation uniformly).
#[test]
fn where_temporal_compare_uses_context_start_column() {
    let sql = build_sql(
        "SELECT c/name/value FROM EHR e CONTAINS COMPOSITION c \
         WHERE c/context/start_time/value > '2021-01-01T00:00:00Z'",
    );
    assert!(
        sql.contains(r#""context_start""#),
        "WHERE comparison reads the promoted column: {sql}"
    );
}

/// A near-miss must fall back to the general lowering: a temporal leaf on an
/// OBSERVATION (not a promoted type) still extracts through the subtree.
#[test]
fn non_promoted_temporal_leaf_falls_back_to_subquery() {
    let sql = build_sql(
        "SELECT o/name/value \
         FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1] \
         ORDER BY o/data[at0001]/events[at0006]/time/value DESC",
    );
    assert!(
        !sql.contains("context_start"),
        "OBSERVATION time is not the promoted composition leaf: {sql}"
    );
    assert!(
        sql.contains("jsonb_path_query_first") && sql.contains("timestamptz"),
        "the general temporal lowering (extraction + ::timestamptz) is used: {sql}"
    );
}

/// A near-miss on the promoted type but a different path (`end_time`, not the
/// registered `start_time`) also falls back — the match is path-exact.
#[test]
fn other_composition_context_path_falls_back() {
    let sql = build_sql(
        "SELECT c/name/value FROM EHR e CONTAINS COMPOSITION c \
         ORDER BY c/context/end_time/value DESC",
    );
    assert!(
        !sql.contains("context_start"),
        "only the registered context/start_time/value promotes: {sql}"
    );
}

/// Projection of the context start-time reads the canonical JSON fragment, not
/// the timestamptz column — the fast path is comparison/ordering only, so the
/// `RESULT_SET` cell keeps the verbatim `DV_DATE_TIME` value.
#[test]
fn projection_of_context_start_time_is_not_promoted() {
    let sql = build_sql("SELECT c/context/start_time/value FROM EHR e CONTAINS COMPOSITION c");
    assert!(
        !sql.contains("context_start"),
        "projection must extract the canonical value, not the timestamptz column: {sql}"
    );
    assert!(
        sql.contains("jsonb_path_query_first"),
        "projection extracts the leaf from the fragment: {sql}"
    );
}

/// F6: `c/uid/value` on a COMPOSITION variable synthesizes the `OBJECT_VERSION_ID`
/// from the joined `vo_version` (RM common master06 §Version Identification) —
/// it is not stored in the fragment.
#[test]
fn composition_uid_value_is_synthesized_from_vo_version() {
    let sql = build_sql("SELECT c/uid/value FROM EHR e CONTAINS COMPOSITION c");
    assert!(
        sql.contains("creating_system_id"),
        "uid/value is composed from vo_version columns: {sql}"
    );
}

/// F6: `c/uid` yields the `OBJECT_VERSION_ID` object cell.
#[test]
fn composition_uid_object_is_built() {
    let sql = build_sql("SELECT c/uid FROM EHR e CONTAINS COMPOSITION c");
    assert!(
        sql.contains("jsonb_build_object"),
        "uid projects the OBJECT_VERSION_ID object: {sql}"
    );
    assert!(
        sql.contains("creating_system_id"),
        "the object's value is the synthesized version id: {sql}"
    );
}

/// F6: a contained object's own `uid` is NOT synthesized — only a
/// versioned-object root gets a server-assigned `OBJECT_VERSION_ID`; the
/// OBSERVATION's uid falls through to the stored fragment.
#[test]
fn contained_object_uid_is_not_synthesized() {
    let sql = build_sql(
        "SELECT o/uid/value \
         FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.bp.v1]",
    );
    assert!(
        sql.contains("jsonb_path_query_first"),
        "the observation uid is read from its stored fragment: {sql}"
    );
}
