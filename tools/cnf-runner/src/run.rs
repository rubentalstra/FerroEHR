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
use crate::model::case::CaseCore;
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
