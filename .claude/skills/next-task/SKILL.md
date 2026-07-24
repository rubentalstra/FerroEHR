---
name: next-task
description: >
  Reads the tracker (GitHub Issues), picks the pinned/top open issue (or the
  issue the user names), and restates it as a concrete in-session work plan
  naming the files and crates involved. Use when the user asks "what's next"
  or "what should I work on".
allowed-tools: [Read, Grep, Glob, Bash]
argument-hint: "[issue number] (optional)"
---

# /next-task

Turns an open tracker issue into an actionable plan — the planning step of
the issue workflow (`CLAUDE.md`). Does not do the work itself; that is a
separate step the caller takes after seeing the plan.

## Steps

1. **Read the tracker**: `gh issue list --state open` (the SessionStart dump
   annotates each issue with `{k/n}` sub-issue progress, `child-of #parent`,
   and open `BLOCKED-by`/`blocks` edges) — take the pinned issue (current
   focus) or the issue the user named. **Respect relationships**
   (`.claude/rules/issue-relationships.md`): do NOT pick an issue shown
   `BLOCKED-by` an open issue (surface its blocker as the real next task
   instead); for a parent issue, point at its next open child rather than the
   parent itself. Then `gh issue view <n> --comments` for the full contract
   (`## Contract` + `## Exit criteria`) and the running discussion, and
   `scripts/gh-rel.sh tree <n>` for its parent/children/blockers. If the issue
   links a plan file (`docs/plans/*.md`), read that too; its unchecked
   (`- [ ]`) tasks are the queue.
2. **Turn the task into a plan**, stating:
   - **What** the task requires, in one or two sentences.
   - **Which files** are involved — search for them (Grep/Glob under
     `crates/` and `app/`) rather than guessing paths; if the task names a
     spec component, resolve it against `docs/architecture.md` (the crate map)
     and `docs/specs/openehr/README.md`.
   - **Which mechanism** applies:
     **openEHR spec/ITS layer** (`openehr-base`/`openehr-rm`/`openehr-am`/
     `openehr-its`) → **the code generator** — change `openehr-codegen`'s
     emitter and regenerate (`/regen-codegen`).
     **Application** (`ehrbase`/`ehrbase-rest`) → idiomatic
     modern Rust of our own design on the generated crates, the openEHR
     specs as the authority (EHRbase = prior art only). Build
     compiling + tested. Note whether the task suits fanning out to an
     `implementer`/`ui-implementer` subagent per the CLAUDE.md
     Model-orchestration section (max 2 concurrent), or belongs in-session
     (architecture, the AQL IR/codec core, spec-conformance judgement).
   - **Which spec sections govern it** — for any spec-facing task, name the
     `docs/specs/openehr/...` files (and CNF test-schedule chapters) the
     implementation must be read against, per `spec-adherence.md` /
     `/spec-lookup`. Doing the work starts by reading those.
   - **What "done" looks like** for this task specifically — the issue's
     `## Exit criteria` checklist, plus what proves it: the CNF pipeline's
     zero-drift gate (`docs/conformance/ehrbase-rs/results.json` +
     `verdicts.json`), the `openehr-its` fidelity gates, or corpus tests.
3. **Do not edit the issue or commit** — recording progress happens after
   the work is actually done, not as part of planning it.
