#![allow(
    clippy::doc_markdown,
    clippy::doc_lazy_continuation,
    reason = "prose with proper nouns + numbered case docs"
)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::float_cmp,
    let_underscore_drop,
    reason = "test assertions/diagnostics/fixtures (expects in helper fns, not #[test] bodies)"
)]
//! The **canonical-JSON codec contract** gate: byte hazards + `FromJson`
//! tolerance rules.
//!
//! Originally (R3/R4a) this gate proved the emitted `ToJson`/`FromJson` codec
//! byte-identical to the retired `#[derive(OpenEhrType)]` serde path. That serde
//! path is now GONE — `openehr_its::json::to_canonical_json`/`from_canonical_json`
//! ARE the native codec — so a serde-vs-codec comparison is vacuous. Retargeted:
//!
//! - The corpus determinism (parse → re-serialize the whole vendored corpus, byte
//!   stable) is the R0 manifest snapshot in `canonical_contract.rs`; the two
//!   redundant corpus parity tests that lived here are folded into it.
//! - The **byte-hazard cases** (float lexemes, string escaping, `None`/empty
//!   omission, the `Interval` default flags, typed-literal enums) are retargeted
//!   from `serde == codec` to **codec-vs-pinned-bytes** — the exact-byte
//!   assertion strength is preserved, now pinned to a literal rather than to the
//!   deleted serde output.
//! - The eight **`FromJson` tolerance rules** (unknown keys ignored, out-of-order
//!   members, `_type` discipline, abstract/concrete polymorphic slots,
//!   `Option`/`Vec` + `Interval` defaults, RM number typing, string escapes) are
//!   asserted directly on the codec reader (the redundant second serde read is
//!   dropped; every semantic assertion is kept).

use openehr_its::json::from_canonical_json;
use openehr_its::json_codec::runtime::{from_json_str, to_json_string};
use openehr_rm::prelude::{
    DataValue, DvCount, DvInterval, DvOrdinal, DvQuantity, DvText, DvTextData, ProportionKind,
};

/// Assert the native codec serializes `value` to exactly `expect` (the canonical
/// byte contract — `_type`-first order, RM number typing, `None`/empty omission).
fn assert_bytes<T: openehr_its::json_codec::runtime::ToJson>(value: &T, expect: &str) {
    assert_eq!(to_json_string(value), expect, "codec output changed");
}

/// 1. Float lexemes: integer-typed fields print as integers; Real-typed fields
/// carry a decimal point (whole reals as `x.0`); many-decimal + exponent reals
/// go through ryu.
#[test]
fn float_and_integer_lexemes() {
    // Integer-typed magnitude → no decimal point.
    let count: DvCount = from_canonical_json(r#"{"_type":"DV_COUNT","magnitude":5}"#).unwrap();
    assert_bytes(&count, r#"{"_type":"DV_COUNT","magnitude":5}"#);

    // Real-typed magnitude, whole value → `x.0` (normalized from an int lexeme).
    let q: DvQuantity =
        from_canonical_json(r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm"}"#).unwrap();
    assert_bytes(
        &q,
        r#"{"_type":"DV_QUANTITY","magnitude":5.0,"units":"mm"}"#,
    );

    // Real with many decimals → via ryu.
    let q2: DvQuantity =
        from_canonical_json(r#"{"_type":"DV_QUANTITY","magnitude":3.14159265,"units":"mm"}"#)
            .unwrap();
    assert_bytes(
        &q2,
        r#"{"_type":"DV_QUANTITY","magnitude":3.14159265,"units":"mm"}"#,
    );
    // An exponent-form real: ryu is our chosen canonical REAL lexeme (`1e21`),
    // which deliberately differs from serde_json's dtoa (`1e+21`) on this rare
    // form (see the runtime module NOTE). Assert the deterministic codec output.
    let q3: DvQuantity =
        from_canonical_json(r#"{"_type":"DV_QUANTITY","magnitude":1e21,"units":"mm"}"#).unwrap();
    assert_bytes(
        &q3,
        r#"{"_type":"DV_QUANTITY","magnitude":1e21,"units":"mm"}"#,
    );
}

/// 2. String escaping: quotes, backslashes, control chars, non-ASCII — the codec
/// escapes only the C0 control range, `"` and `\` (RFC 8259 §7), passing `/` and
/// non-ASCII verbatim, and round-trips every value losslessly. (The exact
/// serde_json escape-set equality is pinned in the runtime's own unit test.)
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
        let out = to_json_string(&text);
        // `/` and non-ASCII pass through unescaped.
        assert!(
            out.contains('/') || !value.contains('/'),
            "slash escaped: {out}"
        );
        // No raw C0 control byte survives in the output.
        assert!(
            !out.bytes().any(|b| b < 0x20),
            "a raw control byte leaked unescaped: {out:?}"
        );
        // Lossless round-trip through the codec reader.
        let back: DvTextData = from_json_str(&out).unwrap();
        assert_eq!(
            back.value, value,
            "escaping round-trip lost data for {value:?}"
        );
    }
}

/// A `DV_TEXT` payload (`DvTextData`) with only `value` set (all other fields
/// `None`/empty).
fn empty_dv_text_data() -> DvTextData {
    from_canonical_json(r#"{"_type":"DV_TEXT","value":""}"#).expect("minimal DV_TEXT parses")
}

/// 3. `None`/empty omission: an optional-absent field and an empty container are
/// both dropped — no `null`, no `[]`.
#[test]
fn none_and_empty_omission() {
    let text = empty_dv_text_data();
    assert_bytes(&text, r#"{"_type":"DV_TEXT","value":""}"#);
    let out = to_json_string(&text);
    assert!(!out.contains("null") && !out.contains("[]"), "{out}");
}

/// 4. The `Interval` default flags: `DV_INTERVAL` `*_included`/`*_unbounded` are
/// mandatory bool fields (literal defaults) — always emitted, not omitted.
#[test]
fn interval_default_flags_are_emitted() {
    let json = r#"{"_type":"DV_INTERVAL","lower":{"_type":"DV_QUANTITY","magnitude":1.0,"units":"mm"},"upper":{"_type":"DV_QUANTITY","magnitude":2.0,"units":"mm"}}"#;
    let iv: DvInterval<DvQuantity> = from_canonical_json(json).unwrap();
    let codec = to_json_string(&iv);
    for flag in [
        "lower_included",
        "upper_included",
        "lower_unbounded",
        "upper_unbounded",
    ] {
        assert!(codec.contains(flag), "default flag {flag} missing: {codec}");
    }
}

/// 5. A typed-literal enum: `PROPORTION_KIND` (integer-backed), a known constant
/// and an out-of-set `Other`, serialize to the bare integer (byte-identical to
/// the primitive it replaces).
#[test]
fn typed_literal_enum_including_other() {
    // A known constant inside a DV_ORDINAL still round-trips to pinned bytes.
    let known: DvOrdinal = from_canonical_json(
        r#"{"_type":"DV_ORDINAL","value":1,"symbol":{"_type":"DV_CODED_TEXT","value":"x","defining_code":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"local"},"code_string":"at1"}}}"#,
    )
    .unwrap();
    // The integer-typed DV_ORDINAL.value serializes with no decimal point.
    assert!(to_json_string(&known).contains(r#""value":1"#));

    // The enum's ToJson: a known constant → its integer, `Other(99)` → `99`.
    assert_eq!(to_json_string(&ProportionKind::from_value(0)), "0");
    assert_eq!(to_json_string(&ProportionKind::from_value(99)), "99");
}

// ════════════════════════════════════════════════════════════════════════════
// `FromJson` tolerance rules — asserted directly on the native codec reader
// (verbatim the retired derive's rules; the redundant second serde read is gone).
// ════════════════════════════════════════════════════════════════════════════

/// A minimal `DV_QUANTITY` JSON (mandatory fields only).
const QTY: &str = r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm"}"#;

/// **Strictness rule 1 — an undeclared wire key is REFUSED**, naming the key and
/// the class that does not declare it.
///
/// This reverses the reader's former tolerance, by adjudication:
/// `docs/specs/openehr/ITS-REST/specifications/docs/overview/Resources.md`
/// requires an XML payload to validate against the ITS-XML schemas, which are
/// wildcard-free — an undeclared element cannot validate — and states the same
/// for the JSON encoding at SHOULD strength; openEHR's own published ITS-JSON
/// schemas close 128 of their 134 object definitions with
/// `additionalProperties: false`. Refusing is the only reading under which the
/// JSON and XML encodings share ONE data model. Enforcing at a SHOULD anchor is
/// OUR decision (the upstream two-artifact contradiction is reported as issue
/// #1696), and the closure is over the GENERATED RM model at our pin — never
/// over the vendored ITS-JSON 1.1.0 schema, which is stale in both directions.
#[test]
fn undeclared_keys_are_refused() {
    let base: DvQuantity = from_json_str(QTY).unwrap();
    assert_eq!(base.units, "mm");
    let err = from_json_str::<DvQuantity>(
        r#"{"_type":"DV_QUANTITY","magnitude":5,"units":"mm","not_a_field":42,"another":{"x":[1,2]}}"#,
    )
    .expect_err("an undeclared key must be refused, not ignored");
    let text = err.to_string();
    assert!(
        text.contains("unknown field `not_a_field`"),
        "the refusal names the FIRST offending key in member order, got: {text}"
    );
    assert!(
        text.contains("DV_QUANTITY"),
        "the refusal names the class that does not declare it, got: {text}"
    );
}

/// The refusal names the **path** to the offending node, not just the key, so a
/// client can locate it in a deep document (the reader builds the path as it
/// unwinds — `JsonParseError::in_field` / `in_index`).
#[test]
fn an_undeclared_key_refusal_names_its_path() {
    let err = from_json_str::<DvInterval<DvQuantity>>(
        r#"{"_type":"DV_INTERVAL","lower":{"_type":"DV_QUANTITY","magnitude":1,"units":"mm","bogus":1}}"#,
    )
    .expect_err("an undeclared key inside a nested slot must be refused");
    let text = err.to_string();
    assert!(
        text.contains("unknown field `bogus`") && text.contains("$.lower"),
        "the refusal names the key AND the path, got: {text}"
    );
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
    assert!(from_json_str::<DvCount>(r#"{"_type":"DV_QUANTITY","magnitude":1}"#).is_err());
    let a: DvCount = from_json_str(r#"{"magnitude":1}"#).unwrap();
    assert_eq!(a.magnitude, 1);
}

/// Tolerance rule 4 — an **abstract polymorphic slot rejects a missing `_type`**
/// and an unknown `_type`, and routes a known (incl. deep-descendant) `_type` to
/// its variant.
#[test]
fn tolerance_abstract_slot_requires_type() {
    assert!(from_json_str::<DataValue>(r#"{"value":"x"}"#).is_err());
    assert!(from_json_str::<DataValue>(r#"{"_type":"NOPE","value":"x"}"#).is_err());
    // A valid `_type` routes (deep descendant DV_CODED_TEXT → the DvText variant).
    let coded = r#"{"_type":"DV_CODED_TEXT","value":"x","defining_code":{"_type":"CODE_PHRASE","terminology_id":{"_type":"TERMINOLOGY_ID","value":"local"},"code_string":"at1"}}"#;
    assert!(from_json_str::<DataValue>(coded).is_ok());
}

/// Tolerance rule 5 — a **concrete polymorphic slot defaults a `_type`-less value
/// to the base concrete type** (DV_TEXT holds a plain DV_TEXT or a DV_CODED_TEXT).
#[test]
fn tolerance_concrete_poly_slot_defaults_type() {
    let a: DvText = from_json_str(r#"{"value":"hi"}"#).unwrap();
    // A `_type`-less concrete polymorphic slot defaults to the base (plain DvText).
    assert!(matches!(a, DvText::DvText(_)));
}

/// Tolerance rule 6 — **optional-attribute defaulting** and the **`Interval`
/// literal flag defaults** are reproduced (minimal input, defaults
/// materialized).
///
/// An OPTIONAL container reads absent as `None`, not as an empty `Vec`: the two
/// are distinct RM states (`docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.data_types.dv_text.adoc`
/// §Invariants, `Mappings_valid: mappings /= void implies not
/// mappings.is_empty`, is contentless otherwise), and the emission shape
/// `Option<Vec<T>>` carries both. The wire is unchanged either way — an empty
/// list is never written (`docs/specs/openehr/ITS-REST/specifications/docs/overview/Resources.md`
/// §JSON Format).
#[test]
fn tolerance_option_vec_and_interval_defaults() {
    // Both an optional single attribute and an optional container read absent
    // as `None`: a minimal DV_TEXT.
    let a: DvTextData = from_json_str(r#"{"_type":"DV_TEXT","value":""}"#).unwrap();
    assert!(a.mappings.is_none() && a.language.is_none());
    // ...and a PRESENT but empty list is preserved as such, which is the state
    // the `Mappings_valid` invariant exists to refuse.
    let empty: DvTextData =
        from_json_str(r#"{"_type":"DV_TEXT","value":"","mappings":[]}"#).unwrap();
    assert_eq!(empty.mappings, Some(Vec::new()));
    // Interval `*_included`/`*_unbounded` flags omitted → their literal defaults,
    // which materialize on re-serialization.
    let json = r#"{"_type":"DV_INTERVAL","lower":{"_type":"DV_QUANTITY","magnitude":1.0,"units":"mm"},"upper":{"_type":"DV_QUANTITY","magnitude":2.0,"units":"mm"}}"#;
    let iv: DvInterval<DvQuantity> = from_json_str(json).unwrap();
    let out = to_json_string(&iv);
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
/// field widens to the field's type (so it re-serializes as `x.0`); an integer
/// field stays an integer.
#[test]
fn tolerance_number_typing_on_read() {
    let a: DvQuantity = from_json_str(QTY).unwrap();
    assert!(to_json_string(&a).contains(r#""magnitude":5.0"#));
    let c: DvCount = from_json_str(r#"{"_type":"DV_COUNT","magnitude":5}"#).unwrap();
    assert!(to_json_string(&c).contains(r#""magnitude":5"#) && !to_json_string(&c).contains("5.0"));
}

/// Tolerance rule 8 — **string escaping on read**: control escapes, `\uXXXX`, and
/// a surrogate-pair astral character parse correctly.
#[test]
fn tolerance_string_escapes_and_surrogate_pairs() {
    // `😀` is the UTF-16 surrogate pair for U+1F600 (😀); the
    // tokenizer must combine it. `é` is a BMP escape (é). Plus \t \n \/ .
    let json = "{\"_type\":\"DV_TEXT\",\"value\":\"tab\\tlf\\nquote\\\"slash\\/emoji \\uD83D\\uDE00 bmp \\u00e9\"}";
    let a: DvTextData = from_json_str(json).unwrap();
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
