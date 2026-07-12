# Content negotiation & errors

Three HTTP mechanisms cut across every openEHR resource: choosing the wire
format (JSON or XML), controlling how much a write returns (the `Prefer`
header), and versioned optimistic concurrency (`ETag` and `If-Match`). This
chapter explains all three and the shape of error responses, so the examples in
[Resource walkthroughs](resources.md) make sense in general.

## JSON and XML

EHRbase-rs speaks **canonical JSON** and **canonical XML** for the RM-typed
resources. Choose with the standard HTTP headers:

- **Request body:** set `Content-Type: application/json` or
  `application/xml`.
- **Response:** set `Accept: application/json` or `application/xml`.

JSON is wired end to end for every operation. XML is supported for the
spec-typed RM objects — a single composition, `EHR_STATUS`, `EHR`, `FOLDER`, and
the version family (versioned objects and revision history) — whose canonical
XML shape the openEHR ITS-XML schemas define. Responses that are not a
spec-typed RM value (collections, item tags, and the query and terminology
DTOs) are JSON-only, as is the CONTRIBUTION envelope.

```shell
# Commit a composition as XML, ask for XML back
curl -u ehrbase:ehrbase \
  -H 'Content-Type: application/xml' \
  -H 'Accept: application/xml' \
  --data-binary @composition.xml \
  http://localhost:8080/ehrbase/rest/openehr/v1/ehr/$EHR_ID/composition
```

For compositions and templates, several endpoints additionally accept the Better
**WebTemplate**, **FLAT** (simSDT), and **STRUCTURED** (structSDT) JSON media
types — `application/openehr.wt+json`, `application/openehr.wt.flat+json`, and
`application/openehr.wt.structured+json`. These are covered in
[Templates & validation](../templates-validation.md).

The query API is **JSON only** — it does not accept XML or the WebTemplate
media types.

## The `Prefer` header

Write operations (create/update) accept a `Prefer` header controlling the
response body. Its default is `return=minimal`:

| `Prefer` value | Effect |
|---|---|
| `return=minimal` (default) | Empty body; the identifier is in `ETag`/`Location`. Status **204** on update, **201** on create. |
| `return=representation` | The full created/updated resource in the body, status **200**/**201**. |
| `return=identifier` | Just the resource identifier object. |

Use `return=representation` when you want the server-completed object back
(with its assigned version id and any server-set audit fields); use
`return=minimal` for throughput when you only need the id.

### `Prefer: resolve_refs`

Contribution reads return their `versions` as `OBJECT_REF`s by default. Add
`resolve_refs` to the `Prefer` header (it combines with the `return=…`
token, e.g. `Prefer: return=representation, resolve_refs`) and the response
carries the full `ORIGINAL_VERSION` objects instead — one round trip instead
of one per version.

## `ETag` and `If-Match` — optimistic concurrency

openEHR objects are versioned, and updates use HTTP preconditions to prevent
lost updates:

- Every read and successful write returns an **`ETag`** header carrying the
  object or version identifier (a *weak* ETag, `W/"..."`).
- Updating or deleting a versioned object requires an **`If-Match`** header
  set to the **current** version id, in double quotes:

  ```text
  If-Match: "8849182c-82ad-4088-a07f-48ead4180515::your.system::2"
  ```

- If the object has moved on since you read it, the write fails with **412
  Precondition Failed** and the *current* version id in the response `ETag`.
  Re-read, reconcile, and retry against the new version.

`Location` may appear on responses too, but treat it as informational for reads
(it is marked deprecated on retrieval responses in the contract); the `ETag` is
the authoritative identifier.

> [!NOTE]
> Version ids normally end in a plain trunk number (`…::2`), but openEHR
> version trees can **branch**: when a version that was created on another
> system is modified locally, the server forks a branch and the new version
> id ends in a three-part tree id (`…::2.1.1`). Treat the version id as an
> opaque token — echo it back in `If-Match` exactly as received — and it
> works the same for trunk and branch versions. `ALL_VERSIONS` queries and
> version reads return branch versions alongside trunk ones; the *latest*
> version of an object is always the latest trunk version.

> [!TIP]
> The round-trip is: read the resource → keep its `ETag` value → send it back as
> `If-Match` on the update → get a new `ETag` for the version you just created.
> Never fabricate a version id; always echo the one the server gave you.

## Error responses

Errors use conventional HTTP status codes (see the summary in
[Resource walkthroughs](resources.md)) with one of two JSON body shapes:

- **Validation errors** (a composition that fails its template) use the
  openEHR error shape:

  ```json
  {
    "message": "Composition validation failed",
    "validationErrors": [
      "/content[0]/data/events[0]/data/items[1]/value/magnitude: value out of range",
      "/content[0]/data/events[0]/data/items[2]/value/defining_code: code not in group"
    ]
  }
  ```

  Each entry is `"<path>: <message>"`, so a client can point the user at the
  exact offending node.

- **All other errors** use a simple shape — the status reason plus a message:

  ```json
  { "error": "Not Found", "message": "No EHR with id ..." }
  ```

Match on the HTTP status first; read the body for the human-readable detail and,
for validation, the per-node list.
