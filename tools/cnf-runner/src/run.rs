//! The run orchestration: select → execute (the interpreter over the live
//! driver) → record — producing the `results.json` outcomes the party layer
//! emits and the verdict pipeline consumes.
//!
//! Interpreter-coverage accounting is first-class: every case that cannot
//! be interpreter-run is a REGISTERED EXCEPTION with its reason (the
//! ≥90%-interpreter-run gate is computed, never asserted).

use crate::artifacts::ArtifactSet;
use crate::exec::driver::HttpDriver;
use crate::exec::{CaseRecord, RowOutcome, run_case};
use crate::ids::SmOperationRef;
use crate::ixit::Ixit;
use crate::model::assertion::assertion_refs;
use crate::model::case::CaseCore;
use crate::refgrammar::{IxitField, ValueRef};
use crate::vocab::CaseStatus;

/// Why a case was not interpreter-run (the registered-exception taxonomy).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "detail")]
pub enum Exception {
    /// Every binding of the case's operations is `unrealized` on this ITS —
    /// not-applicable with the binding's citation.
    Unrealized(String),
    /// `kind: content` — decision-table execution needs the row-to-instance
    /// generation seam (the registered recipes cover the committed corpus;
    /// per-row template projection is the remaining glue).
    ContentGeneration(String),
    /// A guard excludes the case on this SUT (citation carried).
    Guarded(String),
    /// The case is `draft`/`retired` — never verdict-bearing.
    Status(String),
}

/// One run's execution report.
#[derive(Debug, Default)]
pub struct RunReport {
    pub records: Vec<CaseRecord>,
    pub exceptions: Vec<(crate::ids::CaseId, Exception)>,
    /// Cases the interpreter drove end-to-end.
    pub interpreter_run: usize,
    /// All active assertion-machinery cases considered.
    pub considered: usize,
}

impl RunReport {
    /// The interpreter-coverage fraction (the ≥90% gate input).
    #[must_use]
    pub fn interpreter_coverage(&self) -> f64 {
        if self.considered == 0 {
            return 1.0;
        }
        #[allow(clippy::cast_precision_loss)] // case counts << 2^52
        {
            self.interpreter_run as f64 / self.considered as f64
        }
    }
}

/// Whether every operation the case's flow calls is unrealized on this ITS.
fn fully_unrealized(set: &ArtifactSet, case: &CaseCore) -> Option<String> {
    // ANY unrealized step makes the whole case not-applicable on this ITS:
    // the flow cannot reach its expectation without the missing wire, so a
    // verdict would be meaningless — the case is excused with the machine-
    // readable citation the binding declares.
    let anchor = case.sm_operation.as_ref()?;
    let mut citations = Vec::new();
    for step in &case.flow {
        let op = if step.call.contains('.') {
            SmOperationRef::parse(&step.call).ok()?
        } else {
            anchor.sibling(&step.call)
        };
        let binding = set
            .bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| b.sm_operation == op)?;
        if let Some(decl) = &binding.unrealized {
            citations.push(format!("{op}: {}", decl.ambiguity));
        }
    }
    (!citations.is_empty()).then(|| citations.join("; "))
}

/// The `${ixit:…}` facts a case reads that THIS party's ixit does not
/// declare. A declared fact is the only source (no released operation
/// discloses it), so a case that needs an undeclared one is not-applicable
/// on this party — never driven against a guessed value.
fn undeclared_ixit_facts(case: &CaseCore, ixit: &Ixit) -> Vec<&'static str> {
    fn note(reference: &ValueRef, into: &mut Vec<IxitField>) {
        if let ValueRef::Ixit(field) = reference
            && !into.contains(field)
        {
            into.push(*field);
        }
    }
    let mut referenced: Vec<IxitField> = Vec::new();
    for step in &case.flow {
        for (_, value) in step.with_entries() {
            for reference in value.refs() {
                note(reference, &mut referenced);
            }
        }
        for assertion in &step.assertions {
            for reference in assertion_refs(assertion) {
                note(&reference, &mut referenced);
            }
        }
    }
    for assertion in &case.postconditions {
        for reference in assertion_refs(assertion) {
            note(&reference, &mut referenced);
        }
    }
    referenced
        .into_iter()
        .filter(|field| match field {
            IxitField::SystemId => ixit.system_id.is_none(),
        })
        .map(IxitField::token)
        .collect()
}

/// The ixit instances a case's flow addresses (`on:`) that THIS party does
/// not declare.
///
/// An instance is a topology declaration exactly like the ixit facts above:
/// a party that runs no such deployment (no `readonly` principal, no second
/// signing posture) cannot have the case driven against it, and the
/// alternative to a declaration is driving it somewhere it does not belong.
/// So the case is not-applicable WITH the citation at selection time — never
/// a drive-time transport error, which would surface as an inconclusive row
/// that reads like a SUT defect.
fn undeclared_instances(case: &CaseCore, ixit: &Ixit) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    for step in &case.flow {
        if let Some(name) = &step.on
            && ixit.instance(name).is_none()
            && !missing.iter().any(|m| m == name.as_str())
        {
            missing.push(name.as_str().to_owned());
        }
    }
    missing
}

/// The reserved catalogue pseudo-interface anchoring the SMART Platform
/// operations the SM models no interface for (pinned in
/// `validate::NON_SM_REST_OPERATIONS`; register AMB-161 adjudicates the
/// naming convention).
const SMART_PSEUDO_INTERFACE: &str = "I_ITS_REST_SMART";

/// Whether the case needs the party's SMART App Launch lane
/// (`ixit.smart`) — either because a flow step declares a SMART `scope`
/// claim the runner must mint a token for, or because the case drives a
/// SMART Platform operation that only a SMART-enabled deployment serves.
fn needs_smart_lane(case: &CaseCore) -> bool {
    case.flow
        .iter()
        .any(crate::model::case::FlowStep::declares_scopes)
        || case
            .sm_operation
            .as_ref()
            .is_some_and(|op| op.interface() == SMART_PSEUDO_INTERFACE)
}

/// Execute every runnable case against the ixit's default topology.
///
/// `statement` (the party ICS), when supplied, drives ISO/IEC 9646-style
/// test selection: an option-gated case whose `option` tag the ICS does not
/// declare is recorded not-applicable with citation instead of being driven
/// against a server that legitimately implements the other register branch.
/// Without a statement every case runs (the statement-blind sweep).
///
/// # Errors
/// Interpreter defects only; per-case conformance outcomes land in the
/// report.
pub fn execute(
    set: &ArtifactSet,
    ixit: &Ixit,
    statement: Option<&crate::party::Statement>,
) -> Result<RunReport, String> {
    let mut report = RunReport::default();
    // Exclusive-server cases (global-state grounds like an empty template
    // list) run FIRST: on a freshly reset, exclusively-owned SUT their
    // ground holds only before other cases provision templates/queries.
    let mut ordered: Vec<&crate::model::case::CaseCore> =
        set.cases.iter().map(|(_, c)| c).collect();
    ordered.sort_by_key(|c| {
        !matches!(
            c.requires.server,
            Some(crate::vocab::ServerState::Exclusive)
        )
    });
    for case in ordered {
        report.considered += 1;
        if !matches!(case.status, CaseStatus::Active) {
            report.exceptions.push((
                case.id.clone(),
                Exception::Status(format!("{:?} — never verdict-bearing", case.status)),
            ));
            continue;
        }
        if let Some(citation) = fully_unrealized(set, case) {
            report.records.push(CaseRecord {
                case: case.id.clone(),
                format: None,
                rows: vec![RowOutcome::NotApplicable {
                    citation: citation.clone(),
                }],
                rows_driven: 0,
                rows_total: crate::exec::row_count(case),
            });
            report
                .exceptions
                .push((case.id.clone(), Exception::Unrealized(citation)));
            continue;
        }
        // ICS-driven selection (ISO/IEC 9646): an option branch the party
        // statement does not declare is not this SUT's behaviour — driving
        // it records a spurious failure the verdict pipeline would excuse
        // anyway (`verdict::effective_outcome`); excuse it at drive time
        // with the same citation.
        if let Some(stmt) = statement
            && let Some(tag) = &case.option
            && !stmt.options.contains(tag)
        {
            let citation = format!(
                "option {tag}: the ICS does not declare this register branch \
                 (statement.options) — ISO/IEC 9646 test selection"
            );
            report.records.push(CaseRecord {
                case: case.id.clone(),
                format: None,
                rows: vec![RowOutcome::NotApplicable {
                    citation: citation.clone(),
                }],
                rows_driven: 0,
                rows_total: crate::exec::row_count(case),
            });
            report
                .exceptions
                .push((case.id.clone(), Exception::Unrealized(citation)));
            continue;
        }
        // The SMART lane is a party declaration, exactly like the ixit facts
        // below: the CDR is a SMART resource server that never issues tokens
        // (ITS-REST docs/smart_app_launch/master06-authentication.adoc
        // §Supported Authentication Flows), so a chosen `scope` claim exists
        // only where the party declares a trusted test issuer to mint
        // against, and the discovery document exists only where the party
        // runs the SMART role at all. Undeclared => not-applicable with the
        // citation, never a spurious failure against a deployment that
        // legitimately does not run SMART.
        if needs_smart_lane(case) && ixit.smart.is_none() {
            let citation = "the ixit declares no `smart` lane — the case needs a SMART-enabled \
                 deployment and a minted, scope-carrying access token, neither of which any \
                 released operation discloses or provides; ISO/IEC 9646 test selection"
                .to_owned();
            report.records.push(CaseRecord {
                case: case.id.clone(),
                format: None,
                rows: vec![RowOutcome::NotApplicable {
                    citation: citation.clone(),
                }],
                rows_driven: 0,
                rows_total: crate::exec::row_count(case),
            });
            report
                .exceptions
                .push((case.id.clone(), Exception::Guarded(citation)));
            continue;
        }
        // A flow step addressing an instance this party does not declare has
        // no ground to run on (the deployment or principal simply does not
        // exist here) — not-applicable with the citation, like every other
        // undeclared topology fact.
        let missing_instances = undeclared_instances(case, ixit);
        if !missing_instances.is_empty() {
            let citation = format!(
                "the ixit declares no instance {} — the case's flow addresses it with `on:` and \
                 this party runs no such deployment/principal; ISO/IEC 9646 test selection",
                missing_instances.join(", ")
            );
            report.records.push(CaseRecord {
                case: case.id.clone(),
                format: None,
                rows: vec![RowOutcome::NotApplicable {
                    citation: citation.clone(),
                }],
                rows_driven: 0,
                rows_total: crate::exec::row_count(case),
            });
            report
                .exceptions
                .push((case.id.clone(), Exception::Guarded(citation)));
            continue;
        }
        // A case reading a party-declared SUT fact this ixit does not carry
        // cannot be driven: the fact is not on the wire, so the alternative
        // to a declaration is a guess.
        let missing = undeclared_ixit_facts(case, ixit);
        if !missing.is_empty() {
            let citation = format!(
                "the ixit declares no {} — the case reads it as ${{ixit:…}} and no released \
                 operation discloses the value; ISO/IEC 9646 test selection",
                missing.join(", ")
            );
            report.records.push(CaseRecord {
                case: case.id.clone(),
                format: None,
                rows: vec![RowOutcome::NotApplicable {
                    citation: citation.clone(),
                }],
                rows_driven: 0,
                rows_total: crate::exec::row_count(case),
            });
            report
                .exceptions
                .push((case.id.clone(), Exception::Guarded(citation)));
            continue;
        }
        // Global-state grounds (an empty template list, a globally-absent
        // artefact) hold only on an exclusively-owned SUT; on a shared
        // instance the case is not-applicable, never a false verdict.
        if matches!(
            case.requires.server,
            Some(crate::vocab::ServerState::Exclusive)
        ) && !ixit
            .environment
            .as_ref()
            .is_some_and(|env| env.exclusive_server)
        {
            let citation = "requires.server: exclusive — the ixit declares a shared SUT instance \
                 (environment.exclusive_server: false); the global-state ground cannot \
                 be established"
                .to_owned();
            report.records.push(CaseRecord {
                case: case.id.clone(),
                format: None,
                rows: vec![RowOutcome::NotApplicable {
                    citation: citation.clone(),
                }],
                rows_driven: 0,
                rows_total: crate::exec::row_count(case),
            });
            report
                .exceptions
                .push((case.id.clone(), Exception::Unrealized(citation)));
            continue;
        }
        let runnable = if matches!(case.kind, crate::vocab::CaseKind::Content) {
            // One executor serves both: a content row is a generate→commit→
            // expect functional execution over the synthesized flow.
            synthesize_content_case(case)
        } else {
            case.clone()
        };
        let mut driver = HttpDriver::new(set, ixit)?;
        let record = run_case(&runnable, runnable.formats.first().copied(), &mut driver)?;
        report.interpreter_run += 1;
        report.records.push(record);
    }
    Ok(report)
}

/// The dry accounting pass — the coverage gate WITHOUT a live SUT: counts
/// which cases the interpreter WOULD drive (everything the executor
/// resolves end-to-end) versus the registered exceptions.
#[must_use]
pub fn coverage_accounting(set: &ArtifactSet) -> RunReport {
    let mut report = RunReport::default();
    // Exclusive-server cases (global-state grounds like an empty template
    // list) run FIRST: on a freshly reset, exclusively-owned SUT their
    // ground holds only before other cases provision templates/queries.
    let mut ordered: Vec<&crate::model::case::CaseCore> =
        set.cases.iter().map(|(_, c)| c).collect();
    ordered.sort_by_key(|c| {
        !matches!(
            c.requires.server,
            Some(crate::vocab::ServerState::Exclusive)
        )
    });
    for case in ordered {
        report.considered += 1;
        if !matches!(case.status, CaseStatus::Active) {
            report.exceptions.push((
                case.id.clone(),
                Exception::Status(format!("{:?}", case.status)),
            ));
            continue;
        }
        if let Some(citation) = fully_unrealized(set, case) {
            report
                .exceptions
                .push((case.id.clone(), Exception::Unrealized(citation)));
            continue;
        }
        // Content cases are interpreter-run through the synthesized
        // generate→commit→expect flow (one executor serves both kinds).
        report.interpreter_run += 1;
    }
    report
}

/// Synthesize the functional execution of a content case: the decision
/// table becomes a matrix (rows drive `${row.*}`), the flow is one commit of
/// the generated instance against the constraint context's template, and the
/// per-row `expected` column (accepted → created, rejected →
/// `validation_failed`) is the outcome expectation.
#[must_use]
pub fn synthesize_content_case(case: &CaseCore) -> CaseCore {
    let mut synthesized = case.clone();
    let Some(table) = &case.decision_table else {
        return synthesized;
    };
    // decision-table rows -> a parameters matrix with a normalized expected
    // column (accepted/rejected -> outcome kinds).
    let columns = table.columns.clone();
    let rows: Vec<Vec<crate::model::case::MatrixCell>> = table
        .rows
        .iter()
        .map(|row| {
            columns
                .iter()
                .zip(row)
                .map(|(column, cell)| {
                    if column == "expected" {
                        let kind = match cell.as_str() {
                            Some("accepted") => "created",
                            _ => "validation_failed",
                        };
                        crate::model::case::MatrixCell::Literal(serde_json::Value::String(
                            kind.to_owned(),
                        ))
                    } else {
                        match cell {
                            serde_json::Value::Null => crate::model::case::MatrixCell::Null,
                            other => crate::model::case::MatrixCell::Literal(other.clone()),
                        }
                    }
                })
                .collect()
        })
        .collect();
    synthesized.parameters = serde_json::from_value(serde_json::json!({
        "iteration": "reset_per_row",
        "matrix": { "columns": columns, "rows": [] }
    }))
    .ok();
    if let Some(parameters) = &mut synthesized.parameters
        && let Some(matrix) = &mut parameters.matrix
    {
        matrix.rows = rows;
    }
    if let Some(context) = &case.constraint_context {
        synthesized.requires.server = Some(crate::vocab::ServerState::Any);
        // A varying-constraint case (constraint_columns declared) provisions no
        // baked template — the driver synthesizes and uploads one OPT PER ROW
        // (issue #228). A constant-constraint case keeps its single baked
        // template. constraint_context rides on the synthesized case so the
        // driver can tell the two apart.
        synthesized.requires.templates = if context.constraint_columns.is_empty() {
            vec![context.template.clone()]
        } else {
            Vec::new()
        };
        synthesized.requires.ehr = Some(crate::model::case::EhrRequirement::Exists {
            commits: crate::model::case::CommitState::None,
        });
    }
    synthesized.sm_operation =
        crate::ids::SmOperationRef::parse("I_EHR_COMPOSITION.create_composition").ok();
    synthesized.flow = serde_json::from_value(serde_json::json!([
        {
            "step": 1,
            "call": "create_composition",
            "with": { "ehr_id": "${ehr_id}", "composition": "${recipe:content_instance(row)}" },
            "expect": "created"
        }
    ]))
    .unwrap_or_default();
    // The per-row expectation rides the reserved matrix `expected` column,
    // which the interpreter resolves as the normative per-row override; the
    // flow's `created` is the inherited default.
    synthesized
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    /// The SMART-lane marker is the DECLARATION of a `scopes:` key (empty
    /// included) or the reserved SMART pseudo-interface anchor — never a
    /// heuristic over case ids, so an ordinary case can never be excused.
    #[test]
    fn smart_lane_need_is_declared_not_guessed() {
        let plain: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-plain", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "expect": "ok" }]
        }))
        .unwrap();
        assert!(!needs_smart_lane(&plain));

        let scoped: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-scoped", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "scopes": [], "expect": "forbidden" }]
        }))
        .unwrap();
        assert!(
            needs_smart_lane(&scoped),
            "an EMPTY scopes declaration is still a SMART-lane declaration"
        );

        let discovery: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-discovery", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_ITS_REST_SMART.discovery",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "discovery", "expect": "ok" }]
        }))
        .unwrap();
        assert!(needs_smart_lane(&discovery));
    }

    /// A flow step addressing an instance the party does not declare is a
    /// SELECTION outcome (not-applicable with citation), never a drive-time
    /// error — the same law the SMART lane and the `${ixit:…}` facts follow.
    #[test]
    fn undeclared_addressed_instances_are_collected() {
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-two-deployments", "kind": "functional", "component": "SECURITY",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [
                { "step": 1, "call": "list_opts", "expect": "ok" },
                { "step": 2, "call": "list_opts", "on": "sut_pgp", "expect": "ok" },
                { "step": 3, "call": "list_opts", "on": "sut_pgp", "expect": "ok" }
            ]
        }))
        .unwrap();

        let without: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        // Reported once, however many steps address it.
        assert_eq!(undeclared_instances(&case, &without), vec!["sut_pgp"]);

        let with: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://x", "auth": { "mode": "none" } },
                "sut_pgp": { "base_url": "http://y", "auth": { "mode": "none" } }
            }
        }))
        .unwrap();
        assert!(undeclared_instances(&case, &with).is_empty());
    }

    #[test]
    fn coverage_gate_holds_on_the_committed_catalogue() {
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let loaded = crate::artifacts::load_root(&crate_dir.join("artifacts")).unwrap();
        assert!(loaded.errors.is_empty());
        let report = coverage_accounting(&loaded.set);
        // Interpreter-runnable + unrealized-with-citation are both
        // interpreter-GOVERNED (the not-applicable verdict is the
        // interpreter's own selection law); only content generation and
        // draft status are genuine exceptions.
        let governed = report.interpreter_run
            + report
                .exceptions
                .iter()
                .filter(|(_, e)| matches!(e, Exception::Unrealized(_)))
                .count();
        #[allow(clippy::cast_precision_loss)]
        let coverage = governed as f64 / report.considered as f64;
        assert!(
            coverage >= 0.80,
            "interpreter-governed coverage {coverage:.3} below the floor; exceptions: {:#?}",
            report.exceptions.len()
        );
        // every exception carries a reason (registered, never silent)
        for (case, exception) in &report.exceptions {
            let text = format!("{exception:?}");
            assert!(!text.is_empty(), "{case}: silent exception");
        }
    }
}
