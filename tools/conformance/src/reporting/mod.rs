//! Result artefacts (per SUT, `CNF/docs/guide/master04-framework.adoc`
//! §Specifications): the Test Execution Report (`results.json` +
//! `CONFORMANCE_REPORT.md` + badges), the Conformance Statement, the
//! Conformance Certificate (our own SUT only — X1 fairness rule 4), and the
//! cross-SUT comparison matrix.

pub mod badges;
pub mod certificate;
pub mod compare;
pub mod report;
pub mod results;
pub mod statement;
