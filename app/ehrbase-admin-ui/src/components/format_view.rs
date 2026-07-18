//! The shared read-only document viewer: a format selector row plus a
//! scrollable, monospaced rendering of the chosen representation. Used by
//! the template detail (OPT/WT/example), the composition viewer
//! (JSON/XML/FLAT/STRUCTURED), and anywhere else a wire document is shown.
//! Pure Rust — no JS highlighter; pretty-printing happens server-side or
//! via `serde_json` where the body is JSON.

use leptos::prelude::*;

use crate::format::ReprFormat;

/// The format-selector strip: a segmented control, one `<button>` per offered
/// format, driving the shared `selected` signal. The buttons keep their plain
/// visible labels (JSON/XML/FLAT/STRUCTURED) so the E2E suite can click them
/// by text.
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn FormatSelector(
    /// The formats this surface offers, in display order.
    offered: Vec<ReprFormat>,
    /// The currently selected format (shared with the surface's resource
    /// source so a change refetches).
    selected: RwSignal<ReprFormat>,
) -> impl IntoView {
    let buttons = offered
        .into_iter()
        .map(|format| {
            let class = move || {
                if selected.get() == format {
                    "px-3 py-1.5 text-sm font-medium bg-accent text-on-accent"
                } else {
                    "px-3 py-1.5 text-sm font-medium text-ink-muted hover:bg-sunken"
                }
            };
            view! {
                <button type="button" class=class on:click=move |_| selected.set(format)>
                    {format.label()}
                </button>
            }
            .into_any()
        })
        .collect::<Vec<_>>();
    view! {
        <div class="inline-flex overflow-hidden rounded-control border border-edge-strong divide-x divide-edge-strong">
            {buttons}
        </div>
    }
}

/// The document pane: monospaced, both-axis scrollable, whitespace
/// preserved. `body` is the raw representation text (already
/// pretty-printed where applicable).
#[allow(clippy::must_use_candidate)] // #[component] rewrites the fn; view!/mount always consumes the value
#[component]
pub fn DocumentPane(
    /// The document text to display.
    #[prop(into)]
    body: Signal<String>,
) -> impl IntoView {
    view! {
        <pre class="overflow-auto max-h-[70vh] whitespace-pre rounded-card border border-edge bg-sunken p-3 font-mono text-xs leading-relaxed text-ink">
            {move || body.get()}
        </pre>
    }
}

/// One domain error rendered as the standard inline error bar — used by
/// data sections that resolve their `Result` inside `<Suspense>` (SSR'd
/// `ErrorBoundary` fallbacks mismatch at hydration in leptos 0.8, so
/// sections render content-or-this directly; errors never render as
/// nothing).
#[must_use]
pub fn inline_error(error: &crate::error::AdminUiError) -> AnyView {
    let message = error.to_string();
    view! {
        <div
            role="alert"
            class="rounded-control border border-danger/40 bg-danger-subtle px-3 py-2 text-sm text-danger"
        >
            {message}
        </div>
    }
    .into_any()
}

/// Pretty-print a JSON body for display; non-JSON (XML, FLAT with odd
/// content types) passes through unchanged.
#[must_use]
pub fn pretty_body(body: &str, format: ReprFormat) -> String {
    match format {
        ReprFormat::CanonicalXml => body.to_owned(),
        _ => serde_json::from_str::<serde_json::Value>(body)
            .and_then(|v| serde_json::to_string_pretty(&v))
            .unwrap_or_else(|_| body.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use crate::components::format_view::pretty_body;
    use crate::format::ReprFormat;

    #[test]
    fn json_pretty_prints_and_xml_passes_through() {
        assert_eq!(
            pretty_body("{\"a\":1}", ReprFormat::CanonicalJson),
            "{\n  \"a\": 1\n}"
        );
        let xml = "<x a=\"1\"/>";
        assert_eq!(pretty_body(xml, ReprFormat::CanonicalXml), xml);
        // Broken JSON falls back to the raw body rather than erroring.
        assert_eq!(pretty_body("not json", ReprFormat::Flat), "not json");
    }
}
