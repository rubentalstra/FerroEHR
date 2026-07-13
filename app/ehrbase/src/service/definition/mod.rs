//! The DEFINITION component of the platform crate — the openEHR **Definition**
//! service seam (SM `master04-definition_package.adoc`).
//!
//! Layout mirrors the SM interface set one file per interface, each carrying
//! its domain logic **and** its `impl <Interface>Service for EhrbaseService`:
//!
//! - [`adl14`] — `I_DEFINITION_ADL14` (`i_definition_adl14.adoc`): ADL 1.4
//!   source archetypes (keyed by `ARCHETYPE_ID`) + OPTs (keyed by `UUID`).
//! - [`adl2`]  — `I_DEFINITION_ADL2` (`i_definition_adl2.adoc`): ADL2 artefacts
//!   (keyed by `ARCHETYPE_HRID`).
//! - [`query_store`] — `I_DEFINITION_QUERY` (`i_definition_query.adoc`) +
//!   `QUERY_DESCRIPTOR` + the stored-query CRUD. DEFINITION *owns* query
//!   registration (`master04` §Registered Queries); the Query service only
//!   resolves + executes it, so the stored-query store folds in here.
//! - [`wire`]  — the ITS-REST wire-shaped `DefinitionAdapter` extension (rich
//!   template/query shapes the SM interfaces do not express). Its route wiring
//!   is the ITS-REST layer's concern; behaviour rides on the SM logic here.
//!
//! PORT NOTE (interchange form): the SM `I_DEFINITION_*` signatures take/return
//! AOM object types (`ARCHETYPE`, `AUTHORED_ARCHETYPE`). openEHR publishes no
//! BMM meta-model for AOM *instances*, so the interchange form is the artefact's
//! serialization — ADL 1.4 / ADL2 source text and OPT 1.4 canonical XML — which
//! is what these calls carry (`i_definition_adl14.adoc` / `i_definition_adl2.adoc`).
//!
//! Identity canonicalisation (BASE `master05-identification_package.adoc`
//! §Composite Identifiers and Case): "two identifiers identical apart from case
//! are considered to be identical, and therefore to identify the same thing"
//! (l.169). Identity is therefore matched **case-insensitively** while storage
//! is **case-preserving** — every lookup/existence/delete compares
//! `lower(id) = lower($1)` and every write first removes any case-variant of the
//! same id in the same transaction. This is the single canonicalisation boundary
//! ([`CANONICAL_CITE`]).

mod adl14;
mod adl2;
mod query_store;
mod wire;

use regex::Regex;

use ehrbase_sm::{CallStatusType, Page};

use super::ServiceError;

/// A qualified query name decomposed per `master04-definition_package.adoc`
/// §Registered Queries: `<namespace>::<query-name>` or the three-part
/// `<namespace>::<formalism>::<query-name>`.
pub(super) struct QualifiedName {
    /// The namespace segment (`"misc"` when none was supplied — §Registered
    /// Queries, l.34).
    pub namespace: String,
    /// The formalism segment of a three-part name, if present (§Registered
    /// Queries scheme 2). Feeds `QUERY_DESCRIPTOR.formalism` / `query_type`.
    pub formalism: Option<String>,
    /// The bare query-name segment (the store key, never carrying the formalism).
    pub name: String,
}

impl QualifiedName {
    /// The canonical two-part `<namespace>::<query-name>` — the form the
    /// stored-query store keys on (so a three-part input round-trips to the same
    /// row and the formalism is never folded into the name, G-05-04).
    pub(super) fn qualified(&self) -> String {
        format!("{}::{}", self.namespace, self.name)
    }
}

/// Decompose a (possibly unqualified) query name per `master04` §Registered
/// Queries: apply the `"misc"` default namespace, then recognise the two-part
/// `<ns>::<name>` and three-part `<ns>::<formalism>::<name>` schemes. A
/// three-part name's middle segment is lifted out as the formalism (never left
/// folded into the name — G-05-04); any other segment count keeps the first
/// segment as the namespace and the remainder as the (possibly `::`-bearing)
/// name.
pub(super) fn parse_qualified_name(raw: &str) -> QualifiedName {
    let qualified = qualify(raw);
    let segments: Vec<&str> = qualified.split("::").collect();
    match segments.as_slice() {
        [namespace, formalism, name] => QualifiedName {
            namespace: (*namespace).to_owned(),
            formalism: Some((*formalism).to_owned()),
            name: (*name).to_owned(),
        },
        [namespace, rest @ ..] if !rest.is_empty() => QualifiedName {
            namespace: (*namespace).to_owned(),
            formalism: None,
            name: rest.join("::"),
        },
        _ => QualifiedName {
            namespace: "misc".to_owned(),
            formalism: None,
            name: qualified,
        },
    }
}

/// Apply the SM `"misc"` default namespace: a name with no `::` becomes
/// `misc::<name>` (`master04` §Registered Queries: "If no namespace is supplied,
/// the namespace `"misc"` is assumed").
pub(super) fn qualify(name: &str) -> String {
    if name.contains("::") {
        name.to_owned()
    } else {
        format!("misc::{name}")
    }
}

/// Split an already two-part-canonical query name into `(reverse_domain_name,
/// semantic_id)` on the first `::`, so the SM and wire paths key `stored_query`
/// rows identically. Callers pass the [`QualifiedName::qualified`] form, which is
/// always two-part, so the formalism is never captured in `semantic_id`.
pub(super) fn split_qualified(qualified_name: &str) -> (&str, &str) {
    qualified_name
        .split_once("::")
        .unwrap_or(("", qualified_name))
}

/// Apply an SM [`Page`] to an iterator: skip `offset`, take `limit` (`None` ⇒
/// all — `master02-overview.adoc` §List Handling).
pub(super) fn paginate<T>(items: impl Iterator<Item = T>, page: Page) -> Vec<T> {
    let offset = usize::try_from(page.offset()).unwrap_or(usize::MAX);
    let skipped = items.skip(offset);
    match page.limit() {
        Some(n) => skipped
            .take(usize::try_from(n).unwrap_or(usize::MAX))
            .collect(),
        None => skipped.collect(),
    }
}

/// A [`Page`] as `(offset, limit)` SQL bind values; a `None` limit binds SQL
/// `NULL` (`LIMIT NULL` = all rows in `PostgreSQL`).
pub(super) fn page_bounds(page: Page) -> (i64, Option<i64>) {
    let offset = i64::try_from(page.offset()).unwrap_or(i64::MAX);
    let limit = page.limit().and_then(|l| i64::try_from(l).ok());
    (offset, limit)
}

/// Compile an id-pattern regex; an uncompilable pattern is `invalid_id_pattern`
/// (`400`).
///
/// PORT NOTE (G-05-11): the SM spells these "PERL regular expression"; the
/// `regex` crate is RE2-class, so PERL backreferences / lookaround are
/// unsupported — a pattern using them fails to compile and surfaces as
/// `invalid_id_pattern`, the correct SM outcome for an unusable pattern (a
/// narrower accept envelope, never a wrong status).
pub(super) fn compile_pattern(pattern: &str) -> Result<Regex, ServiceError> {
    Regex::new(pattern).map_err(|e| {
        ServiceError::sm(
            CallStatusType::InvalidIdPattern,
            format!("invalid id pattern: {e}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualify_applies_misc_default() {
        // master04 §Registered Queries: no namespace ⇒ "misc".
        assert_eq!(qualify("all_over_50"), "misc::all_over_50");
        assert_eq!(qualify("ehr::x"), "ehr::x");
        assert_eq!(qualify("ns::aql::x"), "ns::aql::x");
    }

    #[test]
    fn three_part_name_lifts_the_formalism_out_of_the_name() {
        // master04 §Registered Queries scheme 2: <ns>::<formalism>::<name>.
        // The formalism segment must NOT be folded into the stored name
        // (G-05-04) — it becomes the descriptor formalism and the store key is
        // the canonical two-part <ns>::<name>.
        let q = parse_qualified_name("task_planning::aql::chemotherapy_plans");
        assert_eq!(q.namespace, "task_planning");
        assert_eq!(q.formalism.as_deref(), Some("aql"));
        assert_eq!(q.name, "chemotherapy_plans");
        assert_eq!(q.qualified(), "task_planning::chemotherapy_plans");

        // Two-part names keep the whole remainder as the name, no formalism.
        let two = parse_qualified_name("ehr::all_over_50");
        assert_eq!(two.namespace, "ehr");
        assert_eq!(two.formalism, None);
        assert_eq!(two.name, "all_over_50");

        // Unqualified ⇒ misc default, two-part canonical form.
        let bare = parse_qualified_name("all_over_50");
        assert_eq!(bare.qualified(), "misc::all_over_50");
    }
}
