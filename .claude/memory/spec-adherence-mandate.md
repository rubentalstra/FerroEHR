---
name: spec-adherence-mandate
description: "User mandates 100% openEHR spec/CNF adherence; always work from the vendored spec text at docs/specs/openehr, never memory or EHRbase behaviour"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 82894252-d336-4055-9108-633ee1c3b604
---

2026-07-06: the user was frustrated that implementation work was "not 1:1
following the openEHR specs and CNF". In response, the normative spec text +
CNF Platform Conformance Test Schedule (incl. the executable Robot suite) was
vendored at `docs/specs/openehr/` (script: `scripts/vendor-spec-docs.sh`), and
the whole `.claude/` setup (rules, skills, agents, hooks) was rebuilt around
it as the enforced oracle.

**Why:** conformance is the project's acceptance bar (ADR-008); answering
spec questions from memory or from EHRbase behaviour caused divergences.

**How to apply:** for ANY spec-facing behaviour, read the vendored section
first (`/spec-lookup`), cross-check the CNF test cases, cite file+section in
commits/PRs; use `/spec-audit` to check subsystems; hand subagents the
governing `docs/specs/openehr/...` paths. Never hand-edit that tree (a
PreToolUse hook blocks it). Related: [[greenfield-pivot-adr-008]].
