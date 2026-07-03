# openEHR spec cache

This directory stores immutable local copies of the official openEHR
specification sources used during transcription. Every cache directory must
name the component and version, record the upstream repo/ref/commit, and keep
the source text verbatim.

## Current caches

| Component | Version | Location | Provenance |
|---|---|---|---|
| BASE | 1.2.0 | `BASE-1.2.0/` | `openEHR/specifications-BASE`, tag `Release-1.2.0`, commit `906441385b7c6cb54f1e281f7417a48381c5f057` |
| RM | 1.1.0 | `RM-1.1.0/` | `openEHR/specifications-RM`, tag `Release-1.1.0`, commit `3cbd85b` |

## Related vendored computable specs

These are not AsciiDoc transcription caches, but they are pinned spec inputs
and should follow the same provenance rule:

| Component | Version | Location | Notes |
|---|---|---|---|
| TERM | 3.0.0 | `crates/openehr-terminology/assets/` | Terminology XML, external terminologies, PropertyUnitData, and XSDs |
| ITS-JSON | development | `crates/openehr-serde/schemas/openehr_rm_1.1.0_all.json` | Pinned commit `5acae056248e917a4b4c56f7e712f4fcfeb616a6` |
| Real-world canonical-JSON corpus | — | `crates/openehr-serde/tests/vendor/` | `ehrbase/openEHR_SDK` @ `22b01e0c99b53669394e56da29c2410838b5cf7e` (Apache-2.0); 72 canonical instances (composition/contribution/ehr/folder/item_structure). Full provenance + exclusions in `crates/openehr-serde/tests/vendor/PROVENANCE.md`. Used as the P4/P5 serde round-trip integration oracle. |

## Rule

Yes: every later spec component should be cached or vendored before it is
transcribed or used for validation. Do this at the first phase that consumes
the component, not ad hoc in implementation files.

Expected future siblings:

| Component | Target cache/vendor location | Consuming phase |
|---|---|---|
| ITS-XML RM 1.1.0 and legacy 1.0.2 XSDs | `crates/openehr-serde/schemas/its-xml-*` | P5 |
| LANG ODIN/BMM grammars | `docs/research/spec-cache/LANG-1.0.0/` or grammar-specific subdirs | P8 |
| AM 2.3.0 and OPT 1.4 XSDs | `docs/research/spec-cache/AM-2.3.0/` and `crates/openehr-adl/schemas/` | P9 |
| QUERY 1.1.0 AQL grammar | `docs/research/spec-cache/QUERY-1.1.0/` | P12 |
