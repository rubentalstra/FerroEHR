<!-- Describe the change itself. No AI/tool attribution anywhere in this PR. -->

## What this changes

Closes #<!-- tracker issue number — the merge into main auto-closes it -->

## Licensing of contributions

- [ ] I accept the terms in [CONTRIBUTING.md § Licensing of contributions](../CONTRIBUTING.md#licensing-of-contributions): I have the right to submit this work, I license it under the project licence of the version it lands in, and I grant the Licensor the relicensing right stated there.

## Checks

- [ ] `cargo fmt --all --check` · `cargo clippy --workspace --all-targets` · `cargo nextest run --workspace`
- [ ] No test weakened, skipped, or edited to route around a bug
- [ ] User-visible change → `CHANGELOG.md [Unreleased]` entry (else the `no-changelog` label)
- [ ] REST/config/CLI/deployment change → matching `website/book/src` page updated
- [ ] Implemented `docs/plans/*.md` plan file deleted (unless another open issue still consumes it)
- [ ] New issues opened from this work are linked as GitHub relationships (sub-issue / blocked-by via `scripts/gh/rel.sh`), not prose (`.claude/rules/issue-relationships.md`)

<!--
HARD RULE: this PR description, its title, and all commits must contain NO
AI/Claude attribution (no "Co-authored-by: Claude", no "Generated with Claude
Code", no robot emoji or 🤖). Describe only the change.
-->
