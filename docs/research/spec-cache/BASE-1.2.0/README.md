# BASE Release-1.2.0 spec cache

AsciiDoc sources of the openEHR BASE component specifications, cached so
transcription (P1 and later phases) reads the exact released text instead of
memory or a moving web page.

- Source repo: `github.com/openEHR/specifications-BASE`
- Ref: tag `Release-1.2.0`, commit `906441385b7c6cb54f1e281f7417a48381c5f057`
- Fetched: 2026-07-02
- Layout:
  - `foundation_types/` — chapter masters (primitives, structures, interval,
    time, terminology, functional, type cross-reference)
  - `base_types/` — chapter masters (definitions, builtins, identification)
  - `resource/` — resource package chapter
  - `uml_classes/` — 78 per-class definition tables (attributes, functions,
    invariants); these are the transcription ground truth the chapters
    include by reference
- Published rendering: https://specifications.openehr.org/releases/BASE/Release-1.2.0

These files are upstream openEHR specification content (openEHR
specifications are published under CC-BY-ND); cached verbatim for reference
during the port, never edited. When a later phase needs another component
(RM 1.1.0, AM 2.3.0, QUERY 1.1.0, …), add a sibling directory with the same
provenance header.
