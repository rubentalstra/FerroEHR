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

/// The `master05-content` §General Structure typical structure (its
/// `leaf_value` placeholders materialized) and the COMPLETE 9-path set
/// §Paths extracts from it, verbatim (#1377).
#[test]
fn ch5_typical_structure_and_its_complete_path_set() {
    let src = r#"attr_1 = <
    attr_2 = <
        attr_3 = <"leaf">
        attr_4 = <"leaf">
    >
    attr_5 = <
        attr_3 = <
            attr_6 = <"leaf">
        >
        attr_7 = <"leaf">
    >
>
attr_8 = <...>"#;
    let parsed = parse(src).unwrap_or_else(|e| panic!("the §5.1 structure should parse: {e}"));
    assert_eq!(
        parsed.paths(),
        vec![
            "/attr_1",
            "/attr_1/attr_2",
            "/attr_1/attr_2/attr_3",
            "/attr_1/attr_2/attr_4",
            "/attr_1/attr_5",
            "/attr_1/attr_5/attr_3",
            "/attr_1/attr_5/attr_3/attr_6",
            "/attr_1/attr_5/attr_7",
            "/attr_8",
        ]
    );
}

/// The `master05-content` §Container Objects `school_schedule` example. The
/// VERBATIM text is refused — its `weighting = <76%>` uses a percent literal
/// no leaf-data production defines (`master07-leaf_data`'s type set is
/// closed; `%` is not an ODIN token) — and the materialized twin (weighting
/// as the plain Real it is declared as) parses, with the section's two
/// listed container paths present in its path set.
#[test]
fn ch5_school_schedule_twins_and_container_paths() {
    let verbatim_weighting = "s = <weighting = <76%>>";
    let err = parse(verbatim_weighting).expect_err("`76%` is not a leaf of any production");
    assert_eq!(err.kind, OdinErrorKind::UnrecognisedToken);

    let src = r#"school_schedule = <
    lesson_times = <08:30:00, 09:30:00, 10:30:00, ...>
    locations = <
        [1] = <"under the big plane tree">
        [2] = <"under the north arch">
        [3] = <"in a garden">
    >
    subjects = <
        ["philosophy:plato"] = <
            name = <"philosophy">
            teacher = <"plato">
            topics = <"meta-physics", "natural science">
            weighting = <76.0>
        >
        ["philosophy:kant"] = <
            name = <"philosophy">
            teacher = <"kant">
            topics = <"meaning and reason", "meta-physics", "ethics">
            weighting = <80.0>
        >
        ["art"] = <
            name = <"art">
            teacher = <"goya">
            topics = <"technique", "portraiture", "satire">
            weighting = <78.0>
        >
    >
>"#;
    let parsed =
        parse(src).unwrap_or_else(|e| panic!("the materialized schedule should parse: {e}"));
    let paths = parsed.paths();
    for expected in [
        "/school_schedule/locations[1]",
        "/school_schedule/subjects[\"philosophy:kant\"]",
    ] {
        assert!(
            paths.contains(&expected.to_owned()),
            "{expected} missing from {paths:?}"
        );
    }
}

/// The `master05-content` §Nested Container Objects `List<List<String>>`
/// example, with its listed nested-key paths (`/list_of_string_lists[1]/[1]`
/// …) present in the extracted set.
#[test]
fn ch5_nested_containers_and_their_paths() {
    let src = r#"list_of_string_lists = <
    [1] = <
        [1] = <"first string in first list">
        [2] = <"second string in first list">
    >
    [2] = <
        [1] = <"first string in second list">
        [2] = <"second string in second list">
        [3] = <"third string in second list">
    >
    [3] = <
        [1] = <"only string in third list">
    >
>"#;
    let parsed = parse(src).unwrap_or_else(|e| panic!("the §5.5 example should parse: {e}"));
    let paths = parsed.paths();
    for expected in [
        "/list_of_string_lists[1]/[1]",
        "/list_of_string_lists[1]/[2]",
        "/list_of_string_lists[2]/[1]",
    ] {
        assert!(
            paths.contains(&expected.to_owned()),
            "{expected} missing from {paths:?}"
        );
    }
}

/// The `master05-content` §Adding Type Information `destinations` example
/// (placeholders materialized as the `<...>` voids the chapter itself
/// writes): dynamic-type casts at keyed and attribute positions, incl. the
/// statically-typed-attribute default (no cast on `hotels`/`attractions`)
/// and the fully-typed `(List<HOTEL>)` variant.
#[test]
fn ch5_adding_type_information_example() {
    let src = r#"destinations = <
    ["seville"] = (TOURIST_DESTINATION) <
        profile = (DESTINATION_PROFILE) <...>
        hotels = <
            ["gran sevilla"] = (HISTORIC_HOTEL) <...>
            ["sofitel"] = (LUXURY_HOTEL) <...>
            ["hotel real"] = (PENSION) <...>
        >
        attractions = <
            ["la corrida"] = (SPORT_VENUE) <...>
            ["Alcázar"] = (HISTORIC_SITE) <...>
        >
    >
>"#;
    let parsed =
        parse(src).unwrap_or_else(|e| panic!("the destinations example should parse: {e}"));
    let paths = parsed.paths();
    assert!(paths.contains(&"/destinations[\"seville\"]/hotels[\"gran sevilla\"]".to_owned()));

    let typed = parse(r#"hotels = (List<HOTEL>) <["gran sevilla"] = (HISTORIC_HOTEL) <...>>"#)
        .unwrap_or_else(|e| panic!("the fully-typed variant should parse: {e}"));
    assert!(matches!(typed, OdinValue::Object(_)));
}
