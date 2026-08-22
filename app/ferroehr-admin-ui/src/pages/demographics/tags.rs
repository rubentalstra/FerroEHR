// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The demographic `ITEM_TAG` surface: the space-wide tag index the browser
//! screen lists, and the per-party tag editor on the party detail's Tags tab.
//!
//! Both are released operations. The index is
//! `GET /demographic/tags`, whose list "can be filtered by the given one or
//! more `tag_key`, `tag_value`, `tag_target_path` query parameters" and where
//! "in case no such parameter is provided then all `ITEM_TAG` resources will be
//! retrieved" (`operations/demographic_tags_get.yaml`). The editor is the
//! per-kind trio `{kind}_tags_get` / `{kind}_tags_update` /
//! `{kind}_tags_delete` (`operations/person_tags_*.yaml`).
//!
//! Two wire facts the editor is built around:
//!
//! - **A tag update replaces the WHOLE collection.** "Providing an empty list
//!   will effectively remove all `ITEM_TAG` associated with the given target"
//!   (`operations/person_tags_update.yaml`), so setting one tag means sending
//!   every tag the target should still hold — this module reads the current
//!   collection and merges. That read-modify-write has no conditional header to
//!   ride: the tag operations declare no `If-Match`, so a concurrent tag edit
//!   by another client can be lost. The panel says so.
//! - **A tag's identity is the `(key, target_path)` PAIR** — "they are uniquely
//!   identified by their `key` and `target_path` pair attributes" (same file) —
//!   which is what the merge keys on. The DELETE addresses a `key` alone, so it
//!   removes every tag sharing that key.
//!
//! `target` and `owner_id` are server-assigned and never sent: the update body
//! is the released `UpdateItemTag` shape (`key` required, optional `value` and
//! `target_path`, and nothing else — `schemas/common/UpdateItemTag.yaml` is
//! `additionalProperties: false`).

#![allow(
    clippy::disallowed_types,
    reason = "the console consumes the CDR JSON wire over ITS-REST — not the CDR internal seams \
              (#1694); the carriers here are ssr-only, so #[expect] would be unfulfilled on the \
              hydrate target"
)]

use leptos::prelude::*;
use leptos::server;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use serde_json::Value;

use crate::components::data_table::{CELL, CELL_MONO, ROW, table_shell, table_skeleton};
use crate::components::empty_state::EmptyState;
use crate::components::field::{BTN_DANGER, BTN_PRIMARY, INPUT, LABEL};
use crate::components::format_view::inline_error;
use crate::components::surface::{CARD_PAD, CARD_TITLE};
use crate::components::toast::{toast_error, toast_success};
use crate::error::AdminUiError;
use crate::pages::demographics::PartyKind;

/// One `ITEM_TAG` as the demographic tag routes serve it.
///
/// The attributes are the RM class's own
/// (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc`):
/// `key`, the optional `value`, the optional `target_path`, the `target`
/// (a `UID_BASED_ID` — a `HIER_OBJECT_ID` for a container target, an
/// `OBJECT_VERSION_ID` for one version) and the `owner_id` reference. Strings
/// only, so the type is WASM-safe over the server-fn boundary (rules §1).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ItemTagRow {
    /// `ITEM_TAG.key`.
    pub key: String,
    /// `ITEM_TAG.value`, empty when the tag carries none.
    pub value: String,
    /// `ITEM_TAG.target_path`, empty when the tag targets the whole object.
    pub target_path: String,
    /// `ITEM_TAG.target.value` — the tagged container or version id.
    pub target: String,
    /// `ITEM_TAG.owner_id.id.value` — the owning object the CDR assigned.
    pub owner_id: String,
}

impl ItemTagRow {
    /// The tag's identity — the `(key, target_path)` pair the released update
    /// operation names — as the `<For>` key (rules §4: stable, unique,
    /// data-derived).
    #[must_use]
    pub fn identity(&self) -> String {
        format!("{}\u{1f}{}", self.key, self.target_path)
    }
}

/// The demographic tag index (`GET /demographic/tags`), optionally filtered.
///
/// Each filter is sent only when non-empty, which is exactly the released
/// "omitted parameter constrains nothing" behaviour
/// (`operations/demographic_tags_get.yaml`). The list spans every party kind
/// and both target forms; a no-match answer is `200 []`, never a `404`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session; CDR transport
/// errors pass through; a non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the list is not valid JSON.
#[server]
pub async fn list_demographic_tags(
    /// Exact `ITEM_TAG.key` filter; empty constrains nothing.
    key: String,
    /// Exact `ITEM_TAG.value` filter; empty constrains nothing.
    value: String,
    /// Exact `ITEM_TAG.target_path` filter; empty constrains nothing.
    target_path: String,
) -> Result<Vec<ItemTagRow>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let mut query = String::new();
    // The released query-parameter names, which the console's own parameter
    // names deliberately do not repeat (`tag_key` on both sides would only
    // stutter).
    for (name, value) in [
        ("tag_key", key.trim()),
        ("tag_value", value.trim()),
        ("tag_target_path", target_path.trim()),
    ] {
        if value.is_empty() {
            continue;
        }
        query.push(if query.is_empty() { '?' } else { '&' });
        query.push_str(name);
        query.push('=');
        query.push_str(&urlencoding::encode(value));
    }
    let url = state.cdr.rest_v1(&format!("demographic/tags{query}"));
    let response = state
        .cdr
        .get(&session.credential, &url, "application/json")
        .await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    parse_item_tags(&response.body)
}

/// One party's `ITEM_TAG`s
/// (`GET /demographic/{kind}/{uid_based_id}/tags`).
///
/// The addressed id is the version CONTAINER, so these are the
/// `VERSIONED_PARTY`'s own tags: the released operation makes the container and
/// a single VERSION two DISJOINT collections of the same object
/// (`operations/person_tags_get.yaml`), and the console edits the container's.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an unknown kind segment; CDR transport errors
/// pass through; a non-2xx CDR answer (the `404` for an unknown id included)
/// normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the list is not valid JSON.
#[server]
pub async fn fetch_party_tags(
    /// The party family, as its route segment ([`PartyKind::segment`]).
    kind: String,
    /// The party's version container id.
    uid: String,
) -> Result<Vec<ItemTagRow>, AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let kind = PartyKind::from_segment(&kind)
        .ok_or_else(|| super::unknown_segment(&kind, "party kind"))?;
    read_party_tags(&state, &session.credential, kind, &uid).await
}

#[cfg(feature = "ssr")]
/// Read one party's container tag collection.
///
/// The plain function behind [`fetch_party_tags`], so [`set_party_tag`]'s
/// read-modify-write calls THIS rather than another public endpoint (rules §7 —
/// a server fn is thin and the logic it shares lives in an ordinary function).
///
/// # Errors
/// CDR transport errors pass through; a non-2xx answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the list is not valid JSON.
async fn read_party_tags(
    state: &crate::state::AppState,
    credential: &crate::session::Credential,
    kind: PartyKind,
    uid: &str,
) -> Result<Vec<ItemTagRow>, AdminUiError> {
    let url = tags_url(state, kind, uid);
    let response = state.cdr.get(credential, &url, "application/json").await?;
    let response = crate::cdr::CdrClient::expect_success(response)?;
    parse_item_tags(&response.body)
}

/// Set one tag on a party, keeping every other tag it already carries
/// (`PUT /demographic/{kind}/{uid_based_id}/tags`).
///
/// Read-modify-write, because the operation replaces the whole collection
/// (module docs): the current tags are read, the requested `(key, target_path)`
/// entry is inserted or replaced, and the merged list is sent. `value` and
/// `target_path` are omitted when blank rather than sent empty — `ITEM_TAG`'s
/// invariant `Inv_value_valid` refuses an empty `value`
/// (`org.openehr.rm.common.item_tag.adoc`), and an absent `target_path` is what
/// "tags the whole object" means.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an unknown kind segment or a blank key; CDR
/// transport errors pass through; any non-2xx CDR answer (the `422` for a tag
/// that breaks an `ITEM_TAG` invariant included) normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success);
/// [`AdminUiError::Internal`] when the current list is not valid JSON.
#[server]
pub async fn set_party_tag(
    /// The party family, as its route segment ([`PartyKind::segment`]).
    kind: String,
    /// The party's version container id.
    uid: String,
    /// The tag key to set.
    key: String,
    /// The tag value; blank stores a tag with no value.
    value: String,
    /// The tag's `target_path`; blank tags the whole object.
    target_path: String,
) -> Result<(), AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let kind = PartyKind::from_segment(&kind)
        .ok_or_else(|| super::unknown_segment(&kind, "party kind"))?;
    if key.trim().is_empty() {
        return Err(AdminUiError::Invalid(
            "a tag key is required (ITEM_TAG invariant Inv_key_valid: the key is not empty)"
                .to_owned(),
        ));
    }
    let existing = read_party_tags(&state, &session.credential, kind, &uid).await?;
    let body = merged_tag_body(&existing, key.trim(), value.trim(), target_path.trim());
    let url = tags_url(&state, kind, &uid);
    let response = state
        .cdr
        .put(
            &session.credential,
            &url,
            "application/json",
            "application/json",
            &[],
            body,
        )
        .await?;
    drop(crate::cdr::CdrClient::expect_success(response)?);
    Ok(())
}

/// Delete a party's tags carrying `key`
/// (`DELETE /demographic/{kind}/{uid_based_id}/tags/{key}`).
///
/// The released operation addresses a key, not a `(key, target_path)` pair, so
/// it removes every tag on the target that shares the key
/// (`operations/person_tags_delete.yaml`); an unknown key on an existing target
/// is its `404`.
///
/// # Errors
/// [`AdminUiError::Unauthenticated`] without a console session;
/// [`AdminUiError::Invalid`] on an unknown kind segment or a blank key; CDR
/// transport errors pass through; any non-2xx CDR answer normalizes via
/// [`CdrClient::expect_success`](crate::cdr::CdrClient::expect_success).
#[server]
pub async fn delete_party_tag(
    /// The party family, as its route segment ([`PartyKind::segment`]).
    kind: String,
    /// The party's version container id.
    uid: String,
    /// The tag key to remove.
    key: String,
) -> Result<(), AdminUiError> {
    let session = crate::session::require_session().await?;
    let state: crate::state::AppState = expect_context();
    let kind = PartyKind::from_segment(&kind)
        .ok_or_else(|| super::unknown_segment(&kind, "party kind"))?;
    let key = key.trim();
    if key.is_empty() {
        return Err(AdminUiError::Invalid("a tag key is required".to_owned()));
    }
    let url = format!(
        "{}/{}",
        tags_url(&state, kind, &uid),
        urlencoding::encode(key)
    );
    let response = state.cdr.delete(&session.credential, &url, &[]).await?;
    drop(crate::cdr::CdrClient::expect_success(response)?);
    Ok(())
}

#[cfg(feature = "ssr")]
/// The `/demographic/{kind}/{uid_based_id}/tags` URL of a party's container tag
/// collection.
fn tags_url(state: &crate::state::AppState, kind: PartyKind, uid: &str) -> String {
    state.cdr.rest_v1(&format!(
        "demographic/{}/{}/tags",
        kind.segment(),
        urlencoding::encode(&super::container_uid_of(uid))
    ))
}

#[cfg(feature = "ssr")]
/// The whole-collection `UpdateItemTag` body that keeps `existing` and sets one
/// `(key, target_path)` entry.
///
/// Pure and unit-tested: the merge keys on the tag identity the released update
/// operation names (module docs), and it emits only the three declared members
/// (`schemas/common/UpdateItemTag.yaml` is `additionalProperties: false`, so a
/// re-sent `target`/`owner_id` would be a `400`).
fn merged_tag_body(existing: &[ItemTagRow], key: &str, value: &str, target_path: &str) -> String {
    let mut entries: Vec<Value> = Vec::with_capacity(existing.len().saturating_add(1));
    let mut replaced = false;
    for tag in existing {
        if tag.key == key && tag.target_path == target_path {
            entries.push(update_tag(key, value, target_path));
            replaced = true;
        } else {
            entries.push(update_tag(&tag.key, &tag.value, &tag.target_path));
        }
    }
    if !replaced {
        entries.push(update_tag(key, value, target_path));
    }
    Value::Array(entries).to_string()
}

#[cfg(feature = "ssr")]
/// One `UpdateItemTag` entry: `key` always, `value`/`target_path` only when
/// non-empty.
fn update_tag(key: &str, value: &str, target_path: &str) -> Value {
    let mut entry = serde_json::Map::new();
    drop(entry.insert("key".to_owned(), Value::String(key.to_owned())));
    if !value.is_empty() {
        drop(entry.insert("value".to_owned(), Value::String(value.to_owned())));
    }
    if !target_path.is_empty() {
        drop(entry.insert(
            "target_path".to_owned(),
            Value::String(target_path.to_owned()),
        ));
    }
    Value::Object(entry)
}

#[cfg(feature = "ssr")]
/// Flatten an `ITEM_TAG` list body into [`ItemTagRow`]s. Defensive throughout —
/// an absent optional attribute reads as empty rather than failing the panel.
///
/// # Errors
/// [`AdminUiError::Internal`] when the body is not valid JSON.
fn parse_item_tags(body: &str) -> Result<Vec<ItemTagRow>, AdminUiError> {
    let doc: Value = serde_json::from_str(body)
        .map_err(|e| AdminUiError::Internal(format!("ITEM_TAG list JSON: {e}")))?;
    let items = doc.as_array().cloned().unwrap_or_default();
    Ok(items
        .iter()
        .map(|tag| ItemTagRow {
            key: tag
                .get("key")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            value: tag
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            target_path: tag
                .get("target_path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            target: super::json_str(tag, &["target", "value"]),
            owner_id: super::json_str(tag, &["owner_id", "id", "value"]),
        })
        .collect())
}

/// The party detail's **Tags** tab: the party's current tags, a set form, and a
/// per-key delete.
///
/// One resource, created in setup and gated on the tab being active (rules §6),
/// refetched after every successful write via the action's version stamp.
pub(super) fn tags_section(
    kind: PartyKind,
    uid: Signal<String>,
    selected: Memo<String>,
) -> AnyView {
    let toaster = thaw::ToasterInjection::expect_context();
    let set: Action<(String, String, String, String), Result<(), AdminUiError>> = Action::new(
        move |(uid, key, value, path): &(String, String, String, String)| {
            let (uid, key, value, path) = (uid.clone(), key.clone(), value.clone(), path.clone());
            async move { set_party_tag(kind.segment().to_owned(), uid, key, value, path).await }
        },
    );
    let remove: Action<(String, String), Result<(), AdminUiError>> =
        Action::new(move |(uid, key): &(String, String)| {
            let (uid, key) = (uid.clone(), key.clone());
            async move { delete_party_tag(kind.segment().to_owned(), uid, key).await }
        });

    let resource: Resource<Result<Option<Vec<ItemTagRow>>, AdminUiError>> = Resource::new(
        move || {
            (selected.get() == "tags")
                .then(|| (uid.get(), set.version().get(), remove.version().get()))
        },
        move |active| async move {
            match active {
                Some((id, _, _)) => fetch_party_tags(kind.segment().to_owned(), id)
                    .await
                    .map(Some),
                None => Ok(None),
            }
        },
    );

    // Every write toasts both outcomes (the console's mutation-feedback rule).
    Effect::new(move |_| match set.value().get() {
        Some(Ok(())) => toast_success(
            toaster,
            "Tag saved",
            "The party's tag collection was replaced with the merged list.",
        ),
        Some(Err(error)) => {
            crate::feedback::toast_write_failure(
                toaster,
                "Tag save failed",
                "this party's tags",
                &error,
            );
        }
        None => {}
    });
    Effect::new(move |_| match remove.value().get() {
        Some(Ok(())) => toast_success(
            toaster,
            "Tag deleted",
            "Every tag carrying that key was removed.",
        ),
        Some(Err(error)) => toast_error(
            toaster,
            "Tag delete failed",
            &crate::feedback::write_failure_copy("this party's tag", &error),
        ),
        None => {}
    });

    let table = view! {
        <Transition fallback=table_skeleton>
            {move || Suspend::new(async move {
                match resource.await {
                    Ok(Some(tags)) if tags.is_empty() => {
                        view! {
                            <EmptyState
                                icon=icondata_lu::LuTags
                                message="No tags on this party"
                                hint="Tags are free key/value markers a client sets on a party; add one below."
                            />
                        }
                            .into_any()
                    }
                    Ok(Some(tags)) => tags_table(tags, uid, remove),
                    Ok(None) => ().into_any(),
                    Err(e) => inline_error(&e),
                }
            })}
        </Transition>
    }
    .into_any();

    let form = set_form(uid, set);
    view! {
        <div class="flex flex-col gap-4">
            <section class=CARD_PAD>
                <h2 class=CARD_TITLE>"Tags"</h2>
                <p class="mb-3 text-xs text-ink-muted">
                    "These are the VERSIONED_PARTY container's tags. Saving one re-sends the whole collection, which is how the openEHR tag update works — a tag another client added in the meantime can be lost, so reload before editing a busy party."
                </p>
                {table}
            </section>
            {form}
        </div>
    }
    .into_any()
}

/// The party's tags in the shared table kit, each row with its delete.
fn tags_table(
    tags: Vec<ItemTagRow>,
    uid: Signal<String>,
    remove: Action<(String, String), Result<(), AdminUiError>>,
) -> AnyView {
    let rows = view! {
        <For each=move || tags.clone() key=ItemTagRow::identity let:tag>
            {tag_row(&tag, uid, remove)}
        </For>
    }
    .into_any();
    table_shell(&["Key", "Value", "Target path", "Target", ""], rows)
}

/// One tag row plus its delete-by-key action.
fn tag_row(
    tag: &ItemTagRow,
    uid: Signal<String>,
    remove: Action<(String, String), Result<(), AdminUiError>>,
) -> AnyView {
    let key = tag.key.clone();
    let hook = tag.key.clone();
    let shown_key = tag.key.clone();
    let value = tag.value.clone();
    let target_path = tag.target_path.clone();
    let target = tag.target.clone();
    view! {
        <tr class=ROW>
            <td class=CELL_MONO data-tag-key=hook.clone()>
                {shown_key}
            </td>
            <td class=CELL>{value}</td>
            <td class=CELL_MONO>{target_path}</td>
            <td class=CELL_MONO>{target}</td>
            <td class=CELL>
                <button
                    type="button"
                    class=BTN_DANGER
                    data-tag-delete=hook
                    disabled=Signal::derive(move || remove.pending().get())
                    on:click=move |_| {
                        remove.dispatch((uid.get_untracked(), key.clone()));
                    }
                >
                    <leptos_icons::Icon icon=icondata_lu::LuTrash width="14" height="14" />
                    "Delete"
                </button>
            </td>
        </tr>
    }
    .into_any()
}

/// The set-a-tag card: key, value, target path, and the save that merges into
/// the current collection.
///
/// Uncontrolled inputs read at dispatch (rules §5) — a controlled input resets
/// to its empty signal at hydration, wiping anything typed before the WASM
/// loaded.
fn set_form(
    uid: Signal<String>,
    set: Action<(String, String, String, String), Result<(), AdminUiError>>,
) -> AnyView {
    let key_ref = NodeRef::<leptos::html::Input>::new();
    let value_ref = NodeRef::<leptos::html::Input>::new();
    let path_ref = NodeRef::<leptos::html::Input>::new();
    let validation = RwSignal::new(Option::<String>::None);
    let on_save = move |_| {
        let field = |node: NodeRef<leptos::html::Input>| {
            node.get_untracked()
                .map(|el| el.value())
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let key = field(key_ref);
        if key.is_empty() {
            validation.set(Some(
                "A tag needs a key (ITEM_TAG invariant Inv_key_valid).".to_owned(),
            ));
            return;
        }
        validation.set(None);
        set.dispatch((uid.get_untracked(), key, field(value_ref), field(path_ref)));
    };
    view! {
        <section class=CARD_PAD id="party-tag-set">
            <h2 class=CARD_TITLE>"Set a tag"</h2>
            <div class="flex flex-wrap items-end gap-3">
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="tag-key">
                        "Key"
                    </label>
                    <input
                        id="tag-key"
                        type="text"
                        class=INPUT
                        placeholder="flag"
                        node_ref=key_ref
                    />
                </div>
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="tag-value">
                        "Value (optional)"
                    </label>
                    <input
                        id="tag-value"
                        type="text"
                        class=INPUT
                        placeholder="follow-up"
                        node_ref=value_ref
                    />
                </div>
                <div class="flex flex-col gap-1">
                    <label class=LABEL r#for="tag-target-path">
                        "Target path (optional)"
                    </label>
                    <input
                        id="tag-target-path"
                        type="text"
                        class=INPUT
                        placeholder="/details/items[at0001]/value"
                        node_ref=path_ref
                    />
                </div>
                <button
                    id="tag-save"
                    type="button"
                    class=BTN_PRIMARY
                    disabled=Signal::derive(move || set.pending().get())
                    on:click=on_save
                >
                    "Save tag"
                </button>
            </div>
            <p class="mt-2 text-xs text-ink-muted">
                "A tag is identified by its key and target path together, so the same key on two different paths is two tags. Deleting addresses the key alone and removes both."
            </p>
            {move || {
                validation
                    .get()
                    .map(|message| {
                        view! {
                            <p
                                role="alert"
                                class="mt-2 rounded-control border border-danger/40 bg-danger-subtle px-3 py-2 text-sm text-danger"
                            >
                                {message}
                            </p>
                        }
                    })
            }}
        </section>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::ItemTagRow;

    #[test]
    fn a_tags_identity_is_its_key_and_target_path_together() {
        let bare = ItemTagRow {
            key: "flag".to_owned(),
            ..ItemTagRow::default()
        };
        let pathed = ItemTagRow {
            key: "flag".to_owned(),
            target_path: "/details".to_owned(),
            ..ItemTagRow::default()
        };
        // The released update operation identifies a tag by the PAIR, so two
        // same-key tags on different paths must be distinct `<For>` keys.
        assert_ne!(bare.identity(), pathed.identity());
        assert_eq!(bare.identity(), bare.clone().identity());
    }
}

#[cfg(all(test, feature = "ssr"))]
mod wire_tests {
    use super::{ItemTagRow, merged_tag_body, parse_item_tags};
    use serde_json::Value;

    #[test]
    fn parses_a_served_tag_list_including_the_optional_attributes() {
        let body = r#"[
            {"_type":"ITEM_TAG","key":"flag","value":"follow-up",
             "target":{"_type":"HIER_OBJECT_ID","value":"8849182c"},
             "owner_id":{"_type":"OBJECT_REF","namespace":"local","type":"SYSTEM","id":{"_type":"HIER_OBJECT_ID","value":"example.org"}}},
            {"_type":"ITEM_TAG","key":"reviewed","target_path":"/details/items[at0001]/value",
             "target":{"_type":"OBJECT_VERSION_ID","value":"8849182c::example.org::2"}}
        ]"#;
        let rows = parse_item_tags(body).expect("a valid list");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key, "flag");
        assert_eq!(rows[0].value, "follow-up");
        assert_eq!(rows[0].target, "8849182c");
        assert_eq!(rows[0].owner_id, "example.org");
        // An absent optional attribute reads as empty, never as a failure.
        assert_eq!(rows[0].target_path, "");
        assert_eq!(rows[1].value, "");
        assert_eq!(rows[1].target_path, "/details/items[at0001]/value");
        // An empty collection is a first-class answer, not an error.
        assert!(parse_item_tags("[]").expect("empty list").is_empty());
        assert!(parse_item_tags("not json").is_err());
    }

    fn row(key: &str, value: &str, target_path: &str) -> ItemTagRow {
        ItemTagRow {
            key: key.to_owned(),
            value: value.to_owned(),
            target_path: target_path.to_owned(),
            target: "8849182c".to_owned(),
            owner_id: "example.org".to_owned(),
        }
    }

    #[test]
    fn setting_a_new_tag_keeps_every_existing_one() {
        let body = merged_tag_body(&[row("flag", "follow-up", "")], "reviewed", "true", "");
        let entries: Vec<Value> = serde_json::from_str(&body).expect("a JSON array");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["key"], "flag");
        assert_eq!(entries[0]["value"], "follow-up");
        assert_eq!(entries[1]["key"], "reviewed");
        // Only the three declared UpdateItemTag members are ever sent — the
        // schema is additionalProperties: false, so target/owner_id would 400.
        for entry in &entries {
            let keys: Vec<&String> = entry.as_object().expect("an object entry").keys().collect();
            assert!(
                keys.iter()
                    .all(|k| matches!(k.as_str(), "key" | "value" | "target_path")),
                "{keys:?}"
            );
        }
    }

    #[test]
    fn setting_the_same_key_and_path_replaces_that_one_entry() {
        let body = merged_tag_body(
            &[row("flag", "old", ""), row("flag", "kept", "/details")],
            "flag",
            "new",
            "",
        );
        let entries: Vec<Value> = serde_json::from_str(&body).expect("a JSON array");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["value"], "new");
        // The same key on a DIFFERENT target_path is a different tag and stays.
        assert_eq!(entries[1]["value"], "kept");
        assert_eq!(entries[1]["target_path"], "/details");
    }

    #[test]
    fn a_blank_value_or_path_is_omitted_rather_than_sent_empty() {
        // ITEM_TAG Inv_value_valid refuses an empty value; an absent
        // target_path is what "tags the whole object" means.
        let body = merged_tag_body(&[], "flag", "", "");
        let entries: Vec<Value> = serde_json::from_str(&body).expect("a JSON array");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].get("value").is_none());
        assert!(entries[0].get("target_path").is_none());
        assert_eq!(entries[0]["key"], "flag");
    }
}
