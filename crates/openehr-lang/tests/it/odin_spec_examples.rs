//! Pins the ODIN specification's own example documents against the reader:
//! the ch.2 overview exemplar of
//! `docs/specs/openehr/LANG/docs/odin/master02-overview.adoc` as its valid +
//! invalid twins (the #852 ch.2 audit, finding R1), and the ch.4 artefact
//! forms of `master04-odin_artefacts.adoc` — the schema-identifier prefix and
//! the three §Document shapes (the #854 ch.4 audit).

use openehr_lang::odin::{OdinErrorKind, OdinKey, OdinSchemaId, OdinValue, parse, parse_document};

/// The master02 exemplar VERBATIM (its lines 7–22). The line
/// `[01235] = < -- etc >` is lexically self-inconsistent with the spec's own
/// comment rule: `master03-basics.adoc` §Comments makes `--` run to end of
/// line ("Multi-line comments are achieved using the \"--\" leader on each
/// line where the comment continues"), so the comment swallows the block's
/// closing `>` and the document cannot parse. Refusing it is spec-right; the
/// invalid twin stays pinned so a lenient reader fails here.
const CH2_EXEMPLAR_VERBATIM: &str = r#"person = (List<PERSON>) <
    [01234] = <
        name = < -- person's name
            forenames = <"Sherlock">
            family_name = <"Holmes">
            salutation = <"Mr">
        >
        address = < -- person's address
            habitation_number = <"221B">
            street_name = <"Baker St">
            city = <"London">
            country = <"England">
        >
    >
    [01235] = < -- etc >
>"#;

/// The exemplar with the elliptical `-- etc` comment materialized on its own
/// line — the reading master02 illustrates once the master03 §Comments rule
/// is applied consistently. Everything else is byte-identical to the chapter
/// text.
const CH2_EXEMPLAR_MATERIALIZED: &str = r#"person = (List<PERSON>) <
    [01234] = <
        name = < -- person's name
            forenames = <"Sherlock">
            family_name = <"Holmes">
            salutation = <"Mr">
        >
        address = < -- person's address
            habitation_number = <"221B">
            street_name = <"Baker St">
            city = <"London">
            country = <"England">
        >
    >
    [01235] = < -- etc
    >
>"#;

#[test]
fn ch2_exemplar_verbatim_is_refused() {
    let err = parse(CH2_EXEMPLAR_VERBATIM).expect_err(
        "the verbatim master02 exemplar must be refused: the `-- etc` comment \
         swallows its block's closing `>` (master03 §Comments)",
    );
    assert_eq!(err.kind, OdinErrorKind::UnexpectedToken);
}

#[test]
fn ch2_exemplar_materialized_parses_to_the_expected_tree() {
    let parsed = parse(CH2_EXEMPLAR_MATERIALIZED)
        .unwrap_or_else(|e| panic!("materialized master02 exemplar should parse: {e}"));

    let OdinValue::Object(top) = parsed else {
        panic!("expected a top-level attribute object, got {parsed:?}");
    };
    let OdinValue::Typed { rm_type, value } = top.get("person").expect("person attribute") else {
        panic!("expected `person` to carry the (List<PERSON>) cast");
    };
    assert_eq!(rm_type, "List<PERSON>");

    let OdinValue::KeyedList(entries) = value.as_ref() else {
        panic!("expected a keyed container under the cast, got {value:?}");
    };
    assert_eq!(entries.len(), 2, "two container items, [01234] and [01235]");

    let OdinValue::Object(person) = &entries[0].1 else {
        panic!("expected the first container item to be an object");
    };
    let OdinValue::Object(name) = person.get("name").expect("name attribute") else {
        panic!("expected `name` to be an object");
    };
    for (attr, expected) in [
        ("forenames", "Sherlock"),
        ("family_name", "Holmes"),
        ("salutation", "Mr"),
    ] {
        assert_eq!(
            name.get(attr),
            Some(&OdinValue::String(expected.to_owned())),
            "name/{attr}"
        );
    }
    let OdinValue::Object(address) = person.get("address").expect("address attribute") else {
        panic!("expected `address` to be an object");
    };
    for (attr, expected) in [
        ("habitation_number", "221B"),
        ("street_name", "Baker St"),
        ("city", "London"),
        ("country", "England"),
    ] {
        assert_eq!(
            address.get(attr),
            Some(&OdinValue::String(expected.to_owned())),
            "address/{attr}"
        );
    }

    assert_eq!(
        entries[1].1,
        OdinValue::Empty,
        "the elliptical second item is an empty block"
    );
}

/// The `master04-odin_artefacts` §Embedded Fragment example, with the
/// illustrative `leaf_value` barewords materialized as real leaves (the
/// verbatim placeholder form is pinned refused by the vendored
/// `anonymous_odin.txt` adjudication in `vendor_odin.rs`). Fragments carry
/// "no object identifiers nor schema identifier".
#[test]
fn ch4_embedded_fragment_form_parses() {
    let src = r#"--
-- ODIN Embedded Fragment
--
    attr_1 = <
        attr_12 = <
            attr_13 = <"leaf">
        >
    >
    attr_2 = <
        attr_22 = <"leaf">
    >
"#;
    let parsed = parse(src).unwrap_or_else(|e| panic!("the fragment form should parse: {e}"));
    let OdinValue::Object(top) = parsed else {
        panic!("expected an attribute object, got {parsed:?}");
    };
    assert_eq!(top.len(), 2);
}

/// The `master04-odin_artefacts` §Anonymous Object Document form — an outer
/// `<>` pair around an embedded fragment; "syntactically more correct, and
/// should be supported by parsers".
#[test]
fn ch4_anonymous_object_document_form_parses() {
    let src = r#"--
-- ODIN Anonymous Object Document
--
<
    attr_1 = <
        attr_12 = <
            attr_13 = <"leaf">
        >
    >
    attr_2 = <
        attr_22 = <"leaf">
    >
>
"#;
    let parsed = parse(src).unwrap_or_else(|e| panic!("the anonymous form should parse: {e}"));
    assert!(
        matches!(parsed, OdinValue::Object(_)),
        "the outer block holds the fragment's attribute object"
    );
}

/// The `master04-odin_artefacts` §Identified Object Document form —
/// top-level keyed objects, one per identified object (the docs-text
/// production the vendored `odin.g4` start rule lacks; #1374). The verbatim
/// chapter example keeps a bare top-level `...` ellipsis line and stays
/// refused (pinned by the `identified_object_document.txt` adjudication in
/// `vendor_odin.rs`); this is its materialized twin.
#[test]
fn ch4_identified_object_document_form_parses() {
    let src = r#"--
-- ODIN Identified Object Document
--
["id_1"] = <
    attr_1 = <
        attr_12 = <
            attr_13 = <"leaf">
        >
    >
>

["id_2"] = <
    attr_1 = <
        attr_12 = <
            attr_13 = <"leaf">
        >
    >
>

["id_N"] = <
    attr_1 = <
        attr_12 = <
            attr_13 = <"leaf">
        >
    >
>
"#;
    let parsed = parse(src).unwrap_or_else(|e| panic!("the identified form should parse: {e}"));
    let OdinValue::KeyedList(entries) = parsed else {
        panic!("expected a top-level keyed list, got {parsed:?}");
    };
    assert_eq!(
        entries.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
        vec![
            OdinKey::String("id_1".to_owned()),
            OdinKey::String("id_2".to_owned()),
            OdinKey::String("id_N".to_owned()),
        ]
    );
}

/// "Identifiers can be values of the String, Integer or any Date/Time
/// primitive types" (`master04-odin_artefacts` §Identified Object Document)
/// — the non-String identifier types at top level.
#[test]
fn ch4_identified_document_identifier_types() {
    let parsed = parse("[1] = <a = <1>>\n[2004-06-11] = <a = <2>>\n[10:30:00] = <a = <3>>")
        .unwrap_or_else(|e| panic!("typed identifiers should parse: {e}"));
    let OdinValue::KeyedList(entries) = parsed else {
        panic!("expected a keyed list");
    };
    assert_eq!(
        entries.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
        vec![
            OdinKey::Integer(1),
            OdinKey::Date("2004-06-11".to_owned()),
            OdinKey::Time("10:30:00".to_owned()),
        ]
    );
}

/// `odin_text ::= ( schema_identifier )? main_text` with
/// `schema_identifier ::= '@' schema '=' URI`
/// (`master04-odin_artefacts` intro; #1373 — the chapter gives no example,
/// so the shape here is the adjudicated reading recorded at the parser
/// site). The prefix composes with every main-text form, `parse_document`
/// preserves it, and `parse` discards it.
#[test]
fn ch4_schema_identifier_prefix() {
    let doc = parse_document("@schema = <http://openehr.org/bmm/1.0>\na = <1>")
        .unwrap_or_else(|e| panic!("the schema-identified fragment should parse: {e}"));
    assert_eq!(
        doc.schema,
        Some(OdinSchemaId {
            name: "schema".to_owned(),
            uri: "<http://openehr.org/bmm/1.0>".to_owned(),
        })
    );
    assert!(matches!(doc.root, OdinValue::Object(_)));

    // …with the anonymous and identified forms too.
    for src in [
        "@schema = <http://x.org/s>\n<a = <1>>",
        "@schema = <http://x.org/s>\n[\"id\"] = <a = <1>>",
    ] {
        let doc = parse_document(src)
            .unwrap_or_else(|e| panic!("the schema prefix should compose with {src:?}: {e}"));
        assert!(doc.schema.is_some(), "{src}");
    }

    // `parse` accepts and discards the prefix.
    let root = parse("@schema = <http://x.org/s>\na = <1>")
        .unwrap_or_else(|e| panic!("parse should tolerate the prefix: {e}"));
    assert!(matches!(root, OdinValue::Object(_)));

    // a misplaced `@` is still a parse error.
    assert!(parse("a = <1> @ b").is_err());
}
