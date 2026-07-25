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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::artifacts::ArtifactSet;
use crate::ids::{CaseId, CorpusKey, SmOperationRef, ViewName};
use crate::literal::{Literal, ViolationRef};
use crate::load::LoadError;
use crate::model::assertion::{Assertion, EquivalentTarget, assertion_refs};
use crate::model::binding::OperationBinding;
use crate::model::case::{CaseCore, ExpectSpec, FlowStep, MatrixCell, Parameters};
use crate::model::wire_surface::WireSurface;
use crate::refgrammar::{CaptureField, TimeExpr, ValueRef};
use crate::vocab::{CaseKind, FormatName, Iteration, OutcomeKind};

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
    /// Journey-catalogue invariants + the population-anchored envelope
    /// reconciliation of every performance workload (write share inside the
    /// 10:1..50:1 derivation band; stage templates resolve in the corpus).
    JourneyEnvelope,
    /// Total wire-surface coverage (issue #271): every spec-defined wire
    /// behaviour — SM operations (Axis 1), per-binding outcome/format branches
    /// (Axis 2), cross-cutting behaviours (Axis 3) — is exercised by ≥ 1 case
    /// or carries an adjudicated `vocab/wire_surface.yaml` exception. Silence
    /// is not coverage.
    SurfaceCoverage,
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
            Self::JourneyEnvelope => "journey-envelope",
            Self::SurfaceCoverage => "surface-coverage",
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
                format!(
                    "unrealized declaration cites {} which is not in the register",
                    decl.ambiguity
                ),
            );
        }
        if let Some(spec_root) = ctx.spec_root {
            resolve_sm_operation(&binding.sm_operation, &who, spec_root, &mut findings);
        }
    }
    check_corpus_integrity(ctx.set, &mut findings);
    check_vocab_drift(ctx.set, &mut findings);
    check_journey_envelope(ctx.set, &mut findings);
    check_surface_coverage(ctx.set, ctx.spec_root, &mut findings);

    findings
}

/// The journey catalogue's own invariants, every performance workload's
/// envelope reconciliation through it, and the resolution of every stage
/// template (OPT + example payload) in the corpus manifest.
fn check_journey_envelope(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((path, catalogue)) = &set.journeys else {
        if !set.performance.is_empty() {
            push(
                findings,
                CheckId::JourneyEnvelope,
                "vocab/journey_catalogue.yaml",
                "performance cases exist but the journey catalogue is missing".to_owned(),
            );
        }
        return;
    };
    let who = path.display().to_string();
    if let Err(message) = catalogue.check_invariants() {
        push(findings, CheckId::JourneyEnvelope, &who, message);
        return;
    }
    // Every stage template resolves in the corpus manifest: the OPT entry
    // (the constraint carrier the seeder uploads) and its `.example`
    // sibling (the committed payload skeleton the driver commits).
    if let Some((_, manifest)) = &set.corpus {
        for (name, journey) in &catalogue.0 {
            for stage in &journey.stages {
                let Some(template) = &stage.template else {
                    continue;
                };
                for (key, role) in [
                    (template.clone(), "OPT"),
                    (format!("{template}.example"), "example payload"),
                ] {
                    match CorpusKey::parse(&key) {
                        Ok(parsed) if manifest.get(&parsed).is_some() => {}
                        _ => push(
                            findings,
                            CheckId::JourneyEnvelope,
                            &who,
                            format!(
                                "journey {name} template {template}: corpus manifest has no \
                                 {role} entry {key}"
                            ),
                        ),
                    }
                }
            }
        }
    }
    for (case_path, case) in &set.performance {
        if let Err(message) = catalogue.expansion(&case.workload.journeys) {
            push(
                findings,
                CheckId::JourneyEnvelope,
                &case_path.display().to_string(),
                message,
            );
        }
    }
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

/// Kinds a step may observe: the fixed expectation, every fixture-set
/// `expected` kind when per-fixture, plus any matrix `expected` column kinds.
fn step_observable_kinds(case: &CaseCore, step: &crate::model::case::FlowStep) -> Vec<OutcomeKind> {
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
    if let Some(matrix) = case.parameters.as_ref().and_then(|p| p.matrix.as_ref())
        && let Some(col) = matrix.columns.iter().position(|c| c == "expected")
    {
        kinds.extend(matrix.rows.iter().filter_map(|row| match row.get(col) {
            Some(MatrixCell::Literal(serde_json::Value::String(s))) => OutcomeKind::from_token(s),
            _ => None,
        }));
    }
    kinds
}

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
            let mut bindings: Vec<_> = set
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
            // Mirror the interpreter's binding selection (`binding_for_variant`):
            // a step's `variant` selects the binding declaring that variant;
            // a variant-less step (or a variant with no dedicated binding)
            // falls back to the variant-less binding. Completeness is judged
            // against the binding the interpreter would actually drive, not
            // against every binding of the operation.
            if let Some(v) = &step.variant
                && bindings
                    .iter()
                    .any(|(_, b)| b.variant.as_deref() == Some(v.as_str()))
            {
                bindings.retain(|(_, b)| b.variant.as_deref() == Some(v.as_str()));
            } else {
                let has_variantless = bindings.iter().any(|(_, b)| b.variant.is_none());
                if has_variantless {
                    bindings.retain(|(_, b)| b.variant.is_none());
                }
            }
            // An explicit `unrealized` declaration satisfies completeness:
            // the gap is machine-readable and the interpreter yields
            // not-applicable with its citation on that ITS.
            if bindings.iter().all(|(_, b)| b.is_unrealized()) {
                continue;
            }
            let kinds = step_observable_kinds(case, step);

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

// ── surface coverage (issue #271) ───────────────────────────────────────────

/// The platform interfaces the CNF catalogue speaks — the Axis-1 SM-operation
/// enumeration domain. The set is the openEHR SM Platform Service Model's
/// platform interfaces (`docs/specs/openehr/SM/docs/UML/classes/`), which
/// anchor the operation identities the case cores use; it is NOT derived from
/// the vendored OAS (owner ruling 2026-07-24, `.claude/rules/spec-adherence.md`
/// — the OAS is `emit-rest` codegen input, never a surface source). Every
/// listed interface has a vendored `i_*.adoc` class export (a missing file is
/// itself a `surface-coverage` finding).
///
/// Interfaces are pinned even when the catalogue binds none of their operations
/// (`I_EHR_INDEX`, `I_TERMINOLOGY_SERVICE`, `I_VALIDITY_CHECKER`,
/// `I_SUBJECT_PROXY_SERVICE`, `I_DATA_BINDING`, `I_MESSAGE_SERVICE`,
/// `I_SYSTEM_LOG`): ITS-REST 1.1.0 surfaces no wire for those SM interfaces, so
/// their operations become explicit, individually cited `off_wire` entries in
/// `vocab/wire_surface.yaml` — a visible, ratchetable boundary, never a silent
/// omission. `I_ADMIN_ARCHIVE` / `I_ADMIN_DUMP_LOAD` are pinned alongside
/// `I_ADMIN_SERVICE` because the catalogue binds them (unrealized) as the SM
/// Admin surface. Sub-interface navigation accessors (return type an
/// interface — `i_ehr`, `i_party`, `i_party_relationship`) are not service
/// operations and are excluded by [`sm_interface_operations`].
const PLATFORM_INTERFACES: &[&str] = &[
    "I_EHR_SERVICE",
    "I_EHR_STATUS",
    "I_EHR_COMPOSITION",
    "I_EHR_DIRECTORY",
    "I_EHR_CONTRIBUTION",
    "I_DEFINITION_ADL14",
    "I_DEFINITION_ADL2",
    "I_DEFINITION_QUERY",
    "I_QUERY_SERVICE",
    "I_DEMOGRAPHIC_SERVICE",
    "I_PARTY",
    "I_PARTY_RELATIONSHIP",
    "I_VALIDITY_CHECKER",
    "I_ADMIN_SERVICE",
    "I_ADMIN_ARCHIVE",
    "I_ADMIN_DUMP_LOAD",
    "I_EHR_INDEX",
    "I_TERMINOLOGY_SERVICE",
    "I_MESSAGE_SERVICE",
    "I_EHR_EXTRACT_SERVICE",
    "I_TDD_SERVICE",
    "I_SUBJECT_PROXY_SERVICE",
    "I_DATA_BINDING",
    "I_SYSTEM_LOG",
];

/// Parse the service operations of an SM interface from its vendored UML class
/// export — the same table shape [`resolve_sm_operation`] resolves against.
/// Operation rows are `|*<name>* (` (a lower-snake signature name followed by
/// its parameter list); sub-interface navigation accessors (`i_*`) are
/// excluded (they return an interface, not a service result).
///
/// # Errors
/// Returns a message when the interface has no vendored class export.
fn sm_interface_operations(spec_root: &Path, interface: &str) -> Result<Vec<String>, String> {
    let file = sm_class_file(spec_root, interface);
    let text = std::fs::read_to_string(&file).map_err(|_| {
        format!(
            "interface {interface} has no vendored SM class export ({})",
            file.display()
        )
    })?;
    let mut ops: Vec<String> = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("|*") else {
            continue;
        };
        let Some((name, tail)) = rest.split_once("* ") else {
            continue;
        };
        if !tail.starts_with('(') || name.starts_with("i_") {
            continue;
        }
        let lower_snake = name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if lower_snake && !ops.iter().any(|o| o == name) {
            ops.push(name.to_owned());
        }
    }
    Ok(ops)
}

/// The three coverage axes (Axis 1 needs the vendored SM tree; Axes 2 & 3 are
/// pure over the artifact set). An absent `wire_surface.yaml` is treated as an
/// empty register — so every gap surfaces as a finding rather than passing
/// silently.
fn check_surface_coverage(
    set: &ArtifactSet,
    spec_root: Option<&Path>,
    findings: &mut Vec<Finding>,
) {
    let empty = WireSurface::default();
    let wire_surface = set.wire_surface.as_ref().map_or(&empty, |(_, w)| w);
    if let Some(spec_root) = spec_root {
        check_surface_sm_operations(set, spec_root, wire_surface, findings);
    }
    check_binding_branch_coverage(set, wire_surface, findings);
    check_wire_surface_elements(set, wire_surface, findings);
}

/// Axis 1 — every SM operation of a pinned platform interface has an `its-rest`
/// binding (realized or unrealized) or a cited `sm_operations` exception.
fn check_surface_sm_operations(
    set: &ArtifactSet,
    spec_root: &Path,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    for interface in PLATFORM_INTERFACES {
        let ops = match sm_interface_operations(spec_root, interface) {
            Ok(ops) => ops,
            Err(message) => {
                push(findings, CheckId::SurfaceCoverage, interface, message);
                continue;
            }
        };
        for name in ops {
            let Ok(op) = SmOperationRef::parse(&format!("{interface}.{name}")) else {
                continue;
            };
            let bound = set.bindings.iter().any(|(_, b)| b.sm_operation == op);
            if bound || wire_surface.sm_exception(&op).is_some() {
                continue;
            }
            push(
                findings,
                CheckId::SurfaceCoverage,
                &op.to_string(),
                "SM operation has no its-rest binding and no wire_surface.yaml sm_operations \
                 exception — add a binding (realized or unrealized) or a cited \
                 off_wire/variant_of/coverage_gap entry"
                    .to_owned(),
            );
        }
    }
    // Ratchet: an sm_operations exception for an operation that now HAS a
    // binding is stale and must be removed (coverage only ratchets up).
    for ex in &wire_surface.sm_operations {
        if set
            .bindings
            .iter()
            .any(|(_, b)| b.sm_operation == ex.operation)
        {
            push(
                findings,
                CheckId::SurfaceCoverage,
                &ex.operation.to_string(),
                "wire_surface.yaml sm_operations exception is redundant — the operation now has \
                 an its-rest binding; remove the exception"
                    .to_owned(),
            );
        }
    }
}

/// The `(operation, variant)` key identifying a binding realization.
type BranchKey = (SmOperationRef, Option<String>);

/// Mirror the interpreter's binding selection (`exec::driver::binding_for_variant`):
/// a step's `variant` selects the binding declaring it, else the variant-less
/// binding for the operation.
fn select_binding_for_step<'a>(
    set: &'a ArtifactSet,
    case: &CaseCore,
    step: &FlowStep,
) -> Option<&'a OperationBinding> {
    let op = if step.call.contains('.') {
        SmOperationRef::parse(&step.call).ok()?
    } else {
        case.sm_operation.as_ref()?.sibling(&step.call)
    };
    if let Some(v) = &step.variant
        && let Some((_, b)) = set
            .bindings
            .iter()
            .find(|(_, b)| b.sm_operation == op && b.variant.as_deref() == Some(v.as_str()))
    {
        return Some(b);
    }
    set.bindings
        .iter()
        .find(|(_, b)| b.sm_operation == op && b.variant.is_none())
        .map(|(_, b)| b)
}

/// The effective wire format a step exercises: its explicit `format`, else the
/// case's format axis, else the canonical-JSON default the driver falls back to
/// when neither is set (`exec::driver` sets no `Content-Type`/`Accept`).
fn step_format(case: &CaseCore, step: &FlowStep) -> FormatName {
    step.format
        .or_else(|| case.formats.first().copied())
        .unwrap_or(FormatName::CanonicalJson)
}

/// Compute, per realized binding, the outcome kinds and formats the catalogue
/// exercises — the inverse of `check_binding_completeness`.
fn exercised_branches(
    set: &ArtifactSet,
) -> BTreeMap<BranchKey, (BTreeSet<OutcomeKind>, BTreeSet<FormatName>)> {
    let mut map: BTreeMap<BranchKey, (BTreeSet<OutcomeKind>, BTreeSet<FormatName>)> =
        BTreeMap::new();
    for (_, case) in &set.cases {
        for step in &case.flow {
            let Some(binding) = select_binding_for_step(set, case, step) else {
                continue;
            };
            if binding.is_unrealized() {
                continue;
            }
            let key = (binding.sm_operation.clone(), binding.variant.clone());
            let entry = map.entry(key).or_default();
            for kind in step_observable_kinds(case, step) {
                entry.0.insert(kind);
            }
            entry.1.insert(step_format(case, step));
        }
    }
    map
}

/// A binding's label for findings/report (`I_X.op` or `I_X.op#variant`).
fn binding_label(binding: &OperationBinding) -> String {
    match &binding.variant {
        Some(v) => format!("{}#{v}", binding.sm_operation),
        None => binding.sm_operation.to_string(),
    }
}

/// The published token of a format (matches `vocab/wire_surface.yaml`).
fn format_token(format: FormatName) -> &'static str {
    match format {
        FormatName::CanonicalJson => "canonical-json",
        FormatName::CanonicalXml => "canonical-xml",
        FormatName::WtFlat => "wt-flat",
        FormatName::WtStructured => "wt-structured",
        FormatName::Wt => "wt",
    }
}

/// The route-table-wide outcome tokens (mapped once in `vocab/selectors.yaml`,
/// exempt from per-binding coverage).
fn universal_outcome_tokens(set: &ArtifactSet) -> Vec<&str> {
    set.selectors
        .as_ref()
        .and_then(|(_, s)| s.universal_outcomes.as_deref())
        .unwrap_or_default()
        .iter()
        .map(|(k, _)| k.as_str())
        .collect()
}

/// Axis 2 — every realized binding's declared outcome key and format is
/// exercised by ≥ 1 case step or carries a cited `branches` exception (universal
/// outcomes exempt).
fn check_binding_branch_coverage(
    set: &ArtifactSet,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    let exercised = exercised_branches(set);
    let universal = universal_outcome_tokens(set);
    for (_, binding) in &set.bindings {
        if binding.is_unrealized() {
            continue;
        }
        let variant = binding.variant.as_deref();
        let key = (binding.sm_operation.clone(), binding.variant.clone());
        let done = exercised.get(&key);
        let outcomes_done = done.map(|d| &d.0);
        let formats_done = done.map(|d| &d.1);
        let who = binding_label(binding);
        for (okey, _) in binding.outcomes.as_deref().unwrap_or_default() {
            let kind = okey.0;
            if universal.contains(&kind.token())
                || outcomes_done.is_some_and(|s| s.contains(&kind))
                || wire_surface
                    .outcome_exception(&binding.sm_operation, variant, kind)
                    .is_some()
            {
                continue;
            }
            push(
                findings,
                CheckId::SurfaceCoverage,
                &who,
                format!(
                    "outcome `{}` is declared by the binding but no case exercises it and no \
                     wire_surface.yaml branch exception covers it",
                    kind.token()
                ),
            );
        }
        for format in &binding.formats {
            if formats_done.is_some_and(|s| s.contains(format))
                || wire_surface
                    .format_exception(&binding.sm_operation, variant, *format)
                    .is_some()
            {
                continue;
            }
            push(
                findings,
                CheckId::SurfaceCoverage,
                &who,
                format!(
                    "format `{}` is declared by the binding but no case exercises it and no \
                     wire_surface.yaml branch exception covers it",
                    format_token(*format)
                ),
            );
        }
    }
}

/// Axis 3 — every cross-cutting wire-surface element resolves (its `covered_by`
/// cases exist; its exception cites a real register entry), plus register
/// self-consistency (element shapes, stale branch exceptions).
fn check_wire_surface_elements(
    set: &ArtifactSet,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    if let Err(messages) = wire_surface.check_invariants() {
        for message in messages {
            push(
                findings,
                CheckId::SurfaceCoverage,
                "vocab/wire_surface.yaml",
                message,
            );
        }
    }
    let case_ids: BTreeSet<&str> = set.cases.iter().map(|(_, c)| c.id.as_str()).collect();
    for element in &wire_surface.elements {
        for cid in &element.covered_by {
            if !case_ids.contains(cid.as_str()) {
                push(
                    findings,
                    CheckId::SurfaceCoverage,
                    &element.id,
                    format!("covered_by case {cid} does not exist"),
                );
            }
        }
        if let Some(ex) = &element.exception
            && let Some(reg) = &ex.register
            && set
                .register
                .as_ref()
                .is_none_or(|(_, r)| r.get(reg).is_none())
        {
            push(
                findings,
                CheckId::SurfaceCoverage,
                &element.id,
                format!("exception cites {reg} which is not in the ambiguity register"),
            );
        }
    }
    // Ratchet: a branch exception matching no realized binding is stale.
    for branch in &wire_surface.branches {
        let matches = set.bindings.iter().any(|(_, b)| {
            !b.is_unrealized()
                && b.sm_operation == branch.binding
                && (branch.variant.is_none() || b.variant.as_deref() == branch.variant.as_deref())
        });
        if !matches {
            push(
                findings,
                CheckId::SurfaceCoverage,
                &branch.binding.to_string(),
                "wire_surface.yaml branch exception matches no realized binding (stale)".to_owned(),
            );
        }
    }
}

/// Render the deterministic coverage report (`docs/conformance/coverage-report.md`):
/// per-interface SM-operation status, per-binding outcome/format coverage, and
/// the cross-cutting wire-surface table. Stable ordering, no timestamps — the
/// same inputs always render byte-identical output.
///
/// Axis 1 (the per-interface section) renders only when `spec_root` is
/// supplied (it reads the vendored SM tree).
#[must_use]
#[allow(clippy::too_many_lines)] // one deterministic report-rendering seam
pub fn render_coverage_report(set: &ArtifactSet, spec_root: Option<&Path>) -> String {
    use std::fmt::Write;

    let empty = WireSurface::default();
    let wire_surface = set.wire_surface.as_ref().map_or(&empty, |(_, w)| w);

    let mut out = String::new();
    out.push_str(
        "# CNF wire-surface coverage report\n\n\
         Generated by `cnf-runner validate --specs …` (the `surface-coverage` gate, issue #271). \
         Deterministic — regenerated in place, never hand-edited. The wire surface is enumerated \
         from the RELEASED spec components (the SM platform interfaces + the ITS-REST docs text), \
         never the vendored OAS. Every un-exercised behaviour is either a covering case or a \
         cited `vocab/wire_surface.yaml` exception; silence is not coverage.\n\n",
    );

    // ── Axis 1 ──
    if let Some(spec_root) = spec_root {
        out.push_str("## Axis 1 — SM-operation coverage (per platform interface)\n\n");
        out.push_str("| Interface | Operations | Realized | Unrealized | Off-wire / exception |\n");
        out.push_str("|---|--:|--:|--:|--:|\n");
        for interface in PLATFORM_INTERFACES {
            let Ok(ops) = sm_interface_operations(spec_root, interface) else {
                let _ = writeln!(out, "| {interface} | (no vendored SM class export) | | | |");
                continue;
            };
            let (mut realized, mut unrealized, mut excepted) = (0_usize, 0_usize, 0_usize);
            for name in &ops {
                let Ok(op) = SmOperationRef::parse(&format!("{interface}.{name}")) else {
                    continue;
                };
                let binding = set.bindings.iter().find(|(_, b)| b.sm_operation == op);
                match binding {
                    Some((_, b)) if b.is_unrealized() => unrealized += 1,
                    Some(_) => realized += 1,
                    None if wire_surface.sm_exception(&op).is_some() => excepted += 1,
                    None => {}
                }
            }
            let _ = writeln!(
                out,
                "| {interface} | {} | {realized} | {unrealized} | {excepted} |",
                ops.len()
            );
        }
        out.push('\n');
    }

    // ── Axis 2 ──
    out.push_str("## Axis 2 — per-binding outcome/format coverage\n\n");
    out.push_str("| Binding | Outcomes covered | Formats covered |\n");
    out.push_str("|---|---|---|\n");
    let exercised = exercised_branches(set);
    let universal = universal_outcome_tokens(set);
    let mut realized: Vec<&OperationBinding> = set
        .bindings
        .iter()
        .map(|(_, b)| b)
        .filter(|b| !b.is_unrealized())
        .collect();
    realized.sort_by_key(|b| binding_label(b));
    for binding in realized {
        let variant = binding.variant.as_deref();
        let key = (binding.sm_operation.clone(), binding.variant.clone());
        let done = exercised.get(&key);
        let (mut ocov, mut oexc, mut ogap) = (0_usize, 0_usize, 0_usize);
        for (okey, _) in binding.outcomes.as_deref().unwrap_or_default() {
            let kind = okey.0;
            if universal.contains(&kind.token()) {
                continue;
            }
            if done.is_some_and(|d| d.0.contains(&kind)) {
                ocov += 1;
            } else if wire_surface
                .outcome_exception(&binding.sm_operation, variant, kind)
                .is_some()
            {
                oexc += 1;
            } else {
                ogap += 1;
            }
        }
        let (mut fcov, mut fexc, mut fgap) = (0_usize, 0_usize, 0_usize);
        for format in &binding.formats {
            if done.is_some_and(|d| d.1.contains(format)) {
                fcov += 1;
            } else if wire_surface
                .format_exception(&binding.sm_operation, variant, *format)
                .is_some()
            {
                fexc += 1;
            } else {
                fgap += 1;
            }
        }
        let _ = writeln!(
            out,
            "| `{}` | {ocov} exercised / {oexc} excepted / {ogap} gap | {fcov} exercised / {fexc} excepted / {fgap} gap |",
            binding_label(binding)
        );
    }
    out.push('\n');

    // ── Axis 3 ──
    out.push_str("## Axis 3 — cross-cutting wire-surface behaviours\n\n");
    out.push_str("| Element | Coverage |\n|---|---|\n");
    for element in &wire_surface.elements {
        let coverage = if let Some(ex) = &element.exception {
            match &ex.register {
                Some(reg) => format!("exception: {} ({reg})", ex.reason.token()),
                None => format!("exception: {}", ex.reason.token()),
            }
        } else {
            format!("{} case(s)", element.covered_by.len())
        };
        let _ = writeln!(out, "| `{}` | {coverage} |", element.id);
    }
    out.push('\n');
    out
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)] // test assertions/fixtures
mod surface_tests {
    use super::*;

    /// The Axis-1 enumeration source is the vendored SM class tree, never the
    /// OAS (exit criterion 5, issue #271; owner ruling 2026-07-24). The path
    /// [`sm_class_file`] builds is under `SM/docs/UML/classes/` and names no
    /// OAS artifact.
    #[test]
    fn sm_operation_source_is_the_sm_tree_never_the_oas() {
        let file = sm_class_file(Path::new("/root"), "I_EHR_SERVICE");
        let text = file.to_string_lossy();
        assert!(
            text.ends_with("SM/docs/UML/classes/i_ehr_service.adoc"),
            "{text}"
        );
        assert!(!text.contains("oas") && !text.contains("rest-oas") && !text.contains("openapi"));
        // Every pinned interface is an SM `I_`-prefixed interface name.
        assert!(PLATFORM_INTERFACES.iter().all(|i| i.starts_with("I_")));
    }

    #[test]
    fn sm_interface_operations_parses_service_ops_only() {
        // A minimal SM class export: operation rows (`|*name* (`), a
        // sub-interface accessor (`|*i_ehr* (`), and non-operation header
        // cells (`h|*…*`, an uppercase interface cell) that must be ignored.
        let adoc = "\
|===\n\
h|*Interface*\n\
2+^h|*I_FIXTURE*\n\
h|*Functions*\n\
^h|*Signature*\n\
h|*1..1*\n\
|*create_thing* ( +\n\
    x: STRING +\n\
): THING\n\
h|*1..1*\n\
|*get_thing* ( +\n\
): THING\n\
h|*1..1*\n\
|*i_ehr* ( +\n\
): I_EHR\n\
|===\n";
        let dir = assert_fs::TempDir::new().unwrap();
        let classes = dir.path().join("SM/docs/UML/classes");
        std::fs::create_dir_all(&classes).unwrap();
        std::fs::write(classes.join("i_fixture.adoc"), adoc).unwrap();

        let ops = sm_interface_operations(dir.path(), "I_FIXTURE").unwrap();
        assert_eq!(ops, vec!["create_thing".to_owned(), "get_thing".to_owned()]);
        // A missing class export is an error, not an empty list.
        assert!(sm_interface_operations(dir.path(), "I_ABSENT").is_err());
    }

    fn build_set(with_exception: bool) -> ArtifactSet {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr" },
            "formats": ["canonical-json"],
            "outcomes": { "created": { "status": 201 }, "already_exists": { "status": 409 } }
        }))
        .unwrap();
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "T-1", "kind": "functional", "component": "EHR",
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "test_purpose": "t", "description": "d", "spec_refs": ["SM i_ehr_service.adoc"],
            "capabilities": [],
            "flow": [ { "step": 1, "call": "create_ehr", "expect": "created" } ]
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings.push((PathBuf::from("b.yaml"), binding));
        set.cases.push((PathBuf::from("c.yaml"), case));
        if with_exception {
            let wire: WireSurface = serde_json::from_value(serde_json::json!({
                "branches": [ {
                    "binding": "I_EHR_SERVICE.create_ehr", "outcome": "already_exists",
                    "reason": "coverage_gap",
                    "source": "ITS-REST Requests_and_responses.md §HTTP status codes"
                } ]
            }))
            .unwrap();
            set.wire_surface = Some((PathBuf::from("wire_surface.yaml"), wire));
        }
        set
    }

    #[test]
    fn axis2_flags_unexercised_outcome_then_exception_suppresses_it() {
        // `created` is exercised by the case; `already_exists` is declared but
        // never exercised → a surface-coverage finding.
        let set = build_set(false);
        let empty = WireSurface::default();
        let mut findings = Vec::new();
        check_binding_branch_coverage(&set, &empty, &mut findings);
        assert!(
            findings.iter().any(
                |f| f.check == CheckId::SurfaceCoverage && f.message.contains("already_exists")
            ),
            "expected an already_exists gap, got: {findings:?}"
        );

        // A branch exception for the same outcome suppresses the finding.
        let set = build_set(true);
        let wire = set.wire_surface.as_ref().map(|(_, w)| w).unwrap();
        let mut findings = Vec::new();
        check_binding_branch_coverage(&set, wire, &mut findings);
        assert!(
            findings.is_empty(),
            "the branch exception should suppress the gap, got: {findings:?}"
        );
    }

    #[test]
    fn axis3_element_covered_by_must_resolve() {
        let set = build_set(false);
        let wire: WireSurface = serde_json::from_value(serde_json::json!({
            "elements": [ {
                "id": "x", "description": "d",
                "source": "ITS-REST Requests_and_responses.md §Location",
                "covered_by": ["NO-SUCH-CASE"]
            } ]
        }))
        .unwrap();
        let mut findings = Vec::new();
        check_wire_surface_elements(&set, &wire, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SurfaceCoverage && f.message.contains("NO-SUCH-CASE")),
            "expected an unresolved covered_by finding, got: {findings:?}"
        );
    }
}
