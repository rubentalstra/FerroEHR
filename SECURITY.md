# Security Policy

FerroEHR is a clinical data repository; security reports are taken seriously
and handled with priority.

## Supported versions

The project is pre-release. Security fixes land on `develop` and in the next
tagged release; there are no maintenance branches yet.

## Reporting a vulnerability

**Please do not open a public issue for suspected vulnerabilities.**

Report privately via
[GitHub private vulnerability reporting](https://github.com/rubentalstra/FerroEHR/security/advisories/new)
("Report a vulnerability" on the repository's Security tab).

Include what you can: affected component/endpoint, reproduction steps or a proof
of concept, impact assessment, and any suggested fix. You will receive an
acknowledgement, and coordinated disclosure is preferred — please allow a
reasonable window for a fix before publishing details.

## Scope notes

- The server handles PHI-class data by design; reports about data exposure
  through the API, AQL, telemetry, or the audit trail are in scope even when
  they look like "just configuration".
- Supply-chain policy is enforced in CI (`cargo deny`, `cargo audit`); known
  advisories that are deliberately accepted are documented with rationale in
  [`deny.toml`](deny.toml) and [`.cargo/audit.toml`](.cargo/audit.toml).
