//! The `/` dashboard: headline stat tiles, per-group match tiles, and a
//! commit-activity trend chart.
//!
//! No openEHR spec governs an admin UI — our own design / product extension.
//! The wire it reads IS spec-bound: every count and the trend run against
//! `POST query/aql` (`docs/specs/openehr/ITS-REST/docs/query/`) with fixed,
//! parse-validated AQL consts — user input never reaches the query text.
//!
//! Every co-located `#[server]` fn guards with
//! [`require_session`](crate::session::require_session) first (a server fn is a
//! public HTTP API — rules §0); the CDR credential never reaches client-visible
//! state. Each tile/section is an `.into_any()`-erased local with its own
//! `<Suspense>`/`<Transition>` skeleton that resolves its `Result` inside the
//! suspense (rendering [`inline_error`](crate::components::format_view::inline_error)
//! on failure) rather than through an `<ErrorBoundary>` — an SSR'd
//! `ErrorBoundary` fallback mismatches at hydration in leptos 0.8 — so one
//! failing section never blanks the dashboard (rules §1/§6).

use leptos::prelude::*;
use leptos::{component, server};
use leptos_chartistry::{
    AspectRatio, AxisMarker, Chart, IntoInner, Line, Series, TickLabels, YGridLine,
};
use leptos_meta::Title;
use leptos_router::components::A;

use crate::error::AdminUiError;
use crate::queries_api::QueryGroup;

/// Total EHRs — a fixed count AQL. Validated by [`tests::dashboard_aql_consts_parse`].
#[cfg(feature = "ssr")]
const EHR_COUNT_AQL: &str = "SELECT COUNT(*) FROM EHR e";
/// Total compositions — a fixed count AQL.
#[cfg(feature = "ssr")]
const COMPOSITION_COUNT_AQL: &str = "SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c";
/// Recent composition commit times, newest first, for the day-bucketed trend.
#[cfg(feature = "ssr")]
const TREND_AQL: &str = "SELECT c/context/start_time/value FROM EHR e CONTAINS COMPOSITION c ORDER BY c/context/start_time/value DESC";
/// Rows pulled for the commit-activity trend before day-bucketing.
#[cfg(feature = "ssr")]
const TREND_FETCH: u32 = 500;

/// Run a `SELECT COUNT(*)`-shaped AQL and read its single numeric cell (0 when
/// the result set is not the expected 1×1 numeric shape).
#[cfg(feature = "ssr")]
async fn run_count_aql(
    state: &crate::state::AppState,
    session: &crate::session::AdminSession,
    aql: &str,
) -> Result<i64, AdminUiError> {
    let url = state.cdr.rest_v1("query/aql");
    let body = serde_json::json!({ "q": aql, "query_parameters": {} }).to_string();
    let response = state
        .cdr
        .post(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[],
            body,
        )
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    let page = crate::pages::ehrs::parse_result_set(&response.body, 0)?;
    Ok(page
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0))
}

/// Bucket ISO-8601 commit timestamps into `(day, count)` pairs ascending by
/// day. The day is the `YYYY-MM-DD` date prefix read with `s.get(..10)` (never
/// `&s[..10]`, which can panic on a non-char-boundary — the `string_slice`
/// lint); a value without a 10-character prefix is skipped. Counts saturate.
#[cfg(feature = "ssr")]
fn bucket_by_day(times: &[String]) -> Vec<(String, u32)> {
    let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for time in times {
        if let Some(day) = time.get(..10) {
            counts
                .entry(day.to_owned())
                .and_modify(|count| *count = count.saturating_add(1))
                .or_insert(1);
        }
    }
    counts.into_iter().collect()
}

/// Split a `name@version` group-member reference into its qualified name and
/// version, or `None` when it lacks the `@version` suffix. Splits on the LAST
/// `@` so a qualified name is never mistaken for the version. Shared with the
/// stored-queries screen's member chips.
pub(crate) fn split_query_ref(reference: &str) -> Option<(String, String)> {
    reference
        .rsplit_once('@')
        .filter(|(name, version)| !name.is_empty() && !version.is_empty())
        .map(|(name, version)| (name.to_owned(), version.to_owned()))
}

/// The dashboard's three headline counts: total EHRs, total compositions, and
/// the number of stored operational templates.
///
/// Runs the two fixed count AQLs against `POST query/aql` and reads the
/// template count from
/// [`list_templates`](crate::pages::templates::list_templates).
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR errors
/// normalized via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] on an unparseable result set.
#[server]
pub async fn dashboard_counts() -> Result<(i64, i64, u32), AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let ehrs = run_count_aql(&state, &session, EHR_COUNT_AQL).await?;
    let compositions = run_count_aql(&state, &session, COMPOSITION_COUNT_AQL).await?;
    let templates =
        u32::try_from(crate::pages::templates::list_templates().await?.len()).unwrap_or(u32::MAX);
    Ok((ehrs, compositions, templates))
}

/// Sum the match counts of a group's member stored queries. Each member is a
/// `name@version` reference; a member that is not `name@version` (e.g. a query
/// deleted from the CDR) is skipped so a stale reference never fails the whole
/// tile.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR errors
/// normalized for any member query that fails to run.
#[server]
pub async fn group_count(members: Vec<String>) -> Result<i64, AdminUiError> {
    crate::session::require_session().await?;
    let mut total: i64 = 0;
    for member in &members {
        if let Some((name, version)) = split_query_ref(member) {
            total =
                total.saturating_add(crate::queries_api::run_stored_count(name, version).await?);
        }
    }
    Ok(total)
}

/// The recent commit-activity trend as `(day, count)` pairs ascending. Pulls
/// the most recent composition commit times (fetch [`TREND_FETCH`]) and buckets
/// them per calendar day BFF-side.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR errors
/// normalized; [`AdminUiError::Internal`] on an unparseable result set.
#[server]
pub async fn commit_trend() -> Result<Vec<(String, u32)>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state.cdr.rest_v1("query/aql");
    let body = serde_json::json!({
        "q": TREND_AQL,
        "query_parameters": {},
        "fetch": TREND_FETCH,
    })
    .to_string();
    let response = state
        .cdr
        .post(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[],
            body,
        )
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    let page = crate::pages::ehrs::parse_result_set(&response.body, 0)?;
    let times: Vec<String> = page
        .rows
        .iter()
        .filter_map(|row| row.first())
        .map(crate::pages::ehrs::cell_text)
        .collect();
    Ok(bucket_by_day(&times))
}

/// The `/` dashboard: headline counts, per-group match tiles, and a
/// commit-activity trend — each an independently-failing section.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn DashboardPage() -> impl IntoView {
    let counts = counts_section();
    let stored = stored_queries_tile();
    let groups = groups_section();
    let trend = trend_section();

    view! {
        <Title text="Dashboard · ehrbase-admin" />
        <div class="p-4">
            <h1 class="text-xl font-semibold mb-6">"Dashboard"</h1>
            <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-8">{counts} {stored}</div>
            <h2 class="text-sm font-semibold text-neutral-500 mb-2">"Query groups"</h2>
            <div class="mb-8">{groups}</div>
            <h2 class="text-sm font-semibold text-neutral-500 mb-2">"Commit activity"</h2>
            {trend}
        </div>
    }
}

/// One headline stat tile: a big number over a label.
fn stat_tile(label: &'static str, value: String) -> AnyView {
    view! {
        <thaw::Card>
            <div class="p-4">
                <div class="text-3xl font-semibold tabular-nums">{value}</div>
                <div class="text-sm text-neutral-500 mt-1">{label}</div>
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// A single stat-tile skeleton (a card-shaped placeholder).
fn tile_skeleton() -> AnyView {
    view! {
        <thaw::Card>
            <div class="p-4">
                <thaw::Skeleton>
                    <thaw::SkeletonItem class="h-8 mb-2" />
                    <thaw::SkeletonItem class="h-4 w-2/3" />
                </thaw::Skeleton>
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// Three stat-tile skeletons for the counts section's loading state (keeps the
/// tile grid stable while the counts load).
fn tiles_skeleton() -> impl IntoView {
    view! { <>{tile_skeleton()} {tile_skeleton()} {tile_skeleton()}</> }
}

/// The EHR / composition / template count tiles, all from one
/// [`dashboard_counts`] round trip.
fn counts_section() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { dashboard_counts().await });
    view! {
        <Suspense fallback=tiles_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok((ehrs, compositions, templates)) => {
                        // Resolve the Result INSIDE the suspense and render either
                        // branch as one erased view: hydrating an SSR'd
                        // ErrorBoundary fallback mismatches in leptos 0.8 (caught
                        // by the E2E console gate), and this keeps server/client
                        // structure identical while errors stay visible.
                        view! {
                            <>
                                {stat_tile("EHRs", ehrs.to_string())}
                                {stat_tile("Compositions", compositions.to_string())}
                                {stat_tile("Templates", templates.to_string())}
                            </>
                        }
                            .into_any()
                    }
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// The stored-query count tile (its own round trip so a Definition-API failure
/// doesn't take the count tiles down with it).
fn stored_queries_tile() -> AnyView {
    let resource = Resource::new(
        || (),
        |()| async move { crate::queries_api::list_stored_queries().await },
    );
    view! {
        <Suspense fallback=tile_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok(rows) => {
                        let count = u32::try_from(rows.len()).unwrap_or(u32::MAX);
                        stat_tile("Stored queries", count.to_string())
                    }
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// The per-group match-count tiles, each linking to the stored-queries screen.
fn groups_section() -> AnyView {
    let resource = Resource::new(
        || (),
        |()| async move { crate::queries_api::list_groups().await },
    );
    view! {
        <Transition fallback=tile_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok(groups) => groups_tiles(groups),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// Render the group tiles, or the empty-state hint.
fn groups_tiles(groups: Vec<QueryGroup>) -> AnyView {
    if groups.is_empty() {
        return view! {
            <p class="text-sm text-neutral-500">
                "No groups yet — create one from the stored queries screen."
            </p>
        }
        .into_any();
    }
    let tiles = groups.into_iter().map(group_tile).collect::<Vec<_>>();
    view! { <div class="grid grid-cols-2 md:grid-cols-4 gap-3">{tiles}</div> }.into_any()
}

/// One group tile: the group name, its summed member match count (its own
/// [`group_count`] round trip), and a link to the stored-queries screen. The
/// `<A>`-wrapped `<thaw::Card>` is a block (non-interactive) descendant, so the
/// anchor stays valid HTML (rules §8).
fn group_tile(group: QueryGroup) -> AnyView {
    let members = group.members.clone();
    let count = Resource::new(
        || (),
        move |()| {
            let members = members.clone();
            async move { group_count(members).await }
        },
    );
    let member_count = group.members.len();
    view! {
        <A href="/queries" attr:class="block">
            <thaw::Card>
                <div class="p-4">
                    <div class="font-medium truncate">{group.name}</div>
                    <div class="text-2xl font-semibold tabular-nums mt-1">
                        <Suspense fallback=|| {
                            view! { <span class="text-neutral-400">"…"</span> }
                        }>
                            {move || Suspend::new(async move {
                                match count.await {
                                    Ok(total) => total.to_string().into_any(),
                                    Err(_) => {
                                        // Resolve inside the Suspense: an SSR'd ErrorBoundary
                                        // fallback mismatches at hydration in leptos 0.8.
                                        view! { <span class="text-sm text-red-600">"error"</span> }
                                            .into_any()
                                    }
                                }
                            })}
                        </Suspense>
                    </div>
                    <div class="text-xs text-neutral-500 mt-1">
                        {format!("{member_count} members")}
                    </div>
                </div>
            </thaw::Card>
        </A>
    }
    .into_any()
}

/// The commit-activity trend section: a minimal line chart of commits per day.
fn trend_section() -> AnyView {
    let resource = Resource::new(|| (), |()| async move { commit_trend().await });
    view! {
        <thaw::Card>
            <div class="p-4">
                <Suspense fallback=|| {
                    view! {
                        <thaw::Skeleton>
                            <thaw::SkeletonItem class="h-40" />
                        </thaw::Skeleton>
                    }
                }>
                    {move || Suspend::new(async move {
                        match resource.await {
                            Ok(pairs) => trend_chart(&pairs),
                            Err(e) => crate::components::format_view::inline_error(&e),
                        }
                    })}
                </Suspense>
            </div>
        </thaw::Card>
    }
    .into_any()
}

/// Render the day/count pairs as a minimal line chart (commits per day over the
/// recent window), or an empty-state hint. The X axis is the day index; the
/// chart draws client-side after the container is measured (server renders a
/// placeholder), so the structure is hydration-stable (rules §8).
fn trend_chart(pairs: &[(String, u32)]) -> AnyView {
    if pairs.is_empty() {
        return view! { <p class="text-sm text-neutral-500">"No commit activity to chart yet."</p> }
            .into_any();
    }
    let data: Vec<(f64, f64)> = pairs
        .iter()
        .enumerate()
        .map(|(index, (_, count))| {
            let x = f64::from(u32::try_from(index).unwrap_or(u32::MAX));
            (x, f64::from(*count))
        })
        .collect();
    let data = RwSignal::new(data);
    view! {
        <div class="overflow-x-auto">
            <Chart
                aspect_ratio=AspectRatio::from_outer_ratio(640.0, 240.0)
                left=TickLabels::aligned_floats()
                bottom=TickLabels::aligned_floats()
                inner=[
                    AxisMarker::left_edge().into_inner(),
                    AxisMarker::bottom_edge().into_inner(),
                    YGridLine::default().into_inner(),
                ]
                series=Series::new(|(x, _): &(f64, f64)| *x)
                    .line(Line::new(|(_, y): &(f64, f64)| *y).with_name("commits"))
                data=data
            />
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::split_query_ref;
    #[cfg(feature = "ssr")]
    use super::{COMPOSITION_COUNT_AQL, EHR_COUNT_AQL, TREND_AQL, bucket_by_day};

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

    #[cfg(feature = "ssr")]
    #[test]
    fn dashboard_aql_consts_parse() {
        for aql in [EHR_COUNT_AQL, COMPOSITION_COUNT_AQL, TREND_AQL] {
            openehr_query::parser::parse_str(aql).expect("dashboard AQL const must parse");
        }
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn bucket_by_day_counts_date_prefix_ascending() {
        let times = vec![
            "2026-07-15T09:00:00Z".to_owned(),
            "2026-07-15T18:30:00Z".to_owned(),
            "2026-07-14T00:00:00Z".to_owned(),
            "short".to_owned(),
        ];
        assert_eq!(
            bucket_by_day(&times),
            vec![("2026-07-14".to_owned(), 1), ("2026-07-15".to_owned(), 2)]
        );
    }
}
