// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The committed schema set (`schemas/*.schema.json`) is the published norm;
//! it must stay byte-identical to what the code emits (regenerate with
//! `cargo run -p cnf-runner -- emit-schemas --out tools/cnf-runner/schemas`).

use cnf_runner::schema::{emit_all, render};

#[test]
fn committed_schemas_are_byte_identical_to_emission() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    for (name, schema) in emit_all() {
        let committed = std::fs::read_to_string(dir.join(name))
            .unwrap_or_else(|e| panic!("{name} is not committed: {e}"));
        assert_eq!(
            committed,
            render(&schema),
            "{name} drifted — regenerate with `cargo run -p cnf-runner -- emit-schemas`"
        );
    }
}

#[test]
fn no_extra_schema_files_are_committed() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    let expected: Vec<&str> = emit_all().into_iter().map(|(n, _)| n).collect();
    for entry in std::fs::read_dir(&dir).expect("schemas dir") {
        let name = entry.expect("dir entry").file_name();
        let name = name.to_string_lossy();
        assert!(
            expected.contains(&name.as_ref()),
            "unexpected committed schema file {name}"
        );
    }
}
