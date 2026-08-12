# Vendored reference models + BMM/ODIN test fixtures

Test-only assets used to cross-validate `openehr-adl` (and the
`openehr_lang::odin` reader) against openEHR's own reference implementation.
The ADL2 conformance corpus (`../adl2-reference/`, `../flattener/`) exercises
the parser + validator; these files supply the **reference models** those
fixtures are authored against, plus the reference BMM/ODIN test inputs.

**Reference input for the test harnesses only** — never a codegen input
(codegen consumes `tools/openehr-codegen/vendor/bmm/*.bmm.json`), never
hand-edited (added, never modified). The reference-model validation harness is
`tests/corpus_validity_rm.rs`.

## Source

- **Repo:** https://github.com/openEHR/archie
- **Commit:** `e8d92f28aca33f92ea08a826ea19f9581d579720` (2026-07-08)
- **archie code license:** Apache-2.0 (`NOTICE`: "Copyright 2015 Nedap
  Healthcare"; the ADL grammar is Thomas Beale's, Apache-2.0).

## `referencemodels/bmm/` — reference-model schemas (48 BMM files)

The complete openEHR-maintained BMM reference-model set archie ships, mirrored
from `referencemodels/src/main/resources/bmm/`, preserving the
`<publisher>/<model>/Release-<v>/BMM/` layout:

- `openEHR/` (40 files) — the openEHR RM/BASE/AM/LANG/PROC schemas across
  releases, incl. `adl_test/…/openehr_adltest_100.bmm` (the `TEST_PKG` test
  schema the corpus `openEHR-TEST_PKG-*` fixtures target) and the `RM` data
  types (`openehr_rm_data_types_1.0.4`) that `adltest` `includes`. openEHR
  Foundation, Apache-2.0 (per each file's own header).
- `CDISC/` (1) — CDISC BRIDG model (backs the `CDISC-Bridg-*` corpus
  fixtures). CDISC content; the BMM serialisation ships in archie under its
  Apache-2.0 distribution.
- `CIMI/` (3) — CIMI RM core/clinical/foundation (backs `CIMI-CORE-*`).
- `FHIR/` (1) — HL7 FHIR DSTU resources model. HL7 content.
- `ISO_13606/` (2), `ISO_21090/` (1) — the ISO EN 13606 and ISO 21090
  datatypes models. ISO/CEN content, redistributed by archie.

The non-openEHR models (CDISC/FHIR/ISO) carry the copyright of their
originating SDOs; they are vendored here solely as the reference models the
corpus fixtures declare, for conformance cross-validation, exactly as archie
redistributes them.

## Related fixtures vendored elsewhere

The archie BMM-reader test schemas (`bmm/src/test/resources/`) and ODIN-reader
test fixtures (`odin/src/test/resources/`) test the `openehr_lang` BMM/ODIN
readers, not the ADL2 RM checks, so they are vendored with that crate at
`crates/openehr-lang/tests/vendor/{bmm,odin}/` (see the PROVENANCE.md there).

## Licence election: the ISO 13606 BMM models are taken under MPL 1.1

Three files here carry the Mozilla tri-licence block —
`cen_EN13606_0.95.bmm`, `cen_ts14796_0.90.bmm`, and
`openehr_ehr_extract_999.bmm`:

```
--| Version: MPL 1.1/GPL 2.0/LGPL 2.1
--| Alternatively, the contents of this file may be used under the terms of
--| either the GNU General Public License Version 2 or later (the 'GPL'), or
--| the GNU Lesser General Public License Version 2.1 or later (the 'LGPL'),
--| in which case the provisions of the GPL or the LGPL are applicable instead
--| of those above.
```

A recipient elects ONE of the three. **This project takes them under
MPL 1.1**, so no GPL or LGPL obligation attaches to anything here. Recorded
because a licence scanner reads the GPL and LGPL names out of the text and
reports them as findings; the election is the answer, and writing it down once
stops it being re-litigated at every scan.

These files are test corpus. They are not packaged by any crate — `openehr-adl`
ships `src/**` only, and Cargo excludes `tests/` from a published package.
