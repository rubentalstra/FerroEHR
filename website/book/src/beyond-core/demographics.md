# Demographics

Alongside clinical records, a CDR often needs to store the people and
organisations those records refer to — patients, clinicians, care teams,
institutions. FerroEHR provides a versioned demographic store for the openEHR
party types and the relationships between them, over a REST surface that mirrors
the EHR APIs.

The demographic endpoints are **always mounted** — there is no feature switch —
and are subject to the same authentication and authorization as the rest of the
API. See [Security & multi-tenancy](../security.md).

> [!NOTE]
> The party wire API is defined by the openEHR REST specification's
> **Demographic API**, which carries DEVELOPMENT lifecycle status inside the
> released specification, and the conformance schedule places demographics in
> the **Options** profile tier rather than Core or Standard. Party
> *relationships* are the one part with no specification at all: those routes are
> a FerroEHR extension realizing the service model's relationship interface, and
> they are excluded from conformance-profile claims.

<!-- toc -->

## What is stored

The store holds the five openEHR party types — `PERSON`, `ORGANISATION`,
`GROUP`, `AGENT`, and `ROLE` — and `PARTY_RELATIONSHIP` between them.

Every party is a fully versioned object with no owning EHR: updates create new
versions, history is retained, and you can read a party as of a point in time or
by a specific version, exactly as for compositions and `EHR_STATUS`. The
`ETag` / `Location` / `Prefer` / `If-Match` conventions are the same ones the EHR
group uses — see [Using the API](../using-the-api/index.md) — and writes are
wrapped in contributions the same way clinical writes are.

Deletion is **logical**, as it is for clinical content: a delete commits a new
version in the deleted state rather than erasing history. A deleted party then
reads as absent, and deleting one twice is refused.

## Party endpoints

All paths are relative to the API base path (`/ferroehr/rest/openehr/v1`), and
`{kind}` is one of `agent`, `group`, `organisation`, `person`, or `role`.

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/demographic/{kind}` | create a party |
| `GET` | `/demographic/{kind}/{uid_based_id}` | read a party |
| `PUT` | `/demographic/{kind}/{uid_based_id}` | update a party |
| `DELETE` | `/demographic/{kind}/{uid_based_id}` | logically delete a party |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}` | the versioned container |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}/revision_history` | revision history |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}/version` | version at a time (query parameter) |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}/version/{version_uid}` | a specific version |

Party changes can also be committed and read as contributions
(`POST /demographic/contribution`,
`GET /demographic/contribution/{contribution_uid}`), and parties support item
tags (`GET /demographic/tags`, plus
`/demographic/{kind}/{uid_based_id}/tags` and
`DELETE …/tags/{key}` per party).

## Relationships

Party relationships are managed through a parallel set of routes — the FerroEHR
extension the note above describes — with the same versioned shape as parties:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/demographic/party_relationship` | create a relationship |
| `GET`/`PUT`/`DELETE` | `/demographic/party_relationship/{uid_based_id}` | read / update / delete |
| `GET` | `/demographic/versioned_party_relationship/{versioned_object_uid}` | the versioned container |
| `GET` | `/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history` | revision history |
| `GET` | `/demographic/versioned_party_relationship/{versioned_object_uid}/version[/{version_uid}]` | version at a time / by id |

A `PARTY_RELATIONSHIP` is a versioned object in its own right, not a party, so
it has its own versioned-container routes rather than appearing under
`/demographic/versioned_party`.

## What a submitted party must satisfy

A party body is validated against the Reference Model before anything is
committed: identities, contacts, and relationship references have to be
well-formed, a `ROLE`'s capabilities and performer have to be present where the
model requires them, and a party reference has to name a legal type and
namespace. A body that fails these is refused as unprocessable, with the
offending path named.

Two limits are worth knowing because they are deliberate rather than accidental:

- There is **no demographic archetype or template store**, so a party is checked
  against the Reference Model but not against an archetype. The service model's
  "definitions valid" precondition therefore does not apply here.
- `PARTY.reverse_relationships` is a derived attribute the server leaves
  unpopulated: it is the computed inverse of `relationships`, and the server
  re-derives rather than storing a client's copy.

## The specification generation a body is read against

The server runs one openEHR specification generation set at a time, chosen by the
`spec_profile` setting (`development` by default, `stable` for the released
generations — see the
[configuration reference](../installation/configuration.md)). For demographics
this shows up at the door, because the two generations disagree about one
attribute:

- Under **`stable`**, a JSON party body may carry
  `PARTY.reverse_relationships` — the released generation defines it, so
  refusing a valid instance of the generation the server advertises would invent
  a prohibition. The value is accepted and then dropped, because the server
  derives that attribute itself.
- Under **`development`**, the attribute is not part of the model and a body
  carrying it is refused as an undeclared field, like any other unknown key.

XML bodies need no such split: undeclared elements are skipped in either
profile.

This is an *acceptance* boundary, not a conversion: nothing is silently
translated between generations. If you feed a mixed estate, pick the profile that
matches what your clients actually send.
