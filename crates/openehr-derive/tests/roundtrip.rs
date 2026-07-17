#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! Behavioural tests for `#[derive(OpenEhrType)]`.
#![allow(clippy::float_cmp, clippy::approx_constant)]

use openehr_derive::OpenEhrType;

#[derive(Debug, Clone, PartialEq, OpenEhrType)]
#[openehr(type_name = "DV_QUANTITY")]
struct DvQuantity {
    magnitude: f64,
    precision: Option<i32>,
    units: String,
    #[openehr(rename = "use")]
    use_: String,
    other_reference_ranges: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, OpenEhrType)]
#[openehr(type_name = "DV_INTERVAL")]
struct DvInterval<T> {
    lower: Option<T>,
    upper: Option<T>,
}

#[test]
fn serialize_injects_type_first_and_skips_none_and_empty() {
    let q = DvQuantity {
        magnitude: 72.0,
        precision: None,
        units: "kg".into(),
        use_: "weight".into(),
        other_reference_ranges: vec![],
    };
    let v: serde_json::Value = serde_json::to_value(&q).unwrap();
    assert_eq!(v["_type"], "DV_QUANTITY");
    assert_eq!(v["magnitude"], 72.0);
    assert_eq!(v["units"], "kg");
    assert_eq!(v["use"], "weight"); // renamed
    assert!(v.get("precision").is_none(), "None omitted");
    assert!(
        v.get("other_reference_ranges").is_none(),
        "empty Vec omitted"
    );
    // `_type` must be the first key for canonical output.
    let s = serde_json::to_string(&q).unwrap();
    assert!(s.starts_with(r#"{"_type":"DV_QUANTITY""#), "got {s}");
}

#[test]
fn deserialize_tolerates_missing_type_and_fills_defaults() {
    let json = r#"{"magnitude": 1.5, "units": "m", "use": "height"}"#;
    let q: DvQuantity = serde_json::from_str(json).unwrap();
    assert_eq!(q.magnitude, 1.5);
    assert_eq!(q.precision, None);
    assert_eq!(q.use_, "height");
    assert!(q.other_reference_ranges.is_empty());
}

#[test]
fn deserialize_validates_present_type() {
    let ok = r#"{"_type":"DV_QUANTITY","magnitude":1.0,"units":"m","use":"x"}"#;
    assert!(serde_json::from_str::<DvQuantity>(ok).is_ok());
    let bad = r#"{"_type":"DV_COUNT","magnitude":1.0,"units":"m","use":"x"}"#;
    let err = serde_json::from_str::<DvQuantity>(bad).unwrap_err();
    assert!(err.to_string().contains("DV_QUANTITY"), "got {err}");
}

#[test]
fn deserialize_missing_required_field_errors() {
    let json = r#"{"units":"m","use":"x"}"#; // magnitude missing
    let err = serde_json::from_str::<DvQuantity>(json).unwrap_err();
    assert!(err.to_string().contains("magnitude"), "got {err}");
}

#[test]
fn roundtrip_is_stable() {
    let q = DvQuantity {
        magnitude: 3.14,
        precision: Some(2),
        units: "mm".into(),
        use_: "len".into(),
        other_reference_ranges: vec!["ref1".into()],
    };
    let s = serde_json::to_string(&q).unwrap();
    let back: DvQuantity = serde_json::from_str(&s).unwrap();
    assert_eq!(q, back);
}

#[test]
fn generic_type_roundtrips() {
    let iv = DvInterval {
        lower: Some(1.0_f64),
        upper: None,
    };
    let v = serde_json::to_value(&iv).unwrap();
    assert_eq!(v["_type"], "DV_INTERVAL");
    assert_eq!(v["lower"], 1.0);
    assert!(v.get("upper").is_none());
    let back: DvInterval<f64> = serde_json::from_value(v).unwrap();
    assert_eq!(iv, back);
}
