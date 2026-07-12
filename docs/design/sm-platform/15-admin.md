# Admin Service (ADMIN) — spec-conformance audit

Read-only audit of the SM `ADMIN` component (`I_ADMIN_SERVICE`,
`I_ADMIN_ARCHIVE`, `I_ADMIN_DUMP_LOAD` and the archive/dump-load data
structures) against the implementation. Structure mirrors
`docs/design/sm-platform/10-subject-proxy.md`: spec oracle → verified current
state → gap register (G-n rows with exact citations) → target design →
PORT-NOTE residue.

**Verdict up front:** the ADMIN surface is **faithfully and completely
realized at the native-API (service) layer** — every SM operation is present
with signature, precondition, error, and return parity, and the dump/load path
is a genuinely lossless indelible-store migration. The honest residue is
(a) a narrow ITS-REST wire (only two of ten operations are reachable over HTTP,
which is spec-consistent — the ADMIN OAS is dev-branch only), (b) two
spec-declared capabilities that fail-closed rather than function (XML/compressed
export; archive as pure storage-movement), and (c) **stale doc/migration
citations in code** that break the owner's citation-hygiene rule.

---

## Spec oracle (read before any change)

- `docs/specs/openehr/SM/docs/openehr_platform/master15-admin_service.adoc` —
  the ADMIN chapter (`sm.platform.interface.admin` package; it is a pure
  `include::` shell over the five class files below).
- `docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc` —
  `I_ADMIN_SERVICE`: `list_contributions` (0..1), `contribution_count` (1..1),
  `versioned_composition_count` (1..1), `composition_version_count` (1..1),
  `physical_ehr_delete` (0..1, `Pre_has_ehr`, error `ehr_id_does_not_exist`),
  `physical_party_delete` (0..1, error `party_id_does_not_exist`).
- `docs/specs/openehr/SM/docs/UML/classes/i_admin_archive.adoc` —
  `I_ADMIN_ARCHIVE`: `archive_ehrs(ehr_ids [0..1])`,
  `archive_parties(party_ids [0..1])`, both "Move selected … to archival
  storage."
- `docs/specs/openehr/SM/docs/UML/classes/i_admin_dump_load.adoc` —
  `I_ADMIN_DUMP_LOAD`: `export_ehrs(file_sys_loc, logical_fmt, comp_fmt,
  enc_format)`, `load_ehrs(file_sys_loc)`; "Repository need not be empty, but
  import EHRs with duplicate EHR ids will fail"; only error `file_not_writable`.
- `docs/specs/openehr/SM/docs/UML/classes/export_spec.adoc` — `EXPORT_SPEC`:
  `logical_format [0..1]`, `compression_format [0..1]`, `encoding [0..1]`,
  `segment_split_size: Integer [1..1]` (kb).
- `docs/specs/openehr/SM/docs/UML/classes/dump_load_fail_report.adoc` —
  `DUMP_LOAD_FAIL_REPORT`: `entity_type`, `entity_id`, `dump_status: Boolean`,
  `error [0..1]`.
- `docs/specs/openehr/SM/docs/UML/classes/export_format.adoc` (`EXPORT_FORMAT`
  = `openehr_canonical_xml`, `openehr_canonical_json`),
  `compression_format.adoc` (`COMPRESSION_FORMAT` = `zip`, `7z`),
  `encoding_format.adoc` (`ENCODING_FORMAT` — **empty enumeration, no values**),
  `platform_service.adoc` (`PLATFORM_SERVICE` = `Admin`, `Definitions`, `Ehr`,
  `Ehr_index`, `Demographic`, `Message`, `Query`, `System_log`).
- Adjacent: `master02-overview.adoc` (component table line 40 "Admin | Service
  providing administrative facilities on all services … such as back-up";
  line 38 "System Log | IHE ATNA-compliant system log"; §Functional Style
  sanctions the return-value/status error style used here).
- Conformance prior art: the CNF Robot suite
  `docs/specs/openehr/CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`
  (physical EHR delete → `204`, full backing-table cascade). The CNF admin
  schedule `docs/specs/openehr/CNF/docs/platform_test_schedule/master12-func_tc_admin.adoc`
  is TBD.

---

## Verified current state (file:line evidence)

**Native-API catalog** — `app/ehrbase-sm/src/services/admin.rs`:
- `AdminService` trait (`admin.rs:38`): `admin_ehr_delete` (:41),
  `admin_ehr_delete_all` (:56, an extension — see G-2), `admin_list_contributions`
  (:72), `admin_contribution_count` (:86), `versioned_composition_count`
  (:104), `composition_version_count` (:118), `physical_party_delete` (:136).
  Every method defaults to `NotImplemented` (→ `501`).
- `AdminArchive` trait (`admin.rs:163`): `archive_ehrs` (:168),
  `archive_parties` (:186).
- `AdminDumpLoad` trait (`admin.rs:322`): `export_ehrs` (:327),
  `load_ehrs` (:339).
- Data structures: `ExportFormat` (:197) + `sm_name` (:208),
  `CompressionFormat` (:220) + `sm_name` (:230), `ExportSpec` (:254),
  `DumpLoadFailReport` (:281). `StatTimeRange` (:19) realizes the SM
  `Interval<Iso8601_date_time> [0..1]` statistics parameter.

**Service implementation** — `app/ehrbase/src/service/`:
- Trait adapters on `EhrbaseService`: `impl AdminService` (`api/admin.rs:18`),
  `impl AdminArchive` (`api/admin.rs:83`), `impl AdminDumpLoad`
  (`dump_load.rs:766`). Id/ISO-bound parsing + `400` on malformed input:
  `parse_uuid` (`api/admin.rs:103`), `parse_range`/`parse_bound` (:111/:120).
- Physical delete + statistics + archive machinery — `service/admin.rs`:
  `physical_ehr_delete` (:63, capture-audit-ids → cascade delete → sweep
  orphaned audits, `rows_affected == 0` → `NotFound`), `physical_ehr_delete_all`
  (:176), the four statistics queries (:203/:229/:256/:280, static
  parameterized SQL with `PLATFORM_SERVICE` scoping via
  `contribution_ehr_scoped` :33), `party_physical_delete` (:315, cascades the
  party VO + every referencing `PARTY_RELATIONSHIP`), `archive_ehr_vos` (:414),
  `archive_party_vos` (:448).
- Dump/load engine — `dump_load.rs`: `export_ehrs_to` (:205), `load_ehrs_from`
  (:282), `plan_segments` (:174, pure/unit-tested segmenting under
  `segment_split_size` kb), `collect_one_ehr` (:408), `reassemble_version`
  (:565, storage-codec inverse), `load_one_ehr` (:605, verbatim re-persist in
  one transaction with preserved ids/audit/commit-times).

**Wire (ITS-REST)** — `app/ehrbase-rest/src/dispatch/admin.rs`: config-gated
(`admin.enabled`, default off → `404`, `admin.rs:52`); dispatches exactly
`admin_ehr_delete` (:62 → `204`) and `admin_ehr_delete_all` (:69 → `200`
`{"deleted": n}`). The generated route table carries only these two
(`crates/openehr-its/src/rest/generated/admin.rs:52-54`). No other admin
operation is mounted anywhere in `ehrbase-rest`.

**SM → HTTP error mapping** — `app/ehrbase-rest/src/error.rs:51`:
`PreconditionViolation → 400` (:56), `EhrIdDoesNotExist`/`PartyIdDoesNotExist`
→ `404` (:59/:60), `NotImplemented → 501` (:79), `FileNotWritable`/`Exception`
→ `500` (:81). `ServiceError::NotFound` reaches this table via `SmError`.

**Storage** — `app/ehrbase/migrations/ehr/0001_baseline.sql`:
`vo_archive` table (:558, SM-4 marker), `ehr_folder` (:310). Physical delete
relies on the `ON DELETE CASCADE` FK graph in the same baseline; `audit` has
no FK from `ehr` and is swept explicitly.

**Tests** — `app/ehrbase/tests/service_admin.rs`: cascade delete +
isolation (:234), unknown-EHR `NotFound` (:272), bulk delete/skip-missing
(:305), per-service + time-range statistics (:413), party cascade sparing a
partner (:550), idempotent archive with unchanged reads (:676). Dump/load
segmenting is unit-tested in `dump_load.rs:780`.

### Faithful realizations (not gaps)

- **All six `I_ADMIN_SERVICE` operations present** with the correct requirement
  levels, the `has_ehr` precondition (`admin.rs:90`), the two declared errors
  mapped to `404`, and the statistics `Interval<Iso8601_date_time>` honoured
  (`i_admin_service.adoc:15-72`).
- **Physical delete is a true cascade** matching the CNF Robot expectation
  (every backing table returns to baseline; audits that the FK graph cannot
  reach are captured-then-swept — `admin.rs:73-103`). Deliberately bypassing
  versioned-store indelibility here is correct: physical delete is the ADMIN
  escape hatch, outside the RM common master06 §Overview indelibility guarantee
  that governs ordinary versioned content.
- **`I_ADMIN_ARCHIVE` present** with all-or-nothing precondition checks and
  idempotent markers (`admin.rs:414`/`:448`).
- **`I_ADMIN_DUMP_LOAD` present and lossless**: `EXPORT_SPEC` with mandatory
  `segment_split_size`, real byte-driven segmenting (`plan_segments`), per-entity
  `DUMP_LOAD_FAIL_REPORT`s, duplicate-EHR-id reported-not-fatal
  (`i_admin_dump_load.adoc:36` ⇒ `dump_load.rs:304-313`), and re-persist that
  preserves original `OBJECT_VERSION_ID`s / audit provenance / commit times
  rather than replaying through the create path (`dump_load.rs` trait
  losslessness PORT NOTE, :310-317).
- **Enumerations faithful**: `EXPORT_FORMAT`/`COMPRESSION_FORMAT` `sm_name()`
  spellings are asserted against the spec literals (`admin.rs:346`);
  `ENCODING_FORMAT` is correctly dropped because the spec enumeration is empty
  (`encoding_format.adoc` has no values — no representable value to carry).

---

## Gap register

Every gap cites the governing spec text. None is a correctness defect in an
implemented operation; the register is wire coverage, spec-declared-but-unbuilt
capability, and citation hygiene.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **Eight of ten operations have no ITS-REST wire.** Only `admin_ehr_delete` + `admin_ehr_delete_all` are HTTP-reachable (`dispatch/admin.rs:62/:69`; `generated/admin.rs:52-54`). `physical_party_delete`, the four statistics calls, `archive_ehrs`/`archive_parties`, and `export_ehrs`/`load_ehrs` are native-API/CLI-only. This is **spec-consistent** — the ITS-REST ADMIN OAS is dev-branch only and vendors no such routes, and `master15` defines only the abstract service — but it means most of the ADMIN surface cannot be exercised by a REST client or the ECC HTTP runner. | `i_admin_service.adoc:16-72`; `i_admin_archive.adoc`; `i_admin_dump_load.adoc`; `master02-overview.adoc:40`; ITS-REST admin API (dev-branch, unvendored) | Native seam complete; wire limited to physical EHR delete. Recorded PORT NOTEs (`admin.rs:70`, `admin.rs:305-308`). |
| G-2 | **`admin_ehr_delete_all` has no spec at all.** Not in `I_ADMIN_SERVICE`, not in any OAS; an invented bulk-delete convenience. | none (`i_admin_service.adoc` defines single-EHR `physical_ehr_delete` only) | Implemented as an extension: comma/repeated `ehr_id` list, empty ⇒ `400` (refuses implicit delete-all), returns `{"deleted": n}` (`admin.rs:56`, `dispatch/admin.rs:69-95`). Honestly PORT-NOTEd. Our own design — no openEHR spec governs it. |
| G-3 | **Export format envelope narrowed to canonical JSON, uncompressed.** `EXPORT_FORMAT` declares `openehr_canonical_xml`; `COMPRESSION_FORMAT` declares `zip`/`7z`. Both are modelled (`ExportFormat`/`CompressionFormat`) but a request for XML or any compression fails-closed with `precondition_violation` (`400`) instead of functioning. | `export_format.adoc:16`; `compression_format.adoc:16-20`; `export_spec.adoc:20` | `export_ehrs_to` rejects XML (`dump_load.rs:212`) and any compression (:218). PORT-NOTEd on the grounds that storage is verbatim canonical JSON (translation-free) and 7z/zip is an ops nicety. A real spec-declared capability, not yet built. |
| G-4 | **Archive is a read-neutral marker — no storage movement happens.** The SM verb is "**Move** selected EHRs/Parties **to archival storage**"; the implementation only inserts `vo_archive` rows and no read path consults them, so nothing is tiered, moved, or made less available. Archival is effectively a recorded intent. | `i_admin_archive.adoc:16/:28` ("Move … to archival storage") | Marker-only (`admin.rs:405-472`, PORT NOTE at :143-156 / :443-447); storage-tier movement deferred to optimization. No openEHR spec governs the archival storage *form*, but the "move" semantics are unrealized. |
| G-5 | **`archive_parties` marks the party VO only, not "and relationships".** | `i_admin_archive.adoc:31` ("Move selected Parties **and relationships**") | Only the party VO is marked (`admin.rs:461-468`); related `PARTY_RELATIONSHIP` VOs are not. PORT-NOTEd as observably harmless while archival is a no-op marker (G-4). Would need extending once G-4 is real. |
| G-6 | **`export_ehrs` does not carry demographic parties or standalone attestations.** `physical_party_delete`/`archive_parties` operate on ehr-less PARTY VOs, but the dump/load path is EHR-content-only (`ehr`, audit, contribution, `vo_version`, `node`, `ehr_folder`, `item_tag`, `vo_archive`). A repository with demographic data cannot be fully migrated by dump/load. | `i_admin_dump_load.adoc:16` ("Export all EHRs" — literally EHR-scoped, so faithful to the *signature*; but the ADMIN component's "back-up" role, `master02-overview.adoc:40`, implies whole-repository) | Scope PORT NOTE (`dump_load.rs:22-33`): parties + standalone `vo_attestation` out of scope this wave. Global DEFINITION artefacts (templates/stored queries) must pre-exist on the target. Faithful to `export_ehrs` narrowly; incomplete as repository back-up. |
| G-7 | **Stale/dangling citations in code (violates the citation-hygiene rule).** `dump_load.rs:7` cites `docs/design/sm-platform/04-message-subject-proxy-terminology-admin.md`, `admin.rs` (`ehrbase-sm`) :251 cites the same 04-doc, and `error.rs:16-17` (`ehrbase-sm`) cites `docs/design/sm-platform/08-target-architecture.md` — **none of those files exist** (only `10-subject-proxy.md` + `README.md` remain). Additionally `service/admin.rs:46` cites migration files `0001_schema.sql` + `0004_vo_attestation.sql` that do **not** exist (the schema was squashed to `0001_baseline.sql`). | `.claude/rules/spec-adherence.md` (cite specs, keep citations findable; scrub dead references) | Four dead references across three files. This document supersedes the deleted design docs for ADMIN; the migration citation should point at `0001_baseline.sql`. |
| G-8 | **`PLATFORM_SERVICE` statistics scoping is a fixed-decision subset.** Only `Ehr` (EHR-scoped) and `Demographic` (ehr-less) yield contribution statistics; `versioned_composition_count`/`composition_version_count` gate on `Ehr` alone; every other member returns empty/0. Correct for this CDR (only those hold contributions), but the SM does not say the other six are always-empty. | `platform_service.adoc:16-45`; `i_admin_service.adoc:24` ("Name of a versioned content service") | `contribution_ehr_scoped` (`admin.rs:33`) + `service != Ehr ⇒ 0` (`admin.rs:262/:286`). Documented; defensible reading of "versioned content service". |
| G-9 | **`time_interval` inclusivity assumed closed.** The SM `Interval<Iso8601_date_time>` bound inclusivity is not specified for these calls; the impl treats it as closed `[lo, hi]` (`>=`/`<=`). | `i_admin_service.adoc:18` (Interval type, no inclusivity stated) | PORT-NOTEd assumption (`admin.rs:14-18`, SQL `admin.rs:216-217`). Harmless but an undocumented-in-spec choice. |

---

## Target design (to close the register)

The service layer needs no structural change — it is spec-true. The work is
wire exposure, two capability completions, and a citation scrub.

### 1. Citation scrub (G-7) — do first, cheap, blocking the hygiene rule

- `dump_load.rs:7`, `ehrbase-sm/src/services/admin.rs:251`,
  `ehrbase-sm/src/error.rs:16-17`: drop the dead `04-…`/`08-…` design-doc
  references; where a rationale is load-bearing, cite the spec file + section
  (or this document for ADMIN-specific design). `service/admin.rs:46`: repoint
  `0001_schema.sql`/`0004_vo_attestation.sql` to `0001_baseline.sql` (the actual
  squashed baseline; the `vo_attestation` and cascade FKs live there).
- No openEHR spec governs the SM → HTTP mapping table location — flag it as our
  own design rather than citing a deleted doc.

### 2. Admin extension wire (G-1) — config-gated, out of CORE/STANDARD scope

The ADMIN OAS is dev-branch only, so this is an **extension** surface (the same
posture as `/terminology` and the tenant/event admin routes already mounted in
`dispatch/mod.rs`), documented as an extension and behind `admin.enabled`:

```
DELETE /admin/ehr/{ehr_id}                     admin_ehr_delete        (exists)
DELETE /admin/ehr/all{?ehr_id*}                admin_ehr_delete_all    (exists, extension)
DELETE /admin/party/{party_id}                 physical_party_delete   → 204 / 404
GET    /admin/statistics/contributions{?service,from,to}   list/count
GET    /admin/statistics/versioned_compositions{?from,to}  count
GET    /admin/statistics/composition_versions{?from,to}    count
POST   /admin/archive/ehrs                     archive_ehrs            → 204 / 404
POST   /admin/archive/parties                  archive_parties         → 204 / 404
POST   /admin/dump                             export_ehrs (EXPORT_SPEC body) → report[]
POST   /admin/load                             load_ehrs                → report[]
```

- Reuse the existing `sm_api_error` table: `party_id_does_not_exist` → `404`,
  `precondition_violation` → `400`, `file_not_writable` → `500` (already wired).
- ATNA system-log events on every mutating admin call (`master02-overview.adoc:38`).
- OAS into the extension bundle (`scripts/assemble-oas.sh`); website book page
  same-PR. This makes the whole ADMIN surface ECC-executable rather than
  native-only adjudications.

### 3. Complete the archive "move" semantics (G-4/G-5)

Decide and record one of:
- **Realize the move**: a read-path effect for `vo_archive` (e.g. archived
  versioned objects served only via an explicit archival read, or physically
  relocated to a tiered store), and extend `archive_parties` to the related
  `PARTY_RELATIONSHIP` VOs (G-5). No openEHR spec governs the archival storage
  *form* — our own design — but "move to archival storage" must have an
  observable effect to be more than a marker.
- **Or keep the marker and re-state the PORT NOTE** with a citation that the
  storage tier is our own deferred design, not a spec requirement — but then the
  operation's spec verb ("Move") is only partially honoured, and that must be
  said plainly, not implied as done.

### 4. Export format envelope (G-3) + repository completeness (G-6)

- Either implement `openehr_canonical_xml` (re-serialize each `body` through
  `openehr-its::to_canonical_xml`) and `zip`/`7z` compression, or keep the
  fail-closed `400` and record it as a deliberate, spec-declared-but-unbuilt
  format with a citation to `export_format.adoc`/`compression_format.adoc`
  (not a deleted design doc).
- For back-up completeness (G-6), extend dump/load to demographic PARTY VOs and
  standalone attestations, or PORT-NOTE that `export_ehrs` is EHR-scoped by its
  signature and a separate demographic dump is future work.

### 5. `time_interval` inclusivity (G-9)

Keep closed `[lo, hi]`; add a one-line note that the SM leaves inclusivity
unspecified and this is our documented realization (already PORT-NOTEd at
`admin.rs:14`).

---

## Standing PORT-NOTE residue (the honest set after closure)

- `admin_ehr_delete_all` is an extension with no SM definition (G-2).
- The ADMIN REST surface is an extension (ITS-REST vendors no ADMIN OAS;
  `master15` is abstract-only) — out of CORE/STANDARD conformance scope (G-1).
- Archive is (until G-4 is realized) a recorded marker; "move to archival
  storage" is partially honoured, and `archive_parties` covers the party VO
  only (G-4/G-5).
- `export_ehrs` supports canonical JSON, uncompressed, EHR-scoped content only;
  XML/compression fail-closed and demographic parties are out of scope (G-3/G-6)
  — no openEHR spec governs the archive storage form, but the declared
  `EXPORT_FORMAT`/`COMPRESSION_FORMAT` values are not all functional.
- `PLATFORM_SERVICE` statistics are non-empty for `Ehr`/`Demographic` only,
  a defensible reading of "versioned content service" (G-8).
- `time_interval` treated as closed `[lo, hi]` where the SM is silent (G-9).
- No openEHR spec governs the SM → HTTP status mapping, the `vo_archive` table,
  or the on-disk archive format — all our own design, and must be cited as such
  rather than via the deleted `docs/design/sm-platform/04-…`/`08-…` docs (G-7).
