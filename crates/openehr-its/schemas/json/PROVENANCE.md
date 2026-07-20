# Vendored ITS-JSON schemas (canonical-JSON validation oracle)

JSON Schemas from `openEHR/specifications-ITS-JSON`, vendored **verbatim** (full
upstream `components/` tree).

Source repo: https://github.com/openEHR/specifications-ITS-JSON
Pinned commit: `5acae056248e917a4b4c56f7e712f4fcfeb616a6` (master; ITS-JSON is
DEVELOPMENT status with no numbered release — this is the latest available)
Fetched: 2026-07-04.

## Layout (verbatim `components/`)

- `RM/Release-1.0.3/`, `RM/Release-1.0.4/`, `RM/Release-1.1.0/`
- `BASE/Release-1.1.0/`
- `AM/Release-1.4/`, `AM/Release-2.1.0/`, `AM/Release-2.2.0/`
- Per package: individual `<TYPE>.json` files + a package `main.json`.
- Per component/version: a consolidated `openehr_<component>_<version>_all.json`
  at the `components/` root (e.g. `openehr_rm_1.1.0_all.json`).

784 files total. Schemas use `if…then` for `_type` polymorphism and organize by
package (draft-07).

## Role

**Validation oracle only** — not a code source. The JSON *model* is the
BMM-generated RM types with the native `_type`
self-tagging; there are no JSON structs to generate. `openehr-its::json` reads
`openehr_rm_1.1.0_all.json` (via `include_str!`) to validate canonical-JSON
output in the fidelity gate (`tests/`).

## Known version divergence (accepted Stage-1 parity nuance)

ITS-JSON tops out at RM 1.1.0 while our generated RM is 1.2.0 (from BMM). The
schema validates against 1.1.0-era shapes; this is a documented parity
consideration (see `docs/VERSIONS.md`), not something to reconcile here.
