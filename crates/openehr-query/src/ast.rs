//! AQL abstract syntax tree, transcribed from `AqlParser.g4` (vendored at
//! `vendor/grammar/`). Each grammar rule maps to a type here; the [`crate::parser`]
//! builds these from the [`crate::lexer`] token stream.
//!
//! Scope note: this is the *syntactic* AST. Semantic concerns (resolving paths
//! against Web Templates, typing quoted temporal literals) are later passes.

use crate::lexer::CompOp;

/// `selectQuery : selectClause fromClause whereClause? orderByClause? limitClause?`
#[derive(Debug, Clone, PartialEq)]
pub struct SelectQuery {
    /// `SELECT [DISTINCT] [TOP n] col, …`
    pub select: SelectClause,
    /// `FROM <containsExpr>`
    pub from: ContainsExpr,
    /// `WHERE <whereExpr>`
    pub where_: Option<WhereExpr>,
    /// `ORDER BY …`
    pub order_by: Vec<OrderByExpr>,
    /// `LIMIT n [OFFSET m]`
    pub limit: Option<Limit>,
}

/// `selectClause : SELECT DISTINCT? top? selectExpr (',' selectExpr)*`
#[derive(Debug, Clone, PartialEq)]
pub struct SelectClause {
    /// `DISTINCT` present.
    pub distinct: bool,
    /// The deprecated `TOP n [FORWARD|BACKWARD]`.
    pub top: Option<Top>,
    /// One or more selected columns.
    pub columns: Vec<SelectExpr>,
}

/// The deprecated `top : TOP INTEGER (FORWARD|BACKWARD)?`.
#[derive(Debug, Clone, PartialEq)]
pub struct Top {
    /// The row count.
    pub count: i64,
    /// Optional direction.
    pub direction: Option<TopDirection>,
}

/// `TOP` direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopDirection {
    /// `FORWARD`
    Forward,
    /// `BACKWARD`
    Backward,
}

/// `selectExpr : columnExpr (AS aliasName)?`
#[derive(Debug, Clone, PartialEq)]
pub struct SelectExpr {
    /// The selected expression.
    pub column: ColumnExpr,
    /// Optional `AS alias`.
    pub alias: Option<String>,
}

/// `columnExpr : identifiedPath | primitive | aggregateFunctionCall | functionCall`
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnExpr {
    /// A data path.
    Path(IdentifiedPath),
    /// A literal.
    Primitive(Primitive),
    /// `COUNT`/`MIN`/`MAX`/`SUM`/`AVG`.
    Aggregate(AggregateCall),
    /// A named function call.
    Function(FunctionCall),
}

/// `containsExpr` — a boolean tree whose leaves are class expressions each
/// optionally constrained by a nested `CONTAINS`.
#[derive(Debug, Clone, PartialEq)]
pub enum ContainsExpr {
    /// `classExprOperand (NOT? CONTAINS containsExpr)?`
    Contained {
        /// The class/version operand.
        operand: ClassExprOperand,
        /// Optional `[NOT] CONTAINS <sub>`.
        contains: Option<Box<ContainsConstraint>>,
    },
    /// `containsExpr AND containsExpr`
    And(Box<ContainsExpr>, Box<ContainsExpr>),
    /// `containsExpr OR containsExpr`
    Or(Box<ContainsExpr>, Box<ContainsExpr>),
}

/// The `[NOT] CONTAINS <containsExpr>` tail of a [`ContainsExpr::Contained`].
#[derive(Debug, Clone, PartialEq)]
pub struct ContainsConstraint {
    /// `NOT CONTAINS`.
    pub negated: bool,
    /// The contained sub-expression.
    pub expr: ContainsExpr,
}

/// `classExprOperand : #classExpression | #versionClassExpr`
#[derive(Debug, Clone, PartialEq)]
pub enum ClassExprOperand {
    /// `IDENTIFIER variable? pathPredicate?` (e.g. `COMPOSITION c[openEHR-…]`).
    Class {
        /// The RM class name.
        rm_type: String,
        /// Optional bound variable.
        variable: Option<String>,
        /// Optional `[predicate]`.
        predicate: Option<PathPredicate>,
    },
    /// `VERSION variable? [versionPredicate]?`
    Version {
        /// Optional bound variable.
        variable: Option<String>,
        /// Optional version predicate.
        predicate: Option<VersionPredicate>,
    },
}

/// `whereExpr` — a boolean tree of [`IdentifiedExpr`] leaves.
#[derive(Debug, Clone, PartialEq)]
pub enum WhereExpr {
    /// A leaf condition.
    Identified(IdentifiedExpr),
    /// `NOT whereExpr`
    Not(Box<WhereExpr>),
    /// `whereExpr AND whereExpr`
    And(Box<WhereExpr>, Box<WhereExpr>),
    /// `whereExpr OR whereExpr`
    Or(Box<WhereExpr>, Box<WhereExpr>),
}

/// `identifiedExpr` — a single WHERE condition.
#[derive(Debug, Clone, PartialEq)]
pub enum IdentifiedExpr {
    /// `EXISTS identifiedPath`
    Exists(IdentifiedPath),
    /// `identifiedPath|functionCall COMPARISON_OPERATOR terminal`
    Compare {
        /// Left operand.
        lhs: CompareOperand,
        /// The operator.
        op: CompOp,
        /// Right operand.
        rhs: Terminal,
    },
    /// `identifiedPath LIKE likeOperand`
    Like {
        /// The path.
        path: IdentifiedPath,
        /// String or parameter.
        operand: LikeOperand,
    },
    /// `identifiedPath MATCHES matchesOperand`
    Matches {
        /// The path.
        path: IdentifiedPath,
        /// The match set.
        operand: MatchesOperand,
    },
}

/// Left side of a comparison: a path or a function call.
#[derive(Debug, Clone, PartialEq)]
pub enum CompareOperand {
    /// `identifiedPath`
    Path(IdentifiedPath),
    /// `functionCall`
    Function(FunctionCall),
}

/// `terminal : primitive | PARAMETER | identifiedPath | functionCall`
#[derive(Debug, Clone, PartialEq)]
pub enum Terminal {
    /// A literal.
    Primitive(Primitive),
    /// `$param`.
    Parameter(String),
    /// A path.
    Path(IdentifiedPath),
    /// A function call.
    Function(FunctionCall),
}

/// `orderByExpr : identifiedPath (DESCENDING|DESC|ASCENDING|ASC)?`
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByExpr {
    /// The path to order by.
    pub path: IdentifiedPath,
    /// Sort direction (defaults to ascending when absent).
    pub order: Option<SortOrder>,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// `ASC` / `ASCENDING`
    Ascending,
    /// `DESC` / `DESCENDING`
    Descending,
}

/// `limitClause : LIMIT INTEGER (OFFSET INTEGER)?`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limit {
    /// Row limit.
    pub limit: i64,
    /// Optional offset.
    pub offset: Option<i64>,
}

/// `identifiedPath : IDENTIFIER pathPredicate? ('/' objectPath)?`
#[derive(Debug, Clone, PartialEq)]
pub struct IdentifiedPath {
    /// The root variable/identifier.
    pub root: String,
    /// Optional predicate on the root.
    pub predicate: Option<PathPredicate>,
    /// Optional trailing `/a/b/c` object path.
    pub path: Option<ObjectPath>,
}

/// `objectPath : pathPart ('/' pathPart)*`
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPath {
    /// The `/`-separated parts.
    pub parts: Vec<PathPart>,
}

/// `pathPart : IDENTIFIER pathPredicate?`
#[derive(Debug, Clone, PartialEq)]
pub struct PathPart {
    /// The attribute name.
    pub name: String,
    /// Optional `[predicate]`.
    pub predicate: Option<PathPredicate>,
}

/// `pathPredicate : '[' (standardPredicate | archetypePredicate | nodePredicate) ']'`
#[derive(Debug, Clone, PartialEq)]
pub enum PathPredicate {
    /// `objectPath COMPARISON_OPERATOR operand`
    Standard(Box<StandardPredicate>),
    /// `ARCHETYPE_HRID | PARAMETER`
    Archetype(ArchetypePredicate),
    /// A node predicate (id/at code, archetype id, boolean tree, …).
    Node(Box<NodePredicate>),
}

/// `standardPredicate : objectPath COMPARISON_OPERATOR pathPredicateOperand`
#[derive(Debug, Clone, PartialEq)]
pub struct StandardPredicate {
    /// The constrained path.
    pub path: ObjectPath,
    /// The operator.
    pub op: CompOp,
    /// The operand.
    pub operand: PathPredicateOperand,
}

/// `archetypePredicate : ARCHETYPE_HRID | PARAMETER`
#[derive(Debug, Clone, PartialEq)]
pub enum ArchetypePredicate {
    /// An archetype HRID.
    Hrid(String),
    /// `$param`.
    Parameter(String),
}

/// `nodePredicate` — the (partial) common forms: a node code with optional
/// name/term, an archetype id with optional name/term, a parameter, a standard
/// comparison, or a boolean combination.
#[derive(Debug, Clone, PartialEq)]
pub enum NodePredicate {
    /// `(ID_CODE|AT_CODE) (',' (STRING|PARAMETER|TERM_CODE|AT_CODE|ID_CODE))?`
    Code {
        /// The `id`/`at` code.
        code: String,
        /// Optional name/term operand.
        name: Option<NodeNameConstraint>,
    },
    /// `ARCHETYPE_HRID (',' …)?`
    Archetype {
        /// The archetype HRID.
        hrid: String,
        /// Optional name/term operand.
        name: Option<NodeNameConstraint>,
    },
    /// `$param`
    Parameter(String),
    /// `objectPath COMPARISON_OPERATOR operand`
    Standard(Box<StandardPredicate>),
    /// `nodePredicate AND nodePredicate`
    And(Box<NodePredicate>, Box<NodePredicate>),
    /// `nodePredicate OR nodePredicate`
    Or(Box<NodePredicate>, Box<NodePredicate>),
}

/// The optional name/term after a node code in a [`NodePredicate`].
#[derive(Debug, Clone, PartialEq)]
pub enum NodeNameConstraint {
    /// A quoted name.
    String(String),
    /// `$param`.
    Parameter(String),
    /// A term code.
    TermCode(String),
    /// An `at`/`id` code.
    Code(String),
}

/// `versionPredicate : LATEST_VERSION | ALL_VERSIONS | standardPredicate`
#[derive(Debug, Clone, PartialEq)]
pub enum VersionPredicate {
    /// `LATEST_VERSION`
    Latest,
    /// `ALL_VERSIONS`
    All,
    /// A standard comparison predicate.
    Standard(Box<StandardPredicate>),
}

/// `pathPredicateOperand : primitive | objectPath | PARAMETER | ID_CODE | AT_CODE`
#[derive(Debug, Clone, PartialEq)]
pub enum PathPredicateOperand {
    /// A literal.
    Primitive(Primitive),
    /// A path.
    Path(ObjectPath),
    /// `$param`.
    Parameter(String),
    /// An `id`/`at` code.
    Code(String),
}

/// `likeOperand : STRING | PARAMETER`
#[derive(Debug, Clone, PartialEq)]
pub enum LikeOperand {
    /// A quoted pattern.
    String(String),
    /// `$param`.
    Parameter(String),
}

/// `matchesOperand : '{' valueListItem (',' valueListItem)* '}' | terminologyFunction | '{' URI '}'`
#[derive(Debug, Clone, PartialEq)]
pub enum MatchesOperand {
    /// A `{ … }` value list.
    ValueList(Vec<ValueListItem>),
    /// `terminology(...)`.
    Terminology(TerminologyFunction),
    /// `{ uri }`.
    Uri(String),
}

/// `valueListItem : primitive | PARAMETER | terminologyFunction`
#[derive(Debug, Clone, PartialEq)]
pub enum ValueListItem {
    /// A literal.
    Primitive(Primitive),
    /// `$param`.
    Parameter(String),
    /// `terminology(...)`.
    Terminology(TerminologyFunction),
}

/// `functionCall` — a named function with terminal arguments (also covers
/// `terminologyFunction`, kept as [`FunctionCall::Terminology`]).
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionCall {
    /// `name ( terminal, … )` — `name` may be a grouped function-id or a plain
    /// identifier (classified later).
    Named {
        /// The function name.
        name: String,
        /// The arguments.
        args: Vec<Terminal>,
    },
    /// `terminology(str, str, str)`.
    Terminology(TerminologyFunction),
}

/// `terminologyFunction : TERMINOLOGY '(' STRING ',' STRING ',' STRING ')'`
#[derive(Debug, Clone, PartialEq)]
pub struct TerminologyFunction {
    /// First argument (operation).
    pub operation: String,
    /// Second argument.
    pub arg2: String,
    /// Third argument.
    pub arg3: String,
}

/// `aggregateFunctionCall`
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateCall {
    /// `COUNT ( DISTINCT? identifiedPath | '*' )`
    Count {
        /// `DISTINCT` present.
        distinct: bool,
        /// The path, or `None` for `COUNT(*)`.
        path: Option<IdentifiedPath>,
    },
    /// `MIN|MAX|SUM|AVG ( identifiedPath )`
    Stat {
        /// Which aggregate.
        func: StatFunc,
        /// The path.
        path: IdentifiedPath,
    },
}

/// The non-count aggregate functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatFunc {
    /// `MIN`
    Min,
    /// `MAX`
    Max,
    /// `SUM`
    Sum,
    /// `AVG`
    Avg,
}

/// `primitive` — an AQL literal.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// A quoted string (unescaping/temporal-typing deferred; raw slice sans
    /// surrounding quotes).
    String(String),
    /// An integer literal.
    Integer(i64),
    /// A real literal.
    Real(f64),
    /// A boolean literal.
    Boolean(bool),
    /// `NULL`.
    Null,
}
