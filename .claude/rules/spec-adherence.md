---
paths: ["crates/**", "scripts/conformance*", "docs/specs/**"]
---

# openEHR spec adherence (the vendored specs are the oracle)

The full normative openEHR spec text is vendored at **`docs/specs/openehr/`**
(pinned to `docs/VERSIONS.md`; index in its `README.md`). Per ADR-008 the
openEHR specifications are the conformance authority — not EHRbase, not
memory, not intuition.

**Hard rules:**

- **Before implementing or changing any spec-facing behaviour** (RM semantics,
  invariants, versioning, REST endpoints/status codes/headers, AQL semantics,
  canonical JSON/XML shapes, template/OPT handling, terminology), **read the
  relevant vendored spec section first** — grep the class/attribute/endpoint
  name under the owning component dir (map in `docs/specs/openehr/README.md`),
  or run `/spec-lookup <topic>`.
- **Cross-check the CNF schedule:** if the behaviour is covered by
  `docs/specs/openehr/CNF/docs/platform_test_schedule/` (or the Robot suites
  under `CNF/tests/platform/robot/`), the implementation must satisfy those
  test cases — exact status codes, headers, and payload shapes. When in doubt,
  the CNF test case wins over a plausible reading of prose.
- **Cite the source:** conformance-relevant decisions name the spec file +
  section heading in the commit/PR description. A deliberate deviation or gap
  gets a `// PORT NOTE:` with the spec reference and the reason.
- **Spec citations ONLY — never ADR citations (owner hard rule, 2026-07-11).**
  In code comments, SQL schema comments, doc comments, and PORT NOTEs, justify
  behaviour by citing the openEHR spec file + section — never `ADR-NNN`. ADRs
  get superseded and leave stale statements in code; spec citations remain
  findable when a spec bumps. Where the specs are SILENT on a decision
  (storage mechanics, indexing, infra, extension features), flag it
  explicitly: "no openEHR spec governs this — our own design/extension".
  Scrub ADR citations from any file you touch.
- **Never resolve a spec question from EHRbase behaviour alone.** EHRbase is
  prior art; if it and the spec text disagree, the spec text wins and the
  divergence is worth a note.
- **Never hand-edit `docs/specs/openehr/**`** (except its top-level README) —
  re-vendor with `scripts/vendor-spec-docs.sh`; version pins live in that
  script and `docs/VERSIONS.md`.
- Subagents doing spec-facing work must be handed the relevant
  `docs/specs/openehr/...` paths in their prompt, and reviewers verify claims
  against those files.
