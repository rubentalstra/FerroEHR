//! AQL planner errors (ADR-008, P16 packages 2+3).
//!
//! Two families, both surfaced before any SQL is built:
//!
//! * [`AqlFeatureError`] — the feature-envelope rejections. Every variant names
//!   the rejected construct and cites the governing QUERY 1.1 spec section, so a
//!   rejection is always explainable against the vendored spec
//!   (`docs/specs/openehr/QUERY/docs/AQL/`). The accept/reject envelope is
//!   documented in `docs/design/aql-engine.md` §Feature envelope and must remain
//!   a superset of `EHRbase`'s (ADR-008 §3).
//! * [`AnalysisError`] — path analysis / typing failures (unknown class or
//!   variable, unresolvable attribute, type mismatch, unbound parameter).
//! * [`SqlError`] — IR→SQL lowering failures (a construct the planner accepted
//!   but the SQL package cannot yet render), surfaced before execution.
//! * [`ExecError`] — execution / `RESULT_SET` assembly failures (a database
//!   error or a reassembly failure), surfaced during execution.

use thiserror::Error;

/// The single error type returned by [`crate::aql::plan`] and the SQL/execution
/// packages ([`crate::aql::exec`]).
#[derive(Debug, Error)]
pub enum AqlError {
    /// A construct outside the accepted feature envelope.
    #[error(transparent)]
    Feature(#[from] AqlFeatureError),
    /// A path-analysis / typing failure.
    #[error(transparent)]
    Analysis(#[from] AnalysisError),
    /// An IR→SQL lowering failure.
    #[error(transparent)]
    Sql(#[from] SqlError),
    /// An execution / `RESULT_SET` assembly failure.
    #[error(transparent)]
    Exec(#[from] ExecError),
}

/// A construct that is syntactically valid AQL but outside the accepted feature
/// envelope. Each variant names the construct and its QUERY 1.1 spec section.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AqlFeatureError {
    /// `TERMINOLOGY(...)` server-side expansion needs a terminology service.
    /// QUERY §Functions/Other functions/TERMINOLOGY.
    #[error(
        "TERMINOLOGY() server-side expansion is not supported \
         (QUERY §Functions/Other functions/TERMINOLOGY)"
    )]
    TerminologyFunction,

    /// `matches TERMINOLOGY(...)` on the right-hand side of `matches`.
    /// QUERY §Operators/Comparison operators/matches + §Other functions.
    #[error(
        "matches against a TERMINOLOGY() operand is not supported \
         (QUERY §Operators/matches, §Other functions/TERMINOLOGY)"
    )]
    MatchesTerminology,

    /// `matches { <uri> }` — a terminology URI operand. QUERY §matches/URI.
    #[error(
        "matches against a terminology URI operand is not supported (QUERY §Operators/matches)"
    )]
    MatchesUri,

    /// A `matches` node predicate carrying a `{/regex/}` — cADL, not AQL value
    /// matching. QUERY §Predicates/Node predicate.
    #[error(
        "regex node predicates ({{/../}}) are not supported (QUERY §Predicates/Node predicate)"
    )]
    RegexNodePredicate,

    /// An `OR` combination inside a node predicate. QUERY §Predicates/Node
    /// predicate (the common forms are a code with an optional `AND name`
    /// criterion; a disjunctive node predicate is out of the accepted subset).
    #[error("OR node predicates are not supported (QUERY §Predicates/Node predicate)")]
    OrNodePredicate,

    /// A FROM source that is not an EHR / VERSION / in-scope RM structure class
    /// (e.g. a demographic PARTY/ROLE/ACTOR source). QUERY §FROM.
    #[error("FROM source class `{0}` is not in scope (demographic/off-scope class; QUERY §FROM)")]
    UnsupportedSourceClass(String),

    /// A time/`now`-family function used in ORDER BY. Accepted in WHERE as a
    /// bind-time constant; rejected in ORDER BY. QUERY §Functions/Date and time.
    #[error(
        "date/time function `{0}` is not supported in ORDER BY \
         (QUERY §Functions/Date and time functions)"
    )]
    CurrentDateTimeInOrderBy(String),

    /// Branch (non-trunk) version addressing. Trunk-only per the storage design
    /// PORT NOTE. QUERY §Predicates/Standard predicate (version).
    #[error("branch version addressing is not supported (trunk-only; QUERY §Predicates)")]
    BranchVersionAddressing,

    /// Both `TOP n` and `LIMIT n` in one query. QUERY §Query structure/LIMIT.
    #[error("TOP and LIMIT cannot be combined in one query (QUERY §Query structure/LIMIT)")]
    TopWithLimit,

    /// A named scalar function that is not on the supported whitelist.
    /// QUERY §Functions.
    #[error("function `{0}` is not supported (QUERY §Functions)")]
    UnsupportedFunction(String),

    /// A version predicate addressing metadata the planner does not model.
    /// QUERY §Predicates/Standard predicate (version).
    #[error("version predicate on `{0}` is not supported (QUERY §Predicates/Standard predicate)")]
    UnsupportedVersionPredicate(String),
}

/// A path-analysis / typing failure against the generated RM model
/// (`openehr_rm::model`).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AnalysisError {
    /// A FROM class name absent from the RM model.
    #[error("unknown RM class `{0}` (not in the generated RM model)")]
    UnknownClass(String),

    /// An identified-path root that is not a variable bound in FROM.
    #[error("unknown variable `{0}` (not bound in the FROM clause)")]
    UnknownVariable(String),

    /// An attribute that no candidate type of the current path step declares.
    #[error("attribute `{attribute}` is not defined on {on} (RM model)")]
    UnresolvableAttribute {
        /// The unresolved attribute name.
        attribute: String,
        /// A human description of the type(s) it was looked up on.
        on: String,
    },

    /// A parameter (`$name`) referenced by the query but not supplied.
    #[error("unbound query parameter `${0}`")]
    UnboundParameter(String),

    /// An operand typing that cannot be reconciled (e.g. comparing a whole
    /// structure object to a scalar literal).
    #[error("type mismatch: {0}")]
    TypeMismatch(String),
}

/// An IR→SQL lowering failure: a construct the planner accepted but the SQL
/// package cannot (yet) render. Distinct from [`AqlFeatureError`] (rejected at
/// planning time) — these surface at SQL build time.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SqlError {
    /// A SELECT / WHERE construct the SQL generator does not render yet.
    #[error("SQL generation for `{0}` is not supported yet")]
    Unsupported(String),

    /// A `$parameter` referenced during SQL generation had no supplied value
    /// (should have been caught by [`super::plan`]; a defensive guard).
    #[error("unbound query parameter `${0}` at SQL build time")]
    UnboundParameter(String),

    /// A REST paging parameter (`fetch`/`offset`) combined with an AQL
    /// `LIMIT`/`OFFSET`/`TOP`. ITS-REST query Request (`fetch` "cannot be
    /// combined with AQL-top") + QUERY §Query structure/LIMIT.
    #[error(
        "the `{param}` query parameter cannot be combined with an AQL {aql} clause \
         (ITS-REST query Request; QUERY §Query structure/LIMIT)"
    )]
    PagingConflict {
        /// The REST parameter in conflict (`fetch` or `offset`).
        param: &'static str,
        /// The AQL clause in conflict (`LIMIT`/`OFFSET`/`TOP`).
        aql: &'static str,
    },
}

/// An execution / `RESULT_SET` assembly failure.
#[derive(Debug, Error)]
pub enum ExecError {
    /// The database rejected or failed the generated query.
    #[error("query execution failed: {0}")]
    Database(#[from] sqlx::Error),

    /// A whole-object `RESULT_SET` cell could not be reassembled from its node
    /// subtree (a storage/codec failure).
    #[error("result assembly failed: {0}")]
    Assembly(#[from] crate::storage::StorageError),
}
