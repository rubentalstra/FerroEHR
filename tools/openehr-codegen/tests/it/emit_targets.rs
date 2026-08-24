// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The three text-producing emit targets — `emit-json`, `emit-xml`,
//! `emit-rest` — as properties over the **real** pipeline on the **real**
//! vendored inputs.
//!
//! Each test drives `openehr_codegen::testsupport::emit_*_to_memory`, which
//! calls the very render function the matching `cmd_*` handler calls: the
//! handler is a write-files shell over it, so tested text and emitted text
//! cannot drift. Every target is asserted three ways — the render is
//! byte-deterministic, it produces exactly its file set (each non-empty,
//! banner-headed and SPDX-stamped), and its bytes equal the committed
//! generated tree, which is the in-process half of the `codegen-drift` check.

use openehr_codegen::testsupport;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The complete file set `emit-json` produces, in emitted-map (sorted) order.
const JSON_FILES: &[&str] = &[
    "openehr-am/src/json_serde.rs",
    "openehr-base/src/json_serde.rs",
    "openehr-its/src/json_codec/generated/mod.rs",
    "openehr-its/src/json_codec/generated/structural.rs",
    "openehr-lang/src/json_serde.rs",
    "openehr-rm/src/json_serde.rs",
    "openehr-term/src/json_serde.rs",
];

/// The complete file set `emit-xml` produces, in emitted-map (sorted) order.
const XML_FILES: &[&str] = &[
    "openehr-its/src/xml/generated/impls.rs",
    "openehr-its/src/xml/generated/mod.rs",
];

/// The complete file set `emit-rest` produces, in emitted-map (sorted) order:
/// the shared `common` module, one module per API group whose OAS declares
/// operations, and the module declaration file.
const REST_FILES: &[&str] = &[
    "openehr-its/src/rest/generated/admin.rs",
    "openehr-its/src/rest/generated/common.rs",
    "openehr-its/src/rest/generated/definition.rs",
    "openehr-its/src/rest/generated/demographic.rs",
    "openehr-its/src/rest/generated/ehr.rs",
    "openehr-its/src/rest/generated/mod.rs",
    "openehr-its/src/rest/generated/query.rs",
    "openehr-its/src/rest/generated/system.rs",
];

// ── emit-json ───────────────────────────────────────────────────────────────

/// Rendering the canonical-JSON impls twice yields byte-identical output.
#[test]
fn emit_json_is_byte_deterministic() {
    let a = testsupport::emit_json_to_memory().unwrap();
    let b = testsupport::emit_json_to_memory().unwrap();
    assert_deterministic("emit-json", &a, &b);
}

/// `emit-json` produces exactly its file set, each file non-empty and carrying
/// the generated banner plus its crate's SPDX header.
#[test]
fn emit_json_emits_its_whole_file_set() {
    let files = testsupport::emit_json_to_memory().unwrap();
    assert_file_set("emit-json", &files, JSON_FILES);
}

/// The rendered canonical-JSON impls equal the committed generated tree.
#[test]
fn emit_json_matches_the_committed_tree() {
    let files = testsupport::emit_json_to_memory().unwrap();
    assert_matches_committed_tree("emit-json", &files);
}

// ── emit-xml ────────────────────────────────────────────────────────────────

/// Rendering the canonical-XML impls twice yields byte-identical output.
#[test]
fn emit_xml_is_byte_deterministic() {
    let a = testsupport::emit_xml_to_memory().unwrap();
    let b = testsupport::emit_xml_to_memory().unwrap();
    assert_deterministic("emit-xml", &a, &b);
}

/// `emit-xml` produces exactly its file set, each file non-empty and carrying
/// the generated banner plus its crate's SPDX header.
#[test]
fn emit_xml_emits_its_whole_file_set() {
    let files = testsupport::emit_xml_to_memory().unwrap();
    assert_file_set("emit-xml", &files, XML_FILES);
}

/// The rendered canonical-XML impls equal the committed generated tree.
#[test]
fn emit_xml_matches_the_committed_tree() {
    let files = testsupport::emit_xml_to_memory().unwrap();
    assert_matches_committed_tree("emit-xml", &files);
}

// ── emit-rest ───────────────────────────────────────────────────────────────

/// Rendering the ITS-REST contract twice yields byte-identical output.
#[test]
fn emit_rest_is_byte_deterministic() {
    let a = testsupport::emit_rest_to_memory().unwrap();
    let b = testsupport::emit_rest_to_memory().unwrap();
    assert_deterministic("emit-rest", &a, &b);
}

/// `emit-rest` produces exactly its file set, each file non-empty and carrying
/// the generated banner plus its crate's SPDX header.
#[test]
fn emit_rest_emits_its_whole_file_set() {
    let files = testsupport::emit_rest_to_memory().unwrap();
    assert_file_set("emit-rest", &files, REST_FILES);
}

/// The rendered ITS-REST contract equals the committed generated tree.
#[test]
fn emit_rest_matches_the_committed_tree() {
    let files = testsupport::emit_rest_to_memory().unwrap();
    assert_matches_committed_tree("emit-rest", &files);
}

// ── shared assertions ───────────────────────────────────────────────────────

/// Assert two renders of the same target agree on every path and every byte.
fn assert_deterministic(target: &str, a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) {
    assert!(!a.is_empty(), "{target}: rendered nothing");
    let paths_a: Vec<&String> = a.keys().collect();
    let paths_b: Vec<&String> = b.keys().collect();
    assert_eq!(
        paths_a, paths_b,
        "{target}: the two renders emitted different files",
    );
    for ((path, body_a), body_b) in a.iter().zip(b.values()) {
        assert!(
            body_a == body_b,
            "{target}: {path} differs between two renders{}",
            difference_hint(body_a, body_b),
        );
    }
}

/// Assert a target emitted exactly `expected`, with every file non-empty, its
/// `@generated` banner naming the target on line one, and its SPDX licence
/// header present.
fn assert_file_set(target: &str, files: &BTreeMap<String, String>, expected: &[&str]) {
    let actual: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(actual, expected, "{target}: emitted a different file set");
    for (path, body) in files {
        assert!(!body.trim().is_empty(), "{target}: {path} rendered empty");
        let banner = body.lines().next().unwrap_or_default();
        assert!(
            banner.contains("@generated") && banner.contains(target),
            "{target}: {path} does not open with its generated banner: {banner:?}",
        );
        // The needle names the SPDX tag WITHOUT an expression, which the
        // REUSE scanner would otherwise parse and refuse as invalid.
        // REUSE-IgnoreStart
        assert!(
            body.contains("SPDX-License-Identifier: "),
            "{target}: {path} carries no SPDX licence header",
        );
        // REUSE-IgnoreEnd
    }
}

/// Assert a target's rendered bytes equal the committed generated tree.
///
/// The rendered body is put through `rustfmt` first, exactly as the `cmd_*`
/// handler puts the file it just wrote through it — `rustfmt` reading stdin
/// formats byte-identically to `rustfmt` reading that same text from a file, so
/// nothing has to be written to compare.
#[expect(
    clippy::panic,
    reason = "test diagnostics: a missing committed file names itself instead of \
              failing as a bare unwrap"
)]
fn assert_matches_committed_tree(target: &str, files: &BTreeMap<String, String>) {
    let root = crates_root();
    for (path, body) in files {
        let full = root.join(path);
        let committed = std::fs::read_to_string(&full)
            .unwrap_or_else(|e| panic!("{target}: cannot read committed {path}: {e}"));
        let rendered = rustfmt(&committed_relative_name(path), body);
        assert!(
            rendered == committed,
            "{target}: {path} does not match the committed generated tree{}",
            difference_hint(&rendered, &committed),
        );
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// The workspace `crates/` directory every emitted path is relative to.
fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates")
}

/// A short label for a path, used only in `rustfmt` failure messages.
fn committed_relative_name(path: &str) -> String {
    format!("crates/{path}")
}

/// Format `text` with the same `rustfmt` invocation the emitter shells out to.
///
/// The body is written from a separate thread because a large file (the biggest
/// emitted impl set is megabytes) fills the pipe buffer long before `rustfmt`
/// finishes, and writing it all before reading stdout would deadlock.
#[expect(
    clippy::panic,
    reason = "test diagnostics: each subprocess failure mode names itself, so a red \
              run points at the toolchain rather than at the emitter"
)]
fn rustfmt(label: &str, text: &str) -> String {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--quiet", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("{label}: rustfmt is not runnable: {e}"));
    let mut stdin = child
        .stdin
        .take()
        .unwrap_or_else(|| panic!("{label}: rustfmt stdin was not piped"));
    let body = text.to_owned();
    let writer = std::thread::spawn(move || stdin.write_all(body.as_bytes()));
    let out = child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("{label}: rustfmt did not complete: {e}"));
    writer
        .join()
        .unwrap_or_else(|_| panic!("{label}: the rustfmt writer thread panicked"))
        .unwrap_or_else(|e| panic!("{label}: writing to rustfmt failed: {e}"));
    assert!(
        out.status.success(),
        "{label}: rustfmt failed ({}): {}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8(out.stdout)
        .unwrap_or_else(|e| panic!("{label}: rustfmt emitted non-UTF-8: {e}"))
}

/// A one-line locator for the first place two texts diverge.
///
/// The emitted files run to megabytes, so a whole-body `assert_eq!` would print
/// an unreadable pair; the first differing line is what actually identifies the
/// defect.
fn difference_hint(a: &str, b: &str) -> String {
    for (i, (la, lb)) in a.lines().zip(b.lines()).enumerate() {
        if la != lb {
            return format!(
                "\n  first difference at line {}:\n  - {la}\n  + {lb}",
                i + 1
            );
        }
    }
    format!(
        "\n  line counts differ: {} vs {}",
        a.lines().count(),
        b.lines().count(),
    )
}
