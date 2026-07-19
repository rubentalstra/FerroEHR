# Vendored ODIN + BMM reader test fixtures

Test-only inputs for cross-validating the hand-written `openehr_lang::odin`
reader (and a future `openehr_lang` ODIN-BMM reader) against openEHR's
reference implementation (archie). **Reference input for tests only** — never a
codegen input (codegen consumes `crates/openehr-codegen/vendor/bmm/*.bmm.json`),
added, never edited.

## Source

- **Repo:** https://github.com/openEHR/archie
- **Commit:** `e8d92f28aca33f92ea08a826ea19f9581d579720` (2026-07-08)
- **License:** Apache-2.0 (archie `NOTICE`: "Copyright 2015 Nedap Healthcare";
  the ADL/ODIN grammar is Thomas Beale's, Apache-2.0).

## `odin/` — ODIN reader test fixtures (17 files)

Mirrored from `odin/src/test/resources/` — ODIN parse inputs (leaf values,
intervals, keyed lists, typed casts, error cases) for asserting
`openehr_lang::odin::parse` matches archie's ODIN reader.

## `bmm/` — BMM-reader test schemas (38 files)

Mirrored from `bmm/src/test/resources/` — BMM ODIN-serialisation test inputs
(`adltest`, `TestBmm*`, edge/malformed schemas). Reference material for a
future `openehr_lang` ODIN-BMM reader.
