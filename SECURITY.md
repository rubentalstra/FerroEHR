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
of concept, impact assessment, and any suggested fix.

### What you can expect from us

- **An acknowledgement within 5 working days.** If you have not heard anything by
  then, the report has not reached us — please escalate by opening a public issue
  saying only that a private report is awaiting acknowledgement, with no details.
- An assessment with a severity and an intended fix window within 10 working
  days of the acknowledgement.
- Coordinated disclosure: we will agree a date with you rather than impose one,
  and we will tell you when the fix ships.

These are commitments to you, not conditions on you. If we miss them, publishing
is your call.

### Safe harbour

We will not pursue or support legal action against anyone who reports a
vulnerability in good faith and follows this policy — meaning: you tested against
your own deployment or a test instance you control, you did not access, modify or
retain data belonging to anyone else, you did not degrade service for others, and
you gave us the window above before publishing. If you are unsure whether
something is in scope, ask first; a question is always in good faith.

**Never test against a live clinical deployment you do not own.** This software
holds patient data.

### Credit

We name reporters in the advisory and the changelog by default, using whatever
name and link you give us. Tell us if you would rather not be named — declining
credit costs you nothing and changes nothing about how the report is handled.

## Scope notes

- The server handles PHI-class data by design; reports about data exposure
  through the API, AQL, telemetry, or the audit trail are in scope even when
  they look like "just configuration".
- Supply-chain policy is enforced in CI by `cargo deny`, which reads the same
  RustSec database `cargo audit` does and adds yanked/licence/source checks on
  top; known advisories that are deliberately accepted are documented with their
  rationale in [`deny.toml`](deny.toml), which is the single advisory gate.
- An advisory reported by a scanner that reads `Cargo.lock` (including the
  OpenSSF Scorecard's) may name a crate our feature set never compiles, because
  the lock file records every dependency any feature combination *could* pull.
  `deny.toml`'s header explains the asymmetry and carries the current example;
  such a report is not an accepted risk, it is a dependency that does not exist
  in the built artifact.
- Findings in an inherited upstream container layer that we have argued are not
  reachable are published as OpenVEX documents under
  [`security/vex/`](security/vex/), with the justification and an impact
  statement you can check. If you think one of those arguments is wrong, that is
  a valid report.

### Reporting a vulnerability in Kubernetes itself

A vulnerability in the Kubernetes platform — the API server, kubelet, etcd, a CNI
or a container runtime — is reported to Kubernetes, not here:
<https://kubernetes.io/docs/reference/issues-security/security/>. Advisories are
published on `kubernetes-announce` and the [official CVE
feed](https://kubernetes.io/docs/reference/issues-security/official-cve-feed/);
following them is the cluster operator's responsibility, as recorded in the
[cluster-hardening chapter](website/book/src/installation/kubernetes-hardening.md).
A vulnerability in FerroEHR — including in the Helm chart — comes to us through
the process above.

## Machine-readable policy

This policy is also published as
[`security.txt`](https://ferroehr.eu/.well-known/security.txt) per
[RFC 9116](https://www.rfc-editor.org/rfc/rfc9116).
