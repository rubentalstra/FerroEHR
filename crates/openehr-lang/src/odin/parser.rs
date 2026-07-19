//! ODIN parser — a `chumsky` parser over the [`super::lexer`] token stream,
//! transcribed from `odin.g4` / `odin_values.g4`. Produces an
//! [`OdinValue`] tree.

use chumsky::prelude::*;
use indexmap::IndexMap;

use super::lexer::{Spanned, Token};
use super::{OdinInterval, OdinKey, OdinValue};

type Err<'a> = chumsky::extra::Err<Simple<'a, Token>>;

/// Parse a spanned ODIN token stream into an [`OdinValue`].
///
/// # Errors
/// Returns the byte offset (into the original source) of the first parse error.
pub(super) fn parse_tokens(spanned: &[Spanned]) -> Result<OdinValue, usize> {
    let tokens: Vec<Token> = spanned.iter().map(|s| s.token.clone()).collect();
    odin_text().parse(&tokens).into_result().map_err(|errs| {
        let idx = errs.first().map_or(spanned.len(), |e| e.span().start);
        spanned
            .get(idx)
            .map(|s| s.span.start)
            .or_else(|| spanned.last().map(|s| s.span.end))
            .unwrap_or(0)
    })
}

/// `odin_text : attr_vals | object_value_block`.
fn odin_text<'a>() -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> {
    let block = object_block();
    let attrs = attr_vals(block.clone());
    choice((attrs, block)).then_ignore(end())
}

/// `attr_vals : ( attr_val ';'? )+` → [`OdinValue::Object`].
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
        .repeated()
        .at_least(1)
        .collect::<Vec<(String, OdinValue)>>()
        .map(|pairs| OdinValue::Object(pairs.into_iter().collect::<IndexMap<_, _>>()))
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

        let inner = choice((
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

/// `rm_type_id : ALPHA_UC_ID ( '<' rm_type_id ( ',' rm_type_id )* '>' )?`,
/// reconstructed as a flat string (e.g. `Interval<Quantity>`).
fn rm_type_id<'a>() -> impl Parser<'a, &'a [Token], String, Err<'a>> + Clone {
    recursive(|t| {
        select! { Token::AlphaUcId(s) => s }
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

    // primitive_value | primitive_list_value: a single leaf, or 2+ comma-
    // separated leaves, or a single leaf followed by `, ...` (open list).
    let list = leaf
        .clone()
        .then(
            choice((
                just(Token::Comma)
                    .ignore_then(leaf.clone())
                    .repeated()
                    .at_least(1)
                    .collect::<Vec<OdinValue>>(),
                just(Token::Comma)
                    .ignore_then(just(Token::ListContinue))
                    .to(vec![OdinValue::ListContinue]),
            ))
            .or_not(),
        )
        .map(|(first, rest)| match rest {
            None => first,
            Some(mut more) => {
                let mut v = vec![first];
                v.append(&mut more);
                OdinValue::List(v)
            }
        });

    interval_value(leaf).or(list)
}

/// One interval `| … |` (`odin_values.g4` `*_interval_value`).
fn interval_value<'a>(
    leaf: impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone + 'a,
) -> impl Parser<'a, &'a [Token], OdinValue, Err<'a>> + Clone {
    // `| '>'? a '..' '<'? b |`
    let range = just(Token::Gt)
        .or_not()
        .then(leaf.clone())
        .then_ignore(just(Token::IvlSep))
        .then(just(Token::Lt).or_not())
        .then(leaf.clone())
        .map(|(((gt, lo), lt), hi)| OdinInterval::Range {
            lower: Some(Box::new(lo)),
            lower_included: gt.is_none(),
            upper: Some(Box::new(hi)),
            upper_included: lt.is_none(),
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
    let single = relop.or_not().then(leaf).map(|(op, v)| match op {
        None => OdinInterval::Range {
            lower: Some(Box::new(v.clone())),
            lower_included: true,
            upper: Some(Box::new(v)),
            upper_included: true,
        },
        Some(RelBound::Lower(incl)) => OdinInterval::Range {
            lower: Some(Box::new(v)),
            lower_included: incl,
            upper: None,
            upper_included: false,
        },
        Some(RelBound::Upper(incl)) => OdinInterval::Range {
            lower: None,
            lower_included: false,
            upper: Some(Box::new(v)),
            upper_included: incl,
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
            let mag = s.parse::<i64>().map_err(|_| Simple::new(None, span))?;
            Ok(OdinValue::Integer(sign.unwrap_or(1) * mag))
        });

    let real_sign = choice((just(Token::Plus).to(1f64), just(Token::Minus).to(-1f64))).or_not();
    let real = real_sign
        .then(select! { Token::Real(s) => s })
        .try_map(|(sign, s), span| {
            let mag = s.parse::<f64>().map_err(|_| Simple::new(None, span))?;
            Ok(OdinValue::Real(sign.unwrap_or(1.0) * mag))
        });

    let string = select! { Token::String(s) => OdinValue::String(decode_string(&s)) };
    let boolean = select! {
        Token::True => OdinValue::Boolean(true),
        Token::False => OdinValue::Boolean(false),
    };
    let character =
        select! { Token::Character(s) => s }.map(|s| OdinValue::Character(decode_char(&s)));
    let term_code = select! { Token::TermCodeRef(s) => OdinValue::TermCode(s) };
    let date = select! { Token::Date(s) => OdinValue::Date(s) };
    let time = select! { Token::Time(s) => OdinValue::Time(s) };
    let date_time = select! { Token::DateTime(s) => OdinValue::DateTime(s) };
    let duration = select! { Token::Duration(s) => OdinValue::Duration(s) };

    choice((
        real, integer, string, boolean, character, term_code, date_time, date, time, duration,
    ))
}

/// Strip the surrounding double quotes and decode `master03` escapes.
fn decode_string(raw: &str) -> String {
    let inner = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);
    decode_escapes(inner)
}

/// Decode a single-quoted `CHARACTER` literal to its `char`.
fn decode_char(raw: &str) -> char {
    let inner = raw
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(raw);
    let decoded = decode_escapes(inner);
    decoded.chars().next().unwrap_or('\u{fffd}')
}

/// Decode the `master03` escape set (`\r \n \t \\ \" \'` + `\uHHHH`/
/// `\uHHHHHHHH`). Unrecognised sequences are impossible here — the lexer has
/// already rejected them — so a stray backslash is passed through verbatim.
fn decode_escapes(inner: &str) -> String {
    if !inner.contains('\\') {
        return inner.to_owned();
    }
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('u') => {
                let hex: String = chars
                    .clone()
                    .take(8)
                    .take_while(char::is_ascii_hexdigit)
                    .collect();
                let take = if hex.len() >= 8 { 8 } else { 4 };
                let code: String = hex.chars().take(take).collect();
                if let Ok(cp) = u32::from_str_radix(&code, 16)
                    && let Some(ch) = char::from_u32(cp)
                {
                    for _ in 0..code.len() {
                        chars.next();
                    }
                    out.push(ch);
                } else {
                    out.push('\\');
                    out.push('u');
                }
            }
            // `\\` (escaped backslash) → one `\`; a lone trailing `\` (None) →
            // one `\`; any other follower is passed through verbatim after the
            // backslash (the lexer has already rejected illegal escapes).
            other => {
                out.push('\\');
                if let Some(c) = other
                    && c != '\\'
                {
                    out.push(c);
                }
            }
        }
    }
    out
}
