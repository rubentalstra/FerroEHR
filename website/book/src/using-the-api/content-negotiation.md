# Content negotiation & errors

A handful of HTTP mechanisms cut across every openEHR resource: choosing the
wire format (JSON or XML), controlling how much a write returns (the `Prefer`
header), versioned optimistic concurrency (`ETag` and `If-Match`), and the
request headers that enrich a commit (audit metadata and item tags). This
chapter explains them all, plus the shape of error responses, so the examples
in [Resource walkthroughs](resources.md) make sense in general.

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

## Simplified formats (FLAT and STRUCTURED)

Beyond the canonical formats, the server implements the openEHR **Simplified
Formats** — template-driven JSON representations that use friendly field
identifiers (`vital_signs/body_temperature:0/any_event:0/temperature|magnitude`)
instead of full RM paths. Select them the same way as JSON/XML, with these
media types:

| Media type | Meaning |
|---|---|
| `application/openehr.wt.flat+json` | FLAT — one flat JSON object of `path: value` pairs |
| `application/openehr.wt.structured+json` | STRUCTURED — the same data as nested JSON |
| `application/openehr.wt+json` | A template rendered as Web Template JSON (template endpoints only) |

Where they work:

- **Compositions** — full round-trip: commit with
  `Content-Type: application/openehr.wt.flat+json` (or `…structured…`) and
  read back with the matching `Accept`.
- **Template examples** — `GET …/definition/template/adl1.4/{id}/example`
  (and the ADL2 form) return the generated example in any of the four
  formats, chosen via `Accept`.
- **Template definitions** — `GET …/definition/template/adl1.4/{id}` with
  `Accept: application/openehr.wt+json` returns the Web Template document.
- **Contributions** — the CONTRIBUTION envelope itself stays canonical JSON;
  a simplified media type applies only to each composition payload inside
  `versions[].data`.

Two rules to know when committing a composition in a simplified format:

- A FLAT/STRUCTURED payload cannot carry its own template id, so the
  **`openehr-template-id` request header is required** — the commit is
  rejected with `422` without it.
- There is **no `?format=` query parameter**: format selection is done
  exclusively through the standard `Accept` and `Content-Type` headers.

Requests naming a media type the endpoint does not support are answered with
**415 Unsupported Media Type** (request body) or **406 Not Acceptable**
(response format), with a body naming the formats that endpoint does
support. EHR, EHR_STATUS, directory, and demographic resources have no
simplified representation (the format is generated from an operational
template, which those resources do not have) — they speak canonical JSON/XML
only.

The query API is **JSON only** — it does not accept XML or the simplified
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

The response echoes the preference the server actually honoured in a
**`Preference-Applied`** header (`return=representation`,
`return=identifier`, or `return=minimal`), so a client can tell what it got
without sniffing the body.

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
  object or version identifier as a *weak* ETag, `W/"..."`.
- Updating or deleting a versioned object requires an **`If-Match`** header
  set to the **current** version id. Both the weak form and a bare quoted
  value are accepted — echoing the `ETag` you received works either way:

  ```text
  If-Match: W/"8849182c-82ad-4088-a07f-48ead4180515::your.system::2"
  If-Match: "8849182c-82ad-4088-a07f-48ead4180515::your.system::2"
  ```

- If the object has moved on since you read it, the write fails with **412
  Precondition Failed** and the *current* version id in the response `ETag`.
  Re-read, reconcile, and retry against the new version.

A **`Location`** header is emitted only when a resource is **created** —
reads and deletes identify the version through `ETag` alone, so do not expect
`Location` on them; the `ETag` is the authoritative identifier.

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

## Commit metadata headers

When you commit through the direct resource endpoints (composition,
EHR_STATUS, directory), the server builds the version's audit for you. Two
request headers let you set parts of it — **`openehr-version`** for the
version's own attributes and **`openehr-audit-details`** for the commit
audit. The value is a comma-separated list of `attribute.subkey="value"`
pairs (quoted values may contain commas; the header may repeat, and repeats
are merged):

```text
# Commit a composition as a draft (lifecycle state "incomplete", code 553)
openehr-version: lifecycle_state.code_string="553"

# Name the committer, describe the change, and stamp the source system
openehr-audit-details: committer.name="John Doe",description.value="Corrected dosage",system_id="pas.example.org"
```

The attributes the server merges:

| Header | Attribute | Sub-keys |
|---|---|---|
| `openehr-version` | `lifecycle_state` | `code_string` |
| `openehr-audit-details` | `change_type` | `code_string` |
| `openehr-audit-details` | `description` | `value` |
| `openehr-audit-details` | `committer` | `name`, `external_ref.id`, `external_ref.namespace`, `external_ref.type` |
| `openehr-audit-details` | `system_id` | (bare value) |

A client-supplied `system_id` is merged into the commit audit — useful when a
gateway commits on behalf of a source system; when absent, the server stamps
its own system id.

> [!NOTE]
> The older dotted spellings — `openEHR-VERSION.lifecycle_state:
> code_string="553"`, `openEHR-AUDIT_DETAILS.committer: name="John Doe"`,
> and so on, with the attribute in the header *name* — are deprecated but
> still accepted. If both forms appear, the lowercase value-form header wins.

## Item tags via headers

Item tags — small `key`/`value` annotations, optionally pointing at a node
inside the data via `target_path` — can ride the same request as a write, so
tagging does not need a second round trip. Two headers carry them:

- **`openehr-item-tag`** — tags targeting the versioned object;
- **`openehr-version-item-tag`** — tags targeting the version being
  committed.

The value is a `;`-separated list of tags, each a comma-separated set of
`key="…"`, `value="…"`, and optional `target_path="…"` pairs:

```text
openehr-version-item-tag: key="diagnosis",value="confirmed",target_path="/content[0]"; key="reviewed",value="true"
```

They are accepted on the EHR-group change-controlled writes (composition
create/update, EHR_STATUS update, directory create/update) and on demographic
party writes, and the stored tags are echoed back in the same headers on the
response. Sending the header with an **empty value** removes all tags.

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

  This shape is used consistently — including for `405 Method Not Allowed`
  and `501 Not Implemented`, which some servers leave bodyless.

Match on the HTTP status first; read the body for the human-readable detail and,
for validation, the per-node list.
