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
use openehr_its::json_codec::runtime::{ToJson, from_json_str, to_json_string};
use openehr_rm::prelude::{
    Composition, Contribution, DataValue, DvCount, DvInterval, DvOrdinal, DvQuantity, DvText,
    DvTextData, EhrStatus, Folder, ItemTree, ProportionKind,
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

// ════════════════════════════════════════════════════════════════════════════
// Deserialize-side parity: the native `FromJson` reader (`from_json_str`) must
// produce the SAME typed value as the serde `Deserialize` (`from_canonical_json`)
// it shadows — and reproduce the retired derive's tolerance rules, rule by rule.
// ════════════════════════════════════════════════════════════════════════════

/// Parse the corpus doc by its top-level `_type` through BOTH readers and compare
/// the typed values for equality; `None` = not a dispatchable root (same skip set
/// as the serialize gate).
fn deser_parity_of(ty: &str, json: &str) -> Option<Result<(), String>> {
    macro_rules! cmp {
        ($T:ty) => {{
            Some((|| {
                let serde: $T = from_canonical_json(json).map_err(|e| format!("serde: {e}"))?;
                let codec: $T = from_json_str(json).map_err(|e| format!("codec: {e}"))?;
                if serde == codec {
                    Ok(())
                } else {
                    // Re-serialize both for a diffable diagnostic.
                    Err(first_divergence(
                        &to_canonical_json(&serde).unwrap_or_default(),
                        &to_json_string(&codec),
                    ))
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

/// 3. Corpus deserialize parity: every dispatchable canonical root reads to the
/// SAME typed value through the native codec and the serde path. This is where
/// the unknown-key tolerance (the RM-1.1 corpus carries keys RM 1.2 does not
/// place identically) is proven for the native reader.
#[test]
fn codec_deserialize_matches_serde_over_the_corpus() {
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
        match deser_parity_of(ty, &text) {
            Some(Ok(())) => compared += 1,
            Some(Err(e)) => failures.push((rel, e)),
            None => {}
        }
    }
    for (f, e) in failures.iter().take(20) {
        println!("\n--- DESER MISMATCH: {f}\n{e}");
    }
    assert!(
        failures.is_empty(),
        "{} corpus doc(s) deserialized differently between the serde path and the native codec",
        failures.len()
    );
    assert!(
        compared >= 50,
        "the corpus shrank unexpectedly ({compared} docs compared) — the deser parity gate lost its subject"
    );
    println!("codec/serde deserialize parity: {compared} canonical corpus roots identical");
}

/// A minimal `DV_QUANTITY` JSON (mandatory fields only).
const QTY: &str = r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm"}"#;

/// Tolerance rule 1 — **unknown wire keys are ignored** (the deliberate superset
/// of the ITS-JSON schema's `additionalProperties: false`, matching the derive).
#[test]
fn tolerance_unknown_keys_ignored() {
    let base: DvQuantity = from_json_str(QTY).unwrap();
    let with_extra: DvQuantity = from_json_str(
        r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm","not_a_field":42,"another":{"x":[1,2]}}"#,
    )
    .unwrap();
    assert_eq!(
        base, with_extra,
        "unknown keys must be ignored, not rejected"
    );
    // And identical to the serde reader's own leniency.
    let serde: DvQuantity = from_canonical_json(
        r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm","not_a_field":42,"another":{"x":[1,2]}}"#,
    )
    .unwrap();
    assert_eq!(with_extra, serde);
}

/// Tolerance rule 2 — **members may arrive out of order** (`_type` last, fields
/// permuted) and still parse to the same value.
#[test]
fn tolerance_out_of_order_members() {
    let canonical: DvQuantity = from_json_str(QTY).unwrap();
    let permuted: DvQuantity =
        from_json_str(r#"{"units":"mm","magnitude":5,"_type":"DV_QUANTITY"}"#).unwrap();
    assert_eq!(canonical, permuted);
}

/// Tolerance rule 3 — a **present-but-wrong `_type` on a concrete type is an
/// error**; an **absent `_type` is accepted** on a concrete (unambiguous) type.
#[test]
fn tolerance_concrete_type_discipline() {
    // Present-but-wrong → error (both readers agree).
    assert!(from_json_str::<DvCount>(r#"{"_type":"DV_QUANTITY","magnitude":1}"#).is_err());
    assert!(from_canonical_json::<DvCount>(r#"{"_type":"DV_QUANTITY","magnitude":1}"#).is_err());
    // Absent → accepted (the slot type is unambiguous).
    let a: DvCount = from_json_str(r#"{"magnitude":1}"#).unwrap();
    let b: DvCount = from_canonical_json(r#"{"magnitude":1}"#).unwrap();
    assert_eq!(a, b);
}

/// Tolerance rule 4 — an **abstract polymorphic slot rejects a missing `_type`**
/// and an unknown `_type`, and routes a known (incl. deep-descendant) `_type` to
/// its variant.
#[test]
fn tolerance_abstract_slot_requires_type() {
    // Missing `_type` on the abstract DATA_VALUE slot → error.
    assert!(from_json_str::<DataValue>(r#"{"value":"x"}"#).is_err());
    // Unknown `_type` → error.
    assert!(from_json_str::<DataValue>(r#"{"_type":"NOPE","value":"x"}"#).is_err());
    // A valid `_type` routes (deep descendant DV_CODED_TEXT → the DvText variant).
    let coded = r#"{"_type":"DV_CODED_TEXT","value":"x","defining_code":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"local"},"code_string":"at1"}}"#;
    let a: DataValue = from_json_str(coded).unwrap();
    let b: DataValue = from_canonical_json(coded).unwrap();
    assert_eq!(a, b);
}

/// Tolerance rule 5 — a **concrete polymorphic slot defaults a `_type`-less value
/// to the base concrete type** (DV_TEXT holds a plain DV_TEXT or a DV_CODED_TEXT).
#[test]
fn tolerance_concrete_poly_slot_defaults_type() {
    let a: DvText = from_json_str(r#"{"value":"hi"}"#).unwrap();
    let b: DvText = from_canonical_json(r#"{"value":"hi"}"#).unwrap();
    assert_eq!(
        a, b,
        "a _type-less concrete polymorphic slot defaults to base"
    );
}

/// Tolerance rule 6 — **`Option`/`Vec` defaulting** and the **`Interval` literal
/// flag defaults** are reproduced (minimal input, defaults materialized).
#[test]
fn tolerance_option_vec_and_interval_defaults() {
    // Option absent → None, Vec absent → empty: a minimal DV_TEXT.
    let a: DvTextData = from_json_str(r#"{"_type":"DV_TEXT","value":""}"#).unwrap();
    let b: DvTextData = from_canonical_json(r#"{"_type":"DV_TEXT","value":""}"#).unwrap();
    assert_eq!(a, b);
    // Interval `*_included`/`*_unbounded` flags omitted → their literal defaults.
    let json = r#"{"_type":"DV_INTERVAL","lower":{"_type":"DV_QUANTITY","magnitude":1.0,"units":"mm"},"upper":{"_type":"DV_QUANTITY","magnitude":2.0,"units":"mm"}}"#;
    let a: DvInterval<DvQuantity> = from_json_str(json).unwrap();
    let b: DvInterval<DvQuantity> = from_canonical_json(json).unwrap();
    assert_eq!(a, b);
    // The flags materialize (they round-trip through re-serialization).
    let out = to_json_string(&a);
    for flag in [
        "lower_included",
        "upper_included",
        "lower_unbounded",
        "upper_unbounded",
    ] {
        assert!(out.contains(flag), "default flag {flag} missing: {out}");
    }
}

/// Tolerance rule 7 — **RM number typing on read**: an integer lexeme in a Real
/// field widens to the field's type (so it re-serializes as `x.0`), matching the
/// serde reader; an integer field stays an integer.
#[test]
fn tolerance_number_typing_on_read() {
    let a: DvQuantity = from_json_str(QTY).unwrap();
    let b: DvQuantity = from_canonical_json(QTY).unwrap();
    assert_eq!(a, b);
    assert!(to_json_string(&a).contains(r#""magnitude":5.0"#));
    let c: DvCount = from_json_str(r#"{"_type":"DV_COUNT","magnitude":5}"#).unwrap();
    assert!(to_json_string(&c).contains(r#""magnitude":5"#) && !to_json_string(&c).contains("5.0"));
}

/// Tolerance rule 8 — **string escaping on read**: control escapes, `\uXXXX`, and
/// a surrogate-pair astral character parse identically to the serde reader.
#[test]
fn tolerance_string_escapes_and_surrogate_pairs() {
    // `\uD83D\uDE00` is the UTF-16 surrogate pair for U+1F600 (😀); the
    // tokenizer must combine it. `\u00e9` is a BMP escape (é). Plus \t \n \/ .
    let json = "{\"_type\":\"DV_TEXT\",\"value\":\"tab\\tlf\\nquote\\\"slash\\/emoji \\uD83D\\uDE00 bmp \\u00e9\"}";
    let a: DvTextData = from_json_str(json).unwrap();
    let b: DvTextData = from_canonical_json(json).unwrap();
    assert_eq!(a, b);
    assert!(
        a.value.contains('\u{1F600}'),
        "surrogate pair not decoded: {:?}",
        a.value
    );
    assert!(
        a.value.contains('\u{00e9}'),
        "\\u00e9 not decoded: {:?}",
        a.value
    );
    assert!(a.value.contains('\t') && a.value.contains('\n') && a.value.contains('/'));
}
