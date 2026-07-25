//! Namespace-derived stored-query grouping: the pure rules the `/queries`
//! screen, the dashboard tiles, and both save flows share. Component-free
//! plain Rust with ordinary unit tests (crate discipline), compiled for both
//! the `ssr` and `hydrate` targets.
//!
//! A stored query's group **is** the namespace of its qualified name. ITS-REST
//! defines the stored-query identifier as `[{namespace}::]{query-name}`, with
//! the namespace optional and, when present, "in a form of a reverse domain
//! name, which allows for separation of use of stored queries by teams,
//! companies, etc."
//! (`ITS-REST specifications/docs/query/Qualified_query_name.md` §Qualified
//! query name) — so the grouping the console shows is the separation the
//! identifier already carries: durable in the CDR, visible to every API
//! client, and nothing the console has to store.
//!
//! NOTE: the *presentation* of that grouping (headings, tiles, the label for
//! the unqualified bucket) is our own design/extension — no openEHR spec
//! governs an admin UI. Only the name format these rules read is spec-bound,
//! cited above.

use crate::queries_api::StoredQueryRow;

/// The heading for stored queries whose name carries no namespace. The
/// namespace is optional per the spec, so this labels an absence rather than
/// naming a namespace — which is why [`QueryNamespaceGroup::namespace`] stays
/// `None` for that bucket and cannot collide with a real namespace of the same
/// text.
pub const UNQUALIFIED_LABEL: &str = "unqualified";

/// One derived group: the namespace (`None` for the unqualified bucket) and
/// its member stored queries in listing order. A plain record — it is built
/// from a listing, never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryNamespaceGroup {
    /// The namespace prefix shared by every member, or `None` when the members
    /// carry no namespace at all.
    pub namespace: Option<String>,
    /// The member stored queries, in the order the CDR listed them.
    pub members: Vec<StoredQueryRow>,
}

/// The namespace prefix of a qualified stored-query name — the text before the
/// FIRST `::` — or `None` when the name carries none.
///
/// Reading `[{namespace}::]{query-name}` left to right makes the first `::`
/// the separator; the spec's `query-name` character class (`[a-zA-Z0-9_.-]`)
/// admits no colon, so a name with a second `::` is not spec-valid and its
/// leading segment is still taken as the namespace. An empty prefix (`::name`)
/// is not a namespace.
#[must_use]
pub fn namespace_of(qualified_name: &str) -> Option<&str> {
    let (namespace, _) = qualified_name.split_once("::")?;
    let namespace = namespace.trim();
    (!namespace.is_empty()).then_some(namespace)
}

/// The bare query name: a qualified name with its namespace prefix removed,
/// or the whole name when it carries none.
#[must_use]
pub fn bare_name_of(qualified_name: &str) -> &str {
    match qualified_name.split_once("::") {
        Some((namespace, rest)) if !namespace.trim().is_empty() => rest,
        _ => qualified_name,
    }
}

/// Split a qualified name into the console's two save fields:
/// `(namespace, bare name)`, with an empty namespace for an unqualified name.
/// This is what pre-fills the save form when a stored query is opened in the
/// editor.
#[must_use]
pub fn split_qualified(qualified_name: &str) -> (String, String) {
    (
        namespace_of(qualified_name).unwrap_or_default().to_owned(),
        bare_name_of(qualified_name).to_owned(),
    )
}

/// Compose the qualified name a save writes, from the namespace field and the
/// name field. Both parts are trimmed; the namespace is optional (per the spec
/// cited in the module docs), so an empty one yields the bare name.
///
/// A `name` that ALREADY carries a `::` prefix is taken as fully qualified and
/// wins: typing `org.example::vitals` into the name field saves exactly that,
/// so the field accepts either form.
#[must_use]
pub fn qualify(namespace: &str, name: &str) -> String {
    let name = name.trim();
    if namespace_of(name).is_some() {
        return name.to_owned();
    }
    let namespace = namespace.trim();
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{namespace}::{name}")
    }
}

/// Split a `name@version` stored-query reference into its qualified name and
/// version, or `None` when it lacks the `@version` suffix. Splits on the LAST
/// `@` so a qualified name is never mistaken for the version.
///
/// `name@version` is the console's own reference form for one stored-query
/// VERSION (the raw editor's `?load=` hand-off carries it) — the openEHR REST
/// API keeps the two as separate path segments, so this pairing is our
/// design/extension, not a spec form.
///
/// NOTE: the version is OPAQUE text here — never parsed, compared, or
/// normalized. A stored-query version is SEMVER-style `major.minor.patch`, and
/// a partial or absent version is resolved by the CDR to "the latest `version`
/// with the supplied prefix" (ITS-REST
/// `specifications/docs/query/Qualified_query_name.md` §Qualified query name),
/// so prefix resolution is the server's job; a client-side semver assumption
/// about a reference would be wrong. [`is_full_semver`] and [`next_minor`] do
/// read the structure, but only where a STORE demands an exact version.
#[must_use]
pub fn split_query_ref(reference: &str) -> Option<(String, String)> {
    reference
        .rsplit_once('@')
        .filter(|(name, version)| !name.is_empty() && !version.is_empty())
        .map(|(name, version)| (name.to_owned(), version.to_owned()))
}

/// Does `version` carry all three SEMVER parts (`major.minor.patch`, each a
/// plain non-negative integer)?
///
/// A stored-query version "is in the format specified by SEMVER style (i.e.
/// `major.minor.patch`)", and a PARTIAL pattern (`{major}` or
/// `{major}.{minor}`) means "the latest query version matching supplied
/// prefix" (ITS-REST `specifications/docs/query/Qualified_query_name.md`
/// §Qualified query name). Prefix RESOLUTION is a read concept: storing at a
/// prefix would file a query under a version string that later prefix lookups
/// would treat as a resolvable pattern rather than a concrete version, so the
/// save screens require all three parts and leave prefixes to reads.
///
/// NOTE: rejecting a prefix ON STORE is our own guard — the spec describes the
/// prefix form for the shared path parameter without saying what storing at one
/// means; nothing in the console interprets a version beyond this check and
/// [`next_minor`].
#[must_use]
pub fn is_full_semver(version: &str) -> bool {
    let mut parts = version.split('.');
    let three = [parts.next(), parts.next(), parts.next()];
    parts.next().is_none()
        && three.iter().all(|part| {
            part.is_some_and(|part| {
                !part.is_empty()
                    && part.bytes().all(|byte| byte.is_ascii_digit())
                    && part.parse::<u64>().is_ok()
            })
        })
}

/// The next MINOR version after `version` (`1.4.2` → `1.5.0`), or `None` when
/// `version` is not a full SEMVER triple or its minor part cannot be
/// incremented.
///
/// Used to pre-fill the save screens after loading a stored query, so editing
/// a loaded definition proposes a NEW version instead of colliding with the
/// immutable one it came from (an explicit `(name, version)` pair is never
/// overwritten — ITS-REST `operations/definition_query_version_store.yaml`
/// answers `409` for an existing pair).
///
/// NOTE: which part to bump is our own UX choice — no openEHR spec governs an
/// admin UI, and the SEMVER-style version says nothing about which change to a
/// query definition warrants which increment. The field stays editable.
#[must_use]
pub fn next_minor(version: &str) -> Option<String> {
    if !is_full_semver(version) {
        return None;
    }
    let mut parts = version.split('.');
    let major = parts.next()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    Some(format!("{major}.{}.0", minor.checked_add(1)?))
}

/// Group a stored-query listing by the namespace of each name: namespaced
/// groups first, sorted by namespace, then the unqualified bucket (omitted
/// when empty). Members keep the CDR's listing order.
#[must_use]
pub fn group_by_namespace(rows: &[StoredQueryRow]) -> Vec<QueryNamespaceGroup> {
    let mut named: std::collections::BTreeMap<String, Vec<StoredQueryRow>> =
        std::collections::BTreeMap::new();
    let mut unqualified: Vec<StoredQueryRow> = Vec::new();
    for row in rows {
        match namespace_of(&row.name) {
            Some(namespace) => named
                .entry(namespace.to_owned())
                .or_default()
                .push(row.clone()),
            None => unqualified.push(row.clone()),
        }
    }
    let mut out: Vec<QueryNamespaceGroup> = named
        .into_iter()
        .map(|(namespace, members)| QueryNamespaceGroup {
            namespace: Some(namespace),
            members,
        })
        .collect();
    if !unqualified.is_empty() {
        out.push(QueryNamespaceGroup {
            namespace: None,
            members: unqualified,
        });
    }
    out
}

/// The heading a derived group renders under: the namespace itself, or
/// [`UNQUALIFIED_LABEL`] for the bucket of names that omit one.
#[must_use]
pub fn group_label(namespace: Option<&str>) -> &str {
    namespace.unwrap_or(UNQUALIFIED_LABEL)
}

#[cfg(test)]
mod tests {
    use crate::queries_api::StoredQueryRow;
    use crate::query_namespace::{
        UNQUALIFIED_LABEL, bare_name_of, group_by_namespace, group_label, is_full_semver,
        namespace_of, next_minor, qualify, split_qualified, split_query_ref,
    };

    /// A listing row with only the fields the grouping reads.
    fn row(name: &str, version: &str) -> StoredQueryRow {
        StoredQueryRow {
            name: name.to_owned(),
            version: version.to_owned(),
            query: String::new(),
            saved: String::new(),
        }
    }

    #[test]
    fn namespace_of_reads_the_spec_examples() {
        // The three valid `qualified_query_name` examples from
        // ITS-REST docs/query/Qualified_query_name.md.
        assert_eq!(
            namespace_of("org.openehr::my_compositions"),
            Some("org.openehr")
        );
        assert_eq!(namespace_of("my_compositions"), None);
        assert_eq!(
            namespace_of("ehr::all_influenza_vacc_candidates"),
            Some("ehr")
        );
    }

    #[test]
    fn namespace_of_handles_the_edge_cases() {
        // Empty name.
        assert_eq!(namespace_of(""), None);
        // No separator at all.
        assert_eq!(namespace_of("plain_name"), None);
        // A single colon is not the separator.
        assert_eq!(namespace_of("a:b"), None);
        // A second `::` is not spec-valid; the FIRST separator still wins.
        assert_eq!(namespace_of("a::b::c"), Some("a"));
        // An empty prefix is not a namespace.
        assert_eq!(namespace_of("::name"), None);
        // A trailing separator names a namespace with an empty query name.
        assert_eq!(namespace_of("ns::"), Some("ns"));
        // Whitespace around the namespace is trimmed away.
        assert_eq!(namespace_of("  ns  ::q"), Some("ns"));
        // Whitespace-only prefix is no namespace.
        assert_eq!(namespace_of("   ::q"), None);
    }

    #[test]
    fn namespace_of_is_char_boundary_safe_for_unicode() {
        // Multi-byte namespace and multi-byte query name both survive; the
        // split is on the `::` pattern, never a byte offset.
        assert_eq!(namespace_of("målinger::blodtryk"), Some("målinger"));
        assert_eq!(bare_name_of("målinger::blodtryk"), "blodtryk");
        assert_eq!(namespace_of("组织::查询"), Some("组织"));
        assert_eq!(bare_name_of("组织::查询"), "查询");
        // Emoji (4-byte scalar) directly adjacent to the separator.
        assert_eq!(namespace_of("🏥::q"), Some("🏥"));
        assert_eq!(bare_name_of("ns::🏥"), "🏥");
    }

    #[test]
    fn bare_name_drops_only_a_real_namespace_prefix() {
        assert_eq!(
            bare_name_of("org.openehr::my_compositions"),
            "my_compositions"
        );
        assert_eq!(bare_name_of("my_compositions"), "my_compositions");
        assert_eq!(bare_name_of("a::b::c"), "b::c");
        assert_eq!(bare_name_of("ns::"), "");
        // No namespace to drop → the name is returned whole.
        assert_eq!(bare_name_of("::name"), "::name");
        assert_eq!(bare_name_of(""), "");
    }

    #[test]
    fn qualify_composes_the_two_save_fields() {
        assert_eq!(qualify("org.example", "vitals"), "org.example::vitals");
        // The namespace is optional.
        assert_eq!(qualify("", "vitals"), "vitals");
        // Both parts are trimmed.
        assert_eq!(
            qualify("  org.example  ", "  vitals  "),
            "org.example::vitals"
        );
    }

    #[test]
    fn qualify_lets_a_typed_prefix_win_over_the_namespace_field() {
        // Typing the whole qualified name into the name field saves exactly it.
        assert_eq!(qualify("", "org.example::vitals"), "org.example::vitals");
        assert_eq!(
            qualify("ignored", "org.example::vitals"),
            "org.example::vitals"
        );
    }

    #[test]
    fn split_and_qualify_round_trip() {
        // Every qualified name the SPEC admits survives split → qualify
        // unchanged: `[{namespace}::]{query-name}` where `query-name` is
        // matched by `[a-zA-Z0-9_.-]` (ITS-REST
        // `specifications/docs/query/Qualified_query_name.md` §Qualified query
        // name), so a spec-valid bare name can never itself contain `::` and
        // can never collide with the typed-prefix rule.
        for name in [
            "org.openehr::my_compositions",
            "my_compositions",
            "ehr::all_influenza_vacc_candidates",
            "ns::",
            "::name",
            "",
            "målinger::blodtryk",
        ] {
            let (namespace, bare) = split_qualified(name);
            assert_eq!(qualify(&namespace, &bare), name, "round trip of `{name}`");
        }
    }

    #[test]
    fn a_second_double_colon_normalizes_to_the_last_segment_pair() {
        // `a::b::c` is NOT a spec-valid qualified name — the bare name would
        // have to contain `::`, which the `query-name` charset excludes — so it
        // cannot round-trip through two fields, and no requirement says it
        // should. Split reads the FIRST `::` (namespace `a`), and re-composing
        // applies the documented typed-prefix rule, which yields `b::c`.
        //
        // This is deliberately NOT silent: the save screens render the composed
        // name ("Saves as …") before the click, so an operator who opened such a
        // malformed name sees exactly what a save would write. Preserving it
        // instead would mean composing another invalid identifier.
        let (namespace, bare) = split_qualified("a::b::c");
        assert_eq!(namespace, "a");
        assert_eq!(bare, "b::c");
        assert_eq!(qualify(&namespace, &bare), "b::c");
    }

    #[test]
    fn split_query_ref_parses_name_at_version() {
        assert_eq!(
            split_query_ref("org.example::vitals@1.2.3"),
            Some(("org.example::vitals".to_owned(), "1.2.3".to_owned()))
        );
        assert_eq!(split_query_ref("no_at_sign"), None);
        assert_eq!(split_query_ref("trailing@"), None);
        assert_eq!(split_query_ref("@leading"), None);
        // Splits on the LAST '@' so a name is never mistaken for the version.
        assert_eq!(
            split_query_ref("weird@name@2.0.0"),
            Some(("weird@name".to_owned(), "2.0.0".to_owned()))
        );
    }

    #[test]
    fn group_by_namespace_sorts_namespaces_and_puts_the_unqualified_bucket_last() {
        let rows = vec![
            row("org.zeta::b", "1.0.0"),
            row("plain", "1.0.0"),
            row("org.alpha::a", "1.0.0"),
            row("org.zeta::a", "2.0.0"),
            row("другой", "1.0.0"),
        ];
        let groups = group_by_namespace(&rows);
        let labels: Vec<&str> = groups
            .iter()
            .map(|g| group_label(g.namespace.as_deref()))
            .collect();
        assert_eq!(labels, vec!["org.alpha", "org.zeta", UNQUALIFIED_LABEL]);
        // Members keep the listing order within their group.
        let zeta = &groups[1].members;
        assert_eq!(zeta.len(), 2);
        assert_eq!(zeta[0].name, "org.zeta::b");
        assert_eq!(zeta[1].name, "org.zeta::a");
        // The unqualified bucket holds both un-namespaced names.
        assert_eq!(groups[2].members.len(), 2);
        assert_eq!(groups[2].namespace, None);
    }

    #[test]
    fn group_by_namespace_omits_an_empty_unqualified_bucket() {
        let groups = group_by_namespace(&[row("ns::a", "1.0.0")]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].namespace.as_deref(), Some("ns"));
    }

    #[test]
    fn group_by_namespace_of_nothing_is_no_groups() {
        assert!(group_by_namespace(&[]).is_empty());
    }

    #[test]
    fn group_label_names_the_namespace_or_the_bucket() {
        assert_eq!(group_label(Some("org.example")), "org.example");
        assert_eq!(group_label(None), UNQUALIFIED_LABEL);
    }

    #[test]
    fn full_semver_needs_three_numeric_parts() {
        assert!(is_full_semver("1.0.0"));
        assert!(is_full_semver("0.0.0"));
        assert!(is_full_semver("10.20.30"));
        // Partial patterns are the READ form (prefix resolution), never a
        // store version.
        assert!(!is_full_semver("1"));
        assert!(!is_full_semver("1.0"));
        assert!(!is_full_semver("1.0.0.0"));
        // Non-numeric, empty, signed, and pre-release/build forms all fail:
        // the console stores only a concrete triple.
        assert!(!is_full_semver(""));
        assert!(!is_full_semver("1.0."));
        assert!(!is_full_semver(".0.0"));
        assert!(!is_full_semver("1.0.x"));
        assert!(!is_full_semver("1.-1.0"));
        assert!(!is_full_semver("1.0.0-rc.1"));
        assert!(!is_full_semver("1.0.0+build"));
        assert!(!is_full_semver(" 1.0.0"));
        // Numeric but unrepresentable parts are rejected rather than silently
        // truncated.
        assert!(!is_full_semver("1.99999999999999999999.0"));
    }

    #[test]
    fn next_minor_bumps_the_middle_part_and_resets_the_patch() {
        assert_eq!(next_minor("1.0.0").as_deref(), Some("1.1.0"));
        assert_eq!(next_minor("1.4.2").as_deref(), Some("1.5.0"));
        assert_eq!(next_minor("0.9.9").as_deref(), Some("0.10.0"));
        // The major part is carried verbatim, never re-formatted.
        assert_eq!(next_minor("07.1.0").as_deref(), Some("07.2.0"));
    }

    #[test]
    fn next_minor_declines_what_it_cannot_bump() {
        assert_eq!(next_minor("1.0"), None);
        assert_eq!(next_minor("latest"), None);
        assert_eq!(next_minor(""), None);
        assert_eq!(next_minor("1.0.0-rc.1"), None);
        // A saturated minor cannot be incremented, so no version is proposed
        // rather than a wrong one.
        assert_eq!(next_minor(&format!("1.{}.0", u64::MAX)), None);
    }
}
