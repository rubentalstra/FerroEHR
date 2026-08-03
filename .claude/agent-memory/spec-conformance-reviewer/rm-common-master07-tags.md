---
name: rm-common-master07-tags
description: RM common ch.7 Tags audit — verified defects (FOLDER tag CHECK 409, post-commit tag failure, UpdateItemTag additionalProperties ignored, demographic "" target_path) and what IS conformant
metadata:
  type: project
---

Walked 2026-08-03 against `RM/docs/common/master07-tags.adoc` +
`RM/docs/UML/classes/org.openehr.rm.common.item_tag.adoc` +
`RM/docs/ehr/master04-ehr_package.adoc` §Tags + ITS-REST overview
`Requests_and_responses.md` §item-tag headers + the released OAS.
Re-verify before citing (code moves).

**Verified live defects**

- `item_tag.ck_item_tag_target_type` (`app/ferroehr/migrations/ehr/0001_baseline.sql:728`)
  omits `'FOLDER'`, but `api/ehr/directory.rs:86,141` route the wrapper headers
  with `target_type = "FOLDER"` → SQLSTATE 23514 → `classify_sqlx`
  (`storage/error.rs:146`) maps every `23*` to **409 Conflict** with the raw
  PG message. Zero tests/cases (`directory_http.rs`, `schedule/directory/`).
- The wrapper headers are applied AFTER the commit (`composition.rs:121,267`,
  `ehr_status.rs:119`, `directory.rs:86,141` all `…await?`), so a bad tag key
  answers 422/409 on a request whose VERSION is already durable.
- `UpdateItemTag`/`ItemTag` are `additionalProperties: false` in the released
  OAS; the handler decodes with `negotiate::json_vec` → `Vec<Value>` and
  `service/ehr/tags.rs:113-153` reads key/value/target_path only. The generated
  `UpdateItemTag` (deny_unknown_fields, `rest/generated/ehr.rs:99`) is unused.
  The served `#[utoipa::path]` prose claims "no released sentence governs the
  extra-member case — our own design" — false; the OAS governs.
- `service/demographic/tags.rs:158` does NOT normalize `target_path: ""` →
  absent, while `service/ehr/tags.rs:144-147` does and AMB-96 says "applied
  identically on the EHR and demographic families".
- A non-string `value`/`target_path` is silently dropped (`Value::as_str`
  → None ⇒ ABSENT) on both seams.
- `emit_item_tag_header` (`overview/params.rs:241`) always renders
  `value=""` even for a valueless tag, and falls back to an EMPTY HeaderValue
  on any non-ASCII byte — an empty value is the spec's "remove all tags" signal.
- `parse_item_tag_header` splits on `;` before quote-awareness and silently
  `continue`s a segment with no `key`.

**Conformant / already settled (do not re-report)**

- All 23 released tag operations are routed; identity IS the (key, target_path)
  pair at DB level (`uq_item_tag_identity … NULLS NOT DISTINCT`) — the old
  key-only collapse is FIXED.
- `target` served as the bare RM `UID_BASED_ID` (owner ruling 2026-07-24, RM
  beats the OAS OBJECT_REF wrapper); container vs VERSION collections disjoint.
- B8 (tagging never re-versions) holds structurally — `replace_tags` touches
  only `item_tag` — but has NO asserted test/case anywhere.
- Register coverage is dense: AMB-91..97, 137, 138, 161, 166. AMB-92/95/96 have
  ZERO CNF cases (covered only by `ferroehr-rest/tests/it/headers.rs`).
- `agent/group/role` tag families (9 ops) are DECLARED `coverage_gap` rows in
  `artifacts/vocab/wire_surface.yaml` — honest gaps, not silent omissions.
- No binding declares the 422 branch AMB-93 fixes, nor the 406/415 branches the
  server actually serves on tag routes.
