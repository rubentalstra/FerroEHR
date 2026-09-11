---
name: sut-reproduction-setup
description: How to curl-reproduce a wire exchange against the composed ferroehr SUT
metadata:
  type: reference
---

Composed SUT base: `http://localhost:8080/ferroehr/rest/openehr/v1`.
Basic-auth dev users (from `docker/ferroehr.dev.toml`, all password `ferroehr`):
`ferroehr` (USER), `ferroehr-admin` (ADMIN+USER — needed for template upload +
admin API), `ferroehr-readonly` (READONLY). scripts/conformance.sh exports
`SUT_USER=ferroehr` / `SUT_PASS=ferroehr` (env-driven via ixit `user_env`/
`password_env`).

Reproduction recipe: `POST /ehr` (201, capture `ehr_id`) → upload OPT as
`ferroehr-admin` via `POST /definition/template/adl1.4` (`Content-Type:
application/xml`, raw OPT XML) → commit FLAT/STRUCTURED via
`POST /ehr/{id}/composition` with `Content-Type: application/openehr.wt.flat+json`
(or `...wt.structured+json`) and header `openehr-template-id: <id>`; read back
with `Accept: application/openehr.wt.flat+json` etc. Capture the version_uid
from the `ETag` response header (weak-quoted `W/"…::system::1"`).
Note: `GET /definition/template/adl1.4` (list) 401'd as plain `ferroehr` in one
test — use the admin user for definition-API reads if that recurs.

Correction 2026-09-11 (measured against the `ferroehr-cnf` compose project):
the definition API refuses BOTH Basic dev users — `POST` and `GET`
`/definition/template/adl1.4` returned `403` as `ferroehr` AND as
`ferroehr-admin`, while `POST /ehr` and `POST /ehr/{id}/composition` worked as
plain `ferroehr`. The run's own principals are minted bearers carrying
`user/template-*.cruds` (`docs/conformance/party/ferroehr/ixit.json`
`instances.*.auth.mode: bearer_mint`), so the Basic upload step of the recipe
above no longer reproduces; commit-path repros that need a template must mint a
bearer or reuse an EHR the run already provisioned. Cause not settled.
