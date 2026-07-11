# ADR-017: DV_MULTIMEDIA externalization to S3-compatible object storage

- **Status:** accepted (posture owner-confirmed 2026-07-10 roadmap §2.4,
  incl. SeaweedFS for dev/test/compose)
- **Date:** 2026-07-11
- **Spec basis:** RM 1.2.0 DV_MULTIMEDIA — `uri` (external) and `data`
  (inline) are alternative/co-present under invariant `is_inline or
  is_external`; `integrity_check` requires `integrity_check_algorithm`
  (coded set); `size` is the mandatory unencoded byte count; RM ehr_extract
  §References-to-Resources even sanctions non-resolvable URIs. Server-side
  blob storage/fetch is **spec-silent** — this ADR fills it.

## Decision

1. **Threshold offload, off by default.** `MultimediaConfig { enabled:
   false, threshold_bytes (default 256 KiB), endpoint/bucket/creds }`.
   Disabled = today's inline behaviour byte-identical (zero-drift gate).
2. **Content-addressed blobs.** On commit, a DV_MULTIMEDIA whose decoded
   `data` exceeds the threshold is written to the object store keyed by its
   sha-256 (natural dedup + immutability, matching version indelibility);
   the stored canonical JSON replaces `data` with `uri`
   (`s3://<bucket>/<sha256>` — an RFC-3986 URI; the RM allows any scheme
   and does not require resolvability), sets `integrity_check` (the sha-256
   octets), `integrity_check_algorithm` (the openEHR coded set's SHA-256
   entry), and keeps the mandatory unencoded `size` — every RM invariant
   honoured both directions.
3. **Wire transparency.** Reads serve the stored form (uri + integrity) by
   default; `?expand_multimedia=true` (or the FLAT path, which needs bytes)
   fetches and re-inlines transparently, verifying the sha-256 before
   serving (integrity failure = 500, never silent corruption).
4. **`object_store` crate** as the backend abstraction (S3-compatible:
   SeaweedFS in dev/test/compose via its S3 gateway — testcontainers
   fixture + a compose service; any S3 endpoint in production). Version
   verified live before pinning.
5. **Lifecycle.** Blobs are immutable and shared (content addressing);
   admin physical EHR delete collects the candidate hashes and deletes
   those no longer referenced by any stored node (a scan-based GC in the
   admin path — conservative, transactional bookkeeping via a `blob_ref`
   count table is deliberately avoided this pass, PORT NOTE). Dump/load
   includes referenced blobs in the archive (export fetches, import
   re-puts).
6. **Never external without integrity.** An externalized value always
   carries integrity fields; inbound compositions that already carry a
   `uri` (client-managed external media) are stored verbatim (spec-legal,
   not our blob).

## Consequences

Commit-path interception in the node codec boundary (decompose sees
canonical JSON before storage); read-path expansion in the serving seam;
new config + testcontainers fixture; no schema change beyond nothing (blobs
live outside PG; the canonical JSON already carries the fields).

## Alternatives considered

Inline-always (bloats the version store; lz4 helps but PACS-scale media
doesn't belong in jsonb); a blob-ref counting table (bookkeeping complexity
now, GC-scan suffices at this stage — revisit at P20 scale); DB large
objects (ties blobs to PG backup weight — object storage is the point).
