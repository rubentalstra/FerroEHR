# Storage-layer redesign — inventory report

Read-only survey of the FerroEHR storage layer as it stands on
`feat/identifier-rules-de-ch` @ `5dfa118cd`. Every fact carries `file:line`
or a URL. Anything I could not verify is marked **UNVERIFIED**.

---

# Inventory

## Schemas

| schema | created by | contents |
|---|---|---|
| `ext` | `app/ferroehr/migrations/ext/0001_openehr_functions.sql` | 7 `IMMUTABLE`/`STABLE` helper functions; `ext/0002_tenant_context.sql:27` adds `current_tenant_id()` |
| `ehr` | `app/ferroehr/migrations/ehr/0001_baseline.sql` | the clinical domain: 27 tables + 3 union views + 3 cold alias views |
| `cold` | `app/ferroehr/migrations/ehr/0007_cold_archive_tier.sql:38` | FK-free mirrors of `vo_version`/`node`/`vo_attestation` |
| `demographic` | `app/ferroehr/migrations/demographic/0001_baseline.sql:72` | relation-for-relation mirror of the clinical change-control core, built `LIKE ehr.*` |
| `cold_demographic` | `demographic/0001_baseline.sql:249` | the demographic cold tier |
| `linkage` | `app/ferroehr/migrations/linkage/0001_baseline.sql:66` | exactly one table, `party_ehr` |
| `audit` | `app/ferroehr/migrations/audit/0001_baseline.sql` | the ATNA Audit Record Repository, hash-chained |
| `secondary` | **draft only**, `git stash@{0}^3` | see §Draft secondary domain |

Migrator run order is `MIGRATION_SETS` at `app/ferroehr/src/db/mod.rs:733-739`:
**`ext` → `ehr` → `demographic` → `linkage` → `audit`**, each on a connection
whose `search_path` starts with its own schema so each set keeps its own
`_sqlx_migrations` bookkeeping table. The bootstrap outside the migrations
(`db/mod.rs:742-750`) creates the five schemas and
`CREATE EXTENSION btree_gist WITH SCHEMA ext` — still required, not for the
removed `vo_version` EXCLUDE constraints but for `linkage.party_ehr`'s
temporal `PRIMARY KEY … WITHOUT OVERLAPS` over uuid equality parts.

## `ext` schema

| object | file:line | notes |
|---|---|---|
| `openehr_date_days(text) → numeric` | `ext/0001:64` | `IMMUTABLE STRICT PARALLEL SAFE`; days since 0001-01-01, partial dates floor-completed |
| `openehr_time_seconds(text)` | `ext/0001:81` | seconds since start of day, tz suffix stripped |
| `openehr_tz_offset_seconds(text)` | `ext/0001:106` | |
| `openehr_date_time_seconds(text)` | `ext/0001:122` | |
| `openehr_duration_seconds(text)` | `ext/0001:141` | nominal year = 365.24 d, month = 30.42 d |
| `openehr_magnitude(jsonb)` | `ext/0001:169` | DV_ORDERED ordering; index-legal (IMMUTABLE) |
| `openehr_timestamp(text) → timestamptz` | `ext/0001:281` | **STABLE**, so NOT index-legal; feeds `node.context_start` and the AQL temporal coercion |
| `current_tenant_id() → uuid` | `ext/0002:27` | `COALESCE(NULLIF(current_setting('ferroehr.tenant_id', true),'')::uuid, '000…0')` — **an unset GUC silently resolves to the default tenant** |

Grants: `ext/0001:221-247` (`EXECUTE` to `ferroehr_app`/`ferroehr_reader` plus
`ALTER DEFAULT PRIVILEGES … GRANT EXECUTE ON FUNCTIONS`), `ext/0001:336-344`
for `openehr_timestamp`. `demographic/0001:413` extends `USAGE ON SCHEMA ext`
to the four split roles; `linkage/0001:160` to `ferroehr_linkage`.

## `ehr` schema — tables

### `ehr` (`ehr/0001:67-121`)

| column | type | null | purpose (from COMMENT) |
|---|---|---|---|
| `id` | uuid | no | PK, `pk_ehr` |
| `system_id` | text | no | creating system, immutable (RM ehr master04 §Root EHR Object) — `ehr/0001:124` |
| `time_created` | timestamptz | no | default `now()` |
| `subject_id` | text | yes | denormalized copy of current `EHR_STATUS.subject.external_ref.id.value` — `ehr/0001:125` |
| `subject_namespace` | text | yes | denormalized copy of the namespace — `ehr/0001:126` |
| `is_queryable` | boolean | no | promoted `EHR_STATUS.is_queryable`; backs the AQL full-population gate — `ehr/0001:127` |
| `is_modifiable` | boolean | no | promoted `EHR_STATUS.is_modifiable`; backs the content-write guard — `ehr/0001:128` |
| `tenant_id` | uuid | no | added `ehr/0004_multitenancy.sql:93-96`, DEFAULT `ext.current_tenant_id()`, FK → `tenant` |

Indexes: `idx_ehr_time_created (time_created DESC, id)` `ehr/0001:111`;
`uq_ehr_subject (subject_id, subject_namespace) WHERE subject_id IS NOT NULL`
`ehr/0001:120`.
Trigger: `ehr_subject_pseudonym_guard` BEFORE INSERT OR UPDATE OF
`subject_id, subject_namespace` — `ehr/0009_subject_pseudonym_guard.sql:54`.
RLS: `tenant_isolation`, FORCE — `ehr/0004:101-108`.

### `audit` (`ehr/0001:135-195`)

`id` uuid PK (`uuidv7()`), `time_committed` timestamptz (server-computed —
`ehr/0001:171`), `system_id` text, `change_type` text, `description` jsonb
lz4, `committer` jsonb lz4 NOT NULL, `attestation` jsonb lz4 (the
`ATTESTATION` subtype discriminator — `ehr/0001:174`), `tenant_id` uuid.
Constraints: `ck_audit_system_id_nonempty`, `ck_audit_description_shape`,
`ck_audit_attestation_shape`, `ck_audit_change_type` (9 codes,
`ehr/0001:193`).
Indexes: `idx_audit_time_committed` `ehr/0001:228` (justified by a
"2026-07-29 POC window … p99 2.9 s" claim — see §Claims vs code).
RLS: `tenant_isolation` (in the `ehr/0004:86` scoped list).

### `contribution` (`ehr/0001:214-223`)

`id` uuid PK, `ehr_id` uuid (FK → `ehr` ON DELETE CASCADE), `audit_id` uuid
(FK → `audit`, NO ACTION), `tenant_id`. Indexes `idx_contribution_ehr_id`,
`idx_contribution_audit_id`. `demographic/0002_move_parties.sql:184` adds
`ck_contribution_ehr_scoped CHECK (ehr_id IS NOT NULL)` — so the nullable
`ehr_id` documented at `ehr/0001:203` is dead on the clinical side.

### `template_ref` (`ehr/0001:247`) / `template_store` (`ehr/0001:262`)

`template_ref(template_id text PK)` — the FK target of
`vo_version.template_id`, union of `template_store.template_id` and
template-kind `adl2_artefact.hrid`.
`template_store(id uuid PK, template_id text UNIQUE, concept, root_archetype,
content text, created_at, tenant_id)` + functional unique
`ux_template_store_template_id_ci ON (lower(template_id))` `ehr/0001:287`.

### `vo_version` (`ehr/0001:323-472`, `WITH (fillfactor = 90)`)

| column | type | null | purpose |
|---|---|---|---|
| `vo_id` | uuid | no | versioned-object id |
| `kind` | text | no | 10-value CHECK `ehr/0001:436` |
| `ehr_id` | uuid | yes→no | `ck_vo_version_ehr_scoped` added `demographic/0002:182` |
| `sys_version` | integer | no | opaque per-vo commit ordinal, the `node`/`vo_attestation` join key — `ehr/0001:516` |
| `trunk_version`,`branch_number`,`branch_version` | integer | no | the wire `VERSION_TREE_ID` |
| `sys_period` | tstzrange | no | `[committed, superseded)` |
| `lifecycle_state` | text | no | 5-value CHECK, default `'532'` |
| `creating_system_id` | text | no | |
| `preceding_version_uid` | text | yes | |
| `signature` | text | yes | |
| `signature_client_supplied` | boolean | no | |
| `wrapped_original` | jsonb lz4 | yes | IMPORTED_VERSION discriminator |
| `other_input_version_uids` | jsonb | yes | merge provenance |
| `contribution_id`,`audit_id` | uuid | no | FK |
| `template_id` | text | yes | FK → `template_ref` |
| `body` | **text** lz4 | yes | **the whole canonical JSON body of the version** — `ehr/0001:423` |
| `tenant_id` | uuid | no | `ehr/0004` |
| `stable_compatible` | boolean | yes | `ehr/0008:34` |
| `origins` | jsonb | yes | `ehr/0010:29` |

Indexes: `uq_vo_version_current (vo_id) WHERE upper_inf ∧ branch_number=0`
`:476`; `uq_vo_version_branch_current` `:478`; `uq_vo_version_trunk_position`
`:499`; `idx_vo_version_ehr (ehr_id, kind)` `:501`;
`idx_vo_version_current_ehr` `:507`; `idx_vo_version_current_template` `:509`;
`idx_vo_version_contribution` `:511`; `idx_vo_version_audit` `:512`;
`idx_vo_version_template` `:513`. Unique constraint `uq_vo_version_tree`
`:425`. The two GiST `EXCLUDE` constraints were removed — the removal note is
`ehr/0001:450-467`.

### `ehr_folder` (`ehr/0001:543`)

`(ehr_id, rank)` PK, `vo_id` UNIQUE, `rank >= 1`, ranks append-only. No
`tenant_id`, no RLS (it is absent from the `ehr/0004:85-90` scoped list) — see
§Defects.

### `node` (`ehr/0001:570-634`)

| column | type | null | purpose |
|---|---|---|---|
| `vo_id`,`sys_version` | uuid,int | no | FK → `vo_version` ON DELETE CASCADE, `DEFERRABLE INITIALLY IMMEDIATE` |
| `num`,`num_cap`,`parent_num` | integer | no | nested set; `num_cap >= num`, `num = 0 OR parent_num < num` |
| `citem_num` | integer | yes | nearest archetyped ancestor |
| `ehr_id` | uuid | yes | |
| `rm_type` | text | no | |
| `archetype` | text | yes | case-folded at write |
| `arch_entity`,`arch_concept`,`arch_major` | text,text,int | yes | subsumption triple, lowercased |
| `name` | text | yes | |
| `path` | text COLLATE "C" | no | materialized path; "reassembly only, never an AQL predicate" `:684` |
| `data` | jsonb lz4 | no | the node's canonical fragment, structure children pruned `:685` |
| `context_start` | timestamptz | yes | promoted `COMPOSITION.context.start_time.value` `:680` |
| `tenant_id` | uuid | no | `ehr/0004` |

Indexes: `idx_node_rm_type (rm_type)` `:644`;
`idx_node_arch_subsume (rm_type, arch_entity, arch_major, arch_concept text_pattern_ops) WHERE arch_entity IS NOT NULL` `:657`;
`idx_node_context_start (ehr_id, context_start) WHERE rm_type='COMPOSITION'` `:672`.
Removed indexes (with the removal rationale) at `ehr/0001:660-667`: a
`gin(data jsonb_ops)` and an `openehr_magnitude(data->'value')` expression
index. **There is no index on `(vo_id, sys_version)` alone**, but the PK
`(vo_id, sys_version, num)` covers it as a prefix.

### `vo_attestation` (`ehr/0001:697`)

`id` uuid PK, `(vo_id, sys_version)` FK CASCADE, `contribution_id` FK,
`time_committed`, `at_committal` boolean, `data` jsonb (uncompressed — no
`COMPRESSION lz4`, unlike its siblings). Indexes
`idx_vo_attestation_version`, `idx_vo_attestation_contribution`.
**No `tenant_id`, no RLS** — absent from `ehr/0004:85-90`.

### Remaining `ehr` tables

| table | file:line | key shape | tenant/RLS |
|---|---|---|---|
| `stored_query` | `:734` | PK `(reverse_domain_name, semantic_id, semver)` | yes |
| `item_tag` | `:751` | `id` PK; `uq_item_tag_identity UNIQUE NULLS NOT DISTINCT (ehr_id, target_vo_id, target_version, key, target_path)` `:774`; `ck_item_tag_target_type` 8 values `:784`; FK `ehr_id` CASCADE | yes |
| `archetype_store` | `:812` | `archetype_id` PK | yes |
| `adl2_artefact` | `:825` | `hrid` PK, `kind` CHECK 3 values, `parent_hrid` | yes |
| `ehr_index` | `:851` | PK `(ehr_id, subject_id, subject_namespace)`; FK `ehr_id` CASCADE; `idx_ehr_index_subject (subject_id, subject_namespace)` `:872` | **no tenant_id, no RLS** |
| `vo_archive` | `:890` | `vo_id` PK, `archived_at`, `reason` | **no tenant_id, no RLS** |
| `sp_subject` | `:908` | `subject_id` text PK | yes |
| `sp_binding` | `:918` | `env_id` PK | yes |
| `sp_data_frame` | `:928` | PK `(env_id, frame_id)`, `frame_id` UNIQUE | yes |
| `sp_variable` | `:943` | PK `(subject_id, canonical_name)` | yes |
| `sp_data_set` | `:967` | PK `(subject_id, id)` | yes |
| `sp_sample` | `:991` | `id` uuid PK, FK `(subject_id, canonical_name)` CASCADE | **no tenant_id, no RLS** |
| `event_outbox` | `0002:19` | `seq` bigint IDENTITY PK, `contribution_id` FK CASCADE, `ehr_id`, `envelope` jsonb, `committed_at`, `published_at`; partial indexes `:52`,`:55` | yes |
| `event_subscription` | `0003:21` | `id` uuid PK, `name` UNIQUE, `kind`/`change_type`/`template_id` filters, `enabled` | yes |
| `tenant` | `0004:51` | `id` uuid PK, `name` UNIQUE, `system_id`; the reserved all-zero default row inserted at `0004:74` | n/a |
| `fhir_mapping` | `0005:26` | `id` uuid PK, `name` UNIQUE, FK `template_id` → `template_store` | yes (`0005:77-87`) |
| `fhir_outbound_cursor` | `0006:27` | singleton (`only_row boolean PK CHECK(only_row)`), `last_seq bigint` | **no tenant_id, no RLS** |
| `posture` | `0009:25` | `key` text PK, `value`, `stamped_at`; key `subject_pseudonyms` ∈ {required, open} | **no tenant_id, no RLS** |

### Views

| view | file:line | definition |
|---|---|---|
| `ehr.vo_version_all` | `0007:124`, re-created `0008:58` and `0010:41` | `vo_version UNION ALL cold.vo_version` |
| `ehr.node_all` | `0007:129` | `node UNION ALL cold.node` |
| `ehr.vo_attestation_all` | `0007:134` | `vo_attestation UNION ALL cold.vo_attestation` |
| `ehr.cold_vo_version` / `cold_node` / `cold_vo_attestation` | `demographic/0001:326-331`, also `ehr/0011:39-44` | alias views over `cold.*` so tier SQL can be schema-agnostic |
| `demographic.vo_version_all` / `node_all` / `vo_attestation_all` | `demographic/0001:293-306` | same union over `cold_demographic.*` |
| `demographic.cold_vo_version` / `cold_node` / `cold_vo_attestation` | `demographic/0001:333-338` | alias views |

All views carry `WITH (security_invoker = true)`, so RLS on the base tables
applies to the caller.

## `cold` schema (`ehr/0007`)

`cold.vo_version`, `cold.node`, `cold.vo_attestation` — `CREATE TABLE … LIKE`
with `INCLUDING DEFAULTS CONSTRAINTS COMMENTS STORAGE COMPRESSION` (`:46-71`),
so CHECKs are inherited but **all foreign keys and all unique/partial indexes
are not**. PKs re-added `:73-78`. Only three indexes exist
(`idx_cold_vo_version_ehr`, `idx_cold_vo_version_current_ehr`,
`idx_cold_node_ehr`, `idx_cold_vo_attestation_version` — `:82-88`). RLS on
`cold.vo_version`/`cold.node` only (`:104-111`); `cold.vo_attestation` has
**no RLS**. Later columns are added to `cold.vo_version` by hand: `:0008:45`
(`stable_compatible`), `:0010:34` (`origins`) — and the union view is dropped
and re-created each time so the column lists stay aligned.

## `demographic` schema

Built entirely by `LIKE ehr.*` at `demographic/0001:81-142` (audit,
contribution, vo_version, node, vo_attestation, item_tag, vo_archive), PKs at
`:151-164`, FKs at `:166-184`, one unique at `:189`, six indexes at
`:196-204`. Note the index set is **not** the clinical one — no
`idx_dem_node_rm_type`, no subsumption index, no `context_start` index, no
`uq_dem_vo_version_tree`, no `uq_dem_vo_version_trunk_position`, no
`uq_dem_vo_version_branch_current`. Own tables:
`demographic.event_outbox` (`:222`),
`demographic.identifier_scheme` (`0003:29`, seeded with `nl-bsn` at `0003:49`),
`demographic.national_identifier` (`0003:55`: AES-256-GCM `ciphertext` +
12-byte `nonce` + 32-byte HMAC `lookup_digest`; two unique indexes `0003:83`,
`0003:85`), and the `SECURITY DEFINER`
`demographic.resolve_national_identifier(uuid,text,bytea)` (`0003:101`).
RLS: `vo_version, node, item_tag, event_outbox` + the two cold mirrors
(`0001:351-373`), `national_identifier` (`0004:37`), `contribution, audit`
(`0005:26-39`). **`demographic.vo_attestation` and `demographic.vo_archive`
have no RLS.**

`demographic/0002_move_parties.sql` is a data migration: it refuses to run if
`ehr.vo_version` holds unclassifiable EHR-less rows (`:22-41`), disables RLS
(`:58-69`), moves the six party kinds and their change-control closure
(`:87-152`), re-enables RLS (`:156-169`) and installs the two
scoping CHECKs (`:182-189`).

## `linkage` schema (`linkage/0001`)

One table, `linkage.party_ehr` (`:76-114`): `party_id uuid`, `ehr_id uuid`,
`tenant_id uuid DEFAULT ext.current_tenant_id()`,
`sys_period tstzrange DEFAULT tstzrange(now(), NULL, '[)')`, and a PG18
temporal `PRIMARY KEY (tenant_id, party_id, sys_period WITHOUT OVERLAPS)`.
Index `idx_party_ehr_by_ehr (tenant_id, ehr_id)` `:120`. RLS FORCE
`tenant_isolation` `:134-138`. Grants `SELECT, INSERT, UPDATE` only (never
DELETE) to `ferroehr_linkage` `:154`; every other domain role is revoked
`:165-183`.

## `audit` schema

`audit_event` (`audit/0001:32-77`): `id` uuid PK, `recorded_at`, `stored_at`,
`action` CHECK ∈ C/R/U/D/E, `outcome` CHECK ∈ 0/4/8/12, `event_code`,
`operation`, `principal`, `patient_id`, `resource_class`, `resource_id`,
`client_ip`, `token_id`, `tenant_id`, `fhir jsonb NOT NULL`,
`delivered_syslog_at`, `delivered_fhir_feed_at`. Five indexes `:99-111`.
Later columns: `domain`/`purpose`/`legal_basis`/`result_count`/`request_id`
(`0003:33-45`, with `ck_audit_event_domain` `0003:51`), `organisation`
(`0004:26`), `roles jsonb` (`0005:23`), `origins`/`origin_count` (`0007:27`).

Tamper evidence (`audit/0002`): columns `chain_seq`/`prev_hash`/`row_hash`
(`:46`), tables `audit_chain_state` (`:60`) and `audit_chain_gap` (`:84`),
functions `audit_chain_genesis()` (`:100`), `audit_event_digest(17 args)`
(`:118`), `reap_audit_events(int)` (`:378`), `verify_audit_chain()` (`:448`),
and five triggers: statement-level `audit_event_chain_begin` (`:251`),
row-level `audit_event_chain_link` (`:279`), statement-level
`audit_event_chain_commit` (`:302`), `audit_event_reject_mutation` (`:328`),
`audit_event_reject_deletion` (`:350`), `audit_event_reject_truncate`
(`:367`). **`audit_event` carries `tenant_id` but is deliberately NOT
RLS-scoped** — `audit/0001:80` ("NOT RLS-scoped: the node's security log is an
operator surface").

## Roles

| role | created | attrs |
|---|---|---|
| `ferroehr_migrator`, `ferroehr_app`, `ferroehr_reader` | `ext/0001:49-57`, mirrored `ehr/0001:46-54` | `NOLOGIN` (inheriting) |
| `ferroehr_ehr`, `ferroehr_demographic`, `ferroehr_ehr_reader`, `ferroehr_demographic_reader` | `demographic/0001:53-64` | `NOLOGIN NOINHERIT` |
| `ferroehr_linkage` | `linkage/0001:56` | `NOLOGIN NOINHERIT` |
| `ferroehr_secondary`, `ferroehr_secondary_reader` | **draft** `stash@{0}` `secondary/0001:61-66` | `NOLOGIN NOINHERIT` |

Every role block is wrapped in `EXCEPTION WHEN insufficient_privilege` and
degrades to a `RAISE NOTICE` — so in dev/compose/testcontainers **no role, no
grant and no isolation barrier exists at all**.

Cross-domain revokes: `demographic/0001:418-425` (ehr ↮ demographic),
`linkage/0001:165-183` (linkage ↮ both).

## Draft secondary domain (`git stash@{0}^3`, not landed)

`app/ferroehr/migrations/secondary/0001_baseline.sql`, 237 lines, untracked in
the stash's third parent. Contents:

- roles `ferroehr_secondary` / `ferroehr_secondary_reader` (`:61-66`);
- `CREATE SCHEMA secondary` (`:74`);
- `secondary.permit(permit_id text PK, created_at, key_fingerprint bytea
  CHECK octet_length = 32)` (`:86-99`) — a keyed fingerprint of the per-permit
  pseudonymisation key, never the key;
- `secondary.read_model_cursor(permit_id PK FK, last_seq, rebuild_watermark,
  parked_seq, failures)` (`:114-133`) — a **second** outbox cursor reader
  beside `fhir_outbound_cursor`;
- `secondary.leaf(permit_id, record_pseudonym bytea(32), subject_pseudonym
  bytea(32), sys_version, rm_type, archetype, name, path, value jsonb,
  context_start)` with `PK (permit_id, record_pseudonym, path)` (`:143-176`)
  and two indexes (`:181`, `:182`);
- grants + cross-domain revokes (`:197-237`).

Draft-specific observations: **no `tenant_id` column and no RLS anywhere in
the draft**, unlike every other domain; the `leaf` PK assumes a path is unique
per record and papers over repeats with a `#n` suffix (`:185`); the cursor
carries its own `rebuild_watermark` because "the outbox cannot rebuild a read
model (its rows exist only while eventing was on at commit time, and they are
pruned)" (`:136`) — an explicit acknowledgement of the outbox-prune defect
covered below.

---

# Access paths

All paths are in `app/ferroehr/src` unless noted. **SQL style**: only eight
files build SQL with `sea-query` — `aql/sql/{expr,from,mod,predicate,select,
value}.rs`, `db/iden.rs` (the `Iden` table/column registry) and
`system_log/store.rs`. Every other statement in the crate is a static
`sqlx::query(...)` string literal. The commit path is the one place a
`LazyLock<String>` assembles a static SQL template at first use
(`storage/version_repo/commit.rs:306`, `:396`), under
`sqlx::AssertSqlSafe`.

| table | writers (file) | readers (file) | SQL style |
|---|---|---|---|
| `ehr` | `storage/ehr_repo.rs`, `service/ehr/status.rs`, `service/admin/delete.rs`, `service/admin/dump_load.rs`, `service/message/import.rs` | `db/mod.rs`, `storage/ehr_repo.rs`, `storage/version_repo/{meta,placement}.rs`, `service/query/execute.rs`, `service/ehr_index/index.rs`, `service/message/{export,import,tdd}.rs`, `service/subject_proxy/store.rs`, `service/admin/{archive,delete,dump_load}.rs`, `extensions/{tenancy,fhir/ingest}.rs` | static sqlx + sea-query (`aql/sql/from.rs:354,563,916,967`) |
| `audit` | `storage/version_repo/commit.rs` (the folded CTE), `service/admin/{delete,dump_load}.rs` | `storage/version_repo/{read,meta,contribution,placement}.rs`, `storage/ehr_repo.rs`, `service/admin/statistics.rs` | static sqlx |
| `contribution` | `storage/version_repo/commit.rs`, `service/admin/{delete,dump_load}.rs` | `storage/version_repo/contribution.rs`, `service/admin/{delete,dump_load,statistics}.rs` | static sqlx |
| `vo_version` | `storage/version_repo/{commit,import,placement,tier}.rs`, `service/admin/{archive,delete,dump_load,integrity/rebuild}.rs` | `storage/version_repo/{read,meta,placement,attestation,tier}.rs`, `storage/{ehr_repo,node_repo}.rs`, `service/linkage/cohort/predicate.rs`, `db/mod.rs` | static sqlx; sea-query in `aql/sql/from.rs:345,684,806,833,857,1002,1013` |
| `node` | `storage/node_repo.rs` (`write_nodes`, `write_nodes_batch`, `delete_version_nodes`), `storage/version_repo/{commit,tier}.rs` | `storage/node_repo.rs`, `storage/{ehr_repo}.rs`, `storage/version_repo/{attestation,placement,read}.rs`, `service/linkage/cohort/predicate.rs` | static sqlx; sea-query in `aql/sql/{from,select,value}.rs` |
| `vo_attestation` | `storage/version_repo/{attestation,tier}.rs` | `storage/version_repo/read.rs` (the LATERAL aggregate) | static sqlx |
| `ehr_folder` | `storage/version_repo/commit.rs:476` | `storage/{ehr_repo,version_repo/meta}.rs` | static sqlx |
| `item_tag` | `storage/tag_repo.rs`, `service/admin/dump_load.rs` | same | static sqlx |
| `vo_archive` | `service/admin/archive.rs`, `storage/version_repo/tier.rs`, `service/admin/{delete,dump_load,integrity/rebuild}.rs`, `storage/version_repo/{attestation,placement}.rs` | same set | static sqlx |
| `template_store`/`template_ref` | `templates/store.rs`, `service/definition/{adl14,adl2}.rs`, `service/admin/delete.rs` | same + `extensions/tenancy.rs` | static sqlx |
| `archetype_store` | `service/definition/adl14.rs` | same + `extensions/tenancy.rs` | static sqlx |
| `adl2_artefact` | `service/definition/adl2.rs` | `service/definition/{adl14,adl2,lineage}.rs`, `service/admin/delete.rs`, `extensions/tenancy.rs` | static sqlx |
| `stored_query` | `service/definition/query.rs` | same + `extensions/tenancy.rs` | static sqlx |
| `ehr_index` | `service/ehr_index/index.rs` | `service/ehr_index/{index,conflicts}.rs`, `service/subject_proxy/store.rs` | static sqlx |
| `sp_*` | `service/subject_proxy/service.rs` | `service/subject_proxy/{service,store}.rs` | static sqlx |
| `event_outbox` | `storage/version_repo/commit.rs:513` (`write_outbox`) | `extensions/events/publisher.rs:335,366,450`, `extensions/fhir/outbound.rs:240` | static sqlx |
| `event_subscription` | `extensions/events/subscription.rs` | `extensions/events/publisher.rs` | static sqlx |
| `fhir_mapping` | `extensions/fhir/ingest.rs` | same | static sqlx |
| `fhir_outbound_cursor` | `extensions/fhir/outbound.rs` | same | static sqlx |
| `tenant` | `extensions/tenancy.rs` | same | static sqlx |
| `posture` | `db/mod.rs` (boot stamp) | **no Rust reader — read only by the `subject_pseudonym_guard()` trigger** (`ehr/0009:42`) | static sqlx |
| `linkage.party_ehr` | `service/linkage/store.rs:69` (INSERT), `:91` (close) | `service/linkage/store.rs:51,118`; named as a forbidden identifier in `service/linkage/cohort/predicate.rs:193` | static sqlx |
| `demographic.national_identifier` | `service/…` via `demographic` pool (single write site) | `demographic.resolve_national_identifier` SECURITY DEFINER fn | static sqlx |
| `audit.audit_event` | `system_log/store.rs:65,174` (INSERT), `:216,:229` (delivery stamps) | `system_log/store.rs:249` (ITI-81 retrieval) | **sea-query** (`system_log/store.rs`) |

## Which reads see the cold tier

| path | relation used | sees cold? |
|---|---|---|
| point read / named-version read | `vo_version_all` ⋈ `audit` + LATERAL over `vo_attestation_all` — `storage/version_repo/read.rs:161,190,750` | **yes** |
| stored body bytes | `SELECT body FROM vo_version_all …` — `read.rs:717` | yes |
| node canonical read (`read_version_canonical_all`) | `node_all` — `node_repo.rs:276` | yes |
| node canonical read (`read_version_canonical`, `_tx`) | `node` — `node_repo.rs:272` | no |
| AQL | `Node::Table` / `VoVersion::Table` / `Ehr::Table` — `aql/sql/from.rs` throughout; **never** a `*_all` view | **no** |
| revision history / version meta | `storage/version_repo/meta.rs` | mixed; `vo_version_all` where the module comment says so — UNVERIFIED per-statement |
| admin delete audit capture | `vo_version_all` — `service/admin/delete.rs:224,299` | yes |
| multimedia blob GC | `node_all` — `service/admin/delete.rs:346,367,400,443` | yes |
| archive/restore | `vo_version` (freeze source) / `cold_vo_version` (thaw source) — `service/admin/archive.rs:120,174` | by construction |
| first-version root probe | `vo_version_all` — `node_repo.rs:683` | yes |

The book's claim that "point reads retry cold only on a primary miss" is
false — see §Claims vs code.

---

# Node data shape

**A node row's `data` holds ONLY that node's own attributes, with structure
children pruned out.** The evidence:

- `storage/codec.rs:161` `prune_children` walks the object's attributes, and
  for any attribute whose value is a structure object — or an array whose
  members are all structure objects — `shift_remove`s it from the parent map
  and recurses into `walk` (`codec.rs:178-187`). The parent's `data` is the
  map *after* removal (`codec.rs:155`, `row.data = json`).
- "structure" is decided by `storage::structure::is_structure_type`
  (`storage/structure.rs:52`): the BMM-generated
  `openehr_rm::v1_2::model::is_structure_root` set, plus the five demographic
  party roots (`structure.rs:28`) and the four demographic containers
  (`structure.rs:46`).
- A non-structure child (every `DV_*` value, `ARCHETYPED`, `EVENT_CONTEXT`'s
  scalar parts, `PARTY_PROXY`, a nested `PARTY_RELATIONSHIP` — explicitly
  `structure.rs:68-72`) stays **inline** in its parent's `data`.
- An array that mixes structure and non-structure members is rejected outright
  (`StorageError::MixedArray`, `codec.rs:207`).
- `path` is the reassembly key: `content0.`, `items3.`, `context.` etc.
  (`codec.rs:180,183`), split back apart by `split_step` (`codec.rs:300`).
- `num_cap` is computed in one reverse pass after the walk
  (`codec.rs:47-72`).
- Promoted leaves are read from the **pre-pruning** JSON so a value inside a
  soon-to-be-split child is still visible (`codec.rs:126`,
  `storage/promoted.rs:71`). Exactly one entry exists:
  `COMPOSITION.context.start_time.value → node.context_start`
  (`promoted.rs:75-81`).

**Reassembly** is `codec.rs:231 reassemble`, the lossless inverse: sort rows by
`num`, take row 0's `data` as the root, and `attach` each later row's `data` at
its materialized path (`codec.rs:266`).

**But the point read does not use it.** `vo_version.body` holds the whole
canonical body as `text` (`ehr/0001:423`, deliberately `text` not `jsonb` so
key order survives), materialized at write from the accepted, uid-stamped value
*before* decomposition. `storage/version_repo/read.rs:209` reads that column
and `serde_json::from_str`s it; `read.rs:222` serves the raw text verbatim for
the JSON passthrough. The module header states it plainly (`read.rs:146-150`):
"a point read is one statement and one TOAST detoast rather than a node-subtree
re-aggregation".

So the store is **dual-write**: every version body is persisted twice — once
whole in `vo_version.body` (lz4 text) and once decomposed across `node.data`
fragments (lz4 jsonb). `node` exists only to serve AQL and the integrity
rebuild; `body` serves everything else. `reassemble` is live only on four
paths: `node_repo::read_version_canonical{,_all,_tx}` (integrity rebuild —
`service/admin/integrity/rebuild.rs:365`; dump/load — `dump_load.rs:1815`),
`node_repo::read_subtrees_canonical` (AQL whole-object projection —
`aql/exec.rs:124`; the stable-profile gate — `versioning/profile.rs:209`),
`read_version_rows_all` (`service/admin/integrity/mod.rs:412`), and signing.

---

# The cold tier

`storage/version_repo/tier.rs` is the whole mechanism, 132 lines, three
statement lists:

- **`freeze`** (`tier.rs:47`): `INSERT INTO cold_vo_version SELECT * FROM
  vo_version WHERE vo_id = ANY($1)`, the same for `cold_node` and
  `cold_vo_attestation`, then `DELETE FROM vo_version WHERE vo_id = ANY($1)`
  (which cascades the primary `node`/`vo_attestation` rows away). Re-archiving
  selects nothing, so it is idempotent.
- **`thaw`** (`tier.rs:71`): the reverse, version rows first (the FKs point at
  them), then `DELETE FROM vo_archive WHERE vo_id = ANY($1)`.
- **`purge_ehrs`** (`tier.rs:96`) / **`purge_vos`** (`tier.rs:118`): the cold
  half of a physical delete, because the mirrors are FK-free so no cascade
  reaches them.

Every statement names the tier **unqualified** (`cold_vo_version` etc.) and
relies on the connection's `search_path` selecting the domain's alias view
(`tier.rs:13-18`) — the alias views created at `demographic/0001:326-338` and
`ehr/0011:39-44`.

Drivers: `service/admin/archive.rs:112 archive_ehr_vos` (insert `vo_archive`
markers then `tier::freeze`), `:142 archive_party_vos`, `:169 restore_ehr_vos`
(`tier::thaw`), `:185 restore_party_vos`. A write to an archived object thaws
via `storage/version_repo/placement.rs` (`next_placement`, per `tier.rs:23`).

`*_all` views cover exactly three relations — `vo_version`, `node`,
`vo_attestation`. Everything else an archived object touches
(`contribution`, `audit`, `item_tag`, `ehr_folder`, `ehr`) has **no cold
mirror and no union view**, because those rows are never moved.

---

# Pseudonymisation domains

## Pool / search_path selection

`db/mod.rs:402` `CLINICAL_SEARCH_PATH = "SET search_path TO ehr, ext, public"`,
`:418` `DEMOGRAPHIC_SEARCH_PATH = "… demographic, ext, public"`, `:433`
`LINKAGE_SEARCH_PATH = "… linkage, ext, public"`. Applied per pooled
connection in `open_session` (`db/mod.rs:455-459`), wired through
`pool_options` (`:481`) and the `after_connect` hook (`:500`). Four
constructors: `connect`, the demographic twin, `connect_linkage` (`:568`),
`connect_tenant_scoped_linkage` (`:647`), plus the infallible
`linkage_pool_from` (`:689`) which reuses an existing pool's DSN. Separate
DSNs are optional: `DbConfig::linkage_url` (`:112`),
`linkage_dsn()` (`:221`), `linkage_role_is_separated()` (`:246`).

## `DOMAIN_ROLE_BARRIERS` (`db/mod.rs:976-991`)

| role | forbidden schemas |
|---|---|
| `ferroehr_ehr` | `demographic`, `cold_demographic`, `linkage` |
| `ferroehr_ehr_reader` | same |
| `ferroehr_demographic` | `ehr`, `cold`, `linkage` |
| `ferroehr_demographic_reader` | same |
| `ferroehr_linkage` | `ehr`, `cold`, `demographic`, `cold_demographic` |

`verify_domain_isolation` (`db/mod.rs:1023`) probes role existence first
(`:1030`, because `has_table_privilege` raises for a missing role), then one
catalog query per role over `pg_class` relkinds `r,p,v,m,f,S` plus `pg_proc`
`EXECUTE` (`:1039-1065`), returning `DbError::DomainIsolationBreached`. **A
role that does not exist is skipped** (`:1035`) — which is every role in dev,
compose and the test harness.

## `linkage.party_ehr` — written and read

`service/linkage/store.rs` only: `:51` `SELECT ehr_id … WHERE party_id = $1
AND upper_inf(sys_period)`, `:69` `INSERT INTO party_ehr (party_id, ehr_id)`,
`:91` `UPDATE party_ehr SET sys_period = tstzrange(lower(sys_period), now(),
'[)')` (the close, never a DELETE — matching the grant at
`linkage/0001:154`), `:118` a batched `SELECT DISTINCT ehr_id … WHERE party_id
= ANY($1)`. `service/linkage/cohort/predicate.rs:193` lists `"party_ehr"` as a
forbidden token in a cohort predicate. So the table **is** live, exercised by
the cohort/linkage chapter — contrary to the architecture doc's "No pool
reaches it yet".

## Subject identifiers inside the clinical domain

Three clinical-schema tables hold subject identifiers:

| table | column | guard |
|---|---|---|
| `ehr.ehr` | `subject_id`, `subject_namespace` | **`ehr_subject_pseudonym_guard`** trigger (`ehr/0009:54`) |
| `ehr.ehr_index` | `subject_id` (NOT NULL), `subject_namespace` (NOT NULL), plus `notes`, `location jsonb` | **none** |
| `ehr.sp_subject` | `subject_id` text PK (+ `sp_variable`, `sp_data_set`, `sp_sample` keyed off it) | **none** |

The guard (`ehr/0009:34-52`) reads `ehr.posture` for `key =
'subject_pseudonyms'`; when the value is `'required'` it refuses any
`ehr.subject_id` that is not a lowercase-hex UUID. `ehr_index` and `sp_subject`
are not covered by it, and both are reachable on the same clinical
`search_path` by the same `ferroehr_ehr` role.

**Who mints the pseudonym**: the trigger only *validates* a UUID shape; it
does not generate one. `EHR_STATUS.subject.external_ref.id` is client-supplied
at EHR creation and copied into `ehr.subject_id` by the service
(`service/ehr/status.rs` `sync_ehr_subject`, referenced at `ehr/0001:56`). So
in a pseudonymised deployment the **caller** mints the pseudonym; the server
only refuses a non-UUID. Nothing in the server derives the pseudonym from
`linkage.party_ehr` or from a demographic party. *(I did not find any
server-side minting path; marked UNVERIFIED only in the sense that a config
option to do so may exist elsewhere in `privacy/`.)*

---

# The outbox

`event_outbox` rows are written inside the commit transaction by
`storage/version_repo/commit.rs:503 write_outbox`. The envelope is
`{contribution_id, ehr_id, committed_at, versions[]}` (`commit.rs:505-510`),
where each version entry is `versioning/change.rs:108 envelope_entry`:

```
{ vo_id, kind, sys_version, version_tree_id, change_type, template_id }
```

**No clinical content, no subject id, no archetype data.** The one identifier
that leaves is `ehr_id` (a pseudonymous surrogate under the posture guard) plus
`vo_id`/`contribution_id`. The INSERT is skipped entirely when no consumer is
configured (`extensions/events/config.rs:11`, `service/mod.rs:575`).

## Two independent readers, one prune

| reader | position kept in | marks `published_at`? |
|---|---|---|
| AMQP events publisher — `extensions/events/publisher.rs:335` `SELECT seq, envelope FROM event_outbox …` | `event_outbox.published_at` (`publisher.rs:366` `UPDATE … SET published_at = now() WHERE seq = $1`) | yes |
| FHIR outbound — `extensions/fhir/outbound.rs:240` `SELECT seq, envelope FROM event_outbox WHERE seq > $1 ORDER BY seq LIMIT $2` | `ehr.fhir_outbound_cursor.last_seq` | **no** |
| (draft) secondary read model — `secondary.read_model_cursor.last_seq` | its own table | no |

`prune` (`extensions/events/publisher.rs:445-460`):

```sql
DELETE FROM event_outbox
 WHERE published_at IS NOT NULL AND published_at < now() - $1::interval
```

**Confirmed: the prune consults only `published_at` and nothing else.** It
does not read `fhir_outbound_cursor.last_seq`, and there is no `MIN(cursor)`
floor anywhere in the crate. A deployment running both the AMQP publisher and
the FHIR outbound path can therefore have rows deleted that the FHIR cursor
has not yet reached, if the FHIR reader is behind by more than
`retention_days`. This is issue #3330 and the code bears it out. The draft
`secondary` migration's own comment (`stash secondary/0001:136`) says the same
thing about the outbox in general: "its rows exist only while eventing was on
at commit time, and they are pruned".

---

# Tenancy

The GUC is `ferroehr.tenant_id`, read by `ext.current_tenant_id()`
(`ext/0002:27`), which **coalesces an unset or empty setting to the reserved
all-zero default tenant** rather than failing. Every scoped table takes
`tenant_id uuid NOT NULL DEFAULT ext.current_tenant_id()` plus `ENABLE` +
`FORCE ROW LEVEL SECURITY` plus one policy
`tenant_isolation USING (tenant_id = ext.current_tenant_id()) WITH CHECK (same)`
(`ehr/0004:92-109`).

| schema | RLS-scoped | **not** scoped |
|---|---|---|
| `ehr` | `ehr`, `contribution`, `vo_version`, `node`, `item_tag`, `audit`, `template_store`, `archetype_store`, `adl2_artefact`, `stored_query`, `sp_subject`, `sp_binding`, `sp_data_frame`, `sp_variable`, `sp_data_set`, `event_outbox`, `event_subscription` (`ehr/0004:85-90`), `fhir_mapping` (`ehr/0005:81`) | **`vo_attestation`, `ehr_folder`, `ehr_index`, `vo_archive`, `sp_sample`, `fhir_outbound_cursor`, `posture`, `template_ref`, `tenant`** |
| `cold` | `vo_version`, `node` (`ehr/0007:104-111`) | **`cold.vo_attestation`** |
| `demographic` | `vo_version`, `node`, `item_tag`, `event_outbox` (`0001:355-363`), `national_identifier` (`0004:37`), `contribution`, `audit` (`0005:30-38`) | **`vo_attestation`, `vo_archive`, `identifier_scheme`** |
| `cold_demographic` | `vo_version`, `node` (`0001:364-372`) | **`cold_demographic.vo_attestation`** |
| `linkage` | `party_ehr` (`0001:134-138`) | — |
| `audit` | — | **`audit_event` deliberately** (`audit/0001:80`) |

So under RLS: the cold tier is covered for version + node but **not for
attestations**; `audit` is deliberately global; `linkage` is covered. The
`*_all` union views are `security_invoker`, so they inherit whatever the base
tables have — which means a union view over an RLS'd `vo_version` and an
RLS'd `cold.vo_version` is correctly scoped, while `vo_attestation_all` is
scoped on neither side.

---

# Multimedia

`ferroehr-ext` carries the store (`app/ferroehr-ext/src/multimedia/store.rs`),
the offload engine (`offload.rs`) and its module root. Blobs are
content-addressed by a hex digest; `engine.store().uri_for(hex)` produces the
URI stored inside a `node.data` fragment, and `engine.referenced_keys(&data)`
extracts them back (`service/admin/delete.rs:352,443`).

GC runs only on physical delete (`service/admin/delete.rs:423
gc_unreferenced_blobs`):

1. before the delete, collect candidate keys from `SELECT data FROM node_all
   WHERE ehr_id = ANY($1)` — the **whole `data` column of every node of the
   EHR, pulled into the process** (`delete.rs:346`, `:367`; the party twin
   reads inside the transaction, `:400`);
2. after the commit, for each of the two domain pools run

```sql
SELECT DISTINCT k.uri FROM node_all n
 JOIN unnest($1::text[]) AS k(uri) ON position(k.uri in n.data::text) > 0
```

(`delete.rs:439-442`) — an unindexable substring match forcing a **full scan
of every node row in both domains, with a per-row jsonb→text cast and
detoast**;
3. delete each candidate no surviving row still references; a store failure is
   a `tracing::warn!`, not an error (`delete.rs:453`).

The doc comment itself concedes "A conservative scan-based GC (a `blob_ref`
count table is a scale nicety)" (`delete.rs:414`).

---

# Admin physical delete

`service/admin/delete.rs:205 delete_ehr`, exact statement order:

1. (feature `multimedia`) `collect_ehr_blob_keys` — `SELECT data FROM node_all
   WHERE ehr_id = $1` on the pool, outside the transaction (`:211`);
2. `BEGIN` (`:216`);
3. `SELECT audit_id FROM vo_version_all WHERE ehr_id = $1 UNION SELECT
   audit_id FROM contribution WHERE ehr_id = $1` (`:224-227`) — captured
   before the cascade removes the referencing rows;
4. `tier::purge_ehrs(&mut tx, &[ehr_id])` (`:234`) — four statements,
   `tier.rs:101-108`;
5. `DELETE FROM ehr WHERE id = $1` (`:238`) — cascades `vo_version` (→ `node`,
   → `vo_attestation`), `contribution` (→ `event_outbox`), `item_tag`,
   `ehr_folder`, `ehr_index`;
6. row count 0 ⇒ `EhrIdDoesNotExist` and rollback (`:243`);
7. `DELETE FROM audit WHERE id = ANY($1)` (`:253`);
8. `COMMIT` (`:259`);
9. `gc_unreferenced_blobs` (`:264`).

`delete_ehr_set` (`:277`) is the same in chunks of 128 (`:279`), with an empty
selector meaning "every EHR" (`:286`).

**What it does NOT delete:**

| left behind | why |
|---|---|
| `linkage.party_ehr` rows naming the deleted `ehr_id` | different schema, different role, no FK; nothing in `delete.rs` touches it. The temporal PK means a stale mapping stays `upper_inf` forever. |
| `audit.audit_event` records naming `patient_id` / `resource_id` | deliberate — the ATNA repository is append-only and reaped only by `audit.reap_audit_events(retention_days)` (`audit/0002:378`) |
| `ehr.sp_subject` / `sp_variable` / `sp_data_set` / `sp_sample` | keyed by a text `subject_id`, no FK to `ehr`; the cascade cannot reach them |
| `demographic.*` party rows for the same human | by design (separate domain), but nothing re-points or closes them |
| `ehr.posture`, `ehr.tenant`, `template_*`, `stored_query`, `archetype_store`, `adl2_artefact` | not EHR-scoped |
| blobs shared with a surviving node | intentional (content-addressed dedup) |

`event_outbox` **is** removed — via `fk_event_outbox_contribution … ON DELETE
CASCADE` (`ehr/0002:44`) once `contribution` cascades from `ehr`.

`physical_delete_party` (`delete.rs:488`) is the demographic twin: collect blob
keys in-transaction (`:391`), `tier::purge_vos` (`:561`), `DELETE FROM
vo_version WHERE vo_id = ANY($1)` (`:562`), then orphan-guarded deletes of
`contribution` (`:570`) and `audit` (`:580`), then `DELETE FROM vo_archive`
(`:589`). It likewise leaves `linkage.party_ehr` rows naming the deleted
party.

---

# Claims vs code

A sibling report already established three: the AQL emitter emits no
`JSON_TABLE` and no GIN operators; the close-out UPDATE is never HOT; the
"GiST exclusion serialises" citation has no basis in the PG docs. Those are
not repeated. Everything below is additional.

## `docs/architecture.md` §Storage / §AQL engine / §Workspace

| # | claim | verdict | evidence |
|---|---|---|---|
| A1 | "Promoted predicate columns (`rm_type`, `archetype`, `name`, `path COLLATE "C"`, `ehr_id`)" | **CONTRADICTED** for `path` | the migration itself says `path` is "Reassembly only — never an AQL predicate" (`ehr/0001:684`, `db/iden.rs:197-200`); `grep '"path"' app/ferroehr/src/aql/` returns nothing. `rm_type`/`archetype`/`name`/`ehr_id` are genuine predicates (`aql/sql/expr.rs:129,313`, `predicate.rs:491`, `from.rs:896`). |
| A2 | "GIN `jsonb_ops` `$.**` equality anchors as document pre-filters" | **CONTRADICTED** | the GIN index was removed in the same baseline that documents the design: `ehr/0001:660-666`. No `gin` appears in any migration. |
| A3 | "`JSON_TABLE` for array unnesting" | **CONTRADICTED** | `grep -rn 'JSON_TABLE\|json_table' app/ferroehr/src` → 0 hits. |
| A4 | "`jsonb_path_query_first` + jsonpath item methods … for typed leaf extraction/comparison/ordering" | **half CONTRADICTED** | `jsonb_path_query_first` is real (`aql/sql/expr.rs:56`). No jsonpath **item method** (`.datetime()`, `.integer()`, …) is ever emitted: temporal comparison routes through the plpgsql `ext.openehr_timestamp` (`aql/sql/value.rs:535,568`), magnitude through `ext.openehr_magnitude` (`value.rs:526`). The only mention of item methods is a stale doc comment at `aql/ir.rs:435`. |
| A5 | "`ext` — our own `IMMUTABLE` helper functions … usable in btree **expression indexes** for measured hot paths" | **UNSUPPORTED** | no expression index on `node` exists; the one that did was removed (`ehr/0001:663`). The only functional index in the tree is `ux_template_store_template_id_ci ON template_store (lower(template_id))` (`ehr/0001:259`). Moreover `ext.openehr_timestamp` is `STABLE` (`ext/0001:282`) and therefore **not index-legal at all**, as its own COMMENT concedes (`ext/0001:329`). |
| A6 | "`linkage` … No pool reaches it yet." | **CONTRADICTED** | `db/mod.rs:568 connect_linkage`, `:647 connect_tenant_scoped_linkage`, `:689 linkage_pool_from`, and live SQL in `service/linkage/store.rs:51,69,91,118`. |
| A7 | "point reads retry cold only on a primary miss" (also in the book) | **CONTRADICTED** | there is no retry anywhere. Every full version read goes through `vo_version_all`/`vo_attestation_all` in one statement (`storage/version_repo/read.rs:161,190`), and the module comment says so explicitly (`read.rs:16-18`: "a miss never pays a cold-tier retry transaction"). The doc describes an earlier design. |
| A8 | "`node` … one row per RM structure node with a nested-set index (`num`, `num_cap`, `parent_num`, `citem_num`): AQL CONTAINS is an integer interval join" | **half CONTRADICTED** | `num`/`num_cap` are genuinely the join (`aql/sql/from.rs:107,126,675,797`). `parent_num` is read only by the reassembly SELECT (`node_repo.rs:272`). **`citem_num` is written on every row and never read by anything** — see §Defects D1. |
| A9 | "every write emits contribution + audit in the same transaction" | **SUPPORTED** | one folded CTE chain, `storage/version_repo/commit.rs:306-336` and `:396-420`. |
| A10 | "`vo_version` … `uuidv7()` keys" | **half SUPPORTED** | `vo_id`/`ehr_id` are minted in Rust as `Uuid::now_v7()` + a licence stamp (`licence/stamp.rs:111-113`, `ids.rs:59,68`), not by the PG `uuidv7()` function. The DB-side `uuidv7()` is used only for `audit.id`, `contribution.id` (fallback, `commit.rs:173`), `vo_attestation.id`, `item_tag.id`. |
| A11 | "The non-overlap invariant is held by partial unique btrees, not by a temporal key" | **SUPPORTED** | `ehr/0001:476,478`; but see §Defects D5 (the demographic twin has neither). |
| A12 | "Nothing selects a domain but the pool's `search_path`" | **SUPPORTED** | `db/mod.rs:402,418,433,455-459`. |
| A13 | "a CHECK on each side refuses the other's rows" | **SUPPORTED** | `demographic/0002:182-189`. |
| A14 | "Four `NOINHERIT` runtime roles … and the server refuses to boot when either can read across" | **SUPPORTED but vacuous in practice** | `db/mod.rs:1023`; the check `continue`s past any absent role (`:1035`), and every role block degrades silently without `CREATEROLE` (`demographic/0001:65`). In dev/compose/CI nothing is enforced. |
| A15 | "AQL stays primary-only — archived content leaves the queryable store" | **SUPPORTED** | `aql/sql/from.rs` uses `Node::Table`/`VoVersion::Table`/`Ehr::Table` throughout; no `*_all` view appears in `aql/`. |

## `website/book/src/concepts/storage.md`

| # | claim | verdict | evidence |
|---|---|---|---|
| B1 | "`JSON_TABLE` serves array unnesting" (`storage.md:304`) | **CONTRADICTED** | as A3. |
| B2 | "point reads retry cold only on a primary miss" (`storage.md:328`) | **CONTRADICTED** | as A7. |
| B3 | "jsonpath item methods" (`storage.md:302`) | **CONTRADICTED** | as A4. |
| B4 | "Decomposed fragments average a few hundred bytes, stay under TOAST, and each read touches only the rows it needs" (`storage.md:347-349`) | **CONTRADICTED as a description of the read path** | the point read does not read fragments at all — it reads `vo_version.body`, the whole document, in one detoast (`read.rs:146-150,209`). That is precisely the "big single-document design [that] pays whole-document decompression" the same paragraph argues against. The fragment path is used only by AQL and the rebuild. The size figure itself is not backed by a committed artifact (see C3). |
| B5 | "PostgreSQL 18's temporal machinery (`tstzrange`, partial unique indexes, `uuidv7()`, **`RETURNING OLD/NEW`**) makes the single temporal version table cheaper" (`storage.md:352-355`) | **CONTRADICTED** for `RETURNING OLD/NEW` | `grep -rni 'RETURNING OLD\|RETURNING NEW' app/ferroehr/src` → 0 hits. |
| B6 | "The body bytes … stored as `text` (not `jsonb`, which would re-order keys)" (`storage.md:288`) | **SUPPORTED** | `ehr/0001:417-422`, `read.rs:209,222`. |
| B7 | "AQL never touches the body" (`storage.md:300`) | **CONTRADICTED in one place** | `node_repo.rs:636 SELECT v.vo_id, v.sys_version, v.body …` is the AQL whole-object projection's batch body loader, and `node_repo.rs:681 SELECT (body)::jsonb ->> 'archetype_node_id' …` casts the body to jsonb for the first-version root probe called from `service/ehr/validation.rs:442`. |
| B8 | "class and archetype predicates hit the promoted columns and their indexes" | **SUPPORTED** | `aql/sql/expr.rs:129,278-282,313`. |
| B9 | "a write to an archived object thaws it back to the primary tier first" | **SUPPORTED** | `storage/version_repo/tier.rs:23`, `placement.rs` `next_placement`. |

## Migration comments

| # | claim | verdict | evidence |
|---|---|---|---|
| C1 | `ehr/0001:196-199`: "at measured-corpus scale (~10^6 rows) the 2026-07-29 POC window put the unindexed form's p99 at 2.9 s" | **UNSUPPORTED in tree** | no committed artifact carries that number. `grep -rn '2.9 s' docs/conformance/` → nothing; `docs/conformance/ferroehr/results.json` and `stress.json` carry different series. The claim is only re-checkable from a closed issue, which the citation rule treats as durable but which no gate re-verifies. |
| C2 | `ehr/0001:636-643`: "83% of node rows carry at-code archetype text (measured 2026-08-25, #2698)" | **UNSUPPORTED in tree** | same — no committed artifact. |
| C3 | `ehr/0001:22`: "fragments average ~360 B (spike), well under TOAST" | **UNSUPPORTED in tree**, and the conclusion drawn from it is wrong | no artifact carries the figure; and if fragments really are ~360 B they never reach the TOAST code at all, which makes `COMPRESSION lz4` on `node.data` **inert** for the typical row (established in the sibling PG-18 research report, `docs/plans/storage-redesign/research-postgresql-18.md:19,163`). |
| C4 | `ehr/0001:650-656`: "the measured cross-EHR profile showed the old (entity, concept, major) order degrading to a BitmapAnd" | **UNSUPPORTED in tree** | no plan artifact is committed. |
| C5 | `ehr/0001:614-615`: `context_start` "Serves the AQL dashboard ORDER BY … instead of a per-candidate-row jsonb extraction (a measured hot path)" | **SUPPORTED as wiring, UNSUPPORTED as measurement** | the read side is real (`aql/sql/value.rs:318 promoted_leaf_expr`, `storage/promoted.rs:75`); the measurement is not in any artifact. |
| C6 | `ehr/0001:321`: "fillfactor 90: one close-out UPDATE per supersession" | **CONTRADICTED** (sibling report: never HOT) | additionally, `fillfactor` is set on `vo_version` only; the `LIKE … INCLUDING STORAGE` clauses that build `cold.vo_version` (`ehr/0007:46-53`) and `demographic.vo_version` (`demographic/0001:99-106`) do **not** copy reloptions, so neither mirror has it. |
| C7 | `ehr/0001:412-419`: "a point read serves these bytes verbatim, one detoast, no re-aggregation (TOAST keeps it out of the main row, so meta-only scans stay slim)" | **SUPPORTED** | `read.rs:209,222,717`. |
| C8 | `audit/0001:80`: `audit_event` is "append-only except per-sink delivery stamps and retention reaping" | **SUPPORTED** | triggers at `audit/0002:328,350,367` and the `ferroehr.audit_reaping` GUC gate at `audit/0002:340`. |

## `docs/postgres-features.md`

| # | claim | verdict | evidence |
|---|---|---|---|
| P1 | "`JSON_TABLE()` … a core tool for the AQL→SQL generator" | **CONTRADICTED** | 0 hits. |
| P2 | "`MERGE … RETURNING` + `merge_action()` — Upsert composition/version/status rows" | **CONTRADICTED** | `grep -rn 'MERGE INTO\|merge_action' app/ferroehr/src` → 0 hits. |
| P3 | "`RETURNING OLD/NEW` — One-statement audit capture … for the `audit`/`contribution` rows on every version write" | **CONTRADICTED** | 0 hits; the commit path uses a CTE chain with plain `RETURNING id` instead (`commit.rs:311,315,318`). |
| P4 | "Virtual generated columns — … candidate indexes/filters for AQL hot paths" | **CONTRADICTED** | the only `GENERATED ALWAYS AS` in the tree is `IDENTITY` on the two `event_outbox.seq` columns (`ehr/0002:24`, `demographic/0001:223`). |
| P5 | "Temporal `PRIMARY KEY`/`UNIQUE`/`FOREIGN KEY` `WITHOUT OVERLAPS` … Used by `linkage.party_ehr`" | **SUPPORTED** | `linkage/0001:113`. |
| P6 | "`OR` → `= ANY(array)` transformation" | **SUPPORTED, but by the emitter not the planner** | `aql/sql/from.rs:896` emits `PgFunc::any` explicitly. |
| P7 | "B-tree skip scan", "Self-join elimination", "Asynchronous I/O (AIO) (`io_method`, `io_combine_limit`)", "Partition planner improvements" | **UNVERIFIED / no evidence in tree** | no `io_method`, `io_combine_limit`, `enable_self_join_elimination` or partitioning appears anywhere in the repository (checked across `*.sql`, `*.yml`, `*.yaml`, `*.conf`, `*.rs`). These are aspirations, not usage. |
| P8 | "`uuidv7()` (native) — Timestamp-ordered UUIDs for `OBJECT_VERSION_ID`/row keys" | **half SUPPORTED** | as A10. |
| P9 | "`jsonb` null → SQL scalar `NULL` cast — Simpler optional-field extraction" | **UNVERIFIED** | no site in the emitter relies on it that I could identify. |

---

# Defects

Ordered by how much they matter for a redesign.

**D1 — `node.citem_num` is written and never read.** Computed in
`storage/codec.rs:112-120`, carried through `storage/row.rs:36`, bound into
the insert arrays at `storage/node_repo.rs:223,236,250`, and present in the
column list at `node_repo.rs:67,109`. No reader exists: the reassembly SELECT
takes only `num, num_cap, parent_num, path, data` (`node_repo.rs:272`), and
`grep '"citem_num"\|CitemNum' app/ferroehr/src/aql/` returns nothing. It is 4
bytes plus write amplification per node row for nothing. (EHRbase, by
contrast, indexes on it — `comp_data_path_skip_idx (vo_id, citem_num, num)`.)

**D2 — the body is stored twice.** Every version persists its canonical bytes
whole in `vo_version.body` (`ehr/0001:423`) *and* fully decomposed across
`node.data` (`ehr/0001:611`). Nothing reconciles them except
`service/admin/integrity/rebuild.rs`, whose existence is the admission that
they can diverge. Storage cost is roughly 2× the content, and every commit
writes both inside the held transaction.

**D3 — every AQL leaf comparison calls a plpgsql function with an `EXCEPTION`
block.** `ext.openehr_magnitude` (`ext/0001:169-197`),
`ext.openehr_timestamp` (`ext/0001:281-326`) and the four parsers they call
each end in `EXCEPTION WHEN others THEN RETURN NULL`. In PL/pgSQL an
`EXCEPTION` clause opens a subtransaction on **every** call, whether or not an
exception is raised. These functions are emitted into `WHERE` and `ORDER BY`
(`aql/sql/value.rs:526,535,568`), i.e. once per candidate row per predicate.
`openehr_timestamp` is additionally `STABLE`, so it is neither inlinable nor
usable in an index.

**D4 — the multimedia blob GC is a full double-domain table scan with a
substring match.** `service/admin/delete.rs:439-442`:
`JOIN unnest($1::text[]) AS k(uri) ON position(k.uri in n.data::text) > 0`
over `node_all`, run once per domain, per EHR delete. Unindexable, casts every
`data` value to text, detoasts everything. The candidate collection before it
(`delete.rs:346`) pulls **every node's whole `data` value of the EHR into the
application process**.

**D5 — the demographic mirror silently lacks the clinical invariants.**
`CREATE TABLE … LIKE` copies CHECKs but not unique indexes, so
`demographic.vo_version` has neither `uq_dem_vo_version_current` equivalent as
a **unique** (it is created as a plain `CREATE INDEX` at
`demographic/0001:196`, not `CREATE UNIQUE INDEX`), nor `uq_vo_version_tree`,
nor `uq_vo_version_trunk_position`, nor `uq_vo_version_branch_current`. The
whole "non-overlap by construction" argument (`ehr/0001:450-467`) rests on
those partial **unique** btrees, and on the demographic side they do not
exist. The same `LIKE` gap loses the clinical `node` indexes
(`idx_node_rm_type`, `idx_node_arch_subsume`, `idx_node_context_start`), so a
demographic AQL anchor has no index at all.

**D6 — RLS coverage has holes on tables that carry content.**
`ehr.vo_attestation`, `cold.vo_attestation`, `demographic.vo_attestation` and
`cold_demographic.vo_attestation` hold canonical RM `ATTESTATION` bodies
(`data jsonb NOT NULL`, `ehr/0001:713`) and are in **no** tenant policy.
`ehr.ehr_folder`, `ehr.ehr_index` (which holds subject identifiers),
`ehr.vo_archive`, `ehr.sp_sample` (which holds `sample jsonb`) are likewise
unscoped. Because `vo_attestation_all` is `security_invoker`, the union view
inherits no policy either.

**D7 — the outbox prune ignores every cursor reader.** Established above:
`extensions/events/publisher.rs:447-452` deletes on `published_at` alone,
while `extensions/fhir/outbound.rs` tracks its position in
`ehr.fhir_outbound_cursor` and never writes `published_at`. Issue #3330 is
confirmed by the code. The draft `secondary` domain would add a third reader
with the same exposure (`stash secondary/0001:114-136`).

**D8 — `ext.current_tenant_id()` fails open.** `ext/0002:29-32` coalesces an
unset or empty `ferroehr.tenant_id` to the reserved all-zero tenant instead of
raising. A connection that fails to set the GUC therefore reads and writes the
default tenant's data rather than failing, and the `FORCE ROW LEVEL SECURITY`
policies all evaluate against that fallback.

**D9 — subject identifiers sit in the clinical schema outside the pseudonym
guard.** `ehr.ehr_index.subject_id`/`subject_namespace` are `NOT NULL`
(`ehr/0001:853-854`) with a plain index on them (`:872`), and
`ehr.sp_subject.subject_id` is a text PK (`:909`). The
`ehr_subject_pseudonym_guard` trigger is attached only to `ehr.ehr`
(`ehr/0009:54-56`). Under `privacy.subject_namespaces = required` the clinical
domain is therefore still free to hold a BSN in `ehr_index` and in the whole
`sp_*` family, reachable by the same `ferroehr_ehr` role on the same
`search_path`. `ehr_index` additionally carries free-text `notes` and a
`location jsonb`.

**D10 — a physical EHR delete leaves the linkage mapping in force.**
`service/admin/delete.rs:205-265` never touches `linkage.party_ehr`, so after
an erasure the map still asserts, with `upper_inf(sys_period)`, that party X
is the subject of a now-nonexistent EHR. The `sp_*` family is likewise
orphaned (no FK path from `ehr`).

**D11 — the cold tier is a schema fork maintained by hand.** `cold.vo_version`
was built by `LIKE` (`ehr/0007:46`) and then had `stable_compatible`
(`ehr/0008:45`) and `origins` (`ehr/0010:34`) bolted on in lockstep with the
primary, each time dropping and re-creating `vo_version_all` so the
`SELECT *` column lists still line up. Any future column that misses a
migration silently breaks the union view's `UNION ALL`. There is no guard.
`cold_demographic` is safe only by accident of ordering:
`db/mod.rs:733-739` runs `ext → ehr → demographic → linkage → audit`, so
`demographic/0001:99` `LIKE`s an `ehr.vo_version` that already has
`stable_compatible` and `origins`, and `demographic/0001:253` `LIKE`s that.
A future column added to `ehr.vo_version` by a new migration would NOT reach
the demographic pair, because their baseline has already run. Relatedly,
`ehr/0011_cold_alias_views.sql:34` skips itself entirely when
`demographic.vo_version` does not exist — which on a fresh install is always,
since `ehr` runs first — so that whole migration is dead on new deployments
and fires only on an upgrade.

**D12 — `db/iden.rs` is 525 lines of `Iden` definitions that almost nothing
uses.** Only `Ehr`, `Node`, `VoVersion` and `Audit` are imported
(`aql/sql/from.rs:27`, `select.rs:17`, `value.rs:22`), and even those are used
only as `::Table` — every column is spelled as a raw string in the emitter
(`aql/sql/mod.rs:442-458`, `expr.rs:129,313`, `predicate.rs:491`). The
column-name typo protection the file exists to give is not in force.

**D13 — `contribution.ehr_id` is documented as nullable but is not.**
`ehr/0001:203` and `:183-185` describe the NULL case as "the demographics
repository"; `demographic/0002:184` then adds
`ck_contribution_ehr_scoped CHECK (ehr_id IS NOT NULL)`. The comment and the
constraint now contradict each other in the same schema. The same applies to
`vo_version.ehr_id` (`ehr/0001:265-267` vs `demographic/0002:182`) and to
`node.ehr_id` ("nullable (demographic content has none)", `ehr/0001:559`).

**D14 — the draft `secondary` domain has no tenancy at all.** No `tenant_id`
column, no `ENABLE`/`FORCE ROW LEVEL SECURITY`, no policy anywhere in
`stash@{0}` `secondary/0001_baseline.sql`, while every other domain including
`linkage` carries them. In a multi-tenant deployment the derived read model
would be the one place tenants are not separated.

**D15 — `vo_attestation.data` is the only canonical-JSON column without
`COMPRESSION lz4`.** `ehr/0001:713` vs `audit.description`/`committer`/
`attestation` (`:153,157,172`), `vo_version.wrapped_original`/`body`
(`:400,423`) and `node.data` (`:611`). No stated reason.

**D16 — `ehr_folder` has no `tenant_id`, so a cross-tenant folder membership
is representable.** It is absent from the `ehr/0004:85-90` scoped list while
`ehr` and `vo_version` are both in it; the row survives only because its
`ehr_id` FK cascades.

---

# Prior art

## EHRbase 2.x storage

Source: `https://github.com/ehrbase/ehrbase`, branch `master`. **The migration
path is `jooq-pg/src/main/resources/db/migration/{ehr,ext}/`**, not
`service/src/main/resources/...` (that path 404s; the module was renamed).
Flyway-versioned `V1` … `V28`, Apache-2.0.

### The model, and how it got here

| step | migration | what it did |
|---|---|---|
| baseline | `ehr/V1__ehr.sql` | `system`, `tenant` (`int2` PK from a sequence), `ehr(id, sys_tenant)`, `users(id, username, sys_tenant)`, `audit_details(id, system_id FK→system, change_type enum, description text, time_committed, committer jsonb, user_id FK→users, sys_tenant)`, `contribution(id, ehr_id, contribution_type enum, state enum, signature, has_audit FK→audit_details, sys_tenant)`, `stored_query`, `template_store`, plus a `plugin` key-value table. **Every table gets `ENABLE`+`FORCE ROW LEVEL SECURITY` and a `ehr_policy_all … USING (sys_tenant = current_setting('ehrbase.current_tenant')::smallint)` policy.** |
| per-node tables | `ehr/V3__locatable.sql` | six tables: `comp`/`comp_history`, `ehr_status`/`ehr_status_history`, `ehr_folder`/`ehr_folder_history`. Current + history **pairs**, not one temporal table. Columns per node row: `vo_id, num, ehr_id, contribution_id, audit_id, [template_id], citem_num, rm_entity, entity_concept, entity_name collate "en_US", entity_attribute, entity_path collate "C", entity_path_cap collate "C", entity_idx collate "C", entity_idx_cap collate "C", entity_idx_len, data jsonb, sys_tenant, sys_version, sys_period_lower` (+ `sys_period_upper`, `sys_deleted` on the `_history` twin). |
| tenancy removed | `ehr/V5_1` … `V5_4__remove_multi_tenancy.sql` | `V5_1` drops every `sys_tenant` FK, `DROP TABLE tenant`, and every index carrying `sys_tenant`; `V5_2` replaces every composite PK with the single-column form (`comp` → `PRIMARY KEY (vo_id, num)`, `ehr` → `(id)`); `V5_3` `DROP COLUMN sys_tenant` on all twelve tables and re-creates the FKs without it; `V5_4` re-creates the indexes. **EHRbase 2.x ships with no multitenancy and no RLS.** |
| version/data split | `ehr/V6_1__version_tables.sql` … `V6_4` | new metadata-only tables `comp_version(vo_id PK, ehr_id, contribution_id, audit_id, template_id, sys_version, sys_period_lower)` and `comp_version_history(… , sys_period_upper, sys_deleted, PK (vo_id, sys_version))`, likewise `ehr_status_version[_history]` (PK `ehr_id`) and `ehr_folder_version[_history]` (PK `(ehr_id, ehr_folders_idx)`). `V6_2` back-fills them from the `num = 0` rows of `comp`/`comp_history`; `V6_3` renames `comp` → `comp_data` and **drops `ehr_id`, `contribution_id`, `audit_id`, `template_id`, `sys_version` and `sys_period_lower` from it** — the node row keeps only `vo_id`, the nested-set/path columns and `data` So from V6 the shape is **`*_version` (one row per version, metadata) + `*_data` (one row per node of the current version) + `*_data_history` (one row per node of every past version)**. |
| nested set added | `ehr/V15__data_tables_add_parent_num.sql` | adds `parent_num` and `num_cap` to every `*_data`/`*_data_history` table and back-fills them in 1000-row batches with a `pg_temp` plpgsql procedure that derives both from the pre-existing `entity_idx_len` depth counter. **Confirms `num_cap`/`parent_num` are a later retrofit onto an `entity_idx`-string model, not the original design.** |
| history re-serialised | `ehr/V25__merge_version_and_data_history_tables.sql` | the migration the task asks about. For each of COMPOSITION / EHR_STATUS / FOLDER it adds `ov_data text` and `ov_ref int` to `*_version_history`, sets `toast_tuple_target = 128` and `ALTER COLUMN ov_data SET STORAGE MAIN`, fills `ov_data` with `string_agg(entity_idx \|\| (CASE WHEN num=0 THEN data-'U' ELSE data END)::text, E'\n' ORDER BY num ASC)` grouped by `(vo_id, sys_version)`, and then `DROP TABLE comp_data_history` / `ehr_status_data_history` / `ehr_folder_data_history`. **Past versions are no longer queryable rows at all — they are one newline-delimited text blob per version, deliberately kept in the main fork rather than TOASTed out.** |

### How it maps AQL CONTAINS

Two mechanisms coexist. The original one is the **materialized index string**:
`entity_idx` / `entity_idx_cap` under `COLLATE "C"` plus `entity_idx_len`
(`ehr/V3__locatable.sql`), so containment is a `C`-collation range on a text
column and depth is an integer. `parent_num` / `num_cap` (`V15`) are the
integer nested set retrofitted on top. The current indexes
(`ehr/V23__add_path_skipping_index.sql`) show which won:

```sql
CREATE INDEX comp_data_path_idx      ON comp_data (vo_id, parent_num, entity_concept)
  INCLUDE (rm_entity, entity_attribute, entity_name, num, num_cap, citem_num, entity_idx);
CREATE INDEX comp_data_path_skip_idx ON comp_data (vo_id, citem_num, num)
  INCLUDE (entity_concept, rm_entity, entity_attribute, parent_num, num_cap, entity_idx);
```

— i.e. a **parent-keyed** index and a **`citem_num`-keyed** "path skipping"
index, both covering (`INCLUDE`) everything the planner needs so the heap is
never touched. Contrast `ehr/V8__vo_data_indexes.sql`, the earlier
seven-column composite
`(vo_id, entity_attribute, entity_idx_len, rm_entity, entity_concept,
entity_name, entity_idx) INCLUDE (entity_idx_cap, num)`.

### `sys_period`

Never a range type. Two plain `timestamptz` columns, `sys_period_lower` on the
current table and `sys_period_lower` + `sys_period_upper` + a `sys_deleted
boolean` on the `_history` twin (`ehr/V3__locatable.sql`,
`ehr/V6_1__version_tables.sql`). No `tstzrange`, no GiST, no temporal
constraint.

### The `ehr_status` / `ehr_folder` split

Not one polymorphic table. Each versioned-object kind gets its own table pair
(and from V6, its own triple), keyed differently: `comp_version` by `vo_id`,
`ehr_status_version` by `ehr_id` (one status per EHR by construction),
`ehr_folder_version` by `(ehr_id, ehr_folders_idx)`.

### `party_proxy` / `users` / `audit_details`

There is **no `party_proxy` table** in 2.x. `audit_details.committer` is a
plain `jsonb` column and `audit_details.user_id` is an FK to a small
`users(id, username)` table (`ehr/V1__ehr.sql`). `system` is a third table the
`audit_details.system_id` FK points at — dropped later by
`ehr/V11__drop_system.sql`. Change type is a Postgres `enum`
(`contribution_change_type`), not a code string.

### JSON key compaction

EHRbase's `data jsonb` uses **short aliases**, not canonical openEHR keys:
`'T'` for `_type`, `'A'` for `archetype_node_id`, `'U'` for `uid`, `'su'`,
`'er'`, `'ns'`, `'X'`, `'V'`. Visible in
`ehr/V3__locatable.sql`'s `ehr_status_subject` index
(`jsonb_extract_path_text(data,'su','er','X','V')`), in
`ehr/V10__ehr_status_subject_aql_idx.sql`
(`(data->'su'->'er'->'X'->'V'->>0)` with `WHERE rm_entity = 'ES'`), and in
V25's `data - 'U'`. FerroEHR explicitly rejects this
("no alias compaction, no synthetic fields", `ehr/0001:20-22`) — the cost is
larger rows, the benefit is storage == API.

### What EHRbase does NOT do

- **No pseudonymisation domain.** One schema, one `data` column; identities
  are `audit_details.committer` jsonb and `users.username`; the subject lives
  inline in `ehr_status.data` and is indexed straight out of it
  (`ehr/V10`). No demographic/linkage split, no role barriers.
- **No archival tier.** No `cold` schema, no union views, no archive markers;
  nothing in V1–V28 moves rows out of the primary tables.
- **No multitenancy or RLS** since `V5_1`–`V5_4` removed both.
- **No materialized whole-version body on the current row.** The current
  version is only the `*_data` node rows; reading a composition means
  re-aggregating them. (Past versions, since V25, are the opposite: only a
  blob, no rows.) FerroEHR carries both at once — see §Defects D2.
- **No branch support in the schema**: `sys_version` is a single `int`, with
  no trunk/branch triple.
- **No per-node denormalisation of the version's identity.** Since
  `ehr/V6_3__version_tables.sql`, `comp_data` carries neither `ehr_id` nor
  `sys_version` nor `audit_id`/`contribution_id`/`template_id` — they live once
  on `comp_version`. FerroEHR's `node` carries `ehr_id` and `sys_version` on
  every row (`ehr/0001:572,581`), which is what lets AQL filter by EHR without
  a join, at the cost of 20 bytes per node row.

## openEHR's own remarks on persistence

Two passages, and nothing more from the specs (a sibling report covers the
rest).

**`docs/specs/openehr/BASE/docs/architecture_overview/master13-deployment.adoc`
§5-tier System Architecture**, lines 3-10 and 37-40:

> The general architectural approach in any openEHR system can be considered
> as 5 layers (i.e. a "5-tier" architecture). The tiers are as follows.
> 1. _persistence_: data storage and retrieval.

and, closing the section:

> In the future, an abstract persistence API and optimised persistence models
> (transformations of the existing RM models) are likely to be published by
> openEHR in order to help with the implementation of databases.

— i.e. persistence is named as a tier and its models are explicitly **future
work not yet published**. The mapping figure is likewise disclaimed: "Clearly
where parts of the architecture are used will depend on various implementation
choices; the mapping shown is therefore not definitive."

**`docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
§Overview**, line 13:

> Although the figure implies physical containment of Versions by a Versioned
> object, this is only one possible implementation. Other implementations
> (e.g. using orthodox relational structures) might use references, separate
> compressed copies, or any other mechanism.

That single sentence is the whole grant of freedom the current schema's
headers lean on (`ehr/0001:269-277` for `vo_version`, `:561-569` for `node`).
Note what it does and does not license: it is about **version containment**,
and it is quoted verbatim for the `node` decomposition too, where it is a
weaker fit — decomposing a COMPOSITION's internal RM containment into rows is
not the same question as where a `VERSION` physically lives inside its
`VERSIONED_OBJECT`. Nothing else in either document constrains the physical
layout.

---

## What I could not verify

- Whether the PostgreSQL planner actually uses skip scan, self-join
  elimination or AIO on any FerroEHR query (P7): no running database, and no
  committed `EXPLAIN` artifact exists.
- The numeric claims C1–C4: they reference closed issues, not committed
  measurement records, so they are not re-checkable from the tree.
- The per-statement cold-tier coverage of `storage/version_repo/meta.rs`
  (revision history / version metadata): the module mixes `vo_version` and
  `vo_version_all`, and I did not enumerate each of its ~20 statements.
- Whether any configuration path mints a subject pseudonym server-side
  (§Pseudonymisation domains): I found none in `service/ehr/`, `privacy/` or
  `service/linkage/`, but did not read the whole `privacy` module.
- Whether the `demographic`/`cold_demographic` column sets really line up at
  runtime (D11): the reasoning is from migration ordering, not from a live
  `\d`.
