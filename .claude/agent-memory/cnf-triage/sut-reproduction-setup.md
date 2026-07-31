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
