# Machine review (SonarQube Cloud) — what it is and what it is not

Every pull request and every develop push is analyzed by SonarQube Cloud
(`.github/workflows/sonar.yml`; scope in `sonar-project.properties`; #2630).
It exists because the local gates catch what a lint can catch, review is one
person, and a deterministic multi-language sweep also covers trees the Rust
gates never see: shell scripts, workflow YAML, the website's JS/HTML/CSS,
Dockerfiles.

It is a **second opinion**. It is not authority, and it gates no merge.

## Precedence — a finding never outranks the sources

1. The vendored openEHR spec text (`docs/specs/openehr/`) — the oracle.
2. The hard rules: root `CLAUDE.md`, the crate `CLAUDE.md` files, `.claude/rules/*.md`.
3. The local gates: `cargo fmt`, `clippy`, `cargo nextest`, the CNF suite, the CI guards.
4. The analyzer.

A finding that contradicts a spec citation is wrong by construction — the
spec text is never a suspect (`spec-adherence.md`). A finding that asks for
something the rules forbid is wrong the same way. Nothing it reports relaxes
`testing.md`: never weaken a test, never adjust a CNF expectation, and never
edit a corpus fixture because a finding suggested it. A rule that contradicts
a deliberate recorded decision (the RFC 2008 rejection, zero re-exports, …)
is rejected as a class in the quality profile, with the reason recorded on
the triage program (#2640).

## New Code = since the last release (#2657)

The project's New Code definition is **"Previous version"**, anchored by
`sonar.projectVersion`: the sonar lane reads the workspace `version` from the
root `Cargo.toml` at scan time and appends it to the properties file — never
a second hand-maintained copy. The workspace version bumps at every release
cut (milestones are releases), so the quality gate's `new_*` conditions, PR
decoration, and the coverage program (#2656) all measure "since the last
release" — the same window the changelog section and the conformance baseline
describe. Do not switch the definition to days/reference-branch without
re-adjudicating that alignment.

## How Rust is analyzed

Rust is first-party in SonarQube Cloud (announced 2025-04-17): the analyzer
runs Clippy itself — 85 Clippy rules managed as quality-profile rules, plus
complexity metrics. That run uses the workspace DEFAULT features, a
deliberately independent second Clippy configuration beside our deny-tier
lanes; for pure Rust it mostly re-reports what the gates already enforce,
and its added value is the multi-language sweep, PR decoration on new code,
and a stable rule taxonomy to adjudicate class-by-class. Coverage IS wired
(owner directive 2026-08-24, reversing the #2630 deferral): the sonar lane
runs the instrumented nextest suites against a real PG18 and imports the
merged lcov via `sonar.rust.lcov.reportPaths`; the former ci.yml coverage
job and the badges-branch machinery are gone — the README's coverage badge
is Sonar's own, measured over the hand-written scan scope. SQL is excluded from scope entirely: the
PLSQL analyzer assumes Oracle and this tree's SQL is PostgreSQL (#2643).

## Sonar Architecture — adjudicated unavailable (2026-08-24, #2655)

The Architecture feature (current/intended architecture maps, deviations as
issues) is not in play here, for two independent reasons verified first-hand:
the organization is on the FREE plan and the feature requires Team/Enterprise
(`api/navigation/organization` → `"subscription":"FREE"`), and its language
support is C#/Java/JavaScript/Python/TypeScript — no Rust — so even a plan
upgrade would only ever cover the website's JS/TS. Re-evaluate if Sonar ships
Rust support for it AND the plan changes; the architecture record remains
`docs/architecture.md`, which no analyzer output outranks.

## It does not gate a merge, and it never writes

No quality gate blocks a merge. Findings worth acting on are written by
hand in a normal change — never applied through any UI that would attribute
a commit to a bot (the no-AI-attribution rule has no exceptions). Promotion
to a gating check would follow a precision measurement, the same bar every
reviewer here has been held to.

## False positives are data

Record a wrong finding on the triage program (#2640) rather than silencing
it; a scope or profile change is made when the scope is actually wrong,
never to make a number go down.

## History: CodeRabbit (removed 2026-08-24, #2638)

CodeRabbit (an LLM reviewer) held this role from #2142. Its measured
precision over 25 PRs was ~60% true positives (#2148) — kept advisory for
exactly that reason — and it was removed in favor of the deterministic
analyzer once SonarQube Cloud landed. The measurement record stays in those
closed issues. CodeQL continues to run separately as the security scanner
(`.github/workflows/codeql.yml`).

Official documentation (durable citations):
<https://docs.sonarsource.com/sonarqube-cloud/> ·
<https://www.sonarsource.com/knowledge/languages/rust/> ·
<https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/languages/rust>
