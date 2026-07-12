---
name: ecc-own-conformance-framework
description: "2026-07-08 owner pivot — conformance is OUR framework (ECC), legacy openEHR CNF is reference reading only; no mapping/Robot/Python ever"
metadata: 
  node_type: memory
  type: project
  originSessionId: 6e259293-e623-4384-b476-748dce5b3ab2
---

The conformance instrument is the **ehrbase-rs Conformance Catalogue (ECC)**
— our own framework, design v4 in `docs/design/conformance-framework.md`,
plan `docs/plans/s2-phase-05-cnf-engine-rewrite.md`, branch
`claude/cnf-hardening`.

**Why:** the official openEHR CNF corpus is frozen/unmaintained upstream
(dormant since 2024-08, stub chapters, 2019 Robot/Python harness). Owner
directives (emphatic, repeated): own numbering (`ECC-<AREA>-<NNN>[.VV]`,
committed `inventory/ecc-catalog.tsv`, numbers never reused), own taxonomy
(15 areas), spec-first case universe from the *pinned current specs*,
generated data sets (not copied 2019 fixtures), version-aware model
(`SpecVersions`, latest-only supported today), ≥2,000 tests at build-out,
enterprise-clean layered crate (`model/testdata/engine/reporting/suites` +
facade in `ehrbase-conformance`). **Never** build mapping/trace machinery to
legacy CNF ids, never parse Robot files as framework machinery — vendored
CNF is design-time reading + input payloads only.

**How to apply:** any conformance work goes through the ECC catalogue +
guards (`REGEN_CATALOG=1` to allocate); reports are catalogue-driven; keep
the crate clippy-clean; check the v4 build-out list in the phase file before
starting. Related: [[spec-adherence-mandate]], [[greenfield-pivot-adr-008]].
