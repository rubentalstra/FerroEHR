//! Regression gate: `serde_json::Value` carriers stay inside the adjudicated
//! seam classes — new service/versioning code carries TYPED values.
//!
//! The foundation phase's carrier inventory (#1694) classified every
//! `serde_json::Value` seam in this crate into three sanctioned classes:
//! stored-content serving (byte-stability of stored canonical fragments —
//! no openEHR spec governs storage mechanics, our own design), genuinely
//! dynamic shapes (AQL projections over a user-written SELECT list, external
//! FHIR resources, archive rows), and the commit seam whose typed conversion
//! is gated on the 400/422 owner decision (#1727). Everything else carries
//! generated `openehr-*` types — the strict reader (#1702) makes typed
//! carriers lossless by construction, so a NEW `Value` seam is a design
//! regression, not a convenience.
//!
//! This gate scans this crate's own `src/service/` + `src/versioning/`
//! production code for mentions of `serde_json::Value` and requires every
//! carrying file to fall under an [`ALLOWLIST`] prefix, each carrying its
//! classification. A stale prefix (no remaining carriers under it) fails
//! too, so the list ratchets DOWN as seams convert (#1727, #1712) and
//! cannot rot upward.

use std::path::{Path, PathBuf};

/// Sanctioned carrier territories, each with its inventory classification.
///
/// Prefixes are crate-`src/`-relative; a file is sanctioned when its path
/// starts with an entry. Keep entries as NARROW as honesty allows — when a
/// seam converts, delete its entry so the gate ratchets.
const ALLOWLIST: &[(&str, &str)] = &[
    (
        "service/admin/",
        "archives and dumps are verbatim by definition (stored-content class)",
    ),
    (
        "service/commit_env.rs",
        "commit-environment fragments feed the Value-based commit seam (#1727)",
    ),
    (
        "service/definition/",
        "stored template/query artefacts served verbatim + ADL/OPT wire envelopes",
    ),
    (
        "service/demographic/",
        "commit bodies pending the typed seam (#1727); stored canonical fragments",
    ),
    (
        "service/ehr/",
        "commit bodies pending the typed seam (#1727); stored fragment serving",
    ),
    (
        "service/message/",
        "EHR-Extract/TDD fragments compose over verbatim stored content",
    ),
    (
        "service/mod.rs",
        "service-surface signatures over the Value-based commit seam (#1727)",
    ),
    (
        "service/query/",
        "AQL projections have no static type — the SELECT list is user-written",
    ),
    (
        "service/response.rs",
        "generic RESULT_SET row values (AQL projection class)",
    ),
    (
        "service/subject_proxy/",
        "external FHIR/AQL projections are not openEHR RM shapes",
    ),
    (
        "service/terminology/",
        "external FHIR terminology resources are not openEHR RM shapes",
    ),
    (
        "service/validity.rs",
        "the validity checker's input is arbitrary submitted JSON by contract",
    ),
    (
        "service/version_update.rs",
        "UpdateVersion<T = Value>'s default instantiation is the #1727 seam",
    ),
    (
        "versioning/",
        "stored canonical envelopes; the serialized form is what gets signed \
         (RM common master06 §Digital Signature), so byte-stability governs",
    ),
];

/// Every production file under `src/service/` + `src/versioning/` mentioning
/// `serde_json::Value`, as crate-`src/`-relative paths.
///
/// # Errors
/// Any I/O failure walking or reading the crate's own `src/` tree.
fn carrying_files() -> std::io::Result<Vec<String>> {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for root in ["service", "versioning"] {
        collect(&src.join(root), root, &mut out)?;
    }
    out.sort();
    Ok(out)
}

/// Recursively walk `dir` (reached at `src/`-relative `prefix`), appending
/// each carrying file to `out`.
fn collect(dir: &Path, prefix: &str, out: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}/{name}");
        let path = entry.path();
        if path.is_dir() {
            collect(&path, &relative, out)?;
            continue;
        }
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        // Unit tests live in a trailing `#[cfg(test)] mod tests` (the repo's
        // test-placement rule); fixtures there are inputs, not seams.
        let production = match text.find("\n#[cfg(test)]") {
            Some(at) => text.get(..at).unwrap_or(&text),
            None => &text,
        };
        if production.contains("serde_json::Value") {
            out.push(relative);
        }
    }
    Ok(())
}

/// Every carrying file falls under a sanctioned prefix.
#[test]
fn value_carriers_stay_inside_the_adjudicated_classes() {
    let files = carrying_files().expect("the crate's src/ tree should be readable");
    let unsanctioned: Vec<&String> = files
        .iter()
        .filter(|f| ALLOWLIST.iter().all(|(p, _)| !f.starts_with(p)))
        .collect();
    assert!(
        unsanctioned.is_empty(),
        "new serde_json::Value seams outside the adjudicated classes \
         (#1694 — carry a generated type, or classify the seam and extend \
         the allowlist with its reason): {unsanctioned:?}"
    );
}

/// Every prefix still sanctions at least one carrier — a stale entry fails,
/// so the list ratchets DOWN as seams convert.
#[test]
fn the_carrier_allowlist_carries_no_stale_entries() {
    let files = carrying_files().expect("the crate's src/ tree should be readable");
    let stale: Vec<&str> = ALLOWLIST
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| !files.iter().any(|f| f.starts_with(p)))
        .collect();
    assert!(
        stale.is_empty(),
        "allowlist prefixes with no remaining Value carriers — the seams \
         converted, so delete the entries and let the gate ratchet: {stale:?}"
    );
}
