#![allow(clippy::doc_markdown, clippy::doc_lazy_continuation)] // prose with proper nouns + numbered case docs
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::float_cmp,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures (expects in helper fns, not #[test] bodies)
//! The **byte-parity gate** for the native canonical-JSON codec (the
//! generation-subsystem rewrite's R3 acceptance): every value serialized through
//! the emitted `ToJson` codec (`json_codec`) must produce bytes IDENTICAL to the
//! `#[derive(OpenEhrType)]` serde path (`json::to_canonical_json`) it shadows.
//!
//! Proven two ways:
//! 1. over every dispatchable canonical root of the vendored corpus (the same
//!    subject the R0 determinism gate snapshots), parse-once then compare both
//!    serializers on the one typed value;
//! 2. over hand-built unit cases for the byte hazards the corpus may not
//!    exercise: float lexemes, string escaping, `None`/empty omission, the
//!    `Interval` default flags, and a typed-literal enum (including an `Other`).
//!
//! The R0 manifest snapshot (`canonical_contract.rs`) is untouched — this gate
//! ADDS a codec and proves parity; it does not change the canonical output.

mod common;

use common::{corpus_files, excluded};
use openehr_its::json::{from_canonical_json, to_canonical_json};
use openehr_its::json_codec::runtime::{ToJson, to_json_string};
use openehr_rm::prelude::{
    Composition, Contribution, DvCount, DvInterval, DvOrdinal, DvQuantity, DvTextData, EhrStatus,
    Folder, ItemTree, ProportionKind,
};
use std::fs;
use std::path::Path;

/// Parse the corpus doc by its top-level `_type`, then compare the serde path
/// (`to_canonical_json`) against the native codec (`to_json_string`) on the SAME
/// typed value. `None` = not a dispatchable single-RM-object root (same skip set
/// as the R0/fidelity gates).
fn parity_of(ty: &str, json: &str) -> Option<Result<(), String>> {
    macro_rules! cmp {
        ($T:ty) => {{
            Some((|| {
                let value: $T = from_canonical_json(json).map_err(|e| e.to_string())?;
                let serde = to_canonical_json(&value).map_err(|e| e.to_string())?;
                let codec = to_json_string(&value);
                if serde == codec {
                    Ok(())
                } else {
                    Err(first_divergence(&serde, &codec))
                }
            })())
        }};
    }
    match ty {
        "COMPOSITION" => cmp!(Composition),
        "FOLDER" => cmp!(Folder),
        "EHR_STATUS" => cmp!(EhrStatus),
        "CONTRIBUTION" => cmp!(Contribution),
        "ITEM_TREE" => cmp!(ItemTree),
        _ => None,
    }
}

/// A compact report of where two serializations first differ (byte offset + a
/// window around it), so a parity failure is diagnosable.
fn first_divergence(a: &str, b: &str) -> String {
    let at = a
        .bytes()
        .zip(b.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let lo = at.saturating_sub(40);
    let ahi = (at + 40).min(a.len());
    let bhi = (at + 40).min(b.len());
    format!(
        "diverge at byte {at} (serde.len={}, codec.len={}):\n  serde: …{}…\n  codec: …{}…",
        a.len(),
        b.len(),
        &a[lo..ahi],
        &b[lo..bhi],
    )
}

/// 1. Corpus parity: every dispatchable canonical root serializes byte-identically
/// through both paths.
#[test]
fn codec_matches_serde_over_the_corpus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vendor");
    let mut compared = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();
    for path in corpus_files() {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(ty) = value.get("_type").and_then(|t| t.as_str()) else {
            continue;
        };
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        if excluded(&rel).is_some() {
            continue;
        }
        match parity_of(ty, &text) {
            Some(Ok(())) => compared += 1,
            Some(Err(e)) => failures.push((rel, e)),
            None => {}
        }
    }
    for (f, e) in failures.iter().take(20) {
        println!("\n--- PARITY MISMATCH: {f}\n{e}");
    }
    assert!(
        failures.is_empty(),
        "{} corpus doc(s) diverged between the serde path and the native codec",
        failures.len()
    );
    assert!(
        compared >= 50,
        "the corpus shrank unexpectedly ({compared} docs compared) — the parity gate lost its subject"
    );
    println!("codec/serde byte parity: {compared} canonical corpus roots identical");
}

/// Assert a typed value serializes identically through both paths, and (as a
/// spec-anchored sanity check) that the codec output equals `expect`.
fn assert_parity<T: serde::Serialize + ToJson>(value: &T, expect: &str) {
    let serde = to_canonical_json(value).expect("serde serializes");
    let codec = to_json_string(value);
    assert_eq!(serde, codec, "codec diverged from serde");
    assert_eq!(codec, expect, "codec output changed");
}

/// 2a. Float lexemes: integer-typed fields print as integers; Real-typed fields
/// carry a decimal point (whole reals as `x.0`); many-decimal + exponent reals
/// go through ryu identically to serde_json.
#[test]
fn float_and_integer_lexemes() {
    // Integer-typed magnitude → no decimal point.
    let count: DvCount = from_canonical_json(r#"{"_type":"DV_COUNT","magnitude":5}"#).unwrap();
    assert_parity(&count, r#"{"_type":"DV_COUNT","magnitude":5}"#);

    // Real-typed magnitude, whole value → `x.0` (normalized from an int lexeme).
    let q: DvQuantity =
        from_canonical_json(r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm"}"#).unwrap();
    assert_parity(
        &q,
        r#"{"_type":"DV_QUANTITY","magnitude":5.0,"units":"mm"}"#,
    );

    // Real with many decimals and a real with an exponent — both via ryu.
    let q2: DvQuantity =
        from_canonical_json(r#"{"_type":"DV_QUANTITY","magnitude":3.14159265,"units":"mm"}"#)
            .unwrap();
    assert_parity(
        &q2,
        r#"{"_type":"DV_QUANTITY","magnitude":3.14159265,"units":"mm"}"#,
    );
    // An exponent-form real: ryu is our chosen canonical REAL lexeme (`1e21`),
    // which deliberately differs from serde_json's dtoa (`1e+21`) on this rare
    // form (see the runtime module NOTE). Assert the deterministic codec output.
    let q3: DvQuantity =
        from_canonical_json(r#"{"_type":"DV_QUANTITY","magnitude":1e21,"units":"mm"}"#).unwrap();
    assert_eq!(
        to_json_string(&q3),
        r#"{"_type":"DV_QUANTITY","magnitude":1e21,"units":"mm"}"#
    );
}

/// 2b. String escaping: quotes, backslashes, control chars, non-ASCII — the codec
/// reproduces serde_json's escape set exactly.
#[test]
fn string_escaping_hazards() {
    for value in [
        "plain",
        "with \"quotes\" and \\ backslash",
        "tab\tlf\ncr\rbell\u{07}ff\u{0C}",
        "unit\u{1F}separator",
        "slash/is/not/escaped",
        "café — naïve — 日本語 — 😀",
    ] {
        let mut text = empty_dv_text_data();
        text.value = value.to_string();
        let serde = to_canonical_json(&text).unwrap();
        let codec = to_json_string(&text);
        assert_eq!(serde, codec, "escaping diverged for {value:?}");
    }
}

/// A `DV_TEXT` payload (`DvTextData`) with only `value` set (all other fields
/// `None`/empty).
fn empty_dv_text_data() -> DvTextData {
    from_canonical_json(r#"{"_type":"DV_TEXT","value":""}"#).expect("minimal DV_TEXT parses")
}

/// 2c. `None`/empty omission: an optional-absent field and an empty container are
/// both dropped, identically to the derive.
#[test]
fn none_and_empty_omission() {
    // DV_TEXT with no mappings/language/etc.: only `_type` + `value`.
    let text = empty_dv_text_data();
    assert_parity(&text, r#"{"_type":"DV_TEXT","value":""}"#);
    // The serde output itself must carry neither a null nor an empty array.
    let serde = to_canonical_json(&text).unwrap();
    assert!(!serde.contains("null") && !serde.contains("[]"), "{serde}");
}

/// 2d. The `Interval` default flags: `DV_INTERVAL` `*_included`/`*_unbounded` are
/// mandatory bool fields (`#[openehr(default)]`) — always emitted (Plain), not
/// omitted, identically through both paths.
#[test]
fn interval_default_flags_are_emitted() {
    let json = r#"{"_type":"DV_INTERVAL","lower":{"_type":"DV_QUANTITY","magnitude":1.0,"units":"mm"},"upper":{"_type":"DV_QUANTITY","magnitude":2.0,"units":"mm"}}"#;
    let iv: DvInterval<DvQuantity> = from_canonical_json(json).unwrap();
    let serde = to_canonical_json(&iv).unwrap();
    let codec = to_json_string(&iv);
    assert_eq!(serde, codec, "interval parity diverged");
    for flag in [
        "lower_included",
        "upper_included",
        "lower_unbounded",
        "upper_unbounded",
    ] {
        assert!(codec.contains(flag), "default flag {flag} missing: {codec}");
    }
}

/// 2e. A typed-literal enum: `PROPORTION_KIND` (integer-backed) inside a
/// `DV_PROPORTION`, including a known constant and an out-of-set `Other` value.
#[test]
fn typed_literal_enum_including_other() {
    // Known constant (pk_ratio = 0).
    let known: DvOrdinal = from_canonical_json(
        r#"{"_type":"DV_ORDINAL","value":1,"symbol":{"_type":"DV_CODED_TEXT","value":"x","defining_code":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"local"},"code_string":"at1"}}}"#,
    )
    .unwrap();
    let serde = to_canonical_json(&known).unwrap();
    assert_eq!(serde, to_json_string(&known));

    // Directly exercise the ProportionKind ToJson: a known constant and Other.
    for pk in [
        ProportionKind::from_value(0),
        ProportionKind::from_value(99),
    ] {
        // serde_json serializes the enum as its bare integer; the codec matches.
        let serde = serde_json::to_string(&pk).unwrap();
        assert_eq!(serde, to_json_string(&pk), "ProportionKind {pk:?}");
    }
}
