---
name: query-etag-presence-is-should
description: No released source makes ETag PRESENCE mandatory on a query 200 — the docs text lists it, the OAS header object is not required, so a `pattern:` matcher over-gates
metadata:
  type: project
---

CATALOGUE over-reach candidate, confirmed first-hand 2026-07-28 against
`bindings/its-rest/I_QUERY_SERVICE.execute_ad_hoc_query.yaml`
(`outcomes.ok.headers: { ETag: 'pattern:W/"[^"]+"' }`).

- `ITS-REST/specifications/docs/query/Request.md` §Common Headers and Query
  Parameters says only "Related response headers: - `ETag` - A unique identifier
  of the resultSet" — a declarative list entry, no RFC 2119 keyword.
- `specifications/responses/200_Query.yaml` declares the `ETag` header via
  `../headers/ETag_RESULT_SET.yaml`; an OpenAPI response header object without
  `required: true` is OPTIONAL by default.
- The only strength anywhere is the overview §ETag and Last-Modified SHOULD.

So the `W/` MUST binds the FORM given the header is sent; PRESENCE is SHOULD.
A `pattern:` matcher asserts presence too, so a server that omits the header on
a query 200 fails a row grounded on a SHOULD (5 rows, AqlBasic/QueryApi, in the
ehrbase record).

**How to apply:** on any header matcher, ask whether the released text makes
PRESENCE mandatory. If not, the matcher must be presence-optional (the
`"present?"`-style optional form the same catalogue already uses for
`Preference-Applied`) or the case non-gating. Same family as
[[amb54-contribution-412-etag-overpromise]] (antecedent-false SHOULD) and
[[contained-uid-is-a-recommendation]] (a SHOULD cannot gate CORE).
