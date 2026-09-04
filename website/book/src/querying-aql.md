# Querying with AQL

The **Archetype Query Language (AQL 1.1)** is how you read data out of FerroEHR.
Instead of querying hidden database tables, you query the clinical model
directly: you name the RM types and archetypes you want, express structural
nesting with `CONTAINS`, and select values by their path within an archetype. The
same query runs unchanged on any conformant openEHR system. This chapter is a
practical walkthrough: the language, how to run queries over HTTP, parameters,
stored queries, version scope, terminology, pagination and limits, and the
supported feature envelope.

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
  (`OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]`). Naming a
  **parent** archetype also returns data recorded under its specialisations, as
  openEHR requires: for ADL 1.4 identifiers the specialisation is the
  hyphen-extended concept (`…blood_pressure` matches `…blood_pressure-cuff`),
  and for ADL 2 identifiers (where the hyphen carries no such meaning) the
  lineage is read from the ADL 2 archetypes and templates you have uploaded, so
  the parent matches every stored specialisation of it whatever its concept is
  named. In both cases the major version is a hard boundary: `.v1` never matches
  `.v2` data.
- **`CONTAINS`** expresses structural containment: "an EHR that contains a
  composition that contains a blood-pressure observation". Chains can nest
  several deep, and combine with `AND`, `OR`, and `NOT`. Folder containment
  follows the RM's reference model: `FOLDER f CONTAINS COMPOSITION c` matches
  the compositions a folder's `items` reference, transitively over the
  folder's sub-tree, and `FOLDER f1 CONTAINS FOLDER f2` matches strict
  sub-folders plus the folders of a versioned folder the `items` reference —
  one reference hop, so a chain of references is expressed by chaining
  `CONTAINS`. A pair the RM defines no containment relationship for (say
  `COMPOSITION CONTAINS COMPOSITION`) is refused with a typed error.
- **`SELECT`** projects values by path. Paths use archetype node ids (`at0004`)
  and RM attribute names (`value/magnitude`); `AS` names a column.
- **`WHERE`** filters on typed leaf values, with comparisons, `EXISTS`, `LIKE`,
  `MATCHES`, and boolean combinators. Comparisons over multi-valued paths use
  any-match semantics: when a path matches several nodes or several elements of
  a list attribute (`links`, `participations`, `identifiers`, ...), the
  predicate holds if **any** matched value satisfies it (the AQL specification
  is silent here; any-match is this engine's documented convention,
  deterministic and index-friendly). Projecting a path that crosses a
  list-valued attribute returns every match as one JSON array cell, and `null`
  where nothing matches.
- **`ORDER BY`**, **`LIMIT`**, and **`OFFSET`** behave as you expect; quantities
  order by their openEHR magnitude semantics.
- **Date/time comparisons accept reduced precision** on both sides: openEHR
  admits partial values (`2019`, `1985-06`), and the engine compares them by
  flooring to the first instant they contain (a partial date assumes the first
  month/day, a partial time assumes zero). A comparison value that is not an
  ISO 8601 date/time at all is refused with a 400.

## Running a query over HTTP

The query API lives under the base path at `/query/aql`. The simplest form is a
`POST` with a JSON body:

```shell
curl -u ferroehr:ferroehr \
  -H 'Content-Type: application/json' \
  -d '{"q":"SELECT e/ehr_id/value FROM EHR e"}' \
  http://localhost:8080/ferroehr/rest/openehr/v1/query/aql
```

The body fields are:

| Field | Meaning |
|---|---|
| `q` | The AQL text (**required**). |
| `offset` | Rows to skip (default 0). |
| `fetch` | Maximum rows to return. |
| `query_parameters` | An object of named parameter values (see below). |

There is also a `GET /query/aql` form taking `q`, `offset`, `fetch`, an optional
`ehr_id`, and `query_parameters` as query-string parameters, convenient for
simple, cacheable reads.

`offset`, `fetch`, `ehr_id` and named parameters are accepted in the query string
on the `POST` forms too. A value supplied in **both** the body and the URL must
agree; a disagreement is a **400 Bad Request** rather than a silent choice
between them.

The query API is **JSON only** (`Accept: application/json`).

### Scoping to an EHR

You can restrict a query to one EHR without writing the constraint into the AQL:
pass an `ehr_id` query-string parameter, or the `openehr-ehr-id` request header.
Both forms work on **every** execution endpoint: ad-hoc and stored, `GET` and
`POST` alike. (`openEHR-EHR-id` is the deprecated spelling of the same header and
still resolves, HTTP header names being case-insensitive.)

If a request carries **both** forms they must name the **same** EHR; a request
whose parameter and header name **different** EHRs is self-contradictory and is
rejected with a **400 Bad Request**. So is a request repeating the header with
two different values. An empty header value counts as "not supplied" and
conflicts with nothing.

The id must exist: a **malformed** id is a **400**, and a well-formed id that
matches **no EHR** is an honest **404 Not Found** rather than an empty result
set, so a typo cannot masquerade as "no data".

## The result set

A query returns a `RESULT_SET`: a description of the columns and an array of row
tuples.

```json
{
  "meta": {
    "_type": "RESULTSET",
    "_schema_version": "1.0.0",
    "_created": "…",
    "_executed_aql": "SELECT e/ehr_id/value FROM EHR e"
  },
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

Each entry in `columns` names the column (the `AS` alias, or `#<index>` when you
did not alias it) and its `path`. Each row in `rows` is an array of cells, one
per column in column order. A cell can be a scalar or a full RM object (for
example `{"_type":"DV_TEXT","value":"Labs"}`) depending on what you selected. A
stored-query execution additionally carries the query's `name`.

The `meta` block's `_executed_aql` field is the AQL the server actually ran, with
your named parameters substituted in as literals. Paste it straight back into an
ad-hoc query when debugging a parameterised call. The top-level `q` keeps the
text exactly as you submitted it, and `_created` stamps when this response was
produced.

Query responses carry a weak **`ETag`** that is a content digest of the result
set: two runs returning identical results carry the identical tag, so a client
can cheaply detect "nothing changed" between polls. The digest deliberately
covers the query, the executed AQL, the columns and the rows, and **not** the
per-response `_created` stamp, so an unchanged result does not mint a new tag
every second.

## Parameters

Parameterise a query with named placeholders (a name preceded by a dollar sign)
and supply the values in `query_parameters`. This is the safe way to inject
values, with no string concatenation:

```shell
curl -u ferroehr:ferroehr -H 'Content-Type: application/json' -d '{
  "q": "SELECT c FROM EHR e CONTAINS COMPOSITION c WHERE c/name/value = $name",
  "query_parameters": { "name": "Vital signs" }
}' http://localhost:8080/ferroehr/rest/openehr/v1/query/aql
```

On the URL, each parameter can also be its **own query-string key**, which is
the form the openEHR request documentation shows and is usually easier to build
by hand:

```shell
curl -u ferroehr:ferroehr --get \
  --data-urlencode 'q=SELECT c FROM EHR e CONTAINS COMPOSITION c WHERE c/name/value = $name' \
  --data-urlencode 'name=Vital signs' \
  http://localhost:8080/ferroehr/rest/openehr/v1/query/aql
```

Three details about the URL form: the leading `$` is optional (`$name` and `name`
bind the same parameter); a value that parses as a JSON scalar binds as that type
(`36` as a number, `true` as a boolean) while everything else binds as text; and
the reserved keys `q`, `offset`, `fetch`, `ehr_id` and `query_parameters` are
request controls, never parameters. Where the same name appears both as its own
key and inside `query_parameters`, the named key wins.

## Stored queries

You can register a query once, under a qualified name and version, and execute it
by name later. Storing is done through the definition API with the AQL as a
plain-text body; executing is done through the query API.

```shell
# Store a query as org.example::bp_over, version 1.0.0
curl -u ferroehr:ferroehr -X PUT -i \
  -H 'Content-Type: text/plain' \
  --data-binary 'SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2]' \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/query/org.example::bp_over/1.0.0

# List and fetch stored queries
curl -u ferroehr:ferroehr \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/query/org.example::bp_over

# Execute it
curl -u ferroehr:ferroehr \
  http://localhost:8080/ferroehr/rest/openehr/v1/query/org.example::bp_over/1.0.0
```

**Storing.** Both store forms answer **200 OK** with an empty body and a
`Location` header naming exactly the version that was written:

- `PUT /definition/query/{name}/{version}` stores at that exact SemVer. The pair
  is immutable (re-storing an existing `(name, version)` is a **409 Conflict**,
  never an overwrite) and a partial or malformed version segment is a **400**.
- `PUT /definition/query/{name}` (no version) stores or updates at the default
  version `1.0.0`.

The body must be `text/plain`: declaring another media type is a **415**, and an
absent `Content-Type` reads as the plain-text body. The AQL is parsed at store
time, so a syntactically invalid query is rejected **400**; the server never
stores a query it cannot execute. An optional `query_type` parameter names the
formalism, default `AQL` (case-insensitive); anything else is rejected **400**
with an honest "unsupported formalism" message rather than a misleading "invalid
AQL".

**Naming.** The qualified name is `[{namespace}::]{query-name}`, and the
three-part `{namespace}::{formalism}::{query-name}` form is recognised as well.
The namespace is optional: a bare name is stored under the assumed namespace
`misc`, so `my_compositions` and `misc::my_compositions` are the same query and
listings show the qualified form. Identity is case-insensitive while the casing
you stored is preserved. The query-name `aql` is **reserved** (case-insensitive; it would collide with
the ad-hoc `/query/aql` route) and is rejected with a
**400**.

**Reading and executing.**

- `GET /definition/query/{name}` lists the queries under that name as a prefix
  pattern; `GET /definition/query` (no name at all) lists every stored query, a
  FerroEHR convenience, since the openEHR API defines only the named form.
- `GET /definition/query/{name}/{version}` fetches one, by exact SemVer or by
  prefix (`1`, `1.0` → the highest matching stored version).
- `GET|POST /query/{name}` executes the **latest** version;
  `GET|POST /query/{name}/{version}` executes that version, again by exact SemVer
  or prefix. Both take the same `offset`, `fetch`, `ehr_id` and named parameters
  as ad-hoc queries; a `POST` body of `{}` executes a parameterless stored query.

Deleting a stored query is an admin operation; see
[Admin & messaging APIs](operations-admin-apis.md).

## Version scope: LATEST_VERSION and ALL_VERSIONS

By default a query sees the **latest** version of each object. FerroEHR also
supports querying the **entire version history**, a capability many CDRs lack.
Wrap a source in `VERSION` and choose the scope:

```text
SELECT v/commit_audit/time_committed, c/name/value
FROM EHR e
    CONTAINS VERSION v[ALL_VERSIONS]
        CONTAINS COMPOSITION c
```

`LATEST_VERSION` (the default) reads only current trunk versions;
`ALL_VERSIONS` reads across history (including branch versions, where a version
tree has any) so you can see how a record changed over time. A version
predicate on `commit_audit/time_committed` reads the trunk version current at
that instant. The `VERSION` variable also exposes commit metadata: the audit, the
committed time, and the version uid.

## Terminology in queries

Value filters can be backed by terminology in three ways:

- `TERMINOLOGY('expand', …)` as (or inside) a `matches` operand expands a value
  set so a coded field matches any code in it, rather than listing codes by
  hand;
- `TERMINOLOGY('validate'|'subsumes', …) = true` as a boolean condition
  evaluates a code-membership or subsumption test once per query;
- a terminology URI operand (`matches { terminology://… }`) expands the set the
  URI identifies.

These require a terminology source; if external terminology is not configured,
the in-process openEHR bundle is used. See
[Terminology servers](beyond-core/terminology.md) for wiring an external FHIR
terminology server.

Two rejections to expect here: only `expand` may stand as a `matches` operand
(the other operations have no value-list meaning), and only `validate` and
`subsumes` have boolean semantics, so `TERMINOLOGY('lookup', …) = true` is
refused rather than guessed at. A value set the configured server does not know
is a **400** naming it; a terminology server that fails mid-query is a **500**,
kept distinct from a bad query on purpose.

## Pagination and limits

Combine `LIMIT`/`OFFSET` in the AQL with the `fetch`/`offset` request parameters
to page through large result sets. Three bounds interact, and it is worth knowing
which one you hit:

| Bound | Where it comes from | Effect |
|---|---|---|
| `LIMIT`/`OFFSET`, or `fetch`/`offset` | your query or request | Exactly the window you asked for. |
| [`query.max_result_rows`](installation/config-integrations.md#query) | server config, default `10000` | The largest page one execution serves: the page of a query nothing else bounds, and the maximum a `LIMIT` or `fetch` may ask for. `0` means unbounded. |
| [`query.timeout_ms`](installation/config-integrations.md#query) | server config, default `30000` | Per-query database execution budget. **On by default**; `0` disables it. |

A query that exceeds the time budget returns **408 Request Timeout**; narrow it
(add archetype constraints, an `ehr_id` scope, or a `WHERE` filter) rather than
retrying it unchanged. A `LIMIT` or `fetch` larger than the row ceiling returns
**400 Bad Request** naming the ceiling; the page is never silently shortened,
because a client paging with its own `fetch` as the stride would skip rows
without noticing. Page with `offset` and a `fetch` at or below the ceiling. A
result that stops at the ceiling without either bound set is the default page:
page explicitly to read past it.

> [!TIP]
> The more specific your `FROM`/`CONTAINS` (name the archetype, scope by
> `ehr_id`), the faster the query: those constraints map to indexed columns,
> while broad "everything that contains anything" queries do the most work.
> Repeated identical query text reuses a cached plan, so parameterising a
> recurring query beats rewriting its literals.

## What is supported

FerroEHR implements the core AQL 1.1 envelope and rejects out-of-envelope
constructs with an explicit, typed error rather than silently returning wrong
results. Supported today includes:

- `SELECT` of paths, literals, aliases, `DISTINCT`, and the aggregates `COUNT`
  (including `COUNT(DISTINCT)`), `MIN`, `MAX`, `SUM`, `AVG`. `MIN` and `MAX`
  order their operand by type (a quantity by its openEHR magnitude, a date/time
  chronologically, text lexically) so they work over non-numeric leaves;
- `FROM` over `EHR`, `VERSION` (`LATEST_VERSION` / `ALL_VERSIONS`), and the RM
  structure classes, with archetype and name predicates;
- `CONTAINS` trees with `AND`, `OR`, and `NOT CONTAINS`;
- `WHERE` comparisons on typed leaves (with openEHR magnitude ordering for
  quantities), `EXISTS`, `LIKE`, `MATCHES` value lists, and range predicates;
- `ORDER BY` typed leaves, `LIMIT`/`OFFSET`, `TOP n`, named query parameters,
  and the `ehr_id`, `offset`, and `fetch` request parameters;
- the single-row functions: `LENGTH`, `SUBSTRING`, `POSITION`, the string
  `CONTAINS`, `CONCAT`/`CONCAT_WS`, `ABS`, `MOD`, `CEIL`, `FLOOR`, `ROUND`, and
  `CURRENT_DATE`/`CURRENT_TIME`/`CURRENT_DATE_TIME`/`NOW`/`CURRENT_TIMEZONE`;
- terminology-backed `TERMINOLOGY()` operands and terminology-URI `matches`
  operands (see above).

### What is refused, and why

Every refusal below is a typed error naming the construct, so you never get a
silently incorrect answer:

- **Demographic sources.** `FROM PARTY`/`ROLE`/`ACTOR` and the other demographic
  classes are out of the query engine's scope; the demographic API serves them
  directly.
- **`TOP … BACKWARD`.** `TOP` is deprecated as of AQL 1.1.0 and the direction
  variant is not implemented; the error carries the specification's own rewrite
  (`ORDER BY <path> DESC LIMIT n`). `TOP` and `LIMIT` in one query are also
  refused; pick one.
- **Regex and `OR` node predicates.** `[{/…/}]` is archetype-definition syntax,
  not AQL value matching, and a disjunctive node predicate is outside the
  accepted subset.
- **Branch version addressing.** A version predicate may address trunk versions;
  reading branch content is done by scoping `ALL_VERSIONS`, not by naming a
  branch.
- **`SELECT DISTINCT` ordered by an unselected expression.** De-duplication and
  sorting by something the projection dropped have no defined meaning together;
  sort by a selected column.
- **Analysis failures**, which are precise: an unknown RM
  class or attribute, an unbound `$parameter`, a duplicate `FROM` variable name,
  `LIMIT 0`, a negative `OFFSET`, wrong function arity, `SUM`/`AVG` over a
  non-numeric path, and an `archetype_node_id` criterion that is neither an
  archetype identifier nor a node code. Variable names are case-insensitive, as
  the specification requires.

### The specification generation is an acceptance boundary

Which Reference Model classes and attributes a query may name depends on the
deployment's [`spec_profile`](installation/configuration.md#spec_profile). On the
default `development` profile the query surface is the full RM 1.2.0 model. On
`stable` (RM 1.1.0), a `FROM` class or a path attribute that only a newer
generation defines is refused with **400 Bad Request**, and the message names
both the offending class or attribute and the active profile. A server of that
generation would answer "unknown", so returning rows instead would silently
overclaim the profile the deployment advertises.

Nothing else about querying changes with the profile: paths, predicates, and the
result-set shape are identical.
