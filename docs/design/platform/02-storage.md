# Platform storage layer — spec-first redesign (W-3f)

Read-only audit + target design for the storage half of the `ehrbase` platform
crate: the codec that decomposes a versioned object (COMPOSITION / EHR_STATUS /
FOLDER / demographic parties) into stored `node` rows of **canonical openEHR
JSON fragments** and reassembles them, plus the `node`/`vo_version`/`audit`/
`contribution` row plumbing and the `db/` foundation.

**Method (owner ruling — map the spec ONTO the code):** §1 enumerates every
requirement the openEHR spec actually places on *persisted data*, each with its
citation, then maps the current code onto it with a verdict. §2 audits the
spec-silent internals under the explicit flag *no openEHR spec governs this — our
own design*. §3 is the G-row register. §4 is the target `storage/` + `db/`
layout and the seam split with `versioning/` (register 01) and `aql/`. §5 is the
PORT-NOTE residue.

**Scope seam with register 01 (versioning).** Register `01-versioning` owns the
*semantics* of the version tree, lifecycle, and the contribution/audit change-set
rules. This register owns the *row layout that makes those semantics true*:
decompose/reassemble, the node/version SQL plumbing, and row mapping. Where a §1
requirement is a versioning semantic, the verdict states "upheld by storage,
enforced by register 01" and the item is cross-referenced, not re-owned.

**Migration `0001_baseline.sql` is SETTLED** (blueprint §2.1, ADR-013). No G-row
below demands a schema change; every finding is Rust-layer or documentation.

## Oracles

- `BASE/docs/base_types/master05-identification_package.adoc` — identifier
  lexical/equality forms at the storage boundary.
- `BASE/docs/foundation_types/master03-primitive_types.adoc`,
  `master05-interval.adoc`, `master06-time_types.adoc` — primitive/time/interval
  forms the codec must preserve.
- `RM/docs/common/master06-change_control_package.adoc` (+ `RM/docs/UML/classes/
  org.openehr.rm.common.*`) — the change-control row contract storage upholds.
- `docs/architecture.md` §Storage + `docs/design/aql-engine.md` — the own-design
  ground for the decomposed node model.
- `docs/spec-audit/architecture-overview/CHECKLIST.md` — storage/persistence rows.

Code audited: `app/ehrbase/src/storage/{codec.rs,mod.rs,error.rs}`,
`app/ehrbase/src/db/{iden,pool,migrate,settings,mod,error}.rs`,
`app/ehrbase/migrations/{ehr/0001_baseline.sql,ext/0001_openehr_functions.sql}`,
the storage half of `app/ehrbase/src/service/vobject.rs`, and the duplicated
reload in `app/ehrbase/src/service/dump_load.rs`.

---

## 1. Spec-governed requirements (the spine)

### 1.A — Identifier lexical/equality forms at the storage boundary

Source: `BASE/docs/base_types/master05-identification_package.adoc`.

| # | Requirement (citation) | Code map | Verdict |
|---|---|---|---|
| A1/A2 | `OBJECT_VERSION_ID` = `object_id '::' creating_system_id '::' version_tree_id` (§"Identifying Versions…" + §Syntaxes; `OBJECT_VERSION_ID Class`) | Stored **decomposed** into columns: `vo_version.vo_id` (= object_id), `creating_system_id text`, `trunk_version/branch_number/branch_version int` (`0001_baseline.sql:187`); reassembled to the wire form by `service/version_id.rs`. Parts preserved losslessly. | conformant (reassembly owned by register 01) |
| A3 | `VERSION_TREE_ID` = `trunk_version['.'branch_number'.'branch_version]`; `Trunk_version_valid ≥1`, `Branch_validity` (both branch parts or neither), first = `"1"` (`VERSION_TREE_ID Class`) | `ck_vo_version_trunk_version_positive`, `ck_vo_version_branch_pair` (`(0,0)` xor `(≥1,≥1)`), `ck_vo_version_sys_version_positive` (`0001_baseline.sql:235-243`) | conformant |
| A7 | `ARCHETYPE_ID` = `rm_originator '-' rm_name '-' rm_entity '.' concept{'-'spec}* '.v' version` (`ARCHETYPE_ID Class` + §Syntaxes) | `codec.rs:260 archetype_parts` parses via the shared `ArchetypeId` type (never a hand-rolled regex) into `arch_entity/arch_concept/arch_major`; at/id-codes leave them NULL | conformant |
| A10 | Composite identifiers are **case-preserving** AND **case-insensitive** (§"Composite Identifiers and Case", verbatim) | `node.archetype` stored **verbatim** (case-preserving ✓); `arch_*` stored lowercased for comparison; `idx_node_archetype_lower` functional index serves case-folded AQL predicates (`0001_baseline.sql:340-395`) | conformant — the headline storage-boundary rule |
| A8 | `TEMPLATE_ID` lexical form is "**to be determined**" (`TEMPLATE_ID Class`) | `template_store.template_id text`, `vo_version.template_id text` — free string, no validity constraint | conformant-by-silence → **PORT NOTE** (§5) |

**Seam note (A5/A6, HIER_OBJECT_ID / UID):** the EHR id, `CONTRIBUTION.uid`, and
`VERSIONED_OBJECT.uid` are bare `uuid` columns (`ehr.id`, `contribution.id`,
`vo_version.vo_id`) — a `HIER_OBJECT_ID` with no `::extension` (satisfies
`VERSIONED_OBJECT.Uid_validity: extension.is_empty`, C3). Parse/format of the
wire form is register 01's; storage stores the uuid.

### 1.B — Primitive / time / interval forms the codec must preserve

Source: `BASE/docs/foundation_types/master03,05,06`. **The codec's whole design
answer to this area is "store the canonical JSON fragment verbatim, prune only
structure children"** (`node.data jsonb`, `0001_baseline.sql:363`;
`codec.rs:208`). The codec operates on `serde_json::Value` and never re-formats a
leaf value.

| # | Requirement (citation) | Code map | Verdict |
|---|---|---|---|
| B2 | String content is UTF-8/Unicode (`master03` §Unicode) | JSON strings stored in `jsonb` — byte-preserved | conformant |
| B3/B4/B5 | ISO-8601 date/time have a **String** physical form; extended vs compact and **partial precision** (`YYYY`, `YYYY-MM`, `hh:mm`) must both be supported and **not silently normalized** (`master06` §Primitive Time Types + NOTE; `Iso8601_*.is_partial`) | These live as **JSON strings** inside the fragment; `jsonb` preserves string bytes exactly, so lexical form + partial precision round-trip verbatim through decompose/reassemble | conformant (string-encoded → safe) |
| B6 | Fractional-second presence and `,`-vs-`.` decimal sign are significant (`Iso8601_time.has_fractional_second`, `is_decimal_sign_comma`) | String-encoded inside the fragment → preserved | conformant |
| B9/B10 | Timezone suffix (`Z` vs `+00:00`) and duration lexical form (W-combined, leading `-`) preserved (`Iso8601_timezone/duration Class`) | String-encoded → preserved | conformant |
| B1 | `Integer`(i32) / `Integer64` / `Real`(f32) / `Double`(f64) are distinct; must not collapse (`master03` §Overview) | `DV_QUANTITY.magnitude`/`DV_COUNT.magnitude` are **JSON numbers**, stored as `jsonb numeric`. The Rust codec is lossless at the `Value` level (`round_trips_losslessly` test); the **`jsonb` layer** may re-represent a number (e.g. `120.0`↔`120`, exponent form) | conformant at value-level; **representation caveat** below |
| B11 | `DV_INTERVAL` structural flags (`_unbounded`/`_included`) and `Limits_consistent` (`master05-interval.adoc Interval Class`) | Interval JSON stored verbatim inside its fragment; flags preserved | conformant (validity enforced upstream) |

**B representation caveat (own-design boundary):** `node.data` is `jsonb`, which
canonicalizes key order, whitespace, and numeric representation — the stored
bytes are **not** byte-identical to the received JSON. Fidelity is guaranteed at
the openEHR *value* level (the full-corpus jsonb round-trip gate is green,
blueprint §2.1) and re-canonicalized on read by `openehr-its`. The whole-number
`f64` representation item (`120.0` vs `120`) is tracked by the ITS canonical-JSON
layer (blueprint ch5 §F), **not** storage — flag, do not re-own.

**B seam (B7 `Day_valid` reject `2021-02-31`, B8 `24:00:00` forbidden, B9 tz
bounds, B11 interval consistency):** these are **validation** obligations, not
storage's. The codec stores what it is handed and must not fabricate or reject
leaf values. Correctly out of storage scope; the seam is *validation runs before
the commit transaction opens*.

### 1.C — Change-control row contract (storage upholds; register 01 owns semantics)

Source: `RM/docs/common/master06-change_control_package.adoc` + class tables.

| # | Requirement (citation) | Code map | Verdict |
|---|---|---|---|
| C1 | **Indelibility** — every change is a new physically-committed Version; never overwrite/destroy (§Contributions, verbatim) | `vo_version` + `node` are **append-only per version**, keyed `(vo_id, sys_version[, num])` (`0001_baseline.sql:232,368`). The only UPDATE is `close_ordinal_at_now` setting `sys_period`'s upper bound (supersession metadata, not data) (`vobject.rs:1989`) | conformant |
| C2 | Logical deletion = new Version, data Void, lifecycle `523\|deleted\|` (§Logical Deletion) | Delete writes a content-less version (no `node` rows); `version_read` returns `Value::Null` for lifecycle `523`, skipping reassembly (`vobject.rs:256`) | conformant |
| C3 | Each version `uid.object_id` == container `uid`; container uid has no extension (`VERSION.Owner_id_valid`, `VERSIONED_OBJECT.Uid_validity`) | All versions of one object share `vo_id` (the object_id); `vo_id` is a bare uuid (no extension) | conformant |
| C4 | ORIGINAL_VERSION row retains `uid`/`preceding_version_uid`/`other_input_version_uids`/`lifecycle_state`/`attestations`/`data`/`contribution`/`commit_audit`/`signature` (`ORIGINAL_VERSION Class`) | Every field has a column: `preceding_version_uid`, `other_input_version_uids jsonb`, `lifecycle_state`, `signature`, `contribution_id`, `audit_id` (`0001_baseline.sql:187-270`); attestations in `vo_attestation`; data in `node` | conformant |
| C6 | `lifecycle_state` coded from the `version_lifecycle_state` group (`VERSION.Lifecycle_state_valid`); five codes `532/553/523/800/801` | `ck_vo_version_lifecycle_state CHECK` exactly that set (`0001_baseline.sql:248`) | conformant |
| C7 | `incomplete` (553) relaxes content validity (§Incomplete Content, verbatim) | `lifecycle_state` default `532`; content never `NOT NULL`; 553 permitted by the CHECK | conformant (relaxation owned by validation) |
| C8 | Stored `data` is the **canonical serial form**; `signature` covers all-but-signature; exact serialization **spec-TBD** (§Digital Signature) | `node.data` is canonical ITS-JSON verbatim; `vo_version.signature text` (0..1); canonicalization TBD | conformant → **PORT NOTE** (§5) |
| C9 | Every version links its CONTRIBUTION; audit `system_id`/`committer`/`time_committed` copied into each version's `commit_audit` (§Committal and Audits, verbatim) | `contribution_id`/`audit_id` FK **NOT NULL** on every `vo_version`; `write_contribution` emits audit + contribution in one tx (`vobject.rs:375`); `version_read` reconstructs `commit_audit` from the joined `audit` row (`vobject.rs:271`) | conformant |
| C10 | CONTRIBUTION is **atomic** (§Committal and Audits, verbatim) | One caller-owned `sqlx::Transaction`: `insert_vo_version` + `insert_nodes` + `write_contribution` + outbox commit together (`vobject.rs:646` `apply_change`) | conformant |
| C11 | AUDIT_DETAILS = `system_id`(1..1, non-empty)/`time_committed`(server)/`change_type`(coded)/`committer`(1..1) (`AUDIT_DETAILS Class`) | `audit` table + `ck_audit_system_id_nonempty`, `ck_audit_change_type`, server `time_committed DEFAULT now()` (`0001_baseline.sql:91-121`) | conformant |
| C13 | IMPORTED_VERSION wraps an ORIGINAL_VERSION; the **original audit/contribution are retained**, the local committal is separate (§Committal and Audits, verbatim) | `insert_imported_vo_version` preserves `creating_system_id` + `preceding_version_uid` verbatim from the source (`vobject.rs:1622`) | **partial — see seam** |
| C15 | VERSIONED_OBJECT holds all trunk + branch versions; `latest_version` vs `latest_trunk_version`; time-travel (`VERSIONED_OBJECT Class`, §Distributed Versioning) | One temporal `vo_version` table (no current/history split); `ALL_VERSIONS` = unfiltered, `LATEST_VERSION` = `uq_vo_version_current` partial index; branches via `trunk/branch` columns + per-lineage `EXCLUDE` (`0001_baseline.sql:271-278`) | conformant — the ADR-008 headline |

**C5 seam** (`Preceding_version_uid_validity: is_first xor preceding /= Void`):
`preceding_version_uid` is nullable and stored; the first⇔null invariant is
enforced by register 01 at commit, **not** a DB CHECK. Correct ownership; no
storage defect.

**C13 seam (partial):** storage preserves the imported version's
`creating_system_id`/`preceding_version_uid`, but whether the **full wrapped
ORIGINAL_VERSION with its original `commit_audit` intact** is retained distinctly
from the local committal audit is a register-01 (versioning/import) semantic —
audited there, not re-owned here. Flag: confirm the import path stores the
original audit, not only the local one.

**C16 seam:** `VERSIONED_OBJECT.time_created` (1..1) has no dedicated container
row — it is derived from the earliest version's audit time. Register-01 concern;
low severity (derivable, lossless).

---

## 2. Spec-silent internals — *no openEHR spec governs this; our own design*

openEHR defines **no SQL schema** (blueprint §2.1). Everything below is our own
PG18-native design (`docs/architecture.md` §Storage; `docs/design/aql-engine.md`),
grounded on docs-verified PostgreSQL physics (no partial jsonb detoast; GIN
serves no ordering).

- **Decomposed node model** — one `node` row per RM structure node, per version;
  `STRUCTURE_TYPES` const enumerates which `_type`s become rows (`codec.rs:29`).
  Leaf content stays inline in each row's `data` fragment. Own design; the AQL
  engine's RM model `is_structure_root` must match this set (`aql-engine.md:22`).
- **Nested-set index** — `num`/`num_cap`/`parent_num`/`citem_num` make AQL
  CONTAINS an integer interval join (`d.num BETWEEN a.num AND a.num_cap`), never a
  JSON walk (`codec.rs:81`; `0001_baseline.sql:327`; `aql-engine.md:56`).
- **Materialized path** `path text COLLATE "C"` — byte-order = tree order;
  reassembly only, never an AQL predicate (`0001_baseline.sql:362`).
- **Promoted subsumption columns** `arch_entity/arch_concept/arch_major` + the
  `idx_node_arch_subsume` prefix index (realize AM master07 §Querying *as an
  index* — the index is own design).
- **`ext` IMMUTABLE helpers** — `openehr_magnitude(jsonb)` + ISO-8601 second/day
  helpers, legal in btree expression indexes (`ext/0001`); `idx_node_magnitude`
  is SPECULATIVE/P20-repriced (own design, PERF residue §5).
- **`lz4` COMPRESSION** on `node.data`/`audit.committer` — storage tuning.
- **Temporal `vo_version`** — `sys_period tstzrange` + per-lineage GiST `EXCLUDE`
  (needs `btree_gist`, bootstrapped in `migrate.rs:21`); `fillfactor 90`.
- **`db/` foundation** — `pool.rs` (`search_path ehr,ext,public`; tenant-scoped
  `before_acquire` GUC stamp for RLS), `migrate.rs` (two migrators `ext`→`ehr`,
  each own `_sqlx_migrations`), `settings.rs`, `iden.rs` (sea-query name catalog),
  `error.rs`.
- **Supporting tables** owned mostly by other registers but co-resident:
  `ehr_folder`, `item_tag`, `vo_archive`, `ehr_index`, `stored_query`,
  `template_store`, `archetype_store`, `adl2_artefact`, `sp_*`. This register owns
  the **`node`/`vo_version`/`audit`/`contribution`** spine.

---

## 3. G-row register

Every divergent/missing verdict from §1 + every unmapped-code classification.

| id | item | citation / flag | severity | disposition |
|---|---|---|---|---|
| G-S1 | `read_nodes` (`vobject.rs:2105`) and `dump_load.rs reassemble_version` (`dump_load.rs:565`) are **near-verbatim duplicate** node→canonical reload plumbing (same SELECT, same `NodeRow` build, same `reassemble`) | own-design (DRY) | med | **fix-in-rewrite** — one `storage::node_repo::read_version_canonical` |
| G-S2 | `insert_nodes` (`vobject.rs:422`), the node bulk-insert, lives in the service file, not `storage/` | own-design (layering) | med | **fix-in-rewrite** — move to `storage::node_repo::write_nodes` |
| G-S3 | `db::iden::Node` enum is **missing** `arch_entity/arch_concept/arch_major`; `VoVersion` is **missing** `trunk_version/branch_number/branch_version/preceding_version_uid` — the Iden catalog has drifted from the schema, and those columns are addressed only via raw SQL strings | own-design (`iden.rs` doc claims "single typed name catalog") | med | **fix-in-rewrite** — complete the catalog |
| G-S4 | `codec.rs STRUCTURE_TYPES` is a hand-maintained const of RM type names; the BMM-generated AQL RM model has an independent `is_structure_root` that "matches the codec's decompose rule" (`aql-engine.md:22`) — **two sources, drift risk** | own-design; AM/RM model is BMM-generated | med | **fix-in-rewrite** candidate — derive from the RM model or add an agreement test |
| G-S5 | `VersionRead.creating_system_id` "legacy empty-string sentinel" fallback (`vobject.rs:201`) is **dead code** — greenfield, nothing deployed, no pre-column rows exist; the schema forbids the empty sentinel (`0001_baseline.sql:214`) | own-design (contradicts schema comment) | low | **fix-in-rewrite** — drop the fallback |
| G-S6 | `read_nodes`/`reassemble_version` SELECT `rm_type/archetype/name` but `reassemble` uses only `num/num_cap/parent_num/path/data` — unused columns fetched | own-design (efficiency) | low | **fix-in-rewrite** — lean read row |
| G-S7 | Node/version/audit SQL is hand-written raw strings via `QueryBuilder`/`query()` rather than compile-checked `query!`/sea-query | `sqlx-conventions.md` ("prefer `query!` where static") | low | **already-correct** for static SQL (raw is permitted); dynamic AQL SQL correctly uses sea-query. Optional `query!` tightening. |
| G-A8 | `TEMPLATE_ID` stored as free string | A8 (spec silent) | low | **PORT NOTE** (§5) |
| G-C8 | signature canonicalization spec-TBD | C8 (spec TBD) | low | **PORT NOTE** (§5) |
| G-B1 | `jsonb` numeric re-representation (`120.0`↔`120`) | B1 caveat; owned by ITS canonical-JSON | low | **quarantine** — tracked by ITS layer (blueprint ch5 §F), not storage |
| G-C13 | confirm import retains the wrapped original's `commit_audit` distinctly | C13 seam | med | **register-01** — verify in versioning audit |

No G-row demands a `0001_baseline.sql` change (all Rust-layer or documentation);
the schema stays SETTLED.

---

## 4. Target design

Fresh `app/ehrbase/src/storage/` (files ≤ ~700 lines), with the storage half of
`service/vobject.rs` split in.

```
app/ehrbase/src/storage/
  mod.rs            module doc + re-exports (decompose, reassemble, NodeRow, node_repo::*)
  row.rs            NodeRow (write) + a lean ReadRow (num/num_cap/parent_num/path/data) — G-S6
  codec.rs          decompose / reassemble / walk / prune_children / attach / split_step (~500 ln)
  structure.rs      STRUCTURE_TYPES, is_structure_type, is_versioned_root_type, archetype_parts
                    — sourced from (or agreement-tested against) the BMM RM model — G-S4
  node_repo.rs      write_nodes(tx, vo_id, sys_version, ehr_id, &[NodeRow])   ← from vobject.rs:422
                    read_version_canonical(pool, vo_id, sys_version) -> Value ← consolidates
                    vobject.rs:2105 + dump_load.rs:565 (G-S1/G-S2)
  error.rs          StorageError (unchanged)
```

`db/` stays as-is except **G-S3** (complete the `iden` catalog); `pool.rs`,
`migrate.rs`, `settings.rs`, `error.rs` are already correct.

**The storage/versioning split (register 01 ↔ this register).** `vobject.rs`
divides cleanly:

- **register 01 (versioning/) keeps:** `apply_change`, `next_version`,
  `resolve_lifecycle`, `insert_vo_version`/`insert_imported_vo_version`,
  `close_ordinal_at_now`, `sign_version`, `write_contribution`/`insert_audit`/
  `insert_contribution*`, `write_outbox`, `attest`, `Kind`, `Committed`,
  `VersionRead`, `AuditInput`.
- **this register (storage/) takes:** `decompose`/`reassemble` (already here) +
  the extracted `node_repo` (`write_nodes`, `read_version_canonical`) + `NodeRow`.

The **seam is a value contract, not shared SQL:**

- write: versioning computes `(vo_id, sys_version, ehr_id, canonical: Value)` and
  calls `storage::node_repo::write_nodes` (which decomposes + bulk-inserts);
- read: versioning calls `storage::node_repo::read_version_canonical(vo_id,
  sys_version)` and gets a reassembled `Value` (or `Null` for a deleted version —
  the deleted-check stays in `version_read`, a versioning semantic).

`version_read` in versioning/ thus calls one storage function instead of inlining
`read_nodes`; `dump_load.rs` calls the same one (kills G-S1).

**Seam to `aql/` (register: AQL) — `TODO(w3f-integrate)`.** The AQL engine reads
the `node` table directly (its own SQL over `node`/`vo_version`) and reassembles
whole-object result cells "via `storage::codec` post-fetch" (`aql-engine.md:87`).
It depends on two storage exports: (a) the public `reassemble` codec, and (b) the
**nested-set contract** (`num`/`num_cap` interval-join semantics) + the `Node`
Iden. Candidate: route the AQL whole-object cell reassembly through
`storage::node_repo::read_version_canonical` so `PgRow`→`NodeRow` mapping lives in
one place (currently a potential third copy). Mark the codec + Node Iden as the
stable storage API the AQL register consumes.

**Migrations:** unchanged. `0001_baseline.sql` SETTLED.

---

## 5. PORT-NOTE residue

| PORT NOTE | location | decision |
|---|---|---|
| Demographic containers (`PARTY_IDENTITY`/`CONTACT`/`ADDRESS`/`CAPABILITY`/`PARTY_RELATIONSHIP` nested in a party) stay inline, not split into rows — lossless, fewer rows | `codec.rs:16-28` | **keep** — well-reasoned, cited, round-trip-tested |
| `signature` canonicalization is spec-TBD (§Digital Signature "not yet defined") | `0001_baseline.sql:223` (C8) | **keep** — spec genuinely TBD; re-verify on a spec bump |
| `TEMPLATE_ID` lexical form "to be determined" → free string | (A8) — add to `template_store` comment | **keep** (add note) — no invariant to enforce |
| `item_tag.target_vo_id` intentionally FK-less (tag may target a container OR a VERSION, RM master07) | `0001_baseline.sql:493` | **keep** — cited |
| `vo_archive` FK-less marker; serving reads never join it | `0001_baseline.sql:565` | **keep** — cited |
| `idx_node_magnitude` SPECULATIVE, P20-repriced; AQL generator must emit the matching `openehr_magnitude(data->'value')` fast path | `0001_baseline.sql:400-416` | **keep** as `PERF(port)` — re-verify + EXPLAIN at P20 |
| `creating_system_id` "legacy empty-string sentinel" read-path fallback | `vobject.rs:201` | **drop** (G-S5) — dead in a greenfield build; contradicts the schema's "never an empty-string sentinel" |
| `ehr_index.location` = `LOCATION_DESC` canonical JSON | `0001_baseline.sql:541` | **keep** — SM-3 register, cited |

---

## W-3f closure (2026-07-13)

The node/version SQL moved out of the service layer into `src/storage/` (`codec.rs`, `node_repo.rs`, `version_repo.rs`, `ehr_repo.rs`, `tag_repo.rs`, `structure.rs`, `row.rs`); `db/iden.rs` catalog completed.

| G | Disposition | Evidence |
|---|---|---|
| G-S1 | FIXED in code | `storage/node_repo.rs:110` `read_version_canonical` — single node→canonical reload (the `read_nodes`/`reassemble_version` duplicate is gone) |
| G-S2 | FIXED in code | `storage/node_repo.rs:32` `write_nodes` — node bulk-insert now in `storage/`, not the service file |
| G-S3 | FIXED in code | `db/iden.rs:70-76` (`TrunkVersion`/`BranchNumber`/`BranchVersion`/`PrecedingVersionUid`) + `:99-101` (`ArchEntity`/`ArchConcept`/`ArchMajor`); catalog tests `:291` |
| G-S4 | FIXED in code | duplicate const removed — `storage/structure.rs:32` delegates to BMM-generated `openehr_rm::model::is_structure_root`; agreement test `structure.rs:69,90` |
| G-S5 | FIXED (dropped) | no empty-string sentinel fallback in `storage/version_repo.rs` (dead `vobject.rs:201` path deleted with the file) |
| G-S6 | FIXED in code | lean read path via `storage/node_repo.rs:110` `read_version_canonical` |
| G-S7 | already-correct | static raw SQL permitted; dynamic AQL SQL uses sea-query (no change required) |
| G-A8 | PORT NOTE | `TEMPLATE_ID` free string — `0001_baseline.sql` `template_store` comment |
| G-C8 | PORT NOTE | signature canonicalization spec-TBD — `0001_baseline.sql:223` (C8) |
| G-B1 | Reassigned (quarantine) | `jsonb` numeric re-representation tracked by the ITS canonical-JSON layer (blueprint ch5 §F), not storage |
| G-C13 | Reassigned (register-01) | import retains the wrapped original's `commit_audit` — verified `versioning/import.rs` |

Open residue: none — S-rows fixed in code, storage PORT NOTEs kept, G-B1/G-C13 reassigned to the ITS layer / register-01.
