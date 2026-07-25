# Dashboard & queries

## Dashboard

The landing screen shows EHR / composition / template / stored-query counts,
one tile per query group (the summed match counts of its member queries),
and a commit-activity trend rendered as pure SVG.

![Dashboard](img/dashboard/dashboard.png)

## The Query Builder

Build AQL without writing it: pick a template, pick paths from its tree,
and add typed conditions — each data type gets the right widget, populated
from the template's own constraints (coded value sets, ordinal scales,
quantity units). Conditions combine into arbitrarily nested ALL/ANY groups
with per-condition and per-group negation. The generated AQL is previewed
live and is always grammatically valid — the builder assembles the same
query syntax tree the server validates, never text.

![Query builder](img/queries/query-builder.png)

Choose what comes back: whole compositions, projected data points (with
column aliases), or a bare match count. Run pages through the result set;
save the query under a qualified name to the CDR's stored-query registry.

## The raw AQL editor

The same run/save surface for hand-written AQL: grammar validation before
anything reaches the CDR, JSON parameter bindings, paged results. The
builder's "open in raw editor" hands its generated query across.

![Raw AQL editor](img/queries/query-aql.png)

When a result column is numeric, the results pane offers a **Table |
Chart** toggle — the values drawn as a line over the (ordered) row index,
so an `ORDER BY` on a time path reads as a time series. The builder's
output shapes include **EHRs (cohort)**: the distinct EHR ids matching
the criteria tree.

![Query results](img/queries/query-aql-results.png)

![Chart view](img/queries/query-results-chart.png)

## Exporting results

Both results panes (the builder and the raw editor) offer **Export CSV**
and **Export JSON** — a plain form download that works even before the
page's WebAssembly loads. The export runs the query's own `LIMIT` window,
or the server's default fetch limit when the query has none. CSV cells
hold scalar values verbatim; structured values are embedded as compact
JSON.

## Stored queries & groups

Fresh repositories start empty, with the action that fills the screen:

![Stored queries — empty](img/queries/queries-empty.png)

List the CDR's stored queries, inspect a query's AQL, and jump into the
editor to run it. Query **groups** are console-local named sets of stored
queries whose combined match counts appear as dashboard tiles — useful as
lightweight cohort counters.

![Stored queries](img/queries/queries.png)

Each stored query row also offers **Open in editor**, which loads the
query text into the raw AQL editor (pre-filling the save name, so saving
again publishes the next version).

### Removing a group vs deleting a stored query

The two are deliberately different actions:

- **Remove group** (on a group card) deletes only the console-local
  grouping. The stored queries keep living in the CDR.
- **Delete from CDR** (on a stored-query row) deletes *that version* of the
  query from the CDR's stored-query store, for every client. It appears only
  when the CDR's admin API is enabled (`admin.enabled` /
  `EHRBASE__ADMIN__ENABLED`, off by default — see
  [`[admin]`](../installation/configuration.md#admin)); the delete itself
  additionally needs the ADMIN role, and a session without it is refused with
  a message naming what is missing.

Both open a confirmation dialog that names the exact object before anything
is sent, and a refused delete is reported with the CDR's own diagnostic and
the next action to take.

![Stored-query delete](img/queries/queries-admin-delete.png)
