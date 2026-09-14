# Research report 1: PostgreSQL 18 facts for the storage redesign (#3337)

Method: first-hand read of `docs/architecture.md` §Storage, `docs/postgres-features.md`, `website/book/src/concepts/storage.md`, all 25 migration files under `app/ferroehr/migrations/`, the write path (`app/ferroehr/src/storage/version_repo/commit.rs`, `import.rs`, `placement.rs`, `node_repo.rs`), the AQL emitter (`app/ferroehr/src/aql/sql/*.rs`), the outbox drainer (`app/ferroehr/src/extensions/events/publisher.rs`), the FHIR cursor (`app/ferroehr/src/extensions/fhir/outbound.rs`) and `app/ferroehr/src/db/mod.rs`. Every PostgreSQL claim is quoted from `https://www.postgresql.org/docs/18/`. Where a page is silent the report says so. Read 2026-09-13.

## What the current schema actually does

- `vo_version`: `body text COMPRESSION lz4` (`ehr/0001_baseline.sql:423`); `WITH (fillfactor = 90)` (`:472`); four partial indexes whose predicate is `upper_inf(sys_period)` (`uq_vo_version_current :476`, `uq_vo_version_branch_current :478`, `idx_vo_version_current_ehr :507`, `idx_vo_version_current_template :509`); plus `pk_vo_version`, `uq_vo_version_tree`, `uq_vo_version_trunk_position`, `idx_vo_version_ehr`, `idx_vo_version_contribution`, `idx_vo_version_audit`, `idx_vo_version_template` (11 indexes total).
- Every supersession runs `UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), $22, '[)') WHERE vo_id = $8 AND sys_version = $23 AND upper_inf(sys_period)` inside one CTE with the audit, contribution, version and node inserts (`commit.rs:307-330`, `:399-417`; also `import.rs:439-449`).
- `node.data jsonb COMPRESSION lz4`, "fragments average ~360 B" (`0001:22, :611`); no GIN index (removed, `:660-666`); node rows are inserted via `unnest(...)` arrays (`node_repo.rs:109-116`), not `COPY`.
- Per-vo serialisation: `pg_advisory_xact_lock(hashtextextended(vo_id::text, 0))` (`commit.rs:69`).
- AQL emits `jsonb_path_query_first`, `jsonb_path_query_array`, `jsonb_path_query` as a lateral set-returning function, `#>> '{}'`, `ext.openehr_magnitude`, `ext.openehr_timestamp`, and integer `BETWEEN num AND num_cap` joins (`sql/expr.rs:52-71`, `sql/value.rs:152, 526, 535, 748`, `sql/from.rs:107-126`). It emits no `JSON_TABLE`, no jsonb `@>`, no `$.**`, no jsonpath datetime methods.
- RLS: `ENABLE + FORCE ROW LEVEL SECURITY` with policy `tenant_id = ext.current_tenant_id()` on 17 clinical tables (`0004:82-110`), the cold mirrors (`0007:100-112`), demographic and linkage tables; `ext.current_tenant_id()` is `LANGUAGE sql STABLE` reading a GUC (`ext/0002:27-33`).
- Cold tier: `CREATE TABLE cold.x (LIKE x INCLUDING …)` mirrors, FK-free, moved with `WITH cv AS (DELETE … RETURNING *) INSERT … SELECT *` (`0007:46-71`, `placement.rs:111-123`).
- Outbox: `seq bigint GENERATED ALWAYS AS IDENTITY`, drained with `SELECT … WHERE published_at IS NULL ORDER BY seq LIMIT $1 FOR UPDATE SKIP LOCKED` (`publisher.rs:335-336`); the FHIR emitter reads `WHERE seq > last_seq ORDER BY seq` (`outbound.rs:240`).
- Only server tuning shipped: `shared_preload_libraries=pg_stat_statements` (`docker-compose.yml:122`). No `io_method`, `default_toast_compression`, autovacuum, or `wal_level` settings anywhere in compose/Helm.

## 1. JSONB storage, TOAST, compression, normalisation

**1.1 TOAST fires only above 2 kB per row.** `storage-toast.html` §66.2.2: "The TOAST management code is triggered only when a row value to be stored in a table is wider than TOAST_TUPLE_THRESHOLD bytes (normally 2 kB). The TOAST code will compress and/or move field values out-of-line until the row value is shorter than TOAST_TUPLE_TARGET bytes (also normally 2 kB, adjustable) or no more gains can be had." Consequence: a ~360 B `node.data` row never reaches the TOAST code, so `COMPRESSION lz4` on `node.data` is inert for the typical row; it only matters for the rare large leaf. `toast_tuple_target` is a per-table knob (default "2040 bytes").

**1.2 Storage strategies and what "partial fetch" covers.** Same page §66.2.1: "EXTENDED allows both compression and out-of-line storage. This is the default for most TOAST-able data types." "EXTERNAL ... will make substring operations on wide text and bytea columns faster ... because these operations are optimized to fetch only the required parts of the out-of-line value when it is not compressed." Partial fetch is documented ONLY for `text`/`bytea` substring ops on uncompressed EXTERNAL values. The docs say nothing about jsonb partial detoast; the architecture's "JSONB has no partial detoast" is consistent with the docs' silence. A whole-document design pays a full decompress per leaf access; the decomposed `node` design is the correct response.

**1.3 Big values are pulled late.** "The big values of TOASTed attributes will only be pulled out (if selected at all) at the time the result set is sent to the client." Meta-only scans over `vo_version` do not pay for `body`; keep `body` out of every predicate and out of `SELECT *` on hot paths.

**1.4 Compression method defaults to pglz; lz4 is build-dependent.** `runtime-config-client.html` §default_toast_compression: "The supported compression methods are pglz and (if PostgreSQL was compiled with --with-lz4) lz4. The default is pglz." Every `COMPRESSION lz4` column (`0001:153, 157, 172, 400, 423, 611`) is a hard dependency on an lz4-enabled build. `ALTER TABLE ... SET COMPRESSION` "does not cause the table to be rewritten".

**1.5 jsonb normalises; json/text do not.** `datatype-json.html` §8.14: "jsonb does not preserve white space, does not preserve the order of object keys, and does not keep duplicate object keys." "numbers entered with E notation will be printed without it ... jsonb will preserve trailing fractional zeroes." "The json data type stores an exact copy of the input text." Table 8.23: JSON string → text, number → numeric. jsonb has no datetime type; a DV_DATE_TIME value is a JSON string preserved character-for-character. Key order is lost and `1.0e2` becomes `100`, so a canonical body served verbatim cannot be jsonb; `text` is right (the decision at `0001:412-423` is doc-grounded).

**1.6 Row-level locking scope of a large document.** §8.14.2: "any update acquires a row-level lock on the whole row. Consider limiting JSON documents to a manageable size ... Ideally, JSON documents should each represent an atomic datum." Argues against consolidating `node` rows back into one row per version.

## 2. HOT updates and the close-out UPDATE

**2.1 HOT conditions.** `storage-hot.html` §66.7: "The update does not modify any columns referenced by the table's indexes, not including summarizing indexes. The only summarizing index method in the core PostgreSQL distribution is BRIN." and "There is sufficient free space on the page containing the old row for the updated row." The close-out UPDATE can never be HOT: it modifies `sys_period`, which is referenced by the predicates of four partial indexes. Every supersession writes a new heap tuple, inserts a new index entry in every one of the 11 indexes, and requires VACUUM to remove the old entries. `fillfactor = 90` only places the new tuple on the same page.

**2.2 fillfactor semantics.** `sql-createtable.html` §Storage Parameters: "This gives UPDATE a chance to place the updated copy of a row on the same page as the original ... and makes heap-only tuple updates more likely. For a table whose entries are never updated, complete packing is the best choice." Three designs compare: (A) the current temporal table: non-HOT update plus 11 index inserts per supersession; (B) append-only versions with a separate one-row-per-object current pointer table: the version table becomes never-updated (fillfactor 100, no dead tuples, BRIN-able), and the pointer table's UPDATE touches only a non-indexed column, so it IS HOT-eligible; (C) `is_current boolean` on the version row indexed partially: the same non-HOT problem as (A). Only (B) makes the hot write path HOT.

**2.3 BRIN is exempt from the HOT rule.** A BRIN index on an append-only, time-correlated column does not disturb HOT.

## 3. Temporal keys, WITHOUT OVERLAPS, exclusion constraints

**3.1 WITHOUT OVERLAPS is an EXCLUDE constraint with a GiST index.** `sql-createtable.html` §UNIQUE/PRIMARY KEY: "such a constraint is enforced with an EXCLUDE constraint rather than a UNIQUE constraint. So for example UNIQUE (id, valid_at WITHOUT OVERLAPS) behaves like EXCLUDE USING GIST (id WITH =, valid_at WITH &&). ... you can use other types by adding the btree_gist extension (which is the expected way to use this feature)." A temporal PK on a version table forces the PK onto GiST, which cannot be the arbiter for `ON CONFLICT DO UPDATE`.

**3.2 Equality-only exclusion is documented as slower than UNIQUE; btree_gist does not outperform btree.** `sql-createtable.html` §EXCLUDE: "If all of the specified operators test for equality, this is equivalent to a UNIQUE constraint, although an ordinary unique constraint will be faster." `btree-gist.html`: "In general, these operator classes will not outperform the equivalent standard B-tree index methods, and they lack one major feature of the standard B-tree code: the ability to enforce uniqueness."

**3.3 The docs do NOT say exclusion-constraint inserts serialise.** `ddl-constraints.html` §5.5.6 is one paragraph plus "Adding an exclusion constraint will automatically create an index of the type specified in the constraint declaration." The word "exclusion" does not appear on `index-unique-checks.html`; the EXCLUDE parameter text has no sentence about concurrency, waiting, locking or serialisation. The schema comment at `ehr/0001_baseline.sql:450-453` ("GiST exclusion inserts serialize under concurrency (PostgreSQL 18 docs, 'Exclusion Constraints')"), `linkage/0001_baseline.sql:107-109`, `docs/postgres-features.md` and `storage.md` cite the docs for a claim the docs do not make. The docs on unique indexes say: "If a conflicting row has been inserted by an as-yet-uncommitted transaction, the would-be inserter must wait to see if that transaction commits." Whether the same applies to exclusion checks: the docs do not say.

**3.4 Partial unique btrees hold non-overlap only with an external serialiser.** `indexes-partial.html` §11.8: "the predicate condition must exactly match part of the query's WHERE condition or the index will not be recognized as usable. Matching takes place at query planning time." `uq_vo_version_current` guarantees at most one open row per lineage; it does not guarantee closed ranges never overlap. Non-overlap rests on the advisory lock plus same-`now()` close-then-insert. A redesign that drops the lock must bring back a database-enforced check.

**3.5 Temporal foreign keys are NO ACTION only.** "RESTRICT, CASCADE, SET NULL, and SET DEFAULT are not supported in temporal foreign keys. Only NO ACTION is supported."

## 4. uuidv7 and key locality

`functions-uuid.html` Table 9.45: "Generates a version 7 (time-ordered) UUID. The timestamp is computed using UNIX timestamp with millisecond precision + sub-millisecond timestamp + random. The optional parameter shift will shift the computed timestamp by the given interval." Right-leaning btree appends hold for `audit.id`, `contribution.id`, `vo_attestation.id`, `item_tag.id`. They do NOT hold for `pk_vo_version (vo_id, sys_version)` or `pk_node`: a new version of an old object inserts into the middle of the key space, and client-supplied `vo_id`s need not be v7. `uuidv7(shift)` lets an import mint ids ordered by the imported commit time. The docs are silent on uuid vs bigint cost.

## 5. Declarative partitioning

**5.1 Keys must contain the partition key.** `ddl-partitioning.html` §5.12.2.3: "To create a unique or primary key constraint on a partitioned table, the partition keys must not include any expressions or function calls and the constraint's columns must include all of the partition key columns." "Similarly an exclusion constraint must include all the partition key columns." Partitioning `vo_version` by `hash(vo_id)` keeps every present unique constraint valid. Partitioning by `ehr_id` invalidates all of them (they lack `ehr_id`, and `ehr_id` is NULL for demographic rows). Partitioning by time is impossible for the PK because the key would need an expression. Only `hash(vo_id)`, or `(tenant_id, …)` after adding `tenant_id` to every key, is compatible with the present constraint set. The same holds for `node`. (A LIST key such as `tier` is compatible only if added to every unique key; see 5.8.)

**5.2 Pruning happens at plan and execution time.** §5.12.4: "Partition pruning can be performed not only during the planning of a given query, but also during its execution ... for example, parameters defined in a PREPARE statement, using a value obtained from a subquery, or using a parameterized value on the inner side of a nested loop join." The RLS qual `tenant_id = ext.current_tenant_id()` is a STABLE call, so a `tenant_id` hash partition prunes at executor start.

**5.3 Partitionwise join is off by default and expensive.** `runtime-config-query.html`: "Partitionwise join currently applies only when the join conditions include all the partition keys ... Query planning also becomes significantly more expensive in terms of memory and CPU. The default value is off."

**5.4 Partition count and planning cost.** §5.12.6: "The query planner is generally able to handle partition hierarchies with up to a few thousand partitions fairly well, provided that typical queries allow the query planner to prune all but a small number of partitions. ... each partition requires its metadata to be loaded into the local memory of each session that touches it." Full-population AQL touches every partition of `node`; keep counts small (tens), never per tenant or per EHR.

**5.5 Indexes propagate; concurrent builds do not.** "Concurrent builds for indexes on partitioned tables are currently not supported. However, you may concurrently build the index on each partition individually and then finally create the partitioned index non-concurrently ... building the partitioned index is a metadata only operation."

**5.6 DETACH CONCURRENTLY constraints.** `sql-altertable.html`: "During the first transaction, a SHARE UPDATE EXCLUSIVE lock is taken on both parent table and partition, and the partition is marked as undergoing detach; at that point, the transaction is committed and all other transactions using the partitioned table are waited for. ... CONCURRENTLY cannot be run in a transaction block and is not allowed if the partitioned table contains a default partition." Cannot run through the transactional sqlx migrator.

**5.7 ATTACH scan avoidance.** "It is possible to avoid this scan by adding a valid CHECK constraint to the table that allows only rows satisfying the desired partition constraint before running this command."

**5.8 Bulk moves and tiering are the documented partition use case.** §5.12.1: "Seldom-used data can be migrated to cheaper and slower storage media." "Dropping an individual partition using DROP TABLE, or doing ALTER TABLE DETACH PARTITION, is far faster than a bulk operation. These commands also entirely avoid the VACUUM overhead caused by a bulk DELETE." The cold tier today is a row copy (`DELETE … RETURNING` + `INSERT`), the "bulk operation" the docs contrast against, leaving dead tuples for VACUUM. A partition-based tier is faster only if the archival unit aligns with a partition key. Per-object archive does not align with `hash(vo_id)`; an `is_archived` LIST key would have to be in every unique key and an UPDATE that changes the partition key is a DELETE+INSERT (row movement). The docs therefore do not make partition-tiering a drop-in replacement for the per-object mirror; they make it attractive for time-bucketed retention and for keeping one relation set with FKs and RLS.

**5.9 Foreign keys and partitioned tables.** The PG18 limitations list no longer names foreign keys; the pages read neither prohibit nor explicitly permit a FK to or from a partitioned table. Verify on the `sql-createtable` REFERENCES paragraph before relying on it. `BEFORE ROW` triggers "cannot change which partition is the final destination".

## 6. Tablespaces as an archival mechanism

`manage-ag-tablespaces.html` §22.6: "a table storing archived data which is rarely used or not performance critical could be stored on a less expensive, slower disk system." "tablespaces are an integral part of the database cluster and cannot be treated as an autonomous collection of data files ... cannot be attached to a different database cluster or backed up individually." `sql-altertable.html` §SET TABLESPACE: "Indexes on the table, if any, are not moved. ... When applied to a partitioned table, nothing is moved, but any partitions created afterwards ... will use that tablespace." A tablespace lowers cost per byte, never the backup or WAL footprint. Per-object archival cannot use it; per-partition archival can. The current `cold` mirror is a same-tablespace copy: it sheds indexes but not storage cost or backup size.

## 7. Index features

**7.1 B-tree skip scan.** `indexes-multicolumn.html` §11.3: "This approach is generally only taken when there are so few distinct x values that the planner expects the scan to skip over most of the index ... If there are many distinct x values, then the entire index will have to be scanned." True only for low-cardinality leading columns (`kind`, `rm_type`, `lifecycle_state`), never for `ehr_id` or `vo_id`.

**7.2 Index-only scans depend on the visibility map.** `indexes-index-only-scans.html`: "If it's not set, the heap entry must be visited ... it will be a win only if a significant fraction of the table's heap pages have their all-visible map bits set." Freshly written pages are not all-visible until VACUUM; covering indexes pay off on append-only `node` more than on churned `vo_version`.

**7.3 INCLUDE columns.** `sql-createindex.html`: "A non-key column cannot be used in an index scan search qualification, and it is disregarded for purposes of any uniqueness or exclusion constraint ... B-tree deduplication is never used with indexes that have a non-key column." Never include `body`.

**7.4 BRIN.** `brin.html`: "BRIN is designed for handling very large tables in which certain columns have some natural correlation with their physical location within the table." "Because a BRIN index is very small, scanning the index adds little overhead compared to a sequential scan." "If the table is vacuumed ... all existing unsummarized page ranges are summarized." Candidates: `audit.time_committed`, the outbox timestamps, an append-only version table's commit time.

**7.5 GIN jsonb_ops vs jsonb_path_ops.** `datatype-json.html` §8.14.4: "The default GIN operator class for jsonb supports queries with the key-exists operators ?, ?| and ?&, the containment operator @>, and the jsonpath match operators @? and @@. The non-default GIN operator class jsonb_path_ops does not support the key-exists operators, but it does support @>, @? and @@." "A jsonb_path_ops index is usually much smaller than a jsonb_ops index over the same data, and the specificity of searches is better." GIN answers existence/containment only; no range or ordering support. The emitter emits `jsonb_path_query*` functions, never `@?`/`@@`/`@>`, so no GIN index can be used today. If a jsonb pre-filter is ever wanted, the emitter must switch to the operators and the index should be `jsonb_path_ops`.

**7.6 Expression indexes need IMMUTABLE.** `xfunc-volatility.html`: "A common error is to label a function IMMUTABLE when its results depend on a configuration parameter ... a function that manipulates timestamps might well have results that depend on the TimeZone setting. For safety, such functions should be labeled STABLE instead." `ext.openehr_timestamp` is STABLE (correct); `openehr_magnitude` is IMMUTABLE and may back expression indexes, but the emitter must produce the identical expression text.

**7.7 Partial indexes and parameters.** "a prepared query with a parameter might specify 'x < ?' which will never imply 'x < 2' for all possible values of the parameter." A pointer table removes the function-call predicate and makes plain equality the fast path.

## 8. Generated columns

`ddl-generated-columns.html` §5.4 and `sql-createtable.html`: "A generated column is by default of the virtual kind." "The generation expression can only use immutable functions." "the generation expression of a virtual generated column must not reference user-defined functions or types, that is, it can only use built-in functions or types. ... (This restriction does not exist for stored generated columns.)" "A generated column cannot be part of a partition key." Replication "is currently only supported for stored generated columns." A VIRTUAL promoted column cannot call `ext.*`; a STORED one may call IMMUTABLE `openehr_magnitude` but not STABLE `openehr_timestamp`. Whether virtual generated columns can be indexed: the docs read do not say. `rm_type`, `archetype`, `name` could be STORED GENERATED from `data`, making write-path drift impossible; whether that beats unnest-time population is a measurement.

## 9. SQL/JSON

**9.1** `functions-json.html`: "The rows produced by JSON_TABLE are laterally joined to the row that generated them." "JSON_VALUE ... Only use JSON_VALUE() if the extracted value is expected to be a single SQL/JSON scalar item; getting multiple values will be treated as an error." "JSON_EXISTS ... The default when no ON ERROR clause is specified is to return the boolean value FALSE." The emitter uses `jsonb_path_query` as a lateral SRF, the function-form equivalent of `JSON_TABLE`. `JSON_VALUE(... RETURNING numeric)` would give typed extraction with `ON ERROR` control.

**9.2 jsonpath datetime methods and TimeZone.** Tables 9.51/9.52: "all but the first of these conversions depend on the current TimeZone setting, and thus can only be performed within timezone-aware jsonpath functions." "jsonb_path_exists_tz, ... jsonb_path_query_first_tz ... these functions are marked as stable, which means these functions cannot be used in indexes. Their counterparts are immutable ... but they will throw errors if asked to make such comparisons." Keep jsonpath temporal methods out of the emitter; `ext.openehr_timestamp` is the right tool.

## 10. Row-level security

**10.1** `ddl-rowsecurity.html`: "Superusers and roles with the BYPASSRLS attribute always bypass the row security system ... Table owners normally bypass row security as well, though a table owner can choose to be subject to row security with ALTER TABLE ... FORCE ROW LEVEL SECURITY." "If no policy exists for the table, a default-deny policy is used." "Referential integrity checks, such as unique or primary key constraints and foreign key references, always bypass row security."

**10.2 Planner order and LEAKPROOF.** `sql-createpolicy.html`: "Generally, the system will enforce filter conditions imposed using security policies prior to qualifications that appear in user queries ... However, functions and operators marked ... as LEAKPROOF may be evaluated before policy expressions." Built-in btree equality operators are leakproof, so `vo_id = $1` probes are unaffected; `ext.*` value predicates run after the policy. The docs give no quantified RLS cost.

**10.3 pg_dump and row_security.** `app-pgdump.html`: "By default, pg_dump will set row_security to off, to ensure that all data is dumped from the table. If the user does not have sufficient privileges to bypass row security, then an error is thrown." BYPASSRLS backup roles are required; `pg_read_all_data` alone is insufficient.

**10.4 security_invoker views.** `sql-createview.html`: "If the view has the security_invoker property set to true, access to the underlying base relations is determined by the permissions of the user executing the query." "Though * was used to create the view, columns added later to the table will not be part of the view." The cold union views were rebuilt three times for this reason (`0008`, `0010`, `0011`).

**10.5 SECURITY DEFINER interplay.** FORCE applies to the owner and the policy reads the session GUC, so a SECURITY DEFINER function is still tenant-filtered by the caller's GUC.

## 11. Roles and the domain barrier

**11.1** `role-membership.html`: "Member roles that have been granted membership with the INHERIT option automatically have use of the privileges of those directly or indirectly a member of, though the chain stops at memberships lacking the inherit option." "Member roles that have been granted membership with the SET option can do SET ROLE to temporarily 'become' the group role." `NOINHERIT` on the domain roles only stops THESE roles from inheriting; a LOGIN role granted membership in two of them re-joins the domains. `verify_domain_isolation` checks `has_table_privilege` per role, which follows inheritance, so it catches that; it cannot catch `SET ROLE` reach, and the docs provide no function for "reachable via SET ROLE".

**11.2** `predefined-roles.html`: "pg_read_all_data ... This role does not bypass row-level security (RLS) policies."

**11.3 What the barrier cannot promise.** `backup-dump.html`: "in order to back up the entire database you almost always have to run it as a database superuser." `app-pgdump.html`: "When -n is specified, pg_dump makes no attempt to dump any other database objects that the selected schema(s) might depend upon. Therefore, there is no guarantee that the results of a specific-schema dump can be successfully restored by themselves." pg_dump "does not dump information about roles or tablespaces." A superuser, a base backup, WAL archiving and physical replication carry every schema together; the domain split holds only at the SQL privilege layer inside one database.

## 12. Logical replication as an outbox alternative

**12.1** `logical-replication-row-filter.html`: "The WHERE clause allows only simple expressions. It cannot contain user-defined functions, operators, types, and collations, system column references or non-immutable built-in functions." `logical-replication-col-lists.html`: "do not rely on this feature for security: a malicious subscriber is able to obtain data from columns that are not specifically published." A row filter cannot call `ext.*`; column lists are not a security boundary; pseudonymisation cannot happen in flight.

**12.2** `logical-replication-restrictions.html`: "The database schema and DDL commands are not replicated." "Replication is only supported by tables, including partitioned tables. ... views ... will result in an error." `runtime-config-wal.html`: "`logical` adds information necessary to support logical decoding ... will increase the WAL volume". The default `wal_level = replica` means a restart before decoding is possible.

## 13. Encryption

**13.1 No TDE in core.** `encryption-options.html` §18.8 enumerates password encryption, "Encryption For Specific Columns ... The pgcrypto module", "Data Partition Encryption ... Storage encryption can be performed at the file system level or the block level ... Block level or full disk encryption options include dm-crypt + LUKS", SSL, and "Client-Side Encryption ... If the system administrator for the server's machine cannot be trusted, it is necessary for the client to encrypt the data". No server-side transparent encryption appears.

**13.2 pgcrypto limits.** `pgcrypto.html`: "All pgcrypto functions run inside the database server. That means that all the data and passwords move between pgcrypto and client applications in clear text. Thus you must: Connect locally or use SSL connections. Trust both system and database administrator. If you cannot, then better do crypto inside client application." "The implementation does not resist side-channel attacks." The application-side AES-256-GCM plus HMAC for `demographic.national_identifier` is the docs' own recommendation.

## 14. RETURNING OLD/NEW, MERGE, COPY, ON CONFLICT

**14.1** `dml-returning.html`: "This syntax for returning old and new values is available in INSERT, UPDATE, DELETE, and MERGE commands". Nothing in the code uses `old.`/`new.`.

**14.2** `sql-merge.html`: "You may also wish to consider using INSERT ... ON CONFLICT as an alternative statement which offers the ability to run an UPDATE if a concurrent INSERT occurs. ... they are not interchangeable." MERGE is not concurrency-safe upsert; the code correctly uses `ON CONFLICT (id) DO NOTHING`.

**14.3** `sql-insert.html`: "ON CONFLICT DO UPDATE guarantees an atomic INSERT or UPDATE outcome ... even under high concurrency." Exclusion constraints "are not supported as arbiters with ON CONFLICT DO UPDATE."

**14.4** `populate.html`: "The COPY command is optimized for loading large numbers of rows ... incurs significantly less overhead for large data loads." "COPY is fastest when used within the same transaction as an earlier CREATE TABLE or TRUNCATE command. In such cases no WAL needs to be written." "Creating an index on pre-existing data is quicker than updating it incrementally as each row is loaded." For a partition-based load: COPY into a fresh unattached table, build indexes, then ATTACH.

## 15. VACUUM, bloat, measurement instruments

**15.1** `routine-vacuuming.html`: "an UPDATE or DELETE of a row does not immediately remove the old version of the row." "vacuum threshold = Minimum(vacuum max threshold, vacuum base threshold + vacuum scale factor * number of tuples)"; defaults: scale factor "0.2 (20% of table size)", `autovacuum_vacuum_max_threshold` "100,000,000 tuples", insert threshold "1000 tuples". `vo_version` produces one dead tuple plus 11 dead index entries per supersession; at 20% a 10M-row table waits for 2M supersessions before autovacuum runs, during which index-only scans degrade. Per-table `autovacuum_vacuum_scale_factor` is the documented lever and none is set. The cold-tier moves create dead tuples in both `vo_version` and `node`. Rewrites are "not MVCC-safe" (`mvcc-caveats.html`).

**15.2** `pgstatstatements.html`: "tracks planning and execution statistics of all SQL statements executed by a server"; `sql-explain.html`: "Buffers information is automatically included when ANALYZE is used." "Keep in mind that the statement is actually executed when the ANALYZE option is used ... BEGIN; EXPLAIN ANALYZE ...; ROLLBACK;" The compose stack already preloads `pg_stat_statements`. Any claim in a redesign PR is checkable with `EXPLAIN (ANALYZE, BUFFERS, WAL)` and `pg_stat_user_tables.n_tup_hot_upd`.

## 16. Asynchronous I/O

`runtime-config-resource.html`: "Selects the method for executing asynchronous I/O. Possible values are: worker ... io_uring (... requires a build with --with-liburing) ... sync ... The default is worker." "io_workers ... The default is 3." Release notes: "allows backends to queue multiple read requests, which allows for more efficient sequential scans, bitmap heap scans, vacuums, etc." Index nested loops are not listed; AIO helps BRIN/bitmap paths, VACUUM and full-population scans. Nothing in compose/Helm tunes it.

## 17. Migrations versus the docs: contradictions and unsupported claims

| # | Location | Claim | What the docs say |
|---|---|---|---|
| 1 | `ehr/0001_baseline.sql:450-453`; `linkage/0001_baseline.sql:107-109`; `docs/postgres-features.md` (temporal row); `storage.md` §Versioning | "GiST exclusion inserts serialize under concurrency (PostgreSQL 18 docs, 'Exclusion Constraints')" | No such sentence exists. Miscitation; the measurement, if it exists, is the only ground. |
| 2 | `ehr/0001_baseline.sql:321-322, 472` | `fillfactor = 90` "one close-out UPDATE per supersession" | HOT requires the update not modify columns referenced by any index; `sys_period` is in four partial-index predicates, so the update is never HOT. |
| 3 | `ehr/0001_baseline.sql:22, 611` | `node.data jsonb COMPRESSION lz4` with "~360 B" fragments | TOAST is "triggered only when a row value ... is wider than ... normally 2 kB". Inert for the typical row. |
| 4 | `docs/postgres-features.md:22,51,56`; `docs/architecture.md:149,252`; `storage.md:304`; `.claude/rules/aql-engine.md:35`; `CLAUDE.md:87` | `JSON_TABLE` "serves array unnesting" in the AQL engine | Code contradiction: zero occurrences of `JSON_TABLE` in `app/` or `crates/`. |
| 5 | `docs/architecture.md:149` | "GIN jsonb_ops `$.**` equality anchors as document pre-filters" | The GIN index was removed in `0001:660-666`; no `@>`/`@?`/`@@` on jsonb anywhere; `jsonb_path_ops` would be the right class anyway. |
| 6 | `docs/postgres-features.md:43` | virtual generated columns as candidate indexes/filters | Virtual columns "must not reference user-defined functions or types"; indexability not stated. Unsupported as written. |
| 7 | `docs/postgres-features.md:26-27, 44` | MERGE for upserts; RETURNING OLD/NEW for audit capture | Neither construct is used; MERGE "not interchangeable" with ON CONFLICT under concurrency. |
| 8 | `ehr/0001_baseline.sql:790-800` | skip scan "covers the remainder" for `tag_key` | Planner-conditional on few distinct leading values; not guaranteed. |
| 9 | `ehr/0007_cold_archive_tier.sql:19-22`, `0008`, `0010`, `0011` | `LIKE … INCLUDING …` mirrors and `SELECT *` union views | One-time copies; "columns added later to the table will not be part of the view." A standing maintenance hazard. |
| 10 | `ehr/0004_multitenancy.sql:28-35` | "RLS appends a constant-equality predicate" | `ext.current_tenant_id()` is STABLE, not a constant; evaluated once per statement; prunes at execution time. |
| 11 | `ehr/0001_baseline.sql:660-666` | expression index removed because the generator never emits the expression verbatim | Consistent with the docs. |
| 12 | `ext/0001_openehr_functions.sql:13-14, 282` | IMMUTABLE parsers; `openehr_timestamp` STABLE | Consistent with `xfunc-volatility.html`. |

## Consequences ranked

1. The version close-out UPDATE is never HOT (2.1). Only an append-only version table plus a separate current-pointer table whose updated columns are unindexed makes the hot write HOT (2.2).
2. The "exclusion inserts serialise" justification is uncited (3.3). Re-ground the removal in a committed measurement or restore the constraint; a design that keeps partial-unique plus advisory lock must keep the lock (3.4).
3. Partitioning is only compatible with `hash(vo_id)`, or with a key set augmented by the partition column (5.1); by-EHR and by-time partitioning break every unique constraint as they stand.
4. A partition tier is not a drop-in for the per-object cold mirror (5.8): row movement on a `tier` key is a DELETE+INSERT; the win is one relation set with FKs and RLS, and DETACH/ATTACH for time-bucketed retention.
5. `body` must stay `text` (1.5). `node.data`'s lz4 annotation is inert below 2 kB (1.1); the lz4 build dependency is real (1.4).
6. RLS partition pruning works at execution time (5.2); a tenant hash partition is viable, a per-tenant partition is not (5.4).
7. Index-only scans on current versions heap-fetch until VACUUM (7.2, 15.1); per-table autovacuum scale factors are the lever and none is set.
8. The documentation layer describes an AQL engine that does not exist (§17 #4, #5): implement or correct the prose.
9. The domain barrier is a SQL-privilege property only (11.3); superusers, base backups, WAL and physical replication carry all schemas; `-n schema` dumps are not restorable alone.
10. Generated columns cannot replace `ext.*`-populated promoted columns as VIRTUAL (8.1).
