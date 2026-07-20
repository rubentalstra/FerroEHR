# Conformance PGP signing key — TEST-ONLY

`signing-key.asc` is an **armored, passphrase-free Ed25519 RFC 4880 secret key
generated solely for the conformance stack**. It is mounted into the
`ehrbase-pgp` service (see `docker/conformance/ehrbase-pgp.override.yml`) so the
server boots in `[signing] mode = "pgp"` and the ECC `ECC-SIG-005`
(`sig/pgp-verifies`) case can observe a real RFC 4880 detached signature on a
served `ORIGINAL_VERSION`.

**This is not a secret.** It protects nothing, authenticates no one, and MUST
NOT be reused in any real deployment. It exists only so the pgp signing path is
exercisable in CI without generating key material at compose-init. Production
deployments configure their own key via `[signing] key_path`
(RM common `master06-change_control_package.adoc` §Digital Signature).
