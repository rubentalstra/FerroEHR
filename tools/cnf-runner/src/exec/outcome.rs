//! Observed-outcome classification — interpreter law (c): transport and
//! connection faults, timeouts, and responses no binding outcome maps →
//! `errored` (ISO/IEC 9646 *inconclusive*, never a conformance finding); a
//! *mapped but unexpected* outcome → `failed`.

use crate::model::binding::OperationBinding;
use crate::model::vocab_files::SelectorsVocab;
use crate::vocab::OutcomeKind;

/// What a step observation resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    /// The wire matched a mapped outcome kind.
    Kind(OutcomeKind),
    /// No mapped outcome (and no universal outcome) matches the response —
    /// inconclusive, not a conformance finding.
    Unmapped { status: u16 },
    /// Transport/connection fault or timeout — inconclusive.
    Transport(String),
}

/// Classify an HTTP status against the binding's outcome map plus the
/// universal (route-table-wide) outcomes from the selectors vocabulary.
#[must_use]
pub fn classify_status(
    binding: &OperationBinding,
    selectors: Option<&SelectorsVocab>,
    status: u16,
) -> Observation {
    // Binding-mapped outcomes first (an operation-specific mapping wins over
    // a universal one for the same status).
    for kind in OutcomeKind::ALL {
        if let Some(expectation) = binding.outcome(*kind)
            && expectation.status.value() == status
        {
            return Observation::Kind(*kind);
        }
    }
    if let Some(universal) = selectors.and_then(|s| s.universal_outcomes.as_deref()) {
        for (token, mapping) in universal {
            if mapping.status == status
                && let Some(kind) = OutcomeKind::from_token(token)
            {
                return Observation::Kind(kind);
            }
        }
    }
    Observation::Unmapped { status }
}

/// The row verdict a step observation produces against its expectation —
/// laws (b) and (c) combined: a mismatch fails the row (and the caller
/// aborts its remaining steps); an unmapped/transport observation errors it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepJudgement {
    /// Expectation met; the row continues.
    Continue,
    /// Mapped but unexpected — the row FAILS and remaining steps abort.
    Failed {
        expected: OutcomeKind,
        observed: OutcomeKind,
    },
    /// Inconclusive — the row ERRORS and remaining steps abort.
    Errored(String),
}

/// Judge one observation against the expected kind.
#[must_use]
pub fn judge(expected: OutcomeKind, observation: &Observation) -> StepJudgement {
    match observation {
        Observation::Kind(observed) if *observed == expected => StepJudgement::Continue,
        Observation::Kind(observed) => StepJudgement::Failed {
            expected,
            observed: *observed,
        },
        Observation::Unmapped { status } => StepJudgement::Errored(format!(
            "status {status} maps to no outcome of the operation's binding (inconclusive)"
        )),
        Observation::Transport(fault) => {
            StepJudgement::Errored(format!("transport fault: {fault} (inconclusive)"))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    fn binding() -> OperationBinding {
        serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr" },
            "outcomes": {
                "created": { "status": 201 },
                "already_exists": { "status": 409 }
            }
        }))
        .unwrap()
    }

    fn selectors() -> SelectorsVocab {
        serde_saphyr::from_str(
            "body_selectors: [prefer_conditional, error_loose, result_set_body, negotiated, present, absent]\nheader_matchers: [\"present\", \"present?\", \"absent\", \"negotiated\", \"latest-version-uid\", \"pattern:<regex>\", \"<literal>\"]\nignore_sets:\n  server_assigned: { per_binding: true, source: s }\n  ctx_defaults: { paths: [context/start_time], source: s }\nuniversal_outcomes:\n  unauthenticated: { status: 401, source: s }\n  forbidden: { status: 403, source: s }\n",
        )
        .unwrap()
    }

    #[test]
    fn law_c_classification() {
        let b = binding();
        let s = selectors();
        assert_eq!(
            classify_status(&b, Some(&s), 201),
            Observation::Kind(OutcomeKind::Created)
        );
        assert_eq!(
            classify_status(&b, Some(&s), 409),
            Observation::Kind(OutcomeKind::AlreadyExists)
        );
        // universal outcome reachable on any operation
        assert_eq!(
            classify_status(&b, Some(&s), 401),
            Observation::Kind(OutcomeKind::Unauthenticated)
        );
        // unmapped status is inconclusive, never a failure
        assert_eq!(
            classify_status(&b, Some(&s), 500),
            Observation::Unmapped { status: 500 }
        );
    }

    #[test]
    fn law_b_and_c_judgement() {
        assert_eq!(
            judge(
                OutcomeKind::Created,
                &Observation::Kind(OutcomeKind::Created)
            ),
            StepJudgement::Continue
        );
        assert!(matches!(
            judge(
                OutcomeKind::Created,
                &Observation::Kind(OutcomeKind::AlreadyExists)
            ),
            StepJudgement::Failed { .. }
        ));
        assert!(matches!(
            judge(OutcomeKind::Created, &Observation::Unmapped { status: 500 }),
            StepJudgement::Errored(_)
        ));
        assert!(matches!(
            judge(
                OutcomeKind::Created,
                &Observation::Transport("timeout".into())
            ),
            StepJudgement::Errored(_)
        ));
    }
}
