//! The EHR-detail Status tab: the EHR's `EHR_STATUS` resource.

use leptos::prelude::*;
use leptos::server;
use serde_json::Value;

use crate::components::format_view::DocumentPane;
use crate::components::surface::CARD_PAD;
use crate::error::AdminUiError;
use crate::pages::ehrs::table_skeleton;

/// The EHR's `EHR_STATUS` resource, raw canonical JSON.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn fetch_ehr_status(ehr_id: String) -> Result<String, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = leptos::prelude::expect_context();
    let url = state
        .cdr
        .rest_v1(&format!("ehr/{}/ehr_status", urlencoding::encode(&ehr_id)));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    Ok(crate::cdr::CdrClient::expect_success(response)?.body)
}

/// Status tab: `fetch_ehr_status` → queryable/modifiable badges, the subject,
/// and the raw JSON in a [`DocumentPane`]. The source is gated on the tab
/// being active so it fetches only when shown.
pub(super) fn status_section(ehr_id: Signal<String>, selected: Memo<String>) -> AnyView {
    let resource = Resource::new(
        move || (selected.get() == "status").then(|| ehr_id.get()),
        |active| async move {
            match active {
                Some(id) => fetch_ehr_status(id).await.map(Some),
                None => Ok(None),
            }
        },
    );
    view! {
        <Suspense fallback=table_skeleton>
            {move || Suspend::new(async move {
                let rendered = resource
                    .await
                    .and_then(|opt| match opt {
                        Some(body) => status_body(&body),
                        None => Ok(().into_any()),
                    });
                match rendered {
                    Ok(view) => view,
                    Err(e) => crate::components::format_view::inline_error(&e),
                }
            })}
        </Suspense>
    }
    .into_any()
}

/// Render an `EHR_STATUS` document: the two capability badges, the subject
/// reference, and the raw JSON.
///
/// # Errors
/// [`AdminUiError::Internal`] when the body is not valid JSON.
fn status_body(body: &str) -> Result<AnyView, AdminUiError> {
    let doc: Value = serde_json::from_str(body)
        .map_err(|e| AdminUiError::Internal(format!("ehr_status JSON: {e}")))?;
    let queryable = doc
        .get("is_queryable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let modifiable = doc
        .get("is_modifiable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let subject = doc
        .get("subject")
        .and_then(|s| s.get("external_ref"))
        .and_then(|r| r.get("id"))
        .and_then(|i| i.get("value"))
        .and_then(Value::as_str)
        .map_or_else(|| "self (no external subject)".to_owned(), str::to_owned);
    let pretty =
        crate::components::format_view::pretty_body(body, crate::format::ReprFormat::CanonicalJson);
    let doc_sig = RwSignal::new(pretty);
    Ok(view! {
        <div class=format!("{CARD_PAD} flex flex-col gap-3")>
            <div class="flex flex-wrap gap-2 items-center">
                {capability_badge("queryable", queryable)}
                {capability_badge("modifiable", modifiable)}
            </div>
            <div class="text-sm">
                <span class="font-medium text-ink-muted">"subject: "</span>
                <span class="font-mono break-all text-ink">{subject}</span>
            </div>
            {(!queryable)
                .then(|| {
                    view! {
                        <div
                            role="status"
                            class="rounded-control border border-warn/40 bg-warn-subtle px-3 py-2 text-sm text-warn"
                        >
                            "This EHR is not queryable — AQL over it returns nothing."
                        </div>
                    }
                })}
            <DocumentPane body=doc_sig />
        </div>
    }
    .into_any())
}

/// An ok/danger capability chip for an `EHR_STATUS` boolean flag.
fn capability_badge(label: &'static str, on: bool) -> AnyView {
    let (mark, class) = if on {
        ("✓", "bg-ok-subtle text-ok")
    } else {
        ("✗", "bg-danger-subtle text-danger")
    };
    view! {
        <span class=format!(
            "inline-flex items-center gap-1 rounded-control px-2 py-0.5 text-xs font-medium {class}",
        )>{mark} " " {label}</span>
    }
    .into_any()
}
