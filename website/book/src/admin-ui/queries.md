# Dashboard & queries

## Dashboard

The landing screen shows EHR / composition / template / stored-query counts,
one tile per **stored-query namespace** (the summed match counts of the
queries in it), and a commit-activity trend rendered as pure SVG.

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
save the query to the CDR's stored-query registry under a namespace and a
name (see [Grouping is the namespace](#grouping-is-the-namespace)).

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

## Stored queries & namespaces

Fresh repositories start empty, with the action that fills the screen:

![Stored queries — empty](img/queries/queries-empty.png)

List the CDR's stored queries, inspect a query's AQL, and jump into the
editor to run it.

![Stored queries](img/queries/queries.png)

Each stored query row also offers **Open in editor**, which loads that
version's query text into the raw AQL editor and pre-fills the namespace,
name, and version fields — with the version set to the *next* one, so
saving again publishes a new version instead of colliding with the one you
opened (see [Versions](#versions)).

### Grouping is the namespace

A stored query is identified by a qualified name — `namespace::name`, the
namespace optional and, when present, a reverse domain name whose purpose in
the openEHR REST specification is exactly "separation of use of stored
queries by teams, companies, etc."

The console therefore does **not** invent a grouping of its own: a query's
group *is* its namespace, chosen when you save it. The right-hand panel on
**Queries** and the cohort tiles on the **Dashboard** are both derived live
from `GET /definition/query`. There is nothing to create, edit, or remove —
and nothing kept on the console's disk, so the grouping is durable in the
CDR and reads identically for every openEHR client and every console
replica. Queries saved without a namespace collect under **unqualified**.

Both save surfaces (the builder and the raw editor) therefore offer the
**Namespace** field beside the **Query name**, and show the exact qualified
name the save will write. Typing the whole `namespace::name` into the name
field works too.

### Versions

A stored query is identified by its qualified name **and a version**, and the
version is SEMVER-style — `major.minor.patch`. The save surfaces expose it as
an optional **Version** field, and the line under the fields always states
which of the two openEHR store operations a click will perform:

| Version field | What a save does |
| --- | --- |
| empty | `PUT /definition/query/{name}` — the server assigns the version and **replaces** the query stored at it |
| `1.2.0` | `PUT /definition/query/{name}/{version}` — stores a **new, immutable** version; if that exact `(name, version)` pair already exists the CDR refuses it (`409`) and the console says so |

Because an explicit version is immutable, **Open in editor** proposes the next
minor version (opening `1.0.0` fills the field with `1.1.0`) — edit, save, and
both versions are then listed side by side. Which part to bump is yours to
change; the field is free text and only checks that a version you type is a
complete triple.

A shorter pattern like `1` or `1.0` is a **read** form, not a store form: when
*fetching or executing* a stored query, openEHR resolves a partial version to
the latest one matching that prefix, and omitting the version entirely means
the latest of all. The console therefore refuses a partial version in the save
field (with an explanation) rather than filing a definition under a string that
later lookups would treat as a pattern.

### Deleting a stored query

**Delete from CDR** (on a stored-query row) deletes *that version* of the
query from the CDR's stored-query store, for every client — the only
destructive action on the screen. It appears only when the CDR's admin API is
enabled (`admin.enabled` / `EHRBASE__ADMIN__ENABLED`, off by default — see
[`[admin]`](../installation/configuration.md#admin)); the delete itself
additionally needs the ADMIN role, and a session without it is refused with
a message naming what is missing.

It opens a confirmation dialog that names the exact query and version before
anything is sent, and a refused delete is reported with the CDR's own
diagnostic and the next action to take. Deleting the last query of a
namespace simply makes that namespace stop appearing.

![Stored-query delete](img/queries/queries-admin-delete.png)
