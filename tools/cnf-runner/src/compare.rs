//! The ECC↔CNF comparison gate: relate every row of the old harness's
//! catalogue (`tools/conformance/inventory/ecc-catalog.tsv`) to the ground
//! the CNF 2.0 catalogue covers, and generate the committed comparison
//! report. Differences from the old baseline are expected — honesty lives in
//! the enumeration: every active ECC row must be mapped, every mapping must
//! resolve, and the report prints the remaining gap explicitly.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::artifacts::ArtifactSet;
use crate::ids::CaseId;

/// Comparison-layer error.
#[derive(Debug, Error)]
pub enum CompareError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// The ECC catalogue TSV is malformed.
    #[error("ecc-catalog.tsv line {line}: {message}")]
    Catalog { line: usize, message: String },
    /// The mapping file is malformed.
    #[error("ecc map: {0}")]
    Map(String),
}

/// One ECC catalogue row.
#[derive(Debug, Clone)]
pub struct EccRow {
    pub id: String,
    pub area: String,
    pub status: String,
    pub primary_ref: String,
    pub title: String,
}

/// Parse the committed ECC catalogue TSV (comment lines skipped).
///
/// # Errors
/// [`CompareError::Catalog`] on a malformed row.
pub fn parse_ecc_catalog(text: &str) -> Result<Vec<EccRow>, CompareError> {
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [id, area, status, primary_ref, title] = fields.as_slice() else {
            return Err(CompareError::Catalog {
                line: i + 1,
                message: format!("expected 5 tab-separated fields, got {}", fields.len()),
            });
        };
        rows.push(EccRow {
            id: (*id).to_owned(),
            area: (*area).to_owned(),
            status: (*status).to_owned(),
            primary_ref: (*primary_ref).to_owned(),
            title: (*title).to_owned(),
        });
    }
    Ok(rows)
}

/// How an old ECC row relates to the new catalogue (closed vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapDisposition {
    /// The ground is covered by the listed CNF cases (possibly reshaped —
    /// the framework changes what is tested and how it is counted).
    Covered,
    /// Coverage arrives with a later authoring wave; the justification names
    /// which chapter carries it.
    Pending,
    /// Adjudicated: the ground deliberately lands with a NAMED later
    /// workstream (justification names it). Unlike `pending`, a deferred row
    /// is a settled decision and does not hold the gate open.
    Deferred,
    /// Deliberately not carried into the CNF catalogue; justification
    /// mandatory.
    Dropped,
    /// Outside the CNF 2.0 platform scope (e.g. runner-internal checks);
    /// justification mandatory.
    OutOfScope,
}

/// One mapping entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapEntry {
    pub disposition: MapDisposition,
    /// CNF case ids carrying the ground (required for `covered`).
    #[serde(default)]
    pub covered_by: Vec<CaseId>,
    /// Why the difference from the old baseline is justified.
    pub justification: String,
}

/// The whole hand-adjudicated map, keyed by ECC id.
#[derive(Debug, Clone)]
pub struct EccMap {
    entries: Vec<(String, MapEntry)>,
}

impl EccMap {
    /// Look up an entry.
    #[must_use]
    pub fn get(&self, ecc_id: &str) -> Option<&MapEntry> {
        self.entries
            .iter()
            .find(|(k, _)| k == ecc_id)
            .map(|(_, e)| e)
    }

    /// All entries.
    #[must_use]
    pub fn entries(&self) -> &[(String, MapEntry)] {
        &self.entries
    }
}

impl<'de> Deserialize<'de> for EccMap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = crate::model::de::ordered_map(deserializer)?;
        Ok(Self { entries })
    }
}

/// The computed comparison.
#[derive(Debug)]
pub struct Comparison {
    /// (row, entry) for mapped active rows.
    pub mapped: Vec<(EccRow, MapEntry)>,
    /// Active ECC rows with no map entry yet — the open gap.
    pub unmapped: Vec<EccRow>,
    /// Map entries referencing ECC ids that do not exist (stale map rows).
    pub stale_map_ids: Vec<String>,
    /// `covered_by` ids that do not exist in the CNF catalogue.
    pub dangling_case_ids: Vec<(String, CaseId)>,
    /// CNF cases not reachable from any map entry (new ground the old
    /// harness never covered — reported, never a defect).
    pub new_ground: Vec<CaseId>,
}

impl Comparison {
    /// The gate: no unmapped active rows, no stale map ids, no dangling case
    /// references. (`pending` entries keep the gate open deliberately: the
    /// report prints them and cutover requires zero.)
    #[must_use]
    pub fn gate_clean(&self) -> bool {
        self.unmapped.is_empty()
            && self.stale_map_ids.is_empty()
            && self.dangling_case_ids.is_empty()
            && self
                .mapped
                .iter()
                .all(|(_, e)| e.disposition != MapDisposition::Pending)
    }
}

/// Compute the comparison.
#[must_use]
pub fn compare(rows: &[EccRow], map: &EccMap, set: &ArtifactSet) -> Comparison {
    let case_ids: Vec<&CaseId> = set.cases.iter().map(|(_, c)| &c.id).collect();
    let mut mapped = Vec::new();
    let mut unmapped = Vec::new();
    let mut dangling = Vec::new();
    let mut reached: Vec<&CaseId> = Vec::new();

    for row in rows {
        if row.status != "active" {
            continue;
        }
        match map.get(&row.id) {
            Some(entry) => {
                for target in &entry.covered_by {
                    match case_ids.iter().find(|id| **id == target) {
                        Some(id) => reached.push(id),
                        None => dangling.push((row.id.clone(), target.clone())),
                    }
                }
                mapped.push((row.clone(), entry.clone()));
            }
            None => unmapped.push(row.clone()),
        }
    }

    let stale_map_ids = map
        .entries()
        .iter()
        .map(|(k, _)| k)
        .filter(|k| !rows.iter().any(|r| &r.id == *k))
        .cloned()
        .collect();

    let new_ground = case_ids
        .iter()
        .filter(|id| !reached.contains(id))
        .map(|id| (*id).clone())
        .collect();

    Comparison {
        mapped,
        unmapped,
        stale_map_ids,
        dangling_case_ids: dangling,
        new_ground,
    }
}

/// Render the committed comparison report (deterministic Markdown).
#[must_use]
pub fn render_report(cmp: &Comparison, set: &ArtifactSet) -> String {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (_, entry) in &cmp.mapped {
        let key = match entry.disposition {
            MapDisposition::Covered => "covered",
            MapDisposition::Pending => "pending",
            MapDisposition::Deferred => "deferred",
            MapDisposition::Dropped => "dropped",
            MapDisposition::OutOfScope => "out_of_scope",
        };
        *counts.entry(key).or_default() += 1;
    }

    let mut out = String::new();
    let _ = writeln!(out, "# ECC ↔ CNF 2.0 catalogue comparison\n");
    let _ = writeln!(
        out,
        "Generated by `cnf-runner compare-ecc` — never hand-edited. The old\n\
         harness's committed catalogue is the comparison reference; differences\n\
         are expected and enumerated here (comparison, not reproduction).\n"
    );
    let _ = writeln!(out, "## Summary\n");
    let _ = writeln!(out, "| measure | count |");
    let _ = writeln!(out, "|---|---|");
    let _ = writeln!(out, "| CNF catalogue cases | {} |", set.cases.len());
    let _ = writeln!(
        out,
        "| active ECC rows | {} |",
        cmp.mapped.len() + cmp.unmapped.len()
    );
    for (key, count) in &counts {
        let _ = writeln!(out, "| mapped: {key} | {count} |");
    }
    let _ = writeln!(out, "| unmapped (open gap) | {} |", cmp.unmapped.len());
    let _ = writeln!(
        out,
        "| CNF cases beyond the old catalogue | {} |",
        cmp.new_ground.len()
    );
    let _ = writeln!(
        out,
        "\nGate clean: **{}**\n",
        if cmp.gate_clean() { "yes" } else { "NO" }
    );

    if !cmp.unmapped.is_empty() {
        let _ = writeln!(
            out,
            "## Unmapped active ECC rows (must reach zero before cutover)\n"
        );
        for row in &cmp.unmapped {
            let _ = writeln!(out, "- `{}` ({}) — {}", row.id, row.area, row.title);
        }
        let _ = writeln!(out);
    }
    if !cmp.stale_map_ids.is_empty() {
        let _ = writeln!(out, "## Stale map entries (ECC id no longer exists)\n");
        for id in &cmp.stale_map_ids {
            let _ = writeln!(out, "- `{id}`");
        }
        let _ = writeln!(out);
    }
    if !cmp.dangling_case_ids.is_empty() {
        let _ = writeln!(out, "## Dangling covered_by targets\n");
        for (ecc, case) in &cmp.dangling_case_ids {
            let _ = writeln!(out, "- `{ecc}` → `{case}` (not in the catalogue)");
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## Mapped rows\n");
    let _ = writeln!(out, "| ECC id | disposition | covered by | justification |");
    let _ = writeln!(out, "|---|---|---|---|");
    for (row, entry) in &cmp.mapped {
        let covered: Vec<String> = entry.covered_by.iter().map(|c| format!("`{c}`")).collect();
        let disposition = match entry.disposition {
            MapDisposition::Covered => "covered",
            MapDisposition::Pending => "pending",
            MapDisposition::Deferred => "deferred",
            MapDisposition::Dropped => "dropped",
            MapDisposition::OutOfScope => "out_of_scope",
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} |",
            row.id,
            disposition,
            covered.join(", "),
            entry.justification.replace('|', "\\|")
        );
    }

    if !cmp.new_ground.is_empty() {
        let _ = writeln!(
            out,
            "\n## CNF cases with no old-harness counterpart (new ground)\n"
        );
        for id in &cmp.new_ground {
            let _ = writeln!(out, "- `{id}`");
        }
    }
    out
}

/// Load, compare, render — the CLI entry.
///
/// # Errors
/// Any [`CompareError`].
pub fn run(
    ecc_catalog: &Path,
    map_path: &Path,
    set: &ArtifactSet,
) -> Result<(Comparison, String), CompareError> {
    let rows = parse_ecc_catalog(&std::fs::read_to_string(ecc_catalog)?)?;
    let map_text = std::fs::read_to_string(map_path)?;
    let map: EccMap =
        serde_saphyr::from_str(&map_text).map_err(|e| CompareError::Map(e.to_string()))?;
    let cmp = compare(&rows, &map, set);
    let report = render_report(&cmp, set);
    Ok((cmp, report))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    #[test]
    fn tsv_parses_and_gate_logic_holds() {
        let rows = parse_ecc_catalog(
            "# comment\nECC-EHR-001\tEHR\tactive\tehr/x\tTitle A\nECC-EHR-002\tEHR\tretired\tehr/y\tTitle B\n",
        )
        .unwrap();
        assert_eq!(rows.len(), 2);

        let map: EccMap = serde_saphyr::from_str(
            "ECC-EHR-001:\n  disposition: pending\n  justification: \"arrives with the EHR chapter wave\"\n",
        )
        .unwrap();
        let set = ArtifactSet::default();
        let cmp = compare(&rows, &map, &set);
        assert_eq!(cmp.mapped.len(), 1);
        assert!(cmp.unmapped.is_empty()); // retired rows are not counted
        assert!(!cmp.gate_clean()); // pending keeps the gate open

        let report = render_report(&cmp, &set);
        assert!(report.contains("Gate clean: **NO**"));
    }

    #[test]
    fn malformed_tsv_is_typed() {
        assert!(parse_ecc_catalog("only\tthree\tfields\n").is_err());
    }
}
