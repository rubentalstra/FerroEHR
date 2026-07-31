//! ODIN parser — a `chumsky` parser over the [`super::lexer`] token stream,
//! transcribed from `odin.g4` / `odin_values.g4`. Produces an
//! [`OdinValue`] tree.

use chumsky::DefaultExpected;
use chumsky::error::{Error, LabelError};
use chumsky::prelude::*;
use chumsky::util::MaybeRef;
use indexmap::IndexMap;

use super::lexer::{Spanned, Token};
use super::{OdinInterval, OdinKey, OdinValue};

// The chumsky extra-parameter alias. `chumsky::extra::Err` stays fully
// qualified deliberately: shortening it to `Err<Failure>` would make this
// alias refer to itself.
#[expect(
    unused_qualifications,
    reason = "the local alias shadows the name being qualified — dropping the path makes the definition self-referential"
)]
type Err<'a> = chumsky::extra::Err<Failure>;

/// A parse failure: a plain syntactic conflict, or the typed
/// duplicate-sibling-attribute violation.
#[derive(Debug, Clone)]
pub(super) enum Failure {
    /// An unexpected token at the given token-index span.
    Syntax(SimpleSpan),
    /// VDATU: two sibling attributes share a name
    /// (`LANG/docs/odin/master05-content` §General Structure).
    DuplicateAttribute {
        /// The repeated attribute name.
        name: String,
        /// Token-index span of the offending (second) occurrence.
        span: SimpleSpan,
    },
}

impl Failure {
    /// The token-index span the failure points at.
    fn span(&self) -> SimpleSpan {
        match self {
            Self::Syntax(s) | Self::DuplicateAttribute { span: s, .. } => *s,
        }
    }

    /// Keep whichever failure carries the most information: a typed
    /// duplicate-attribute violation always outranks a bare syntax conflict
    /// (the enclosing `choice`s retry other alternatives after it, and each of
    /// those contributes a syntax error at the same position).
    fn preferred(self, other: Self) -> Self {
        match (&self, &other) {
            (Self::DuplicateAttribute { .. }, _) => self,
            (_, Self::DuplicateAttribute { .. }) => other,
            _ => self,
        }
    }
}

impl<'a> Error<'a, &'a [Token]> for Failure {
    fn merge(self, other: Self) -> Self {
        self.preferred(other)
    }
}

impl<'a> LabelError<'a, &'a [Token], DefaultExpected<'a, Token>> for Failure {
    fn expected_found<E: IntoIterator<Item = DefaultExpected<'a, Token>>>(
        _expected: E,
        _found: Option<MaybeRef<'a, Token>>,
        span: SimpleSpan,
    ) -> Self {
        Self::Syntax(span)
    }

    fn merge_expected_found<E: IntoIterator<Item = DefaultExpected<'a, Token>>>(
        self,
        _expected: E,
        _found: Option<MaybeRef<'a, Token>>,
        span: SimpleSpan,
    ) -> Self {
        self.preferred(Self::Syntax(span))
    }

    fn replace_expected_found<E: IntoIterator<Item = DefaultExpected<'a, Token>>>(
        self,
        _expected: E,
        _found: Option<MaybeRef<'a, Token>>,
        span: SimpleSpan,
    ) -> Self {
        self.preferred(Self::Syntax(span))
    }
}

/// A parse failure resolved to a byte offset in the original source.
pub(super) struct Located {
    /// The failure.
    pub(super) failure: Failure,
    /// Byte offset of the failure in the original source.
    pub(super) offset: usize,
}

/// Parse a spanned ODIN token stream into an [`OdinValue`].
///
/// # Errors
/// Returns the first failure, resolved to a byte offset in the original
/// source.
pub(super) fn parse_tokens(spanned: &[Spanned]) -> Result<OdinValue, Located> {
    let tokens: Vec<Token> = spanned.iter().map(|s| s.token.clone()).collect();
    odin_text().parse(&tokens).into_result().map_err(|errs| {
        let failure = errs
            .into_iter()
            .reduce(Failure::preferred)
            .unwrap_or(Failure::Syntax(SimpleSpan::new(
                (),
                spanned.len()..spanned.len(),
            )));
        let idx = failure.span().start;
        let offset = spanned
            .get(idx)
            .map(|s| s.span.start)
            .or_else(|| spanned.last().map(|s| s.span.end))
            .unwrap_or(0);
        Located { failure, offset }
    })
}

/// `odin_text : attr_vals | object_value_block`.
fn odin_text<'a>() -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> {
    let block = object_block();
    let attrs = attr_vals(block.clone());
    choice((attrs, block)).then_ignore(end())
}

/// `attr_vals : ( attr_val ';'? )+` → [`OdinValue::Object`].
///
/// Sibling attribute names must be unique — `LANG/docs/odin/master05-content`
/// §General Structure rule *VDATU*: "attribute name uniqueness: sibling
/// attributes occurring within an object node must be uniquely named with
/// respect to each other, in the same way as in class definitions in an
/// information model", stated as the principle "Sibling attribute names must
/// be unique" in `AM/docs/ADL1.4/master04-dadl` §General Form. A repeat is a
/// typed [`Failure::DuplicateAttribute`], never a silent last-one-wins
/// overwrite.
fn attr_vals<'a>(
    block: impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone {
    let key = select! {
        Token::AlphaUcId(s) => s,
        Token::AlphaLcId(s) => s,
        Token::AlphaUnderscoreId(s) => s,
    };
    key.then_ignore(just(Token::Eq))
        .then(block)
        .then_ignore(just(Token::SemiColon).or_not())
        .map_with(|pair, e| (pair, e.span()))
        .repeated()
        .at_least(1)
        .collect::<Vec<((String, OdinValue), SimpleSpan)>>()
        .try_map(|pairs, _| {
            let mut map: IndexMap<String, OdinValue> = IndexMap::with_capacity(pairs.len());
            for ((name, value), span) in pairs {
                if map.insert(name.clone(), value).is_some() {
                    return Err(Failure::DuplicateAttribute { name, span });
                }
            }
            Ok(OdinValue::Object(map))
        })
}

/// `object_block : object_value_block | object_reference_block` (+ the
/// `EMBEDDED_URI` and typed-cast forms).
fn object_block<'a>() -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone {
    recursive(|block| {
        // keyed list: ( '[' key_id ']' '=' object_block )+
        let key_id = select! {
            Token::String(s) => OdinKey::String(decode_string(&s)),
            Token::Integer(s) => OdinKey::Integer(s.parse::<i64>().unwrap_or(0)),
            Token::Date(s) => OdinKey::Date(s),
            Token::Time(s) => OdinKey::Time(s),
            Token::DateTime(s) => OdinKey::DateTime(s),
        };
        let keyed_list = just(Token::LBracket)
            .ignore_then(key_id)
            .then_ignore(just(Token::RBracket))
            .then_ignore(just(Token::Eq))
            .then(block.clone())
            .repeated()
            .at_least(1)
            .collect::<Vec<(OdinKey, OdinValue)>>()
            .map(OdinValue::KeyedList);

        // object_reference_block : odin_path ( (',' odin_path)+ | '...' )?
        let path = select! { Token::Path(s) => s }.or(just(Token::Slash).to("/".to_owned()));
        let ref_list = path
            .clone()
            .then(
                choice((
                    just(Token::Comma)
                        .ignore_then(path)
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<String>>(),
                    just(Token::ListContinue).to(Vec::new()),
                ))
                .or_not(),
            )
            .map(|(first, rest)| {
                let mut v = vec![first];
                if let Some(mut more) = rest {
                    v.append(&mut more);
                }
                OdinValue::PathList(v)
            });

        // `<...>` — an empty section / void object, legal at any level.
        // `AM/docs/ADL1.4/master04-dadl` §Empty Sections: "Empty sections are
        // allowed at both internal and leaf node levels … Nested empty
        // sections can be used", with the principle "Empty sections can appear
        // anywhere"; `LANG/docs/odin/master05-content` §Void Objects: "A void
        // object, i.e. an object attribute that has no value is allowed in an
        // ODIN text, but ignored by parsers."
        //
        // NOTE: it maps to [`OdinValue::Empty`], the same value as `<>` —
        // both spec passages define the construct as "this attribute exists
        // and has no data", so no consumer can act on the distinction between
        // the two spellings, and a separate variant would force every match
        // arm in every consumer to handle a second no-data case.
        let void = just(Token::ListContinue).to(OdinValue::Empty);

        let inner = choice((
            void,
            ref_list,
            keyed_list,
            attr_vals(block.clone()),
            primitive_object(),
        ));

        let value_block = rm_type_id()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .or_not()
            .then(
                inner
                    .or_not()
                    .delimited_by(just(Token::Lt), just(Token::Gt)),
            )
            .map(|(cast, inner_opt)| {
                let val = inner_opt.unwrap_or(OdinValue::Empty);
                match cast {
                    Some(rm_type) => OdinValue::Typed {
                        rm_type,
                        value: Box::new(val),
                    },
                    None => val,
                }
            });

        let uri = select! { Token::EmbeddedUri(s) => OdinValue::Uri(s) };

        choice((uri, value_block))
    })
}

/// `rm_type_id : ( package_id '.' )* ALPHA_UC_ID ( '<' rm_type_id ( ','
/// rm_type_id )* '>' )?`, reconstructed as a flat string (e.g.
/// `Interval<Quantity>`, `org.openehr.rm.ehr.content.ENTRY`).
///
/// NOTE: the vendored `odin.g4` writes this rule as bare
/// `ALPHA_UC_ID ( '<' rm_type_id ( ',' rm_type_id )* '>' )?` — no namespace
/// form. The docs text, which is the oracle where it and a grammar artefact
/// disagree, allows the qualified spelling on any type identifier:
/// "Type identifiers can also include namespace information, which is
/// necessary when same-named types appear in different packages of a model.
/// Namespaces are included by prepending package names, separated by the '.'
/// character, in the same way as in most programming languages, as in the
/// qualified type names `org.openehr.rm.ehr.content.ENTRY` and
/// `Core.Abstractions.Relationships.Relationship`"
/// (`LANG/docs/odin/master05-content` §Adding Type Information, verbatim in
/// `AM/docs/ADL1.4/master04-dadl` §Adding Type Information, whose principle
/// box restates it normatively: "Type Information can be included optionally
/// on any node immediately before the opening '<' of any block, in the form
/// of a UML-style type identifier in parentheses. Dot-separated namespace
/// identifiers and template parameters may be used.").
///
/// Two consequences of that wording are honoured here. The package segments
/// take either case — the chapter's own two examples are a lowercase package
/// path (`org.openehr.rm.ehr.content`) and an upper-case one
/// (`Core.Abstractions.Relationships`) — while the TYPE the qualification
/// names stays `ALPHA_UC_ID`, since a package prefix qualifies the same type
/// identifier the unqualified form spells. And because a template parameter
/// is itself a type identifier, a parameter may be qualified too
/// (`List<org.openehr.rm.ehr.content.ENTRY>`); the recursion gives that for
/// free.
///
/// The qualified name is preserved FLAT, exactly as authored, so no caller
/// loses the package information the author supplied.
fn rm_type_id<'a>() -> impl Parser<'a, &'a [Token], String, Err<'a>> + Clone {
    recursive(|t| {
        let package_segment = select! {
            Token::AlphaUcId(s) => s,
            Token::AlphaLcId(s) => s,
        };
        package_segment
            .then_ignore(just(Token::Dot))
            .repeated()
            .collect::<Vec<String>>()
            .then(select! { Token::AlphaUcId(s) => s })
            .map(|(namespace, name)| {
                if namespace.is_empty() {
                    name
                } else {
                    format!("{}.{name}", namespace.join("."))
                }
            })
            .then(
                t.separated_by(just(Token::Comma))
                    .at_least(1)
                    .collect::<Vec<String>>()
                    .delimited_by(just(Token::Lt), just(Token::Gt))
                    .or_not(),
            )
            .map(|(name, generics)| match generics {
                Some(gs) => format!("{name}<{}>", gs.join(",")),
                None => name,
            })
    })
}

/// `primitive_object : primitive_value | primitive_list_value |
/// primitive_interval_value`.
fn primitive_object<'a>() -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone {
    let leaf = leaf_value();

    // primitive_value | primitive_list_value: a single leaf (scalar), or a
    // comma-separated list of leaves, optionally left open with a trailing
    // `, ...` continuation marker. Per `master07` §"Lists of Built-in Types",
    // `...` is the open-list continuation marker: a single-datum list `v, ...`
    // *requires* it (to distinguish the list from a bare scalar `v`), and a
    // multi-datum list `v1, v2, ..., vn, ...` may equally be left open with it
    // (as the published CIMI reference-model schemas do). The `odin_values.g4`
    // `string_list_value : v ( (',' v)+ | ',' SYM_LIST_CONTINUE )` encoding
    // admits only the single-datum-plus-continue and the closed-multi forms;
    // the general `v (',' v)* (',' '...')?` accepted here is a strict superset
    // that additionally admits the open multi-datum list the spec prose
    // describes. A bare leaf with no following comma stays a scalar.
    let list = leaf
        .clone()
        .then(
            just(Token::Comma)
                .ignore_then(leaf.clone())
                .repeated()
                .collect::<Vec<OdinValue>>(),
        )
        .then(
            just(Token::Comma)
                .ignore_then(just(Token::ListContinue))
                .or_not(),
        )
        .map(|((first, mut more), open)| {
            if more.is_empty() && open.is_none() {
                first
            } else {
                let mut v = vec![first];
                v.append(&mut more);
                if open.is_some() {
                    v.push(OdinValue::ListContinue);
                }
                OdinValue::List(v)
            }
        });

    interval_value(leaf).or(list)
}

/// One interval `| … |` (`odin_values.g4` `*_interval_value`).
fn interval_value<'a>(
    leaf: impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone {
    // An interval endpoint: a leaf value, or one of the unbounded markers
    // `infinity` / `-infinity` / `*` (`AM/docs/ADL1.4/master04-dadl`
    // §Intervals of Ordered Primitive Types — its own example `|0..infinity|`
    // is glossed "0 - infinity (i.e. >= 0)"). An unbounded marker yields
    // `None`, the representation this crate already uses for the open side of
    // a one-sided interval; the sign carried by `-infinity` is not recorded
    // separately because the side of the interval the endpoint sits on already
    // determines the direction.
    let unbounded = choice((
        just(Token::Minus)
            .or_not()
            .ignore_then(just(Token::Infinity))
            .ignored(),
        just(Token::Star).ignored(),
    ))
    .to(None);
    let bound = choice((unbounded, leaf.clone().map(Some)));

    // `| '>'? a '..' '<'? b |`
    let range = just(Token::Gt)
        .or_not()
        .then(bound.clone())
        .then_ignore(just(Token::IvlSep))
        .then(just(Token::Lt).or_not())
        .then(bound.clone())
        .map(|(((gt, lo), lt), hi)| OdinInterval::Range {
            lower_included: gt.is_none() && lo.is_some(),
            upper_included: lt.is_none() && hi.is_some(),
            lower: lo.map(Box::new),
            upper: hi.map(Box::new),
        });

    // `| a '+/-' b |`
    let plus_minus = leaf
        .clone()
        .then_ignore(just(Token::PlusOrMinus))
        .then(leaf.clone())
        .map(|(centre, delta)| OdinInterval::PlusMinus {
            centre: Box::new(centre),
            delta: Box::new(delta),
        });

    // `| relop? a |`  (relop absent ⇒ a point interval `[a, a]`)
    let relop = choice((
        just(Token::Ge).to(RelBound::Lower(true)),
        just(Token::Gt).to(RelBound::Lower(false)),
        just(Token::Le).to(RelBound::Upper(true)),
        just(Token::Lt).to(RelBound::Upper(false)),
    ));
    let single = relop.or_not().then(bound).map(|(op, v)| match (op, v) {
        (None, Some(v)) => OdinInterval::Range {
            lower: Some(Box::new(v.clone())),
            lower_included: true,
            upper: Some(Box::new(v)),
            upper_included: true,
        },
        (Some(RelBound::Lower(incl)), Some(v)) => OdinInterval::Range {
            lower: Some(Box::new(v)),
            lower_included: incl,
            upper: None,
            upper_included: false,
        },
        (Some(RelBound::Upper(incl)), Some(v)) => OdinInterval::Range {
            lower: None,
            lower_included: false,
            upper: Some(Box::new(v)),
            upper_included: incl,
        },
        // A one-sided form whose single endpoint is itself an unbounded marker
        // (`|>=-infinity|`, `|<*|`) constrains nothing at all.
        (_, None) => OdinInterval::Range {
            lower: None,
            lower_included: false,
            upper: None,
            upper_included: false,
        },
    });

    just(Token::IvlDelim)
        .ignore_then(choice((range, plus_minus, single)))
        .then_ignore(just(Token::IvlDelim))
        .map(OdinValue::Interval)
}

/// A single-bound relational operator's target side + inclusivity.
#[derive(Clone, Copy)]
enum RelBound {
    Lower(bool),
    Upper(bool),
}

/// A single primitive leaf value (`primitive_value` in `odin_values.g4`).
fn leaf_value<'a>() -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone {
    let int_sign = choice((just(Token::Plus).to(1i64), just(Token::Minus).to(-1i64))).or_not();
    let integer = int_sign
        .then(select! { Token::Integer(s) => s })
        .try_map(|(sign, s), span| {
            let mag = integer_lexeme(&s).ok_or(Failure::Syntax(span))?;
            sign.unwrap_or(1)
                .checked_mul(mag)
                .map(OdinValue::Integer)
                .ok_or(Failure::Syntax(span))
        });

    let real_sign = choice((just(Token::Plus).to(1f64), just(Token::Minus).to(-1f64))).or_not();
    let real = real_sign
        .then(select! { Token::Real(s) => s })
        .try_map(|(sign, s), span| {
            // The lexeme is already pinned by the token and the span; a
            // `ParseFloatError` adds nothing a `Syntax(span)` does not carry.
            let Ok(mag) = s.parse::<f64>() else {
                return Err(Failure::Syntax(span));
            };
            Ok(OdinValue::Real(sign.unwrap_or(1.0) * mag))
        });

    let string = select! { Token::String(s) => OdinValue::String(decode_string(&s)) };
    let boolean = select! {
        Token::True => OdinValue::Boolean(true),
        Token::False => OdinValue::Boolean(false),
    };
    let character =
        select! { Token::Character(s) => s }.map(|s| OdinValue::Character(decode_char(&s)));
    let term_code = select! {
        Token::TermCodeRef(s) => OdinValue::TermCode(s),
        Token::LocalTermCodeRef(s) => OdinValue::TermCode(s),
    };
    let date = select! { Token::Date(s) => OdinValue::Date(s) };
    let time = select! { Token::Time(s) => OdinValue::Time(s) };
    let date_time = select! { Token::DateTime(s) => OdinValue::DateTime(s) };
    let duration = select! { Token::Duration(s) => OdinValue::Duration(s) };

    choice((
        real, integer, string, boolean, character, term_code, date_time, date, time, duration,
    ))
}

/// The value of an `INTEGER` lexeme, evaluating the optional exponent suffix.
///
/// `AM/docs/ADL1.4/master04-dadl` §Integer Data lists `29e6` beside `25` and
/// `300000` as integer data, and both the chapter's lex rule
/// (`[0-9]+[eE][+-]?[0-9]+`) and the vendored `base_lexer.g4`
/// (`INTEGER : DIGIT+ E_SUFFIX?`) admit the suffix — so the lexeme is
/// *evaluated*, not parsed as a bare decimal literal.
///
/// NOTE: neither chapter says what a negative exponent means for an integer
/// (`29e-2` is not an integer). It is accepted only when the scaling is exact
/// (`2900e-2` = 29); an inexact one has no integer value and is rejected as a
/// malformed leaf rather than silently truncated. No openEHR spec governs that
/// case — our own design/extension.
#[expect(
    clippy::integer_division,
    reason = "the `mantissa % scale == 0` guard proves the division is exact — a negative exponent that does not divide evenly returns None instead"
)]
fn integer_lexeme(s: &str) -> Option<i64> {
    let Some(e) = s.find(['e', 'E']) else {
        return s.parse::<i64>().ok();
    };
    let mantissa = s.get(..e)?.parse::<i64>().ok()?;
    let exponent = s.get(e + 1..)?.parse::<i32>().ok()?;
    let scale = 10i64.checked_pow(exponent.unsigned_abs())?;
    if exponent >= 0 {
        mantissa.checked_mul(scale)
    } else if mantissa % scale == 0 {
        Some(mantissa / scale)
    } else {
        None
    }
}

/// Strip the surrounding double quotes and decode `master03` escapes.
///
/// The lexer (`validate_string`) has already run
/// [`crate::escape::validate`] over the same text, so the decode cannot fail
/// here.
#[expect(
    clippy::expect_used,
    reason = "`Token::String` only exists when the lexer's validate_string ran crate::escape::validate over the same body and it succeeded, so this decode of that body cannot fail"
)]
fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    crate::escape::decode(inner).expect("a lexer-validated string literal should decode")
}

/// Decode a single-quoted `CHARACTER` literal to its `char`.
///
/// The lexer (`validate_char`) admits only the six quoted forms in a character
/// literal, so the decode cannot fail here.
#[expect(
    clippy::expect_used,
    reason = "`Token::Character` only exists when the lexer's validate_char admitted the body, which restricts an escape to the six quoted forms none of which can fail to decode"
)]
fn decode_char(raw: &str) -> char {
    let inner = raw
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw);
    let decoded =
        crate::escape::decode(inner).expect("a lexer-validated character literal should decode");
    decoded.chars().next().unwrap_or('\u{fffd}')
}
