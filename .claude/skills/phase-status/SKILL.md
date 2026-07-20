---
name: phase-status
description: >
  Prints the tracker's open issues (pinned first), any linked plan file's
  unchecked tasks, and a short git status. Use when the user asks "where are
  we", "what's in flight", or at the start of a work session to orient.
allowed-tools: [Read, Bash]
argument-hint: (none)
---

# /phase-status

A fast orientation dump — step 1 of the issue workflow (`CLAUDE.md`).
Read-only; makes no changes. The live state below is injected at invocation
time — ground the answer in it, not in stale conversation memory.

## Live state (injected)

### The tracker — open GitHub issues

```!
gh issue list --state open --limit 100 --json number,title,labels,milestone --template '{{range .}}#{{.number}}  {{.title}}  [{{range $i, $l := .labels}}{{if $i}}, {{end}}{{$l.name}}{{end}}]{{if .milestone}}  ({{.milestone.title}}){{end}}{{"\n"}}{{end}}'
```

### Git

```!
cd "${CLAUDE_PROJECT_DIR}" && git status --short | head -40 && echo "---" && git log --oneline -5
```

## Steps

1. Summarize the tracker state: which issue is the current focus (pinned /
   milestone-assigned / the one the branch implements) and what its stated
   next action is. For the in-flight issue, run
   `gh issue view <n> --comments` and report the latest status comment and
   the unchecked `## Exit criteria` boxes.
2. If that issue links a plan file (`docs/plans/*.md`), **Read it** and list
   every unchecked `- [ ]` line under its Tasks and Exit criteria, in
   order, so it is clear how close the work is to done.
3. Summarize the git state from the injected output: current branch work,
   uncommitted files, last commits — flag uncommitted work that looks
   finished (the owner rule: never leave finished work sitting unmerged).
4. **Do not** modify any issue, plan file, or make any commit — this is a
   read-only status check. If the user wants the next task turned into a
   work plan, point them at `/next-task` instead.
