# Using the API

FerroEHR exposes the openEHR **REST API** (ITS-REST Release-1.1.0 — the
version the server reports and is conformance-tested against): a
resource-based HTTP interface for creating EHRs, committing and
retrieving versioned clinical documents, managing folders and contributions,
and running queries. This part
is the practical reference for client developers — the resources and their
operations, the headers that drive versioning and content negotiation, and the
error contract. The complete, machine-generated endpoint reference (every path,
parameter, and schema) is published separately as the **API reference** on the
documentation site (under `/ferroehr/api/`); this book explains how to *use*
it.

## Base path

All clinical API routes hang off a configurable base path, which defaults to:

```text
/ferroehr/rest/openehr/v1
```

Every path in these chapters is relative to that base. So "`POST /ehr`" means
`POST http://your-host:8080/ferroehr/rest/openehr/v1/ehr`. The base path is set
with `FERROEHR__SERVER__BASE_PATH` (see the
[configuration reference](../installation/configuration.md)).

The public, unauthenticated status probe lives just outside the base path at
`/ferroehr/rest/status`, and interactive docs at `/ferroehr/rest/swagger-ui` when
enabled.

An `OPTIONS` request to the API base path (also answered at `/`) returns the
server's **conformance manifest**: the product name and version, the openEHR
REST API edition it implements, its conformance profile, and the endpoint
groups actually mounted in this deployment — useful for capability discovery
before you call anything else. The identity fields are configurable
(`FERROEHR__SERVER__IDENTITY__*`, see the
[configuration reference](../installation/configuration.md)); the endpoint
list always reflects reality.

## Authentication

Requests are authenticated unless auth is explicitly disabled. Two mechanisms
ship:

- **HTTP Basic** — a configured user store; send `Authorization: Basic ...`.
  The examples in this book use `-u user:password` with curl.
- **OAuth2 / OIDC bearer tokens** — send `Authorization: Bearer <token>`,
  validated against a configured issuer (Keycloak, Active Directory, any
  standards-compliant provider).

Authorization is coarse role-based access control by default (a `USER` role for
clinical operations, an `ADMIN` role for admin operations), with optional
attribute-based policies. The full picture — mechanisms, roles, multi-tenancy —
is in [Security & multi-tenancy](../security.md).

### Which status a credential problem gets

The distinction matters when you are writing a client, because only one of these
means "fix your credential":

| Situation | Status | What it means |
|---|---|---|
| No `Authorization` header | `401` | with a `WWW-Authenticate` challenge listing the schemes this server implements, and **no** `error=` code — nothing has gone wrong yet ([RFC 6750 §3.1](https://www.rfc-editor.org/rfc/rfc6750#section-3.1)) |
| Credential presented and rejected | `401` | challenge carries `error="invalid_token"` |
| `Authorization` header malformed | `400` | an unparsable header, an unknown scheme, or a bearer token outside the [RFC 6750 §2.1](https://www.rfc-editor.org/rfc/rfc6750#section-2.1) `b64token` grammar. The server never got as far as a credential, so this is a request defect (`error="invalid_request"`) |
| Authenticated, not permitted | `403` | for a bearer caller the challenge carries `error="insufficient_scope"`, naming what is missing |
| The token issuer is unreachable | `503` | with `Retry-After`. No token can be validated, so the server cannot decide — it is **not** a statement about your credential ([RFC 9110 §15.6.4](https://www.rfc-editor.org/rfc/rfc9110#section-15.6.4)). Retry; do not discard the token |

Two Basic-auth details worth knowing: the credential must be **padded** base64
(RFC 7617 §2 defers to [RFC 4648 §3.2](https://www.rfc-editor.org/rfc/rfc4648#section-3.2),
which requires the pad characters), and an unknown username costs the same time
as a known one, so response timing reveals nothing about which accounts exist.

> [!NOTE]
> The quickstart ships a throwaway Basic user (`ferroehr` / `ferroehr`).
> Replace it before any real use.

## The chapters here

- **[Resource walkthroughs](resources.md)** — EHR, EHR_STATUS, COMPOSITION,
  DIRECTORY, and CONTRIBUTION, each with real curl examples, the headers they
  need, and the status codes they return.
- **[Content negotiation & errors](content-negotiation.md)** — choosing JSON or
  XML, the `Prefer` header, `ETag`/`If-Match` optimistic concurrency, and the
  error response shape.

For querying, see [Querying with AQL](../querying-aql.md); for loading templates,
[Templates & validation](../templates-validation.md).
