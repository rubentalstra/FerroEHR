# Platform crate — `service/message/` + `service/admin/` (W-3f)

Spec-first redesign register for the **Message** (SM `I_MESSAGE_SERVICE` /
`I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE`) and **Admin** (SM `I_ADMIN_SERVICE` /
`I_ADMIN_ARCHIVE` / `I_ADMIN_DUMP_LOAD`) areas of the `ehrbase` platform crate.

**Method (owner ruling 2026-07-12): the spec is mapped ONTO the code, never the
reverse.** §1 builds the register skeleton from the vendored oracle
(RM EHR-Extract IM, RM common import/versioning, the BASE Architecture Overview
integration chapter, the SM interfaces) item-by-item with citations; §2 maps the
existing code onto each item (file:line + verdict); §3 lists code that maps to
**no** spec item; §4 is the consolidated G-row register with dispositions; §5 is
the target `service/message/` + `service/admin/` structure mirroring
`app/ehrbase-sm/src/services/`; §6 is the PORT-NOTE residue (keep / re-verify /
drop). This document supersedes the prior impl-side audits
`docs/design/sm-platform/09-message.md` and `15-admin.md` (their G-rows are
absorbed and re-cited here) and replaces the deleted
`docs/design/sm-platform/10-message-integration.md` / `04-…` / `08-…` digests
that code still cites (see G-D1).

**The SM native trait surface is FIXED** (`ehrbase-sm`, already split into
`services/message/{extract,tdd}.rs` and `services/admin/service.rs`). W-3f
reorganises only the *implementation* in `app/ehrbase/src/service/`.

**Spec oracle** (read before any change):

- SM: `docs/specs/openehr/SM/docs/openehr_platform/master09-message_service.adoc`
  + `master15-admin_service.adoc` (both are pure `include::` shells) and their
  UML class files under `docs/specs/openehr/SM/docs/UML/classes/`:
  `i_message_service.adoc`, `i_ehr_extract_service.adoc`, `i_tdd_service.adoc`,
  `i_admin_service.adoc`, `i_admin_archive.adoc`, `i_admin_dump_load.adoc`,
  `export_spec.adoc`, `dump_load_fail_report.adoc`, `export_format.adoc`,
  `compression_format.adoc`, `encoding_format.adoc`, `platform_service.adoc`.
- RM EHR-Extract IM: `docs/specs/openehr/RM/docs/ehr_extract/master05-openehr_extract_package.adoc`
  (X_VERSIONED_* + §Demographic Referencing), `master06-generic_extract_package.adoc`
  (GENERIC_CONTENT_ITEM), `master09-semantics.adoc` (§Creation Semantics; the
  `namespace = "local"` rewrite step); class tables `org.openehr.rm.ehr_extract.*`.
- RM common change control:
  `docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
  §Copying / §Distributed Versioning / §Version Merging (`IMPORTED_VERSION`,
  Cases 1/2/3, `creating_system_id`, local commit-time rule) + the
  `org.openehr.rm.common.imported_version.adoc` class table.
- BASE Architecture Overview:
  `docs/specs/openehr/BASE/docs/architecture_overview/master14-integration.adoc`
  (§Integration Archetypes, §Data Conversion Architecture — `GENERIC_ENTRY`,
  `FEEDER_AUDIT`, the two-step import switch) + the distilled rows in
  `docs/spec-audit/architecture-overview/CHECKLIST.md` §5.5.1.7/§5.5.1.8.
- `master02-overview.adoc` (component table: Admin = "administrative facilities
  … such as back-up"; System Log = "IHE ATNA-compliant"; §Functional Style
  sanctions the return-value/status error idiom).

**Current code inventory** (verified 2026-07-12, `app/ehrbase/src/service/`):
flat files `message.rs` (1,185 — export+import+extract), `tdd.rs` (233),
`dump_load.rs` (814), `admin.rs` (490 — physical delete + statistics + archive),
plus the trait-adapter `api/admin.rs` (`impl AdminService`/`AdminArchive`, :18/:83).
Trait impls are scattered: `impl EhrExtractService` in `message.rs:592`,
`impl TddService` in `tdd.rs:209`, `impl AdminDumpLoad` in `dump_load.rs:766`,
`impl AdminService`/`AdminArchive` in `api/admin.rs`. Import replay leans on
`service/vobject.rs` (`commit_import`/`commit_import_scoped`); dump/load and
physical delete talk directly to the `0001_baseline.sql` tables.

---

## 1. The spec skeleton (built from the oracle, before any code)

### 1.1 SM interfaces — operation by operation

| # | SM operation (citation) | Requirement |
|---|---|---|
| S-M0 | `I_MESSAGE_SERVICE` — no functions (`i_message_service.adoc`) | Empty interface; nothing to realise. |
| S-M1 | `export_ehrs(an_ehr_id: UUID): List<EXTRACT>` (`i_ehr_extract_service.adoc`) | Whole-EHR export, latest-only default. |
| S-M2 | `export_ehr_extracts(extract_spec: EXTRACT_SPEC): List<EXTRACT>` | One EXTRACT per manifest entity, honouring EXTRACT_VERSION_SPEC + selectors. |
| S-M3 | `import_ehr(an_ehr_id: UUID[0..1], an_extract: EXTRACT)` | Clone whole EHR into empty target; fixed-id else reuse source id. |
| S-M4 | `import_ehr_extract(an_ehr_id: UUID, an_extract: EXTRACT)` | Land VOs into an existing EHR. |
| S-M5 | `import_tdd(an_ehr_id: UUID, tdd: String)` (`i_tdd_service.adoc`) | TDD XML → COMPOSITION → validated commit. |
| S-M6 | `import_tdds` (no signature declared) | Bulk TDD import. |
| S-A1 | `list_contributions(a_service: PLATFORM_SERVICE, time_interval[0..1]): List<UUID>` (`i_admin_service.adoc`) | Contribution ids, optional time range. |
| S-A2 | `contribution_count(a_service, time_interval[0..1]): Integer` | Count. |
| S-A3 | `versioned_composition_count(a_service, time_interval[0..1]): Integer` | Count. |
| S-A4 | `composition_version_count(a_service, time_interval[0..1]): Integer` | Count. |
| S-A5 | `physical_ehr_delete(an_ehr_id: UUID)` — `Pre_has_ehr`, error `ehr_id_does_not_exist` | Physical cascade delete. |
| S-A6 | `physical_party_delete(a_party_id: UUID)` — error `party_id_does_not_exist` | Delete party + relationships. |
| S-A7 | `archive_ehrs(ehr_ids: List<UUID>[0..1])` (`i_admin_archive.adoc`) — "**Move** … to archival storage" | Move EHRs to archive. |
| S-A8 | `archive_parties(party_ids: List<UUID>[0..1])` — "Move … Parties **and relationships**" | Move parties + relationships. |
| S-A9 | `export_ehrs(file_sys_loc, logical_fmt, comp_fmt, enc_format)` (`i_admin_dump_load.adoc`) — error `file_not_writable` | Dump all EHRs to a fs location. |
| S-A10 | `load_ehrs(file_sys_loc)` — "duplicate EHR ids will fail" | Load archive; report dup-id failures. |
| S-A11 | `EXPORT_SPEC` (`export_spec.adoc`): `logical_format[0..1]`, `compression_format[0..1]`, `encoding[0..1]`, `segment_split_size: Integer[1..1]` (kb) | Dump parameters. |
| S-A12 | `DUMP_LOAD_FAIL_REPORT` (`dump_load_fail_report.adoc`): `entity_type`, `entity_id`, `dump_status: Boolean`, `error[0..1]` | Per-entity result. |
| S-A13 | `EXPORT_FORMAT`=`{openehr_canonical_xml, openehr_canonical_json}`; `COMPRESSION_FORMAT`=`{zip, 7z}`; `ENCODING_FORMAT` empty; `PLATFORM_SERVICE`=`{Admin, Definitions, Ehr, Ehr_index, Demographic, Message, Query, System_log}` | Enumerations. |

### 1.2 RM EHR-Extract model (the substance `export`/`import` carry)

| # | Spec item (citation) | Requirement |
|---|---|---|
| R-X1 | `EXTRACT` / `EXTRACT_CHAPTER` / `EXTRACT_CONTENT_ITEM` / `OPENEHR_CONTENT_ITEM` (`master05` §Design; class tables) | LOCATABLE skeleton carrying content items. |
| R-X2 | `X_VERSIONED_OBJECT<T>` attrs: `uid`, `owner_id`, `time_created`, `total_version_count`, `extract_version_count`, `revision_history[0..1]`, `versions[0..1]` (`x_versioned_object.adoc`) | Data-oriented VERSIONED_OBJECT; count fields. |
| R-X3 | Five `X_VERSIONED_*` wrappers (EHR_ACCESS/EHR_STATUS/COMPOSITION/FOLDER/PARTY) (`master05` class list) | Per-kind binding. |
| R-X4 | `EXTRACT_VERSION_SPEC`: `include_all_versions`, `include_revision_history`, `include_data`, `commit_time_interval`; invariant `Includes_revision_history_valid` (`extract_version_spec.adoc`) | Version selection. |
| R-X5 | `EXTRACT_SPEC`: `criteria` (AQL primary-set), `manifest`/`entities`, `item_list`, `link_depth`, `include_multimedia` (`master04-common_package.adoc`) | Content selection. |
| R-X6 | §Demographic Referencing: `PARTICIPATION`/`PARTY_PROXY` preserved, final `OBJECT_ID.value` may be rewritten; master09 step: rewrite `OBJECT_REF.namespace` to `"local"` (`master05` §Demographic Referencing; `master09-semantics.adoc`) | Reference rewrite on export. |
| R-X7 | §Creation Semantics — the extract-building algorithm, primary Composition set, link following (`master09-semantics.adoc`) | Export algorithm. |
| R-X8 | `GENERIC_CONTENT_ITEM` (ISO 13606/CDA) (`master06-generic_extract_package.adoc`) | Non-openEHR content. |

### 1.3 RM common import / versioning semantics

| # | Spec item (citation) | Requirement |
|---|---|---|
| R-I1 | `IMPORTED_VERSION<T>`: wraps an `ORIGINAL_VERSION` in `item`; `uid()`/`preceding_version_uid()`/`lifecycle_state()`/`data()` derived from `item`; own `commit_audit`+`contribution` (`imported_version.adoc`; `master06` §Overview) | Import wrapper preserving original identity. |
| R-I2 | Import change_type = `249|creation|` (`master06` line 65) | Committal code. |
| R-I3 | Case 1 (§Copying line 255): no EHR → create clone re-using **source EHR id**; first receipt of item → new `VERSIONED_OBJECT` with `uid = received uid.object_id()` (line 257); `IMPORTED_VERSION` committed in a CONTRIBUTION (line 259) | Clone-into-empty. |
| R-I4 | Cases 2/3: subsequent copies append later trunk versions; VERSIONED_OBJECT already exists (line 257) | Import-into-existing. |
| R-I5 | `creating_system_id` preserved from source; local modifications force **branch** numbering (lines 238, 263, 240) | Distributed identity. |
| R-I6 | Commit times reflect the **local** act of committal, not the original (line 278) | Time-travel correctness. |
| R-I7 | `ORIGINAL_VERSION` never modified through any copy (line 259); signature travels inside the wrapped version (line 104) | Fidelity + signing. |
| R-I8 | §Version Merging: new `ORIGINAL_VERSION` with `other_input_version_uids` (line 296) | Merge-back (trunk). |

### 1.4 BASE Architecture Overview — integration chapter (master14)

| # | Spec item (citation) | Requirement |
|---|---|---|
| R-N1 | Integration archetypes over `GENERIC_ENTRY`; `FEEDER_AUDIT` holds integration meta-data (`master14` §Integration Archetypes, §Data Conversion Architecture) | Two-step legacy→openEHR conversion. |
| R-N2 | Legacy sources: HL7v2/CDA/ISO 13606/EDIFACT (`master14` §Overview) | External-format import surface. |

---

## 2. Code mapped onto the skeleton (file:line + verdict)

**Message — `EhrExtractService`** (`message.rs`): S-M0 no trait (correct — empty
interface). **S-M1** `export_ehrs` impl `:593`, `build_openehr_content_item` `:190`,
`has_ehr`→`ehr_id_does_not_exist` `:594` — **conformant**. **S-M2**
`export_ehr_extracts` `:602`; version selection `version_selection :452` (R-X4,
invariant `:463`), item resolution `:626`, multimedia `strip_inline_multimedia :480`
(R-X5 partial), link following `:664` (R-X7 partial), `validate_extract_type :543`
— **conformant for the covered envelope**; `criteria`/`commit_time_interval`
**divergent → typed reject** (`:619`, `version_selection`). **S-M3** `import_ehr`
`:694`, source id `source_ehr_id :802`, dup→`ehr_create_fail_duplicate_id` `:728`
(R-I3) — **conformant**. **S-M4** `import_ehr_extract` `:741`,
`commit_import_scoped` (`vobject.rs:1866`) (R-I4) — **conformant**. R-X1/R-X2/R-X3
built in `message.rs` (`x_versioned_type :110`, `:190`) — **conformant** (synthetic
archetype ids, R-X1 caveat → G-M6). R-X6 `namespace="local"` rewrite — **missing**
(`:214`, not flagged) → G-M2. R-X8 `GENERIC_CONTENT_ITEM` — **missing/typed reject**
`:863` → G-M7. R-I1/R-I2/R-I5/R-I6/R-I7 realised in `vobject.rs commit_import*`
(`:1787`/`:1825`) — **conformant** (verified by `service_import.rs`,
`service_branching.rs`). R-I8 merge — **not built, PORT-NOTEd trunk-only**.

**TDD — `TddService`** (`tdd.rs`): **S-M5** `import_tdd` `:210`, envelope
`TddEnvelope :57`/`:76`, `has_ehr`+`template_does_not_exist` preconditions,
body walk `openehr_flat::from_tdd`, validated `create_composition`, returns OVID
— **conformant** (return design-filled). **S-M6** `import_tdds` `:214` — **present,
signature fully design-filled** → G-M8. R-N1/R-N2 integration archetypes /
`GENERIC_ENTRY` / feeder-import — **not a message-service concern here** (RM types
generated, `crates/openehr-rm/src/integration/`; behavioural legacy-format import
out of scope) → §3 spec-silent-for-this-area.

**Admin service** (`api/admin.rs` adapters + `admin.rs` machinery):
**S-A1..S-A4** statistics `admin.rs:203/:229/:256/:280`, `PLATFORM_SERVICE`
scoping `contribution_ehr_scoped :33`, adapter `api/admin.rs:18` — **conformant**
(non-empty for `Ehr`/`Demographic` only → G-A5). **S-A5** `physical_ehr_delete`
`admin.rs:63` (cascade + orphan-audit sweep, `rows==0`→NotFound) — **conformant**
(matches CNF Robot `001-EHR.robot`). **S-A6** `party_physical_delete` `admin.rs:315`
(cascades PARTY_RELATIONSHIP) — **conformant**. **S-A7/S-A8** `archive_ehr_vos`
`:414`/`archive_party_vos` `:448`, adapter `api/admin.rs:83` — **divergent**:
marker-only, no storage movement → G-A2; `archive_parties` omits relationships →
G-A3.

**Admin dump/load** (`dump_load.rs`): **S-A9** `export_ehrs_to :205`,
`plan_segments :174` (byte-driven, unit-tested `:780`), `collect_one_ehr :408`,
`reassemble_version :565` — **conformant** for canonical JSON, uncompressed;
XML/compression **divergent → fail-closed 400** (`:212`/`:218`) → G-A4. **S-A10**
`load_ehrs_from :282`, `load_one_ehr :605` (verbatim re-persist, preserved
ids/audit/commit-times, R-I6/R-I7), dup-id → `DUMP_LOAD_FAIL_REPORT` `:304`
(S-A12) — **conformant**. **S-A11** `ExportSpec` (`ehrbase-sm` admin/service.rs)
mandatory `segment_split_size` — **conformant**. **S-A13** `sm_name()` literals
asserted against spec; empty `ENCODING_FORMAT` correctly dropped — **conformant**.
Repository-completeness (parties + standalone attestations not dumped) →
**divergent from "back-up" role** G-A6.

---

## 3. Code with no spec item (spec-silent / quarantine / delete)

| Code | Disposition |
|---|---|
| `admin_ehr_delete_all` (`ehrbase-sm` admin/service.rs; `dispatch/admin.rs:69`) | **Spec-silent extension** — no SM operation. No openEHR spec governs it; keep, flagged. (G-A1) |
| `vo_archive` table + on-disk archive JSON format (`0001_baseline.sql:558`; `dump_load.rs` record structs) | **Spec-silent — our own design** (openEHR defines no SQL schema / archive form). Keep, flag in schema comments (specs only). |
| `StatTimeRange`, closed `[lo,hi]` inclusivity (`admin.rs:216`) | **Spec-silent** (SM leaves Interval inclusivity unspecified). Keep as documented realization. (G-A7) |
| SM→HTTP status mapping (`ehrbase-rest/src/error.rs`) | **Spec-silent — our own design.** Keep; must NOT cite a deleted design doc. |
| Import replay living in `service/vobject.rs` (`commit_import*`) | Correct home is the versioning engine — **seam, keep**, referenced by `message/` via a `TODO(w3f-integrate)` (see §5). |

No delete candidates in this area — every function maps to a spec item or is a
justified extension.

---

## 4. Consolidated G-row register

Severity: **H** blocks a spec MUST / an active W-2 skip; **M** spec-declared
capability unbuilt or observable divergence; **L** doc-hygiene / defensible read.
"Prior" cross-references the absorbed audits.

| id | citation / flag | sev | disposition |
|---|---|---|---|
| **G-M1** | No REST wire; 10 `Area::Msg` ECC cases `SKIPPED(NativeApiOnly)` vs W-2 zero-skip ruling (`suites/message.rs`; ITS-REST vends no message endpoints) — prior 09-message G-1 | H | **fix-in-rewrite** — mount a config-gated **extension** wire (§5.3) OR reclassify each ECC case N/A with platform-suite citation. Extension = spec-silent transport, flag. |
| **G-M2** | `OBJECT_REF.namespace="local"` rewrite on export not done, not flagged (`master09-semantics.adoc`; `message.rs:214`) — prior 09-message G-4 | M | **fix-in-rewrite** — implement the recursive rewrite in the export path, OR a cited PORT NOTE if verbatim namespaces are kept for round-trip fidelity. Today silently neither. |
| **G-M3** | `EXTRACT_SPEC.criteria` (AQL primary set) not applied (`master04`; `message.rs:619`) — prior 09-message G-2 | M | **PORT NOTE (keep, re-verify)** — typed reject is correct until the `$ehr`-bound AQL export wave; `TODO(w3f-integrate)` on `aql/`. |
| **G-M4** | `EXTRACT_VERSION_SPEC.commit_time_interval` not applied (`extract_version_spec.adoc`; `message.rs:456`) — prior 09-message G-3 | M | **PORT NOTE (keep)** — typed reject; lands with G-M3. |
| **G-M5** | Import stored unvalidated + OPT-unlinked (`vo_version.template_id` NULL); clone leaves `ehr.subject_id` unset (`message.rs:39-47`) — prior 09-message G-7/G-8 | M | **PORT NOTE (re-verify)** — documented limitation; re-state against `templates/`+`storage/` seams, decide whether re-validation is in scope. |
| **G-M6** | Synthetic `archetype_node_id` on the extract skeleton (`LOCATABLE.archetype_node_id[1..1]`; `message.rs:49-55`) — prior 09-message G-9 | L | **PORT NOTE (keep)** — no generating archetype exists; RM class token as placeholder, deliberate. |
| **G-M7** | `GENERIC_CONTENT_ITEM` (ISO 13606/CDA) import unsupported (`master06-generic_extract_package.adoc`; `message.rs:863`) | L | **PORT NOTE (keep)** — typed reject; openEHR-only scope. Ties to R-N1/R-N2 (integration IM behaviour deferred). |
| **G-M8** | `import_tdds` signature entirely design-filled (`i_tdd_service.adoc`; `tdd.rs:214`) | L | **PORT NOTE (keep)** — SM defines none; `(UUID, Vec<String>)→Vec<String>`, fail-fast, flagged extension. |
| **G-D1** | Dangling/dead design-doc + migration citations across `message.rs`, `tdd.rs`, `dump_load.rs:7`, `api/admin.rs`, `ehrbase-sm` error.rs (cite `10-message-integration.md`/`04-…`/`08-…` and `0001_schema.sql`/`0004_vo_attestation.sql` — none exist) — prior 09-message G-5/G-6/G-13 + 15-admin G-7 | M | **fix-in-rewrite (do first, cheap)** — scrub; repoint to this doc / spec sections / `0001_baseline.sql`. Also delete the stale "version-branching = typed rejection" line (`message.rs:27`, contradicted by `vobject.rs:1843`) and the over-broad export PORT NOTE (`message.rs:32-37`). |
| **G-A1** | `admin_ehr_delete_all` — no SM definition (`i_admin_service.adoc`) — prior 15-admin G-2 | L | **PORT NOTE (keep)** — spec-silent extension, honestly flagged (§3). |
| **G-A2** | Archive is a marker, no storage movement ("**Move** … to archival storage", `i_admin_archive.adoc`; `admin.rs:405-472`) — prior 15-admin G-4 | M | **PORT NOTE (re-verify) or fix** — decide: realise a read-path effect for `vo_archive`, or re-state that "move" is partially honoured (storage tier = spec-silent, our own design). `TODO(w3f-integrate)` on `storage/`. |
| **G-A3** | `archive_parties` marks the party VO only, not "and relationships" (`i_admin_archive.adoc`; `admin.rs:461`) — prior 15-admin G-5 | L | **PORT NOTE (keep)** — harmless while G-A2 is a no-op marker; extend when G-A2 is realised. |
| **G-A4** | Export format narrowed to canonical JSON, uncompressed; XML/`zip`/`7z` fail-closed (`export_format.adoc`/`compression_format.adoc`; `dump_load.rs:212/:218`) — prior 15-admin G-3 | M | **PORT NOTE (re-verify) or fix** — implement `openehr_canonical_xml` via `openehr-its::to_canonical_xml` + compression, OR cite the spec enums as deliberately-unbuilt. |
| **G-A5** | `PLATFORM_SERVICE` statistics non-empty for `Ehr`/`Demographic` only (`platform_service.adoc`; `admin.rs:262/:286`) — prior 15-admin G-8 | L | **already-correct (PORT NOTE)** — defensible reading of "versioned content service"; only those hold contributions here. |
| **G-A6** | Dump/load is EHR-content-only; demographic parties + standalone attestations not carried (`master02-overview.adoc:40` "back-up") — prior 15-admin G-6 | M | **PORT NOTE (re-verify)** — faithful to the `export_ehrs` signature (EHR-scoped); incomplete as whole-repository back-up. Decide a demographic dump wave. |
| **G-A7** | `time_interval` inclusivity assumed closed `[lo,hi]` (`i_admin_service.adoc` — Interval, no inclusivity stated; `admin.rs:14/:216`) | L | **already-correct (PORT NOTE)** — SM silent; documented realization. |

---

## 5. Target design

### 5.1 `app/ehrbase/src/service/message/` (mirrors `ehrbase-sm/src/services/message/`)

Split the 1,185-line `message.rs` + 233-line `tdd.rs` along the spec's own
export / import / TDD decomposition, every file ≤ ~700 lines:

```
service/message/
├── mod.rs        # impl EhrExtractService + TddService for EhrbaseService (thin
│                 # delegation, like api/); module docs = the §1 spec map + the
│                 # §6 PORT-NOTE register
├── export.rs     # S-M1/S-M2: build_openehr_content_item, X_VERSIONED_* wrappers
│                 # (R-X1..R-X3), version_selection (R-X4), item/criteria
│                 # resolution (R-X5), strip_inline_multimedia, link following
│                 # (R-X7), namespace="local" rewrite (G-M2)
├── import.rs     # S-M3/S-M4: parse_import_containers, parse_imported_version,
│                 # singleton guards, clone-vs-append dispatch (R-I3/R-I4).
│                 # The actual IMPORTED_VERSION replay stays in versioning/
│                 # (commit_import*) — reached via a TODO(w3f-integrate) seam
└── tdd.rs        # S-M5/S-M6: TddEnvelope parse, from_tdd body walk, validated
                  # commit via templates/ + validation/ (TODO(w3f-integrate))
```

`I_MESSAGE_SERVICE` gets no code (empty interface). Import replay
(`commit_import`/`commit_import_scoped`, currently `service/vobject.rs`) belongs
to the **`versioning/`** engine (RM common master06 §Copying) — `import.rs`
calls it through a `TODO(w3f-integrate): versioning::commit_import` seam rather
than duplicating it.

### 5.2 `app/ehrbase/src/service/admin/` (mirrors `ehrbase-sm/src/services/admin/`)

Split `admin.rs` (490) + `dump_load.rs` (814) + `api/admin.rs`:

```
service/admin/
├── mod.rs        # impl AdminService + AdminArchive + AdminDumpLoad for
│                 # EhrbaseService (the trait adapters — parse ids/bounds,
│                 # delegate); collapses today's split between api/admin.rs and
│                 # dump_load.rs so all three admin traits impl in one place
├── delete.rs     # S-A5/S-A6: physical_ehr_delete (+ _all extension, G-A1),
│                 # party_physical_delete — cascade + orphan-audit sweep
├── statistics.rs # S-A1..S-A4: the four count/list queries + PLATFORM_SERVICE
│                 # scoping (G-A5), StatTimeRange closed-interval (G-A7)
├── archive.rs    # S-A7/S-A8: archive_ehr_vos/archive_party_vos (marker today;
│                 # G-A2/G-A3 — storage movement is a storage/ TODO(w3f-integrate))
└── dump_load.rs  # S-A9..S-A13: export/load engine, plan_segments,
                  # collect_one_ehr, reassemble_version (storage-codec inverse —
                  # TODO(w3f-integrate): storage::codec), load_one_ehr,
                  # DUMP_LOAD_FAIL_REPORT
```

### 5.3 Seams (`TODO(w3f-integrate)` candidates)

- **`versioning/`** — `import.rs` → `commit_import`/`commit_import_scoped`
  (IMPORTED_VERSION replay, R-I1..R-I7); `load_one_ehr`/`insert_version` verbatim
  re-persist (R-I6/R-I7).
- **`storage/`** — `reassemble_version` (node-codec inverse), the `vo_version`/
  `node`/`ehr_folder`/`item_tag`/`vo_archive`/`contribution`/`audit` reads and
  cascade deletes; archive storage-movement (G-A2).
- **`templates/` + `validation/`** — TDD commit (`from_tdd` → WebTemplate walk →
  `create_composition`); import re-validation decision (G-M5).
- **`aql/`** — deferred `EXTRACT_SPEC.criteria` / `commit_time_interval`
  (G-M3/G-M4).
- **The extension wire (G-M1) lives in `ehrbase-rest`**, not here; this crate only
  needs its trait impls reachable. Any message/admin extension route is
  spec-silent transport (ITS-REST vends none) → flag "our own extension".

### 5.4 Rules

Schema settled (`0001_baseline.sql` unchanged unless a G-row forces it; comments
cite specs only). `Platform` trait surface fixed. No `use X as Y`. Zero TODO
except inventoried `TODO(w3f-integrate)`. Integration suites
(`service_extract.rs`, `service_import.rs`, `service_tdd.rs`,
`service_branching.rs`, `service_admin.rs`) are the safety net — pass at close,
never weakened.

---

## 6. PORT-NOTE residue (keep / re-verify / drop)

**Keep (honest, cited):** synthetic extract `archetype_node_id` (G-M6);
`GENERIC_CONTENT_ITEM`/ISO-13606 out of scope (G-M7); `import_tdds` design-filled
signature (G-M8); `admin_ehr_delete_all` extension (G-A1); `PLATFORM_SERVICE`
statistics `Ehr`/`Demographic`-only (G-A5); `time_interval` closed interval
(G-A7); trunk-only version branching / no merge (R-I8) — SM-5 merge deferred;
extension wires are spec-silent transport.

**Re-verify (re-state against the new seams):** import re-validation +
`template_id`-NULL + unset clone `subject_id` (G-M5) — decide scope against
`templates/`+`storage/`; archive "move" semantics (G-A2/G-A3); export
format/compression envelope (G-A4); dump/load repository completeness (G-A6).

**Drop / rewrite (doc-drift — G-D1):** the "version-branching = typed rejection"
line (`message.rs:27`, contradicted by `vobject.rs:1843`); the export PORT NOTE
claiming multimedia/`link_depth`/demographic-following are unbuilt
(`message.rs:32-37` — all three are built); every citation of
`docs/design/sm-platform/10-message-integration.md` / `04-…` / `08-…` and the
non-existent `0001_schema.sql` / `0004_vo_attestation.sql` (repoint to this doc,
the spec sections, and `0001_baseline.sql`).

---

## W-3f closure (2026-07-13)

`message.rs`/`tdd.rs`/`admin.rs`/`dump_load.rs` re-grounded into `service/message/` (`export.rs`, `import.rs`, `tdd.rs`, `mod.rs`) and `service/admin/` (`archive.rs`, `delete.rs`, `dump_load.rs`, `statistics.rs`, `mod.rs`).

| G | Disposition | Evidence |
|---|---|---|
| G-M1 | Reassigned | the config-gated extension wire lives in `ehrbase-rest`, not this crate — `service/message/mod.rs:42-44`; message service is native-API here |
| G-M2 | FIXED in code | `OBJECT_REF.namespace="local"` rewrite on export — `service/message/export.rs:44,421,543` `rewrite_content_refs` |
| G-M3 | PORT NOTE | `EXTRACT_SPEC.criteria` typed reject until `$ehr`-bound AQL export — `service/message/export.rs` (TODO on `aql/`) |
| G-M4 | PORT NOTE | `EXTRACT_VERSION_SPEC.commit_time_interval` typed reject — `service/message/export.rs` |
| G-M5 | PORT NOTE | import stored unvalidated / OPT-unlinked / clone `subject_id` unset — `service/message/import.rs` (re-stated vs `templates/`+`storage/`) |
| G-M6 | PORT NOTE | synthetic `archetype_node_id` on the extract skeleton — `service/message/export.rs` |
| G-M7 | PORT NOTE | `GENERIC_CONTENT_ITEM` (ISO 13606/CDA) typed reject, openEHR-only scope — `service/message/import.rs` |
| G-M8 | PORT NOTE | `import_tdds` design-filled signature — `service/message/mod.rs:126`, `tdd.rs` |
| G-D1 | FIXED (scrub) | dangling `10-message-integration.md`/`0001_schema.sql`/`0004_vo_attestation.sql` citations and the stale version-branching line all removed (grep-clean across `message/`, `admin/`) |
| G-A1 | PORT NOTE | `admin_ehr_delete_all` spec-silent extension — `service/admin/delete.rs` |
| G-A2 | PORT NOTE | archive = marker; physical storage-tier move spec-silent — `service/admin/{mod,archive}.rs:33` |
| G-A3 | PORT NOTE | `archive_parties` marks the party VO only — `service/admin/archive.rs` |
| G-A4 | PORT NOTE | export = canonical JSON, uncompressed; XML/`zip`/`7z` → 400, cited as deliberately-unbuilt enum members — `service/admin/dump_load.rs:201-218` |
| G-A5 | already-correct | `PLATFORM_SERVICE` statistics for `Ehr`/`Demographic` only — `service/admin/statistics.rs` |
| G-A6 | PORT NOTE | dump/load EHR-content-scoped; demographic + standalone attestations out of scope — `service/admin/dump_load.rs:24-33` |
| G-A7 | already-correct | `time_interval` closed `[lo,hi]` (SM silent) — `service/admin/mod.rs` |

Open residue: none — G-M2/G-D1 fixed in code, G-M1 reassigned to `ehrbase-rest`, the remaining message/admin items kept as cited PORT NOTE / already-correct.
