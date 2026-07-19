#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! The canonical-JSON **output contract** gate (the generation-subsystem
//! rewrite's R0 prerequisite).
//!
//! The openEHR specs do not define canonical JSON at the byte level: the
//! vendored ITS-JSON schemas (`schemas/json/`) govern structure, and JSON
//! itself (RFC 8259 §6) makes `5` and `5.0` the same number and object
//! member order insignificant. This gate therefore pins the parts of our
//! serialized output that ARE contract:
//!
//! 1. **Determinism** — the typed serializer's output for the whole vendored
//!    corpus is snapshot-pinned (a hash manifest). Any change to our emitted
//!    bytes is a deliberate, reviewed contract change — never silent drift.
//! 2. **RM number typing** — integer-typed RM fields (e.g.
//!    `DV_COUNT.magnitude`, `DV_ORDINAL.value`) serialize as JSON integers
//!    (no decimal point); Real-typed fields (e.g. `DV_QUANTITY.magnitude`)
//!    always carry a decimal point, even for whole values, and the TYPED
//!    serializer normalizes an integer input lexeme to the field's RM type.
//!    (Schema-governed: the vendored ITS-JSON schema types these fields
//!    `integer` / `number`; the whole-real `x.0` rendering itself is
//!    spec-silent — NOTE: no openEHR spec governs the lexeme choice — our
//!    own design, matching the canonical-XML runtime's rule.)
//! 3. **`_type`-first member order** — spec-silent; our own design, pinned
//!    here so it can only change deliberately.

mod common;

use common::{corpus_files, excluded};
use openehr_its::json::{from_canonical_json, to_canonical_json};
use openehr_rm::prelude::{
    Composition, Contribution, DvCount, DvOrdinal, DvQuantity, EhrStatus, Folder, ItemTree,
};
use sha2::Digest;
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Typed-parse the corpus doc by its top-level `_type` and re-serialize it
/// through the canonical entry point; `None` = not a dispatchable
/// single-RM-object root (same skip set as the fidelity gate).
fn serialize_typed(ty: &str, json: &str) -> Option<Result<String, String>> {
    macro_rules! ser {
        ($T:ty) => {{
            Some(
                from_canonical_json::<$T>(json)
                    .map_err(|e| e.to_string())
                    .map(|v| to_canonical_json(&v)),
            )
        }};
    }
    match ty {
        "COMPOSITION" => ser!(Composition),
        "FOLDER" => ser!(Folder),
        "EHR_STATUS" => ser!(EhrStatus),
        "CONTRIBUTION" => ser!(Contribution),
        "ITEM_TREE" => ser!(ItemTree),
        _ => None,
    }
}

/// 1. Determinism: hash-manifest snapshot of the typed serializer's output
///    over the entire vendored corpus. A changed line means our canonical
///    output changed — that must be a reviewed contract change, never drift.
#[test]
fn typed_canonical_output_is_deterministic_over_the_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vendor");
    let mut manifest = String::new();
    let mut serialized = 0usize;
    for path in corpus_files() {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue; // non-JSON fixtures are outside this gate
        };
        let Some(ty) = value.get("_type").and_then(|t| t.as_str()) else {
            continue; // fragments without a root _type: not canonical roots
        };
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        if let Some(reason) = excluded(&rel) {
            println!("excluded {rel}: {reason}");
            continue; // the shared documented exclusion list (common::excluded)
        }
        match serialize_typed(ty, &text) {
            Some(Ok(out)) => {
                let digest = sha2::Sha256::digest(out.as_bytes());
                let mut hex = String::with_capacity(64);
                for byte in digest {
                    let _ = write!(hex, "{byte:02x}");
                }
                let _ = writeln!(manifest, "{rel}  {hex}  {len}", len = out.len());
                serialized += 1;
            }
            Some(Err(e)) => panic!("corpus doc {rel} failed the typed round-trip: {e}"),
            None => {} // non-dispatchable root types: the fidelity gate owns them
        }
    }
    assert!(
        serialized >= 50, // 54 dispatchable canonical roots at authoring time
        "the corpus shrank unexpectedly ({serialized} docs serialized) — \
         the determinism gate lost its subject"
    );
    insta::assert_snapshot!("canonical_output_manifest", manifest);
}

/// 2. RM number typing: integer-typed fields print as JSON integers,
///    Real-typed fields always carry a decimal point, and the typed serializer
///    normalizes input lexemes to the field's RM type.
#[test]
fn rm_number_typing_governs_the_output_lexeme() {
    // DV_COUNT.magnitude is an Integer64 (vendored ITS-JSON schema: integer).
    let count: DvCount =
        from_canonical_json(r#"{"_type":"DV_COUNT","magnitude":5}"#).expect("count parses");
    let out = to_canonical_json(&count);
    assert!(
        out.contains(r#""magnitude":5"#) && !out.contains(r#""magnitude":5.0"#),
        "integer-typed magnitude must not print a decimal point: {out}"
    );

    // DV_ORDINAL.value is an Integer (schema: integer).
    let ordinal: DvOrdinal = from_canonical_json(
        r#"{"_type":"DV_ORDINAL","value":2,"symbol":{"_type":"DV_CODED_TEXT","value":"x","defining_code":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"local"},"code_string":"at0001"}}}"#,
    )
    .expect("ordinal parses");
    let out = to_canonical_json(&ordinal);
    assert!(
        out.contains(r#""value":2"#) && !out.contains(r#""value":2.0"#),
        "integer-typed ordinal value must not print a decimal point: {out}"
    );

    // DV_QUANTITY.magnitude is a Real (schema: number): whole reals keep a
    // decimal point, and an INTEGER INPUT LEXEME is normalized to the RM
    // typing by the typed serializer (unlike the historical Value
    // passthrough, which echoed the input lexeme by accident).
    let quantity: DvQuantity =
        from_canonical_json(r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm"}"#)
            .expect("quantity parses from an integer lexeme");
    let out = to_canonical_json(&quantity);
    assert!(
        out.contains(r#""magnitude":5.0"#),
        "Real-typed magnitude prints with a decimal point (normalized): {out}"
    );
}

/// 3. `_type` is the first member of every serialized RM object.
#[test]
fn type_discriminator_is_the_first_member() {
    let count: DvCount =
        from_canonical_json(r#"{"_type":"DV_COUNT","magnitude":1}"#).expect("parses");
    let out = to_canonical_json(&count);
    assert!(
        out.starts_with(r#"{"_type":"DV_COUNT""#),
        "_type must lead the object: {out}"
    );
}
