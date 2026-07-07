-- Version signing (VERSION.signature) — RM common §"Digital Signature";
-- design docs/design/version-signing.md §4.3.
--
-- Additive: a nullable, no-default, no-backfill column on the temporal version
-- table. VERSION.signature is 0..1 with no invariants (an OpenPGP RFC 4880
-- signature or a SHA-256 digest, radix-64 encoded), so historical versions
-- legitimately carry none. Runs with search_path = ehr (unqualified table name,
-- matching 0001_schema.sql).
ALTER TABLE vo_version ADD COLUMN signature text;
