// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The `/templates/adl2/{template_id}` screen — ADL2 template detail.
//!
//! Three panes over one stored ADL2 operational template, matching what the
//! ITS-REST Definition API actually serves for the resource: **Source** (the
//! stored artefact verbatim, `Accept: text/plain`), **AOM2 JSON** (the
//! `OperationalTemplateV2` canonical-JSON projection,
//! `Accept: application/json`), and **Example** (the CDR-generated example
//! composition, negotiated across the four `Accept_LOCATABLE` forms). A
//! version bar drives the versioned get
//! (`definition/template/adl2/{template_id}/{version}`) as `?version=` URL
//! state, so a pinned view is shareable.
//!
//! There is deliberately no path-catalog pane: the console's catalog is built
//! from an ADL 1.4 OPT's Web Template, and the ADL2 resource serves no Web
//! Template representation (an `Accept` asking for one is refused `406`), so
//! the screen says so rather than inventing one.
//!
//! No openEHR spec governs an admin UI — our own design / product extension;
//! the wire it reads is the ITS-REST Definition API.
//!
//! Discipline (rules §0/§1/§6/§8/§9): each `#[server]` fn guards the session
//! first and keeps CDR credentials server-side; absence (`404`) is `Ok(None)`,
//! a first-class state rather than an error; the view is composed from
//! `.into_any()`-erased sections; every pane resolves its `Result` INSIDE the
//! `<Transition>` (an SSR'd `ErrorBoundary` fallback mismatches at hydration in
//! leptos 0.8); tab and version state live in the URL.

use leptos::prelude::*;
use leptos::{component, server};
use leptos_meta::Title;
use leptos_router::hooks::{use_params_map, use_query_map};

use crate::adl2::{Adl2Tab, TemplateFamily};
use crate::components::field::{BTN_SECONDARY, INPUT};
use crate::components::page_header::{Crumb, PageHeader};
use crate::components::surface::{CARD_PAD, CARD_TITLE};
use crate::error::AdminUiError;
use crate::format::ReprFormat;
use crate::pages::templates::TemplateRow;

/// Fetch the stored ADL2 operational-template SOURCE.
///
/// GET `definition/template/adl2/{template_id}` (or
/// `…/{template_id}/{version}` when `version` is given) with
/// `Accept: text/plain` — the artefact interchange form the resource serves
/// verbatim. `version` is an exact SEMVER or a `{major}` / `{major}.{minor}`
/// prefix resolving to the highest match; an unknown template or version is a
/// `404`, returned as `Ok(None)` because absence is a state this screen
/// renders, not a failure.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::CdrUnauthorized`] / [`AdminUiError::Forbidden`] / [`AdminUiError::Cdr`] /
/// [`AdminUiError::CdrUnreachable`] from the CDR.
#[server]
pub async fn fetch_adl2_source(
    /// The ADL2 template to read (a full HRID, or a partial that resolves to
    /// the latest matching version).
    template_id: String,
    /// The release version to pin the read to, or `None` for the artefact
    /// `template_id` itself resolves to.
    version: Option<String>,
) -> Result<Option<String>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1(&crate::adl2::template_path(
        &template_id,
        version.as_deref(),
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "text/plain")
        .await?;
    if response.is(http::StatusCode::NOT_FOUND) {
        return Ok(None);
    }
    Ok(Some(crate::cdr::CdrClient::expect_success(response)?.body))
}

/// Fetch the `OperationalTemplateV2` canonical JSON of a stored ADL2 template.
///
/// The same resource as [`fetch_adl2_source`] under
/// `Accept: application/json`, which the CDR answers with the AOM2
/// canonical-JSON projection of the operational template. `404` → `Ok(None)`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::CdrUnauthorized`] / [`AdminUiError::Forbidden`] / [`AdminUiError::Cdr`] /
/// [`AdminUiError::CdrUnreachable`] from the CDR.
#[server]
pub async fn fetch_adl2_json(
    /// The ADL2 template to read.
    template_id: String,
    /// The release version to pin the read to, or `None`.
    version: Option<String>,
) -> Result<Option<String>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1(&crate::adl2::template_path(
        &template_id,
        version.as_deref(),
    ));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    if response.is(http::StatusCode::NOT_FOUND) {
        return Ok(None);
    }
    Ok(Some(crate::cdr::CdrClient::expect_success(response)?.body))
}

/// Fetch the CDR-generated example composition for a stored ADL2 template, in
/// `format`.
///
/// GET `definition/template/adl2/{template_id}/example` with `Accept` set to
/// the selected representation's media type. The Definition API declares no
/// VERSIONED example resource, so the example is generated from the artefact
/// `template_id` itself resolves to. `404` → `Ok(None)`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::CdrUnauthorized`] / [`AdminUiError::Forbidden`] / [`AdminUiError::Cdr`] /
/// [`AdminUiError::CdrUnreachable`] from the CDR (a template that cannot be
/// compiled into a Web Template answers `422`).
#[server]
pub async fn fetch_adl2_example(
    /// The ADL2 template to generate an example composition for.
    template_id: String,
    /// Which representation to negotiate for the example.
    format: ReprFormat,
) -> Result<Option<String>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let url = state.cdr.rest_v1(&crate::adl2::example_path(&template_id));
    let response = state
        .cdr
        .get(&session.credential, &url, format.media_type())
        .await?;
    if response.is(http::StatusCode::NOT_FOUND) {
        return Ok(None);
    }
    Ok(Some(crate::cdr::CdrClient::expect_success(response)?.body))
}

/// What one pane holds.
///
/// The three states are distinct on purpose: a pane whose tab is not selected
/// has fetched NOTHING and renders nothing, while a pane whose fetch came back
/// `404` has a real answer to report. Collapsing both into `Option` would show
/// the absence bar behind every closed tab, and flash it while a newly opened
/// one loads.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum PaneBody {
    /// This pane's tab is not selected; nothing has been requested.
    Idle,
    /// The CDR answered `404` — no such template at the selected version.
    Absent,
    /// The representation the CDR served.
    Loaded(String),
}

impl PaneBody {
    /// Read one fetched representation: `Some` is what the CDR served, `None`
    /// its `404`.
    fn of(fetched: Option<String>) -> Self {
        match fetched {
            Some(body) => Self::Loaded(body),
            None => Self::Absent,
        }
    }
}

/// The ADL2 template detail screen: the version bar, the tab bar, and the
/// source / AOM2-JSON / example panes.
#[expect(
    clippy::must_use_candidate,
    reason = "#[component] rewrites the fn; view!/mount always consumes the value"
)]
#[component]
pub fn Adl2TemplateDetailPage() -> impl IntoView {
    // NOTE: the route param arrives ALREADY percent-decoded on both targets
    // (`leptos_router`'s `ParamsMap::insert` runs every value through
    // `Url::unescape`), so decoding here would be a second, corrupting pass.
    let params = use_params_map();
    let template_id =
        Signal::derive(move || params.with(|map| map.get("template_id").unwrap_or_default()));

    // Both view selections are URL state, read in setup (rules §9): the pane
    // (`?tab=`) and the release version the reads are pinned to (`?version=`).
    let query = use_query_map();
    let tab =
        Memo::new(move |_| Adl2Tab::from_query(&query.with(|q| q.get("tab").unwrap_or_default())));
    let version = Memo::new(move |_| {
        query
            .with(|q| q.get("version"))
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    });
    let example_format = RwSignal::new(ReprFormat::CanonicalJson);

    // Each pane's resource is gated on its tab being active so only the
    // visible one fetches — the example fetch in particular runs the CDR's
    // example GENERATOR. The stable source keeps loaded state on re-show.
    let source: Resource<Result<PaneBody, AdminUiError>> = Resource::new(
        move || (tab.get() == Adl2Tab::Source).then(|| (template_id.get(), version.get())),
        |active| async move {
            match active {
                Some((id, version)) => Ok(PaneBody::of(fetch_adl2_source(id, version).await?)),
                None => Ok(PaneBody::Idle),
            }
        },
    );
    let json: Resource<Result<PaneBody, AdminUiError>> = Resource::new(
        move || (tab.get() == Adl2Tab::Json).then(|| (template_id.get(), version.get())),
        |active| async move {
            match active {
                Some((id, version)) => Ok(PaneBody::of(fetch_adl2_json(id, version).await?)),
                None => Ok(PaneBody::Idle),
            }
        },
    );
    let example: Resource<Result<PaneBody, AdminUiError>> = Resource::new(
        move || (tab.get() == Adl2Tab::Example).then(|| (template_id.get(), example_format.get())),
        |active| async move {
            match active {
                Some((id, format)) => Ok(PaneBody::of(fetch_adl2_example(id, format).await?)),
                None => Ok(PaneBody::Idle),
            }
        },
    );
    // The stored versions of this HRID family, read from the ONE listing
    // endpoint the Template Manager already reads (crate CLAUDE.md — one
    // reader per claim), so the version bar offers what the CDR actually
    // holds instead of guessing at a range.
    let listing: Resource<Result<Vec<TemplateRow>, AdminUiError>> = Resource::new(
        || (),
        |()| async move { crate::pages::templates::list_adl2_templates().await },
    );

    let versions = version_section(template_id, tab, version, listing);
    let catalog_note = no_catalog_note();
    let source_pane = source_tab(source);
    let json_pane = json_tab(json);
    let example_pane = example_tab(example, example_format);

    let tab_link = move |value: Adl2Tab| {
        let class = move || {
            let base = "rounded-control px-3 py-1.5 text-sm font-medium transition-colors";
            if tab.get() == value {
                format!("{base} bg-accent-subtle text-accent-ink")
            } else {
                format!("{base} text-ink-muted hover:bg-sunken")
            }
        };
        let href =
            move || crate::adl2::view_href(&template_id.get(), value, version.get().as_deref());
        view! {
            <leptos_router::components::A
                href=href
                attr:class=class
                attr:data-adl2-tab=value.as_query()
            >
                {value.label()}
            </leptos_router::components::A>
        }
        .into_any()
    };

    view! {
        <Title text=move || format!("ADL2 template · {}", template_id.get()) />
        <div class="p-6">
            <PageHeader
                title=template_id
                crumbs=vec![Crumb::new("Templates", TemplateFamily::Adl2.href())]
                mono=true
            />
            {versions}
            {catalog_note}
            <nav aria-label="ADL2 template views" class="flex gap-1 mb-4">
                {tab_link(Adl2Tab::Source)}
                {tab_link(Adl2Tab::Json)}
                {tab_link(Adl2Tab::Example)}
            </nav>
            <div>
                <div id="adl2-source-pane" class:hidden=move || tab.get() != Adl2Tab::Source>
                    {source_pane}
                </div>
                <div id="adl2-json-pane" class:hidden=move || tab.get() != Adl2Tab::Json>
                    {json_pane}
                </div>
                <div id="adl2-example-pane" class:hidden=move || tab.get() != Adl2Tab::Example>
                    {example_pane}
                </div>
            </div>
        </div>
    }
}

/// The version bar: one chip per stored version of this HRID family, plus a
/// free-text box for the wire's prefix resolution.
///
/// Both controls write the same `?version=` URL state, so a pinned view is
/// shareable and survives a reload. The chips come from the ADL2 listing —
/// the exact versions the CDR holds — while the box reaches the versioned
/// get's `{major}` / `{major}.{minor}` prefix forms, which resolve to the
/// highest match.
fn version_section(
    template_id: Signal<String>,
    tab: Memo<Adl2Tab>,
    version: Memo<Option<String>>,
    listing: Resource<Result<Vec<TemplateRow>, AdminUiError>>,
) -> AnyView {
    let chips = move || {
        Suspend::new(async move {
            let rows = match listing.await {
                Ok(rows) => rows,
                Err(e) => return crate::components::format_view::inline_error(&e),
            };
            let ids = rows
                .into_iter()
                .map(|row| row.template_id)
                .collect::<Vec<_>>();
            let id = template_id.get();
            let (stem, _) = crate::adl2::split_hrid(&id);
            let stored = crate::adl2::family_versions(&ids, stem);
            if stored.is_empty() {
                return view! {
                    <p class="text-sm text-ink-muted">
                        "The listing holds no other version of this template family."
                    </p>
                }
                .into_any();
            }
            let buttons = stored
                .into_iter()
                .map(|value| version_chip(template_id, tab, version, Some(value)))
                .collect::<Vec<_>>();
            view! {
                <div class="flex flex-wrap items-center gap-1">
                    {version_chip(template_id, tab, version, None)} {buttons}
                </div>
            }
            .into_any()
        })
    };
    view! {
        <section class=format!("{CARD_PAD} mb-4")>
            <h2 class=CARD_TITLE>"Version"</h2>
            <Transition fallback=|| {
                view! {
                    <thaw::Skeleton class="h-8">
                        <thaw::SkeletonItem />
                    </thaw::Skeleton>
                }
            }>{chips}</Transition>
            {version_form(template_id, tab, version)}
        </section>
    }
    .into_any()
}

/// One version chip: `None` reads the artefact the route names, `Some(v)` pins
/// the read to that release version through the versioned get.
fn version_chip(
    template_id: Signal<String>,
    tab: Memo<Adl2Tab>,
    version: Memo<Option<String>>,
    value: Option<String>,
) -> AnyView {
    let label = value.clone().unwrap_or_else(|| "As stored".to_owned());
    let marker = value.clone().unwrap_or_else(|| "stored".to_owned());
    let selected = {
        let value = value.clone();
        move || version.get() == value
    };
    let class = move || {
        let base = "rounded-full px-3 py-1 text-xs font-medium transition-colors";
        if selected() {
            format!("{base} bg-accent text-on-accent")
        } else {
            format!("{base} bg-accent-subtle text-accent-ink hover:bg-sunken")
        }
    };
    let href = move || crate::adl2::view_href(&template_id.get(), tab.get(), value.as_deref());
    view! {
        <leptos_router::components::A href=href attr:class=class attr:data-adl2-version=marker>
            {label}
        </leptos_router::components::A>
    }
    .into_any()
}

/// The free-text version box: a GET `<Form>` submitting `?version=` back to
/// this screen, so it works before the WASM bundle loads (rules §9). The
/// hidden `tab` field keeps the open pane across the submit — a GET form
/// replaces the action's whole query string, so a `?tab=` in the action would
/// be discarded.
fn version_form(
    template_id: Signal<String>,
    tab: Memo<Adl2Tab>,
    version: Memo<Option<String>>,
) -> AnyView {
    let action = move || {
        format!(
            "/templates/adl2/{}",
            urlencoding::encode(&template_id.get())
        )
    };
    let initial = version.get_untracked().unwrap_or_default();
    view! {
        <leptos_router::components::Form method="GET" action=action attr:class="mt-3">
            <div class="flex flex-wrap items-end gap-2">
                // The hidden field carries live state on `prop:value` with the
                // attribute for the server pass (rules §2 — attributes set the
                // initial state, properties carry the live one).
                <input
                    type="hidden"
                    name="tab"
                    value=tab.get_untracked().as_query()
                    prop:value=move || tab.get().as_query()
                />
                <label class="flex flex-col gap-1 text-xs text-ink-muted">
                    "Release version (1.1.0, or a 1 / 1.0 prefix)"
                    <input
                        id="adl2-version-input"
                        type="text"
                        name="version"
                        class=format!("w-64 {INPUT}")
                        placeholder="as stored"
                        value=initial
                    />
                </label>
                <button id="adl2-version-apply" type="submit" class=BTN_SECONDARY>
                    "Show version"
                </button>
            </div>
        </leptos_router::components::Form>
    }
    .into_any()
}

/// The standing statement that this screen has no path catalog, and why.
///
/// Verified against the CDR's own answer: the ADL2 template resource serves
/// `text/plain` source and `application/json` `OperationalTemplateV2` only,
/// and an `Accept` asking for a Web Template is refused `406`. The ADL 1.4
/// detail screen's catalog is built from an OPT 1.4 Web Template, so there is
/// nothing equivalent to show here — and a fabricated one would be worse than
/// none.
fn no_catalog_note() -> AnyView {
    view! {
        <p id="adl2-no-catalog" class="mb-4 text-sm text-ink-muted">
            "No path catalog: the Web Template tree on the ADL 1.4 detail screen is built from an
             OPT 1.4, and the CDR serves no Web Template representation of an ADL2 artefact. Use
             the AOM2 JSON pane to read the template's constraint structure."
        </p>
    }
    .into_any()
}

/// The `<Transition>` fallback every pane shares.
fn pane_skeleton() -> impl IntoView {
    view! {
        <thaw::Skeleton>
            <thaw::SkeletonItem class="h-4 mb-2" />
            <thaw::SkeletonItem class="h-4 mb-2 ml-4" />
            <thaw::SkeletonItem class="h-4 ml-4" />
        </thaw::Skeleton>
    }
}

/// The absence state: the CDR answered `404` for the template (or the pinned
/// version), which is a fact about the store, not a failure of the screen.
fn absent_view(what: &str) -> AnyView {
    let message = format!(
        "The CDR holds no {what} for this template at the selected version. Pick another version, \
         or go back to the ADL2 listing."
    );
    view! {
        <thaw::MessageBar intent=thaw::MessageBarIntent::Warning>
            <thaw::MessageBarBody>
                {message} " "
                <leptos_router::components::A
                    href=TemplateFamily::Adl2.href()
                    attr:class="text-accent hover:underline"
                >
                    "ADL2 templates"
                </leptos_router::components::A>
            </thaw::MessageBarBody>
        </thaw::MessageBar>
    }
    .into_any()
}

/// The Source pane: the stored ADL2 artefact verbatim in the shared document
/// pane (monospace, scrollable, copyable).
fn source_tab(source: Resource<Result<PaneBody, AdminUiError>>) -> AnyView {
    view! {
        <Transition fallback=pane_skeleton>
            {move || Suspend::new(async move {
                match source.await {
                    Ok(PaneBody::Idle) => ().into_any(),
                    Ok(PaneBody::Loaded(text)) => {
                        view! { <crate::components::format_view::DocumentPane body=text /> }
                            .into_any()
                    }
                    Ok(PaneBody::Absent) => absent_view("ADL2 source"),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// The AOM2 JSON pane: the `OperationalTemplateV2` canonical JSON,
/// pretty-printed into the shared document pane.
fn json_tab(json: Resource<Result<PaneBody, AdminUiError>>) -> AnyView {
    view! {
        <Transition fallback=pane_skeleton>
            {move || Suspend::new(async move {
                match json.await {
                    Ok(PaneBody::Idle) => ().into_any(),
                    Ok(PaneBody::Loaded(raw)) => {
                        let pretty = crate::components::format_view::pretty_body(
                            &raw,
                            ReprFormat::CanonicalJson,
                        );
                        view! { <crate::components::format_view::DocumentPane body=pretty /> }
                            .into_any()
                    }
                    Ok(PaneBody::Absent) => absent_view("OperationalTemplateV2 JSON"),
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any()
}

/// The Example pane: a format selector over the four representations the
/// example resource negotiates, then the generated composition.
fn example_tab(
    example: Resource<Result<PaneBody, AdminUiError>>,
    format: RwSignal<ReprFormat>,
) -> AnyView {
    let offered = vec![
        ReprFormat::CanonicalJson,
        ReprFormat::CanonicalXml,
        ReprFormat::Flat,
        ReprFormat::Structured,
    ];
    view! {
        <div class="space-y-3">
            <p class="text-sm text-ink-muted">
                "The example is generated from the artefact this route names — the Definition API
                 declares no versioned example resource, so the version bar above does not change
                 it."
            </p>
            <crate::components::format_view::FormatSelector offered=offered selected=format />
            <Transition fallback=pane_skeleton>
                {move || Suspend::new(async move {
                    match example.await {
                        Ok(PaneBody::Idle) => ().into_any(),
                        Ok(PaneBody::Loaded(raw)) => {
                            let pretty = crate::components::format_view::pretty_body(
                                &raw,
                                format.get_untracked(),
                            );
                            view! { <crate::components::format_view::DocumentPane body=pretty /> }
                                .into_any()
                        }
                        Ok(PaneBody::Absent) => absent_view("example composition"),
                        Err(e) => crate::components::format_view::inline_error(&e),
                    }
                })}
            </Transition>
        </div>
    }
    .into_any()
}
