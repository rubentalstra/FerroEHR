# Conformance register 08 — Demographic component (`suites/demographic.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **Demographic** component of
`tools/conformance`. Method is spec-first (README + owner ruling): the spine
below is the governing CNF schedule chapter enumerated operation-by-operation;
the existing ECC cases are mapped **onto** each schedule item with a `file:line`
verdict (conformant / divergent / missing / instrument-encodes-server-behaviour).
§3 lists ECC cases with no schedule home; §4 carries the G-rows for the rewrite,
marking every edition-/version-specific assertion the version-ladder runner must
know about.

**The governing chapter is a stub.** `master10-func_tc_demographic.adoc` ships
**no concrete test cases** — every one of its 12 SM-operation subsections carries
only placeholder `==== Test Case aaaa` / `bbbb` bodies reading `TBD`, and
`== Test Environment` + `== Test Data Sets` are `TBD` too (26 `TBD` markers
total; blueprint `07-cnf.md` §master10). So the spine below records each
schedule stub **verbatim (cited)** and derives the honest spine from what *does*
exist: the chapter's operation headings (§Test Cases subsection titles), the
profiles capability rows (`master03-profiles.adoc` §Functional → *Demographic
Persistence*), and the existing ECC case universe. Every spine row whose only
backing is a schedule *heading with a TBD body* — not a normative condition — is
flagged **ECC-original (schedule stub)**.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master10-func_tc_demographic.adoc`
  — the DEMOGRAPHIC_SERVICE suite; read whole. §Normative Reference names the
  abstract interfaces `I_DEMOGRAPHIC_SERVICE`, `I_PARTY`, `I_PARTY_RELATIONSHIP`
  and the RM Demographic/Common/Data-Structures/Data-Types/Support IMs +
  Versioning; §Test Cases enumerates the 12 SM operations (all bodies `TBD`).
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form `<SERVICE_COMPONENT>.<operation>-<test-specific id>`
  (§API Conformance Test Design) and the RM-version note (§API Conformance:
  "minimum required version is RM 1.0.2"; supported versions from the
  Conformance Statement).
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` §Functional —
  *Demographic Persistence* (Party Operations, Party Relationship Operations,
  Archetype validation) = **OPTIONS**; §REST APIs — DEMOGRAPHIC API = **OPTIONS**.

**Mapped suite:** `tools/conformance/src/suites/demographic.rs` (24 ECC entries,
`ECC-DEM-001..024`) and the shared `suites/support.rs` helpers.

---

## 1. Verdict

The Demographic suite is a **wire-lifecycle suite our own design**, not a
mapping of master10 — because master10 has nothing to map to (all-TBD). It
exercises the ITS-REST `/demographic/{person,agent,group,organisation,role}`
CRUD contract (201+ETag+Location, 200 get, 200/204 update, 204 delete, 404
absent, 4xx bad `If-Match`) plus `versioned_party`, revision history, and tags.
Coverage of the *ACTOR* subtype create/get/delete matrix is complete and
faithful to the RM Demographic invariants (`PERSON.identities [1..*]` enforced
by `dem/create-bad-body`; `ROLE.Capabilities_valid` respected by the role body).

Three honest gaps, all traceable to the empty schedule: (a) the whole
**`I_PARTY_RELATIONSHIP` operation family** (6 SM operations) has **no ECC
case** — the suite has no relationship coverage at all; (b) two temporal
read operations (`get_party_at_time`, `get_party_relationship_at_time`) are
missing; (c) every ECC case carries `schedule_ref: None` (`demographic.rs:160`),
so nothing threads to a master10 operation id even though the operation headings
exist. And the wire itself is an **ehrbase-rs extension**: the ITS-REST
DEMOGRAPHIC API is DEVELOPMENT status (`docs/VERSIONS.md` §openEHR spec matrix)
and the profiles matrix places it under OPTIONS — for a foreign SUT the entire
suite is fairness-register **N/A** (`adjudications/ehrbase-java-2.34.toml`
`[[area]] area = "DEM" disposition = "extension"`). The rewrite must: thread a
`schedule_ref` per operation, add the relationship + at-time coverage, and source
party bodies from register 80 rather than the inline `actor()`/`role_body()`
literals (RM-1.2.0-pinned).

---

## 2. The spine (master10 operations → ECC map)

Schedule ids use the overview form `I_DEMOGRAPHIC_SERVICE.<operation>-<id>`. The
concrete `<id>` is **TBD** in every subsection (the chapter's `aaaa`/`bbbb`
stubs), recorded verbatim. Data-set classes: master10 §Test Data Sets is `TBD`,
so classes are **derived** (RM Demographic IM shapes) and flagged. Capability /
profile from `master03-profiles.adoc` §Functional — *Demographic Persistence*.
ECC file:line is in `suites/demographic.rs` unless noted.

### `I_DEMOGRAPHIC_SERVICE.create_party()` — Party Operations · OPTIONS

Schedule stub: `==== Test Case aaaa` / `bbbb` — both **`TBD`** (master10 §Test
Cases, *create_party*). Derived normative intent (SM §I_DEMOGRAPHIC_SERVICE +
RM Demographic IM): commit a valid PARTY (ACTOR subtype or ROLE) → positive
creation; a PARTY violating an RM invariant → negative.

| Schedule case (id TBD) | Derived condition | Data sets (derived) | ECC map — verdict |
|---|---|---|---|
| `create_party-<TBD>` (positive, PERSON) | valid PERSON → 201 + ETag + Location | VALID: ACTOR subtype bodies | `ECC-DEM-001` `dem/person-create` (`demographic.rs:265`) — **conformant** (201, `etag`+`location` asserted). Flag: ECC-original (schedule stub). |
| `create_party-<TBD>` (positive, other kinds) | valid AGENT/GROUP/ORGANISATION/ROLE → 201 + ETag | VALID: 4 further kinds | `ECC-DEM-009/012/015/018` `dem/{agent,group,organisation,role}-create` (`demographic.rs:444–447` via `kind_crud!`) — **conformant** (201 + `etag`). Role body respects `ROLE.Capabilities_valid` (absent list, `demographic.rs:216`). |
| `create_party-<TBD>` (negative) | PARTY missing mandatory `identities [1..*]` → reject | INVALID: identity-less PERSON | `ECC-DEM-021` `dem/create-bad-body` (`demographic.rs:451`) — **conformant** ([400,422]); realizes `PERSON.identities` cardinality (RM Demographic IM). |

### `I_DEMOGRAPHIC_SERVICE.get_party()` — Party Operations · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`**. Derived: known versioned-party uid →
retrieve current version; unknown/absent → negative.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `get_party-<TBD>` (existing) | known party uid → 200 | pre: 1 party | `ECC-DEM-002/010/013/016/019` `dem/{person,agent,group,organisation,role}-get` (`demographic.rs:282`, `kind_crud!` gets) — **divergent (shallow)**: asserts only status 200, not that the served body's identity matches the created party. |
| `get_party-<TBD>` (absent) | random uid → 404 | pre: empty | `ECC-DEM-007` `dem/person-get-absent` (`demographic.rs:363`) — **conformant** (404 on random UUID). |
| `get_party-<TBD>` (deleted) | current version of a deleted party → negative | pre: 1 deleted party | `ECC-DEM-006` `dem/person-get-deleted` (`demographic.rs:342`) — **instrument-encodes-server-behaviour**: accepts `[204,404]` (a deleted party's current-version read is server-dependent), a widened set masking two behaviours. |

### `I_DEMOGRAPHIC_SERVICE.get_party_at_time()` — Party Operations · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`**. Derived: retrieve the party version
current at a given time.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `get_party_at_time-<TBD>` | party as of timestamp T → the version live at T | pre: multi-version party | **missing** — no ECC case exercises a time-parameterised party read. G-2. |

### `I_DEMOGRAPHIC_SERVICE.update_party()` — Party Operations · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`**. Derived: update an existing party with a
correct `If-Match` → new version; wrong `If-Match` → negative.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `update_party-<TBD>` (positive) | valid update + matching OVID `If-Match` → 200/204 | pre: 1 party | `ECC-DEM-004` `dem/person-update` (`demographic.rs:310`) — **conformant** (create → PUT with `If-Match: ovid` → [200,204]). |
| `update_party-<TBD>` (negative) | wrong `If-Match` → precondition/version failure | pre: 1 party | `ECC-DEM-008` `dem/person-update-bad-if-match` (`demographic.rs:376`) — **instrument-encodes-server-behaviour**: constructs `If-Match: {vo}::conformance::99` inline (`demographic.rs:385`, assumes our `::conformance::` system id) and accepts `[400,409,412]` — the W-3f ETag lesson (G-1). |

### `I_DEMOGRAPHIC_SERVICE.delete_party()` — Party Operations · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`**. Derived: delete an existing party →
positive; the party's later reads reflect deletion.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `delete_party-<TBD>` | existing party + `If-Match` → 200/204 | pre: 1 party | `ECC-DEM-005/011/014/017/020` `dem/{person,agent,group,organisation,role}-delete` (`demographic.rs:328`, `kind_crud!` dels) — **conformant** ([200,204]). |

### `I_DEMOGRAPHIC_SERVICE.get_party_at_version()` — Party Operations · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`**. Derived: retrieve a specific party
version by OVID.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `get_party_at_version-<TBD>` | known OVID → that exact version, 200 | pre: 1 party | `ECC-DEM-003` `dem/person-get-by-version` (`demographic.rs:296`) — **divergent (shallow)**: GET `/demographic/person/{ovid}` asserts 200 only; does not verify the returned version's `uid` equals the requested OVID. |

### `I_DEMOGRAPHIC_SERVICE.create_party_relationship()` — Party Relationship Operations · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`** (master10 §Test Cases,
*create_party_relationship*; SM `I_PARTY_RELATIONSHIP`).

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `create_party_relationship-<TBD>` | valid PARTY_RELATIONSHIP between two parties → positive | pre: 2 parties | **missing** — the suite has no `PARTY_RELATIONSHIP` case at all. G-3. |

### `I_DEMOGRAPHIC_SERVICE.get_party_relationship()` — Party Relationship Operations · OPTIONS

Schedule stub: **`TBD`**. → **missing** (G-3).

### `I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_time()` — Party Relationship Operations · OPTIONS

Schedule stub: **`TBD`**. → **missing** (G-3).

### `I_DEMOGRAPHIC_SERVICE.update_party_relationship()` — Party Relationship Operations · OPTIONS

Schedule stub: **`TBD`**. → **missing** (G-3).

### `I_DEMOGRAPHIC_SERVICE.delete_party_relationship()` — Party Relationship Operations · OPTIONS

Schedule stub: **`TBD`**. → **missing** (G-3).

### `I_DEMOGRAPHIC_SERVICE.get_party_relationship_at_version()` — Party Relationship Operations · OPTIONS

Schedule stub: **`TBD`**. → **missing** (G-3).

**Schedule coverage:** master10 defines **12 SM operations × 2 TBD stubs = 24
placeholder test cases** (no concrete case). Of the 12 operations: **5 mapped**
(create/get/update/delete/get_at_version_party), **1 mapped-shallow**
(get_at_version), **6 missing** (all `I_PARTY_RELATIONSHIP` +
`get_party_at_time`). Since no operation has a normative body, every mapped row
is **ECC-original (schedule stub)** — the ECC is the *source* of demographic
test substance, not a mapping of it.

---

## 3. Existing ECC cases with no schedule home

Even the operation headings do not cover these — they exercise the ITS-REST
wire surface (versioned resource, tags) that the abstract SM operations do not
name:

| ECC | Suite | Nature | Flag |
|---|---|---|---|
| `ECC-DEM-022` `dem/versioned-party-get` (`demographic.rs:471`) | DEM | `GET /demographic/versioned_party/{vo}` → 200. Realizes the `VERSIONED_PARTY` read (RM Common Versioning), a wire resource with no SM operation in master10. | **Keep, re-home** to a Versioning-derived row; ECC-original extension. |
| `ECC-DEM-023` `dem/versioned-party-revision-history` (`demographic.rs:485`) | DEM | `GET …/versioned_party/{vo}/revision_history` → 200. `REVISION_HISTORY` read; no SM operation. | **Keep**; ECC-original extension (Versioning). |
| `ECC-DEM-024` `dem/person-tags` (`demographic.rs:501`) | DEM | `GET /demographic/person/{vo}/tags` → [200,204]. Item-tag read — an **ehrbase-rs extension** (no openEHR spec governs item tags). | **Keep, flag extension**: no CNF/SM backing at all; ad-hoc wire. |

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (bad-`If-Match` construction — the W-3f ETag lesson). EDITION-SPECIFIC.**
  `dem/person-update-bad-if-match` hand-builds `If-Match: {vo}::conformance::99`
  (`demographic.rs:385`), assuming our `::conformance::` system id, and the
  create-helper splits the OVID at `::` inline (`demographic.rs:253`). The bad
  value and the OVID/ETag extraction must move to the one wire-adapter (register
  90) that records which ETag edition form (`W/"…"` weak vs bare-quoted) matched.
  The `[400,409,412]` acceptance (`demographic.rs:388`) masks three distinct
  server behaviours — record which the SUT returns as an edition finding.

- **G-2 (`get_party_at_time` missing).** No ECC case exercises a
  time-parameterised party read (`get_party_at_time`). The SM operation exists
  (master10 §Test Cases) and RM Common Versioning defines `version_at_time`; the
  rewrite adds a multi-version party fixture and a T-of-interest read.

- **G-3 (the entire `I_PARTY_RELATIONSHIP` family missing).** Six SM operations
  (`create/get/get_at_time/update/delete/get_at_version_party_relationship`,
  master10 §Test Cases) have **zero** ECC coverage. The rewrite must add
  `PARTY_RELATIONSHIP` create/get/update/delete + the two temporal reads against
  the `/demographic/party_relationship` wire, sourced from register-80 bodies.

- **G-4 (RM wire version pinning — no ladder). VERSION-SPECIFIC.** Party bodies
  are RM-1.2.0-shaped Rust literals — `actor()` (`demographic.rs:170`) and
  `role_body()` (`demographic.rs:195`) — and validity is implicit (status only).
  `master03-overview.adoc` §API Conformance sets the minimum at RM 1.0.2 with the
  supported version from the Conformance Statement. The rewrite must express
  PARTY payloads at each supported RM edition, try highest-first, and record the
  satisfied level.

- **G-5 (`schedule_ref` not threaded).** Every DEM entry sets
  `schedule_ref: None` (`demographic.rs:160`), unlike MSG (which threads
  `with_schedule_ref`). Since the master10 operation headings exist (even with
  TBD bodies), the rewrite must thread the SM-operation ref per case
  (`I_DEMOGRAPHIC_SERVICE.create_party (CNF master10, TBD)`), so the report shows
  the derived provenance and the schedule-stub status honestly.

- **G-6 (shallow read assertions).** `*-get` (`ECC-DEM-002` etc.) and
  `get-by-version` (`ECC-DEM-003`) assert status only; the rewrite adds the
  create→read identity round-trip (served `uid`/identity == created), matching the
  depth register 03 mandates for EHR reads.

- **G-7 (fairness N/A for foreign SUTs — the whole suite). EDITION-/VERSION- and
  EXTENSION-SPECIFIC.** The ITS-REST DEMOGRAPHIC API is **DEVELOPMENT** status
  (`docs/VERSIONS.md` §openEHR spec matrix) and OPTIONS in
  `master03-profiles.adoc` §REST APIs; our `/demographic/*` wire is an
  **ehrbase-rs extension**. The seeded adjudication register already rules DEM →
  `extension` for `ehrbase-java` (`adjudications/ehrbase-java-2.34.toml`), i.e.
  NotApplicable, never a failure, for any SUT lacking a demographic REST API. The
  rewrite must keep every DEM row a fairness-register N/A row for foreign SUTs and
  never let its absence dent a CORE/STANDARD verdict (Demographic is OPTIONS-only).

---

*Register 80 owns the party/relationship data-set bodies referenced by
G-3/G-4; register 90 owns the wire-adapter/version-ladder + `schedule_ref`
threading referenced by G-1/G-4/G-5.*
