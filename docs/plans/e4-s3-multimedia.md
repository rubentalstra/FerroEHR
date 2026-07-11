# E4 — DV_MULTIMEDIA externalization (S3-compatible; SeaweedFS dev/test)

- Status: done (2026-07-11)
- Started: 2026-07-11   Owner: Ruben
- Governing design: docs/enterprise/product-roadmap.md §2.4 (owner-confirmed
  incl. SeaweedFS for dev/test/compose) → ADR-017; spec basis RM
  DV_MULTIMEDIA (uri/data alternatives, `is_inline or is_external`
  invariant, integrity_check + algorithm invariants, mandatory unencoded
  size); server-side blob storage is spec-silent.
- Gates: workspace suites green; full ECC zero drift (341/315/0 — inline
  behaviour unchanged by default); blob round-trip tests against SeaweedFS.

## Tasks

- [x] 1. ADR-017 — externalization design record: threshold-based offload
      (default off / inline-preserving), content-addressing (sha-256 →
      integrity_check + coded algorithm), uri scheme + fetch behaviour,
      object_store crate abstraction, blob GC tied to admin physical
      delete, wire transparency (serve inline on demand).
- [x] 2. Storage: `object_store` integration (verify live version), config
      (off by default; endpoint/bucket/creds), commit-path interception
      (DV_MULTIMEDIA.data above threshold → put object, store uri +
      integrity fields + size, honour the RM invariant both directions),
      read-path transparent expansion option.
- [x] 3. SeaweedFS: docker-compose service + testcontainers fixture (S3
      gateway) for integration tests; document production S3 pointing.
- [x] 4. Blob lifecycle: content-addressed dedup; admin physical EHR delete
      cascades to unreferenced blobs; dump/load carries external blobs.
- [x] 5. Tests: round-trip (commit large multimedia → offloaded, wire
      read returns inline-on-demand and uri forms, integrity verified);
      threshold-off default = byte-identical inline behaviour (suites
      unchanged); GC test; dump/load with blobs.

## Exit criteria

- [x] ADR-017 accepted; SeaweedFS e2e 7/7; default-mode ECC 341/315/0 zero
      drift; scorecard flipped.
