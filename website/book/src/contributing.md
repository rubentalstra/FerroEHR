# Contributing

FerroEHR is open source (MIT; vendored openEHR artifacts remain Apache-2.0) and welcomes contributions. This
chapter is a short orientation for anyone who wants to file an issue, report a
vulnerability, or open a pull request; the authoritative documents live in the
repository and are linked below.

## Where to start

The three governing documents are kept in the repository root:

- [CONTRIBUTING](https://github.com/rubentalstra/FerroEHR/blob/develop/CONTRIBUTING.md)
  — the practical rules for setup, the required checks, and pull requests.
- [Code of conduct](https://github.com/rubentalstra/FerroEHR/blob/develop/CODE_OF_CONDUCT.md)
  — the Contributor Covenant (v2.1) the community follows.
- [Security policy](https://github.com/rubentalstra/FerroEHR/blob/develop/SECURITY.md)
  — how to report a vulnerability privately.

## Setting up

The Rust toolchain is pinned by the repository's `rust-toolchain.toml`, so
`rustup` installs the right version automatically on your first build. Two
extra tools are needed for the full test suite:

- **Docker**, for the PostgreSQL 18 integration tests (they spin up a real
  database via testcontainers).
- **`xmllint`** (from `libxml2`), used by the canonical-XML parity tests.

Install the shared git hooks once with `bash scripts/install-hooks.sh`.

## The checks every pull request must pass

CI runs the same set of gates locally and on every pull request — none of them
are advisory:

```bash
cargo build --workspace
cargo nextest run --workspace          # unit + integration (real PostgreSQL 18)
cargo test --workspace --doc
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo deny check && cargo audit && cargo machete
bash scripts/check-codegen-drift.sh    # generated layer matches the vendored specs
```

> [!IMPORTANT]
> Two rules are absolute. Never hand-edit a generated file — anything under a
> `// @generated … DO NOT EDIT` header is produced by the code generator;
> change the generator and regenerate instead. And never weaken, skip, or
> delete a test to make a build pass, or edit a test to route around a bug it
> exposes.

A few more conventions worth knowing before you open a pull request:

- Branch from `develop`, and target your pull request at `develop`.
- Keep changes focused, and describe **what** changed and **why**. For
  anything that touches openEHR behaviour, cite the relevant specification
  section.
- Behaviour changes come with tests. Snapshot changes must be reviewed, not
  blindly accepted.
- Any user-visible change (the REST surface, AQL, validation, configuration,
  the CLI, or the deployment artifacts) adds an entry to the changelog in the
  same pull request — a CI guard enforces this.

## Reporting issues and vulnerabilities

Use the GitHub issue tracker for bugs and feature requests.

> [!WARNING]
> Do **not** open a public issue for a suspected security vulnerability.
> Report it privately through
> [GitHub's private vulnerability reporting](https://github.com/rubentalstra/FerroEHR/security/advisories/new)
> ("Report a vulnerability" on the repository's Security tab). Because the
> server handles PHI-class data by design, reports about data exposure through
> the API, AQL, telemetry, or the audit trail are in scope even when they look
> like "just configuration". Coordinated disclosure is preferred — please
> allow a reasonable window for a fix before publishing details.
