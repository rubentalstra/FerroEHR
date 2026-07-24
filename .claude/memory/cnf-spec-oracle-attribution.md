---
name: cnf-spec-oracle-attribution
description: Owner ruling 2026-07-24 — on a red CNF run the vendored spec is the ONLY oracle; the app is NEVER assumed correct; never "fix the CNF" or "check the SUT" to decide expectations; the catalogue must have TOTAL wire coverage
metadata:
  type: feedback
---

Owner ruling (2026-07-24, emphatic — "we run into this very very often"): when
CNF goes red, the recurring failure is agents assuming `crates/openehr-*` /
`app/ehrbase-*` are correct and "fixing the CNF" (editing the catalogue or
runner) to make it green, or saying "let me check our SUT" and treating the
server's observed behaviour as the reference. Both are banned.

**The law:** the vendored openEHR spec (`docs/specs/openehr/`) is the ONLY
oracle and is ALWAYS right. The three things WE built — the application
(`app/*` + `crates/openehr-*`), the runner (`tools/cnf-runner/src`), the
catalogue (`tools/cnf-runner/artifacts`) — are ALL suspects, none privileged. A
red row → read the spec first-hand, derive the required behaviour
independently, three-way compare (spec-required vs catalogue-expected vs
SUT-observed), attribute to ONE suspect, fix only that (delegate to the
`cnf-triage` agent).

- **The application is NEVER assumed correct** because "we wrote it to the
  spec" — it is the most common real culprit; the instrument exists to catch
  it. (But not always: the first live triage put 7/7 defects on the runner —
  attribute by evidence, never by default, in EITHER direction.)
- **"Let me check our SUT" to decide an expected value is banned** — the SUT
  response is EVIDENCE for the comparison, never the reference.
- **Coverage is a mandate, not just pass rate:** the catalogue must exercise
  EVERY wire behaviour the spec defines (operation, status branch, header,
  negotiation variant, precondition/error family, RM/AQL) as its own small
  isolated case; a behaviour with no case is a gap to close or an honest
  boundary to register, never silent.

**How it is enforced:** the always-loaded root CLAUDE.md CNF hard rule;
`.claude/rules/cnf-triage.md` (the attribution law) + the `cnf-triage` agent;
`.claude/rules/testing.md` §CNF coverage; the `/run-conformance` skill's
mandatory attribute-before-fix step; and `.claude/hooks/cnf_attribution_guard.sh`
(PreToolUse — injects the law when a catalogue expectation under
schedule/bindings/vocab is edited). See [[ecc-own-conformance-framework]],
[[vendored-corpora-fully-exercised]], [[owner-work-style]].
