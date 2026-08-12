// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Regression gate: canonical RM shapes are BUILT from the generated
//! `openehr-*` types, never hand-written as `json!` literals.
//!
//! A `json!` literal carrying a `"_type"` key is, by definition, a hand-rolled
//! canonical openEHR fragment: `_type` is the canonical-JSON discriminator the
//! native codec emits (`openehr-its`, `emit-json`), so nothing else has any
//! business writing it. Hand-written shapes drift from the model — wrong
//! attribute order, a missing mandatory attribute, a stale spelling after a
//! spec-pin bump — and none of that is caught by a compiler. Building the
//! generated type and serializing it through
//! `openehr_its::json::to_canonical_value` makes every one of those a build
//! error instead (issue #1686).
//!
//! This gate scans **this crate's own** `src/`. Unit-test fixtures inside
//! `#[cfg(test)]` modules are deliberately out of scope: a test fixture is an
//! input to assert against, not a served wire shape, and pinning literals
//! there is exactly how a codec regression gets caught.
//!
//! To add a site, classify it and put it in [`ALLOWLIST`] with a one-line
//! reason — every entry must name why that file's literals are NOT a
//! synthesized canonical shape. A stale entry (allowlisted file with no
//! remaining literals) fails too, so the list cannot rot.

use std::path::{Path, PathBuf};

/// Files whose `_type`-carrying `json!` literals are classified as something
/// other than a synthesized canonical shape, each with the reason.
const ALLOWLIST: &[(&str, &str)] = &[
    (
        "service/message/export.rs",
        "EXTRACT envelope composed over ALREADY-CANONICAL opaque fragments \
         (versioned-object bodies, version envelopes, revision histories) — \
         see the TODO(#1695) at the composition site",
    ),
    (
        "service/demographic/contribution.rs",
        "CONTRIBUTION envelope over the opaque canonical AUDIT_DETAILS \
         fragment; the synthesized parts are built from their generated types",
    ),
    (
        "versioning/contribution.rs",
        "CONTRIBUTION envelope whose `versions` hold either OBJECT_REFs or \
         whole resolved VERSION envelopes, which `Contribution.versions: \
         Vec<ObjectRef>` cannot express",
    ),
    (
        "versioning/wire.rs",
        "ORIGINAL_VERSION / IMPORTED_VERSION envelopes over VERBATIM stored \
         fragments — this serialization is what gets digitally signed (RM \
         common master06 §Digital Signature), so re-encoding it is unsafe",
    ),
];

/// Every `json!` invocation in `src` whose body carries a `"_type"` key,
/// as `(crate-relative path, 1-based line)`.
///
/// # Errors
/// Any I/O failure walking or reading the crate's own `src/` tree.
fn offending_sites() -> std::io::Result<Vec<(String, usize)>> {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    collect(&src, "", &mut out)?;
    out.sort();
    Ok(out)
}

/// Recursively walk `dir` (reached at crate-relative `prefix`), appending each
/// offending site to `out`.
fn collect(dir: &Path, prefix: &str, out: &mut Vec<(String, usize)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
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
        // test-placement rule), so everything from the first line-initial
        // `#[cfg(test)]` on is fixture territory and out of scope.
        let production = match text.find("\n#[cfg(test)]") {
            Some(at) => text.get(..at).unwrap_or(&text),
            None => &text,
        };
        for start in json_macro_bodies(production) {
            let line = production
                .get(..start)
                .map_or(1, |before| before.matches('\n').count() + 1);
            out.push((relative.clone(), line));
        }
    }
    Ok(())
}

/// Byte offsets of every `json!` invocation in `src` whose delimited body
/// contains a `"_type"` key.
fn json_macro_bodies(src: &str) -> Vec<usize> {
    let bytes = src.as_bytes();
    let mut hits = Vec::new();
    let mut from = 0;
    while let Some(rel) = src.get(from..).and_then(|rest| rest.find("json!")) {
        let at = from + rel;
        from = at + "json!".len();
        // `serde_json::json!` is the same macro; `foojson!` is not.
        let preceded_by_ident = at
            .checked_sub(1)
            .and_then(|i| bytes.get(i))
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
        if preceded_by_ident {
            continue;
        }
        let Some(open) = src
            .get(from..)
            .and_then(|rest| rest.find(|c: char| !c.is_whitespace()).map(|i| from + i))
        else {
            continue;
        };
        if !matches!(bytes.get(open), Some(b'(' | b'{' | b'[')) {
            continue;
        }
        if let Some(end) = matching_delimiter(bytes, open)
            && src.get(open..=end).is_some_and(|b| b.contains("\"_type\""))
        {
            hits.push(at);
        }
    }
    hits
}

/// The index of the delimiter closing the one at `open`, skipping over string
/// literals so a brace inside a JSON string cannot unbalance the scan.
fn matching_delimiter(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, byte) in bytes.iter().enumerate().skip(open) {
        if in_string {
            match byte {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// No production `json!` literal outside the classified allowlist synthesizes a
/// canonical openEHR shape.
#[test]
fn canonical_shapes_are_built_from_the_generated_types() {
    let sites = offending_sites().expect("the crate's src/ tree should be readable");
    let unlisted: Vec<&(String, usize)> = sites
        .iter()
        .filter(|(file, _)| !ALLOWLIST.iter().any(|(listed, _)| listed == file))
        .collect();

    assert!(
        unlisted.is_empty(),
        "issue #1686: {} `json!` literal(s) in ferroehr/src synthesize a \
         canonical openEHR shape (they carry a `\"_type\"` key). Build the \
         generated `openehr-rm`/`openehr-base` type and serialize it with \
         `openehr_its::json::to_canonical_value` instead — that is what keeps \
         attribute order and mandatory attributes correct by construction. If \
         a site is genuinely a verbatim pass-through, an internal \
         (non-canonical) shape, or an OAS documentation example, add its file \
         to the ALLOWLIST in this test with a one-line reason.\nSites:\n{}",
        unlisted.len(),
        unlisted
            .iter()
            .map(|(file, line)| format!("  app/ferroehr/src/{file}:{line}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Every allowlist entry still describes a real site — a stale entry is a
/// silent licence to reintroduce hand-written canonical JSON.
#[test]
fn the_allowlist_carries_no_stale_entries() {
    let sites = offending_sites().expect("the crate's src/ tree should be readable");
    let stale: Vec<&str> = ALLOWLIST
        .iter()
        .filter(|(listed, _)| !sites.iter().any(|(file, _)| file == listed))
        .map(|(listed, _)| *listed)
        .collect();

    assert!(
        stale.is_empty(),
        "issue #1686: these ALLOWLIST entries no longer match any `json!` \
         literal carrying a `\"_type\"` key — the sites were converted, so \
         drop the entries: {stale:?}",
    );
}
