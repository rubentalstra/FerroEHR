//! The official openEHR CKM template pack.
//!
//! Five templates vendored as CKM's own OPT exports under `templates/ckm/`
//! (`scripts/vendor-ckm-templates.sh`; see `templates/ckm/PROVENANCE.md`), each
//! paired with a committed **example skeleton** obtained once from the composed
//! server's `GET /definition/template/adl1.4/{id}/example` and committed
//! byte-identical, so every benchmarked SUT receives the same request payload.
//!
//! Fairness rule: the skeleton is a committed artefact and is **never** fetched
//! from a SUT at run time — fetching `/example` per-SUT would break the
//! request-identity guarantee. This module only reads the vendored files and
//! hands the raw OPT XML
//! and the parsed skeleton to [`crate::render`] / [`crate::drive`].
//!
//! NOTE: no openEHR spec governs the benchmark template selection. The
//! pack membership and the `template_id`s below are stable vendored facts from
//! CKM (PROVENANCE.md's table); they are hardcoded as constants because a CKM
//! OPT export registers under a fixed `template_id`.

use serde_json::Value;

use crate::{BenchError, TemplateKind};

/// The pack directory, anchored to the crate manifest so the path is absolute
/// at compile time and never resolves against the process CWD (mirrors how the
/// vendored CNF corpus is anchored in [`crate::sutclient::fixtures`]).
const PACK_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/ckm");

/// One CKM template: its workload [`TemplateKind`], the CKM slug, the
/// `template_id` its OPT registers under (the wire identity on every commit),
/// and the vendored OPT + example-skeleton file basenames.
#[derive(Debug, Clone, Copy)]
pub struct CkmTemplate {
    /// The workload kind this template fills.
    pub kind: TemplateKind,
    /// The CKM slug (`vital-signs`, …) — the stable pack identifier.
    pub slug: &'static str,
    /// The `template_id` the OPT registers under (PROVENANCE.md, verbatim).
    pub template_id: &'static str,
    /// The vendored OPT file basename under `templates/ckm/`.
    opt_file: &'static str,
    /// The committed example-skeleton file basename under `templates/ckm/`.
    example_file: &'static str,
}

impl CkmTemplate {
    /// Read the raw OPT 1.4 XML text (for a provisioning upload).
    ///
    /// # Errors
    /// [`BenchError::Io`] if the vendored OPT file cannot be read.
    pub fn opt_text(&self) -> Result<String, BenchError> {
        Ok(std::fs::read_to_string(format!(
            "{PACK_DIR}/{}",
            self.opt_file
        ))?)
    }

    /// Read and parse the committed example composition skeleton.
    ///
    /// # Errors
    /// [`BenchError::Io`] if the file cannot be read, [`BenchError::Json`] if it
    /// does not parse as JSON.
    pub fn skeleton(&self) -> Result<Value, BenchError> {
        let text = std::fs::read_to_string(format!("{PACK_DIR}/{}", self.example_file))?;
        Ok(serde_json::from_str(&text)?)
    }

    /// The stable `workload.lock` source descriptor for this template
    /// (`ckm:<slug>|<template_id>|<example_file>|sha256:<skeleton-hash>`). The
    /// skeleton **content hash** is part of the descriptor: the payload bytes
    /// are workload model, so regenerating a skeleton (e.g. an
    /// empty→populated payload fix) shifts the lock and two runs with different
    /// payloads can never be conflated as the same workload. Falls back to
    /// `sha256:unreadable` if the file cannot be read (the run will fail later
    /// with the real I/O error; the lock stays total).
    #[must_use]
    pub fn source_descriptor(&self) -> String {
        use sha2::{Digest, Sha256};
        let hash = std::fs::read(format!("{PACK_DIR}/{}", self.example_file)).map_or_else(
            |_| "unreadable".to_owned(),
            |bytes| {
                let mut h = Sha256::new();
                h.update(&bytes);
                h.finalize()
                    .iter()
                    .fold(String::with_capacity(64), |mut out, b| {
                        use std::fmt::Write as _;
                        let _ = write!(out, "{b:02x}");
                        out
                    })
            },
        );
        format!(
            "ckm:{}|{}|{}|sha256:{hash}",
            self.slug, self.template_id, self.example_file
        )
    }
}

/// The pack, in a stable order (the `template_id`s are the PROVENANCE.md table
/// values, verbatim).
const PACK: [CkmTemplate; 5] = [
    CkmTemplate {
        kind: TemplateKind::CkmVitalSigns,
        slug: "vital-signs",
        template_id: "Vital signs",
        opt_file: "vital-signs.opt",
        example_file: "vital-signs.example.json",
    },
    CkmTemplate {
        kind: TemplateKind::CkmLabResult,
        slug: "generic-lab-test-result",
        template_id: "Generic lab test result example simple",
        opt_file: "generic-lab-test-result.opt",
        example_file: "generic-lab-test-result.example.json",
    },
    CkmTemplate {
        kind: TemplateKind::CkmMedicationOrder,
        slug: "eprescription-fhir",
        template_id: "ePrescription (FHIR)",
        opt_file: "eprescription-fhir.opt",
        example_file: "eprescription-fhir.example.json",
    },
    CkmTemplate {
        kind: TemplateKind::CkmSummary,
        slug: "international-patient-summary",
        template_id: "International Patient Summary",
        opt_file: "international-patient-summary.opt",
        example_file: "international-patient-summary.example.json",
    },
    CkmTemplate {
        kind: TemplateKind::CkmSynopsis,
        slug: "gp-data-set",
        template_id: "GP data set",
        opt_file: "gp-data-set.opt",
        example_file: "gp-data-set.example.json",
    },
];

/// The five CKM [`TemplateKind`]s, in pack order.
pub const KINDS: [TemplateKind; 5] = [
    TemplateKind::CkmVitalSigns,
    TemplateKind::CkmLabResult,
    TemplateKind::CkmMedicationOrder,
    TemplateKind::CkmSummary,
    TemplateKind::CkmSynopsis,
];

/// Every CKM template in the pack, in a stable order.
#[must_use]
pub fn all() -> &'static [CkmTemplate] {
    &PACK
}

/// The CKM template for a kind, or `None` for the CNF-corpus kinds (which are
/// sourced from the vendored CNF fixtures, not this pack).
#[must_use]
pub fn get(kind: TemplateKind) -> Option<CkmTemplate> {
    PACK.iter().copied().find(|t| t.kind == kind)
}

/// Whether a kind is sourced from the CKM pack.
#[must_use]
pub fn is_ckm(kind: TemplateKind) -> bool {
    get(kind).is_some()
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn pack_has_the_five_ckm_templates() {
        assert_eq!(all().len(), 5);
        assert_eq!(KINDS.len(), 5);
        for tpl in all() {
            assert!(is_ckm(tpl.kind), "{:?} must be a CKM kind", tpl.kind);
            assert!(KINDS.contains(&tpl.kind));
        }
    }

    #[test]
    fn template_ids_match_provenance() {
        // The PROVENANCE.md "template_id" column, verbatim — the id the OPT
        // registers under and the wire identity on every commit.
        let expected = [
            (TemplateKind::CkmVitalSigns, "Vital signs"),
            (
                TemplateKind::CkmLabResult,
                "Generic lab test result example simple",
            ),
            (TemplateKind::CkmMedicationOrder, "ePrescription (FHIR)"),
            (TemplateKind::CkmSummary, "International Patient Summary"),
            (TemplateKind::CkmSynopsis, "GP data set"),
        ];
        for (kind, template_id) in expected {
            let tpl = get(kind).expect("CKM kind resolves");
            assert_eq!(tpl.template_id, template_id, "template_id for {kind:?}");
        }
    }

    #[test]
    fn every_template_loads_its_opt_and_skeleton() {
        for tpl in all() {
            let opt = tpl.opt_text().expect("OPT reads");
            assert!(opt.contains("template"), "{} OPT looks like XML", tpl.slug);
            let skeleton = tpl.skeleton().expect("skeleton parses");
            assert!(
                skeleton.is_object(),
                "{} skeleton is a JSON object",
                tpl.slug
            );
            // The template_id must be present in the skeleton's archetype_details
            // (the composition commit's wire identity — verify, do not reinvent).
            let embedded = skeleton
                .pointer("/archetype_details/template_id/value")
                .and_then(Value::as_str)
                .expect("skeleton carries a template_id");
            assert_eq!(
                embedded, tpl.template_id,
                "{} skeleton template_id matches the OPT id",
                tpl.slug
            );
        }
    }

    #[test]
    fn corpus_kinds_are_not_ckm() {
        for kind in [
            TemplateKind::Vitals,
            TemplateKind::Nested,
            TemplateKind::Persistent,
        ] {
            assert!(!is_ckm(kind), "{kind:?} is an ECC-corpus kind");
            assert!(get(kind).is_none());
        }
    }

    #[test]
    fn source_descriptors_carry_the_slug() {
        let d = get(TemplateKind::CkmVitalSigns)
            .expect("resolves")
            .source_descriptor();
        assert!(d.starts_with("ckm:vital-signs|"), "{d}");
    }
}
