//! The parser's error contract: a caller branches on the failure, and reads
//! the located diagnostic only to show it to a human.

use openehr_query::lexer::LexError;
use openehr_query::parser::{ParseError, SyntaxFault, parse_str};

/// A lex failure and a syntax failure are DIFFERENT variants, so telling them
/// apart never means matching a substring — the property the typed error
/// exists for.
#[test]
fn the_two_passes_are_distinguishable_without_reading_the_message() {
    // `#` is in no AQL token, so the lexer refuses before the parser runs.
    let lexed = parse_str("SELECT # FROM EHR e").expect_err("`#` lexes as nothing");
    assert!(
        matches!(lexed, ParseError::Lex(_)),
        "a character outside every token is a lex failure, got {lexed:?}"
    );

    // Every token here is valid AQL; the sequence is not.
    let syntax = parse_str("SELECT FROM WHERE").expect_err("keywords in that order are no query");
    assert!(
        matches!(syntax, ParseError::Syntax { .. }),
        "well-formed tokens in an invalid order are a syntax failure, got {syntax:?}"
    );
}

/// The lex variant carries the lexer's own error as its `source`, so the cause
/// chain is walkable rather than flattened into text (RFC 0201).
#[test]
fn a_lex_failure_carries_the_lexer_error_as_its_source() {
    let err = parse_str("SELECT # FROM EHR e").expect_err("`#` lexes as nothing");
    let source = std::error::Error::source(&err).expect("the lex variant carries its cause");
    let lex: &LexError = source
        .downcast_ref()
        .expect("and that cause is the lexer's own error type");
    assert_eq!(lex.slice, "#", "the offending slice survives the hop");

    // Display is unchanged by the wrapping: the message a client sees is still
    // the lexer's located diagnostic verbatim.
    assert_eq!(err.to_string(), lex.to_string());
}

/// A syntax failure carries the position and the token found, so a caller can
/// act on WHERE the query broke instead of parsing a sentence.
#[test]
fn a_syntax_failure_carries_its_position_and_the_token_found() {
    let err = parse_str("SELECT FROM WHERE").expect_err("keywords in that order are no query");
    let ParseError::Syntax { faults } = err else {
        panic!("expected a syntax failure, got {err:?}");
    };
    let SyntaxFault { tokens, found } = faults.first().expect("at least one reported position");
    assert!(
        tokens.start < tokens.end,
        "the reported token range is non-empty: {tokens:?}"
    );
    assert!(
        found.is_some(),
        "the input did not end early, so a token was found"
    );
}

/// An empty source ends before any token, which the parser reports as a
/// position with no token found rather than as a lex failure.
#[test]
fn end_of_input_is_reported_with_no_token_found() {
    let err = parse_str("").expect_err("an empty source is not a query");
    let ParseError::Syntax { faults } = err else {
        panic!("an empty source lexes cleanly and fails the grammar, got {err:?}");
    };
    assert!(
        faults.iter().any(|f| f.found.is_none()),
        "the end-of-input position is reported: {faults:?}"
    );
}
