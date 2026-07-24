---
paths: ["crates/**", "app/**", "tools/cnf-runner/**", "scripts/conformance*", "docs/specs/**"]
---

# openEHR spec adherence (the vendored specs are the oracle)

The full normative openEHR spec text is vendored at **`docs/specs/openehr/`**
(pinned to `docs/VERSIONS.md`; index in its `README.md`). The openEHR
specifications are the conformance authority — not EHRbase, not memory, not
intuition.

**Hard rules:**

- **Before implementing or changing any spec-facing behaviour** (RM semantics,
  invariants, versioning, REST endpoints/status codes/headers, AQL semantics,
  canonical JSON/XML shapes, template/OPT handling, terminology), **read the
  relevant vendored spec section first** — grep the class/attribute/endpoint
  name under the owning component dir (map in `docs/specs/openehr/README.md`),
  or run `/spec-lookup <topic>`.
- **The CNF schedule + Robot suites are STALLED GUIDES, not the oracle (owner
  ruling 2026-07-24).** openEHR CNF never released a stable version, and the
  Robot suites/data sets are stalled/broken, so
  `docs/specs/openehr/CNF/docs/platform_test_schedule/` and
  `CNF/tests/platform/robot/` tell you WHICH behaviours to cover — they are NOT
  authority for the correct answer. Derive every enforceable expectation from
  the RELEASED spec component (RM / BASE / AM / QUERY / TERM / ITS-XML / SM /
  ITS-REST docs text); where the schedule or a Robot data set conflicts with a
  released spec, the RELEASED SPEC WINS, and an expectation with no
  released-spec ground is not enforceable.
- **The ITS-REST docs text is the wire oracle, NOT the OAS (owner ruling
  2026-07-24).** The vendored ITS-REST OAS is **stalled** — it is `emit-rest`
  codegen input only, never a behavioural oracle. Read
  `docs/specs/openehr/ITS-REST/` prose (esp. overview `Requests_and_responses`)
  for required wire behaviour; where the OAS and the docs text disagree, the
  docs text wins.
- **CNF red-run triage is spec-adjudicated** (`.claude/rules/cnf-triage.md`;
  the `cnf-triage` agent): when the CNF runner and the application disagree,
  the vendored spec text decides — it is always right and never a suspect.
  The failure is attributed to the application, the runner machinery, or the
  catalogue artifacts by three-way comparison against the spec text, never
  by assuming either side; no fix lands before the attribution, and every
  attribution carries the spec citation.
- **Cite the source:** conformance-relevant decisions name the spec file +
  section heading in the commit/PR description. A deliberate deviation or gap
  gets a `// NOTE:` with the spec reference and the reason.
- **Cite ONLY the vendored specs + official external docs — never an
  internal markdown file (owner hard rules, 2026-07-11 + 2026-07-17).**
  In code comments, SQL schema comments, and doc comments, justify
  behaviour by citing the openEHR spec file + section (`docs/specs/openehr/`)
  or official external documentation (the PostgreSQL docs, the Rust
  book/reference, the docs.rs/crates.io docs of a pinned crate) — never an
  internal doc, because internal docs move or die. **The ADR layer has been
  deleted** (it caused more confusion than value): no file may instruct anyone
  to read, write, or cite an ADR. Internal plan/design markdown is likewise
  never a citable authority — a plan or design file is deleted in the same PR
  that implements it, and the durable record is the closed issues + PR descriptions,
  `CHANGELOG.md`, git history, and the living reference docs
  (`docs/architecture.md`, `docs/endpoint-map.md`, `docs/VERSIONS.md`). Where
  the specs are SILENT on a decision (storage mechanics, indexing, infra,
  extension features), flag it explicitly: "no openEHR spec governs this — our
  own design/extension". Scrub any ADR or internal-doc citation from a file you
  touch.
- **Never resolve a spec question from EHRbase behaviour alone.** EHRbase is
  prior art; if it and the spec text disagree, the spec text wins and the
  divergence is worth a note.
- **Never hand-edit `docs/specs/openehr/**`** (except its top-level README) —
  re-vendor with `scripts/vendor-spec-docs.sh`; version pins live in that
  script and `docs/VERSIONS.md`.
- Subagents doing spec-facing work must be handed the relevant
  `docs/specs/openehr/...` paths in their prompt, and reviewers verify claims
  against those files.
