# VEX statements

Vulnerability Exploitability eXchange documents, in [OpenVEX](https://openvex.dev)
format, asserting the exploitability of specific findings in the images this
project publishes.

A VEX document is not a way to silence a scanner. It is the machine-readable form
of an argument, and it carries the argument with it: each statement names the
vulnerability, the product, a `status`, a controlled-vocabulary `justification`,
and an `impact_statement` a reader can check. The alternative — an ignore list —
records the decision without the reasoning, which decays into a list nobody can
re-evaluate.

## Rules

- **`not_affected` needs a justification from the OpenVEX vocabulary**, and the
  `impact_statement` must say concretely why the code is unreachable in *our*
  usage. "Low risk" is not a justification.
- **A finding we can fix is fixed, not VEXed.** These documents exist for
  findings in inherited upstream layers, where the fix belongs to someone else.
- **Re-check on every base-image bump.** When an upstream image rebuilds its
  bundled binaries, statements about them become obsolete and the entries go —
  a stale `not_affected` is worse than no VEX at all.
- The scanners consume these files (`trivy --vex`), so a statement that stops
  being true stops being invisible: the finding returns and the gate fails.

## Documents

| File | Subject |
|---|---|
| `postgres-gosu.openvex.json` | Go standard-library advisories in `/usr/local/bin/gosu`, the privilege-dropping helper the upstream `postgres` image ships. Not reachable: gosu sets uid/gid and execs, opening no socket and parsing no untrusted input. |
