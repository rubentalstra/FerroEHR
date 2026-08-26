---
name: pgp-subkey-signature-runner-cannot-verify
description: RUNNER bin — rpgp 0.20 SignedPublicKey::verify is primary-key-only, so the runner rejects the app's (conformant) signing-subkey signature
metadata:
  type: project
---

Confirmed 2026-08-12: all three `SIG-VERSION-*-pgp` cases asserting
`verifiable: true` fail with "signature: does not verify over the agreed
canonical form"; every digest twin and every mode-agnostic pgp case passes.

The chain, all read first-hand:

1. `app/ferroehr/src/versioning/signature/key.rs:157-160` selects a
   signing-capable SUBKEY by RFC 9580 §5.2.3.29 key flag 0x02 and signs with it
   (`:181-195`). Landed in **7a8a8c9a3** (2026-08-07) — AFTER the last green
   baseline; before it the primary key signed.
2. The committed test certificate HAS one: `gpg --show-keys
   corpus/keys/cnf-signing.sec.asc` →
   `sec [SCEAR] C87EEF94…` + `ssb [S] 376C44DE…`. The ixit's
   `instances.sut_pgp.signing.public_key` is byte-identical to the committed
   `.pub.asc`, i.e. the same certificate.
3. **Pinned `pgp` 0.20.0, `src/composed/signed_key/public.rs:200-204`:**
   `impl VerifyingKey for SignedPublicKey { fn verify(..) { self.primary_key.verify(..) } }`
   — `public_subkeys` is never consulted.
4. Veredictum's `src/exec/signature.rs:126` is
   `sig.verify(&key, bytes).is_ok()` on the primary `SignedPublicKey` → `Ok(false)`.

The app already has the correct shape:
`key.rs::verify_against_certificate` (`:224-236`) tries the primary key then
every subkey. The runner needs the same walk.

Not environmental: `PgpKey::load` is fail-closed at boot (a test signature), so
the pgp deployment could not have started with a missing/unusable key, and
`SIG-VERSION-client_supplied_verbatim-pgp` passed on the same instance in the
same run. `Ok(false)` (not `Err`) also proves the armor PARSED.

**How to apply:** a `-pgp` verifiable failure with the digest twin green is
never a canonicalization bug — the digest case proves the JCS bytes agree. Look
at which key component signed.
