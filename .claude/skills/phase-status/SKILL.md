---
name: phase-status
description: >
  Prints the worklist's open rows, any open plan file's unchecked tasks,
  and a short git status. Use when the user asks "where are we", "what's
  in flight", or at the start of a work session to orient.
allowed-tools: [Read, Bash]
argument-hint: (none)
---

# /phase-status

A fast orientation dump — step 1 of the worklist workflow
(`CLAUDE.md`). Read-only; makes no changes. The live
state below is injected at invocation time — ground the answer in it, not
in stale conversation memory.

## Live state (injected)

### docs/plans/WORKLIST.md — open rows

```!
awk '/^## Closed/{exit} {print}' "${CLAUDE_PROJECT_DIR}/docs/plans/WORKLIST.md" 2>/dev/null || echo "(no WORKLIST.md)"
```

### Git

```!
cd "${CLAUDE_PROJECT_DIR}" && git status --short | head -40 && echo "---" && git log --oneline -5
```

## Steps

1. Summarize the pointer: which phase/worklist item is active and what the
   stated next action is, quoting the worklist rows verbatim where they
   matters.
2. If an open worklist row references a plan file
   (`docs/plans/phase-NN-*.md` or a `w*-*.md` plan), **Read it** and list
   every unchecked `- [ ]` line under its Tasks and Exit criteria, in
   order, so it is clear how close the phase is to done.
3. Summarize the git state from the injected output: current branch work,
   uncommitted files, last commits — flag uncommitted work that looks
   finished (the owner rule: never leave finished work sitting unmerged).
4. **Do not** modify the worklist, any plan file, or make any commit
   — this is a read-only status check. If the user wants the next task
   turned into a work plan, point them at `/next-task` instead.
