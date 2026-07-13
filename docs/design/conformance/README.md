# Conformance framework design registers (W-10)

Spec-first registers for the redesign + rewrite of `tools/conformance`
(owner ruling 2026-07-13: the incrementally-grown instrument is not trusted;
rethink from the spec up). Method identical to `docs/design/platform/`
(W-3f, proven three times): each register's spine is the governing CNF
Platform Conformance Test Schedule chapter enumerated test-case-by-test-case
(with citation); the EXISTING ECC cases are mapped onto that spine
(conformant / divergent / missing / instrument-encodes-server-behaviour,
with `file:line` evidence); cases with no schedule home are flagged; G-rows
carry gaps + rulings. The target design (register 90) derives from the
spec's own decomposition, never from the legacy file layout.

Oracle: `docs/specs/openehr/CNF/` (pinned, `PROVENANCE.md` commit
`33251d2a`). Methodology: guide master03/04/05; profiles master03 (the
capability × CORE/STANDARD/OPTIONS matrix); certificate master03 (the
Statement/Certificate shapes). ECC law: own numbering/taxonomy, generated
data sets, never Robot/legacy mapping as machinery — the Robot suite is
coverage evidence + raw material only.

## Registers — schedule chapter → register

| # | Register | Schedule oracle | Existing suites mapped |
|---|---|---|---|
| 01 | [Definitions: ADL](01-definitions-adl.md) | master04-func_tc_definition_adl | `suites/definition_adl14.rs` |
| 02 | [Definitions: query](02-definitions-query.md) | master05-func_tc_definition_query | `suites/definition_query.rs` |
| 03 | [EHR](03-ehr.md) | master06-func_tc_ehr | `suites/ehr.rs` |
| 04 | [Composition](04-composition.md) | master07-func_tc_ehr_composition | `suites/composition.rs` |
| 05 | [Contribution](05-contribution.md) | master08-func_tc_ehr_contribution | `suites/contribution.rs` |
| 06 | [Directory](06-directory.md) | master09-func_tc_ehr_directory | `suites/directory.rs` |
| 07 | [Querying](07-querying.md) | master11-func_tc_querying | `suites/query.rs`, `suites/query_golden.rs` |
| 08 | [Demographic](08-demographic.md) | master10-func_tc_demographic | `suites/demographic.rs` |
| 09 | [Admin](09-admin.md) | master12-func_tc_admin | `suites/admin.rs` |
| 10 | [Messaging](10-messaging.md) | master13-func_tc_messaging | `suites/message.rs` |
| 11 | [Cross-cutting: security, signing, terminology, extensions](11-crosscutting.md) | profiles master03 non-functional; no schedule chapter (flagged) | `suites/security.rs`, `suites/signing.rs`, `suites/terminology.rs` |
| 12 | [Content: composition + entry](12-content-composition-entry.md) | master15 + master16 | `suites/content/*` |
| 13 | [Content: data types](13-content-data-types.md) | master17.1–17.7 | `suites/content/data_types.rs` |
| 80 | [Data-set strategy](80-data-sets.md) | schedule data-set classes × ECC generated-data law | `testdata/fixtures.rs`, `testdata/fixtures/REGISTER.md` |
| 90 | [Target design](90-target-design.md) | all of the above + guide/profiles/certificate | (architecture, orchestrator-owned) |

## Owner rulings (2026-07-13, session start)

- No named third-party SUT; multi-SUT = ehrbase-rs (compose default) +
  upstream EHRbase Java (Docker) + **bring-your-own-endpoint** (URL + auth
  config entry; full report for any CDR).
- **Version ladder:** versioned assertions carry per-edition/per-RM-version
  forms ordered newest→oldest; the runner tries the highest first, steps
  down, and records the satisfied level as an edition finding (never a
  silent pass; failure only when no supported form matches). Normative
  backing: schedule master03 §API Conformance ("supported RM version(s) …
  stated in the Conformance Statement; minimum required version is RM
  1.0.2").
