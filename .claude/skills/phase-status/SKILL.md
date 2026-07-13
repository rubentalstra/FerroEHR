---
name: phase-status
description: >
  Prints the current phase pointer, the current phase file's unchecked
  tasks, and a short git status. Use when the user asks "where are we",
  "what phase are we on", or at the start of a work session to orient.
allowed-tools: [Read, Bash]
argument-hint: (none)
---

# /phase-status

A fast orientation dump — step 1 of the six-step loop
(`CLAUDE.md` "Phase workflow"). Read-only; makes no changes. The live
state below is injected at invocation time — ground the answer in it, not
in stale conversation memory.

## Live state (injected)

### docs/plans/current-phase.md

```!
cat "${CLAUDE_PROJECT_DIR}/docs/plans/current-phase.md"
```

### Worklist open items (if present)

```!
[ -f "${CLAUDE_PROJECT_DIR}/docs/plans/WORKLIST.md" ] && grep -E '^\|' "${CLAUDE_PROJECT_DIR}/docs/plans/WORKLIST.md" | head -30 || echo "(no WORKLIST.md)"
```

### Git

```!
cd "${CLAUDE_PROJECT_DIR}" && git status --short | head -40 && echo "---" && git log --oneline -5
```

## Steps

1. Summarize the pointer: which phase/worklist item is active and what the
   stated next action is, quoting `current-phase.md` verbatim where it
   matters.
2. If `current-phase.md` references a specific phase file
   (`docs/plans/phase-NN-*.md` or a `w*-*.md` plan), **Read it** and list
   every unchecked `- [ ]` line under its Tasks and Exit criteria, in
   order, so it is clear how close the phase is to done.
3. Summarize the git state from the injected output: current branch work,
   uncommitted files, last commits — flag uncommitted work that looks
   finished (the owner rule: never leave finished work sitting unmerged).
4. **Do not** modify `current-phase.md`, any phase file, or make any commit
   — this is a read-only status check. If the user wants the next task
   turned into a work plan, point them at `/next-task` instead.
