//! The data-driven executor: the flow interpreter under the five step-3
//! interpreter laws —
//!
//! **(a)** `reset_per_row` re-establishes the whole `requires` block around
//! every row; **(b)** a step whose observed outcome differs from `expect`
//! fails the row and aborts its remaining steps and row postconditions;
//! **(c)** transport faults and unmapped responses → `errored`
//! (inconclusive), a mapped-but-unexpected outcome → `failed`; **(d)**
//! `${time:*}` resolution is fixed (±1 ms, midpoint — [`state::VarStore`]);
//! **(e)** aggregate assertions collect across rows and evaluate once after
//! the last row.
//!
//! The row engine is transport-agnostic: a [`StepDriver`] performs one step
//! call and reports what it observed, so the same laws run against the live
//! reqwest driver and the verification-pack transcript player.

pub mod assertions;
pub mod content_synth;
pub mod driver;
pub mod opt_synth;
pub mod outcome;
pub mod player;
pub mod recipes;
pub mod resolve;
pub mod resultset;
pub mod state;

use crate::ids::CaseId;
use crate::model::case::{CaseCore, FlowStep};
use crate::vocab::{FormatName, OutcomeKind};

use outcome::{Observation, StepJudgement};
use state::VarStore;

/// The per-row outcome record (ISO/IEC 9646 mapping: passed→pass,
/// failed→fail, errored→inconclusive; not-applicable/skipped are selection
/// records, each with a mandatory citation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowOutcome {
    Passed,
    Failed { step: u32, reason: String },
    Errored { step: u32, reason: String },
    NotApplicable { citation: String },
    Skipped { citation: String },
}

/// The record one case×format execution produces — the direct input to the
/// party-artifact emission (`results.json` outcomes[]).
#[derive(Debug, Clone)]
pub struct CaseRecord {
    pub case: CaseId,
    pub format: Option<FormatName>,
    pub rows: Vec<RowOutcome>,
    /// rows driven / rows selected (the printed coverage bound).
    pub rows_driven: usize,
    pub rows_total: usize,
}

impl CaseRecord {
    /// A case passes iff every driven row passed (verdict rollup input).
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.rows.is_empty()
            && self
                .rows
                .iter()
                .all(|r| matches!(r, RowOutcome::Passed | RowOutcome::NotApplicable { .. }))
    }
}

/// What a driver observed for one step, plus the captures it bound.
#[derive(Debug)]
pub struct StepObservation {
    pub observation: Observation,
    /// Post-step assertion failures (empty when all held). Only meaningful
    /// when the observation matched the expectation.
    pub assertion_failures: Vec<String>,
}

impl StepObservation {
    /// A transport-class observation with no assertion results — the shape
    /// every driver-internal failure takes (see `StepDriver::perform`).
    #[must_use]
    pub fn transport(message: String) -> Self {
        Self {
            observation: Observation::Transport(message),
            assertion_failures: Vec::new(),
        }
    }
}

/// One step execution seam: the live HTTP driver and the transcript player
/// both implement this. The driver resolves the step's operation against
/// the bindings, performs the call on the selected instance, classifies the
/// response (law c), binds the step's captures into `vars`, and evaluates
/// the step's post-step assertions.
pub trait StepDriver {
    /// Perform one flow step for the given expected kind.
    ///
    /// # Errors
    /// Driver-internal failures surface as `Observation::Transport`, never
    /// as `Err` — the `Err` channel is reserved for interpreter defects
    /// (unresolvable artifacts), which abort the run.
    fn perform(
        &mut self,
        case: &CaseCore,
        step: &FlowStep,
        expected: OutcomeKind,
        row: usize,
        vars: &mut VarStore,
    ) -> Result<StepObservation, String>;

    /// Re-establish the case's `requires` block (law a). Called before the
    /// first row and again before every row under `reset_per_row`.
    ///
    /// # Errors
    /// As [`StepDriver::perform`]: interpreter defects only.
    fn provision(&mut self, case: &CaseCore, row: usize, vars: &mut VarStore)
    -> Result<(), String>;

    /// Evaluate the case's per-row postconditions (non-aggregate).
    ///
    /// # Errors
    /// Interpreter defects only; assertion failures return in the list.
    fn postconditions(
        &mut self,
        case: &CaseCore,
        row: usize,
        vars: &mut VarStore,
    ) -> Result<Vec<String>, String>;

    /// Evaluate the aggregate assertions once after the last row (law e),
    /// over the values collected across all rows.
    ///
    /// # Errors
    /// Interpreter defects only.
    fn aggregates(&mut self, case: &CaseCore, all_rows: &[VarStore])
    -> Result<Vec<String>, String>;
}

/// Resolve the expected kind for a step in a given row (the per-fixture
/// `${fixture.expected}` override).
fn expected_kind(case: &CaseCore, step: &FlowStep, row: usize) -> Option<OutcomeKind> {
    // The reserved matrix `expected` column is the normative per-row
    // override; rows without it inherit the flow's expectation.
    if let Some(matrix) = case.parameters.as_ref().and_then(|p| p.matrix.as_ref())
        && let Some(col) = matrix.columns.iter().position(|c| c == "expected")
        && let Some(crate::model::case::MatrixCell::Literal(serde_json::Value::String(s))) =
            matrix.rows.get(row).and_then(|cells| cells.get(col))
        && let Some(kind) = OutcomeKind::from_token(s)
    {
        return Some(kind);
    }
    match step.expect {
        crate::model::case::ExpectSpec::Kind(kind) => Some(kind),
        crate::model::case::ExpectSpec::FixtureExpected => case
            .parameters
            .as_ref()
            .and_then(|p| p.fixture_set.as_ref())
            .and_then(|fixtures| fixtures.get(row))
            .map(|f| f.expected),
    }
}

/// Row count of a case (matrix rows, fixture rows, or one).
#[must_use]
pub fn row_count(case: &CaseCore) -> usize {
    case.parameters
        .as_ref()
        .map_or(1, |p| {
            p.matrix
                .as_ref()
                .map(|m| m.rows.len())
                .or_else(|| p.fixture_set.as_ref().map(Vec::len))
                .unwrap_or(1)
        })
        .max(1)
}

/// Run one case×format through the interpreter laws.
///
/// # Errors
/// Interpreter defects only (unresolvable artifacts); conformance outcomes
/// — incl. errored rows — return in the record.
pub fn run_case<D: StepDriver>(
    case: &CaseCore,
    format: Option<FormatName>,
    driver: &mut D,
) -> Result<CaseRecord, String> {
    let total = row_count(case);
    let reset_per_row = case
        .parameters
        .as_ref()
        .is_none_or(|p| matches!(p.iteration, crate::vocab::Iteration::ResetPerRow));

    let mut rows = Vec::with_capacity(total);
    let mut row_states: Vec<VarStore> = Vec::with_capacity(total);
    let mut vars = VarStore::default();

    for row in 0..total {
        if reset_per_row || row == 0 {
            vars = VarStore::default();
            driver.provision(case, row, &mut vars)?; // law a
        }

        let mut row_outcome = RowOutcome::Passed;
        'steps: for step in &case.flow {
            let Some(expected) = expected_kind(case, step, row) else {
                row_outcome = RowOutcome::Errored {
                    step: step.step,
                    reason: "no expected kind resolvable for this row".to_owned(),
                };
                break 'steps;
            };
            let observed = driver.perform(case, step, expected, row, &mut vars)?;
            match outcome::judge(expected, &observed.observation) {
                StepJudgement::Continue => {
                    if let Some(failure) = observed.assertion_failures.first() {
                        row_outcome = RowOutcome::Failed {
                            step: step.step,
                            reason: failure.clone(),
                        };
                        break 'steps; // law b
                    }
                }
                StepJudgement::Failed { expected, observed } => {
                    row_outcome = RowOutcome::Failed {
                        step: step.step,
                        reason: format!(
                            "expected `{}`, observed `{}`",
                            expected.token(),
                            observed.token()
                        ),
                    };
                    break 'steps; // law b: abort remaining steps
                }
                StepJudgement::Errored(reason) => {
                    row_outcome = RowOutcome::Errored {
                        step: step.step,
                        reason,
                    };
                    break 'steps;
                }
            }
        }

        // Law b: row postconditions run only when every step held.
        if matches!(row_outcome, RowOutcome::Passed)
            && let Some(failure) = driver
                .postconditions(case, row, &mut vars)?
                .into_iter()
                .next()
        {
            row_outcome = RowOutcome::Failed {
                step: 0,
                reason: failure,
            };
        }

        row_states.push(vars.clone());
        rows.push(row_outcome);
    }

    // Law e: aggregates once after the last row.
    if rows.iter().all(|r| matches!(r, RowOutcome::Passed))
        && let Some(failure) = driver.aggregates(case, &row_states)?.into_iter().next()
        && let Some(last) = rows.last_mut()
    {
        {
            *last = RowOutcome::Failed {
                step: 0,
                reason: format!("aggregate: {failure}"),
            };
        }
    }

    Ok(CaseRecord {
        case: case.id.clone(),
        format,
        rows_driven: rows.len(),
        rows_total: total,
        rows,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test drivers/fixtures
mod tests {
    use super::*;
    use crate::vocab::OutcomeKind;

    /// A scripted driver: each (row, step) yields a fixed observation.
    struct Scripted {
        provisioned: usize,
        script: Vec<Observation>,
        aggregate_failure: Option<String>,
        cursor: usize,
    }

    impl StepDriver for Scripted {
        fn perform(
            &mut self,
            _case: &CaseCore,
            _step: &FlowStep,
            _expected: OutcomeKind,
            _row: usize,
            _vars: &mut VarStore,
        ) -> Result<StepObservation, String> {
            let observation = self.script.get(self.cursor).cloned().unwrap();
            self.cursor += 1;
            Ok(StepObservation {
                observation,
                assertion_failures: Vec::new(),
            })
        }
        fn provision(&mut self, _c: &CaseCore, _r: usize, _v: &mut VarStore) -> Result<(), String> {
            self.provisioned += 1;
            Ok(())
        }
        fn postconditions(
            &mut self,
            _c: &CaseCore,
            _r: usize,
            _v: &mut VarStore,
        ) -> Result<Vec<String>, String> {
            Ok(Vec::new())
        }
        fn aggregates(&mut self, _c: &CaseCore, _rows: &[VarStore]) -> Result<Vec<String>, String> {
            Ok(self.aggregate_failure.clone().into_iter().collect())
        }
    }

    fn two_row_case() -> CaseCore {
        serde_json::from_value(serde_json::json!({
            "id": "I_EHR_SERVICE.create_ehr-law_test",
            "kind": "functional",
            "component": "EHR",
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "capabilities": ["EhrOperations"],
            "profiles": ["CORE"],
            "test_purpose": "laws",
            "description": "laws",
            "spec_refs": ["CNF platform_test_schedule master06 §create_ehr data sets"],
            "requires": { "server": "empty" },
            "parameters": { "iteration": "reset_per_row",
                             "matrix": { "columns": ["ehr_id"], "rows": [["absent"], ["provided"]] } },
            "flow": [
                { "step": 1, "call": "create_ehr", "expect": "created" },
                { "step": 2, "call": "create_ehr", "expect": "already_exists" }
            ]
        }))
        .unwrap()
    }

    #[test]
    fn law_a_reprovisions_per_row_and_law_b_aborts() {
        let case = two_row_case();
        // Row 0: step 1 ok, step 2 observes created (mapped, unexpected) -> FAILED, step 2 aborts row.
        // Row 1: both steps ok -> PASSED.
        let mut driver = Scripted {
            provisioned: 0,
            script: vec![
                Observation::Kind(OutcomeKind::Created),
                Observation::Kind(OutcomeKind::Created), // mismatch
                Observation::Kind(OutcomeKind::Created),
                Observation::Kind(OutcomeKind::AlreadyExists),
            ],
            aggregate_failure: None,
            cursor: 0,
        };
        let record = run_case(&case, None, &mut driver).unwrap();
        assert_eq!(driver.provisioned, 2); // law a: one provision per row
        assert!(matches!(record.rows[0], RowOutcome::Failed { step: 2, .. }));
        assert!(matches!(record.rows[1], RowOutcome::Passed));
        assert!(!record.passed());
    }

    #[test]
    fn law_c_errored_is_not_failed_and_law_e_runs_last() {
        let case = two_row_case();
        let mut driver = Scripted {
            provisioned: 0,
            script: vec![
                Observation::Kind(OutcomeKind::Created),
                Observation::Kind(OutcomeKind::AlreadyExists),
                Observation::Kind(OutcomeKind::Created),
                Observation::Kind(OutcomeKind::AlreadyExists),
            ],
            aggregate_failure: Some("ehr_id values are not pairwise distinct".to_owned()),
            cursor: 0,
        };
        let record = run_case(&case, None, &mut driver).unwrap();
        // aggregate failure lands on the LAST row (law e: evaluated once, after all rows)
        assert!(matches!(record.rows[1], RowOutcome::Failed { step: 0, .. }));

        let mut errored = Scripted {
            provisioned: 0,
            script: vec![
                Observation::Transport("connection refused".into()),
                Observation::Kind(OutcomeKind::Created),
                Observation::Kind(OutcomeKind::AlreadyExists),
            ],
            aggregate_failure: None,
            cursor: 0,
        };
        let record = run_case(&case, None, &mut errored).unwrap();
        assert!(matches!(
            record.rows[0],
            RowOutcome::Errored { step: 1, .. }
        ));
        assert!(matches!(record.rows[1], RowOutcome::Passed));
    }
}
