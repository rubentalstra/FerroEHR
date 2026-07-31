# Demographics

Alongside clinical records, a CDR often needs to store the people and
organisations they refer to — patients, clinicians, care teams, institutions.
FerroEHR provides a versioned demographic store for openEHR party types and
the relationships between them, served over a REST surface that mirrors the EHR
APIs.

> [!NOTE]
> The demographic wire API — including party relationships — is defined by
> the openEHR ITS-REST Release-1.1.0 Demographic API (carried at DEVELOPMENT
> lifecycle status within the released specification). It is an
> **Options-profile** capability, not part of the Core or Standard profile.

## What is stored

The store holds the five openEHR party types — `PERSON`, `ORGANISATION`,
`GROUP`, `AGENT`, and `ROLE` — and `PARTY_RELATIONSHIP` between them. Every
party is fully versioned as a `VERSIONED_PARTY`: updates create new versions,
history is retained, and you can read a party as of a point in time or by a
specific version, exactly as for compositions and EHR_STATUS (see
[Using the API](../using-the-api/index.md) for the versioning and `If-Match`
conventions, which apply here too). Writes are wrapped in contributions the same
way clinical writes are.

## Party endpoints

All paths are relative to the API base path (`/ferroehr/rest/openehr/v1`), and
`{kind}` is one of `agent`, `group`, `organisation`, `person`, or `role`.

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/demographic/{kind}` | create a party |
| `GET` | `/demographic/{kind}/{uid_based_id}` | read a party |
| `PUT` | `/demographic/{kind}/{uid_based_id}` | update a party |
| `DELETE` | `/demographic/{kind}/{uid_based_id}` | delete a party |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}` | the versioned container |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}/revision_history` | revision history |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}/version` | version at time (query parameter) |
| `GET` | `/demographic/versioned_party/{versioned_object_uid}/version/{version_uid}` | a specific version |

Party changes can also be committed and read as contributions
(`POST /demographic/contribution`,
`GET /demographic/contribution/{contribution_uid}`), and parties support item
tags (`/demographic/tags`, `/demographic/{kind}/{uid_based_id}/tags`, and
`DELETE …/tags/{key}`).

## Relationships

Party relationships are managed through a parallel set of routes (an FerroEHR
extension), with the same versioned shape as parties:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/demographic/party_relationship` | create a relationship |
| `GET`/`PUT`/`DELETE` | `/demographic/party_relationship/{uid_based_id}` | read / update / delete |
| `GET` | `/demographic/versioned_party_relationship/{versioned_object_uid}` | the versioned container |
| `GET` | `/demographic/versioned_party_relationship/{versioned_object_uid}/revision_history` | revision history |
| `GET` | `/demographic/versioned_party_relationship/{versioned_object_uid}/version[/{version_uid}]` | version at time / by id |

The demographic endpoints are always mounted (not behind a feature switch), and
are subject to the same authentication and authorization as the rest of the API
— see [Security & multi-tenancy](../security.md).
