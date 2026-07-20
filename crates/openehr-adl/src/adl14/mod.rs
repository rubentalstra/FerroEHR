//! ADL 1.4 → ADL 2 conversion (the `adl14` 1.4→2 upgrade pipeline).
//!
//! NOTE: **no openEHR spec governs 1.4 → 2 conversion — the entire `adl14`
//! module is our own design/extension.** The strategy was researched from
//! openEHR `archie`'s converter (`ADL14NodeIDConverter` +
//! `ADL14TermConstraintConverter` + `Differentiator`) as PRIOR ART only —
//! archie is never a parity target, and there is no vendored-spec authority to
//! cite for any rule here. The behaviour is pinned by the paired
//! `tests/corpus/adl2-reference/upgrade/upgrade_from_14/**` fixtures (each 1.4
//! `.adl` source paired with its expected ADL2 `.adls`), which are the
//! converter oracle.
//!
//! Pipeline:
//! 1. **Front end** — [`crate::assemble::parse_artefact_adl14`] parses a 1.4
//!    `.adl` into a *1.4-shaped* `openehr_am::am24` [`Archetype`] (at-code node
//!    ids; qualified/listed terminology constraints preserved verbatim in the
//!    `C_TERMINOLOGY_CODE.constraint` string; inline dADL `C_DV_QUANTITY`/
//!    `C_DV_ORDINAL` lowered to `DV_QUANTITY`/`DV_ORDINAL` with an attribute
//!    tuple). The same [`crate::cadl::Dialect::Adl14`] tolerance also feeds a
//!    (future) OPT-1.4 front end — the converter core takes the assembled
//!    `Archetype`, not raw text.
//!    TODO: add the `openehr_its::opt14` OPT-1.4 front end so stored 1.4 OPTs
//!    feed the same [`convert::convert`] core over the REST seam.
//! 2. **Converter core** ([`convert`]) — node-id renumbering (the `+1`
//!    first-segment shift, separate id-/at-code spaces, `0.`-prefixed
//!    new-at-level codes kept, missing ids synthesised in document order),
//!    terminology-constraint conversion (local single → at-code, local list →
//!    synthesised `ac` value set, external code(s) → synthesised at-code(s) +
//!    term-binding URIs), terminology rebuild, description/meta transform, and
//!    cardinality/occurrences elision.
//! 3. **Differ** ([`differ`]) — for a specialised 1.4 source, re-differentialise
//!    the converted child against its converted+flattened parent (strip
//!    inherited-unchanged nodes).
//! 4. **Conversion log** ([`log`]) — records every synthesised code/value set so
//!    a re-conversion consulting the same log yields identical codes.

pub mod convert;
pub mod differ;
pub mod log;
