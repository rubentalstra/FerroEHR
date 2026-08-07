# The AI reviewer (CodeRabbit) — what it is and what it is not

Every pull request here is reviewed by CodeRabbit, configured entirely by
the committed `.coderabbit.yaml` (issue #2142). It exists because the local
gates catch what a lint can catch, while the properties this repository
actually turns on — a hard rule bent, a spec citation that does not support
the behaviour it justifies, a defect swallowed into an `Option` — are
marked "review-enforced" in `reliability.md`, and review is one person.

It is a **second opinion**. It is not authority, not a gate, and not a
committer.

## Precedence — a finding never outranks the sources

1. The vendored openEHR spec text (`docs/specs/openehr/`) — the oracle.
2. The hard rules: root `CLAUDE.md`, the crate `CLAUDE.md` files, `.claude/rules/*.md`.
3. The local gates: `cargo fmt`, `clippy`, `cargo nextest`, the CNF suite, the CI guards.
4. The reviewer.

A finding that contradicts a spec citation is wrong by construction — the
spec text is never a suspect (`spec-adherence.md`). A finding that asks for
something the rules forbid is wrong the same way. Nothing it says relaxes
`testing.md`: never weaken a test, never adjust a CNF expectation, and never
edit a corpus fixture because a review comment suggested it.

## It never writes

Every commit-producing recipe is disabled in `.coderabbit.yaml` —
`docstrings`, `unit_tests`, `simplify`, `autofix`, `fix_ci`,
`resolve_merge_conflict`. **Do not apply a committable suggestion through
the GitHub UI**: GitHub attributes the resulting commit to the bot with a
co-author trailer, which the no-AI-attribution hard rule forbids without
exception. If a suggestion is right, write the change yourself.

The installed app does hold `code: write` — a GitHub App's permission set is
declared by the app and cannot be narrowed by the installer — so this
property is a configuration choice, not a capability boundary.

## It does not gate a merge

No pre-merge check is at `error`, and `request_changes_workflow` is off. The
app publishes a check run named `CodeRabbit`; it is **not required**, and
making it required is a deliberate future step, not something to inherit by
accident. Merge authority stays where it was: the local gates green, and the
owner's call.

A pre-merge check reported as a warning is information. It does not block,
and it is not a reason to reword a PR description that is already correct.

## The configuration lives in git, and only the base branch's copy runs

`.coderabbit.yaml` at the repository root is the whole configuration; the
web dashboard holds nothing (verify with `@coderabbitai configuration` on
any PR — it prints the resolved settings annotated with their source).

Two mechanics to know before editing it:

- **A config change cannot be previewed.** On public repositories CodeRabbit
  ignores the branch's copy and applies the base branch's: *"For security,
  only the configuration from the base branch is applied for open source
  repositories."* An edit takes effect only once merged.
- **Validate before committing** against
  <https://coderabbit.ai/integrations/schema.v2.json>; an invalid key is
  accepted silently by git and simply never applies.

## The rules are the source; the config points at them

`knowledge_base.code_guidelines` reads `**/CLAUDE.md`, `.claude/rules/*.md`,
`docs/architecture.md` and `docs/VERSIONS.md`, so a rule change reaches the
reviewer with no configuration edit. The `path_instructions` in
`.coderabbit.yaml` are per-path reminders of what to look for — never a
second copy of a rule. When the two disagree, the rule file is right and the
instruction is the defect.

## False positives are data

The reviewer reads the rule text itself, so it will occasionally flag prose
that *describes* a rule as if it violated one. Record such a finding on the
measurement issue rather than silencing it; an instruction is only edited
when the instruction is actually wrong, never to make a comment go away.

Official documentation (durable citations):
<https://docs.coderabbit.ai/configure-coderabbit/> ·
<https://docs.coderabbit.ai/reference/yaml-template>
