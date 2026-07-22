---
name: sut-reproduction-setup
description: How to curl-reproduce a wire exchange against the composed ehrbase-rs SUT
metadata:
  type: reference
---

Composed SUT base: `http://localhost:8080/ehrbase/rest/openehr/v1`.
Basic-auth dev users (from `docker/ehrbase.dev.toml`, all password `ehrbase`):
`ehrbase` (USER), `ehrbase-admin` (ADMIN+USER — needed for template upload +
admin API), `ehrbase-readonly` (READONLY). scripts/conformance.sh exports
`SUT_USER=ehrbase` / `SUT_PASS=ehrbase` (env-driven via ixit `user_env`/
`password_env`).

Reproduction recipe: `POST /ehr` (201, capture `ehr_id`) → upload OPT as
`ehrbase-admin` via `POST /definition/template/adl1.4` (`Content-Type:
application/xml`, raw OPT XML) → commit FLAT/STRUCTURED via
`POST /ehr/{id}/composition` with `Content-Type: application/openehr.wt.flat+json`
(or `...wt.structured+json`) and header `openehr-template-id: <id>`; read back
with `Accept: application/openehr.wt.flat+json` etc. Capture the version_uid
from the `ETag` response header (weak-quoted `W/"…::system::1"`).
Note: `GET /definition/template/adl1.4` (list) 401'd as plain `ehrbase` in one
test — use the admin user for definition-API reads if that recurs.
