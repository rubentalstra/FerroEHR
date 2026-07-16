# `ehrbase-rest` — the ITS-REST protocol adapter

Implements the **generated** ITS-REST contract (server traits + DTOs from
`openehr-its`) over axum 0.8, plus auth, the `access` authz module
(RBAC/ABAC PEP), the `smart` module (SMART App Launch resource-server
role), ATNA audit middleware, and the EhrScape adapter.

- **The wire is the spec:** status codes, headers (`ETag`, `Location`,
  `Last-Modified`, `openEHR-VERSION.*` committal merge, `Prefer`), and
  content negotiation (canonical JSON + XML) come from
  `docs/specs/openehr/ITS-REST/` + the vendored OAS — never invented.
  Cross-check the CNF schedule; the ECC suite is the acceptance instrument.
- **Implement the generated traits; never fork the contract.** If the
  contract is wrong, fix `openehr-codegen -- emit-rest` and regenerate —
  never hand-edit `src/rest/generated/` in `openehr-its`.
- Auth: Basic + OAuth2/OIDC via the pinned crates (`jsonwebtoken`,
  `oauth2`, `openidconnect`, `argon2`) — never hand-rolled. SMART scope
  enforcement AND-composes onto the ABAC PEP and is config-gated off by
  default (zero wire drift when disabled); spec:
  `docs/specs/openehr/ITS-REST/docs/smart_app_launch/`.
- URL/percent encoding ONLY via the `urlencoding` crate (owner hard rule).
- Rules: `.claude/rules/rest-axum.md`, `.claude/rules/auth.md`,
  `.claude/rules/serialization.md`.
- Gates: `cargo clippy -p ehrbase-rest --all-targets` +
  `cargo nextest run -p ehrbase-rest` green; wire changes re-verified by an
  ECC run (`scripts/conformance.sh`) with zero drift.
