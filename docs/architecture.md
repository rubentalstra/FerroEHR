# Target Architecture

This is a summary of the system this port produces: what EHRbase is, the
surfaces it exposes, the pipelines behind those surfaces, and how the target
Rust workspace is organized. It distills `PORT_MASTER_PLAN.md` Sections 6–9;
that document is authoritative on any discrepancy.

## What EHRbase is

EHRbase is a Spring Boot openEHR Clinical Data Repository (CDR): a server
that stores, versions, and queries clinical data shaped by openEHR archetypes
and templates, exposed through the standard openEHR REST API plus a set of
EHRbase-specific extensions. The upstream Maven reactor is organized into
modules — `api`, `application`, `aql-engine`, `base`, `bom`, `cli`,
`configuration`, `db_scripts`, `jooq-pg`, `plugin`, `rest-ehr-scape`,
`rest-openehr`, `rm-db-format`, `service`, `test-coverage`/`tests` — which
this port's Phase 0 `git mv` step concentrates into three Rust server crates
plus ten openEHR-spec crates written fresh (see "Workspace layout" below).

## REST surface

Base path `/ehrbase/rest/openehr/v1`, implementing openEHR ITS-REST 1.0.3:

- **EHR** — create/read EHRs, EHR_STATUS.
- **COMPOSITION** — versioned clinical documents.
- **DIRECTORY / FOLDER** — the EHR's folder hierarchy.
- **CONTRIBUTION** — the audit/versioning envelope every write belongs to.
- **QUERY** — ad hoc AQL (`/aql`) and stored queries.
- **DEFINITION** — template management (`/template/adl1.4`, `/template/adl2`,
  the latter mostly `501 Not Implemented` upstream).

EHRbase-specific additions layered on top: the Admin API (`/rest/admin`),
`/rest/status`, `/management/*`, experimental Item Tags, and the legacy
EhrScape API (`/rest/ecis/v1/*`) for FLAT/STRUCTURED payloads.

## The AQL pipeline (the crown jewel)

AQL (Archetype Query Language) is openEHR's query language over archetyped
data. The pipeline, in order:

1. **Parse** — lex + parse AQL text against the QUERY 1.1.0 grammar into an
   AST. Upstream uses an ANTLR grammar; this port reimplements the grammar
   natively (`openehr-aql`, via `logos` + `chumsky`/`winnow`).
2. **Semantic / path analysis** — resolve archetype paths in the query
   against the relevant Web Templates, producing typed, path-resolved query
   nodes.
3. **AST → ASL** — lower the analyzed AST into EHRbase's own intermediate
   representation, the Abstract SQL Layer (ASL). This is the layer that knows
   about the row-per-locatable persistence shape without yet committing to
   literal SQL text.
4. **ASL rewrite / optimize** — rule-based rewrites over the ASL (predicate
   pushdown, redundant-join elimination, and similar).
5. **ASL → SQL** — generate literal SQL: JSONB path extraction, array
   unnesting for repeating structures, and a `current` + `_history` table
   UNION so a query transparently spans live and versioned data. PostgreSQL
   18's `JSON_TABLE()` (inherited from PG 17) is used where it can replace
   hand-rolled JSONB extraction.
6. **Execute** — run the generated SQL against PostgreSQL.
7. **Assemble RESULT_SET** — reshape the flat SQL result rows back into the
   openEHR RESULT_SET wire schema (1.0.3).

This pipeline plus RM transcription plus the persistence bridge (below) make
up roughly 60% of the total port's complexity; see "Port-difficulty map".

## Persistence model (v2 shape)

The v2 schema decomposes each composition **row-per-locatable**: every
LOCATABLE node in a composition tree becomes its own row, with leaf
attributes held as JSONB rather than one row per whole composition. Key
tables:

- `ehr.comp_data` / `ehr.comp_data_history` — the decomposed locatable rows,
  current and historical.
- `ehr.comp_version` — version envelope per composition version.
- `ehr.ehr`, `ehr.ehr_status_data`, `ehr.ehr_folder_data` — EHR-level state.
- `ehr.contribution` — the audit/versioning envelope; every write inserts one.
- `ehr.audit_details` — who/when/why for every write; every write inserts one.
- `ehr.template_store` — ingested OPT templates.
- `ehr.stored_query` — saved AQL queries.
- `ehr.item_tag` — the experimental Item Tags feature.

Versioning uses paired current/`_history` tables driven by triggers: an
update moves the prior row to `_history` before writing the new current row,
so `VERSIONED_OBJECT`/`ORIGINAL_VERSION` semantics are reconstructable from
two tables rather than a single append-only log.

The Rust port keeps the migrations verbatim (`openehr-server/migrations/`)
and replaces jOOQ's generated DSL with `sea-query` + `sqlx` for query
construction and execution. The RM ↔ row-per-locatable bridge itself is
`rm-db-format`, ported into `openehr-server/src/rm_db_format/`.

## Serialization formats

- **Canonical JSON** — the primary wire format, `_type`-discriminated,
  targeting the openEHR ITS-JSON schemas (`openehr-serde`).
- **Canonical XML** — JAXB/XSD-shaped in EHRbase; targets ITS-XML 1.0.2 and
  2.0.0 in this port, for round-trip with both schema generations.
- **FLAT (simSDT) / STRUCTURED (structSDT)** — Better/Marand vendor formats
  being retro-standardized as SDT; implemented in `openehr-flat` against
  Better's `web-template` semantics plus documented EHRbase quirks.
- **Web Template JSON** — the flattened, path-addressable template shape both
  FLAT/STRUCTURED and AQL path analysis are built on.
- A newer Matrix format exists upstream and is lower priority.

## Workspace layout (13 crates)

Crate boundaries mirror openEHR component boundaries: each crate maps ~1:1 to
a spec component and versions independently. Ten crates are openEHR-spec
crates that start empty and are written from the published specifications
(EHRbase itself sourced this surface from the external `archie`/openEHR-SDK
libraries, which are not in this repo). Three crates receive EHRbase's actual
server Java via the Phase 0 `git mv` and are ported in place.

Dependency arrows (downward only — server code depends on spec crates, never
the reverse):

```
openehr-foundation
      │
      ▼
openehr-base ──────────────► openehr-terminology
      │                             │
      ▼                             ▼
      └──────────► openehr-rm ◄─────┘
                       │
        ┌──────────────┼───────────────┐
        ▼              ▼               ▼
  openehr-serde   openehr-odin    (consumed by
        │              │           openehr-adl,
        │              ▼           below)
        │        openehr-bmm
        │              │
        └──────┬───────┘
               ▼
          openehr-adl ──────────────► openehr-flat
               │                            │
               ▼                            │
          openehr-aql                       │
               │                            │
               ▼                            │
          openehr-rest ◄────────────────────┘
               │
               ▼
     openehr-ehrbase-compat
               │
               ▼
         openehr-server   (binary; depends on all of the above)
```

| Crate | Role | Depends on |
|---|---|---|
| `openehr-foundation` | BASE Foundation Types: `Any`, `Interval<T>`, containers, ISO 8601 temporals, functional types | — |
| `openehr-base` | BASE Base Types: definitions, builtins, identification, resource | `openehr-foundation` |
| `openehr-terminology` | TERM 3.x XML bundle + terminology service | `openehr-base` |
| `openehr-rm` | RM 1.1.0: data_types, data_structures, common, ehr, demographic, integration, support | `openehr-base`, `openehr-terminology` |
| `openehr-serde` | Canonical JSON (ITS-JSON) + canonical XML (ITS-XML), `_type` dispatch | `openehr-rm` |
| `openehr-odin` | ODIN parser | — |
| `openehr-bmm` | BMM object model + P_BMM parser (schema v2.3) | `openehr-odin` |
| `openehr-adl` | ADL 1.4 + ADL 2 parsers, AOM 1.4 + AOM 2, OPT 1.4 XML, OPT 2 flattener | `openehr-odin`, `openehr-bmm`, `openehr-rm`, `openehr-serde` |
| `openehr-flat` | FLAT / STRUCTURED / Web Template (Better semantics + EHRbase quirks) | `openehr-rm`, `openehr-serde`, `openehr-adl` |
| `openehr-aql` | AQL 1.1.0 lexer + parser + AST + semantic analyser | `openehr-rm`, `openehr-adl` |
| `openehr-rest` | ITS-REST 1.0.3 server + client (axum). **Receives** EHRbase REST Java. | `openehr-rm`, `openehr-serde`, `openehr-adl`, `openehr-aql` |
| `openehr-ehrbase-compat` | EHRbase-compatible endpoint aliases, admin API, OPT 1.4 ingestion, WebTemplate export, EhrScape. **Receives** EhrScape + admin Java. | `openehr-rest`, `openehr-flat` |
| `openehr-server` | Reference binary: persistence (sqlx + sea-query), AQL execution engine (ASL), versioning, contributions. **Receives** most server Java. | all of the above |

## Port-difficulty map

| Area | Difficulty | Share |
|---|---|---|
| REST controllers, DTOs, admin, security wiring, cache, config, metrics, Swagger, migrations, CLI, item tags, stored queries, AQL DTO model | EASY | ~15% |
| Service orchestration & transactions, jOOQ→sea-query, schema, canonical JSON, OPT XML parsing, versioning, EhrScape, terminology client, AQL grammar parsing | MEDIUM | ~25% |
| RM classes (own transcription), WebTemplate builder, composition validation, canonical XML, FLAT/STRUCTURED, AQL path analysis, result rebuild, rm-db-format | HARD | ~35% |
| AQL planner (AST→ASL), AQL SQL generator (ASL→SQL), full ADL2/AOM2 (deferrable) | VERY HARD | ~25% |

Critical path (~60% of total complexity): RM transcription + the AQL engine
(parse + plan + SQL + result) + rm-db-format + composition validation. Phases
are sequenced so an increasingly capable partial system is usable at each
boundary rather than requiring the whole stack before anything runs.

## Stage sequencing

The whole project in one line:

> **Stage 1:** faithful 1:1 Rust-native port → **Stage 2:** restore
> enterprise features (RBAC and others removed between the v1 and v2 lines)
> → **Stage 3:** improve the codebase.

- **Stage 1** is everything in `PORT_MASTER_PLAN.md` Section 10's phase table
  (P0–P99): transcribe the openEHR spec surface natively, port EHRbase's
  server Java in place, make the workspace compile (P17), and reach ≥99%
  behavioural parity at the REST surface against stock EHRbase (P18).
  Phases P1–P16 are explicitly not required to compile — capturing intent
  correctly is the goal, not a green `cargo build`, until P17.
- **Stage 2** begins only once P18 parity holds. It restores capabilities
  EHRbase removed between its pre-v2 line and v2 — RBAC/access control being
  the highest-priority candidate — based on the archaeology diff produced in
  Phase 0 against the `reference/v1` git ref. Each confirmed item gets its
  own Stage 2 phase file.
- **Stage 3** is idiomatic refactoring, performance work, new capabilities,
  and any upstream-worthy cleanups. It happens only after Stage 2, once the
  codebase is both faithful and feature-complete relative to the pre-v2
  baseline.
