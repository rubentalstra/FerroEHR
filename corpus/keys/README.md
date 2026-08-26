# CNF version-signing test keys (OpenPGP, RFC 4880)

**Test-only key material** for the openPGP-posture half of every conformance
run. Not a secret: it exists so the composed SUT can sign VERSIONs with a known
OpenPGP key and the runner can verify the signatures against the paired public
key (RM common `master06-change_control_package.adoc` §Digital Signature — the
signature is generated per the openPGP standard). Never use these keys for
anything real.

- `cnf-signing.sec.asc` — the armored OpenPGP **secret** key (no passphrase).
  Mounted into the pgp-posture deployment (`docker/sut-signing-pgp.yml`) as
  `FERROEHR__SIGNING__KEY_PATH`; that deployment signs each committed VERSION
  with it.
- `cnf-signing.pub.asc` — the armored OpenPGP **public** key. Inlined into the
  `sut_pgp` instance's own `signing` block (`party/ferroehr/ixit.json`); the
  runner verifies `ORIGINAL_VERSION.signature` against it (RFC 4880 detached
  signature over the agreed canonical form: RFC 8785 JCS of the version minus
  `signature`).

**Provenance / regeneration.** A primary key certifying a dedicated signing
subkey (the in-OpenPGP certification chain — the OpenPGP analogue of a CA→cert
chain; openEHR version signing is OpenPGP, not X.509). Regenerate with:

```
export GNUPGHOME=/tmp/cg && mkdir -p "$GNUPGHOME" && chmod 700 "$GNUPGHOME"
gpg --batch --pinentry-mode loopback --gen-key <<'EOF'
%no-protection
Key-Type: RSA
Key-Length: 3072
Subkey-Type: RSA
Subkey-Length: 3072
Subkey-Usage: sign
Name-Real: CNF Test Version Signing
Name-Email: cnf-signing@test.ferroehr.local
Expire-Date: 0
%commit
EOF
gpg --armor --export         cnf-signing@test.ferroehr.local > cnf-signing.pub.asc
gpg --armor --export-secret-keys cnf-signing@test.ferroehr.local > cnf-signing.sec.asc
```
