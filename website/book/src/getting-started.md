# Getting started

This chapter takes you from nothing to a running server with a template loaded,
a clinical composition stored, and an AQL query returning results — in a few
minutes, using Docker Compose. It is the fastest way to see EHRbase-rs work
end to end and to get a feel for the API before reading the reference chapters.
Everything here uses the built-in development credentials; do not use them
outside local evaluation.

> [!WARNING]
> The steps below enable Basic auth with the throwaway user `ehrbase` /
> `ehrbase` and a permissive CORS policy — this is a development configuration
> only. See [Security & multi-tenancy](security.md) and the
> [configuration reference](installation/configuration.md) before exposing a
> server.

## 1. Start the stack

You need Docker with the Compose plugin. From a checkout of the repository:

```shell
docker compose up --build
```

This builds and starts two services: the server (`ehrbase`) on port **8080**,
and a preconfigured **PostgreSQL 18** database. The server runs its schema
migrations automatically on first boot, so the database is ready as soon as it
reports healthy. The Compose file also defines optional SeaweedFS (S3) and
Keycloak (OIDC) services that the quickstart does not depend on.

The development server configuration is mounted from `docker/ehrbase.dev.toml`,
which ships one Basic-auth user (`ehrbase` / `ehrbase`) and one admin user
(`ehrbase-admin` / `ehrbase`) so the API authenticates out of the box.

## 2. Probe the status endpoint

The status endpoint is public and confirms the server is up:

```shell
curl http://localhost:8080/ehrbase/rest/status
```

All clinical API routes live under the base path
`/ehrbase/rest/openehr/v1`. Interactive OpenAPI documentation is served at
<http://localhost:8080/ehrbase/swagger-ui>, and the full endpoint reference is
published on the documentation site under `/ehrbase-rs/api/` (the **API** tab).

## 3. Create an EHR

An **EHR** is the container for one subject's records. Create one with a `POST`
(no body needed):

```shell
curl -u ehrbase:ehrbase -X POST -i \
  http://localhost:8080/ehrbase/rest/openehr/v1/ehr
```

The `-i` flag shows the response headers. On success you get `201 Created`; the
new EHR's identifier is in the `ETag` header (and the `Location` header points
at the created resource). Copy the UUID; the examples below refer to it as
`EHR_ID`.

By default the response body is empty. Add `-H 'Prefer: return=representation'`
to have the server return the full `EHR` object instead.

## 4. Upload a template

Before you can store a composition, the server needs the **Operational Template
(OPT 1.4)** that the composition conforms to. Templates are XML documents;
upload one with `Content-Type: application/xml`:

```shell
curl -u ehrbase:ehrbase \
  -H 'Content-Type: application/xml' \
  --data-binary @my-template.opt \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4
```

A successful upload returns `201 Created`. If you do not have a template to
hand, the openEHR community publishes example OPTs (for instance the
`Vital Signs` templates used in the openEHR training material), and the
international [Clinical Knowledge Manager](https://ckm.openehr.org/) is the
source for the archetypes they are built from. List what is loaded, and inspect
a template's derived WebTemplate (a JSON description convenient for building
forms), with:

```shell
# List templates
curl -u ehrbase:ehrbase \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4

# Fetch the WebTemplate for one template
curl -u ehrbase:ehrbase \
  -H 'Accept: application/openehr.wt+json' \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4/my_template_id
```

See [Templates & validation](templates-validation.md) for the full template
lifecycle and the WebTemplate/FLAT/STRUCTURED formats.

## 5. Commit a composition

A **composition** is one clinical document, stored inside an EHR and validated
against its template. Post the composition JSON (its `archetype_details`
name the template it belongs to):

```shell
curl -u ehrbase:ehrbase \
  -H 'Content-Type: application/json' \
  -H 'Prefer: return=representation' \
  --data-binary @my-composition.json \
  http://localhost:8080/ehrbase/rest/openehr/v1/ehr/$EHR_ID/composition
```

On success you get `201 Created` and — because of `Prefer: return=representation`
— the stored composition in the body, now carrying a server-assigned version
identifier in its `uid`. If the composition does not conform to its template,
you get `422 Unprocessable Entity` with the validation errors; a malformed
request gets `400 Bad Request`. The composition walkthrough in
[Resource walkthroughs](using-the-api/resources.md) covers update and delete,
which use the `If-Match` header for optimistic concurrency.

## 6. Query with AQL

Now query across the data with the Archetype Query Language. The simplest query
lists the EHR ids the server holds:

```shell
curl -u ehrbase:ehrbase \
  -H 'Content-Type: application/json' \
  -d '{"q":"SELECT e/ehr_id/value FROM EHR e"}' \
  http://localhost:8080/ehrbase/rest/openehr/v1/query/aql
```

The response is a `RESULT_SET`: a `columns` array describing each selected
value and a `rows` array of result tuples. To pull values out of the
compositions you committed, select by their archetype path — for example, every
systolic blood pressure above 140:

```shell
curl -u ehrbase:ehrbase -H 'Content-Type: application/json' -d '{
  "q": "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2] WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 140"
}' http://localhost:8080/ehrbase/rest/openehr/v1/query/aql
```

[Querying with AQL](querying-aql.md) is the full language guide — parameters,
stored queries, version scope, terminology, pagination, and the supported
feature set.

## 7. Explore the API interactively

Open <http://localhost:8080/ehrbase/swagger-ui> to browse and try every
endpoint from your browser, or read the static API reference on the
documentation site (the **API** tab, under `/ehrbase-rs/api/`) for the complete
contract.

## Next steps

- [Installation](installation/index.md) — running it for real (Compose,
  Kubernetes, or from source) and the [configuration reference](installation/configuration.md).
- [Using the API](using-the-api/index.md) — the per-resource reference with
  headers, status codes, and versioning.
- [Concepts](concepts/index.md) — the openEHR model and how EHRbase-rs is
  built, if the terms above were unfamiliar.
