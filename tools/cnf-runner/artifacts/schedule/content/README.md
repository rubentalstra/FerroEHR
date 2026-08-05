# Content decision tables (`schedule/content/`)

The CONT cases are decision tables, not hand-written flows: each carries a
`decision_table` of `columns` + `rows`, and the loader synthesizes one commit
per row against the case's constraint-context template. Only `*.yaml` is
loaded — this file is documentation.

## The `expected` column is a two-token vocabulary

A row's `expected` cell is authored as exactly one of:

| token | meaning |
|---|---|
| `accepted` | the generated instance commits — resolves to the `created` outcome |
| `rejected` | the server refuses it |

There is **no third token**, and none of the wire outcome kinds
(`created` / `bad_request` / `validation_failed`) is ever authored directly in
a table. The tables state the *content verdict*; the wire class is derived.

## `rejected` covers TWO wire outcomes, split by `violates`

The released OAS draws the refusal boundary at convertibility:
`responses/400.yaml` covers a request that "could not be parsed or is invalid
(e.g. malformed request URL syntax, missing required header or parameter, or
syntactically invalid header, parameter or content)", while
`responses/422.yaml` covers content whose "content type and syntax is correct,
could be converted to a resource, but there are semantic validation errors".
The docs-text status table (`specifications/docs/overview/
Requests_and_responses.md` §HTTP status codes) states the two rows without
saying where one ends and the other begins, so the OAS grounds it.

A row already carries the discriminator in its `violates` list, so the split is
**derived, never re-authored** (owner decision: ratify, no new token):

> A `rejected` row resolves to **`bad_request`** when any `violates` entry is
> the `rm_schema: … mandatory` class — an entry beginning `rm_schema:` whose
> text contains `mandatory`. Otherwise it resolves to **`validation_failed`**.

The rationale is convertibility applied literally: a body missing a member the
release's own request-body schema lists as `required:` never becomes the
resource `responses/422.yaml` presupposes, so it cannot reach the 422 branch;
everything else converts first and fails a check afterwards.

Across the current tables that partitions the authored violation vocabulary as:

| `violates` class | resolves to |
|---|---|
| `rm_schema: <attr> is mandatory` | `bad_request` (400) |
| `constraint(...)` — a template/archetype constraint | `validation_failed` (422) |
| `rm_invariant(...)` — an RM class invariant | `validation_failed` (422) |
| `iso8601` — a malformed date/time/duration literal | `validation_failed` (422) |
| `rm_schema:` about a VALUE's lexical form | `validation_failed` (422) — see below |

### The URI boundary (register AMB-209)

The one `rm_schema:` shape that is NOT the mandatory class is
`rm_schema: value is not a valid RFC 3986 URI` (row 1 of
`CONT-DV_URI-validate_open` and of `CONT-DV_EHR_URI-validate_open`). It stays
**422**: the value is a present String in a String-typed mandatory attribute,
so the body converts and what fails is the value's domain. Register
**AMB-209** carries the adjudication, including the competing reading (an
RFC-3986-invalid value read as "syntactically invalid content" → 400) as its
noted alternative. This is why the derivation keys on the word `mandatory`
rather than on the `rm_schema:` prefix alone.

## Extending the split

If a row ever needs a refusal class this derivation cannot express, the
sanctioned path is an **explicit override token in the `expected` cell** —
authoring the wire outcome kind on that row so it wins over the derivation —
not a new `violates` spelling chosen to steer the keyword match, and not a
widening of the `mandatory` test. A `violates` entry is evidence about the
content; it must stay readable as the reason the row is refused. Any such
override lands with the register entry or spec citation that justifies its
class, exactly like a hand-written case's `expect:`.
