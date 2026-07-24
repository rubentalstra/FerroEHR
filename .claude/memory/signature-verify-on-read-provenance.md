---
name: signature-verify-on-read-provenance
description: "verify-on-read judges ONLY server-generated signatures; client-supplied sigs stored verbatim + never re-verified (provenance bit), spec-grounded RM master06"
metadata: 
  node_type: memory
  type: project
  originSessionId: de7375a6-e0dc-4d86-b3ec-25f2b1c8813a
  modified: 2026-07-24T15:03:06.826Z
---

#273 (v3.9.0): `signing.verify_on_read` defaults to **strict when signing is
enabled** (was `off`) — our-own-design integrity hardening, NOT a spec
requirement (RM common master06 §Digital Signature frames verification as the
reader/receiver's role; canonical serialisation is "To Be Determined"; no
openEHR spec governs server-side verify-on-read timing).

**The load-bearing correctness insight:** read-time verification must judge ONLY
signatures the SERVER produced. A **client-supplied** signature (an author sig
over "another agreed serialization", or an `IMPORTED_VERSION` carrying its
origin's signature — master06 §Digital Signature / §Copying) is stored
byte-for-byte and MUST NEVER be re-verified — we cannot recompute a foreign
canonical form. Format heuristics are LEAKY (a client `sha256:` sig, or in
**pgp mode** a foreign client PGP sig, both false-fail against our key). The
only robust fix is a stored provenance bit:

- `vo_version.signature_client_supplied boolean NOT NULL DEFAULT false` (baseline edit).
- Write: `change::version_signature` returns `(Option<String>, bool)`; imports bind `true`; archive dump/load round-trips the bit.
- Read: threads `StoredVersion → VersionRead → wire::original_version → integrity::verify_on_read`, which returns early (skips) when client-supplied.
- Config: `verify_on_read: Option<VerifyOnRead>` (None = unset) + `SigningConfig::effective_verify_on_read()` → Strict when enabled, Off when disabled. `off`/`warn` explicitly selectable.

Result: strict-by-default catches tampered SERVER sigs in both digest+pgp modes,
never false-fails a stored client sig → the CNF SIG-VERSION cases (incl.
client-verbatim) stay green in both modes with NO compose pin. See
[[owner-work-style]] (no quick fixes / proper rewrites).
