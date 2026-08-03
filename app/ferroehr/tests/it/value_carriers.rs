//! Regression gate: `serde_json::Value` carriers stay inside the adjudicated
//! seam classes — new service/versioning code carries TYPED values.
//!
//! The foundation phase's carrier inventory (#1694) classified every
//! `serde_json::Value` seam in this crate into three sanctioned classes:
//! stored-content serving (byte-stability of stored canonical fragments —
//! no openEHR spec governs storage mechanics, our own design), genuinely
//! dynamic shapes (AQL projections over a user-written SELECT list, external
//! FHIR resources, archive rows), and the commit INTERIOR — which now begins
//! at the single serialization boundary the typed commit seam takes (#1727),
//! so what it carries is a canonical fragment, never an unparsed body.
//! Everything else carries
//! generated `openehr-*` types — the strict reader (#1702) makes typed
//! carriers lossless by construction, so a NEW `Value` seam is a design
//! regression, not a convenience.
//!
//! This gate scans this crate's WHOLE `src/` production tree for mentions of `serde_json::Value` and requires every
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
        "commit-environment fragments are the canonical form the commit interior carries (stored-content class)",
    ),
    (
        "service/definition/",
        "stored template/query artefacts served verbatim + ADL/OPT wire envelopes",
    ),
    (
        "service/demographic/",
        "the commit interior carries the canonical fragment the seam produced once; stored-content serving",
    ),
    (
        "service/ehr/",
        "the commit interior carries the canonical fragment the seam produced once; stored-content serving",
    ),
    (
        "service/message/",
        "EHR-Extract/TDD fragments compose over verbatim stored content",
    ),
    (
        "service/mod.rs",
        "service-surface signatures over stored canonical fragments and dynamic shapes",
    ),
    (
        "service/query/",
        "AQL projections have no static type — the SELECT list is user-written",
    ),
    (
        "service/response.rs",
        "ServiceResponse::body — the hybrid RM/stored/dynamic carrier (commit-seam class, #1694 step 6)",
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
        "versioning/",
        "stored canonical envelopes; the serialized form is what gets signed \
         (RM common master06 §Digital Signature), so byte-stability governs",
    ),
    (
        "storage/",
        "the node table's verbatim canonical fragments (release-strategy \
         superset skew: a typed round-trip would drop forward-compatible \
         keys — owner-approved family 1, #1694)",
    ),
    (
        "extensions/",
        "external FHIR resources, tenancy/event CRUD rows, multimedia \
         offload over stored fragments (owner-approved families 3/6/8, \
         #1694; fixed-shape ops rows convert to DTOs per the family-8 \
         condition)",
    ),
    (
        "aql/",
        "AQL result rows are arbitrary projections by specification \
         (QUERY 1.1 — owner-approved family 5, #1694)",
    ),
    (
        "system_log/",
        "FHIR AuditEvent/BALP renderings and syslog payloads are external \
         formats (owner-approved families 6/8, #1694)",
    ),
    (
        "templates/",
        "stored OPT/WebTemplate artefacts served verbatim (owner-approved \
         families 1/8, #1694)",
    ),
    (
        "config/",
        "the redacted config dump is genuinely open operational JSON \
         (owner-approved family 8, #1694)",
    ),
];

/// Every production file under the crate's WHOLE `src/` tree mentioning
/// `serde_json::Value`, as crate-`src/`-relative paths (extended from
/// `service/` + `versioning/` to full coverage — the #1694 owner directive:
/// zero unadjudicated `Value` seams anywhere).
///
/// # Errors
/// Any I/O failure walking or reading the crate's own `src/` tree.
fn carrying_files() -> std::io::Result<Vec<String>> {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&src)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().is_dir() {
            collect(&entry.path(), &name, &mut out)?;
        } else if Path::new(&name).extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(entry.path())?;
            let production = match text.find("\n#[cfg(test)]") {
                Some(at) => text.get(..at).unwrap_or(&text),
                None => &text,
            };
            if production.contains("serde_json::Value") {
                out.push(name);
            }
        }
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
