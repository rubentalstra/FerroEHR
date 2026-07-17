//! The hospital-day stress instrument.
//!
//! Implements a pre-registered clinical workload and a fixed measurement set
//! under a fairness methodology: an identical client for both SUTs (via the
//! conformance transport), coordinated-omission correction, symmetric warmup,
//! published raw data, and an explicit "where the other side wins" account.
//! No openEHR spec governs this — it is our own design.
//!
//! Shape: a deterministic generator ([`model`]) turns a ward of patients and
//! a clinical day into an **open-loop arrival schedule** of [`PlannedOp`]s;
//! [`render`] produces seeded instance payloads over the vendored fixture
//! skeletons; the driver ([`drive`]) dispatches at planned times against any
//! SUT (the conformance `SutClient` — the provably-ECC-identical client),
//! resolving per-patient runtime ids; [`measure`] records per-class
//! `HdrHistogram`s against *planned* send times so a stalled SUT cannot hide
//! its tail; [`sample`] captures container CPU/RSS, cold start, and storage
//! footprint; [`report`] emits `results.json` + `REPORT.md` — generated,
//! never hand-typed.
//!
//! Two pedantic lints are allowed crate-wide as they fight this crate's
//! nature, not any defect: `format_push_string` (the report builder appends
//! `format!` to a `String`) and `cast_precision_loss` (metric math casts
//! counts/latencies to `f64`; the loss is irrelevant at these magnitudes).
#![allow(clippy::format_push_string, clippy::cast_precision_loss)]
// Verification CLI: progress/diagnostics on the console ARE this tool's user
// interface — the reliability deny-tier for shipped code deliberately relaxes
// stdio here (.claude/rules/reliability.md §tools).
#![allow(clippy::print_stdout, clippy::print_stderr)]

pub mod drive;
pub mod measure;
pub mod model;
pub mod pack;
pub mod render;
pub mod report;
pub mod sample;
pub mod seed;

use std::time::Duration;

/// A harness error.
#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    /// A transport-level failure reaching the SUT.
    #[error("transport: {0}")]
    Transport(#[from] conformance::harness::TransportError),
    /// A fixture could not be read.
    #[error("fixture: {0}")]
    Fixture(String),
    /// The SUT returned an unexpected response during setup/seeding.
    #[error("unexpected: {0}")]
    Unexpected(String),
    /// Artefact/sampler I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialization of an artefact.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// The latency-histogram operation classes (register 01 §1). One histogram
/// per class; the class names are the stable keys in `results.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OpClass {
    EhrCreate,
    EhrRead,
    CompCreateSmall,
    CompCreateLarge,
    CompUpdate,
    CompReadLatest,
    CompReadVersion,
    ContributionCommit,
    AqlPatient,
    AqlWard,
    DirRead,
    DirUpdate,
    HistoryRead,
    StatusUpdate,
    OptUpload,
    TplList,
}

impl OpClass {
    /// Every class, in report order.
    pub const ALL: [OpClass; 16] = [
        OpClass::EhrCreate,
        OpClass::EhrRead,
        OpClass::CompCreateSmall,
        OpClass::CompCreateLarge,
        OpClass::CompUpdate,
        OpClass::CompReadLatest,
        OpClass::CompReadVersion,
        OpClass::ContributionCommit,
        OpClass::AqlPatient,
        OpClass::AqlWard,
        OpClass::DirRead,
        OpClass::DirUpdate,
        OpClass::HistoryRead,
        OpClass::StatusUpdate,
        OpClass::OptUpload,
        OpClass::TplList,
    ];

    /// The stable `results.json` key.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            OpClass::EhrCreate => "ehr-create",
            OpClass::EhrRead => "ehr-read",
            OpClass::CompCreateSmall => "comp-create-small",
            OpClass::CompCreateLarge => "comp-create-large",
            OpClass::CompUpdate => "comp-update",
            OpClass::CompReadLatest => "comp-read-latest",
            OpClass::CompReadVersion => "comp-read-version",
            OpClass::ContributionCommit => "contribution-commit",
            OpClass::AqlPatient => "aql-patient",
            OpClass::AqlWard => "aql-ward",
            OpClass::DirRead => "dir-read",
            OpClass::DirUpdate => "dir-update",
            OpClass::HistoryRead => "history-read",
            OpClass::StatusUpdate => "status-update",
            OpClass::OptUpload => "opt-upload",
            OpClass::TplList => "tpl-list",
        }
    }

    /// Whether the class is a read (the 70:30 budget check).
    #[must_use]
    pub fn is_read(self) -> bool {
        matches!(
            self,
            OpClass::EhrRead
                | OpClass::CompReadLatest
                | OpClass::CompReadVersion
                | OpClass::AqlPatient
                | OpClass::AqlWard
                | OpClass::DirRead
                | OpClass::HistoryRead
                | OpClass::TplList
        )
    }
}

/// A run profile (register 00 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// CI/self-test: fixed small event count, steady rates, ~2 min.
    Smoke,
    /// The standard measured run: steady state at daily-mean rates.
    Hour,
    /// The realism profile: a compressed day with the diurnal curve.
    Day,
}

impl Profile {
    /// Stable name (CLI value + artefact field).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Profile::Smoke => "smoke",
            Profile::Hour => "hour",
            Profile::Day => "day",
        }
    }
}

/// A scale-ladder rung (register 00 §5): pre-seeded compositions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Empty,
    TenK,
    HundredK,
    OneM,
}

impl Scale {
    /// The number of compositions the seeder provisions.
    #[must_use]
    pub fn compositions(self) -> u64 {
        match self {
            Scale::Empty => 0,
            Scale::TenK => 10_000,
            Scale::HundredK => 100_000,
            Scale::OneM => 1_000_000,
        }
    }

    /// Stable name (CLI value + artefact field).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Scale::Empty => "empty",
            Scale::TenK => "10k",
            Scale::HundredK => "100k",
            Scale::OneM => "1m",
        }
    }
}

/// The template a payload is rendered from (register 00 §4). Two packs: the
/// retained ECC-corpus fixtures ([`Vitals`](TemplateKind::Vitals)/
/// [`Nested`](TemplateKind::Nested)/[`Persistent`](TemplateKind::Persistent),
/// keyed to the fixtures the ECC suite provisions) and the official openEHR CKM
/// pack (`Ckm*`, sourced from the vendored [`crate::pack`] — `templates/ckm/`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateKind {
    /// Small/hot-path event composition (vitals-class) — ECC corpus fixture.
    Vitals,
    /// Large, deeply nested event composition — ECC corpus fixture.
    Nested,
    /// Persistent composition (care plan / directory class) — ECC corpus
    /// fixture; retained for the persistent/directory structure.
    Persistent,
    /// Official CKM **Vital signs** template (E2 shift observations; small,
    /// hot-path event composition). Sourced from the vendored CKM pack
    /// ([`crate::pack`]), not the ECC corpus.
    CkmVitalSigns,
    /// Official CKM **Generic lab test result** template (E4 lab-result
    /// contribution batches).
    CkmLabResult,
    /// Official CKM **Medication order** template (E3 medication rounds).
    CkmMedicationOrder,
    /// Official CKM **International Patient Summary** template (E1 admission
    /// assessment / E9 discharge summary; large, deeply nested — the
    /// deep-stress payload).
    CkmSummary,
    /// Official CKM **Clinical synopsis** template (E7 documentation
    /// corrections; the per-patient correction target seeded at admission).
    CkmSynopsis,
}

/// One scheduled operation: dispatched at `at` (offset from the measurement
/// window start — the *planned* send time coordinated-omission correction
/// measures against), on behalf of ward patient `patient`.
#[derive(Debug, Clone)]
pub struct PlannedOp {
    /// Planned send offset from the window start.
    pub at: Duration,
    /// The latency class the sample is recorded under.
    pub class: OpClass,
    /// Ward patient index (the driver resolves ids per patient).
    pub patient: usize,
    /// What to execute.
    pub action: Action,
    /// The business transaction (clinical-event occurrence) this op is a step
    /// of. Threaded through so the driver can count an event *completed* only
    /// when every one of its steps succeeded — the steps of one occurrence are
    /// dispatched as independent open-loop tasks, so completion cannot be
    /// inferred at the op level (checklist item 25b).
    pub event: crate::model::event::EventInstance,
}

/// The semantic operation of a [`PlannedOp`]. Payloads are pre-rendered by
/// [`render`] at schedule build time (deterministic); runtime identifiers
/// (`ehr_id`, composition object/version uids) are resolved by the driver
/// from its per-patient table at dispatch. AQL query strings carry the
/// `{{ehr_id}}` placeholder the driver substitutes.
#[derive(Debug, Clone)]
pub enum Action {
    /// `POST /ehr` — registers the patient's `ehr_id` (with an `EHR_STATUS`
    /// carrying the generated subject id).
    CreateEhr { status: serde_json::Value },
    /// `GET /ehr/{ehr_id}`.
    ReadEhr,
    /// `POST …/composition` with a rendered canonical-JSON body.
    CreateComposition {
        template: TemplateKind,
        payload: serde_json::Value,
    },
    /// `PUT …/composition/{object}` (If-Match latest) — a new version of the
    /// patient's most recent composition of `template`.
    UpdateComposition {
        template: TemplateKind,
        payload: serde_json::Value,
    },
    /// `GET …/composition/{object}` (latest) of a previously created one.
    ReadLatestComposition,
    /// `GET …/composition/{ovid}` — a specific earlier version.
    ReadCompositionVersion,
    /// `POST …/contribution` — a rendered multi-version batch of `template`
    /// compositions (the `template` is recorded so an excluded CKM template
    /// skips its contribution ops at dispatch rather than erroring silently).
    CommitContribution {
        template: TemplateKind,
        payload: serde_json::Value,
    },
    /// Patient-scoped AQL (`{{ehr_id}}` substituted at dispatch).
    AqlPatient { query: String },
    /// Ward-population AQL (no patient filter).
    AqlWard { query: String },
    /// `GET /ehr/{ehr_id}/directory`.
    ReadDirectory,
    /// Create-or-update the patient's directory (versioned FOLDER write).
    UpdateDirectory { payload: serde_json::Value },
    /// `GET …/versioned_composition/{uid}/revision_history`.
    ReadRevisionHistory,
    /// `PUT /ehr/{ehr_id}/ehr_status` (If-Match latest).
    UpdateStatus { payload: serde_json::Value },
    /// OPT upload (provisioning; outside the measured mix).
    UploadOpt { template: TemplateKind },
    /// `GET /definition/template/adl1.4`.
    ListTemplates,
}
