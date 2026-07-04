//! AQL parser — a `chumsky` parser transcribed from `AqlParser.g4`, turning the
//! [`crate::lexer`] token stream into an [`crate::ast::SelectQuery`]. No ANTLR
//! runtime (see `.claude/rules/aql-engine.md`).
//!
//! Precedence: within both `containsExpr` and `whereExpr`, `AND` binds tighter
//! than `OR` (the grammar's left recursion is realized here as `or` over `and`
//! over atoms), with parenthesized grouping.
//!
//! Coverage note: this is the core of the grammar (select/from/where/order/
//! limit, `CONTAINS` trees, identified paths, standard/node/archetype
//! predicates, comparisons, primitives, params, aggregates, functions). A few
//! rarely-used corners are marked `// TODO(port):` and parse into the nearest
//! faithful AST node.

use crate::ast::{
    AggregateCall, ArchetypePredicate, ClassExprOperand, ColumnExpr, CompareOperand,
    ContainsConstraint, ContainsExpr, FunctionCall, IdentifiedExpr, IdentifiedPath, LikeOperand,
    Limit, MatchesOperand, NodeNameConstraint, NodePredicate, ObjectPath, OrderByExpr, PathPart,
    PathPredicate, PathPredicateOperand, Primitive, SelectClause, SelectExpr, SelectQuery,
    SortOrder, StandardPredicate, StatFunc, Terminal, TerminologyFunction, Top, TopDirection,
    ValueListItem, VersionPredicate, WhereExpr,
};
use crate::lexer::Token;
use chumsky::prelude::*;

type Err<'a> = chumsky::extra::Err<Simple<'a, Token>>;

/// Parse a token slice into a [`SelectQuery`].
///
/// # Errors
/// Returns a list of [`Simple`] parse errors (with token spans) on failure.
pub fn parse(tokens: &[Token]) -> Result<SelectQuery, Vec<Simple<'_, Token>>> {
    query().parse(tokens).into_result()
}

/// Lex then parse `src` in one step.
///
/// # Errors
/// Returns a human-readable message on a lex or parse failure.
pub fn parse_str(src: &str) -> Result<SelectQuery, String> {
    let tokens = crate::lexer::lex(src).map_err(|e| e.to_string())?;
    parse(&tokens).map_err(|errs| {
        errs.iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })
}

// ── leaf parsers ─────────────────────────────────────────────────────────────

fn ident<'a>() -> impl Parser<'a, &'a [Token], String, Err<'a>> + Clone {
    select! { Token::Identifier(s) => s }
}

/// Strip the surrounding quotes from a lexed string literal (unescaping is a
/// later concern — see the lexer PORT NOTE).
fn unquote(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn primitive<'a>() -> impl Parser<'a, &'a [Token], Primitive, Err<'a>> + Clone {
    let unsigned = select! {
        Token::Integer(s) => Primitive::Integer(s.parse().unwrap_or_default()),
        Token::Real(s) => Primitive::Real(s.parse().unwrap_or_default()),
        Token::SciInteger(s) => Primitive::Real(s.parse().unwrap_or_default()),
        Token::SciReal(s) => Primitive::Real(s.parse().unwrap_or_default()),
    };
    // numericPrimitive : … | SYM_MINUS numericPrimitive
    let negative = just(Token::Minus).ignore_then(unsigned).map(|p| match p {
        Primitive::Integer(n) => Primitive::Integer(-n),
        Primitive::Real(r) => Primitive::Real(-r),
        other => other,
    });
    let other = select! {
        Token::String(s) => Primitive::String(unquote(&s)),
        Token::True => Primitive::Boolean(true),
        Token::False => Primitive::Boolean(false),
        Token::Null => Primitive::Null,
    };
    negative.or(unsigned).or(other)
}

fn parameter<'a>() -> impl Parser<'a, &'a [Token], String, Err<'a>> + Clone {
    select! { Token::Parameter(s) => s }
}

// ── paths & predicates (mutually recursive: predicate ← objectPath ← predicate) ──

/// Returns `(identified_path, object_path, path_predicate)` parsers. Built
/// together because a `pathPredicate` contains an `objectPath` which contains
/// `pathPart`s that may themselves carry a `pathPredicate`.
#[allow(clippy::type_complexity)]
fn path_parsers<'a>() -> (
    impl Parser<'a, &'a [Token], IdentifiedPath, Err<'a>> + Clone,
    impl Parser<'a, &'a [Token], PathPredicate, Err<'a>> + Clone,
) {
    let mut identified = Recursive::declare();
    let mut object = Recursive::declare();
    let mut predicate = Recursive::declare();

    // pathPredicateOperand : primitive | objectPath | PARAMETER | ID_CODE | AT_CODE
    let code = select! { Token::IdCode(s) => s, Token::AtCode(s) => s };
    let predicate_operand = primitive()
        .map(PathPredicateOperand::Primitive)
        .or(parameter().map(PathPredicateOperand::Parameter))
        .or(code.map(PathPredicateOperand::Code))
        .or(object.clone().map(PathPredicateOperand::Path));

    // standardPredicate : objectPath COMPARISON_OPERATOR pathPredicateOperand
    let comparison = select! { Token::Comparison(op) => op };
    let standard = object
        .clone()
        .then(comparison)
        .then(predicate_operand)
        .map(|((path, op), operand)| StandardPredicate { path, op, operand });

    // archetypePredicate : ARCHETYPE_HRID | PARAMETER
    let archetype = select! { Token::ArchetypeHrid(s) => ArchetypePredicate::Hrid(s) }
        .or(parameter().map(ArchetypePredicate::Parameter));

    // nodePredicate : node/archetype code (+optional name) | parameter |
    //   objectPath MATCHES CONTAINED_REGEX | standardPredicate |
    //   nodePredicate (AND|OR) nodePredicate. AND binds tighter than OR.
    let name_constraint = select! {
        Token::String(s) => NodeNameConstraint::String(unquote(&s)),
        Token::TermCode(s) => NodeNameConstraint::TermCode(s),
        Token::IdCode(s) => NodeNameConstraint::Code(s),
        Token::AtCode(s) => NodeNameConstraint::Code(s),
    }
    .or(parameter().map(NodeNameConstraint::Parameter));

    let node_code = code
        .then(
            just(Token::Comma)
                .ignore_then(name_constraint.clone())
                .or_not(),
        )
        .map(|(code, name)| NodePredicate::Code { code, name });
    let node_archetype = select! { Token::ArchetypeHrid(s) => s }
        .then(just(Token::Comma).ignore_then(name_constraint).or_not())
        .map(|(hrid, name)| NodePredicate::Archetype { hrid, name });
    let node_matches_regex = object
        .clone()
        .then_ignore(just(Token::Matches))
        .then(select! { Token::ContainedRegex(s) => s })
        .map(|(path, regex)| NodePredicate::MatchesRegex { path, regex });
    let node_atom = node_code
        .or(node_archetype)
        .or(parameter().map(NodePredicate::Parameter))
        .or(node_matches_regex)
        .or(standard
            .clone()
            .map(|s| NodePredicate::Standard(Box::new(s))));
    let node_and = node_atom.clone().foldl(
        just(Token::And).ignore_then(node_atom.clone()).repeated(),
        |l, r| NodePredicate::And(Box::new(l), Box::new(r)),
    );
    let node = node_and
        .clone()
        .foldl(just(Token::Or).ignore_then(node_and).repeated(), |l, r| {
            NodePredicate::Or(Box::new(l), Box::new(r))
        });

    // pathPredicate : '[' (standardPredicate | archetypePredicate | nodePredicate) ']'
    // Ordered so the most specific (archetype/node codes) win before the
    // generic standard comparison.
    predicate.define(
        archetype
            .clone()
            .map(PathPredicate::Archetype)
            .or(node.map(|n| PathPredicate::Node(Box::new(n))))
            .or(standard.map(|s| PathPredicate::Standard(Box::new(s))))
            .delimited_by(just(Token::LeftBracket), just(Token::RightBracket)),
    );

    // pathPart : IDENTIFIER pathPredicate?
    let path_part = ident()
        .then(predicate.clone().or_not())
        .map(|(name, predicate)| PathPart { name, predicate });

    // objectPath : pathPart ('/' pathPart)*
    object.define(
        path_part
            .separated_by(just(Token::Slash))
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|parts| ObjectPath { parts }),
    );

    // identifiedPath : IDENTIFIER pathPredicate? ('/' objectPath)?
    identified.define(
        ident()
            .then(predicate.clone().or_not())
            .then(just(Token::Slash).ignore_then(object.clone()).or_not())
            .map(|((root, predicate), path)| IdentifiedPath {
                root,
                predicate,
                path,
            }),
    );

    (identified, predicate)
}

// ── functions & terminals ───────────────────────────────────────────────────

fn terminology_fn<'a>() -> impl Parser<'a, &'a [Token], TerminologyFunction, Err<'a>> + Clone {
    let string = select! { Token::String(s) => unquote(&s) };
    just(Token::Terminology)
        .ignore_then(
            string
                .then_ignore(just(Token::Comma))
                .then(string)
                .then_ignore(just(Token::Comma))
                .then(string)
                .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
        )
        .map(|((operation, arg2), arg3)| TerminologyFunction {
            operation,
            arg2,
            arg3,
        })
}

/// `functionCall` and `aggregateFunctionCall`, given the `identified_path` and
/// `terminal` parsers.
#[allow(clippy::type_complexity)]
fn function_parsers<'a>(
    identified: impl Parser<'a, &'a [Token], IdentifiedPath, Err<'a>> + Clone + 'a,
    terminal: impl Parser<'a, &'a [Token], Terminal, Err<'a>> + Clone + 'a,
) -> (
    impl Parser<'a, &'a [Token], FunctionCall, Err<'a>> + Clone,
    impl Parser<'a, &'a [Token], AggregateCall, Err<'a>> + Clone,
) {
    // functionCall : terminologyFunction | name '(' (terminal (',' terminal)*)? ')'
    let named = ident()
        .then(
            terminal
                .separated_by(just(Token::Comma))
                .collect::<Vec<_>>()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
        )
        .map(|(name, args)| FunctionCall::Named { name, args });
    let function = terminology_fn().map(FunctionCall::Terminology).or(named);

    // aggregateFunctionCall
    let count = just(Token::Count).ignore_then(
        just(Token::Distinct)
            .or_not()
            .then(identified.clone())
            .map(|(d, p)| AggregateCall::Count {
                distinct: d.is_some(),
                path: Some(p),
            })
            .or(just(Token::Asterisk).map(|_| AggregateCall::Count {
                distinct: false,
                path: None,
            }))
            .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
    );
    let stat_name = select! {
        Token::Min => StatFunc::Min,
        Token::Max => StatFunc::Max,
        Token::Sum => StatFunc::Sum,
        Token::Avg => StatFunc::Avg,
    };
    let stat = stat_name
        .then(
            identified
                .clone()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen)),
        )
        .map(|(func, path)| AggregateCall::Stat { func, path });
    (function, count.or(stat))
}

// ── top-level query ──────────────────────────────────────────────────────────

#[allow(clippy::too_many_lines)] // one combinator builder for the whole grammar
fn query<'a>() -> impl Parser<'a, &'a [Token], SelectQuery, Err<'a>> {
    let (identified, _predicate) = path_parsers();

    // terminal : primitive | PARAMETER | identifiedPath | functionCall
    // (functionCall needs terminal → declare terminal recursively.)
    let mut terminal = Recursive::declare();
    let (function, aggregate) = function_parsers(identified.clone(), terminal.clone());
    terminal.define(
        primitive()
            .map(Terminal::Primitive)
            .or(parameter().map(Terminal::Parameter))
            .or(function.clone().map(Terminal::Function))
            .or(identified.clone().map(Terminal::Path)),
    );

    // ── SELECT ──
    // columnExpr : identifiedPath | primitive | aggregateFunctionCall | functionCall
    let column = aggregate
        .clone()
        .map(ColumnExpr::Aggregate)
        .or(function.clone().map(ColumnExpr::Function))
        .or(primitive().map(ColumnExpr::Primitive))
        .or(identified.clone().map(ColumnExpr::Path));
    let select_expr = column
        .then(just(Token::As).ignore_then(ident()).or_not())
        .map(|(column, alias)| SelectExpr { column, alias });
    // top (deprecated)
    let top = just(Token::Top)
        .ignore_then(select! { Token::Integer(s) => s.parse::<i64>().unwrap_or_default() })
        .then(
            select! {
                Token::Forward => TopDirection::Forward,
                Token::Backward => TopDirection::Backward,
            }
            .or_not(),
        )
        .map(|(count, direction)| Top { count, direction });
    let select_clause = just(Token::Select)
        .ignore_then(just(Token::Distinct).or_not())
        .then(top.or_not())
        .then(
            select_expr
                .separated_by(just(Token::Comma))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|((distinct, top), columns)| SelectClause {
            distinct: distinct.is_some(),
            top,
            columns,
        });

    // ── FROM / containsExpr ──
    // classExprOperand : IDENTIFIER variable? pathPredicate? | VERSION variable? [versionPredicate]?
    let (_ip2, predicate2) = path_parsers();
    let version_predicate = select! {
        Token::LatestVersion => VersionPredicate::Latest,
        Token::AllVersions => VersionPredicate::All,
    };
    let class_operand = just(Token::Version)
        .ignore_then(ident().or_not())
        .then(
            version_predicate
                .delimited_by(just(Token::LeftBracket), just(Token::RightBracket))
                .or_not(),
        )
        .map(|(variable, predicate)| ClassExprOperand::Version {
            variable,
            predicate,
        })
        .or(ident()
            .then(ident().or_not())
            .then(predicate2.or_not())
            .map(|((rm_type, variable), predicate)| ClassExprOperand::Class {
                rm_type,
                variable,
                predicate,
            }));

    let contains = recursive(|contains| {
        let atom = class_operand
            .then(
                just(Token::Not)
                    .or_not()
                    .then_ignore(just(Token::Contains))
                    .then(contains.clone())
                    .map(|(neg, expr)| {
                        Box::new(ContainsConstraint {
                            negated: neg.is_some(),
                            expr,
                        })
                    })
                    .or_not(),
            )
            .map(|(operand, contains)| ContainsExpr::Contained { operand, contains })
            .or(contains
                .clone()
                .delimited_by(just(Token::LeftParen), just(Token::RightParen)));
        let and = atom.clone().foldl(
            just(Token::And).ignore_then(atom.clone()).repeated(),
            |l, r| ContainsExpr::And(Box::new(l), Box::new(r)),
        );
        and.clone()
            .foldl(just(Token::Or).ignore_then(and).repeated(), |l, r| {
                ContainsExpr::Or(Box::new(l), Box::new(r))
            })
    });

    // ── WHERE / whereExpr ──
    let like_operand = select! { Token::String(s) => LikeOperand::String(unquote(&s)) }
        .or(parameter().map(LikeOperand::Parameter));
    let value_item = primitive()
        .map(ValueListItem::Primitive)
        .or(parameter().map(ValueListItem::Parameter))
        .or(terminology_fn().map(ValueListItem::Terminology));
    let matches_operand = terminology_fn()
        .map(MatchesOperand::Terminology)
        .or(select! { Token::Uri(u) => MatchesOperand::Uri(u) }
            .delimited_by(just(Token::LeftCurly), just(Token::RightCurly)))
        .or(value_item
            .separated_by(just(Token::Comma))
            .at_least(1)
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LeftCurly), just(Token::RightCurly))
            .map(MatchesOperand::ValueList));

    let comparison = select! { Token::Comparison(op) => op };
    let identified_expr = just(Token::Exists)
        .ignore_then(identified.clone())
        .map(IdentifiedExpr::Exists)
        .or(function
            .clone()
            .then(comparison)
            .then(terminal.clone())
            .map(|((f, op), rhs)| IdentifiedExpr::Compare {
                lhs: CompareOperand::Function(f),
                op,
                rhs,
            }))
        .or(identified
            .clone()
            .then(just(Token::Like).ignore_then(like_operand))
            .map(|(path, operand)| IdentifiedExpr::Like { path, operand }))
        .or(identified
            .clone()
            .then(just(Token::Matches).ignore_then(matches_operand))
            .map(|(path, operand)| IdentifiedExpr::Matches { path, operand }))
        .or(identified
            .clone()
            .then(comparison)
            .then(terminal.clone())
            .map(|((path, op), rhs)| IdentifiedExpr::Compare {
                lhs: CompareOperand::Path(path),
                op,
                rhs,
            }));

    let where_expr = recursive(|where_expr| {
        let atom = identified_expr.map(WhereExpr::Identified).or(where_expr
            .clone()
            .delimited_by(just(Token::LeftParen), just(Token::RightParen)));
        // Precedence: NOT (unary, tightest) > AND > OR. `NOT a AND b` parses as
        // `(NOT a) AND b`; group with parens for `NOT (a AND b)`.
        let unary = just(Token::Not)
            .repeated()
            .foldr(atom, |_not, e| WhereExpr::Not(Box::new(e)));
        let and = unary
            .clone()
            .foldl(just(Token::And).ignore_then(unary).repeated(), |l, r| {
                WhereExpr::And(Box::new(l), Box::new(r))
            });
        and.clone()
            .foldl(just(Token::Or).ignore_then(and).repeated(), |l, r| {
                WhereExpr::Or(Box::new(l), Box::new(r))
            })
    });

    // ── ORDER BY / LIMIT ──
    let order_by_expr = identified
        .clone()
        .then(
            select! {
                Token::Descending => SortOrder::Descending,
                Token::Desc => SortOrder::Descending,
                Token::Ascending => SortOrder::Ascending,
                Token::Asc => SortOrder::Ascending,
            }
            .or_not(),
        )
        .map(|(path, order)| OrderByExpr { path, order });
    let int = select! { Token::Integer(s) => s.parse::<i64>().unwrap_or_default() };
    let limit = just(Token::Limit)
        .ignore_then(int)
        .then(just(Token::Offset).ignore_then(int).or_not())
        .map(|(limit, offset)| Limit { limit, offset });

    // selectQuery : selectClause fromClause whereClause? orderByClause? limitClause? '--'? EOF
    select_clause
        .then(just(Token::From).ignore_then(contains))
        .then(just(Token::Where).ignore_then(where_expr).or_not())
        .then(
            just(Token::Order)
                .ignore_then(just(Token::By))
                .ignore_then(
                    order_by_expr
                        .separated_by(just(Token::Comma))
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .or_not(),
        )
        .then(limit.or_not())
        .then_ignore(just(Token::DoubleDash).or_not())
        .map(
            |((((select, from), where_), order_by), limit)| SelectQuery {
                select,
                from,
                where_,
                order_by: order_by.unwrap_or_default(),
                limit,
            },
        )
        .then_ignore(end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_select_from() {
        let q = parse_str("SELECT c FROM COMPOSITION c").expect("parse");
        assert_eq!(q.select.columns.len(), 1);
        assert!(matches!(
            &q.from,
            ContainsExpr::Contained {
                operand: ClassExprOperand::Class { rm_type, variable, .. },
                ..
            } if rm_type == "COMPOSITION" && variable.as_deref() == Some("c")
        ));
    }

    #[test]
    fn select_path_where_compare() {
        let q = parse_str("SELECT e/ehr_id/value FROM EHR e WHERE e/ehr_id/value = $id")
            .expect("parse");
        assert!(matches!(q.select.columns[0].column, ColumnExpr::Path(_)));
        assert!(matches!(
            q.where_,
            Some(WhereExpr::Identified(IdentifiedExpr::Compare { .. }))
        ));
    }

    #[test]
    fn contains_with_archetype_predicate() {
        let q = parse_str(
            "SELECT o FROM EHR e CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v1]",
        )
        .expect("parse");
        match &q.from {
            ContainsExpr::Contained {
                contains: Some(c), ..
            } => {
                assert!(!c.negated);
                assert!(matches!(c.expr, ContainsExpr::Contained { .. }));
            }
            other => panic!("expected contains, got {other:?}"),
        }
    }

    #[test]
    fn distinct_alias_orderby_limit() {
        let q = parse_str(
            "SELECT DISTINCT c/name/value AS n FROM COMPOSITION c ORDER BY c/name/value DESC LIMIT 10 OFFSET 5",
        )
        .expect("parse");
        assert!(q.select.distinct);
        assert_eq!(q.select.columns[0].alias.as_deref(), Some("n"));
        assert_eq!(q.order_by.len(), 1);
        assert_eq!(q.order_by[0].order, Some(SortOrder::Descending));
        assert_eq!(
            q.limit,
            Some(Limit {
                limit: 10,
                offset: Some(5)
            })
        );
    }

    #[test]
    fn aggregate_count() {
        let q = parse_str("SELECT COUNT(*) FROM COMPOSITION c").expect("parse");
        assert!(matches!(
            q.select.columns[0].column,
            ColumnExpr::Aggregate(AggregateCall::Count { path: None, .. })
        ));
    }

    #[test]
    fn where_boolean_precedence() {
        // a AND b OR c  ⇒  Or(And(a,b), c)
        let q = parse_str("SELECT c FROM COMPOSITION c WHERE c/x = 1 AND c/y = 2 OR c/z = 3")
            .expect("parse");
        assert!(matches!(q.where_, Some(WhereExpr::Or(_, _))));
    }

    #[test]
    fn not_binds_tighter_than_and() {
        // NOT a AND b  ⇒  And(Not(a), b)  — not Not(And(a,b))
        let q = parse_str("SELECT c FROM COMPOSITION c WHERE NOT EXISTS c/x AND EXISTS c/y")
            .expect("parse");
        match q.where_ {
            Some(WhereExpr::And(l, _)) => assert!(matches!(*l, WhereExpr::Not(_))),
            other => panic!("expected And(Not(..), ..), got {other:?}"),
        }
    }

    #[test]
    fn node_predicate_boolean_tree() {
        let q = parse_str("SELECT o FROM OBSERVATION o[at0001 AND value/magnitude > 5]")
            .expect("parse");
        match &q.from {
            ContainsExpr::Contained {
                operand:
                    ClassExprOperand::Class {
                        predicate: Some(PathPredicate::Node(n)),
                        ..
                    },
                ..
            } => assert!(matches!(**n, NodePredicate::And(_, _))),
            other => panic!("expected node AND predicate, got {other:?}"),
        }
    }

    #[test]
    fn contained_regex_predicate() {
        let q = parse_str("SELECT o FROM OBSERVATION o[name/value MATCHES {/blood.*/}]")
            .expect("parse");
        assert!(matches!(
            &q.from,
            ContainsExpr::Contained {
                operand: ClassExprOperand::Class {
                    predicate: Some(PathPredicate::Node(_)),
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn trailing_tokens_are_rejected() {
        // EOF is enforced: junk after a complete query must fail.
        assert!(parse_str("SELECT c FROM COMPOSITION c EXTRA").is_err());
    }

    #[test]
    fn nested_contains_and_matches_valueset() {
        let q = parse_str(
            "SELECT a/value FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o \
             WHERE o/value/defining_code MATCHES {'at0001', 'at0002'}",
        )
        .expect("parse");
        // EHR CONTAINS (COMPOSITION CONTAINS OBSERVATION)
        assert!(matches!(
            &q.from,
            ContainsExpr::Contained {
                contains: Some(_),
                ..
            }
        ));
        assert!(matches!(
            q.where_,
            Some(WhereExpr::Identified(IdentifiedExpr::Matches { .. }))
        ));
    }
}
