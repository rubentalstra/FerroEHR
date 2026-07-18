//! The shared table shell: one styled `<table>` used by every listing
//! screen (EHRs, compositions, stored queries, results) so tables stop
//! being hand-rolled per page. Headers are NAMED — raw AQL column indexes
//! (`#0`…) must never reach a header cell.

use leptos::prelude::*;

/// Class set for a body row (hover tint; hairline separators).
pub const ROW: &str = "border-b border-edge last:border-0 hover:bg-sunken/60";

/// Class set for a body cell.
pub const CELL: &str = "px-3 py-2 align-top";

/// Class set for a monospace body cell (ids, paths, AQL).
pub const CELL_MONO: &str = "px-3 py-2 align-top font-mono text-xs";

/// The styled table shell around pre-rendered `<tr>` rows.
///
/// Renders the card surface, the muted uppercase header row, and an
/// explicit `<tbody>` (hydration correctness: browsers insert one
/// otherwise, breaking DOM↔view correspondence — rules §8). `body` is the
/// collected `<tr>` views; build cells with [`CELL`]/[`CELL_MONO`] and
/// rows with [`ROW`].
#[must_use]
pub fn table_shell(headers: &[&str], body: AnyView) -> AnyView {
    let head = headers
        .iter()
        .map(|h| {
            view! {
                <th class="px-3 py-2 text-left text-xs font-semibold uppercase tracking-wide text-ink-muted">
                    {h.to_string()}
                </th>
            }
        })
        .collect_view();
    view! {
        <div class="overflow-x-auto rounded-card border border-edge bg-raised shadow-card">
            <table class="w-full border-collapse text-sm text-ink">
                <thead class="border-b border-edge bg-sunken">
                    <tr>{head}</tr>
                </thead>
                <tbody>{body}</tbody>
            </table>
        </div>
    }
    .into_any()
}
