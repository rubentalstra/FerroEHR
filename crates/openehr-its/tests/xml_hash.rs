//! Verify `Hash<String,String>` (`StringDictionaryItem`) XML round-trips.
use openehr_base::prelude::TranslationDetails;
use openehr_its::xml::{from_xml, to_canonical_xml};

#[test]
fn string_hash_round_trips() {
    // TRANSLATION_DETAILS.author is Hash<String,String>.
    let json = r#"{
        "_type": "TRANSLATION_DETAILS",
        "language": {"terminology_id": "ISO_639-1", "code_string": "de"},
        "author": {"name": "Dr. Ada", "organisation": "openEHR"}
    }"#;
    let td: TranslationDetails = serde_json::from_str(json).expect("deserialize JSON");
    let xml = to_canonical_xml(&td, "translations").expect("serialize");
    eprintln!("{xml}");
    assert!(
        xml.contains("<author id=\"name\">Dr. Ada</author>"),
        "kv shape: {xml}"
    );
    assert!(xml.contains("<author id=\"organisation\">openEHR</author>"));
    let td2: TranslationDetails = from_xml(&xml).expect("parse");
    assert_eq!(td2.author.get("name").map(String::as_str), Some("Dr. Ada"));
    assert_eq!(
        td2.author.get("organisation").map(String::as_str),
        Some("openEHR")
    );
    // full XML round-trip stable
    let xml2 = to_canonical_xml(&td2, "translations").expect("serialize 2");
    assert_eq!(xml, xml2);
}
