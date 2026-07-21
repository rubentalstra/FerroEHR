//! Cross-artifact validation — the schedule's machine gates, generalized
//! from ECC's coverage-guard discipline: id uniqueness, SM-operation
//! resolution, spec-ref link checks, binding completeness, `verified_by`
//! resolution, corpus integrity, ambiguity/option resolution,
//! capability-vs-tier consistency, reference/sentinel grammar,
//! decision-table literals, and vocabulary drift.
//!
//! Every check is pure over the loaded [`ArtifactSet`] (+ the vendored spec
//! tree for the two resolution checks); every violation is one typed
//! [`Finding`].

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::artifacts::ArtifactSet;
use crate::ids::{CaseId, CorpusKey, SmOperationRef, ViewName};
use crate::literal::{Literal, ViolationRef};
use crate::load::LoadError;
use crate::model::assertion::{Assertion, EquivalentTarget, assertion_refs};
use crate::model::case::{CaseCore, ExpectSpec, MatrixCell, Parameters};
use crate::refgrammar::{CaptureField, TimeExpr, ValueRef};
use crate::vocab::{CaseKind, Iteration, OutcomeKind};

/// The check taxonomy (one id per machine gate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CheckId {
    /// File-level load failure (YAML, schema, or typed parse).
    Load,
    /// Case-id uniqueness.
    IdUniqueness,
    /// Kind-conditional structure + assertion/parameter invariants.
    KindShape,
    /// `${…}` reference resolution + sentinel discipline.
    ReferenceGrammar,
    /// Decision-table literals + violation categories.
    LiteralGrammar,
    /// `sm_operation` (and flow `call:`) resolution against the vendored SM.
    SmOperation,
    /// `spec_refs` resolution against the vendored spec tree.
    SpecRef,
    /// Every used outcome kind mapped, every used capture wired, per binding.
    BindingCompleteness,
    /// `verified_by` targets exist.
    VerifiedBy,
    /// Corpus keys/views/sources exist; entry invariants hold.
    CorpusIntegrity,
    /// `ambiguities` ids resolve in the register.
    AmbiguityLink,
    /// `option:` tags resolve to an `option_select` register entry.
    OptionTag,
    /// Capabilities exist in the matrix; profile tiers consistent.
    CapabilityTier,
    /// Published vocabulary files drift from the compiled enums.
    VocabDrift,
}

impl CheckId {
    /// Stable token for reports/tests.
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::IdUniqueness => "id-uniqueness",
            Self::KindShape => "kind-shape",
            Self::ReferenceGrammar => "reference-grammar",
            Self::LiteralGrammar => "literal-grammar",
            Self::SmOperation => "sm-operation",
            Self::SpecRef => "spec-ref",
            Self::BindingCompleteness => "binding-completeness",
            Self::VerifiedBy => "verified-by",
            Self::CorpusIntegrity => "corpus-integrity",
            Self::AmbiguityLink => "ambiguity-link",
            Self::OptionTag => "option-tag",
            Self::CapabilityTier => "capability-tier",
            Self::VocabDrift => "vocab-drift",
        }
    }
}

/// One violation.
#[derive(Debug, Clone)]
pub struct Finding {
    pub check: CheckId,
    /// The offending artifact (file path or case id).
    pub artifact: String,
    pub message: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.check.token(),
            self.artifact,
            self.message
        )
    }
}

/// Validation context: the artifact set + optional vendored-spec root
/// (`docs/specs/openehr`) enabling the two resolution checks.
#[derive(Debug)]
pub struct Context<'a> {
    pub set: &'a ArtifactSet,
    pub load_errors: &'a [LoadError],
    pub spec_root: Option<&'a Path>,
}

/// Run every gate; findings in check order.
#[must_use]
pub fn validate(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for e in ctx.load_errors {
        findings.push(Finding {
            check: CheckId::Load,
            artifact: e.path().display().to_string(),
            message: e.detail(),
        });
    }

    check_id_uniqueness(ctx.set, &mut findings);
    for (_path, case) in &ctx.set.cases {
        let who = case.id.to_string();
        check_kind_shape(case, &who, &mut findings);
        check_references(case, &who, ctx.set, &mut findings);
        check_literals(case, &who, &mut findings);
        check_capability_tier(case, &who, ctx.set, &mut findings);
        check_links(case, &who, ctx.set, &mut findings);
        if let Some(spec_root) = ctx.spec_root {
            check_sm_operations(case, &who, spec_root, &mut findings);
            check_spec_refs(case, &who, spec_root, &mut findings);
        }
    }
    check_binding_completeness(ctx.set, &mut findings);
    for (path, binding) in &ctx.set.bindings {
        let who = path.display().to_string();
        if let Err(message) = binding.check_invariants() {
            push(&mut findings, CheckId::KindShape, &who, message);
        }
        if let (Some(decl), Some((_, register))) = (&binding.unrealized, &ctx.set.register)
            && register.get(&decl.ambiguity).is_none()
        {
            push(
                &mut findings,
                CheckId::AmbiguityLink,
                &who,
                format!("unrealized declaration cites {} which is not in the register", decl.ambiguity),
            );
        }
        if let Some(spec_root) = ctx.spec_root {
            resolve_sm_operation(&binding.sm_operation, &who, spec_root, &mut findings);
        }
    }
    check_corpus_integrity(ctx.set, &mut findings);
    check_vocab_drift(ctx.set, &mut findings);

    findings
}

fn push(findings: &mut Vec<Finding>, check: CheckId, artifact: &str, message: String) {
    findings.push(Finding {
        check,
        artifact: artifact.to_owned(),
        message,
    });
}

// ── id uniqueness ───────────────────────────────────────────────────────────

fn check_id_uniqueness(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (path, case) in &set.cases {
        if !seen.insert(case.id.as_str()) {
            push(
                findings,
                CheckId::IdUniqueness,
                &path.display().to_string(),
                format!("case id {} is declared more than once", case.id),
            );
        }
    }
}

// ── kind shape + structural invariants ──────────────────────────────────────

fn check_kind_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    check_kind_blocks(case, who, findings);
    check_parameters_shape(case, who, findings);
    check_assertion_shape(case, who, findings);
    check_flow_shape(case, who, findings);
    for guard in &case.guards {
        if !guard.contains('—') && !guard.to_lowercase().contains("master") {
            push(
                findings,
                CheckId::KindShape,
                who,
                format!("guard {guard:?} carries no spec citation"),
            );
        }
    }
}

fn check_kind_blocks(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    match case.kind {
        CaseKind::Functional => {
            if case.sm_operation.is_none() || case.flow.is_empty() {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "functional case must carry sm_operation and a non-empty flow".to_owned(),
                );
            }
            if case.decision_table.is_some() || case.constraint_context.is_some() {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "functional case must not carry content blocks".to_owned(),
                );
            }
        }
        CaseKind::Content => {
            if case.rm_class.is_none()
                || case.constraint_context.is_none()
                || case.decision_table.is_none()
            {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "content case must carry rm_class, constraint_context and decision_table"
                        .to_owned(),
                );
            }
        }
    }
}

fn check_parameters_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    if let Some(parameters) = &case.parameters {
        match (&parameters.matrix, &parameters.fixture_set) {
            (Some(_), Some(_)) | (None, None) => push(
                findings,
                CheckId::KindShape,
                who,
                "parameters must carry exactly one of matrix | fixture_set".to_owned(),
            ),
            _ => {}
        }
        if let Some(matrix) = &parameters.matrix {
            for (i, row) in matrix.rows.iter().enumerate() {
                if row.len() != matrix.columns.len() {
                    push(
                        findings,
                        CheckId::KindShape,
                        who,
                        format!(
                            "matrix row {i} has {} cells for {} columns",
                            row.len(),
                            matrix.columns.len()
                        ),
                    );
                }
            }
            let expected_col = matrix.columns.iter().position(|c| c == "expected");
            if let Some(col) = expected_col {
                for (i, row) in matrix.rows.iter().enumerate() {
                    match row.get(col) {
                        Some(MatrixCell::Literal(serde_json::Value::String(s)))
                            if OutcomeKind::from_token(s).is_some() => {}
                        _ => push(
                            findings,
                            CheckId::KindShape,
                            who,
                            format!("matrix row {i}: `expected` cell must be an outcome kind"),
                        ),
                    }
                }
            }
        }
    }
}

fn check_assertion_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    let aggregate_needs_single_pass = case
        .postconditions
        .iter()
        .chain(case.flow.iter().flat_map(|s| s.assertions.iter()))
        .any(Assertion::is_aggregate);
    if aggregate_needs_single_pass {
        let single_pass = matches!(
            case.parameters,
            Some(Parameters {
                iteration: Iteration::SinglePass,
                ..
            })
        );
        if !single_pass {
            push(
                findings,
                CheckId::KindShape,
                who,
                "aggregate assertions require parameters.iteration: single_pass".to_owned(),
            );
        }
    }

    for assertion in case
        .postconditions
        .iter()
        .chain(case.flow.iter().flat_map(|s| s.assertions.iter()))
    {
        if let Err(message) = assertion.check_invariants() {
            push(findings, CheckId::KindShape, who, message);
        }
        if let Assertion::State { verified_by, .. } = assertion {
            let verified = verified_by.is_some()
                || !case.verified_by.is_empty()
                || case.flow.len() > 1
                || case.flow.iter().any(|s| !s.assertions.is_empty());
            if !verified {
                push(
                    findings,
                    CheckId::KindShape,
                    who,
                    "state assertion needs verified_by or an in-case verification step".to_owned(),
                );
            }
        }
    }
}

fn check_flow_shape(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    let mut last_step = 0_u32;
    for step in &case.flow {
        if step.step <= last_step {
            push(
                findings,
                CheckId::KindShape,
                who,
                format!(
                    "flow step numbers must strictly increase (step {})",
                    step.step
                ),
            );
        }
        last_step = step.step;
        if matches!(step.expect, ExpectSpec::FixtureExpected)
            && case
                .parameters
                .as_ref()
                .is_none_or(|p| p.fixture_set.is_none())
        {
            push(
                findings,
                CheckId::KindShape,
                who,
                "expect: ${fixture.expected} requires parameters.fixture_set".to_owned(),
            );
        }
        if let ExpectSpec::Kind(kind) = step.expect {
            for (name, source) in step.captures() {
                if source.outcome != kind {
                    push(
                        findings,
                        CheckId::KindShape,
                        who,
                        format!(
                            "capture {name} reads outcome `{}` but the step expects `{}`",
                            source.outcome.token(),
                            kind.token()
                        ),
                    );
                }
            }
        }
    }
}

// ── reference resolution ────────────────────────────────────────────────────

fn matrix_columns(case: &CaseCore) -> Vec<&str> {
    case.parameters
        .as_ref()
        .and_then(|p| p.matrix.as_ref())
        .map(|m| m.columns.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

struct RefCtx<'a> {
    columns: Vec<&'a str>,
    has_fixtures: bool,
    who: &'a str,
    set: &'a ArtifactSet,
}

fn check_one_ref(
    r: &ValueRef,
    ctx: &RefCtx<'_>,
    defined: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
) {
    let who = ctx.who;
    match r {
        ValueRef::Row(column) => {
            if !ctx.columns.contains(&column.as_str()) {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{row.{column}}} names no matrix column"),
                );
            }
        }
        ValueRef::Fixture(_) | ValueRef::FixtureDataSet => {
            if !ctx.has_fixtures {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    "${fixture.*} reference without parameters.fixture_set".to_owned(),
                );
            }
        }
        ValueRef::Capture { name, optional } => {
            if *optional {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{{name}?}} optional form is binding-template-only"),
                );
            } else if !defined.contains(name.as_str()) {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{{name}}} references no earlier capture or requires handle"),
                );
            }
        }
        ValueRef::DataSet { key, view } => {
            check_ds_ref(key, view.as_ref(), who, ctx.set, findings);
        }
        ValueRef::Recipe(name) => {
            let declared = ctx.set.corpus.as_ref().is_some_and(|(_, corpus)| {
                corpus.entries().iter().any(|(_, e)| {
                    e.recipes
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .any(|(n, _)| n == name)
                        || e.generated_by.as_ref().is_some_and(|g| g.recipe == *name)
                })
            });
            if !declared {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    format!("${{recipe:{name}(row)}} is not declared in the corpus manifest"),
                );
            }
        }
        ValueRef::Time(expr) => {
            let (a, b) = match expr {
                TimeExpr::Before(t) | TimeExpr::After(t) => (t, None),
                TimeExpr::Between(t1, t2) => (t1, Some(t2)),
            };
            for t in std::iter::once(a).chain(b) {
                if !defined.contains(t.as_str()) {
                    push(
                        findings,
                        CheckId::ReferenceGrammar,
                        who,
                        format!("${{time:…({t})}} references no earlier capture"),
                    );
                }
            }
        }
    }
}

fn check_references(case: &CaseCore, who: &str, set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let ctx = RefCtx {
        columns: matrix_columns(case),
        has_fixtures: case
            .parameters
            .as_ref()
            .is_some_and(|p| p.fixture_set.is_some()),
        who,
        set,
    };
    let mut defined: BTreeSet<String> = case
        .requires
        .minted_handles()
        .iter()
        .map(ToString::to_string)
        .collect();

    for step in &case.flow {
        // `with` executes before the step's captures exist ...
        for (_, value) in step.with_entries() {
            for r in value.refs() {
                check_one_ref(r, &ctx, &defined, findings);
            }
        }
        for (name, _source) in step.captures() {
            defined.insert(name.to_string());
        }
        // ... while post-step assertions run after capture and may use them.
        for assertion in &step.assertions {
            for r in assertion_refs(assertion) {
                check_one_ref(&r, &ctx, &defined, findings);
            }
        }
    }
    for assertion in &case.postconditions {
        for r in assertion_refs(assertion) {
            check_one_ref(&r, &ctx, &defined, findings);
        }
        if let Assertion::Equivalent {
            to: EquivalentTarget::Committed,
            ..
        } = assertion
        {
            // `to: committed` needs something committed in-flow — a step that
            // carried a payload.
            if !case.flow.iter().any(|s| !s.with_entries().is_empty()) {
                push(
                    findings,
                    CheckId::ReferenceGrammar,
                    who,
                    "equivalent to: committed, but no flow step commits a payload".to_owned(),
                );
            }
        }
    }

    for key in case
        .data_sets
        .iter()
        .chain(case.requires.templates.iter())
        .chain(case.requires.commit.iter())
        .chain(case.constraint_context.as_ref().map(|c| &c.template))
    {
        check_ds_ref(key, None, who, set, findings);
    }
}

fn check_ds_ref(
    key: &CorpusKey,
    view: Option<&ViewName>,
    who: &str,
    set: &ArtifactSet,
    findings: &mut Vec<Finding>,
) {
    let Some((_, corpus)) = &set.corpus else {
        return;
    };
    match corpus.get(key) {
        None => push(
            findings,
            CheckId::CorpusIntegrity,
            who,
            format!("corpus key {key} is not in the manifest"),
        ),
        Some(entry) => {
            if let Some(view) = view
                && entry.view(view).is_none()
            {
                push(
                    findings,
                    CheckId::CorpusIntegrity,
                    who,
                    format!("view {view} is not declared on corpus entry {key}"),
                );
            }
        }
    }
}

// ── literals ────────────────────────────────────────────────────────────────

fn check_literals(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    let Some(table) = &case.decision_table else {
        return;
    };
    let violates_col = table.columns.iter().position(|c| c == "violates");
    let expected_col = table.columns.iter().position(|c| c == "expected");
    for (i, row) in table.rows.iter().enumerate() {
        for (j, cell) in row.iter().enumerate() {
            if Some(j) == violates_col {
                match cell {
                    serde_json::Value::Array(items) => {
                        for item in items {
                            match item.as_str().map(ViolationRef::parse) {
                                Some(Ok(_)) => {}
                                Some(Err(e)) => push(
                                    findings,
                                    CheckId::LiteralGrammar,
                                    who,
                                    format!("row {i}: {e}"),
                                ),
                                None => push(
                                    findings,
                                    CheckId::LiteralGrammar,
                                    who,
                                    format!("row {i}: violates entries must be strings"),
                                ),
                            }
                        }
                    }
                    _ => push(
                        findings,
                        CheckId::LiteralGrammar,
                        who,
                        format!("row {i}: violates cell must be a list"),
                    ),
                }
            } else if Some(j) == expected_col {
                if !matches!(cell.as_str(), Some("accepted" | "rejected")) {
                    push(
                        findings,
                        CheckId::LiteralGrammar,
                        who,
                        format!("row {i}: expected cell must be accepted | rejected"),
                    );
                }
            } else if let Err(e) = Literal::from_cell(cell) {
                push(
                    findings,
                    CheckId::LiteralGrammar,
                    who,
                    format!("row {i}: {e}"),
                );
            }
        }
    }
}

// ── capability / tier ───────────────────────────────────────────────────────

fn check_capability_tier(
    case: &CaseCore,
    who: &str,
    set: &ArtifactSet,
    findings: &mut Vec<Finding>,
) {
    let Some((_, matrix)) = &set.matrix else {
        return;
    };
    let mut tiers = BTreeSet::new();
    for capability in case.capabilities.iter().chain(case.exercises.iter()) {
        match matrix.get(capability) {
            None => push(
                findings,
                CheckId::CapabilityTier,
                who,
                format!("capability {capability} is not in the capability matrix"),
            ),
            Some(entry) => {
                if case.capabilities.contains(capability) {
                    tiers.insert(entry.tier);
                }
            }
        }
    }
    for tier in &case.profiles {
        if !tiers.contains(tier) {
            push(
                findings,
                CheckId::CapabilityTier,
                who,
                format!(
                    "profiles lists {tier:?} but no verdict-bearing capability carries that tier"
                ),
            );
        }
    }
    for tier in &tiers {
        if !case.profiles.contains(tier) {
            push(
                findings,
                CheckId::CapabilityTier,
                who,
                format!("capability tier {tier:?} is missing from profiles"),
            );
        }
    }
}

// ── register links ──────────────────────────────────────────────────────────

fn check_links(case: &CaseCore, who: &str, set: &ArtifactSet, findings: &mut Vec<Finding>) {
    if let Some((_, register)) = &set.register {
        for id in &case.ambiguities {
            if register.get(id).is_none() {
                push(
                    findings,
                    CheckId::AmbiguityLink,
                    who,
                    format!("{id} is not in the ambiguity register"),
                );
            }
        }
        if let Some(option) = &case.option
            && !register.declares_option(option)
        {
            push(
                findings,
                CheckId::OptionTag,
                who,
                format!("option tag {option} is not declared by any option_select register entry"),
            );
        }
    }
    let ids: BTreeSet<&CaseId> = set.cases.iter().map(|(_, c)| &c.id).collect();
    let assertion_targets = case
        .postconditions
        .iter()
        .chain(case.flow.iter().flat_map(|s| s.assertions.iter()))
        .filter_map(|a| match a {
            Assertion::State {
                verified_by: Some(target),
                ..
            } => Some(target),
            _ => None,
        });
    for target in case.verified_by.iter().chain(assertion_targets) {
        if !ids.contains(target) {
            push(
                findings,
                CheckId::VerifiedBy,
                who,
                format!("verified_by target {target} does not exist"),
            );
        }
    }
}

// ── SM + spec-ref resolution ────────────────────────────────────────────────

fn sm_class_file(spec_root: &Path, interface: &str) -> PathBuf {
    spec_root
        .join("SM/docs/UML/classes")
        .join(format!("{}.adoc", interface.to_lowercase()))
}

fn resolve_sm_operation(
    op: &SmOperationRef,
    who: &str,
    spec_root: &Path,
    findings: &mut Vec<Finding>,
) {
    let file = sm_class_file(spec_root, op.interface());
    match std::fs::read_to_string(&file) {
        Err(_) => push(
            findings,
            CheckId::SmOperation,
            who,
            format!(
                "interface {} has no vendored SM class export ({})",
                op.interface(),
                file.display()
            ),
        ),
        Ok(text) => {
            if !text.contains(&format!("|*{}*", op.operation())) {
                push(
                    findings,
                    CheckId::SmOperation,
                    who,
                    format!(
                        "operation {op} is not defined by the vendored SM interface ({})",
                        file.display()
                    ),
                );
            }
        }
    }
}

fn check_sm_operations(case: &CaseCore, who: &str, spec_root: &Path, findings: &mut Vec<Finding>) {
    let Some(anchor) = &case.sm_operation else {
        return;
    };
    resolve_sm_operation(anchor, who, spec_root, findings);
    for step in &case.flow {
        let op = if step.call.contains('.') {
            match SmOperationRef::parse(&step.call) {
                Ok(op) => op,
                Err(e) => {
                    push(findings, CheckId::SmOperation, who, e.to_string());
                    continue;
                }
            }
        } else {
            anchor.sibling(&step.call)
        };
        resolve_sm_operation(&op, who, spec_root, findings);
    }
}

/// Component token → vendored directory.
fn component_dir(token: &str) -> Option<&'static str> {
    Some(match token {
        "SM" => "SM",
        "CNF" => "CNF",
        "RM" => "RM",
        "BASE" => "BASE",
        "AM" => "AM",
        "QUERY" => "QUERY",
        "TERM" => "TERM",
        "LANG" => "LANG",
        "ITS-REST" => "ITS-REST",
        "ITS-JSON" => "ITS-JSON",
        "ITS-XML" => "ITS-XML",
        _ => return None,
    })
}

fn check_spec_refs(case: &CaseCore, who: &str, spec_root: &Path, findings: &mut Vec<Finding>) {
    for citation in &case.spec_refs {
        let mut tokens = citation.split_whitespace();
        let Some(component) = tokens.next() else {
            push(findings, CheckId::SpecRef, who, "empty spec_ref".to_owned());
            continue;
        };
        let Some(dir) = component_dir(component) else {
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!("{citation:?}: unknown component {component:?}"),
            );
            continue;
        };
        let root = spec_root.join(dir);
        if !root.is_dir() {
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!(
                    "{citation:?}: vendored component dir {} missing",
                    root.display()
                ),
            );
            continue;
        }
        // The document token: the first token before the § section marker.
        let doc_token = tokens.next().map(|t| t.trim_end_matches(',').to_owned());
        let Some(doc_token) = doc_token.filter(|t| !t.starts_with('§')) else {
            continue; // component-only citation: dir existence was the check
        };
        if !path_contains_token(&root, &doc_token.to_lowercase()) {
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!("{citation:?}: no vendored path under {dir} matches {doc_token:?}"),
            );
        }
    }
}

/// Case-insensitive substring match of `token` against any path under `root`.
fn path_contains_token(root: &Path, token: &str) -> bool {
    let mut stack = vec![root.to_owned()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let matches = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.to_lowercase().contains(token));
            if matches {
                return true;
            }
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    false
}

// ── binding completeness ────────────────────────────────────────────────────

fn check_binding_completeness(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    for (_, case) in &set.cases {
        let Some(anchor) = &case.sm_operation else {
            continue;
        };
        let who = case.id.to_string();
        for step in &case.flow {
            let op = if step.call.contains('.') {
                match SmOperationRef::parse(&step.call) {
                    Ok(op) => op,
                    Err(_) => continue, // reported by the SM check
                }
            } else {
                anchor.sibling(&step.call)
            };
            let bindings: Vec<_> = set
                .bindings
                .iter()
                .filter(|(_, b)| b.sm_operation == op)
                .collect();
            if bindings.is_empty() {
                push(
                    findings,
                    CheckId::BindingCompleteness,
                    &who,
                    format!("no binding declares operation {op}"),
                );
                continue;
            }
            // An explicit `unrealized` declaration satisfies completeness:
            // the gap is machine-readable and the interpreter yields
            // not-applicable with its citation on that ITS.
            if bindings.iter().all(|(_, b)| b.is_unrealized()) {
                continue;
            }
            // Kinds this step may observe: the fixed expectation, or every
            // fixture-set `expected` kind when per-fixture.
            let mut kinds: Vec<OutcomeKind> = Vec::new();
            match step.expect {
                ExpectSpec::Kind(kind) => kinds.push(kind),
                ExpectSpec::FixtureExpected => {
                    if let Some(fixtures) = case
                        .parameters
                        .as_ref()
                        .and_then(|p| p.fixture_set.as_ref())
                    {
                        kinds.extend(fixtures.iter().map(|f| f.expected));
                    }
                }
            }
            let expected_column_kinds: Vec<OutcomeKind> = case
                .parameters
                .as_ref()
                .and_then(|p| p.matrix.as_ref())
                .map(|m| {
                    let col = m.columns.iter().position(|c| c == "expected");
                    col.map(|col| {
                        m.rows
                            .iter()
                            .filter_map(|row| match row.get(col) {
                                Some(MatrixCell::Literal(serde_json::Value::String(s))) => {
                                    OutcomeKind::from_token(s)
                                }
                                _ => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default()
                })
                .unwrap_or_default();
            kinds.extend(expected_column_kinds);

            let universal: Vec<&str> = set
                .selectors
                .as_ref()
                .and_then(|(_, s)| s.universal_outcomes.as_deref())
                .unwrap_or_default()
                .iter()
                .map(|(k, _)| k.as_str())
                .collect();
            for (path, binding) in bindings {
                for kind in &kinds {
                    if universal.contains(&kind.token()) {
                        continue;
                    }
                    if binding.outcome(*kind).is_none() {
                        push(
                            findings,
                            CheckId::BindingCompleteness,
                            &who,
                            format!(
                                "outcome kind `{}` on {op} is not mapped by {}",
                                kind.token(),
                                path.display()
                            ),
                        );
                    }
                }
                for (name, source) in step.captures() {
                    if let CaptureField::Field { name: field, .. } = &source.field
                        && !binding.maps_capture(field)
                    {
                        push(
                            findings,
                            CheckId::BindingCompleteness,
                            &who,
                            format!(
                                "capture {name} needs wire source `{field}` on {op}, not mapped by {}",
                                path.display()
                            ),
                        );
                    }
                }
            }
        }
    }
}

// ── corpus integrity ────────────────────────────────────────────────────────

fn check_corpus_integrity(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((path, corpus)) = &set.corpus else {
        return;
    };
    let who = path.display().to_string();
    for (key, entry) in corpus.entries() {
        if let Err(message) = entry.check_invariants() {
            push(
                findings,
                CheckId::CorpusIntegrity,
                &who,
                format!("{key}: {message}"),
            );
        }
        if let (Some(source), Some(dir)) = (&entry.source, &set.corpus_dir) {
            let file = dir.join(source);
            if !file.is_file() {
                push(
                    findings,
                    CheckId::CorpusIntegrity,
                    &who,
                    format!("{key}: source {source} does not exist"),
                );
            }
        }
    }
}

// ── vocabulary drift ────────────────────────────────────────────────────────

fn check_vocab_drift(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    if let Some((path, outcomes)) = &set.outcomes
        && let Err(drift) = outcomes.check_against_enum()
    {
        for message in drift {
            push(
                findings,
                CheckId::VocabDrift,
                &path.display().to_string(),
                message,
            );
        }
    }
    if let Some((path, selectors)) = &set.selectors
        && let Err(drift) = selectors.check_against_enum()
    {
        for message in drift {
            push(
                findings,
                CheckId::VocabDrift,
                &path.display().to_string(),
                message,
            );
        }
    }
    if let Some((path, matrix)) = &set.matrix
        && let Err(drift) = matrix.check_tier_scoping()
    {
        for message in drift {
            push(
                findings,
                CheckId::VocabDrift,
                &path.display().to_string(),
                message,
            );
        }
    }
    if let Some((path, register)) = &set.register {
        for (id, entry) in register.entries() {
            if let Err(message) = entry.check_invariants() {
                push(
                    findings,
                    CheckId::VocabDrift,
                    &path.display().to_string(),
                    format!("{id}: {message}"),
                );
            }
        }
    }
}
