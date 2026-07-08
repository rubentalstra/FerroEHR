# Version Signing (VERSION.signature) — design

- **Status:** implemented (2026-07-07; owner-prioritized — closed the sole
  STANDARD-profile gap in the CNF conformance claim, see
  `docs/design/conformance-framework.md` §3.1). Landed on
  `claude/s2-access-control` per `docs/plans/s2-phase-02-version-signing.md`:
  `openehr-rm` `VERSION.canonical_form()` (RFC 8785 JCS), the `ehrbase-signing`
  crate (digest + OpenPGP RFC 4880 via rPGP), the `vobject` commit-path reshape
  (sign the assembled `ORIGINAL_VERSION`), `verify_on_read`, and the binary
  wiring — all tested (openehr-rm property/golden, ehrbase-signing unit + pgp,
  service e2e on PG18).
- **Spec authority (extracted 2026-07-07, citations verified):**
  - `docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
    §Digital Signature — the normative signing process
  - `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.version.adoc`
    — `VERSION.signature: String [0..1]`, `canonical_form(): String`
  - `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — "Signing"
    capability, Security & Privacy, STANDARD profile
  - ITS-REST: `schemas/common/Version.yaml` (signature on read),
    `schemas/common/UpdateVersion.yaml` (signature on contribution commit)
- **Greenfield mandate:** no wrappers, no interim stubs. Both spec modes
  (digest and OpenPGP signature) are built properly from day one; the commit
  path is reshaped where needed rather than decorated.

---

## 1. What the spec requires (the extracted contract)

From RM common §Digital Signature, distilled and binding:

1. **When:** at committal of a Version.
2. **Input:** the **entire** `ORIGINAL_VERSION` / `IMPORTED_VERSION` object
   with the `signature` attribute **Void** during serialization (the RM's
   `canonical_form()`: "serialising all attributes except signature, suitable
   for generating reliable hashes and signatures").
3. **Process:** canonical serialization → hash → *(if key infrastructure
   exists)* digital signature over the hash with the author's private key →
   **radix-64** encoding → written into `signature`.
4. **Format:** the OpenPGP standard, **IETF RFC 4880** — the signature is
   self-describing (algorithms indicated within it).
5. **Two spec-blessed modes:** hash-only (**digest** — data-integrity check)
   or hash+sign (**signature** — authentication + non-repudiation). Both are
   first-class; neither is a stub of the other.
6. **Optionality:** `signature` is 0..1 with **no invariants**; the REST API
   carries it on every VERSION read and accepts a client-supplied value in
   `UPDATE_VERSION` on the CONTRIBUTION commit path; the direct composition
   endpoints carry data only (server-side signing there).
7. **Explicitly implementation-defined (spec `[.tbd]`):** the exact canonical
   serialization ("not yet defined by openEHR; ODIN might be preferred").
   **We own this choice** (§3.1).
8. Distinct from **ATTESTATION.proof** (human attestation of a view of
   content, `commit_audit`/`attestations`-carried, own signing process) —
   related capability, *not* this design; hooks noted in §8.

CNF reality check: "Signing" is a declared STANDARD capability with **zero
executable CNF tests** (non-functional conformance is out of the guide's
scope). Meeting it = implementing the RM mechanism faithfully + proving it
with our own tests + declaring it in the Conformance Statement. Our
conformance runner adds runner-defined `SIGN-*` cases (§7).

## 2. Design at a glance

```
commit (vobject, one tx)                          read (reassemble)
─────────────────────────                         ─────────────────
build ORIGINAL_VERSION (signature = None)         SELECT … , signature
        │                                         ORIGINAL_VERSION.signature = row
        ▼                                                 │
canonical_form()  ← openehr-rm *_impl.rs                  ▼ (opt-in)
        │                                         verify_on_read: Off | Warn | Strict
        ▼
ehrbase-signing::Signer
  ├─ Digest  : SHA-256 → radix-64
  └─ Pgp     : RFC 4880 detached sig (rPGP) → ASCII armor
        │
        ▼
INSERT vo_version(…, signature)
```

- **`canonical_form()` is an RM spec function** → hand-written in the spec
  layer (`crates/openehr-rm/src/common/change_control/version_impl.rs`, the
  ADR-003 sanctioned home), NOT in the app. It is the single source of the
  signed bytes for signing *and* verification.
- **The crypto + policy live in a new leaf app crate `ehrbase-signing`**
  (the `ehrbase-audit` pattern): modes, key handling, verification, config.
- **Storage:** a new `signature text` column on `vo_version` (additive
  migration `0002`) — version-level metadata lives on the version row, like
  `template_id`; the node table stays pure content.

## 3. The decisions (each a `// PORT NOTE:` in code)

### 3.1 Canonical serialization = canonical openEHR JSON + RFC 8785 (JCS)

The spec leaves the serialization TBD and *suggests* ODIN. Decision: the
Version object is serialized to our **canonical openEHR JSON** (the ITS-JSON
encoding the generated `OpenEhrType` derive produces — `_type`-tagged, nulls
omitted) and then canonicalized with **RFC 8785 JSON Canonicalization Scheme**
(`serde_jcs`, already in the pinned stack) for byte-determinism (key ordering,
number formatting, string escaping are all pinned by the RFC).

Rationale: (a) JSON is our primary wire format and the ITS-JSON encoding is
spec-defined, so "an agreed XML, ODIN or other text format" is satisfied with
a *published, standardized* canonicalization on top; (b) no ODIN serializer
for RM instances exists in this codebase or practically anywhere — inventing
one for signing would be a bigger unilateral choice than JCS; (c) RFC 8785 is
exactly the "unambiguous encoding" property the spec wants from ODIN.
Interop caveat (inherent to the spec's TBD, not to our choice): a signature is
verifiable only by a party using the same canonical form — therefore the
Conformance Statement and this doc both declare it: **"canonical form =
openEHR canonical JSON per ITS-JSON, canonicalized per RFC 8785"**.

### 3.2 Both modes, properly

- **`digest` mode** (spec: "If only the hashing step is done, the digest acts
  as a data integrity check"): `radix-64(SHA-256(canonical_form))`, stored as
  `sha256:<base64>` — the prefix makes the digest self-describing (the spec's
  self-description requirement is defined by RFC 4880 only for the signed
  form; a bare radix-64 hash is otherwise ambiguous, so the algorithm prefix
  is our documented concretization). `sha2` is already in the workspace.
- **`pgp` mode** (spec: authentication + non-repudiation): an RFC 4880
  **detached signature over the canonical_form bytes**, ASCII-armored, using
  a server-held OpenPGP private key. Crate: **`pgp` (rPGP)** — pure Rust,
  actively maintained, the only serious no-C OpenPGP implementation (fits the
  rustls-only, no-OpenSSL stack; sequoia-openpgp rejected for its C backend
  default and weight). Add to `[workspace.dependencies]` (verify latest on
  crates.io at implementation; run `cargo deny check` — rPGP is MIT/Apache).
  Key: Ed25519 recommended; the config takes any armored RFC 4880 secret key.
- Signing the *hash* vs the *content*: RFC 4880 signatures internally hash
  the signed data; producing the detached signature over the canonical bytes
  is the standard-conformant realization of the spec's "digital signature can
  be created from the hash" prose. (PORT NOTE in code.)

### 3.3 Who signs what, when

- **Server signing** happens inside the same transaction scope as the version
  write, in the shared `vobject` commit path (all five versioned-object
  services get it at once — COMPOSITION, EHR_STATUS, FOLDER, and the
  contribution multi-version path). The ORIGINAL_VERSION is fully built
  (uid, audit, contribution ref, data, attestations) with `signature = None`,
  canonicalized, signed, then persisted — matching the spec's ordering
  exactly.
- **Client-supplied signatures win.** The CONTRIBUTION path (`UPDATE_VERSION`)
  may carry an author-generated signature; the spec's actor is "the user".
  A client-supplied signature is stored **verbatim** (never re-signed,
  never validated against our canonical form — the author may have used
  another agreed serialization; rejecting it would exceed the spec). Server
  signing applies only when the client supplied none.
- **IMPORTED_VERSION:** the spec has the importing system sign the
  IMPORTED_VERSION wrapper itself. We do not implement version import yet;
  the seam handles `Version<T>` generically so the rule is honored the day
  import lands (scope note, not a stub).

### 3.4 Defaults: signing ON, digest mode

`signing.enabled = true`, `mode = digest` by default: every committed version
carries an integrity digest with zero key management, which is what makes the
STANDARD "Signing" capability *demonstrably met* out of the box. `pgp` mode is
config-opt-in (needs a key). This intentionally changes default behaviour —
existing tests/snapshots that assert VERSION payloads must be updated to
expect the signature field (a legitimate expectation change, not weakening;
the implementer updates them with the diff cited).

### 3.5 Verification: `verify_on_read = off | warn | strict`

Recompute-and-compare (digest) / RFC 4880 verify (pgp) at reassembly time:
- `off` (default) — signature is served as stored (the spec's model: a stored
  fact, verified by whoever needs trust).
- `warn` — mismatch logs `tracing` error + `metrics` counter
  (`version_signature_invalid_total`), response still served.
- `strict` — mismatch is a 500-class integrity failure (the stored record is
  provably corrupt or tampered; serving it silently would be dishonest).
Client-supplied signatures are exempt from digest/pgp recomputation (§3.3) —
they get PGP structural validation only (parseable armor) in warn/strict.

## 4. Components

### 4.1 `openehr-rm` — the spec function (hand-written impl, ADR-003)

`crates/openehr-rm/src/common/change_control/version_impl.rs` (new):

```rust
impl OriginalVersion { pub fn canonical_form(&self) -> Result<String, CanonicalFormError>; }
impl ImportedVersion { pub fn canonical_form(&self) -> Result<String, CanonicalFormError>; }
impl Version          { pub fn canonical_form(&self) -> Result<String, CanonicalFormError>; }
```

Implementation: clone-with-`signature = None` (cheap field swap on a
serde_json::Value, not a deep clone of the RM object: serialize → remove the
`signature` key → `serde_jcs::to_string`), documented as the RM
`canonical_form` function with the §3.1 PORT NOTE. `serde_jcs` becomes a
dependency of `openehr-rm` (workspace-pinned). Property test: canonical form
is byte-stable across repeated serialization and independent of the
`signature` value present on the object.

### 4.2 New crate `app/ehrbase-signing` (leaf, app layer)

```
src/
├── lib.rs
├── config.rs     # SigningConfig (figment, EHRBASE_SIGNING_*): enabled, mode,
│                 #   key_path, key_passphrase (secrecy), verify_on_read
├── signer.rs     # Signer::{digest, pgp} — sign(canonical: &str) -> String
├── verify.rs     # verify(canonical: &str, signature: &str) -> Verdict
│                 #   (DigestMatch/DigestMismatch/PgpValid/PgpInvalid/ClientForeign)
└── key.rs        # rPGP key loading (armored secret key + passphrase), boot validation
```

Deps: `pgp` (rPGP — new workspace pin), `sha2`, `base64`, `secrecy`,
`serde`/`figment`, `thiserror`, `tracing`, `metrics`. No `ehrbase-*` deps.
Boot validation: `pgp` mode without a loadable key = refuse to start
(fail-closed at boot, the access-control precedent).

### 4.3 Service integration (`app/ehrbase`)

- `vobject` commit path: after the ORIGINAL_VERSION is assembled and before
  persist — `if client_signature { store it } else if signing.enabled {
  signer.sign(version.canonical_form()?) }`. One seam, all object kinds.
  **Reshape, don't wrap:** if the current commit path builds the
  ORIGINAL_VERSION only implicitly (fields scattered into the INSERT), it is
  restructured so the full RM object exists as a value before persistence —
  that is what the spec signs, and the object is already needed for
  reassembly parity.
- Migration `0002` (via `sqlx migrate add --sequential`): `ALTER TABLE
  ehr.vo_version ADD COLUMN signature text;` — nullable, no default, no
  backfill (historical versions legitimately have none; the spec makes it
  0..1).
- `VersionRead` + `read_current`/`read_version` select `signature`;
  reassembly sets `ORIGINAL_VERSION.signature`. (Coordination note: the
  access-control work later adds `template_id` to the same struct/queries —
  whichever lands second rebases trivially.)
- Contribution service: map `UPDATE_VERSION.signature` through to the commit
  seam (client-supplied path).
- `main.rs`: `SigningConfig::load()` beside the other configs; `Signer`
  handed to `EhrbaseService` construction.

### 4.4 Wire (already done by the generated layer — verified)

`signature` is a plain optional String in the generated RM structs, ITS-JSON
schema, XSD, and generated XML impls, and rides every VERSION-returning
endpoint. **No REST/serialization work needed** beyond the service filling
the field.

## 5. Configuration

| Key | Env | Default | Meaning |
|---|---|---|---|
| `signing.enabled` | `EHRBASE_SIGNING_ENABLED` | `true` | server-side signing of committed versions |
| `signing.mode` | `EHRBASE_SIGNING_MODE` | `digest` | `digest` \| `pgp` |
| `signing.key_path` | `EHRBASE_SIGNING_KEY_PATH` | — | armored RFC 4880 secret key (required for `pgp`) |
| `signing.key_passphrase` | `EHRBASE_SIGNING_KEY_PASSPHRASE` | — | key passphrase (secrecy-wrapped, redacted from `/management/env`) |
| `signing.verify_on_read` | `EHRBASE_SIGNING_VERIFY_ON_READ` | `off` | `off` \| `warn` \| `strict` (§3.5) |

## 6. Tests (binding)

1. **canonical_form** (openehr-rm): property tests — byte-stability,
   signature-field independence (same output whether `signature` is None or
   set), JCS golden vector (insta) for a corpus composition's ORIGINAL_VERSION.
2. **Signer/verify unit** (ehrbase-signing): digest golden vector; pgp
   sign→verify round-trip with a generated test key; tampered-canonical
   detection; armor parse failures; boot-validation rejects missing/garbled
   key in pgp mode.
3. **Service e2e** (testcontainers PG18): commit composition → versioned
   composition GET returns `signature`; digest recomputes correctly from the
   *served* VERSION's canonical form (proves commit-time and read-time object
   identity — the strongest test in this design); EHR_STATUS update + FOLDER
   + contribution multi-version all signed; contribution with client-supplied
   signature stored verbatim; `verify_on_read=strict` with a row tampered via
   SQL → 5xx; XML negotiation carries the same signature.
4. **Snapshot updates**: existing VERSION-payload snapshots updated for the
   new field, each reviewed (`cargo insta review` discipline).
5. **Conformance runner** (when built, per `conformance-framework.md`):
   runner-defined `SIGN-*` cases covering the e2e set; "Signing" capability
   flips green in the Profile Report — closing the STANDARD gap.

## 7. Conformance statement text (generated, once green)

> Signing (Security & Privacy, STANDARD): supported. Server-generated
> integrity digests by default; OpenPGP RFC 4880 detached signatures with a
> configured key. Canonical form: openEHR canonical JSON (ITS-JSON)
> canonicalized per RFC 8785. Client-supplied signatures on CONTRIBUTION
> commits are preserved verbatim.

## 8. Explicit non-scope (each with its future hook)

- **ATTESTATION.proof / attestation commits** (distinct RM mechanism,
  `master04-generic_package.adoc`): the `Signer`/`canonical_form` machinery is
  reusable for it; not built now.
- **EHR Extract / MESSAGE.signature**: same pattern, out of Stage-1 scope.
- **Version import (IMPORTED_VERSION)**: §3.3 scope note.
- **Key rotation/HSM**: single configured key now; the `key.rs` seam is where
  a provider abstraction would go *when a requirement exists* — not before.

## 9. Implementation plan (single implementer task, worktree-isolated)

Branch `claude/version-signing` **based on `origin/develop`** (independent of
the access-control branch; merge order: access-control first, this rebases).
Compiling, clippy-clean, tested increments; commits `signing: <step>`:

1. `canonical_form` in `openehr-rm` `version_impl.rs` + `serde_jcs` workspace
   dep + property/golden tests (§4.1, §6.1).
2. `ehrbase-signing` crate: config + signer + verify + key handling + unit
   tests (§4.2, §6.2); `pgp` workspace pin (verify version, `cargo deny`).
3. Migration `0002` + `VersionRead`/reassembly + the vobject commit-path
   reshape + contribution client-signature path (§4.3).
4. Binary wiring + `verify_on_read` + e2e suite + snapshot updates (§6.3–4).
5. Docs: this file's status flip; `docs/plans/s2-phase-01-access-control.md`
   untouched (separate phase file `docs/plans/s2-phase-02-version-signing.md`
   with the step checklist, ticked as landed).

Hard rules apply unchanged: spec citations in commit messages
(`RM common §Digital Signature`), no `// @generated` edits, no test
weakening — the snapshot updates cite this design.
