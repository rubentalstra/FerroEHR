---
name: cnf-spec-oracle-attribution
description: "Owner ruling 2026-07-24 — on a red CNF run the vendored spec is the ONLY oracle; the app is NEVER assumed correct; never \"fix the CNF\" or \"check the SUT\" to decide expectations; the catalogue must have TOTAL wire coverage"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 871531fb-1884-468e-9033-ae616ae2eb2b
  modified: 2026-07-26T18:49:42.185Z
---

Owner ruling (2026-07-24, emphatic — "we run into this very very often"): when
CNF goes red, the recurring failure is agents assuming `crates/openehr-*` /
`app/ferroehr-*` are correct and "fixing the CNF" (editing the catalogue or
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
- **Only the RELEASED spec components are the oracle; the CNF schedule and
  the Robot suites are STALLED guides** (owner 2026-07-24): adjudicate from
  the released components (RM / BASE / AM / QUERY / TERM / ITS-XML / SM /
  ITS-REST **docs text** — SM anchors operations + naming, ITS-REST the wire;
  an SM op the released ITS-REST doesn't realize is an SM↔ITS gap, not a
  REFUTE). The CNF Platform Conformance Test Schedule (never released stable)
  says WHICH behaviour to test, not the correct answer; the Robot suites/data
  sets are stalled/broken (e.g. AMB-47). Where any conflicts with a released
  component, the released component wins — which is precisely why we build
  the first *enforceable* CNF framework. See [[served-openapi-is-native]].
- **The OAS oracle order (owner ruling 2026-07-28, superseding the
  2026-07-24 never-an-oracle absolutism):** the vendored released OAS is
  part of the release's own specification artifacts (ITS-REST overview
  `Specifications.md`: "Specifications can be downloaded as YAML files in
  OpenAPI Specification 3.0 format") and **grounds an expectation where the
  docs text is SILENT** — cited AS the OAS (file + element), never read
  beyond what it states (an optional schema member is not a presence
  requirement). It **loses to the docs text on every conflict** (that half
  of 2026-07-24 stands), and only both-silent behaviour goes to the
  register. First application: AMB-160 (System OPTIONS manifest). The
  register-wide re-adjudication of pre-ruling OAS-silence dispositions is
  its own tracked program.
- **An OAS-vs-docs-text disagreement is NEVER an ambiguity (owner ruling
  2026-07-26, group-3 audit):** no `ambiguities.yaml` entry may exist whose
  only conflict is the OAS disagreeing with the released text — the docs
  text wins SILENTLY, and citing the OAS as one half of a "conflict"
  launders a subordinate source into the record. A register entry is
  legitimate only for released-vs-released conflicts (e.g. RM vs ITS docs
  text) or silence in both the docs text and the OAS. Seven such
  pseudo-AMBs were withdrawn from the #379 findings the day this was ruled.

**How it is enforced:** the always-loaded root CLAUDE.md CNF hard rule;
`.claude/rules/cnf-triage.md` (the attribution law) + the `cnf-triage` agent;
`.claude/rules/testing.md` §CNF coverage; the `/run-conformance` skill's
mandatory attribute-before-fix step; and `.claude/hooks/cnf_attribution_guard.sh`
(PreToolUse — injects the law when a catalogue expectation under
schedule/bindings/vocab is edited). See [[ecc-own-conformance-framework]],
[[vendored-corpora-fully-exercised]], [[owner-work-style]].
