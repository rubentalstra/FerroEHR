# Querying with AQL

The **Archetype Query Language (AQL 1.1)** is how you read data out of
EHRbase-rs. Instead of querying hidden database tables, you query the clinical
model directly: you name the RM types and archetypes you want, express
structural nesting with `CONTAINS`, and select values by their path within an
archetype. The same query runs unchanged on any conformant openEHR system. This
chapter is a practical walkthrough — the language, how to run queries over
HTTP, parameters, stored queries, version scope, terminology, pagination, and
the supported feature envelope.

<!-- toc -->

## The shape of a query

An AQL statement has the familiar `SELECT … FROM … WHERE … ORDER BY` skeleton,
but the "tables" are RM types and the "columns" are archetype paths:

```text
SELECT
    c/name/value AS composition_name,
    o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic
FROM EHR e
    CONTAINS COMPOSITION c
        CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]
WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 140
ORDER BY systolic DESC
```

- **`FROM`** binds variables to RM types (`EHR e`, `COMPOSITION c`,
  `OBSERVATION o`). A type can be constrained by archetype id in square brackets
  (`OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]`).
- **`CONTAINS`** expresses structural containment — "an EHR that contains a
  composition that contains a blood-pressure observation". Chains can nest
  several deep, and combine with `AND`, `OR`, and `NOT`.
- **`SELECT`** projects values by path. Paths use archetype node ids
  (`at0004`) and RM attribute names (`value/magnitude`); `AS` names a column.
- **`WHERE`** filters on typed leaf values, with comparisons, `EXISTS`, `LIKE`,
  `MATCHES`, and boolean combinators.
- **`ORDER BY`**, **`LIMIT`**, and **`OFFSET`** behave as you expect; quantities
  order by their openEHR magnitude semantics.

## Running a query over HTTP

The query API lives under the base path at `/query/aql`. The simplest form is a
`POST` with a JSON body:

```shell
curl -u ehrbase:ehrbase \
  -H 'Content-Type: application/json' \
  -d '{"q":"SELECT e/ehr_id/value FROM EHR e"}' \
  http://localhost:8080/ehrbase/rest/openehr/v1/query/aql
```

The body fields are:

| Field | Meaning |
|---|---|
| `q` | The AQL text (**required**). |
| `offset` | Rows to skip (default 0). |
| `fetch` | Maximum rows to return. |
| `query_parameters` | An object of named parameter values (see below). |

There is also a `GET /query/aql` form taking `q`, `offset`, `fetch`, an optional
`ehr_id`, and `query_parameters` as query-string parameters — convenient for
simple, cacheable reads.

The query API is **JSON only** (`Accept: application/json`).

## The result set

A query returns a `RESULT_SET`: a description of the columns and an array of row
tuples.

```json
{
  "q": "SELECT e/ehr_id/value FROM EHR e",
  "columns": [
    { "name": "#0", "path": "/ehr_id/value" }
  ],
  "rows": [
    [ "7d44b88c-4199-4bad-9764-5da0e2a97441" ],
    [ "b1e2c3d4-5678-90ab-cdef-1234567890ab" ]
  ]
}
```

Each entry in `columns` names the column — the `AS` alias, or `#<index>` when
you did not alias it — and its `path`. Each row in `rows` is an array of cells,
one per column in column order. A cell can be a scalar or a full RM object
(for example `{"_type":"DV_TEXT","value":"Labs"}`) depending on what you
selected. The response also carries a `meta` block (schema version, creation
time, the executed AQL).

## Parameters

Parameterise a query with named placeholders (a name preceded by a dollar sign)
and supply the values in `query_parameters`. This is the safe way to inject
values — no string concatenation:

```shell
curl -u ehrbase:ehrbase -H 'Content-Type: application/json' -d '{
  "q": "SELECT c FROM EHR e CONTAINS COMPOSITION c WHERE c/name/value = $name",
  "query_parameters": { "name": "Vital signs" }
}' http://localhost:8080/ehrbase/rest/openehr/v1/query/aql
```

## Stored queries

You can register a query once, under a qualified name and version, and execute
it by name later. Storing is done through the definition API with the AQL as a
plain-text body; executing is done through the query API.

```shell
# Store a query as org.example::bp_over, version 1.0.0
curl -u ehrbase:ehrbase -X PUT \
  -H 'Content-Type: text/plain' \
  --data-binary 'SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]' \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.example::bp_over/1.0.0

# List and fetch stored queries
curl -u ehrbase:ehrbase \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/query/org.example::bp_over

# Execute it
curl -u ehrbase:ehrbase \
  http://localhost:8080/ehrbase/rest/openehr/v1/query/org.example::bp_over/1.0.0
```

- `PUT /definition/query/{name}[/{version}]` — store (version is a SemVer;
  storing an existing version returns **409**).
- `GET /definition/query/{name}[/{version}]` — list or fetch.
- `GET|POST /query/{name}[/{version}]` — execute, taking the same `offset`,
  `fetch`, and `query_parameters` as ad-hoc queries. A version can be given
  exactly (`1.0.0`) or as a prefix (`1`).

## Version scope: LATEST_VERSION and ALL_VERSIONS

By default a query sees the **latest** version of each object. EHRbase-rs also
supports querying the **entire version history** — a capability many CDRs lack.
Wrap a source in `VERSION` and choose the scope:

```text
SELECT v/commit_audit/time_committed, c/name/value
FROM EHR e
    CONTAINS VERSION v[ALL_VERSIONS]
        CONTAINS COMPOSITION c
```

`LATEST_VERSION` (the default) reads only current versions; `ALL_VERSIONS` reads
across history, so you can see how a record changed over time. The `VERSION`
variable also exposes commit metadata — the audit, the committed time, and the
version uid.

## Terminology in queries

Value filters can be backed by terminology in three ways:

- `TERMINOLOGY('expand', …)` as (or inside) a `matches` operand expands a
  value set so a coded field matches any code in it, rather than listing
  codes by hand;
- `TERMINOLOGY('validate'|'subsumes', …) = true` as a boolean condition
  evaluates a code-membership or subsumption test once per query;
- a terminology URI operand — `matches { terminology://… }` — expands the
  set the URI identifies.

These require a terminology source; if external terminology is not
configured, the in-process openEHR bundle is used. See
[Terminology servers](beyond-core/terminology.md) for wiring an external
FHIR terminology server.

## Pagination and limits

Combine `LIMIT`/`OFFSET` in the AQL with the `fetch`/`offset` request
parameters to page through large result sets. When you ask for more than the
server will return in one response, page with `offset`. Queries that run too
long return **408 Request Timeout** — narrow the query (add archetype
constraints or a `WHERE` filter) rather than retrying unchanged.

> [!TIP]
> The more specific your `FROM`/`CONTAINS` (name the archetype, scope by
> `ehr_id`), the faster the query: those constraints map to indexed columns,
> while broad "everything that contains anything" queries do the most work.

## What is supported

EHRbase-rs implements the core AQL 1.1 envelope and rejects out-of-envelope
constructs with an explicit, typed error rather than silently returning wrong
results. Supported today includes:

- `SELECT` of paths, literals, aliases, `DISTINCT`, and the aggregates
  `COUNT` (including `COUNT(DISTINCT)`), `MIN`, `MAX`, `SUM`, `AVG`;
- `FROM` over `EHR`, `VERSION` (`LATEST_VERSION` / `ALL_VERSIONS`), and RM
  classes with archetype and name predicates;
- `CONTAINS` trees with `AND`, `OR`, and `NOT CONTAINS`;
- `WHERE` comparisons on typed leaves (with openEHR magnitude ordering for
  quantities), `EXISTS`, `LIKE`, `MATCHES` value lists, and range predicates;
- `ORDER BY` typed leaves, `LIMIT`/`OFFSET`, named query parameters, and the
  `ehr_id`, `offset`, and `fetch` request parameters;
- the single-row functions: `LENGTH`, `SUBSTRING`, `POSITION`, the string
  `CONTAINS`, `CONCAT`/`CONCAT_WS`, `ABS`, `MOD`, `CEIL`, `FLOOR`, `ROUND`,
  and `CURRENT_DATE`/`CURRENT_TIME`/`CURRENT_DATE_TIME`/`NOW`/
  `CURRENT_TIMEZONE`;
- terminology-backed `TERMINOLOGY()` operands and terminology-URI `matches`
  operands (see above).

Semantic analysis is strict: duplicate `FROM` variable names, `LIMIT 0`,
negative `OFFSET`, wrong function arity, and `SUM`/`AVG` over non-numeric
paths are all rejected with a clear message. Variable names are
case-insensitive, as the specification requires.

Where a construct is outside the supported set, the server returns a clear
error identifying it — you never get a silently incorrect answer.
