//! `TERMINOLOGY('expand', …)` `matches`-operand expansion (B4 stage (a)).
//!
//! master03 §TERMINOLOGY (`docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc`
//! lines 748–767) allows a `TERMINOLOGY()` call as — or *inside* — the
//! right-hand operand of `matches`, "for merging explicit codes with the
//! function results (in which case the AQL interpreter is responsible for
//! generating a valid list of codes during semantic analysis)":
//!
//! ```text
//! WHERE …/code_string matches TERMINOLOGY('expand', 'hl7.org/fhir/4.0', '<vs-url>')
//! WHERE …/code_string matches {'http://snomed.info/id/442031002',
//!                              TERMINOLOGY('expand', 'hl7.org/fhir/4.0', '<vs-url>')}
//! ```
//!
//! This module is the semantic-analysis pre-pass that realises that merge: it
//! resolves each `expand` call through a [`TerminologyExpander`] (the SM
//! `I_TERMINOLOGY_SERVICE` seam — the in-process `openehr-term` bundle by
//! default, a remote FHIR TS when configured) and rewrites the AST value list
//! *in place* to explicit string codes, **before** planning/SQL generation. The
//! engine planner ([`super::lower`]) then sees an ordinary `matches { … }` value
//! list and needs no terminology awareness.
//!
//! Staging (blueprint §3 build-order item 5): only `expand` is merged here
//! (stage (a)). Other `TERMINOLOGY()` forms are left untouched for the planner
//! to reject with a typed, spec-cited error — the Boolean value-expression form
//! (`TERMINOLOGY(…) = true`, stage (b)) and the bare-URI operand
//! (`matches { … }`, stage (c)) — see [`super::error::AqlFeatureError`].

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use openehr_query::ast::{
    IdentifiedExpr, MatchesOperand, Primitive, SelectQuery, TerminologyFunction, ValueListItem,
    WhereExpr,
};

use super::error::AqlError;

/// The only `TERMINOLOGY()` operation supported inside a `matches` operand
/// (case-insensitive): "expand: … retrieve all the codes contained in a value
/// set as an explicit set" (master03 §TERMINOLOGY).
const EXPAND: &str = "expand";

/// The resolver seam for `TERMINOLOGY('expand', …)`: turns a `(service_api,
/// params_uri)` pair into the explicit list of codes of the referenced value
/// set. Implemented by the application service over the SM
/// `I_TERMINOLOGY_SERVICE` providers.
#[async_trait]
pub trait TerminologyExpander: Send + Sync {
    /// Expand the value set identified by `params_uri` via the terminology
    /// service selected by `service_api`, returning its codes.
    ///
    /// # Errors
    ///
    /// * [`AqlError::Feature`] with `UnknownTerminologyService` — `service_api`
    ///   names no configured service; or `TerminologyValueSetNotFound` —
    ///   `params_uri` names no known value set (both → 400).
    /// * [`AqlError::Exec`] with `ExecError::Terminology` — the upstream
    ///   terminology server call failed (→ 500).
    async fn expand(&self, service_api: &str, params_uri: &str) -> Result<Vec<String>, AqlError>;
}

/// Resolve and merge every `TERMINOLOGY('expand', …)` used as (or inside) a
/// `matches` operand in `query`'s `WHERE` clause, rewriting the value lists in
/// place to explicit string codes. A no-op when the query has no `WHERE` clause
/// or no `expand` operand.
///
/// Each distinct `(service_api, params_uri)` is resolved exactly once. Non-
/// `expand` `TERMINOLOGY()` operands are left untouched (the planner rejects
/// them).
///
/// # Errors
///
/// Propagates the [`TerminologyExpander::expand`] error for the first operand
/// that fails to resolve.
pub async fn expand_matches(
    query: &mut SelectQuery,
    expander: &(impl TerminologyExpander + ?Sized),
) -> Result<(), AqlError> {
    let Some(where_) = query.where_.as_ref() else {
        return Ok(());
    };

    // Pass 1 (sync): collect the distinct `expand` requests keyed by
    // (service_api, params_uri).
    let mut requests: BTreeSet<(String, String)> = BTreeSet::new();
    collect(where_, &mut requests);
    if requests.is_empty() {
        return Ok(());
    }

    // Resolve each request exactly once (async, off the sync borrow).
    let mut resolved: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (service_api, params_uri) in requests {
        let codes = expander.expand(&service_api, &params_uri).await?;
        resolved.insert((service_api, params_uri), codes);
    }

    // Pass 2 (sync): splice the resolved codes into the value lists.
    if let Some(where_) = query.where_.as_mut() {
        rewrite(where_, &resolved);
    }
    Ok(())
}

/// Whether a `TERMINOLOGY()` call is an `expand` operation (case-insensitive).
fn is_expand(tf: &TerminologyFunction) -> bool {
    tf.operation.eq_ignore_ascii_case(EXPAND)
}

/// The resolution key for an `expand` call: `(service_api, params_uri)`.
fn key(tf: &TerminologyFunction) -> (String, String) {
    (tf.arg2.clone(), tf.arg3.clone())
}

/// Collect the distinct `expand` requests reachable from a `WHERE` sub-tree.
fn collect(expr: &WhereExpr, out: &mut BTreeSet<(String, String)>) {
    match expr {
        WhereExpr::Identified(IdentifiedExpr::Matches { operand, .. }) => match operand {
            MatchesOperand::Terminology(tf) if is_expand(tf) => {
                out.insert(key(tf));
            }
            MatchesOperand::ValueList(items) => {
                for tf in items.iter().filter_map(expand_item) {
                    out.insert(key(tf));
                }
            }
            MatchesOperand::Terminology(_) | MatchesOperand::Uri(_) => {}
        },
        WhereExpr::Identified(_) => {}
        WhereExpr::Not(w) => collect(w, out),
        WhereExpr::And(a, b) | WhereExpr::Or(a, b) => {
            collect(a, out);
            collect(b, out);
        }
    }
}

/// A value-list item that is an `expand` `TERMINOLOGY()` call, if it is one.
fn expand_item(item: &ValueListItem) -> Option<&TerminologyFunction> {
    match item {
        ValueListItem::Terminology(tf) if is_expand(tf) => Some(tf),
        _ => None,
    }
}

/// Rewrite the value lists of a `WHERE` sub-tree, replacing resolved `expand`
/// operands with explicit string codes.
fn rewrite(expr: &mut WhereExpr, resolved: &BTreeMap<(String, String), Vec<String>>) {
    match expr {
        WhereExpr::Identified(IdentifiedExpr::Matches { operand, .. }) => {
            rewrite_operand(operand, resolved);
        }
        WhereExpr::Identified(_) => {}
        WhereExpr::Not(w) => rewrite(w, resolved),
        WhereExpr::And(a, b) | WhereExpr::Or(a, b) => {
            rewrite(a, resolved);
            rewrite(b, resolved);
        }
    }
}

/// Rewrite a single `matches` operand: a standalone `expand` call becomes a
/// value list of its codes; a value list has each `expand` item spliced into
/// its codes (explicit codes and non-`expand` items preserved in order).
fn rewrite_operand(
    operand: &mut MatchesOperand,
    resolved: &BTreeMap<(String, String), Vec<String>>,
) {
    match operand {
        MatchesOperand::Terminology(tf) if is_expand(tf) => {
            *operand = MatchesOperand::ValueList(code_items(resolved.get(&key(tf))));
        }
        MatchesOperand::ValueList(items) => {
            let taken = std::mem::take(items);
            let mut rebuilt = Vec::with_capacity(taken.len());
            for item in taken {
                match expand_item(&item) {
                    Some(tf) => rebuilt.extend(code_items(resolved.get(&key(tf)))),
                    None => rebuilt.push(item),
                }
            }
            *items = rebuilt;
        }
        MatchesOperand::Terminology(_) | MatchesOperand::Uri(_) => {}
    }
}

/// The resolved codes as string-primitive value-list items (empty when the
/// expansion produced no codes — the `matches` then matches nothing).
fn code_items(codes: Option<&Vec<String>>) -> Vec<ValueListItem> {
    codes
        .into_iter()
        .flatten()
        .map(|c| ValueListItem::Primitive(Primitive::String(c.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_query::parser::parse_str;

    /// A canned expander: every `expand` returns a fixed code list.
    struct Fixed(Vec<String>);

    #[async_trait]
    impl TerminologyExpander for Fixed {
        async fn expand(&self, _api: &str, _uri: &str) -> Result<Vec<String>, AqlError> {
            Ok(self.0.clone())
        }
    }

    fn matches_values(q: &SelectQuery) -> Vec<String> {
        // Descend to the single WHERE matches operand and read its value list.
        let Some(WhereExpr::Identified(IdentifiedExpr::Matches { operand, .. })) =
            q.where_.as_ref()
        else {
            panic!("expected a WHERE matches");
        };
        let MatchesOperand::ValueList(items) = operand else {
            panic!("expected a value list after expansion, got {operand:?}");
        };
        items
            .iter()
            .map(|i| match i {
                ValueListItem::Primitive(Primitive::String(s)) => s.clone(),
                other => panic!("expected a string primitive, got {other:?}"),
            })
            .collect()
    }

    #[tokio::test]
    async fn standalone_expand_becomes_a_value_list() {
        let mut q = parse_str(
            "SELECT c FROM COMPOSITION c \
             WHERE c/category/defining_code/code_string \
             matches TERMINOLOGY('expand', 'openehr', 'audit_change_type')",
        )
        .expect("parse");
        expand_matches(&mut q, &Fixed(vec!["249".into(), "250".into()]))
            .await
            .expect("expand");
        assert_eq!(matches_values(&q), vec!["249".to_owned(), "250".to_owned()]);
    }

    #[tokio::test]
    async fn mixed_list_merges_explicit_and_expanded_codes_in_order() {
        let mut q = parse_str(
            "SELECT c FROM COMPOSITION c \
             WHERE c/category/defining_code/code_string \
             matches {'X', TERMINOLOGY('expand', 'openehr', 'g'), 'Y'}",
        )
        .expect("parse");
        expand_matches(&mut q, &Fixed(vec!["A".into(), "B".into()]))
            .await
            .expect("expand");
        assert_eq!(
            matches_values(&q),
            vec![
                "X".to_owned(),
                "A".to_owned(),
                "B".to_owned(),
                "Y".to_owned()
            ]
        );
    }

    #[tokio::test]
    async fn no_terminology_operand_is_a_no_op() {
        let src = "SELECT c FROM COMPOSITION c \
             WHERE c/category/defining_code/code_string matches {'249', '250'}";
        let mut q = parse_str(src).expect("parse");
        let before = q.clone();
        expand_matches(&mut q, &Fixed(vec!["ignored".into()]))
            .await
            .expect("no-op");
        assert_eq!(
            q, before,
            "a query with no expand operand must be untouched"
        );
    }

    #[tokio::test]
    async fn empty_expansion_yields_an_empty_value_list() {
        let mut q = parse_str(
            "SELECT c FROM COMPOSITION c \
             WHERE c/category/defining_code/code_string \
             matches TERMINOLOGY('expand', 'openehr', 'g')",
        )
        .expect("parse");
        expand_matches(&mut q, &Fixed(vec![]))
            .await
            .expect("expand");
        assert!(matches_values(&q).is_empty());
    }
}
