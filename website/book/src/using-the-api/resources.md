# Resource walkthroughs

This chapter walks through the core openEHR resources — **EHR**, **EHR_STATUS**,
**COMPOSITION**, **DIRECTORY**, and **CONTRIBUTION** — with real `curl` examples
you can adapt. For each resource it shows the operations, the headers they need,
and the status codes they return. Paths are relative to the base
`/ferroehr/rest/openehr/v1` (see [Using the API](index.md)); examples use Basic
auth (`-u ferroehr:ferroehr`) and JSON. Content negotiation, the `Prefer` header,
and `ETag`/`If-Match` versioning are cross-cutting and get their own chapter,
[Content negotiation & errors](content-negotiation.md); this chapter uses them
in context.

**Datetime parameters.** Several operations below take a point in time
(`version_at_time`, the CONTRIBUTION `time_range` bounds). Write it in the
_extended_ ISO 8601 form — `YYYY-MM-DDThh:mm:ss.sss[Z|±hh:mm]`, e.g.
`2016-06-23T13:42:16.117+02:00`. The timezone is optional: leave it off
(`2016-06-23T13:42:16`) and the value is read in the **server's** local
timezone, so supply `Z` or an explicit offset whenever the client's timezone
may differ from the server's. The time itself is required — a bare date
(`2016-06-23`), the compact "basic" ISO form (`20160623T134216Z`), and
anything unparseable return **400 Bad Request**.

<!-- toc -->

## EHR

An **EHR** is the top-level container for one subject's health record.

### Create an EHR

```shell
curl -u ferroehr:ferroehr -X POST -i \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr
```

`POST /ehr` — the body is optional; you may supply an `EHR_STATUS` to set the
subject and flags at creation. Returns **201 Created** with the new EHR id in
`ETag` and a `Location` header. With `Prefer: return=representation` the body is
the full `EHR`; otherwise it is empty. Supplying an `EHR_STATUS` whose subject
already has an EHR returns **409 Conflict**.

To create with a specific id, use `PUT /ehr/{ehr_id}` (also 201; 409 if that id
is already used).

### Retrieve an EHR

```shell
curl -u ferroehr:ferroehr \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID
```

`GET /ehr/{ehr_id}` returns **200** with the `EHR`, or **404** if unknown. You
can also look one up by subject: `GET /ehr?subject_id=...&subject_namespace=...`
(both parameters required).

## EHR_STATUS

**EHR_STATUS** holds the record's metadata — the link to the subject, and the
`is_queryable` / `is_modifiable` flags. It is itself versioned.

Setting `is_modifiable` to `false` **deactivates** the EHR: any attempt to
create, update, or delete its content — a composition, the directory, or a
folder — is refused with **409 Conflict**, through *every* write path
including a CONTRIBUTION commit. The EHR_STATUS itself stays writable (so you
can set the flag back to `true` to reactivate), and reads and queries are
unaffected.

### Read the current status

```shell
curl -u ferroehr:ferroehr \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/ehr_status
```

`GET /ehr/{ehr_id}/ehr_status` returns the current `EHR_STATUS`, its version id
in `ETag`. Add `?version_at_time=<ISO 8601>` to read it as of a point in time.
`GET .../ehr_status/{version_uid}` reads a specific version.

### Update the status

Updates require the current version id in an `If-Match` header (optimistic
concurrency):

```shell
curl -u ferroehr:ferroehr -X PUT \
  -H 'Content-Type: application/json' \
  -H 'If-Match: "<current-version-uid>"' \
  -H 'Prefer: return=representation' \
  --data-binary @ehr-status.json \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/ehr_status
```

`PUT /ehr/{ehr_id}/ehr_status` returns **200** (with representation) or **204**
(minimal), plus the new `ETag`. A stale or wrong `If-Match` returns **412
Precondition Failed** with the current version id in `ETag`.

### Status version history

The `versioned_ehr_status` sub-resource exposes the full version history:

- `GET .../versioned_ehr_status` — the `VERSIONED_EHR_STATUS` object,
- `GET .../versioned_ehr_status/revision_history` — the revision history,
- `GET .../versioned_ehr_status/version` (optionally `?version_at_time=`) and
  `.../version/{version_uid}` — a specific version.

## COMPOSITION

A **COMPOSITION** is a committed clinical document, validated against its
template.

### Create a composition

```shell
curl -u ferroehr:ferroehr \
  -H 'Content-Type: application/json' \
  -H 'Prefer: return=representation' \
  --data-binary @composition.json \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/composition
```

`POST /ehr/{ehr_id}/composition` returns **201 Created** with the version id in
`ETag`. Validation failures against the template return **422 Unprocessable
Entity** (with the errors); a malformed request returns **400**; an unknown EHR,
**404**.

### Retrieve a composition

```shell
curl -u ferroehr:ferroehr \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/composition/$UID
```

`GET /ehr/{ehr_id}/composition/{uid_based_id}` accepts either a full version id
(`<uuid>::<system>::<n>`) or a bare object uuid (in which case add
`?version_at_time=` to pick a point in time). It returns **200** with the
composition, **204** if the composition was (logically) deleted at that time, or
**404**.

### Update and delete

```shell
# Update — If-Match is the CURRENT version id; the URL uses the bare object uuid
curl -u ferroehr:ferroehr -X PUT \
  -H 'Content-Type: application/json' \
  -H 'If-Match: "<current-version-uid>"' \
  --data-binary @composition.json \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/composition/$OBJECT_UUID

# Delete — the URL uses the FULL version id
curl -u ferroehr:ferroehr -X DELETE \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/composition/$VERSION_UID
```

`PUT` returns **200**/**204** (per `Prefer`) with the new version id, **412** on
an `If-Match` mismatch, **422** on validation failure. `DELETE` is a *logical*
delete — the history is retained — returning **204**; deleting something already
deleted returns **400**, and a version id that is not the latest returns **409**.

> [!NOTE]
> Watch the id you pass. `PUT` takes the **object uuid** (the versioned object),
> while `DELETE` takes the **full version id** (the version you are superseding).
> `GET` accepts either.

> [!NOTE]
> A version's lifecycle state (set through the `openehr-version:
> lifecycle_state.code_string` header — see
> [Content negotiation & errors](content-negotiation.md); the default on a
> commit is `532|complete|`) must follow the openEHR version-lifecycle state
> machine. An illegal transition is rejected with **422 Unprocessable
> Entity** naming the states. In particular, a version left in the
> `801|abandoned|` state cannot be updated straight to `complete` — you must
> first retrieve it back to `553|incomplete|`, then complete it.

### Composition version history

`GET .../versioned_composition/{versioned_object_uid}` and its
`revision_history`, `version`, and `version/{version_uid}` sub-resources mirror
the EHR_STATUS history endpoints.

## DIRECTORY

The **DIRECTORY** is an optional `FOLDER` tree for organising compositions
within an EHR. The `/directory` endpoints manage the EHR's primary hierarchy
(the openEHR `EHR.directory`, which is always the first member of
`EHR.folders`).

An EHR can also index **additional folder hierarchies** beyond the directory:
commit further root `FOLDER`s through the CONTRIBUTION endpoint (the openEHR
REST API defines no dedicated endpoint for them). The EHR resource then lists
every live hierarchy in its `folders` attribute, in creation order, with
`directory` always equal to the first member; deleting the directory promotes
the next live hierarchy.

```shell
# Create the directory
curl -u ferroehr:ferroehr \
  -H 'Content-Type: application/json' \
  --data-binary @folder.json \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/directory

# Read it (optionally at a time, or a sub-path)
curl -u ferroehr:ferroehr \
  'http://localhost:8080/ferroehr/rest/openehr/v1/ehr/'$EHR_ID'/directory?path=episodes/2024'
```

- `POST /ehr/{ehr_id}/directory` — create the root folder; **201**.
- `PUT /ehr/{ehr_id}/directory` — update it; requires `If-Match`; **200**/**204**.
- `DELETE /ehr/{ehr_id}/directory` — logical delete; requires `If-Match`;
  **204**.
- `GET /ehr/{ehr_id}/directory` — the current folder tree, optionally filtered
  by `?version_at_time=` and `?path=` (slash-separated folder names). **204** if
  deleted at that time.
- `GET /ehr/{ehr_id}/directory/{version_uid}` — a specific version, optionally
  `?path=`.

## CONTRIBUTION

A **CONTRIBUTION** is an atomic change-set: a group of versioned-object changes
(compositions, statuses, folders) committed together with one shared audit. Use
it when several changes must land as a unit.

```shell
curl -u ferroehr:ferroehr \
  -H 'Content-Type: application/json' \
  --data-binary @contribution.json \
  http://localhost:8080/ferroehr/rest/openehr/v1/ehr/$EHR_ID/contribution
```

`POST /ehr/{ehr_id}/contribution` takes a contribution whose `versions` array
each describe a change (the RM object, its `change_type`, and per-version
`commit_audit`) plus a shared `audit`. The audit objects are of type
`UPDATE_AUDIT` (the server fills in `time_committed` and `system_id`). It returns
**201** with the contribution id in `ETag`, or **400**/**404**/**409** on
invalid input, unknown EHR, or a uid conflict.

`GET /ehr/{ehr_id}/contribution/{contribution_uid}` returns **200** with the
contribution, or **404**.

`GET /ehr/{ehr_id}/contribution` (no uid) lists the EHR's contributions,
newest first — a FerroEHR extension (the openEHR REST API defines only the
by-uid read). Paginate with `?offset=` (default 0) and `?fetch=` (default 20,
capped at 100). It returns **200** with a JSON summary, or **404** for an
unknown EHR:

```json
{
  "rows": [
    { "uid": "…", "time_committed": "…", "committer": "…", "change_type": "…" }
  ],
  "total": 123
}
```

> [!NOTE]
> The contribution envelope is always canonical JSON (or XML). The FLAT and
> STRUCTURED formats, when used, apply only to the inner composition `data` of
> each version, not to the envelope.

## Status-code summary

| Code | Meaning across these resources |
|---|---|
| 200 | Retrieved, or updated with `Prefer: return=representation`. |
| 201 | Created (EHR, composition, directory, contribution). |
| 204 | Success with no body (`return=minimal`), or deleted / deleted-at-time. |
| 400 | Malformed request, missing required header/parameter, or already-deleted. |
| 404 | Unknown EHR, object, version, or no version at the requested time. |
| 409 | Conflict — duplicate subject/id, a version that is not the latest, or a content write to a deactivated (`is_modifiable = false`) EHR. |
| 412 | `If-Match` did not match the latest version (current id returned in `ETag`). |
| 422 | Composition is well-formed but fails template/semantic validation, or an illegal version-lifecycle transition. |

The [Content negotiation & errors](content-negotiation.md) chapter covers the
error body shape and the headers referenced above in full.
