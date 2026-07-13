//! The clinical-event catalogue (`docs/design/benchmark/00-workload-model.md`
//! §2): each event `E1..E10` expands to a fixed sequence of CDR operations, and
//! carries a per-patient-day rate. [`schedule`](crate::model::schedule) assigns
//! arrival times and renders payloads; this module is the catalogue's data.
//!
//! PORT NOTE: no openEHR spec governs the benchmark workload; this is our own
//! pre-registered clinical-day model (register 00). E1's admission sequence is
//! extended beyond the register's literal "POST /ehr → GET /ehr → admission
//! composition" to also seed an initial vitals composition and establish the
//! patient DIRECTORY, so that every dependent op (E5 reads, E6 directory reads,
//! E7 updates) has existing state — the register's E6 assumes a directory
//! exists and E7 assumes a composition of the updated template exists.

use crate::{OpClass, TemplateKind};

/// Probability that a medication round (E3) is followed by a correction PUT.
pub const MED_CORRECTION_PROB: f64 = 0.05;

/// Probability that a care-plan review (E6) is followed by a directory update.
pub const DIR_UPDATE_PROB: f64 = 0.10;

/// One operation in an event's sequence. [`Step::op_class`] maps it to the
/// latency histogram class; [`schedule`](crate::model::schedule) turns each into
/// a [`crate::Action`] with a rendered payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `POST /ehr` with a subject-carrying `EHR_STATUS`.
    CreateEhr,
    /// `GET /ehr/{id}`.
    ReadEhr,
    /// `POST …/composition` of `template` (`large` selects the latency class).
    CreateComposition { template: TemplateKind, large: bool },
    /// `PUT …/composition/{object}` — a new version of the patient's latest
    /// composition of `template`.
    UpdateComposition { template: TemplateKind },
    /// `GET …/composition/{object}` latest.
    ReadLatest,
    /// `GET …/composition/{ovid}` — a specific version.
    ReadVersion,
    /// `POST …/contribution` — a batch of result compositions of `template`.
    Contribution { template: TemplateKind },
    /// Patient-scoped AQL.
    AqlPatient,
    /// Ward-population AQL.
    AqlWard,
    /// `GET /ehr/{id}/directory`.
    ReadDirectory,
    /// Create-or-update the patient's directory (versioned `FOLDER` write).
    UpdateDirectory,
    /// `GET …/versioned_composition/{uid}/revision_history`.
    ReadHistory,
    /// `PUT /ehr/{id}/ehr_status`.
    UpdateStatus,
    /// OPT upload of `template` (provisioning).
    UploadOpt { template: TemplateKind },
    /// `GET /definition/template/adl1.4`.
    ListTemplates,
}

impl Step {
    /// The latency histogram class this step records under.
    #[must_use]
    pub fn op_class(self) -> OpClass {
        match self {
            Step::CreateEhr => OpClass::EhrCreate,
            Step::ReadEhr => OpClass::EhrRead,
            Step::CreateComposition { large: true, .. } => OpClass::CompCreateLarge,
            Step::CreateComposition { large: false, .. } => OpClass::CompCreateSmall,
            Step::UpdateComposition { .. } => OpClass::CompUpdate,
            Step::ReadLatest => OpClass::CompReadLatest,
            Step::ReadVersion => OpClass::CompReadVersion,
            Step::Contribution { .. } => OpClass::ContributionCommit,
            Step::AqlPatient => OpClass::AqlPatient,
            Step::AqlWard => OpClass::AqlWard,
            Step::ReadDirectory => OpClass::DirRead,
            Step::UpdateDirectory => OpClass::DirUpdate,
            Step::ReadHistory => OpClass::HistoryRead,
            Step::UpdateStatus => OpClass::StatusUpdate,
            Step::UploadOpt { .. } => OpClass::OptUpload,
            Step::ListTemplates => OpClass::TplList,
        }
    }
}

/// A clinical event (register 00 §2, `E1..E10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClinicalEvent {
    /// E1 — admission (turnover only).
    Admission,
    /// E2 — shift vitals (q4h).
    ShiftVitals,
    /// E3 — medication round.
    MedicationRound,
    /// E4 — lab results arrive.
    LabResults,
    /// E5 — chart review (round).
    ChartReview,
    /// E6 — care-plan / directory.
    CarePlan,
    /// E7 — documentation correction.
    DocCorrection,
    /// E8 — ward dashboard / reporting.
    WardDashboard,
    /// E9 — discharge (turnover only).
    Discharge,
    /// E10 — provisioning (fixed per-run, outside the measured mix).
    Provisioning,
}

impl ClinicalEvent {
    /// Every event, in catalogue order.
    pub const ALL: [ClinicalEvent; 10] = [
        ClinicalEvent::Admission,
        ClinicalEvent::ShiftVitals,
        ClinicalEvent::MedicationRound,
        ClinicalEvent::LabResults,
        ClinicalEvent::ChartReview,
        ClinicalEvent::CarePlan,
        ClinicalEvent::DocCorrection,
        ClinicalEvent::WardDashboard,
        ClinicalEvent::Discharge,
        ClinicalEvent::Provisioning,
    ];

    /// The events driven per standing patient during the measured window
    /// (E2–E8; admission/discharge are turnover-only, provisioning is per-run).
    pub const MEASURED: [ClinicalEvent; 7] = [
        ClinicalEvent::ShiftVitals,
        ClinicalEvent::MedicationRound,
        ClinicalEvent::LabResults,
        ClinicalEvent::ChartReview,
        ClinicalEvent::CarePlan,
        ClinicalEvent::DocCorrection,
        ClinicalEvent::WardDashboard,
    ];

    /// The stable event key (`E1..E10`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            ClinicalEvent::Admission => "E1",
            ClinicalEvent::ShiftVitals => "E2",
            ClinicalEvent::MedicationRound => "E3",
            ClinicalEvent::LabResults => "E4",
            ClinicalEvent::ChartReview => "E5",
            ClinicalEvent::CarePlan => "E6",
            ClinicalEvent::DocCorrection => "E7",
            ClinicalEvent::WardDashboard => "E8",
            ClinicalEvent::Discharge => "E9",
            ClinicalEvent::Provisioning => "E10",
        }
    }

    /// The per-patient-day rate (register 00 §2). Admission/Discharge are the
    /// ~10% ward turnover; Provisioning is per-run (returns 0 here).
    #[must_use]
    pub fn rate_per_patient_day(self) -> f64 {
        match self {
            ClinicalEvent::Admission | ClinicalEvent::Discharge => 0.1,
            ClinicalEvent::ShiftVitals => 6.0,
            ClinicalEvent::MedicationRound => 4.0,
            ClinicalEvent::LabResults | ClinicalEvent::CarePlan => 2.0,
            ClinicalEvent::ChartReview => 8.0,
            ClinicalEvent::DocCorrection => 1.0,
            ClinicalEvent::WardDashboard => 0.5,
            ClinicalEvent::Provisioning => 0.0,
        }
    }

    /// The fixed per-patient occurrence count for the `smoke` profile (a handful
    /// per class; register 00 §3).
    #[must_use]
    pub fn smoke_count(self) -> u32 {
        match self {
            ClinicalEvent::ShiftVitals | ClinicalEvent::MedicationRound => 2,
            ClinicalEvent::LabResults
            | ClinicalEvent::ChartReview
            | ClinicalEvent::CarePlan
            | ClinicalEvent::DocCorrection
            | ClinicalEvent::WardDashboard => 1,
            ClinicalEvent::Admission | ClinicalEvent::Discharge | ClinicalEvent::Provisioning => 0,
        }
    }

    /// The base operation sequence (probabilistic follow-ups are appended by the
    /// scheduler — see [`MED_CORRECTION_PROB`] / [`DIR_UPDATE_PROB`]).
    #[must_use]
    pub fn steps(self) -> Vec<Step> {
        match self {
            ClinicalEvent::Admission => admission_steps(),
            ClinicalEvent::ShiftVitals | ClinicalEvent::MedicationRound => {
                vec![Step::CreateComposition {
                    template: TemplateKind::Vitals,
                    large: false,
                }]
            }
            ClinicalEvent::LabResults => vec![Step::Contribution {
                template: TemplateKind::Vitals,
            }],
            ClinicalEvent::ChartReview => vec![
                Step::ReadLatest,
                Step::ReadLatest,
                Step::ReadLatest,
                Step::ReadVersion,
                Step::AqlPatient,
            ],
            ClinicalEvent::CarePlan => vec![Step::ReadDirectory],
            ClinicalEvent::DocCorrection => vec![
                Step::UpdateComposition {
                    template: TemplateKind::Vitals,
                },
                Step::ReadHistory,
            ],
            ClinicalEvent::WardDashboard => vec![Step::AqlWard],
            ClinicalEvent::Discharge => discharge_steps(),
            ClinicalEvent::Provisioning => provisioning_steps(),
        }
    }
}

/// The admission / bootstrap sequence (E1): create the EHR, read it back, commit
/// the admission-assessment (nested/large) composition, seed an initial vitals
/// composition, and establish the patient directory. See the module PORT NOTE.
#[must_use]
pub fn admission_steps() -> Vec<Step> {
    vec![
        Step::CreateEhr,
        Step::ReadEhr,
        Step::CreateComposition {
            template: TemplateKind::Nested,
            large: true,
        },
        Step::CreateComposition {
            template: TemplateKind::Vitals,
            large: false,
        },
        Step::UpdateDirectory,
    ]
}

/// The discharge sequence (E9): a large discharge-summary composition then an
/// `EHR_STATUS` update.
#[must_use]
pub fn discharge_steps() -> Vec<Step> {
    vec![
        Step::CreateComposition {
            template: TemplateKind::Persistent,
            large: true,
        },
        Step::UpdateStatus,
    ]
}

/// The provisioning sequence (E10): upload every template's OPT then list.
#[must_use]
pub fn provisioning_steps() -> Vec<Step> {
    vec![
        Step::UploadOpt {
            template: TemplateKind::Vitals,
        },
        Step::UploadOpt {
            template: TemplateKind::Nested,
        },
        Step::UploadOpt {
            template: TemplateKind::Persistent,
        },
        Step::ListTemplates,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_creates_before_dependents() {
        let steps = admission_steps();
        assert_eq!(steps.first(), Some(&Step::CreateEhr));
        assert!(
            steps.iter().any(|s| matches!(
                s,
                Step::CreateComposition {
                    template: TemplateKind::Vitals,
                    ..
                }
            )),
            "admission must seed a vitals composition for later E7 updates"
        );
        assert!(
            steps.contains(&Step::UpdateDirectory),
            "admission must establish the directory for E6 reads"
        );
    }

    #[test]
    fn chart_review_is_five_reads() {
        let steps = ClinicalEvent::ChartReview.steps();
        assert_eq!(steps.len(), 5);
        assert!(steps.iter().all(|s| s.op_class().is_read()));
    }

    #[test]
    fn every_event_maps_to_a_class() {
        for ev in ClinicalEvent::ALL {
            for step in ev.steps() {
                // op_class is total; this just exercises every variant.
                let _ = step.op_class();
            }
        }
    }
}
