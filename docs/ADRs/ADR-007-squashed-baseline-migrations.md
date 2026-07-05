# ADR-007: Squashed sqlx baseline migrations, verified against the EHRbase Flyway chain

- **Status:** accepted
- **Date:** 2026-07-05
- **Amends:** ADR-006 §4 ("the 41 Flyway SQL migrations … are the schema, run
  via `sqlx migrate`") — the *schema* is still EHRbase's v2 schema verbatim;
  the *files* that create it are now our own clean baselines.
- **Phase:** P09 (persistence foundation).

## Context

ADR-006 vendored EHRbase v2.33.0's 41 Flyway migrations
(`ehr/V1..V27` + `ext/V1..V4`) to be run via `sqlx migrate`. Two problems
surfaced when P09 implemented that:

1. **Flyway artifacts don't fit sqlx.** `V5_1__x.sql` version names don't
   parse under `sqlx::migrate!`; `beforeValidate.sql` callbacks only patch
   `flyway_schema_history` checksums (no-ops on a fresh database); one `.conf`
   sidecar (`executeInTransaction=false`) has a different sqlx spelling
   (`-- no-transaction`); nothing creates the `ehr`/`ext` schemas or the
   extensions (Flyway provisioning did that externally).
2. **The chain replays another project's history.** A fresh install creates
   the `tenant` and `system` tables, enables row-level security, and then
   drops all of it again (the V5.x multi-tenancy removal, V11, V13, V25
   history-table merge). That history belongs to EHRbase's evolution, not to
   this project: no ehrbase-rs database exists at any intermediate version,
   so a migration tracker replaying those steps tracks nothing real.

## Decision

**Ship one clean baseline migration per schema, squashed from the legacy
chain's end state, and prove equivalence with an executable equality gate.**

1. **Baselines** — `crates/ehrbase/migrations/{ext,ehr}/0001_baseline.sql`,
   created with the official CLI (`sqlx migrate add --sequential baseline`).
   Content was derived by applying the full legacy chain to PostgreSQL 18 and
   dumping the result (`pg_dump --schema-only`), then organized for
   readability. Everything schema-relevant is reproduced exactly: 17 tables
   with column order, types, defaults, named NOT NULL/PK/FK constraints, all
   21 indexes, 3 enum types, collations (`en_US` ICU copy in `ext`,
   `COLLATE "C"` on `entity_idx`), storage options
   (`toast_tuple_target=128`, `ov_data` STORAGE MAIN), the `ext` AQL
   aggregate functions (`max/min/sum/avg(jsonb)`, `*_dv_ordered`), and table
   comments.
2. **Runner** (`ehrbase::db::run_migrations`) — bootstraps the `ext`/`ehr`
   schemas + extensions (`uuid-ossp`, `pgcrypto`, `pg_trgm`), then runs the
   `ext` migrator and the `ehr` migrator on a connection whose `search_path`
   leads with the target schema, giving each set its own `_sqlx_migrations`
   table (mirroring Flyway's two `flyway_schema_history` tables). `ext` runs
   first because `ehr` DDL references its collation.
3. **Equality gate** — the original 40-file chain is preserved verbatim under
   `crates/ehrbase/tests/resources/legacy_schema/` (only renamed to sortable
   numbers; V15's `.conf` folded into a leading comment). The integration
   test `baseline_schema_is_identical_to_legacy_flyway_chain` applies the
   legacy chain (psql inside the PG 18 testcontainer) and the baseline to two
   databases and asserts a pg_catalog fingerprint — columns (dense-ranked
   order), constraints (`pg_get_constraintdef`), indexes (`indexdef`),
   functions, aggregates, enum labels, collations, sequences, reloptions,
   attstorage, comments — is identical.
4. **Documented deviations** (encoded in the gate):
   - the orphaned `tenant_id_seq` (referenced by nothing after the V5.x
     multi-tenancy removal) is not recreated;
   - column attnum *gaps* left by legacy `DROP COLUMN`s are not reproduced —
     relative column order is compared instead;
   - sqlx's `_sqlx_migrations` tables are excluded from comparison.
5. **Future evolution** starts at `0002_…` via `sqlx migrate add`, written as
   modern PostgreSQL 18 SQL (`uuidv7()`, temporal constraints,
   `RETURNING OLD/NEW` where the plans call for them). When upstream EHRbase
   ships a new Flyway migration, we translate it into our next sqlx migration
   and extend the legacy fixture + gate.

## Consequences

- **Easier:** fresh installs run 2 migrations instead of 40 with no
  create-then-drop churn; the tracker records *this* project's history from a
  truthful, verified starting point; sqlx conventions are followed exactly.
- **Preserved:** EHRbase's schema history remains in-tree, byte-for-byte, as
  an *executable* fixture — the gate re-proves equivalence on every CI run,
  which is stronger provenance than shipping the replay.
- **Cost:** upstream schema changes need manual translation into our
  migration line (this was equally true under the rename approach, since
  Flyway names never parsed under sqlx anyway).

## Alternatives considered

- **Rename the Flyway files and ship the full chain** (P09's first
  implementation). Works (it is now the test fixture), but replays foreign
  history on every install and pollutes our tracker with 40 steps no deployed
  database ever sat between.
- **"Modernize" the historical steps for PG 18.** Rejected: rewriting history
  falsifies provenance *and* isn't clean SQL either; the DDL itself is
  version-neutral, so there was nothing to modernize.
- **Hand-author a fresh schema.** Rejected (ADR-006): the schema must be
  EHRbase-identical for the AQL engine and parity harness; deriving from the
  applied chain plus a machine-checked equality gate removes the transcription
  risk.
