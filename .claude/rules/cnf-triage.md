---
paths: ["tools/cnf-runner/**", "docs/conformance/**", "scripts/conformance*"]
---

# CNF failure triage (the attribution law)

When a CNF run goes red, the failure is attributed BEFORE anything is
changed. **The vendored openEHR spec text (`docs/specs/openehr/`) is always
right and is never a suspect** — every red row is a defect in something WE
wrote, in exactly one of three bins. Run the `cnf-triage` agent (or follow
this protocol in-session) for every red row; never "fix" a red run by
guessing which side is wrong.

## The three suspects (and the only fix path for each)

| Bin | What it means | Fix path |
|---|---|---|
| **Application** | The SUT violates the spec | `app/*`; for `crates/openehr-*` shapes: the `openehr-codegen` emitter + regeneration, never hand-edits or consumer workarounds |
| **Runner machinery** | The SUT behaved correctly but `tools/cnf-runner/src/**` misdrove the case or misjudged the response (driver, provisioning, resolver, outcome classification, comparator, verdicts) | Fix the runner module; the affected rows were inconclusive, not SUT failures |
| **Catalogue artifact** | The hand-authored schedule is wrong vs the spec (case core, operation binding, corpus, vocabulary) | Edit the artifact WITH a new spec-cited source for the corrected expectation |

## The protocol (per red row)

1. Read the observed wire exchange (results.json / transcript — what was
   actually sent, received, classified).
2. Read the case core + operation binding (what the catalogue expects, and
   its cited spec/OAS source).
3. Read the governing spec text FIRST-HAND (`/spec-lookup`; CNF schedule,
   ITS-REST + overview `Requests_and_responses`, SM interface, RM/QUERY).
   Never adjudicate from memory or from EHRbase behaviour.
4. Derive independently what a conformant server must return for that
   exchange; compare three-way: spec-required vs catalogue-expected vs
   SUT-observed. The mismatching side is the defect.
5. An attribution that changes application code carries a reproduced wire
   exchange (`curl` against the composed SUT) + the spec citation (file +
   section, decisive sentence quoted).
6. Spec silence/ambiguity → an `artifacts/registers/ambiguities.yaml` entry
   with a typed `disposition` — never a private resolution.

## Hard rules

- **Never edit `docs/specs/openehr/**` or adjust a catalogue expectation to
  match observed SUT behaviour** — expectations trace to spec text only.
- **A red run is not presumptive evidence of an app bug** (the first live
  triage attributed 7/7 defects to the runner) — nor of a runner bug.
  Every row gets the full derivation; no attribution without a citation.
- Transport faults / step-resolution failures classify as inconclusive
  runner-side rows, never SUT failures.
- Standing test discipline applies: never weaken a test or expectation to
  go green (`.claude/rules/testing.md`).
