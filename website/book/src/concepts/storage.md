# Storage architecture

How FerroEHR physically stores clinical data. This page goes one level below
the [system architecture](architecture.md): the tables, the write path, the
read paths, and the reasons the layout looks the way it does. If you read the
source, the schema itself is the authority: every column in
`app/ferroehr/migrations/` carries a `COMMENT ON` line with its citation.

One thing up front: **openEHR defines no SQL schema.** What the specs do
define, and what this storage realizes, are the versioning and change-control
semantics (RM `common` change control), canonical data fidelity (ITS-JSON),
and the contribution and audit duties. The relational layout below is
FerroEHR's own PostgreSQL-18-native design, and the RM explicitly sanctions
that freedom: "Although the figure implies physical containment of Versions
by a Versioned object, this is only one possible implementation. Other
implementations (e.g. using orthodox relational structures) might use
references, separate compressed copies, or any other mechanism."

<!-- toc -->

## The big picture

Every versioned object (COMPOSITION, EHR_STATUS, EHR_ACCESS, FOLDER, the
demographic party kinds) is stored twice over, deliberately, in one
transaction:

1. **`vo_version`** holds the version row: identity, version tree position,
   validity interval, lifecycle state, and the canonical JSON body bytes
   served verbatim on point reads.
2. **`node`** holds the same content decomposed: one row per RM structure
   node, carrying a nested-set index and promoted predicate columns, so AQL
   never walks JSON to answer CONTAINS.

```mermaid
flowchart LR
    client[REST client] --> rest["ITS-REST adapter"]
    rest --> svc["service layer<br/>(validation, versioning)"]
    svc --> tx{{"one transaction<br/>per commit"}}
    tx --> audit[(audit)]
    tx --> contrib[(contribution)]
    tx --> vov[(vo_version)]
    tx --> node[(node)]
    vov -. "point read: body bytes verbatim" .-> rest
    node -. "AQL: interval joins + promoted columns" .-> rest
```

The database is PostgreSQL 18, split into four schemas:

| Schema | Holds |
|---|---|
| `ehr` | the CDR proper: versions, nodes, EHRs, contributions, templates, queries, tags |
| `ext` | FerroEHR's own `IMMUTABLE` helper functions (`openehr_magnitude`, `openehr_timestamp`) and the tenant context |
| `audit` | the IHE ATNA Audit Record Repository (`audit_event`) |
| `cold` | the archival tier: FK-free mirrors of `vo_version` / `node` / `vo_attestation` |

## Core tables and how they relate

```mermaid
erDiagram
    ehr ||--o{ contribution : "owns (NULL for demographics)"
    contribution ||--|| audit : "its own audit"
    contribution ||--o{ vo_version : "change set members"
    audit ||--o{ vo_version : "commit_audit"
    vo_version ||--o{ node : "decomposed content (per version)"
    vo_version ||--o{ vo_attestation : "appended attestations"
    template_ref ||--o{ vo_version : "template identity (FK)"
    template_store ||--|| template_ref : "registers"
    ehr ||--o{ ehr_folder : "folder hierarchies (rank order)"
    ehr ||--o{ item_tag : "ITEM_TAGs"

    vo_version {
        uuid vo_id PK
        int sys_version PK "opaque commit ordinal"
        text kind "COMPOSITION | EHR_STATUS | ..."
        uuid ehr_id FK "NULL for demographics"
        int trunk_version "VERSION_TREE_ID part 1"
        int branch_number "0 = trunk"
        int branch_version "0 = trunk"
        tstzrange sys_period "[committed, superseded)"
        text lifecycle_state "532/553/523/800/801"
        text creating_system_id "OBJECT_VERSION_ID middle segment"
        text preceding_version_uid
        text signature "VERSION.signature, 0..1"
        jsonb wrapped_original "IMPORTED_VERSION discriminator"
        text body "canonical JSON bytes, lz4"
    }
    node {
        uuid vo_id PK
        int sys_version PK
        int num PK "pre-order number, root = 0"
        int num_cap "subtree = num..=num_cap"
        int parent_num
        int citem_num "nearest archetyped ancestor"
        text rm_type
        text archetype "case-folded"
        text path "materialized, COLLATE C"
        jsonb data "canonical fragment, children pruned"
        timestamptz context_start "promoted, COMPOSITION root only"
    }
```

Supporting tables not drawn above: `stored_query` (stored AQL, qualified name
plus SemVer), `archetype_store` and `adl2_artefact` (the two DEFINITION
dialects), `ehr_index`, `vo_archive` (the admin archive marker), and the
`sp_*` family (Subject Proxy Service). The `ehr` table itself carries the
three creation-immutable values the RM names (`system_id`, `id`,
`time_created`) plus promoted copies of the current EHR_STATUS subject
reference and `is_queryable` / `is_modifiable` flags, which back the
one-EHR-per-subject rule, the AQL full-population gate, and the content-write
guard without probing a JSON root per request.

## Versioning: one temporal table, no history pairs

Most CDRs split storage into a "current" table and a "_history" table.
FerroEHR does not: `vo_version` is one temporal table, and currency is a
predicate, not a location.

- Every version row carries `sys_period tstzrange`, the half-open validity
  interval `[committed, superseded)`. The current trunk version of an object
  is simply the row with `upper_inf(sys_period) AND branch_number = 0`, held
  unique by a partial index, which is what realizes the RM's
  `latest_trunk_version`.
- `ALL_VERSIONS` is the unfiltered table; `LATEST_VERSION` is that partial
  index. Time travel is a range containment test on `sys_period`.
- The spec-facing version identity is the three-part `OBJECT_VERSION_ID`
  `{object_id, creating_system_id, version_tree_id}`, stored as `vo_id` +
  `creating_system_id` + the `trunk_version`/`branch_number`/`branch_version`
  triple and held unique together. `sys_version` is deliberately not that
  number: it is an opaque per-object commit ordinal (1..n across trunk and
  branch commits) used as the join key for `node` and `vo_attestation`.
- Version keys and generated ids use PostgreSQL 18's native `uuidv7()`, so
  keys are time-ordered and index-friendly.
- A logical delete writes a content-less version with lifecycle state `523`;
  nothing is physically deleted.
- An import (EHR-Extract, archive load) stores the wrapped
  `ORIGINAL_VERSION`'s own provenance verbatim in `wrapped_original`, while
  the row's own contribution and audit columns record the local act of
  committal. `NULL` there means a locally created `ORIGINAL_VERSION`;
  `NOT NULL` means the row is an `IMPORTED_VERSION`.

Non-overlap per lineage (one valid version per lineage at any instant) is
enforced by construction rather than by GiST exclusion constraints, which
were measured to serialize concurrent inserts: at most one open row per
lineage exists (the partial unique indexes), and every write closes the open
row and inserts its successor at the same `now()` inside one transaction, so
half-open ranges meet exactly.

```mermaid
flowchart TD
    subgraph one_object ["one versioned object (vo_id)"]
        v1["sys_version 1<br/>1.0.0 (trunk)<br/>sys_period [t1, t2)"]
        v2["sys_version 2<br/>2.0.0 (trunk)<br/>sys_period [t2, t3)"]
        v3["sys_version 4<br/>3.0.0 (trunk, CURRENT)<br/>sys_period [t3, ∞)"]
        b1["sys_version 3<br/>2.1.1 (branch, open)<br/>sys_period [t2b, ∞)"]
        v1 --> v2 --> v3
        v2 -.->|branch 1| b1
    end
```

## Content decomposition: the `node` table

At commit, the accepted composition is decomposed into one row per RM
structure node. Each row stores the node's **canonical openEHR JSON fragment
verbatim** (the ITS-JSON encoding) with its structure children pruned out: no
alias compaction, no synthetic fields, so what sits in `node.data` is
byte-identical in shape to what the API serves. Storage equals wire.

The tree shape is captured as a **nested-set interval**: nodes are numbered
in pre-order (`num`, root = 0), and each row records the maximum number in
its subtree (`num_cap`). "B is contained in A" is then the integer test
`A.num < B.num AND B.num <= A.num_cap`, which makes AQL CONTAINS chains
plain integer range joins instead of JSON tree walks.

```mermaid
flowchart TD
    c["COMPOSITION<br/>num 0, cap 5"] --> s["SECTION<br/>num 1, cap 5"]
    s --> o1["OBSERVATION<br/>num 2, cap 3"]
    o1 --> e1["ELEMENT<br/>num 3, cap 3"]
    s --> o2["EVALUATION<br/>num 4, cap 5"]
    o2 --> e2["ELEMENT<br/>num 5, cap 5"]
```

For the tree above, "OBSERVATIONs inside the SECTION" is
`section.num (1) < obs.num AND obs.num <= section.num_cap (5)`: rows 2 and 4
qualify by arithmetic alone.

Beside the interval, each row promotes the predicates AQL actually filters
on, so hot paths never open the JSON:

- `rm_type` (full RM type names, never compacted), `name`, `archetype`
  (case-folded at write, because openEHR identifier equality is
  case-insensitive);
- the archetype-subsumption columns `arch_entity` / `arch_concept` /
  `arch_major`, parsed from full archetype HRIDs so a query naming a parent
  archetype matches specialised children through an indexed prefix scan (the
  major-version boundary stays hard, as the AM requires);
- `citem_num`, the nearest archetyped ancestor, for archetype-anchored path
  resolution;
- `context_start`, the promoted `EVENT_CONTEXT.start_time` on COMPOSITION
  roots, serving dashboard ordering from a partial index;
- `path`, the materialized path from the root (`COLLATE "C"`, so byte order
  equals tree order), used only for reassembly, never as an AQL predicate.

## The write path: one transaction per commit

Every write realizes the openEHR contribution rule: a `CONTRIBUTION` lists
the affected `VERSION`s and carries its own audit, and it commits only if
every member commits. In storage terms, one transaction per service-level
write:

```mermaid
sequenceDiagram
    participant R as REST adapter
    participant S as service layer
    participant PG as PostgreSQL 18

    R->>S: commit (COMPOSITION, EHR_STATUS, ...)
    S->>S: validate (RM invariants, WebTemplate, terminology)
    S->>PG: BEGIN
    S->>PG: advisory lock on vo_id (serializes the lineage)
    S->>PG: INSERT audit (change_type, committer, time_committed = now())
    S->>PG: INSERT contribution (audit_id, ehr_id)
    S->>PG: UPDATE vo_version SET sys_period = [.., now()) on the open row
    S->>PG: INSERT vo_version (new tip, sys_period = [now(), ∞), body bytes)
    S->>PG: INSERT node rows (decomposed fragments, nested-set numbers)
    S->>PG: COMMIT
    S-->>R: OBJECT_VERSION_ID of the new version
```

Details that matter:

- `time_committed` is always server-computed, never client-supplied; the RM
  requires the committal time to reflect the EHR server's own clock.
- The close-out UPDATE and the successor INSERT use the same `now()`, which
  is what makes the half-open intervals meet with no gap and no overlap.
- The body bytes in `vo_version.body` are materialized from the accepted,
  uid-stamped value **before** decomposition, stored as `text` (not `jsonb`,
  which would re-order keys) so a point read serves the canonical
  `_type`-first field order verbatim.

## Read paths

**Point reads** (GET composition, EHR_STATUS, a named version) resolve the
version row and serve `vo_version.body` verbatim: one detoast, no
re-aggregation, zero translation between storage and wire.

**AQL** never touches the body. The engine plans over `node`: CONTAINS
chains become nested-set interval joins, class and archetype predicates hit
the promoted columns and their indexes, and leaf values are extracted from
the canonical fragments with `jsonb_path_query_first`, jsonpath item
methods, and `ext.openehr_magnitude` (the `IMMUTABLE` helper realizing
DV_ORDERED ordering semantics). `JSON_TABLE` serves array unnesting. The
whole pipeline has [its own page](aql-engine.md).

**Time travel** (a version at a point in time) is a `sys_period @>
timestamptz` containment test on the same one table.

## The cold archival tier

Admin-archived objects move physically out of the primary tables into the
`cold` schema (FK-free mirror relations of `vo_version`, `node`,
`vo_attestation`), transactionally and reversibly. The consequences are
deliberate and visible:

- point reads retry cold only on a primary miss;
- whole-repository readers (exports, dumps) use the union views;
- **AQL stays primary-only**: archived content leaves the queryable store
  until restored;
- a write to an archived object thaws it back to the primary tier first.

```mermaid
flowchart LR
    subgraph primary ["ehr schema (hot)"]
        pv[(vo_version)]
        pn[(node)]
    end
    subgraph coldtier ["cold schema (archive)"]
        cv[(cold.vo_version)]
        cn[(cold.node)]
    end
    pv -- "admin archive (transactional move)" --> cv
    cv -- "restore / thaw-on-write" --> pv
    pv -. "AQL reads primary only" .-> aql[AQL engine]
    pv & cv -. "union views" .-> dump[whole-repo readers]
```

No openEHR spec governs archival tiers; this is FerroEHR's own design.

## Why this design

The shape follows measured PostgreSQL physics rather than habit:

- JSONB has no partial detoast: a big single-document design pays
  whole-document decompression for every leaf access. Decomposed fragments
  average a few hundred bytes, stay under TOAST, and each read touches only
  the rows it needs.
- GIN indexes serve neither ranges nor ordering, so CONTAINS and ORDER BY
  ride integers and promoted btree columns instead.
- PostgreSQL 18's temporal machinery (`tstzrange`, partial unique indexes,
  `uuidv7()`, `RETURNING OLD/NEW`) makes the single temporal version table
  cheaper than current/history pairs, with `ALL_VERSIONS` a plain scan of
  one relation.

The performance this buys is measured, not asserted: see
[Performance](../performance.md) for the earned deployment classes and the
committed measurement records behind them.
