//! Pins the ODIN specification's own example documents against the reader —
//! currently the ch.2 overview exemplar of
//! `docs/specs/openehr/LANG/docs/odin/master02-overview.adoc`, as its valid +
//! invalid twins (the #852 ch.2 audit, finding R1).

use openehr_lang::odin::{OdinErrorKind, OdinValue, parse};

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
