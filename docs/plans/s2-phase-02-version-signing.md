# Phase S2-02 — Version Signing (VERSION.signature)

- Status: done
- Started: 2026-07-07   Owner: claude
- Consumes (spec/layer): RM 1.2.0 common §"Digital Signature"; BASE `VERSION.canonical_form()`
- Compile required: yes (compiling, tested increments)

## Objectives

Implement the openEHR `VERSION.signature` mechanism (RM common §"Digital
Signature") — the sole STANDARD-profile gap in the CNF conformance claim
(`docs/design/conformance-framework.md` §3.1). Both spec-blessed modes
(hash-only **digest** and hash+sign **OpenPGP**) are first-class; the design is
`docs/design/version-signing.md`.

## Spec authority

- `docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
  §"Digital Signature" — the normative signing process (canonical form → hash →
  optional OpenPGP signature → radix-64; the `signature` attribute is Void
  during serialization; OpenPGP RFC 4880).
- `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.version.adoc` —
  `VERSION.signature: String [0..1]`, `VERSION.canonical_form(): String`.

## Preconditions

- [x] S2-01 access-control base present on the branch (shared `VersionRead` /
      `main.rs` edits kept additive; overlap resolved by the orchestrator).

## Tasks

- [x] `VERSION.canonical_form()` in `openehr-rm` (`version_impl.rs`, ADR-003):
      canonical openEHR JSON with `signature` removed, canonicalised per RFC
      8785 (JCS, `serde_jcs`); property + insta golden tests.
- [x] New leaf crate `ehrbase-signing`: config (figment `EHRBASE_SIGNING_*`),
      signer (digest = `sha256:`+radix-64; pgp = RFC 4880 detached via rPGP),
      verify (`Verdict`), key handling + fail-closed boot validation; unit +
      pgp integration tests; `pgp` 0.20 workspace pin; `cargo deny` green.
- [x] Migration `0002_add_vo_version_signature` (ALTER TABLE vo_version ADD
      COLUMN signature text); `vobject` commit-path reshape (assemble + sign the
      `ORIGINAL_VERSION`); `VersionRead` + read SELECTs carry the signature;
      contribution `UPDATE_VERSION.signature` client-supplied path;
      `verify_on_read` (off/warn/strict).
- [x] Binary wiring (`SigningConfig::load()` + `Signer::from_config()` into
      `EhrbaseService`); e2e suite on testcontainers PG18; XML carries signature.
- [x] Docs: this phase file; `docs/design/version-signing.md` status → implemented.

## Exit criteria

- [x] Every committed version (COMPOSITION / EHR_STATUS / EHR_ACCESS / FOLDER /
      contribution) carries a signature by default (digest mode).
- [x] The digest recomputes from the **served** `ORIGINAL_VERSION`'s
      `canonical_form` (commit-time ≡ read-time object identity).
- [x] Client-supplied signatures stored verbatim; `verify_on_read=strict` turns
      a tampered row into a 5xx; canonical XML carries the signature.
- [x] `cargo build` + `cargo clippy --all-targets -D warnings` + `cargo nextest`
      green for every touched crate; `cargo fmt` clean; `cargo deny` licenses ok.

## Decisions made this phase

- Canonical form = openEHR canonical JSON (ITS-JSON) canonicalised per RFC 8785
  (the spec leaves the serialization `[.tbd]`; design §3.1 `// PORT NOTE:`).
- Digest self-describing via the `sha256:` prefix (design §3.2).
- `pgp` (rPGP) 0.20 with `default-features = false` (drops the bzip2 /
  libbz2-rs-sys dep that fails the cargo-deny license gate); `rand` 0.8 pinned
  at the crate level because rPGP's signing API requires a rand-0.8 `CryptoRng`
  (trait-incompatible with the workspace rand 0.10) — `// PORT NOTE:` in the
  crate manifest.
- `verify_on_read` classifies by signature format (no provenance column stored):
  `sha256:` → digest recompute, PGP armor → verify against the configured key,
  anything else → `ClientForeign` (served) — design §3.5 `// PORT NOTE:`.

## Handoff for next session

Version signing is complete and merged into `claude/s2-access-control` by the
orchestrator (this work was done in an isolated worktree). The Conformance
Statement's "Signing (Security & Privacy, STANDARD)" capability can flip green;
the conformance runner's `SIGN-*` cases (when built per
`docs/design/conformance-framework.md`) cover the e2e set.
