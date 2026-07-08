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
use super::{ParamValue, Params, plan};

// ── helpers ──────────────────────────────────────────────────────────────────

fn plan_ok(q: &str) -> QueryIr {
    let ast = parse_str(q).unwrap_or_else(|e| panic!("parse failed for {q:?}: {e}"));
    plan(&ast, &Params::new()).unwrap_or_else(|e| panic!("plan failed for {q:?}: {e}"))
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
