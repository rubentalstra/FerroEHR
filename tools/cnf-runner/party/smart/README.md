# CNF SMART lane — the static test issuer

## THIS IS COMMITTED TEST KEY MATERIAL. IT IS NOT SECRET AND MUST NEVER BE USED IN PRODUCTION.

`cnf-smart-test.key.pem` is a **passphrase-less RSA-2048 private key that lives
in public version control**. Anyone with a checkout can mint tokens with it. It
exists for exactly one purpose: to let the CNF runner act as the
Authorization Server the SMART conformance lane needs, without the conformance
stack having to run one.

Never configure a deployment that holds real data to trust this issuer. If a
deployment ever does, every token it accepts is forgeable by anyone.

## What is here

| File | What it is |
|---|---|
| `cnf-smart-test.key.pem` | The RSA-2048 private key (PKCS#8 PEM) the runner signs RS256 access tokens with. |
| `jwks.json` | The matching **public** JWKS (`kid: cnf-smart-test`), mounted into the SUT as `auth.oidc.jwks_json_file`. |

The pair is generated once and committed, so a conformance run is reproducible
and needs no key-generation step. It was produced with:

```bash
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
  -out tools/cnf-runner/party/smart/cnf-smart-test.key.pem
# jwks.json is the public half: kty=RSA, kid=cnf-smart-test, alg=RS256,
# n = the modulus (base64url), e = AQAB.
```

## How the lane wires it together

1. `docker/sut-smart.yml` (the compose overlay) mounts `jwks.json` into the
   SUT and points `auth.oidc` at it, with a static issuer + audience, and turns
   the SMART resource-server role on (`smart.enabled`,
   `smart.require_smart_scopes`).
2. `tools/cnf-runner/party/ehrbase-rs/ixit.smart.json` declares the lane: the
   `smart` block names this key file, the issuer, the audience and the `kid`,
   and the `smart_app` instance's `bearer_mint` auth mode says "sign a fresh
   token per step, carrying that step's declared `scopes`".
3. `scripts/conformance.sh` selects all of the above with `CONF_SMART_MODE=1`.

Cases that declare `scopes:` on a flow step — and every case anchored on the
`I_ITS_REST_SMART` pseudo-interface — are **not-applicable** when the party's
ixit declares no `smart` block (ISO/IEC 9646 test selection), so the default
lane is completely unaffected by this directory's existence.

Spec: `docs/specs/openehr/ITS-REST/docs/smart_app_launch/` —
`master04-service_discovery.adoc` (the discovery document),
`master08-scopes.adoc` (the resource-scope grammar),
`master06-authentication.adoc` (the CDR is a resource server: it validates
tokens and never issues them, which is why the *runner* mints here).
