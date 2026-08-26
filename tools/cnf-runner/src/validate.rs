// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Cross-artifact validation — the schedule's machine gates.
//!
//! Generalized from ECC's coverage-guard discipline: id uniqueness,
//! SM-operation resolution, spec-ref link checks, binding completeness,
//! `verified_by` resolution, corpus integrity, ambiguity/option resolution,
//! capability-vs-tier consistency, reference/sentinel grammar,
//! decision-table literals, and vocabulary drift.
//!
//! Every check is pure over the loaded [`ArtifactSet`] (+ the vendored spec
//! tree for the two resolution checks); every violation is one typed
//! [`Finding`].

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::artifacts::ArtifactSet;
use crate::exec::headers::structural_token;
use crate::ids::{CapabilityName, CaseId, CorpusKey, SmOperationRef, ViewName};
use crate::literal::{Literal, ViolationRef};
use crate::load::LoadError;
use crate::model::assertion::{Assertion, EquivalentTarget, assertion_refs};
use crate::model::binding::{HeaderMatcher, OperationBinding, placeholder_names};
use crate::model::capability::Realization;
use crate::model::case::{
    CaseCore, ExpectSpec, FlowStep, ImportRequirement, MatrixCell, Parameters,
    PartyRelationshipRequirement,
};
use crate::model::value::TemplatedValue;
use crate::model::wire_surface::{ServedExtension, WireSurface};
use crate::refgrammar::{CaptureField, TimeExpr, ValueRef};
use crate::vocab::{CaseKind, CaseStatus, Disposition, FormatName, Iteration, OutcomeKind};

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
    /// Decision-table literals + violation categories + the closed token
    /// vocabularies a flow's bundled `versions:` members may spell.
    LiteralGrammar,
    /// `sm_operation` (and flow `call:`) resolution against the vendored SM.
    SmOperation,
    /// `spec_refs` resolution against the vendored spec tree.
    SpecRef,
    /// Every used outcome kind mapped, every used capture wired, per binding.
    BindingCompleteness,
    /// A binding file's stem states the binding it holds:
    /// `<sm_operation>[-<variant>]`.
    BindingFilename,
    /// Every `with:` key a flow step authors is CONSUMED by the binding the
    /// driver would select for it (issue #1830): an argument no request form
    /// reads is decoration the SUT never sees, so the case's assertion about
    /// it passes vacuously — the whole reason the gate exists.
    StepArguments,
    /// Every `pattern:` header-matcher placeholder a driven outcome declares
    /// can RESOLVE at the step it is judged on (issue #1852, the un-evaluable
    /// side of #1830): a name that is neither a structural token nor a
    /// variable in scope refuses at drive time, reddening the row on the
    /// runner's own resolution failure instead of on the SUT's behaviour.
    MatcherPlaceholder,
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
    /// Claim completeness (issue #622): a capability a committed party
    /// statement claims has ≥ 1 verdict-bearing catalogue case, and a
    /// capability whose every case resolves excused/deselected names the
    /// register entry that adjudicated that. Declaring a capability IS the
    /// obligation to run the framework against it, so a hollow claim cannot
    /// even enter a run — the gate is at validate time, before any SUT is
    /// composed.
    ClaimCompleteness,
    /// Guard scope (issue #2378): a `guards:` entry may not state a
    /// CAPABILITY-scoped selection rule. That rule is global, implemented once
    /// in the runner's selection over the case's own `capabilities:` list; a
    /// prose restatement either duplicates it (free to drift, undetectably) or
    /// names a capability the case does not gate, and then states a rule
    /// nothing implements.
    GuardScope,
    /// A party statement's declared `served_extensions` families (issue
    /// #2377): each resolves in the catalogue's outward wire-surface axis and
    /// is declared once. A statement publishes the route families ITS OWN
    /// party declares — never the catalogue's global table, which is one
    /// product's own design and a false claim about any other vendor.
    ServedExtensionDeclaration,
    /// Per-capability case-count floors (issue #622): one token case never
    /// certifies a capability. The capability matrix records each row's
    /// `min_cases`; a battery below its floor is a finding naming the
    /// shortfall. Floors ratchet UP only.
    CapabilityDepth,
    /// Measured-workload coverage (issue #622): every claimed capability is
    /// either exercised by the hospital-simulation journeys the performance
    /// workloads name, or carries a register-linked `workload_exclusion` the
    /// certificate renders. A bare `NO — catalogue gap` row is an undecided
    /// hole, never a publishable one.
    WorkloadCoverage,
    /// Realization scoping (issue #623): an `extension` binding drives a
    /// route no openEHR specification governs, so it is fenced off from every
    /// released-wire judgement — its family and path must resolve in the
    /// `served_extensions` axis, its adjudication must resolve in the
    /// register, and every capability its cases carry must be
    /// `realization: extension` in the matrix (never `required`). The reverse
    /// bites too: an `extension` matrix row whose cases drive released wire
    /// is mislabelled, and understates the conformance the product earned.
    RealizationScope,
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
            Self::BindingFilename => "binding-filename",
            Self::StepArguments => "step-arguments",
            Self::MatcherPlaceholder => "matcher-placeholder",
            Self::VerifiedBy => "verified-by",
            Self::CorpusIntegrity => "corpus-integrity",
            Self::AmbiguityLink => "ambiguity-link",
            Self::OptionTag => "option-tag",
            Self::CapabilityTier => "capability-tier",
            Self::VocabDrift => "vocab-drift",
            Self::JourneyEnvelope => "journey-envelope",
            Self::ClaimCompleteness => "claim-completeness",
            Self::GuardScope => "guard-scope",
            Self::ServedExtensionDeclaration => "served-extension-declaration",
            Self::CapabilityDepth => "capability-depth",
            Self::WorkloadCoverage => "workload-coverage",
            Self::RealizationScope => "realization-scope",
            Self::SurfaceCoverage => "surface-coverage",
        }
    }
}

/// One violation.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Which gate produced the finding.
    pub check: CheckId,
    /// The offending artifact (file path or case id).
    pub artifact: String,
    /// What is wrong, in one line.
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
    /// The typed artifacts every gate reads.
    pub set: &'a ArtifactSet,
    /// The files that failed to load, reported as findings of their own.
    pub load_errors: &'a [LoadError],
    /// The vendored spec tree, enabling the citation-resolution gates.
    pub spec_root: Option<&'a Path>,
}

/// Run every gate; findings in check order.
#[must_use]
pub fn validate(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    // One memoizing reader for the whole run: the citation gates otherwise
    // re-read the same SM class exports and re-walk the same vendored
    // component directories once per case.
    let spec = ctx.spec_root.map(SpecIndex::new);

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
        if let Some(spec) = spec.as_ref() {
            check_sm_operations(case, &who, spec, &mut findings);
            check_spec_refs(case, &who, spec, &mut findings);
        }
    }
    check_binding_completeness(ctx.set, &mut findings);
    check_step_arguments(ctx.set, &mut findings);
    check_matcher_placeholders(ctx.set, &mut findings);
    for (path, binding) in &ctx.set.bindings {
        let who = path.display().to_string();
        if let Err(message) = binding.check_invariants() {
            push(&mut findings, CheckId::KindShape, &who, message);
        }
        check_binding_filename(path, binding, &mut findings);
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
        if let Some(spec) = spec.as_ref() {
            resolve_sm_operation(&binding.sm_operation, &who, spec, &mut findings);
            check_binding_sources(binding, &who, spec, &mut findings);
        }
    }
    check_corpus_integrity(ctx.set, &mut findings);
    if let Some(spec) = spec.as_ref() {
        check_corpus_spec_refs(ctx.set, spec, &mut findings);
        check_register_sources(ctx.set, spec, &mut findings);
    }
    check_vocab_drift(ctx.set, &mut findings);
    check_journey_envelope(ctx.set, &mut findings);
    check_claim_completeness(ctx.set, &mut findings);
    check_guard_scope(ctx.set, &mut findings);
    check_served_extension_declarations(ctx.set, &mut findings);
    check_capability_depth(ctx.set, &mut findings);
    check_workload_coverage(ctx.set, &mut findings);
    check_realization_scope(ctx.set, &mut findings);
    check_surface_coverage(ctx.set, spec.as_ref(), &mut findings);

    findings
}

// ── claim completeness, depth floors, workload coverage (issue #622) ────────

/// How a catalogue case will resolve for verdict purposes, as far as the
/// CATALOGUE alone can say — the static twin of [`crate::verdict::Evidence`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    /// The case can carry executed evidence.
    Gating,
    /// Every operation the flow calls is `unrealized` on this ITS, so the
    /// runner records it not-applicable with the binding's citation.
    ExcusedUnrealized,
    /// The case realizes one branch of an `option_select` ambiguity: it
    /// carries evidence only for a party whose ICS declares that branch.
    OptionGated,
}

/// The catalogue-side resolution of one case (see [`Resolution`]).
fn resolution(set: &ArtifactSet, case: &CaseCore) -> Resolution {
    if crate::run::fully_unrealized(set, case).is_some() {
        Resolution::ExcusedUnrealized
    } else if case.option.is_some() {
        Resolution::OptionGated
    } else {
        Resolution::Gating
    }
}

/// Whether a `report_only` register entry suspends the case's gating (such a
/// case reports but never contributes to a verdict — [`crate::verdict`]).
fn suspended_report_only(set: &ArtifactSet, case: &CaseCore) -> bool {
    let Some((_, register)) = &set.register else {
        return false;
    };
    case.ambiguities.iter().any(|id| {
        register
            .get(id)
            .is_some_and(|e| e.disposition == Disposition::ReportOnly)
    })
}

/// The verdict-bearing cases of one capability: active cases naming it whose
/// gating is not suspended by a `report_only` register entry.
///
/// This is the count the depth floor measures and the set the claim gate
/// requires to be non-empty.
#[must_use]
pub fn verdict_bearing<'a>(set: &'a ArtifactSet, cap: &CapabilityName) -> Vec<&'a CaseCore> {
    set.cases
        .iter()
        .map(|(_, c)| c)
        .filter(|c| {
            c.status == CaseStatus::Active
                && c.capabilities.contains(cap)
                && !suspended_report_only(set, c)
        })
        .collect()
}

/// A claim without cases is a certification hole, and a capability whose
/// every case resolves excused/deselected is one too unless a register entry
/// says otherwise.
///
/// ISO/IEC 9646 test selection legitimizes "not applicable" only for a
/// capability the party does NOT claim; a claimed-but-unevidenced row is not
/// a selection outcome (owner directive 2026-07-28).
fn check_claim_completeness(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    let matrix_who = matrix_path.display().to_string();

    for (party_path, statement) in &set.parties {
        let who = party_path.display().to_string();
        for cap in &statement.claims.capabilities {
            // An unknown capability is the static review's / capability-tier
            // gate's finding, not this one.
            if matrix.get(cap).is_none() {
                continue;
            }
            if verdict_bearing(set, cap).is_empty() {
                push(
                    findings,
                    CheckId::ClaimCompleteness,
                    &who,
                    format!(
                        "claimed capability {cap} has zero verdict-bearing catalogue cases — \
                         declaring a capability is the obligation to run the CNF framework \
                         against it; author its battery or withdraw the claim"
                    ),
                );
            }
        }
    }

    for (name, entry) in matrix.entries() {
        let cases = verdict_bearing(set, name);
        let all_excused = !cases.is_empty()
            && cases
                .iter()
                .all(|c| resolution(set, c) != Resolution::Gating);
        match (&entry.evidence_exception, all_excused) {
            (None, true) => push(
                findings,
                CheckId::ClaimCompleteness,
                &matrix_who,
                format!(
                    "{name}: every one of its {} verdict-bearing case(s) resolves excused or \
                     deselected, so the capability can never carry executed evidence — name the \
                     adjudicating register entry in `evidence_exception`, realize the wire, or \
                     move the capability to the extension surface",
                    cases.len()
                ),
            ),
            (Some(adjudication), true) => {
                if set
                    .register
                    .as_ref()
                    .is_some_and(|(_, r)| r.get(&adjudication.register).is_none())
                {
                    push(
                        findings,
                        CheckId::ClaimCompleteness,
                        &matrix_who,
                        format!(
                            "{name}: evidence_exception cites {} which is not in the register",
                            adjudication.register
                        ),
                    );
                }
            }
            (Some(adjudication), false) => push(
                findings,
                CheckId::ClaimCompleteness,
                &matrix_who,
                format!(
                    "{name}: evidence_exception ({}) is stale — the capability now has cases \
                     that can carry executed evidence ({} of {}); delete the exception",
                    adjudication.register,
                    cases
                        .iter()
                        .filter(|c| resolution(set, c) == Resolution::Gating)
                        .count(),
                    cases.len()
                ),
            ),
            (None, false) => {}
        }
    }
}

/// A `guards:` entry may not state a CAPABILITY-scoped selection rule.
///
/// Capability scoping is a GLOBAL law the runner implements once
/// ([`crate::run`] selection: a case gating only capabilities the ICS does not
/// claim is not-applicable with its citation, and the verdict pipeline selects
/// on the same predicate), driven by the case's own `capabilities:` list. A
/// per-case prose restatement of that law is either redundant — and then free
/// to drift from what the runner does, undetectably, which is the defect this
/// gate exists for — or it names a capability the case does not gate, in which
/// case the rule it states is not implemented for this case at all.
///
/// Prose guards stay legal for every rule outside the typed shapes; the
/// boundary is stated on the `guards` property of the published case-core
/// schema.
fn check_guard_scope(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((_, matrix)) = &set.matrix else {
        return;
    };
    let scoping_phrases = [
        "not-applicable",
        "not applicable",
        "applies only",
        "only applies",
        "apply only",
        "unless the sut",
        "when the sut declares no",
        "when the sut does not",
    ];
    // The DECLARATION-ABSENCE phrasings the instance/requirement shapes key
    // on: a guard saying the party "declares no X" restates a selection the
    // runner already decides from the case's own typed shape (`on:`
    // addressing, `requires.terminology`, `requires.spec_profile`).
    let declaration_phrases = ["declares no", "does not declare", "declaring no"];
    for (path, case) in &set.cases {
        let who = path.display().to_string();
        for guard in &case.guards {
            let lowered = guard.to_lowercase();
            if declaration_phrases.iter().any(|p| lowered.contains(p)) {
                // The INSTANCE shape (#2378's sibling, #2389): the typed shape
                // is the flow's own `on:` addressing — `run.rs` excuses a case
                // addressing an instance the party does not declare, with the
                // citation, once and globally.
                for name in crate::run::addressed_instances(case) {
                    if mentions_word(guard, name.as_str()) {
                        push(
                            findings,
                            CheckId::GuardScope,
                            &who,
                            format!(
                                "guard {guard:?} restates the undeclared-instance selection \
                                 rule for `{name}`, which the runner implements globally from \
                                 the flow's own `on:` addressing — drop the guard and keep any \
                                 spec citation in spec_refs"
                            ),
                        );
                    }
                }
                // The REQUIREMENT shapes: `requires.terminology` and
                // `requires.spec_profile` are matched at selection time by the
                // same law, so a prose copy on a case that declares the typed
                // block restates an implemented rule.
                if case.requires.terminology.is_some() && lowered.contains("terminology") {
                    push(
                        findings,
                        CheckId::GuardScope,
                        &who,
                        format!(
                            "guard {guard:?} restates the terminology selection rule the \
                             runner implements from this case's own `requires.terminology` — \
                             drop the guard and keep any spec citation in spec_refs"
                        ),
                    );
                }
                let requires_profile = case.requires.spec_profile.is_some()
                    || case
                        .requires
                        .instances
                        .as_ref()
                        .is_some_and(|map| map.values().any(|r| r.spec_profile.is_some()));
                if requires_profile && lowered.contains("spec_profile") {
                    push(
                        findings,
                        CheckId::GuardScope,
                        &who,
                        format!(
                            "guard {guard:?} restates the generation-set selection rule the \
                             runner implements from this case's own `requires.spec_profile` — \
                             drop the guard and keep any spec citation in spec_refs"
                        ),
                    );
                }
            }
            if !scoping_phrases.iter().any(|p| lowered.contains(p)) {
                continue;
            }
            for (name, _) in matrix.entries() {
                if !mentions_word(guard, &name.to_string()) {
                    continue;
                }
                let gated = case.capabilities.contains(name);
                push(
                    findings,
                    CheckId::GuardScope,
                    &who,
                    if gated {
                        format!(
                            "guard {guard:?} restates the capability-scoping rule for {name}, \
                             which the runner implements globally from the case's own \
                             `capabilities:` list — drop the guard; a per-case restatement can \
                             drift from the implemented rule with nothing to catch it"
                        )
                    } else {
                        format!(
                            "guard {guard:?} states a selection rule scoped to {name}, but the \
                             case does not gate that capability — the runner selects on \
                             `capabilities:` alone, so this rule is stated and not implemented; \
                             declare the capability or drop the claim"
                        )
                    },
                );
            }
        }
    }
}

/// Whether `text` contains `word` as a standalone token (no alphanumeric or
/// `_` neighbour), so a capability name is not matched inside a longer word.
fn mentions_word(text: &str, word: &str) -> bool {
    let boundary = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric() && c != '_');
    text.match_indices(word).any(|(at, _)| {
        boundary(text.get(..at).and_then(|s| s.chars().next_back()))
            && boundary(text.get(at + word.len()..).and_then(|s| s.chars().next()))
    })
}

/// A party's declared `served_extensions` families resolve in the catalogue's
/// outward wire-surface axis, and each is declared once.
///
/// The statement renders the route detail of exactly what the party declares,
/// so an unresolvable family would publish a family name with no routes behind
/// it, and a repeated one would publish the same family twice. The declaration
/// is per party by construction: a route family is one product's own design
/// (no openEHR specification governs it), so no party's statement may carry
/// another's surface.
fn check_served_extension_declarations(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let known: BTreeSet<&str> = set
        .wire_surface
        .as_ref()
        .map_or_else(BTreeSet::new, |(_, w)| {
            w.served_extensions
                .iter()
                .map(|e| e.family.as_str())
                .collect()
        });
    for (party_path, statement) in &set.parties {
        let who = party_path.display().to_string();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for family in &statement.served_extensions {
            if !seen.insert(family.as_str()) {
                push(
                    findings,
                    CheckId::ServedExtensionDeclaration,
                    &who,
                    format!("served_extensions declares {family:?} more than once"),
                );
            }
            if !known.contains(family.as_str()) {
                push(
                    findings,
                    CheckId::ServedExtensionDeclaration,
                    &who,
                    format!(
                        "served_extensions declares {family:?}, which is not a family of the \
                         served_extensions axis of vocab/wire_surface.yaml — a declared family \
                         must carry its routes and configuration gate there, or the statement \
                         publishes a name with nothing behind it"
                    ),
                );
            }
        }
    }
}

/// One token case does not certify a capability: every matrix row records the
/// verdict-bearing case count its battery must keep (`min_cases`), and the
/// floors ratchet UP only.
fn check_capability_depth(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    let who = matrix_path.display().to_string();
    for (name, entry) in matrix.entries() {
        let count = verdict_bearing(set, name).len();
        if count < entry.min_cases {
            push(
                findings,
                CheckId::CapabilityDepth,
                &who,
                format!(
                    "{name}: {count} verdict-bearing case(s) against a floor of {} — short by \
                     {}; coverage only ratchets up, so restore the battery (never lower the \
                     floor)",
                    entry.min_cases,
                    entry.min_cases.saturating_sub(count)
                ),
            );
        }
    }
}

// ── realization scoping (issue #623) ────────────────────────────────────────

/// A route path with every `{parameter}` segment collapsed to `{}` — the shape
/// two artifacts can be compared on when each names its parameters locally.
fn path_shape(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether every operation the case's flow calls that resolves to a binding
/// at all is an EXTENSION realization — the extension twin of
/// [`crate::run::fully_unrealized`]. A case that touches even one released
/// operation is NOT an extension-only case: it earns released-wire evidence
/// too, so its capabilities may keep the released-wire marker.
fn drives_only_extension_bindings(set: &ArtifactSet, case: &CaseCore) -> bool {
    let mut saw_binding = false;
    for step in &case.flow {
        // A step with no resolvable binding is the binding-completeness gate's
        // finding, not this one — skip it rather than double-report.
        let Some(binding) = select_binding_for_step(set, case, step) else {
            continue;
        };
        saw_binding = true;
        if !binding.is_extension() {
            return false;
        }
    }
    saw_binding
}

/// An extension realization is fenced off from every released-wire judgement,
/// and the fence is structural rather than conventional:
///
/// 1. the binding's `family` + request path resolve in the `served_extensions`
///    axis, so a binding can only drive a route the SUT actually DECLARES;
/// 2. its adjudication resolves in the ambiguity register;
/// 3. a capability whose cases drive only extension bindings is
///    `realization: extension` in the matrix — and, since
///    `check_realization_scoping` forbids `required` there, no openEHR
///    profile tier can ever rest on it (owner ruling 2026-07-28, #610);
/// 4. the reverse: an `extension` row whose cases drive released wire is
///    mislabelled and understates what the product earned.
fn check_realization_scope(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    check_extension_bindings(set, findings);
    check_realization_markers(set, findings);
}

/// (1) + (2) of [`check_realization_scope`]: every extension binding names a
/// declared family, drives one of that family's declared routes, and cites a
/// register entry that resolves.
fn check_extension_bindings(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    for (path, binding) in &set.bindings {
        let Some(decl) = &binding.extension else {
            continue;
        };
        let who = path.display().to_string();
        if set
            .register
            .as_ref()
            .is_some_and(|(_, r)| r.get(&decl.ambiguity).is_none())
        {
            push(
                findings,
                CheckId::RealizationScope,
                &who,
                format!(
                    "extension declaration cites {} which is not in the register",
                    decl.ambiguity
                ),
            );
        }
        let Some((_, wire_surface)) = &set.wire_surface else {
            continue;
        };
        let Some(family) = wire_surface
            .served_extensions
            .iter()
            .find(|e| e.family == decl.family)
        else {
            push(
                findings,
                CheckId::RealizationScope,
                &who,
                format!(
                    "extension family {:?} is not declared in the served_extensions axis of \
                     vocab/wire_surface.yaml — an extension binding may only drive a route the \
                     SUT declares outwardly",
                    decl.family
                ),
            );
            continue;
        };
        let Some(request) = binding.request.as_ref() else {
            continue; // the shape invariant already reported it
        };
        // Path-parameter NAMES are local to each artifact (the axis writes the
        // served `{uid_based_id}`, a binding writes the capture it fills the
        // segment from), so the comparison is on path SHAPE: every `{…}`
        // segment collapses to `{}`. The axis writes absolute
        // default-deployment paths and a binding path is base-relative, hence
        // the suffix match.
        let wanted = path_shape(request.path.raw());
        let declared = family.routes.iter().any(|route| {
            ServedExtension::route_path(route)
                .is_some_and(|declared| path_shape(declared).ends_with(wanted.as_str()))
        });
        if !declared {
            push(
                findings,
                CheckId::RealizationScope,
                &who,
                format!(
                    "request path {} is not one of the routes the {:?} served_extensions family \
                     declares — declare the route or bind a declared one",
                    request.path.raw(),
                    decl.family
                ),
            );
        }
    }
}

/// (3) + (4) of [`check_realization_scope`]: the matrix `realization` marker
/// matches what the capability's verdict-bearing cases actually drive, in both
/// directions.
fn check_realization_markers(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    let matrix_who = matrix_path.display().to_string();
    for (name, entry) in matrix.entries() {
        let cases = verdict_bearing(set, name);
        if cases.is_empty() {
            continue; // claim-completeness / capability-depth own the empty row
        }
        let all_extension = cases
            .iter()
            .all(|case| drives_only_extension_bindings(set, case));
        match (entry.realization, all_extension) {
            (Realization::ReleasedWire, true) => push(
                findings,
                CheckId::RealizationScope,
                &matrix_who,
                format!(
                    "{name}: every one of its {} verdict-bearing case(s) drives EXTENSION \
                     routes only, so the row must carry `realization: extension` — a \
                     released-wire marker would claim openEHR wire conformance the release \
                     does not define",
                    cases.len()
                ),
            ),
            (Realization::Extension, false) => push(
                findings,
                CheckId::RealizationScope,
                &matrix_who,
                format!(
                    "{name}: `realization: extension` is stale — at least one of its \
                     verdict-bearing cases drives RELEASED ITS-REST operations; delete the \
                     marker so the row claims the conformance it earns"
                ),
            ),
            _ => {}
        }
    }
}

/// The capabilities the measured hospital simulation exercises: the union of
/// the capability sets of every operation of every journey a performance
/// workload names. The certificate's Workload Coverage table computes the
/// same union from the measurement records that actually ran; this is its
/// catalogue-side twin, so a gap is caught before a run, not after one.
fn workload_exercised(set: &ArtifactSet) -> BTreeSet<&'static str> {
    let mut exercised = BTreeSet::new();
    let Some((_, catalogue)) = &set.journeys else {
        return exercised;
    };
    for (_, case) in &set.performance {
        for (name, _) in &case.workload.journeys {
            let Some(journey) = catalogue.get(name) else {
                continue; // the journey-envelope gate reports the dangling name
            };
            for stage in &journey.stages {
                if let Ok(op) = crate::perf::PerfOp::parse(&stage.op) {
                    exercised.extend(op.capabilities().iter().copied());
                }
            }
        }
    }
    exercised
}

/// A claimed capability the measured workload never touches is either a
/// journey-catalogue gap to close or an adjudicated exclusion — never a bare
/// `NO — catalogue gap` row on a published certificate.
fn check_workload_coverage(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    let Some((matrix_path, matrix)) = &set.matrix else {
        return;
    };
    if set.performance.is_empty() {
        return; // nothing is measured, so nothing can be excluded from it
    }
    let matrix_who = matrix_path.display().to_string();
    let exercised = workload_exercised(set);

    for (party_path, statement) in &set.parties {
        let who = party_path.display().to_string();
        for cap in &statement.claims.capabilities {
            let Some(entry) = matrix.get(cap) else {
                continue;
            };
            if exercised.contains(cap.as_str()) || entry.workload_exclusion.is_some() {
                continue;
            }
            push(
                findings,
                CheckId::WorkloadCoverage,
                &who,
                format!(
                    "claimed capability {cap} is neither exercised by the measured \
                     hospital-simulation workload nor carries a `workload_exclusion` — extend \
                     the journey catalogue or adjudicate the exclusion in the capability matrix"
                ),
            );
        }
    }

    for (name, entry) in matrix.entries() {
        let Some(adjudication) = &entry.workload_exclusion else {
            continue;
        };
        if set
            .register
            .as_ref()
            .is_some_and(|(_, r)| r.get(&adjudication.register).is_none())
        {
            push(
                findings,
                CheckId::WorkloadCoverage,
                &matrix_who,
                format!(
                    "{name}: workload_exclusion cites {} which is not in the register",
                    adjudication.register
                ),
            );
        }
        if exercised.contains(name.as_str()) {
            push(
                findings,
                CheckId::WorkloadCoverage,
                &matrix_who,
                format!(
                    "{name}: workload_exclusion ({}) is stale — the hospital simulation now \
                     exercises the capability; delete the exclusion",
                    adjudication.register
                ),
            );
        }
    }
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
    // Every AUXILIARY payload a stage carries resolves in the corpus
    // manifest too: the Simplified-FLAT pair and the demographic fixtures
    // are committed corpus entries the functional catalogue already
    // adjudicates — the load instrument never invents a payload, so a
    // missing entry is an authoring defect, not a run-time surprise.
    if let Some((_, manifest)) = &set.corpus {
        let mut needed: Vec<crate::perf::AuxPayloadKind> = Vec::new();
        for (_, journey) in &catalogue.0 {
            for stage in &journey.stages {
                if let Some(kind) = crate::perf::PerfOp::parse(&stage.op)
                    .ok()
                    .and_then(crate::perf::PerfOp::aux_payload)
                    && !needed.contains(&kind)
                {
                    needed.push(kind);
                }
            }
        }
        for kind in needed {
            let keys: &[&str] = match kind {
                crate::perf::AuxPayloadKind::Flat => &[
                    crate::perf_run::pack::FLAT_OPT_KEY,
                    crate::perf_run::pack::FLAT_BODY_KEY,
                ],
                crate::perf::AuxPayloadKind::Person => &[
                    crate::perf_run::pack::PERSON_KEY,
                    crate::perf_run::pack::PERSON_AMENDED_KEY,
                ],
                crate::perf::AuxPayloadKind::PartyRelationship => {
                    &[crate::perf_run::pack::PARTY_RELATIONSHIP_KEY]
                }
                crate::perf::AuxPayloadKind::Tdd => &[
                    crate::perf_run::pack::TDD_OPT_KEY,
                    crate::perf_run::pack::TDD_BODY_KEY,
                ],
            };
            for key in keys {
                match CorpusKey::parse(key) {
                    Ok(parsed) if manifest.get(&parsed).is_some() => {}
                    _ => push(
                        findings,
                        CheckId::JourneyEnvelope,
                        &who,
                        format!(
                            "the catalogue names a stage whose payload is {kind:?}, but the \
                             corpus manifest has no entry {key}"
                        ),
                    ),
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
        // A `${ixit:…}` fact is declared by the PARTY, not by the catalogue,
        // so there is nothing in the artifact tree to resolve it against —
        // the closed field set is enforced at parse. A party that declares
        // the fact runs the case; one that does not records it
        // not-applicable with that citation (crate::run).
        ValueRef::Ixit(_) => {}
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
        // The SMART `scope` claim resolves on the same pre-step footing as
        // `with` (it is minted into the request's own Authorization header).
        for value in step.scope_templates() {
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

    // The three payloads a `requires.party_relationship` provisions (both
    // endpoint parties + the relationship) resolve like any other corpus
    // reference, so a typo is a catalogue finding here, not a drive-time error.
    let relationship_keys: Vec<&CorpusKey> = match &case.requires.party_relationship {
        Some(PartyRelationshipRequirement::Exists {
            source,
            target,
            relationship,
        }) => vec![source, target, relationship],
        Some(PartyRelationshipRequirement::None) | None => Vec::new(),
    };
    // The EXTRACT a `requires.import` replays resolves the same way, so a
    // dangling fixture reference is a catalogue finding rather than a
    // drive-time provisioning error.
    let import_key: Option<&CorpusKey> = match &case.requires.import {
        Some(ImportRequirement::Received { extract, .. }) => Some(extract),
        Some(ImportRequirement::None) | None => None,
    };
    for key in case
        .data_sets
        .iter()
        .chain(case.requires.templates.iter())
        .chain(case.requires.commit.iter())
        .chain(case.constraint_context.as_ref().map(|c| &c.template))
        .chain(relationship_keys)
        .chain(import_key)
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

/// The closed token vocabularies a CONTRIBUTION member of a flow's bundled
/// `versions:` construct may spell: its audit change kind
/// ([`crate::vocab::MemberChangeType`]), its class self-tag
/// ([`crate::vocab::MemberVersionType`]) and its committed version lifecycle
/// ([`crate::vocab::VersionLifecycleState`]).
///
/// The driver refuses an unknown token at drive time too — this gate is what
/// makes it a CATALOGUE finding instead, caught before a SUT is composed.
fn check_member_tokens(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    /// One member key and the closed vocabulary it must spell a value of.
    struct ClosedKey {
        /// The member key (`change_type`, `_type`, `lifecycle_state`).
        key: &'static str,
        /// Whether a token is in that key's vocabulary.
        accepts: fn(&str) -> bool,
        /// What the key expects, for the finding text.
        expected: &'static str,
    }
    let closed = [
        ClosedKey {
            key: "change_type",
            accepts: |t| crate::vocab::MemberChangeType::from_token(t).is_some(),
            expected: "a member of the openEHR audit_change_type group \
                       (creation | amendment | modification | synthesis | deleted | \
                       attestation | restoration | format conversion | unknown)",
        },
        ClosedKey {
            key: "_type",
            accepts: |t| crate::vocab::MemberVersionType::from_token(t).is_some(),
            expected: "a class of the RM VERSION family the commit wire is addressed with \
                       (UPDATE_VERSION | ORIGINAL_VERSION | IMPORTED_VERSION)",
        },
        ClosedKey {
            key: "lifecycle_state",
            accepts: |t| crate::vocab::VersionLifecycleState::from_token(t).is_some(),
            expected: "a state of the openEHR version_lifecycle_state group \
                       (complete | incomplete | deleted | inactive | abandoned)",
        },
    ];
    for step in &case.flow {
        for (name, value) in step.with_entries() {
            if name != "versions" {
                continue;
            }
            let TemplatedValue::Seq(members) = value else {
                continue;
            };
            for (i, member) in members.iter().enumerate() {
                let TemplatedValue::Map(entries) = member else {
                    continue;
                };
                for (key, cell) in entries {
                    let Some(closed_key) = closed.iter().find(|c| c.key == key) else {
                        continue;
                    };
                    // A member that authors the whole ORIGINAL_VERSION verbatim
                    // spells its `_type` INSIDE `data`, not here, so only the
                    // member-level key is judged; a templated token resolves per
                    // row and is judged by the driver.
                    let TemplatedValue::Text(template) = cell else {
                        continue;
                    };
                    let token = template.raw();
                    if !token.contains("${") && !(closed_key.accepts)(token) {
                        push(
                            findings,
                            CheckId::LiteralGrammar,
                            who,
                            format!(
                                "step {}: versions[{i}].{key}: {token:?} is not {}",
                                step.step, closed_key.expected
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn check_literals(case: &CaseCore, who: &str, findings: &mut Vec<Finding>) {
    check_member_tokens(case, who, findings);
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

/// One SM interface's parsed operation list, or the message explaining why it
/// could not be read.
type InterfaceOperations = Result<Rc<[String]>, String>;

/// A memoized section-name lookup: the component-relative document, and how
/// many `include::` levels below it were followed.
type SectionKey = (PathBuf, u8);

/// A memoizing reader over the vendored spec tree.
///
/// The citation-resolution gates ask the same questions once per case: the
/// same handful of SM class exports are read thousands of times, and the same
/// `(component dir, document token)` pairs are answered by re-walking a whole
/// vendored component directory each time. The vendored tree is read-only for
/// the lifetime of a validation run, so every answer is a pure function of the
/// path — the index caches those answers and nothing else. Findings are
/// byte-identical to an uncached run, including the io-error text a failed
/// read produces.
#[derive(Debug)]
struct SpecIndex<'a> {
    root: &'a Path,
    /// `path → file text`, or the io error's display text.
    texts: RefCell<BTreeMap<PathBuf, Result<Rc<str>, String>>>,
    /// `component dir → its file listing + basename index`.
    components: RefCell<BTreeMap<PathBuf, Rc<ComponentFiles>>>,
    /// `(document, include depth) → the section names it offers`.
    sections: RefCell<BTreeMap<SectionKey, Rc<BTreeSet<String>>>>,
    /// `document directory → the asciidoc attributes its book files define`.
    attributes: RefCell<BTreeMap<PathBuf, Rc<BTreeMap<String, String>>>>,
    /// `interface → its parsed SM operation names`.
    interfaces: RefCell<BTreeMap<String, InterfaceOperations>>,
    /// The ITS-XML component's SECOND root: the released XSD bundles.
    /// See [`SpecIndex::component_roots`].
    xml_schemas: Option<PathBuf>,
    /// The ITS-JSON component's SECOND root: the vendored ITS-JSON schema.
    json_schemas: Option<PathBuf>,
    /// The ITS-REST component's SECOND root: the vendored released OAS
    /// bundle artifacts (`*-codegen/html/validation.openapi.yaml`).
    rest_oas: Option<PathBuf>,
}

/// One vendored machine-readable bundle, located from the vendored spec root.
///
/// Both are repo-relative and both are vendored by committed scripts, so the
/// spec root's grandparent IS the workspace root (`docs/specs/openehr` →
/// `docs/specs` → `docs` → the workspace). The same derivation already
/// locates `docs/conformance/` for the coverage report. `None` when the
/// bundle is not there (a spec tree used outside the workspace, e.g. a test
/// fixture): the component's citations then resolve against the docs tree
/// alone, exactly as before issue #1833.
fn bundle_root(spec_root: &Path, relative: &str) -> Option<PathBuf> {
    let workspace = spec_root.parent()?.parent()?.parent()?;
    let bundle = workspace.join(relative);
    bundle.is_dir().then_some(bundle)
}

/// One vendored component directory's file listing, indexed for the two
/// lookups citation resolution needs.
#[derive(Debug, Default)]
struct ComponentFiles {
    /// Every file under the component dir: its lowercased, `/`-joined
    /// component-relative path paired with the real relative path.
    all: Vec<(String, PathBuf)>,
    /// Lowercased file name → the component-relative paths carrying it.
    by_name: BTreeMap<String, Vec<PathBuf>>,
}

impl<'a> SpecIndex<'a> {
    /// An empty index over one vendored spec root.
    fn new(root: &'a Path) -> Self {
        Self {
            root,
            texts: RefCell::new(BTreeMap::new()),
            components: RefCell::new(BTreeMap::new()),
            sections: RefCell::new(BTreeMap::new()),
            attributes: RefCell::new(BTreeMap::new()),
            interfaces: RefCell::new(BTreeMap::new()),
            xml_schemas: bundle_root(root, "crates/openehr-its/schemas/xml"),
            json_schemas: bundle_root(root, "crates/openehr-its/schemas/json"),
            rest_oas: bundle_root(root, "crates/openehr-its/vendor/rest-oas"),
        }
    }

    /// The directories a citation of `component` may resolve against, most
    /// authoritative first.
    ///
    /// Every component has exactly one — its vendored docs directory —
    /// except the three whose machine-readable artifacts are vendored
    /// OUTSIDE the docs tree. `scripts/vendor/spec-docs.sh` vendors PROSE,
    /// so the docs tree's `ITS-XML/components/**` holds only the upstream
    /// `README.adoc` stubs while the released XSD bundles live at
    /// `crates/openehr-its/schemas/xml/` as the canonical-XML codec's input —
    /// an XSD-element citation therefore resolved nowhere. Adjudication
    /// (issue #1833): the gate learns the bundle as a SECOND ROOT — one
    /// vendored copy, two readers — rather than the bundle being copied into
    /// the docs tree, which would fork the schemas the codec and the
    /// citations speak about. The same law (issue #2545) covers **ITS-JSON**
    /// (the validation-oracle schema at `crates/openehr-its/schemas/json/`)
    /// and **ITS-REST**'s released OAS BUNDLE artifacts
    /// (`crates/openehr-its/vendor/rest-oas/` — the assembled
    /// codegen/html/validation variants exist only there, and a
    /// bundle-variant divergence can only be cited against the bundle; the
    /// `specifications/**` source files stay first, in the docs tree).
    ///
    /// A root that does not exist is dropped, so an empty result is the
    /// "component dir missing" finding.
    fn component_roots(&self, component: &str, dir: &str) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        let docs = self.root.join(dir);
        if docs.is_dir() {
            roots.push(docs);
        }
        let second = match component {
            "ITS-XML" => &self.xml_schemas,
            "ITS-JSON" => &self.json_schemas,
            "ITS-REST" => &self.rest_oas,
            _ => &None,
        };
        if let Some(bundle) = second {
            roots.push(bundle.clone());
        }
        roots
    }

    /// The vendored spec root this index reads.
    fn root(&self) -> &Path {
        self.root
    }

    /// Read one vendored file, once per run.
    fn read(&self, path: &Path) -> Result<Rc<str>, String> {
        if let Some(hit) = self.texts.borrow().get(path) {
            return hit.clone();
        }
        let result = std::fs::read_to_string(path)
            .map(|text| Rc::from(text.as_str()))
            .map_err(|error| error.to_string());
        self.texts
            .borrow_mut()
            .insert(path.to_owned(), result.clone());
        result
    }

    /// One component directory's file listing, walked once per run.
    fn component_files(&self, dir: &Path) -> Rc<ComponentFiles> {
        if let Some(hit) = self.components.borrow().get(dir) {
            return Rc::clone(hit);
        }
        let mut listing = ComponentFiles::default();
        let mut stack = vec![dir.to_owned()];
        while let Some(current) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let Ok(relative) = path.strip_prefix(dir) else {
                    continue;
                };
                let lowered = relative
                    .components()
                    .filter_map(|c| c.as_os_str().to_str())
                    .collect::<Vec<_>>()
                    .join("/")
                    .to_lowercase();
                if let Some(name) = path.file_name().and_then(std::ffi::OsStr::to_str) {
                    listing
                        .by_name
                        .entry(name.to_lowercase())
                        .or_default()
                        .push(relative.to_owned());
                }
                listing.all.push((lowered, relative.to_owned()));
            }
        }
        listing.all.sort();
        let listing = Rc::new(listing);
        self.components
            .borrow_mut()
            .insert(dir.to_owned(), Rc::clone(&listing));
        listing
    }

    /// The asciidoc attributes the book files of one document directory define
    /// (`:pkg: org.openehr.rm.common.` and friends), read once per directory.
    /// They are what an `include::{uml_export_dir}/classes/{pkg}x.adoc[]`
    /// directive is resolved through.
    fn attributes(&self, dir: &Path) -> Rc<BTreeMap<String, String>> {
        if let Some(hit) = self.attributes.borrow().get(dir) {
            return Rc::clone(hit);
        }
        let mut attributes = BTreeMap::new();
        for book in ["master.adoc", "manifest_vars.adoc"] {
            let Ok(text) = self.read(&dir.join(book)) else {
                continue;
            };
            for line in text.lines() {
                let Some(rest) = line.strip_prefix(':') else {
                    continue;
                };
                let Some((name, value)) = rest.split_once(':') else {
                    continue;
                };
                if !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                {
                    attributes
                        .entry(name.to_owned())
                        .or_insert_with(|| value.trim().to_owned());
                }
            }
        }
        let attributes = Rc::new(attributes);
        self.attributes
            .borrow_mut()
            .insert(dir.to_owned(), Rc::clone(&attributes));
        attributes
    }

    /// The section names one vendored document offers, transitively through
    /// its `include::` directives (the class and interface tables live in the
    /// component's `UML/classes` export and are pulled into the chapter that
    /// documents them, so a chapter's section space is not what its own file
    /// literally contains).
    fn document_sections(&self, component: &Path, relative: &Path) -> Rc<BTreeSet<String>> {
        self.sections_at_depth(component, relative, INCLUDE_DEPTH)
    }

    fn sections_at_depth(
        &self,
        component: &Path,
        relative: &Path,
        depth: u8,
    ) -> Rc<BTreeSet<String>> {
        let key = (relative.to_owned(), depth);
        if let Some(hit) = self.sections.borrow().get(&key) {
            return Rc::clone(hit);
        }
        // Insert the empty set first: a cyclic include then terminates.
        self.sections
            .borrow_mut()
            .insert(key.clone(), Rc::new(BTreeSet::new()));
        let path = component.join(relative);
        let mut names = BTreeSet::new();
        if let Ok(text) = self.read(&path) {
            let yaml = matches!(
                path.extension().and_then(std::ffi::OsStr::to_str),
                Some("yaml" | "yml" | "json")
            );
            let xsd = matches!(
                path.extension().and_then(std::ffi::OsStr::to_str),
                Some("xsd")
            );
            if xsd {
                names.extend(xsd_declared_names(&text));
            } else if yaml {
                names.extend(structured_keys(&text));
            } else {
                names.extend(asciidoc_section_names(&text));
                if depth > 0 {
                    let directory = path.parent().unwrap_or(component).to_owned();
                    let attributes = self.attributes(&directory);
                    for target in include_targets(&text, &attributes) {
                        for included in self.included_files(component, &target) {
                            names.extend(
                                self.sections_at_depth(component, &included, depth - 1)
                                    .iter()
                                    .cloned(),
                            );
                        }
                    }
                }
            }
        }
        let names = Rc::new(names);
        self.sections.borrow_mut().insert(key, Rc::clone(&names));
        names
    }

    /// The vendored files one `include::` target names. The vendored tree
    /// carries no boilerplate directory, so `{ref_dir}`-style attributes stay
    /// unresolved: the target is matched by file name, and a still-unresolved
    /// prefix (`{pkg}extract_manifest.adoc`) by name suffix.
    fn included_files(&self, component: &Path, target: &str) -> Vec<PathBuf> {
        let name = target.rsplit('/').next().unwrap_or(target).to_lowercase();
        let files = self.component_files(component);
        match name.rsplit_once('}') {
            None => files.by_name.get(&name).cloned().unwrap_or_default(),
            Some((_, suffix)) if !suffix.is_empty() => files
                .by_name
                .iter()
                .filter(|(candidate, _)| candidate.ends_with(suffix))
                .flat_map(|(_, paths)| paths.iter().cloned())
                .collect(),
            Some(_) => Vec::new(),
        }
    }

    /// The service operations of an SM interface, parsed once per run.
    fn interface_operations(&self, interface: &str) -> InterfaceOperations {
        if let Some(hit) = self.interfaces.borrow().get(interface) {
            return hit.clone();
        }
        let result = parse_sm_interface_operations(self, interface).map(Rc::from);
        self.interfaces
            .borrow_mut()
            .insert(interface.to_owned(), result.clone());
        result
    }
}

/// How deep a document's `include::` graph is followed when its section space
/// is collected. Two levels reach the chapter's class/interface tables and the
/// tables those pull in; deeper is book boilerplate.
const INCLUDE_DEPTH: u8 = 2;

/// The `include::TARGET[]` targets of one asciidoc text, with the document
/// directory's attributes substituted where they are known.
fn include_targets(text: &str, attributes: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("include::") else {
            continue;
        };
        let Some((target, _)) = rest.split_once('[') else {
            continue;
        };
        let mut target = target.to_owned();
        for (name, value) in attributes {
            if value.is_empty() {
                continue;
            }
            target = target.replace(&format!("{{{name}}}"), value);
        }
        out.push(target);
    }
    out
}

/// The section names one asciidoc/markdown text offers: `=`/`#` headings (plus
/// the bare class/interface name of a `X Class` heading), block anchors, and
/// the labelled cells of the UML class tables (`h|*Attributes*`,
/// `|*upload_opt* (`) — the class exports carry their attribute, function and
/// invariant sections as table labels, not as headings.
fn asciidoc_section_names(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed
            .strip_prefix('=')
            .or_else(|| trimmed.strip_prefix('#'))
        {
            let heading = heading.trim_start_matches(['=', '#']).trim();
            if !heading.is_empty() {
                let name = normalize_section(heading);
                for suffix in [" class", " interface", " enumeration"] {
                    if let Some(bare) = name.strip_suffix(suffix) {
                        out.insert(bare.to_owned());
                    }
                }
                out.insert(name);
            }
        }
        if let Some(anchor) = trimmed.strip_prefix("[[").or_else(|| {
            trimmed
                .strip_prefix("[#")
                .filter(|rest| rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_'))
        }) {
            let anchor = anchor
                .split([',', ']'])
                .next()
                .unwrap_or_default()
                .trim_start_matches('_');
            if !anchor.is_empty() {
                out.insert(normalize_section(anchor));
            }
        }
        // The AM validity rules are anchored blocks, not headings:
        // `[.rule,id=VCACA]` names the section a citation addresses.
        if let Some(attributes) = trimmed.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            for attribute in attributes.split(',') {
                if let Some(id) = attribute.trim().strip_prefix("id=") {
                    let id = id.trim_matches(['"', '\'']);
                    if !id.is_empty() {
                        out.insert(normalize_section(id));
                    }
                }
            }
        }
        if let Some(label) = table_cell_label(trimmed) {
            out.insert(label);
        }
        // An asciidoc block title (`.Parser grammar`) labels the block a
        // citation addresses when the chapter has no heading for it.
        if let Some(title) = trimmed.strip_prefix('.')
            && title.starts_with(|c: char| c.is_ascii_alphabetic())
            && !title.contains('|')
        {
            out.insert(normalize_section(title));
        }
        // The ITS-REST markdown chapters carry their own title in a comment
        // line rather than a `#` heading, and a citation names that title.
        if let Some(rest) = trimmed.strip_prefix("[comment]: # (title:")
            && let Some((title, _)) = rest.split_once(')')
        {
            out.insert(normalize_section(title));
        }
    }
    out
}

/// The bolded label of an asciidoc table cell (`h|*Attributes*`,
/// `2+^h|*OBJECT_REF*`, `|*upload_opt* ( +`), normalized.
fn table_cell_label(line: &str) -> Option<String> {
    let (prefix, rest) = line.split_once("|*")?;
    if !prefix
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '^' | '<' | '>' | 'h' | 'a' | 'm' | 's'))
    {
        return None;
    }
    let label = rest.split('*').next()?.trim();
    if label.is_empty() || !label.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return None;
    }
    Some(normalize_section(label))
}

/// The declared names of an XSD — its `name="…"` values, which are exactly
/// what an ITS-XML citation addresses with `§`: a globally declared element
/// (`§composition`), a `complexType` (`§COMPOSITION`), an attribute or a
/// group. A schema is the one vendored artifact with no headings, so without
/// this its `§` citations could never resolve (issue #1833).
///
/// Deliberately a lexical scan, not an XML parse: the gate needs the set of
/// declared names, and an XSD's `name` attribute is unambiguous — nothing
/// else in the grammar spells `name="`.
fn xsd_declared_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for tail in text.split("name=").skip(1) {
        let Some(rest) = tail.strip_prefix('"').or_else(|| tail.strip_prefix('\'')) else {
            continue;
        };
        let quote = if tail.starts_with('"') { '"' } else { '\'' };
        let Some(value) = rest.split(quote).next() else {
            continue;
        };
        // A qualified name cites its local part (`openehr:COMPOSITION`).
        let local = value.rsplit(':').next().unwrap_or(value).trim();
        if !local.is_empty() {
            names.insert(normalize_section(local));
        }
    }
    names
}

/// The keys of a YAML/JSON document (the OAS operation, response and schema
/// files a citation addresses by key, e.g. `§requestBody`, `§'409'`).
fn structured_keys(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim_start().trim_start_matches("- ");
        let key = trimmed.trim_start_matches(['\'', '"']);
        let Some((key, _)) = key.split_once(':') else {
            continue;
        };
        let key = key.trim_end_matches(['\'', '"']);
        if !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '$'))
        {
            out.insert(normalize_section(key));
        }
    }
    out
}

fn sm_class_file(spec_root: &Path, interface: &str) -> PathBuf {
    spec_root
        .join("SM/docs/UML/classes")
        .join(format!("{}.adoc", interface.to_lowercase()))
}

/// Resolve an operation reference against the vendored SM, or — for a
/// RELEASED ITS-REST operation the SM does not model — against the pinned
/// [`NON_SM_REST_OPERATIONS`] table. The pseudo-interface prefix is reserved:
/// an `I_ITS_REST_*` reference that is NOT pinned is a finding, so nobody can
/// invent an anchor to dodge SM resolution.
fn resolve_sm_operation(
    op: &SmOperationRef,
    who: &str,
    spec: &SpecIndex<'_>,
    findings: &mut Vec<Finding>,
) {
    if non_sm_operation_source(op).is_some() {
        return;
    }
    if op.interface().starts_with(PSEUDO_INTERFACE_PREFIX) {
        push(
            findings,
            CheckId::SmOperation,
            who,
            format!(
                "{op} uses the reserved {PSEUDO_INTERFACE_PREFIX}* pseudo-interface but is not \
                 pinned in the NON_SM_REST_OPERATIONS table (tools/cnf-runner/src/validate.rs) — \
                 a non-SM anchor exists only for a RELEASED ITS-REST operation the SM defines no \
                 interface for, and that table is the only place one is declared"
            ),
        );
        return;
    }
    let file = sm_class_file(spec.root(), op.interface());
    match spec.read(&file) {
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

fn check_sm_operations(
    case: &CaseCore,
    who: &str,
    spec: &SpecIndex<'_>,
    findings: &mut Vec<Finding>,
) {
    let Some(anchor) = &case.sm_operation else {
        return;
    };
    resolve_sm_operation(anchor, who, spec, findings);
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
        resolve_sm_operation(&op, who, spec, findings);
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

/// Marker tokens a citation may carry between the component and its path
/// hint: they name the SOURCE the claim is grounded on, not a path segment.
/// `OAS` is the one in use — the oracle order requires an OAS-only ground to
/// be cited AS the OAS (`.claude/rules/spec-adherence.md`).
const CITATION_SOURCE_MARKERS: [&str; 1] = ["OAS"];

/// One `<COMPONENT> <path hint> §<section>` clause of a citation. A citation
/// may carry several, separated by `;` or ` + `.
#[derive(Debug)]
struct CitationClause<'a> {
    /// The component token opening the clause.
    component: &'a str,
    /// The path-hint tokens naming the document (possibly empty: a
    /// component-only citation).
    tokens: Vec<&'a str>,
    /// The `§`-introduced section names, in citation order.
    sections: Vec<&'a str>,
}

/// Whether a token can be part of a path hint (a file/directory name, or a
/// `/`-joined path). Prose stops the hint.
fn path_like(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '.' | '_' | '/' | '-' | '{' | '}' | ',' | '*' | '+')
        })
}

/// Split a citation into its component clauses. A citation cites several
/// documents by separating the clauses with `;` or ` + `; a fragment that does
/// not open with a known component is commentary on the preceding clause and
/// is dropped. A citation naming no component at all yields one clause so the
/// unknown-component finding still fires.
fn citation_clauses(citation: &str) -> Vec<CitationClause<'_>> {
    let mut clauses = Vec::new();
    for semi in citation.split(';') {
        for fragment in semi.split(" + ") {
            let fragment = fragment.trim();
            let mut words = fragment.split_whitespace();
            let Some(component) = words.next() else {
                continue;
            };
            if component_dir(component).is_none() {
                continue;
            }
            let mut tokens = Vec::new();
            for word in words {
                if word.contains('§') || !path_like(word) {
                    break;
                }
                let word = word.trim_end_matches([',', '.']);
                if word.is_empty() || CITATION_SOURCE_MARKERS.contains(&word) {
                    continue;
                }
                tokens.push(word);
            }
            let sections = fragment
                .split('§')
                .skip(1)
                .map(|rest| {
                    let rest = rest.trim_start();
                    match rest.strip_prefix('"') {
                        Some(quoted) => quoted.split('"').next().unwrap_or(quoted),
                        None => rest,
                    }
                })
                .filter(|s| !s.trim().is_empty())
                .collect();
            clauses.push(CitationClause {
                component,
                tokens,
                sections,
            });
        }
    }
    if clauses.is_empty() {
        let component = citation.split_whitespace().next().unwrap_or("");
        clauses.push(CitationClause {
            component,
            tokens: Vec::new(),
            sections: Vec::new(),
        });
    }
    clauses
}

/// Expand one `{a,b}` brace group in a path-hint token into its concrete
/// alternatives, recursing for a second group in the produced tail. A token
/// with no well-formed group passes through literally (a spaced group has
/// already been broken by the whitespace splitter, and the literal's no-match
/// finding is the honest failure).
fn expand_one_token(token: &str) -> Vec<String> {
    let Some(open) = token.find('{') else {
        return vec![token.to_owned()];
    };
    let Some(close) = token.get(open..).and_then(|rest| rest.find('}')) else {
        return vec![token.to_owned()];
    };
    let (Some(head), Some(body), Some(tail)) = (
        token.get(..open),
        token.get(open + 1..open + close),
        token.get(open + close + 1..),
    ) else {
        return vec![token.to_owned()];
    };
    body.split(',')
        .flat_map(|alternative| expand_one_token(&format!("{head}{alternative}{tail}")))
        .collect()
}

/// Expand `{a,b}` brace groups across a clause's path-hint tokens into the
/// concrete token-list variants — the authored shorthand for one clause citing
/// several sibling documents (`operations/directory_{update,delete}.yaml`).
/// Every variant must resolve on its own, so a half-phantom shorthand still
/// fails. Bounded at 32 variants: past that the literal tokens come back
/// unexpanded and fail loudly rather than exploding.
fn expand_braces(tokens: &[&str]) -> Vec<Vec<String>> {
    let mut variants: Vec<Vec<String>> = vec![Vec::new()];
    for token in tokens {
        let expansions = expand_one_token(token);
        let mut next = Vec::with_capacity(variants.len() * expansions.len());
        for variant in &variants {
            for expansion in &expansions {
                let mut grown = variant.clone();
                grown.push(expansion.clone());
                next.push(grown);
            }
        }
        if next.len() > 32 {
            return vec![tokens.iter().map(|t| (*t).to_owned()).collect()];
        }
        variants = next;
    }
    variants
}

/// The vendored documents one path hint names, or `None` when it names none.
///
/// Every token must appear in the file's component-relative path, and the LAST
/// token must name the target: the file name equals it, starts with it
/// (`master06` → `master06-change_control_package.adoc`), ends with `.`+it (the
/// `org.openehr.rm.common.locatable.adoc` UML class-export convention), or it
/// names one of the file's parent directories — a chapter directory, whose
/// every file is then a target.
fn resolve_documents(spec: &SpecIndex<'_>, dir: &Path, tokens: &[&str]) -> Vec<PathBuf> {
    let Some(last) = tokens.last() else {
        return Vec::new();
    };
    let lowered: Vec<String> = tokens.iter().map(|t| t.to_lowercase()).collect();
    let last = last.rsplit('/').next().unwrap_or(last).to_lowercase();
    let files = spec.component_files(dir);
    let mut out = Vec::new();
    for (lowered_path, path) in &files.all {
        if !lowered
            .iter()
            .all(|token| lowered_path.contains(token.as_str()))
        {
            continue;
        }
        let mut segments = lowered_path.split('/');
        let name = segments.next_back().unwrap_or_default();
        let names_file =
            name == last || name.starts_with(&last) || name.ends_with(&format!(".{last}"));
        if names_file || segments.any(|segment| segment == last) {
            out.push(path.clone());
        }
    }
    out
}

/// Normalize a section name (cited or vendored) for comparison: lowercase,
/// asciidoc emphasis and underscores flattened to spaces, quotes dropped,
/// whitespace collapsed.
fn normalize_section(text: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    for ch in text.chars() {
        match ch {
            '"' | '\'' | '‘' | '’' | '“' | '”' => {}
            '`' | '*' | '_' | '\u{a0}' => pending_space = !out.is_empty(),
            c if c.is_whitespace() => pending_space = !out.is_empty(),
            c => {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                out.extend(c.to_lowercase());
            }
        }
    }
    out
}

/// The forms a cited section may take once its trailing commentary is cut:
/// citations routinely append a parenthetical, an em-dash gloss, or a
/// qualifier after the heading proper.
fn section_candidates(section: &str) -> BTreeSet<String> {
    let full = normalize_section(section);
    let mut out = BTreeSet::new();
    let mut add = |text: &str| {
        let text = text
            .trim()
            .trim_end_matches([',', ';', '.', '/', '-', ':', ')'])
            .trim();
        if !text.is_empty() {
            out.insert(text.to_owned());
        }
    };
    for cut in [" (", " — ", " -- ", " - ", ", ", ": ", "; ", " §"] {
        if let Some((head, _)) = full.split_once(cut) {
            add(head);
        }
    }
    if let Some((head, _)) = full.split_once('.') {
        add(head);
    }
    add(&full);
    out
}

/// Whether one cited section resolves against a document's section names.
/// Equality first; a longer cited form may carry a section name inside it
/// (`I_DEFINITION_ADL14.upload_opt`), and a section name may carry the cited
/// form inside it (`OBJECT_REF Class`).
fn section_resolves(candidates: &BTreeSet<String>, names: &BTreeSet<String>) -> bool {
    candidates.iter().any(|candidate| {
        names.iter().any(|name| {
            name == candidate
                || (name.len() >= 4 && candidate.contains(name.as_str()))
                || (candidate.len() >= 3 && name.contains(candidate.as_str()))
        })
    })
}

/// Resolve every citation of one artifact against the vendored spec tree:
/// the component directory exists, the path hint names a real DOCUMENT, and
/// every `§section` names a real section of it.
fn check_citations(
    citations: &[&str],
    who: &str,
    spec: &SpecIndex<'_>,
    findings: &mut Vec<Finding>,
) {
    for citation in citations {
        if citation.trim().is_empty() {
            push(findings, CheckId::SpecRef, who, "empty spec_ref".to_owned());
            continue;
        }
        for clause in citation_clauses(citation) {
            let Some(dir) = component_dir(clause.component) else {
                push(
                    findings,
                    CheckId::SpecRef,
                    who,
                    format!("{citation:?}: unknown component {:?}", clause.component),
                );
                continue;
            };
            let roots = spec.component_roots(clause.component, dir);
            let Some(first) = roots.first() else {
                push(
                    findings,
                    CheckId::SpecRef,
                    who,
                    format!(
                        "{citation:?}: vendored component dir {} missing",
                        spec.root().join(dir).display()
                    ),
                );
                continue;
            };
            if clause.tokens.is_empty() {
                let _ = first; // component-only citation: dir existence was the check
                continue;
            }
            // Each root is resolved independently and the hits pooled: an
            // ITS-XML citation may name a docs-tree chapter OR an XSD of the
            // vendored schema bundle (issue #1833). Brace shorthands expand
            // first, and EVERY variant must resolve (issue #2545) — a
            // half-phantom `{a,b}` still fails, naming the missing variant.
            let mut documents: Vec<(&PathBuf, PathBuf)> = Vec::new();
            let mut unmatched: Option<String> = None;
            for variant in expand_braces(&clause.tokens) {
                let tokens: Vec<&str> = variant.iter().map(String::as_str).collect();
                let mut hits: Vec<(&PathBuf, PathBuf)> = Vec::new();
                for root in &roots {
                    hits.extend(
                        resolve_documents(spec, root, &tokens)
                            .into_iter()
                            .map(|document| (root, document)),
                    );
                }
                if hits.is_empty() {
                    unmatched = Some(variant.join(" "));
                    break;
                }
                documents.extend(hits);
            }
            if let Some(missing) = unmatched {
                push(
                    findings,
                    CheckId::SpecRef,
                    who,
                    format!("{citation:?}: no vendored document under {dir} matches {missing:?}"),
                );
                continue;
            }
            if clause.sections.is_empty() {
                continue;
            }
            let mut names = BTreeSet::new();
            for (root, document) in &documents {
                names.extend(spec.document_sections(root, document).iter().cloned());
            }
            for section in &clause.sections {
                if !section_resolves(&section_candidates(section), &names) {
                    push(
                        findings,
                        CheckId::SpecRef,
                        who,
                        format!(
                            "{citation:?}: the {} vendored document(s) matching {:?} carry no \
                             section matching §{}",
                            documents.len(),
                            clause.tokens.join(" "),
                            section.trim()
                        ),
                    );
                }
            }
        }
    }
}

/// Every citation a case core carries: its own `spec_refs` plus the per-row
/// grounds of its fixture set.
fn check_spec_refs(case: &CaseCore, who: &str, spec: &SpecIndex<'_>, findings: &mut Vec<Finding>) {
    let mut citations: Vec<&str> = case.spec_refs.iter().map(String::as_str).collect();
    if let Some(fixtures) = case
        .parameters
        .as_ref()
        .and_then(|p| p.fixture_set.as_ref())
    {
        citations.extend(fixtures.iter().filter_map(|f| f.spec_ref.as_deref()));
    }
    check_citations(&citations, who, spec, findings);
}

/// The one non-citation clause a binding `source` may carry: the explicit
/// spec-silence flag an `unrealized`/`extension` declaration must state
/// (`.claude/rules/spec-adherence.md` — where the specs are SILENT, flag it).
const SPEC_SILENCE_FLAG_PREFIX: &str = "no openehr spec governs";
/// The other non-citation clause: the ambiguity-register entry that
/// adjudicated the boundary (`ambiguity:` carries the id; the `source` may
/// repeat it as the closing clause of the derivation).
const REGISTER_CLAUSE_PREFIX: &str = "register amb-";

/// A binding's `unrealized.source` / `extension.source` is a DERIVATION, not
/// a sentence: what the SM defines, what the released ITS surfaces (or does
/// not), and the spec-silence flag that follows. This gate pins it to the
/// `spec_ref` clause grammar so every citation inside it is machine-resolved
/// like a case's `spec_refs`.
///
/// Two halves:
///
/// 1. **Every clause is accounted for.** [`citation_clauses`] silently DROPS
///    a fragment that does not open with a known component token — that is
///    what let these fields escape the citation gate (issue #1832): a
///    derivation written `SM x.adoc … vs ITS-REST y.yaml …` reads as ONE
///    clause, so the ITS-REST half was never checked, and a typo in a
///    component token vanished with it. Here a fragment must open with a
///    component, state the spec-silence flag, or name the register entry.
/// 2. **Every citation resolves**, via the shared [`check_citations`].
///
/// The sibling `reason` field is deliberately NOT gated: `source` and
/// `reason` are the citation/free-text SPLIT of one declaration — `source`
/// carries the citations, `reason` says in prose why the released ITS
/// surfaces no wire and what this product serves instead. Gating prose would
/// only push it back into `source`.
fn check_binding_sources(
    binding: &OperationBinding,
    who: &str,
    spec: &SpecIndex<'_>,
    findings: &mut Vec<Finding>,
) {
    let sources = [
        binding
            .unrealized
            .as_ref()
            .map(|d| ("unrealized", &d.source)),
        binding.extension.as_ref().map(|d| ("extension", &d.source)),
    ];
    for (field, source) in sources.into_iter().flatten() {
        let source = source.trim();
        if source.is_empty() {
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!("{field}.source is empty"),
            );
            continue;
        }
        for fragment in source.split(';').flat_map(|semi| semi.split(" + ")) {
            let fragment = fragment.trim();
            if fragment.is_empty() {
                continue;
            }
            let opens_component = fragment
                .split_whitespace()
                .next()
                .is_some_and(|word| component_dir(word).is_some());
            let normalized = normalize_section(fragment);
            if opens_component
                || normalized.starts_with(SPEC_SILENCE_FLAG_PREFIX)
                || normalized.starts_with(REGISTER_CLAUSE_PREFIX)
            {
                continue;
            }
            push(
                findings,
                CheckId::SpecRef,
                who,
                format!(
                    "{field}.source clause {fragment:?} opens with neither a spec component nor \
                     the spec-silence flag nor the register entry — the citation gate would drop \
                     it unread; separate the derivation's citations with `;` or ` + `"
                ),
            );
        }
        check_citations(
            &[source],
            &format!("{who} [{field}.source]"),
            spec,
            findings,
        );
    }
}

/// The ambiguity register's own `source` citations resolve (issue #2545).
///
/// A register entry is a claim that the released text is silent, and its
/// `source` field is where that claim grounds — so a phantom file, a moved
/// chapter or a misattributed section there is exactly the drift class this
/// gate exists to catch, yet the register went unchecked while every OTHER
/// artifact's citations resolved. The field-format convention (recorded on
/// the register schema): `source` splits into `;`/` + ` fragments; every
/// fragment opening with a spec component token is a CITATION CLAUSE and
/// resolves like a case `spec_ref` (document + `§` sections, the shared
/// [`check_citations`] machinery, both ITS-XML roots); any other fragment is
/// adjudication prose and passes — the register deliberately narrates its
/// verification (grep footprints, oracle-tier notes), unlike a binding
/// derivation, so the binding gate's every-clause-accounted-for rule does not
/// apply here. A source with NO citation clause at all is accused through
/// the unknown-component fallback: a silence claim must ground on at least
/// one resolvable citation.
fn check_register_sources(set: &ArtifactSet, spec: &SpecIndex<'_>, findings: &mut Vec<Finding>) {
    let Some((path, register)) = &set.register else {
        return;
    };
    let who = path.display().to_string();
    for (id, entry) in register.entries() {
        check_citations(
            &[entry.source.as_str()],
            &format!("{who} [{id}] source"),
            spec,
            findings,
        );
    }
}

/// The corpus manifest's own citations: every invalid fixture grounds its
/// defect in a spec rule, and that citation resolves like any other.
fn check_corpus_spec_refs(set: &ArtifactSet, spec: &SpecIndex<'_>, findings: &mut Vec<Finding>) {
    let Some((path, manifest)) = &set.corpus else {
        return;
    };
    let who = path.display().to_string();
    for (key, entry) in manifest.entries() {
        let Some(citation) = entry.validity.spec_ref.as_deref() else {
            continue;
        };
        check_citations(&[citation], &format!("{who} [{key}]"), spec, findings);
    }
}

// ── step arguments (no vacuous `with:` key) ─────────────────────────────────

/// The payload-role aliases [`crate::exec::driver`]'s `select_body` accepts
/// for a `Named` body beside the declared role name.
const BODY_ROLE_ALIASES: [&str; 2] = ["composition", "opt"];
/// The two keys the bundled-CONTRIBUTION construct reads when the declared
/// body role is `contribution`: the version set and the client-supplied
/// commit audit (the latter is what issue #1818 wired through).
const CONTRIBUTION_KEYS: [&str; 2] = ["versions", "audit"];

/// Every `${name}` a template addresses as a capture/handle.
fn template_capture_names(template: &crate::refgrammar::Template, into: &mut BTreeSet<String>) {
    for reference in template.refs() {
        if let ValueRef::Capture { name, .. } = reference {
            into.insert(name.to_string());
        }
    }
}

/// The same, over a whole templated value tree.
fn value_capture_names(value: &TemplatedValue, into: &mut BTreeSet<String>) {
    match value {
        TemplatedValue::Text(template) => template_capture_names(template, into),
        TemplatedValue::Seq(items) => {
            for item in items {
                value_capture_names(item, into);
            }
        }
        TemplatedValue::Map(entries) => {
            for (_, item) in entries {
                value_capture_names(item, into);
            }
        }
        TemplatedValue::Null | TemplatedValue::Bool(_) | TemplatedValue::Number(_) => {}
    }
}

/// Every `${ds:…}` a templated value tree addresses.
fn value_data_set_refs(value: &TemplatedValue, into: &mut BTreeSet<String>) {
    match value {
        TemplatedValue::Text(template) => {
            for reference in template.refs() {
                if let ValueRef::DataSet { key, .. } = reference {
                    into.insert(key.to_string());
                }
            }
        }
        TemplatedValue::Seq(items) => {
            for item in items {
                value_data_set_refs(item, into);
            }
        }
        TemplatedValue::Map(entries) => {
            for (_, item) in entries {
                value_data_set_refs(item, into);
            }
        }
        TemplatedValue::Null | TemplatedValue::Bool(_) | TemplatedValue::Number(_) => {}
    }
}

/// Whether a `with:` value could resolve to the single-payload body the
/// `Named` role's fallback picks.
///
/// Mirrors `select_body`'s own test on the RESOLVED value — an object, an
/// array, or (for a `*_text` role, whose payload IS a string) any string.
/// Statically, a reference-bearing string may resolve to either: a
/// `${ds:…}` resolves to the corpus entry's parsed JSON, a capture to
/// whatever was captured.
fn could_be_payload(value: &TemplatedValue, text_role: bool) -> bool {
    match value {
        TemplatedValue::Seq(_) | TemplatedValue::Map(_) => true,
        TemplatedValue::Text(template) => {
            text_role
                || template.refs().any(|r| {
                    matches!(
                        r,
                        ValueRef::DataSet { .. }
                            | ValueRef::FixtureDataSet
                            | ValueRef::Recipe(_)
                            | ValueRef::Capture { .. }
                            | ValueRef::Row(_)
                            | ValueRef::Fixture(_)
                    )
                })
        }
        TemplatedValue::Null | TemplatedValue::Bool(_) | TemplatedValue::Number(_) => false,
    }
}

/// The `with:` keys the driver would READ when it drives `step` through
/// `binding` — the union of every consumption path in
/// [`crate::exec::driver`].
fn consumed_with_keys(
    set: &ArtifactSet,
    binding: &OperationBinding,
    step: &FlowStep,
) -> BTreeSet<String> {
    let mut consumed = BTreeSet::new();
    // `auto_variant`: a sibling binding named `with_<p>` is selected BY the
    // step binding `<p>` non-null, so `<p>` is read even when the selected
    // binding's own request never names it.
    for (_, sibling) in &set.bindings {
        if sibling.sm_operation == binding.sm_operation
            && let Some(param) = sibling
                .variant
                .as_deref()
                .and_then(|v| v.strip_prefix("with_"))
        {
            consumed.insert(param.to_owned());
        }
    }
    let Some(request) = &binding.request else {
        return consumed;
    };
    // `build_url`: path params, declared query names, and every capture name
    // a query template addresses (scalars are promoted by `merge_with_vars`).
    for param in request.path.params() {
        consumed.insert(param.to_string());
    }
    for (name, value) in request.query.iter().flatten() {
        consumed.insert(name.clone());
        for template in value.templates() {
            template_capture_names(template, &mut consumed);
        }
    }
    // `compose_headers` resolves header templates over the same merged vars.
    for (_, template) in request.headers.iter().flatten() {
        template_capture_names(template, &mut consumed);
    }
    // A `required` format header (`openehr-template-id`) takes its value from
    // the step's own `${ds:…}` argument — the committed data set's
    // manifest-declared template identity — so a data-set key is read there
    // even when the request declares no body of its own.
    let requires_template_id = binding.format_headers.iter().flatten().any(|(_, map)| {
        map.0
            .iter()
            .any(|(_, req)| matches!(req, crate::model::binding::FormatHeaderReq::Required))
    });
    if requires_template_id {
        for (key, value) in step.with_entries() {
            let mut refs = BTreeSet::new();
            value_data_set_refs(value, &mut refs);
            if !refs.is_empty() {
                consumed.insert(key.clone());
            }
        }
    }
    // `select_body`.
    match &request.body {
        None => {}
        Some(crate::model::binding::RequestBody::Named { name, .. }) => {
            let authored = |key: &str| step.with_entries().iter().any(|(k, _)| k == key);
            // `select_body` resolves the payload in a fixed ORDER, and each arm
            // short-circuits the ones after it. Modelling that order is what makes
            // this gate sharp: once an earlier arm answers, the later arms read
            // nothing, so a key they might have picked up is genuinely unread.
            //
            // 1. the bundled-CONTRIBUTION construct, when `versions:` is authored:
            //    the envelope is built from `versions` + `audit` and NOTHING else.
            if name == "contribution" && authored("versions") {
                consumed.extend(CONTRIBUTION_KEYS.iter().map(|k| (*k).to_owned()));
            }
            // 2. the declared role name, then the two aliases, in that order.
            else if let Some(hit) = std::iter::once(name.as_str())
                .chain(BODY_ROLE_ALIASES)
                .find(|key| authored(key))
            {
                consumed.insert(hit.to_owned());
            }
            // 3. only with none of those authored does the single-payload
            //    scan run. It picks ONE key, but WHICH one is a runtime
            //    property (the resolved values' JSON shapes, in the driver's
            //    `BTreeMap` order), so every key that could be it counts —
            //    an accusation must be certain.
            else {
                let text_role = name.ends_with("_text");
                for (key, value) in step.with_entries() {
                    if could_be_payload(value, text_role) {
                        consumed.insert(key.clone());
                    }
                }
            }
        }
        Some(crate::model::binding::RequestBody::Structured(template)) => {
            value_capture_names(template, &mut consumed);
        }
        Some(crate::model::binding::RequestBody::Patched { from_capture, set }) => {
            consumed.insert(from_capture.to_string());
            for (_, value) in set {
                if let Ok(value) = TemplatedValue::from_value(value) {
                    value_capture_names(&value, &mut consumed);
                }
            }
        }
    }
    consumed
}

/// The case whose FLOW the step-level gates judge.
///
/// An authored flow is judged as written. A CONTENT case authors no flow at
/// all — `crate::run::synthesize_content_case` turns its decision table into
/// the generate→commit→expect flow the driver actually runs — so the gates
/// judge that synthesis instead of skipping the case, which is how a content
/// case's one driven step used to escape both of them (issue #1903).
fn driven_case(case: &CaseCore) -> std::borrow::Cow<'_, CaseCore> {
    if matches!(case.kind, CaseKind::Content) {
        std::borrow::Cow::Owned(crate::run::synthesize_content_case(case))
    } else {
        std::borrow::Cow::Borrowed(case)
    }
}

/// The binding `crate::exec::driver` would select for `step`, mirroring its
/// variant resolution: an explicitly named variant when one exists, otherwise
/// the bare binding.
fn step_binding<'a>(
    set: &'a ArtifactSet,
    anchor: &SmOperationRef,
    step: &FlowStep,
) -> Option<&'a OperationBinding> {
    let op = if step.call.contains('.') {
        SmOperationRef::parse(&step.call).ok()?
    } else {
        anchor.sibling(&step.call)
    };
    let mut bindings: Vec<&OperationBinding> = set
        .bindings
        .iter()
        .map(|(_, b)| b)
        .filter(|b| b.sm_operation == op)
        .collect();
    if let Some(v) = &step.variant
        && bindings
            .iter()
            .any(|b| b.variant.as_deref() == Some(v.as_str()))
    {
        bindings.retain(|b| b.variant.as_deref() == Some(v.as_str()));
    } else if bindings.iter().any(|b| b.variant.is_none()) {
        bindings.retain(|b| b.variant.is_none());
    }
    bindings.first().copied()
}

/// Every `pattern:` header matcher a driven step could be judged by resolves
/// (issue #1852).
///
/// `crate::exec::headers::resolve_placeholders` substitutes a `<name>`
/// placeholder from exactly two sources: the closed STRUCTURAL vocabulary
/// (`crate::exec::headers::structural_token`, a released grammar per name),
/// and the step's template scope — `requires`-minted handles, the captures
/// earlier steps declared, and the step's own `with:` arguments
/// (`crate::exec::driver`'s `merge_with_vars`). A name in neither is a hard
/// refusal at drive time, which books the row as a conformance FAILURE of the
/// SUT even though nothing about the response was inspected. The parse-time
/// probe cannot see this: it only compiles the pattern with every placeholder
/// wildcarded, so the whole class was run-time-only.
///
/// Two deliberate boundaries, because an accusation must be certain. The gate
/// judges the flow the driver runs ([`driven_case`], so content cases are
/// included) but not provisioning, which drives no matcher. And it counts a
/// `with:` key by NAME, though the driver promotes only scalar-valued ones,
/// since which value an argument resolves to is a runtime property.
fn check_matcher_placeholders(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    for (_, authored) in &set.cases {
        let driven = driven_case(authored);
        let case = driven.as_ref();
        let Some(anchor) = &case.sm_operation else {
            continue;
        };
        let who = case.id.to_string();
        let mut captured: BTreeSet<String> = case
            .requires
            .minted_handles()
            .iter()
            .map(ToString::to_string)
            .collect();
        for step in &case.flow {
            if let Some(binding) = step_binding(set, anchor, step)
                && !binding.is_unrealized()
            {
                let mut in_scope = captured.clone();
                in_scope.extend(step.with_entries().iter().map(|(key, _)| key.clone()));
                // One accusation per (step, outcome, placeholder): a fixture
                // set or matrix that expects the same kind on many rows
                // observes it once.
                let kinds: BTreeSet<OutcomeKind> =
                    step_observable_kinds(case, step).into_iter().collect();
                for kind in kinds {
                    let Some(expectation) = binding.outcome(kind) else {
                        continue; // reported by binding-completeness
                    };
                    for (header, declared) in expectation.headers.iter().flatten() {
                        let HeaderMatcher::Pattern(pattern) = &declared.matcher else {
                            continue;
                        };
                        for name in placeholder_names(pattern) {
                            if structural_token(name).is_some() || in_scope.contains(name) {
                                continue;
                            }
                            push(
                                findings,
                                CheckId::MatcherPlaceholder,
                                &who,
                                format!(
                                    "step {}: the {} `{}` outcome matches header {header} with \
                                     <{name}>, which is neither a structural token nor in the \
                                     template scope at that step (requires-minted handles + \
                                     earlier captures + this step's `with:` arguments) — the \
                                     matcher refuses instead of judging, so the row reddens on \
                                     the runner's own resolution failure",
                                    step.step,
                                    binding.sm_operation,
                                    kind.token(),
                                ),
                            );
                        }
                    }
                }
            }
            for (name, _source) in step.captures() {
                captured.insert(name.to_string());
            }
        }
    }
}

/// No flow step authors a `with:` key its binding never reads (issue #1830).
///
/// A `with:` key is an ARGUMENT, and an argument the request form does not
/// consume never leaves the runner: the SUT is driven exactly as if the key
/// were absent. That is worse than a no-op — it makes the case ASSERT
/// vacuously about an input it never sent. The live instance was
/// `SEC-AUDIT_ACCOUNTABILITY-server_set_commit_audit`, whose deliberately
/// ancient client-supplied `audit.time_committed` was dropped by the driver,
/// so its `not_equals` assertion could not have failed however the server
/// behaved.
///
/// The consumption model is [`consumed_with_keys`] — the union of every path
/// `crate::exec::driver` reads a key by. It is deliberately GENEROUS where a
/// path's choice is a runtime property (the single-payload body fallback),
/// because an accusation must be certain: a key this gate names is one no
/// request form can reach.
///
/// A step whose binding is `unrealized` is skipped — nothing is driven, so
/// nothing can be vacuous; its case is not-applicable with the citation.
fn check_step_arguments(set: &ArtifactSet, findings: &mut Vec<Finding>) {
    for (_, authored) in &set.cases {
        let driven = driven_case(authored);
        let case = driven.as_ref();
        let Some(anchor) = &case.sm_operation else {
            continue;
        };
        let who = case.id.to_string();
        for step in &case.flow {
            if step.with_entries().is_empty() {
                continue;
            }
            let Some(binding) = step_binding(set, anchor, step) else {
                continue; // reported by the SM / binding-completeness checks
            };
            if binding.is_unrealized() {
                continue;
            }
            let consumed = consumed_with_keys(set, binding, step);
            for (key, _) in step.with_entries() {
                if consumed.contains(key) {
                    continue;
                }
                push(
                    findings,
                    CheckId::StepArguments,
                    &who,
                    format!(
                        "step {}: `with.{key}` is authored but {} reads it on no request path \
                         (not a path param, query parameter, header/body template reference, \
                         payload role or variant selector) — the SUT is driven as if the key \
                         were absent, so anything the case asserts about it passes vacuously",
                        step.step, binding.sm_operation
                    ),
                );
            }
        }
    }
}

// ── binding completeness ────────────────────────────────────────────────────

/// Kinds a step may observe: the fixed expectation, every fixture-set
/// `expected` kind when per-fixture, plus any matrix `expected` column kinds.
fn step_observable_kinds(case: &CaseCore, step: &FlowStep) -> Vec<OutcomeKind> {
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

// ── binding filename ↔ declared identity ────────────────────────────────────

/// The file name a binding's declared identity requires:
/// `<sm_operation>[-<variant>].yaml`.
///
/// Selection is by the DECLARED `sm_operation` + `variant`, so a disagreeing
/// file name misleads nobody at run time — and exactly for that reason it
/// drifts silently: a grep for `-<variant>` misses the file that declares it,
/// and a reader reasons about the wrong realization. The name is therefore
/// gated, not merely conventional.
fn expected_binding_stem(binding: &OperationBinding) -> String {
    match &binding.variant {
        Some(variant) => format!("{}-{variant}", binding.sm_operation),
        None => binding.sm_operation.to_string(),
    }
}

fn check_binding_filename(path: &Path, binding: &OperationBinding, findings: &mut Vec<Finding>) {
    let who = path.display().to_string();
    let expected = expected_binding_stem(binding);
    match path.file_stem().and_then(std::ffi::OsStr::to_str) {
        None => push(
            findings,
            CheckId::BindingFilename,
            &who,
            format!("binding file has no readable stem; expected {expected}.yaml"),
        ),
        Some(stem) if stem != expected => push(
            findings,
            CheckId::BindingFilename,
            &who,
            format!(
                "file stem {stem:?} disagrees with the declared identity \
                 (sm_operation {}{}) — rename the file to {expected}.yaml",
                binding.sm_operation,
                binding
                    .variant
                    .as_deref()
                    .map_or(String::new(), |v| format!(", variant {v:?}")),
            ),
        ),
        Some(_) => {}
    }
}

#[cfg(test)]
mod binding_filename_tests {
    use super::*;

    fn binding(variant: Option<&str>) -> OperationBinding {
        let mut doc = serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr" },
            "outcomes": { "created": { "status": 201 } }
        });
        if let (Some(variant), Some(map)) = (variant, doc.as_object_mut()) {
            map.insert("variant".to_owned(), serde_json::json!(variant));
        }
        serde_json::from_value(doc).unwrap()
    }

    fn findings_for(file: &str, variant: Option<&str>) -> Vec<Finding> {
        let mut findings = Vec::new();
        check_binding_filename(
            &PathBuf::from("bindings/its-rest").join(file),
            &binding(variant),
            &mut findings,
        );
        findings
    }

    #[test]
    fn variant_less_binding_is_named_after_its_operation() {
        assert!(findings_for("I_EHR_SERVICE.create_ehr.yaml", None).is_empty());
        let findings = findings_for("create_ehr.yaml", None);
        assert_eq!(findings.len(), 1);
        let finding = findings.first().unwrap();
        assert_eq!(finding.check, CheckId::BindingFilename);
        assert!(
            finding
                .message
                .contains("rename the file to I_EHR_SERVICE.create_ehr.yaml"),
            "{}",
            finding.message
        );
    }

    #[test]
    fn variant_binding_carries_its_variant_in_the_stem() {
        assert!(
            findings_for(
                "I_EHR_SERVICE.create_ehr-with_ehr_id.yaml",
                Some("with_ehr_id")
            )
            .is_empty()
        );
        // The exact drift #558 caught: an abbreviated stem for a longer
        // declared variant.
        let findings = findings_for("I_EHR_SERVICE.create_ehr-with_id.yaml", Some("with_ehr_id"));
        assert_eq!(findings.len(), 1);
        let finding = findings.first().unwrap();
        assert!(finding.message.contains("with_id"), "{}", finding.message);
        assert!(
            finding
                .message
                .contains("rename the file to I_EHR_SERVICE.create_ehr-with_ehr_id.yaml"),
            "{}",
            finding.message
        );
        // A variant-less file for a variant-declaring binding is caught too.
        assert_eq!(
            findings_for("I_EHR_SERVICE.create_ehr.yaml", Some("with_ehr_id")).len(),
            1
        );
    }

    #[test]
    fn expected_stem_is_the_declared_identity() {
        assert_eq!(
            expected_binding_stem(&binding(None)),
            "I_EHR_SERVICE.create_ehr"
        );
        assert_eq!(
            expected_binding_stem(&binding(Some("wrong_media_type"))),
            "I_EHR_SERVICE.create_ehr-wrong_media_type"
        );
    }
}

#[cfg(test)]
mod matcher_placeholder_tests {
    use super::*;

    /// A create binding whose `created` `ETag` matcher names `<{placeholder}>`.
    fn world(placeholder: &str, capture_on_step_one: bool) -> ArtifactSet {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_COMPOSITION.create_composition",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr/{ehr_id}/composition", "body": "composition" },
            "outcomes": {
                "created": {
                    "status": 201,
                    "headers": { "ETag": format!("pattern:W/\"<{placeholder}>::<system_id>::1\"") }
                }
            },
            "captures": { "versioned_object_uid": { "from": "header ETag", "strip": "weak-quotes" } }
        }))
        .unwrap();
        let mut step = serde_json::json!({
            "step": 1,
            "call": "create_composition",
            "with": { "ehr_id": "${ehr_id}", "composition": "${ds:cnf.x}" },
            "expect": "created"
        });
        if capture_on_step_one && let Some(map) = step.as_object_mut() {
            map.insert(
                "capture".to_owned(),
                serde_json::json!({ placeholder: format!("created.{placeholder}") }),
            );
        }
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-matcher", "kind": "functional", "component": "EHR_COMPOSITION",
            "sm_operation": "I_EHR_COMPOSITION.create_composition",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "requires": { "ehr": { "commits": "none" } },
            "flow": [step]
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings.push((PathBuf::from("b.yaml"), binding));
        set.cases.push((PathBuf::from("c.yaml"), case));
        set
    }

    fn findings_for(placeholder: &str, capture_on_step_one: bool) -> Vec<Finding> {
        let mut findings = Vec::new();
        check_matcher_placeholders(&world(placeholder, capture_on_step_one), &mut findings);
        findings
    }

    /// The #1852 vocabulary gap, at VALIDATE time: a placeholder that is
    /// neither a structural token nor in the step's template scope guarantees
    /// a refused matcher at drive time, and the parse-time compile probe
    /// (which wildcards every placeholder) cannot see it.
    #[test]
    fn an_unresolvable_placeholder_is_a_finding() {
        let findings = findings_for("no_such_name", false);
        assert_eq!(findings.len(), 1, "{findings:?}");
        let finding = findings.first().unwrap();
        assert_eq!(finding.check, CheckId::MatcherPlaceholder);
        assert!(
            finding.message.contains("<no_such_name>"),
            "{}",
            finding.message
        );
    }

    /// A capture declared on the SAME step is bound only after the response
    /// arrives, so it is not in the matcher's scope — the exact shape the
    /// create-time `ETag` matchers had before `versioned_object_uid` became a
    /// structural token.
    #[test]
    fn a_same_step_capture_does_not_put_a_name_in_scope() {
        assert_eq!(findings_for("no_such_name", true).len(), 1);
    }

    /// The two names the vocabulary gained resolve structurally, so the
    /// server-assigned container id and the resolved HRID need no capture.
    #[test]
    fn structural_tokens_need_no_capture() {
        assert!(findings_for("versioned_object_uid", false).is_empty());
        assert!(findings_for("template_hrid", false).is_empty());
    }

    /// A `requires`-minted handle and a `with:` argument are both in scope —
    /// the identity names (`ehr_id`, `contribution_uid`) the catalogue pins
    /// against the request are not accused.
    #[test]
    fn minted_handles_and_step_arguments_are_in_scope() {
        assert!(findings_for("ehr_id", false).is_empty());
        assert!(findings_for("composition", false).is_empty());
    }

    /// A CONTENT case authors no flow — the driver runs
    /// `crate::run::synthesize_content_case`'s generate→commit→expect flow —
    /// so both step-level gates judge THAT (issue #1903). `content_path` is
    /// the create binding's request path: dropping `{ehr_id}` from it makes
    /// the synthesized `with.ehr_id` unread, which is the `step-arguments`
    /// half of the same escape.
    fn content_world(placeholder: &str, content_path: &str) -> ArtifactSet {
        let binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_COMPOSITION.create_composition",
            "its": "its-rest",
            "request": { "method": "POST", "path": content_path, "body": "composition" },
            "outcomes": {
                "created": {
                    "status": 201,
                    "headers": { "ETag": format!("pattern:W/\"<{placeholder}>::<system_id>::1\"") }
                },
                "validation_failed": { "status": 422 }
            }
        }))
        .unwrap();
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "CONT-X-validate_open", "kind": "content", "component": "CONTENT",
            "rm_class": "DV_COUNT",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "constraint_context": {
                "template": "cnf.content.minimal",
                "path": "/content[at0001]",
                "constraint_columns": []
            },
            "decision_table": {
                "columns": ["magnitude", "expected", "violates"],
                "rows": [[1, "accepted", []]]
            }
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings.push((PathBuf::from("b.yaml"), binding));
        set.cases.push((PathBuf::from("c.yaml"), case));
        set
    }

    #[test]
    fn a_content_cases_synthesized_flow_is_judged_by_both_step_gates() {
        let clean = content_world("versioned_object_uid", "/ehr/{ehr_id}/composition");
        let mut findings = Vec::new();
        check_matcher_placeholders(&clean, &mut findings);
        check_step_arguments(&clean, &mut findings);
        assert!(findings.is_empty(), "{findings:?}");

        // The matcher half: an unresolvable placeholder on the outcome the
        // synthesized step expects.
        let mut findings = Vec::new();
        check_matcher_placeholders(
            &content_world("no_such_name", "/ehr/{ehr_id}/composition"),
            &mut findings,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        let finding = findings.first().unwrap();
        assert_eq!(finding.check, CheckId::MatcherPlaceholder);
        assert!(
            finding.message.contains("<no_such_name>"),
            "{}",
            finding.message
        );

        // The argument half: the synthesized `with.ehr_id` reaches no request
        // form once the path stops naming it.
        let mut findings = Vec::new();
        check_step_arguments(
            &content_world("versioned_object_uid", "/composition"),
            &mut findings,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        let finding = findings.first().unwrap();
        assert_eq!(finding.check, CheckId::StepArguments);
        assert!(
            finding.message.contains("`with.ehr_id`"),
            "{}",
            finding.message
        );
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
        // A DECLARED view must have a REGISTERED evaluator (#971): the
        // resolver errors on an unregistered view only at RUN time, so the
        // cross-check lives here where a dead declaration fails before any
        // SUT is composed.
        for (view, _) in entry.views.iter().flatten() {
            if !crate::exec::resolve::Resolver::REGISTERED_VIEWS.contains(&view.as_str()) {
                push(
                    findings,
                    CheckId::CorpusIntegrity,
                    &who,
                    format!("{key}: view {view} has no registered evaluator (exec::resolve)"),
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
    if let Some((path, matrix)) = &set.matrix {
        let who = path.display().to_string();
        for message in matrix
            .check_tier_scoping()
            .err()
            .into_iter()
            .chain(matrix.check_realization_scoping().err())
            .flatten()
        {
            push(findings, CheckId::VocabDrift, &who, message);
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
/// the vendored OAS (owner ruling 2026-07-24: the OAS is `emit-rest` codegen
/// input, never a surface source). Every
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
/// operations and are excluded by `sm_interface_operations`.
///
/// This table is the SM half of the Axis-1 domain only. A RELEASED ITS-REST
/// operation the SM defines no interface for is invisible to it by
/// construction, so it is enumerated by the second, ITS-side table
/// [`NON_SM_REST_OPERATIONS`].
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

/// The reserved interface prefix a non-SM ITS-REST anchor uses. It is a
/// CATALOGUE naming convention and never a claim that the SM defines the
/// operation — the same distinction `AMB-127` draws for variant anchoring.
const PSEUDO_INTERFACE_PREFIX: &str = "I_ITS_REST_";

/// RELEASED ITS-REST operations with NO SM interface — pinned from the
/// released ITS-REST sources (docs text first; the released operation files
/// where the docs text is silent, per the oracle order). The catalogue anchors
/// each under a reserved `I_ITS_REST_*` pseudo-interface, ONE per released
/// resource family (a catalogue naming convention, never an SM claim — the
/// same convention AMB-127 pins for variant anchoring, adjudicated for this
/// table in AMB-161).
///
/// Four families are pinned today, and the operation name of every row is the
/// RELEASED `operationId` verbatim, so a row is traceable to exactly one
/// vendored `specifications/operations/<operationId>.yaml`. A family is the
/// SERVED RESOURCE, not the OAS tag — which is why the demographic
/// revision-history route sits with its EHR-side twins rather than with the
/// container read that shares its tag:
///
/// - `I_ITS_REST_SYSTEM` — the System API (ITS-REST
///   `docs/system/Description.md`, STABLE) defines `OPTIONS {base_path}`
///   (overview `Requests_and_responses.md` §HTTP Methods).
/// - `I_ITS_REST_ITEM_TAGS` — the 23 released `ITEM_TAG` routes: the two
///   space-wide lists, the EHR-side `COMPOSITION/EHR_STATUS` triples, and the
///   five demographic party triples. The SM models no tag concept at all —
///   `docs/specs/openehr/SM/docs/` contains zero occurrences of "tag"
///   (grep-verified) — while the released ITS-REST calls the
///   `openehr-item-tag` / `openehr-version-item-tag` headers "convenient
///   wrappers around the dedicated `ITEM_TAG` operations" (overview
///   `Requests_and_responses.md` §openehr-item-tag and
///   openehr-version-item-tag), so the operations are unambiguously part of
///   the released wire with no service-model anchor to name them by.
/// - `I_ITS_REST_REVISION_HISTORY` — the three released revision-history
///   reads (COMPOSITION, `EHR_STATUS`, PARTY). The SM declares no
///   revision-history operation on any interface —
///   `docs/specs/openehr/SM/docs/` contains zero occurrences of
///   "`revision_history`" (grep-verified); the abstract counterpart lives in
///   the RM (`common` `versioned_object.adoc` §Functions
///   `revision_history`), which is a model, not a service interface.
/// - `I_ITS_REST_VERSIONED_PARTY` — the `VERSIONED_PARTY` container read. Its
///   two EHR-side twins DO have SM anchors
///   (`I_EHR_COMPOSITION.get_versioned_composition`,
///   `I_EHR_STATUS.get_versioned_ehr_status`); `I_PARTY` declares no
///   container read at all, and SM
///   `docs/openehr_platform/master06-demographic_service.adoc` includes no
///   versioned-party interface (register AMB-136).
///
/// Without this table the Axis-1 enumeration is structurally blind to such an
/// operation: [`check_surface_sm_operations`] walks SM class exports, so a
/// wire behaviour the SM never models could never be reported missing.
const NON_SM_REST_OPERATIONS: &[(&str, &str)] = &[
    (
        "I_ITS_REST_SYSTEM.options",
        "ITS-REST docs/system/Description.md (STABLE System API) + overview \
         Requests_and_responses.md §HTTP Methods (OPTIONS)",
    ),
    // ── SMART on openEHR: the Platform's service-discovery document ──
    (
        "I_ITS_REST_SMART.discovery",
        "ITS-REST docs/smart_app_launch/master04-service_discovery.adoc §Service Discovery — \
         \"The configuration endpoint should be always available relative to the _Platform_ base \
         URL\", served as `application/json`, its `services` map carrying at minimum \
         `org.openehr.rest` with an absolute `baseUrl` (§Services). The SM models no Platform \
         interface (the Platform is \"a software ecosystem comprising at minimum an Authorization \
         Server, an openEHR Clinical Data Repository (CDR), and a FHIR Server\" — \
         master02-overview.adoc §Glossary — not an SM service), and SMART is the one API area of \
         the release with no OpenAPI group, so the operation is enumerable only from this table",
    ),
    // ── ITEM_TAG: the two space-wide lists ──
    (
        "I_ITS_REST_ITEM_TAGS.ehr_tags_get",
        "ITS-REST specifications/operations/ehr_tags_get.yaml — GET \
         /ehr/{ehr_id}/tags, the EHR-scoped list of \"the ITEM_TAG resources \
         associated with any target VERSION or VERSIONED_OBJECT within the EHR \
         identified by ehr_id\"",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.demographic_tags_get",
        "ITS-REST specifications/operations/demographic_tags_get.yaml — GET \
         /demographic/tags, the space-wide list of \"the ITEM_TAG resources \
         associated with any target VERSION or VERSIONED_PARTY within the \
         Demographic space\" (its unbounded scope is adjudicated in AMB-138)",
    ),
    // ── ITEM_TAG: the EHR-side typed families ──
    (
        "I_ITS_REST_ITEM_TAGS.composition_tags_get",
        "ITS-REST specifications/operations/composition_tags_get.yaml — GET \
         /ehr/{ehr_id}/composition/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.composition_tags_update",
        "ITS-REST specifications/operations/composition_tags_update.yaml — PUT \
         /ehr/{ehr_id}/composition/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.composition_tags_delete",
        "ITS-REST specifications/operations/composition_tags_delete.yaml — \
         DELETE /ehr/{ehr_id}/composition/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.ehr_status_tags_get",
        "ITS-REST specifications/operations/ehr_status_tags_get.yaml — GET \
         /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.ehr_status_tags_update",
        "ITS-REST specifications/operations/ehr_status_tags_update.yaml — PUT \
         /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.ehr_status_tags_delete",
        "ITS-REST specifications/operations/ehr_status_tags_delete.yaml — \
         DELETE /ehr/{ehr_id}/ehr_status/{uid_based_id}/tags/{key}",
    ),
    // ── ITEM_TAG: the five demographic party families ──
    (
        "I_ITS_REST_ITEM_TAGS.person_tags_get",
        "ITS-REST specifications/operations/person_tags_get.yaml — GET \
         /demographic/person/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.person_tags_update",
        "ITS-REST specifications/operations/person_tags_update.yaml — PUT \
         /demographic/person/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.person_tags_delete",
        "ITS-REST specifications/operations/person_tags_delete.yaml — DELETE \
         /demographic/person/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.agent_tags_get",
        "ITS-REST specifications/operations/agent_tags_get.yaml — GET \
         /demographic/agent/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.agent_tags_update",
        "ITS-REST specifications/operations/agent_tags_update.yaml — PUT \
         /demographic/agent/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.agent_tags_delete",
        "ITS-REST specifications/operations/agent_tags_delete.yaml — DELETE \
         /demographic/agent/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.group_tags_get",
        "ITS-REST specifications/operations/group_tags_get.yaml — GET \
         /demographic/group/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.group_tags_update",
        "ITS-REST specifications/operations/group_tags_update.yaml — PUT \
         /demographic/group/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.group_tags_delete",
        "ITS-REST specifications/operations/group_tags_delete.yaml — DELETE \
         /demographic/group/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.organisation_tags_get",
        "ITS-REST specifications/operations/organisation_tags_get.yaml — GET \
         /demographic/organisation/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.organisation_tags_update",
        "ITS-REST specifications/operations/organisation_tags_update.yaml — PUT \
         /demographic/organisation/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.organisation_tags_delete",
        "ITS-REST specifications/operations/organisation_tags_delete.yaml — \
         DELETE /demographic/organisation/{uid_based_id}/tags/{key}",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.role_tags_get",
        "ITS-REST specifications/operations/role_tags_get.yaml — GET \
         /demographic/role/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.role_tags_update",
        "ITS-REST specifications/operations/role_tags_update.yaml — PUT \
         /demographic/role/{uid_based_id}/tags",
    ),
    (
        "I_ITS_REST_ITEM_TAGS.role_tags_delete",
        "ITS-REST specifications/operations/role_tags_delete.yaml — DELETE \
         /demographic/role/{uid_based_id}/tags/{key}",
    ),
    // ── REVISION_HISTORY: the three released revision-history reads ──
    (
        "I_ITS_REST_REVISION_HISTORY.versioned_composition_revision_history",
        "ITS-REST specifications/operations/versioned_composition_revision_history.yaml \
         — GET /ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/revision_history, \
         \"Retrieves revision history of the VERSIONED_COMPOSITION identified by \
         `versioned_object_uid` and associated with the EHR identified by `ehr_id`\"",
    ),
    (
        "I_ITS_REST_REVISION_HISTORY.versioned_ehr_status_revision_history",
        "ITS-REST specifications/operations/versioned_ehr_status_revision_history.yaml \
         — GET /ehr/{ehr_id}/versioned_ehr_status/revision_history, \"Retrieves \
         revision history of the VERSIONED_EHR_STATUS associated with the EHR \
         identified by `ehr_id`\"",
    ),
    (
        "I_ITS_REST_REVISION_HISTORY.versioned_party_revision_history",
        "ITS-REST specifications/operations/versioned_party_revision_history.yaml — \
         GET /demographic/versioned_party/{versioned_object_uid}/revision_history, \
         \"Retrieves revision history of the VERSIONED_PARTY identified by \
         `versioned_object_uid`\"",
    ),
    // ── VERSIONED_PARTY: the container read the SM models no interface for ──
    (
        "I_ITS_REST_VERSIONED_PARTY.versioned_party_get",
        "ITS-REST specifications/operations/versioned_party_get.yaml — GET \
         /demographic/versioned_party/{versioned_object_uid}, \"Retrieves a \
         VERSIONED_PARTY identified by `versioned_object_uid`\" (register AMB-136: \
         I_PARTY declares no container read)",
    ),
];

/// The pinned citation of a non-SM ITS-REST operation, or `None` when the
/// reference is not in [`NON_SM_REST_OPERATIONS`].
fn non_sm_operation_source(op: &SmOperationRef) -> Option<&'static str> {
    let reference = op.to_string();
    NON_SM_REST_OPERATIONS
        .iter()
        .find(|(name, _)| *name == reference)
        .map(|(_, source)| *source)
}

/// Parse the service operations of an SM interface from its vendored UML class
/// export — the same table shape [`resolve_sm_operation`] resolves against.
/// Operation rows are `|*<name>* (` (a lower-snake signature name followed by
/// its parameter list); sub-interface navigation accessors (`i_*`) are
/// excluded (they return an interface, not a service result).
///
/// This reads the SM only. An ITS-REST operation with no SM interface has no
/// class export to parse and is enumerated from [`NON_SM_REST_OPERATIONS`]
/// instead — never through this function.
///
/// Callers go through [`SpecIndex::interface_operations`], which parses each
/// interface once per run.
///
/// # Errors
/// Returns a message when the interface has no vendored class export.
fn parse_sm_interface_operations(
    spec: &SpecIndex<'_>,
    interface: &str,
) -> Result<Vec<String>, String> {
    let file = sm_class_file(spec.root(), interface);
    let text = spec.read(&file).map_err(|error| {
        format!(
            "interface {interface} has no vendored SM class export ({}): {error}",
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

/// The three coverage axes (Axis 1 and the Axis-3 section derivation need the
/// vendored spec tree; the rest is pure over the artifact set). An absent
/// `wire_surface.yaml` is treated as an empty register — so every gap surfaces
/// as a finding rather than passing silently.
fn check_surface_coverage(
    set: &ArtifactSet,
    spec: Option<&SpecIndex<'_>>,
    findings: &mut Vec<Finding>,
) {
    let empty = WireSurface::default();
    let wire_surface = set.wire_surface.as_ref().map_or(&empty, |(_, w)| w);
    if let Some(spec) = spec {
        check_surface_sm_operations(set, spec, wire_surface, findings);
        check_axis3_section_derivation(wire_surface, spec, AXIS3_SECTION_EXCLUSIONS, findings);
    }
    check_binding_branch_coverage(set, wire_surface, findings);
    check_wire_surface_elements(set, wire_surface, findings);
    check_served_extensions(set, wire_surface, findings);
}

/// Axis 4 — the outward declaration is well-formed and does not claim a route
/// the RELEASED wire already defines.
///
/// This check reads the axis; it never derives an obligation FROM it. Nothing
/// here can require a case, expect a branch, or move a verdict — the axis is a
/// declaration, and `never_gates: true` on every entry (shape-checked by
/// [`WireSurface::check_invariants`], which
/// [`check_wire_surface_elements`] reports) states that in the artifact
/// itself.
///
/// The one cross-artifact check that IS meaningful: a family must not declare
/// a route whose path is a realized ITS-REST binding's path, which would
/// mislabel a released operation as our own extension. The axis writes
/// absolute default-deployment paths while a binding path is relative to the
/// API base, so the comparison strips a leading MOUNT prefix — and a prefix
/// only counts as a mount when its last segment is not itself a released
/// first segment. That distinction is what separates
/// `/…/v1` + `/ehr` (the same released operation, re-declared) from
/// `/…/v1/admin` + `/query/{q}/{v}` (a different resource that merely ends in
/// the same tail).
fn check_served_extensions(
    set: &ArtifactSet,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    // An `extension` binding realizes its operation over one of THESE routes
    // (the `realization-scope` gate proves the family/path pairing), so its
    // path is by construction not a released one — including it here would
    // make every extension family collide with its own bindings.
    let released: Vec<&str> = set
        .bindings
        .iter()
        .filter(|(_, b)| !b.is_unrealized() && !b.is_extension())
        .filter_map(|(_, b)| b.request.as_ref())
        .map(|r| r.path.raw())
        .collect();
    let released_roots: BTreeSet<&str> = released
        .iter()
        .filter_map(|p| p.trim_start_matches('/').split('/').next())
        .collect();
    for extension in &wire_surface.served_extensions {
        for route in &extension.routes {
            let Some(path) = ServedExtension::route_path(route) else {
                continue; // shape finding already raised by check_invariants
            };
            for binding_path in &released {
                let claims = path == *binding_path
                    || path.strip_suffix(*binding_path).is_some_and(|mount| {
                        !mount.is_empty()
                            && !mount.ends_with('/')
                            && mount
                                .rsplit('/')
                                .next()
                                .is_some_and(|last| !released_roots.contains(last))
                    });
                if claims {
                    push(
                        findings,
                        CheckId::SurfaceCoverage,
                        &extension.family,
                        format!(
                            "served_extensions route {route:?} claims the released ITS-REST path \
                             {binding_path} — an extension family may not declare an operation \
                             the release defines"
                        ),
                    );
                }
            }
        }
    }
}

/// Axis 1 — every SM operation of a pinned platform interface, PLUS every
/// pinned non-SM RELEASED ITS-REST operation ([`NON_SM_REST_OPERATIONS`]), has
/// an `its-rest` binding (realized or unrealized) or a cited `sm_operations`
/// exception.
fn check_surface_sm_operations(
    set: &ArtifactSet,
    spec: &SpecIndex<'_>,
    wire_surface: &WireSurface,
    findings: &mut Vec<Finding>,
) {
    for interface in PLATFORM_INTERFACES {
        let ops = match spec.interface_operations(interface) {
            Ok(ops) => ops,
            Err(message) => {
                push(findings, CheckId::SurfaceCoverage, interface, message);
                continue;
            }
        };
        for name in ops.iter() {
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
    // The ITS-side half of the domain: a RELEASED ITS-REST operation the SM
    // models no interface for is enumerated from the pinned table, so it is
    // held to exactly the same obligation as an SM operation.
    for (name, source) in NON_SM_REST_OPERATIONS {
        let Ok(op) = SmOperationRef::parse(name) else {
            push(
                findings,
                CheckId::SurfaceCoverage,
                name,
                "NON_SM_REST_OPERATIONS entry is not a parsable operation reference".to_owned(),
            );
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
            format!(
                "non-SM ITS-REST operation ({source}) has no its-rest binding and no \
                 wire_surface.yaml sm_operations exception — add a binding (realized or \
                 unrealized) or a cited off_wire/variant_of/coverage_gap entry"
            ),
        );
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

// ── Axis 3 section derivation ───────────────────────────────────────────────

/// The RELEASED overview documents whose section headings ARE the Axis-3
/// enumeration domain, relative to the vendored spec root. Both are ITS-REST
/// docs text (never the OAS): the cross-cutting wire behaviours the API defines
/// outside any single operation live in these two chapters and nowhere else.
const AXIS3_OVERVIEW_DOCS: &[&str] = &[
    "ITS-REST/specifications/docs/overview/Requests_and_responses.md",
    "ITS-REST/specifications/docs/overview/Resources.md",
];

/// Sections of [`AXIS3_OVERVIEW_DOCS`] that define no distinct testable
/// cross-cutting wire behaviour, each with the citation saying why. A heading
/// listed here is exempt from the derivation; every other heading must appear
/// in at least one authored `elements`/`branches` source string.
///
/// Empty today: every heading of both released overview chapters is named by
/// an authored source. The table is the pinned escape hatch for a heading that
/// is genuinely un-testable (a pure framing paragraph), never a way to retire
/// an inconvenient behaviour — an entry whose heading IS named by a source, or
/// which names no heading at all, is itself a finding.
const AXIS3_SECTION_EXCLUSIONS: &[(&str, &str)] = &[];

/// Whitespace-normalized, lower-cased form used for heading↔source matching
/// (source strings are YAML-folded, so they carry newlines and runs of spaces;
/// one vendored heading — "Prefer only identifier " — carries a trailing space
/// the vendored file must keep).
fn normalize_heading(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The `#`/`##` section headings of a released markdown chapter, in document
/// order, de-duplicated, with fenced code blocks skipped (a `# …` line inside
/// a fence is code, not a heading).
fn markdown_section_headings(text: &str) -> Vec<String> {
    let mut headings: Vec<String> = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some(rest) = trimmed
            .strip_prefix("# ")
            .or_else(|| trimmed.strip_prefix("## "))
        else {
            continue;
        };
        let heading = rest.trim().to_owned();
        if heading.is_empty() || headings.iter().any(|h| h == &heading) {
            continue;
        }
        headings.push(heading);
    }
    headings
}

/// Every `source` string the Axis-3 register authors (elements + branches) —
/// the corpus a derived heading must be named by.
fn wire_surface_source_texts(wire_surface: &WireSurface) -> Vec<String> {
    let mut sources: Vec<String> = Vec::new();
    for element in &wire_surface.elements {
        sources.push(normalize_heading(&element.source));
    }
    for branch in &wire_surface.branches {
        sources.push(normalize_heading(&branch.source));
    }
    sources
}

/// One released overview chapter's derivation result.
struct DocDerivation {
    /// The chapter's path relative to the spec root.
    doc: &'static str,
    /// The chapter could not be read (its own finding).
    unreadable: bool,
    /// Headings named by ≥ 1 authored source.
    covered: Vec<String>,
    /// Headings exempted by [`AXIS3_SECTION_EXCLUSIONS`].
    excluded: Vec<String>,
    /// Headings with neither (each one a finding).
    uncovered: Vec<String>,
}

/// Derive the Axis-3 enumeration domain from the released overview chapters and
/// classify every heading. Pure over (`wire_surface`, the vendored files,
/// `exclusions`) — the exclusion table is a parameter so the mechanism is
/// testable without touching the pinned const.
fn axis3_derivation(
    wire_surface: &WireSurface,
    spec: &SpecIndex<'_>,
    exclusions: &[(&str, &str)],
) -> Vec<DocDerivation> {
    let sources = wire_surface_source_texts(wire_surface);
    let mut out = Vec::new();
    for doc in AXIS3_OVERVIEW_DOCS.iter().copied() {
        let Ok(text) = spec.read(&spec.root().join(doc)) else {
            out.push(DocDerivation {
                doc,
                unreadable: true,
                covered: Vec::new(),
                excluded: Vec::new(),
                uncovered: Vec::new(),
            });
            continue;
        };
        let mut derivation = DocDerivation {
            doc,
            unreadable: false,
            covered: Vec::new(),
            excluded: Vec::new(),
            uncovered: Vec::new(),
        };
        for heading in markdown_section_headings(&text) {
            let needle = normalize_heading(&heading);
            if sources.iter().any(|source| source.contains(&needle)) {
                derivation.covered.push(heading);
            } else if exclusions
                .iter()
                .any(|(excluded, _)| normalize_heading(excluded) == needle)
            {
                derivation.excluded.push(heading);
            } else {
                derivation.uncovered.push(heading);
            }
        }
        out.push(derivation);
    }
    out
}

/// Axis 3, derivation half — the register's element list is AUTHORED, so a
/// cross-cutting behaviour nobody thought of is invisible to
/// [`check_wire_surface_elements`]. This derives the domain instead: every
/// `#`/`##` section of the two RELEASED overview chapters must be named by an
/// authored `elements`/`branches` source, or be pinned in
/// [`AXIS3_SECTION_EXCLUSIONS`] with its citation. Stale exclusions (covered,
/// or naming no heading at all) are findings too — the table only ratchets.
fn check_axis3_section_derivation(
    wire_surface: &WireSurface,
    spec: &SpecIndex<'_>,
    exclusions: &[(&str, &str)],
    findings: &mut Vec<Finding>,
) {
    let derivations = axis3_derivation(wire_surface, spec, exclusions);
    let mut every_heading: BTreeSet<String> = BTreeSet::new();
    let mut covered_headings: BTreeSet<String> = BTreeSet::new();
    for derivation in &derivations {
        if derivation.unreadable {
            push(
                findings,
                CheckId::SurfaceCoverage,
                derivation.doc,
                "released overview chapter is not readable under the vendored spec root — the \
                 Axis-3 section derivation cannot run"
                    .to_owned(),
            );
            continue;
        }
        for heading in derivation
            .covered
            .iter()
            .chain(&derivation.excluded)
            .chain(&derivation.uncovered)
        {
            every_heading.insert(normalize_heading(heading));
        }
        for heading in &derivation.covered {
            covered_headings.insert(normalize_heading(heading));
        }
        for heading in &derivation.uncovered {
            push(
                findings,
                CheckId::SurfaceCoverage,
                derivation.doc,
                format!(
                    "§{heading} is a section of a RELEASED overview chapter that no \
                     wire_surface.yaml elements/branches source names — add a cross-cutting \
                     element for the behaviour (covered_by a case, or a cited exception), or pin \
                     the heading in AXIS3_SECTION_EXCLUSIONS \
                     (tools/cnf-runner/src/validate.rs) with the citation saying why it defines \
                     no distinct testable wire behaviour"
                ),
            );
        }
    }
    if derivations.iter().any(|d| d.unreadable) {
        return; // an incomplete domain cannot judge exclusion staleness
    }
    for (heading, _) in exclusions {
        let needle = normalize_heading(heading);
        if !every_heading.contains(&needle) {
            push(
                findings,
                CheckId::SurfaceCoverage,
                "AXIS3_SECTION_EXCLUSIONS",
                format!("exclusion {heading:?} names no section of the released overview chapters"),
            );
        } else if covered_headings.contains(&needle) {
            push(
                findings,
                CheckId::SurfaceCoverage,
                "AXIS3_SECTION_EXCLUSIONS",
                format!(
                    "exclusion {heading:?} is stale — an authored wire_surface.yaml source now \
                     names that section; remove the exclusion"
                ),
            );
        }
    }
}

/// Render the deterministic coverage report
/// (`docs/conformance/coverage-report.md`): per-interface SM-operation
/// status, per-binding outcome/format coverage, and the cross-cutting
/// wire-surface table.
///
/// Stable ordering, no timestamps — the same inputs always render
/// byte-identical output.
///
/// Axis 1 (the per-interface section) and the Axis-3 section derivation render
/// only when `spec_root` is supplied (they read the vendored spec tree).
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one deterministic report-rendering seam"
)]
pub fn render_coverage_report(set: &ArtifactSet, spec_root: Option<&Path>) -> String {
    use std::fmt::Write;

    let empty = WireSurface::default();
    let wire_surface = set.wire_surface.as_ref().map_or(&empty, |(_, w)| w);
    let spec = spec_root.map(SpecIndex::new);

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
    if let Some(spec) = spec.as_ref() {
        out.push_str("## Axis 1 — SM-operation coverage (per platform interface)\n\n");
        out.push_str("| Interface | Operations | Realized | Unrealized | Off-wire / exception |\n");
        out.push_str("|---|--:|--:|--:|--:|\n");
        for interface in PLATFORM_INTERFACES {
            let Ok(ops) = spec.interface_operations(interface) else {
                let _ = writeln!(out, "| {interface} | (no vendored SM class export) | | | |");
                continue;
            };
            let (mut realized, mut unrealized, mut excepted) = (0_usize, 0_usize, 0_usize);
            for name in ops.iter() {
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
        // The ITS-side half of Axis 1: the pinned non-SM ITS-REST operations,
        // grouped by their reserved pseudo-interface. Computed exactly like an
        // SM row — the anchor differs, the obligation does not.
        let mut pseudo: BTreeMap<String, Vec<SmOperationRef>> = BTreeMap::new();
        for (name, _) in NON_SM_REST_OPERATIONS {
            let Ok(op) = SmOperationRef::parse(name) else {
                continue;
            };
            pseudo
                .entry(op.interface().to_owned())
                .or_default()
                .push(op);
        }
        for (interface, ops) in pseudo {
            let (mut realized, mut unrealized, mut excepted) = (0_usize, 0_usize, 0_usize);
            for op in &ops {
                match set.bindings.iter().find(|(_, b)| b.sm_operation == *op) {
                    Some((_, b)) if b.is_unrealized() => unrealized += 1,
                    Some(_) => realized += 1,
                    None if wire_surface.sm_exception(op).is_some() => excepted += 1,
                    None => {}
                }
            }
            let _ = writeln!(
                out,
                "| {interface} (docs-text pinned, non-SM) | {} | {realized} | {unrealized} | \
                 {excepted} |",
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

    // ── Axis 3, derivation half ──
    if let Some(spec) = spec.as_ref() {
        out.push_str("### Axis 3 derivation — RELEASED overview sections\n\n");
        out.push_str(
            "The element list above is AUTHORED; this table is DERIVED — every `#`/`##` section \
             of the two released overview chapters must be named by an authored \
             `elements`/`branches` source or pinned in `AXIS3_SECTION_EXCLUSIONS`.\n\n",
        );
        out.push_str(
            "| Chapter | Sections | Named by a source | Excluded (pinned) | Uncovered |\n",
        );
        out.push_str("|---|--:|--:|--:|--:|\n");
        for derivation in axis3_derivation(wire_surface, spec, AXIS3_SECTION_EXCLUSIONS) {
            if derivation.unreadable {
                let _ = writeln!(out, "| `{}` | (not readable) | | | |", derivation.doc);
                continue;
            }
            let sections =
                derivation.covered.len() + derivation.excluded.len() + derivation.uncovered.len();
            let _ = writeln!(
                out,
                "| `{}` | {sections} | {} | {} | {} |",
                derivation.doc,
                derivation.covered.len(),
                derivation.excluded.len(),
                derivation.uncovered.len()
            );
        }
        out.push('\n');
    }
    out
}
#[cfg(test)]
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

        let spec = SpecIndex::new(dir.path());
        let ops = spec.interface_operations("I_FIXTURE").unwrap();
        assert_eq!(
            *ops,
            vec!["create_thing".to_owned(), "get_thing".to_owned()]
        );
        // A missing class export is an error, not an empty list.
        assert!(spec.interface_operations("I_ABSENT").is_err());
    }

    /// A minimal vendored RM tree: one chapter that pulls its class table in
    /// through the `include::{uml_export_dir}/classes/{pkg}…` indirection the
    /// real spec uses, plus the class export itself.
    fn spec_tree_fixture() -> assert_fs::TempDir {
        let dir = assert_fs::TempDir::new().unwrap();
        let chapter = dir.path().join("RM/docs/ehr_extract");
        let classes = dir.path().join("RM/docs/UML/classes");
        std::fs::create_dir_all(&chapter).unwrap();
        std::fs::create_dir_all(&classes).unwrap();
        std::fs::write(
            chapter.join("master.adoc"),
            ":pkg: org.openehr.rm.ehr_extract.\n",
        )
        .unwrap();
        std::fs::write(
            chapter.join("master04-common_package.adoc"),
            "= Common Package\n\n== Version Specification\n\n\
             include::{uml_export_dir}/classes/{pkg}extract_manifest.adoc[]\n",
        )
        .unwrap();
        std::fs::write(
            classes.join("org.openehr.rm.ehr_extract.extract_manifest.adoc"),
            "=== EXTRACT_MANIFEST Class\n\n|===\nh|*Attributes*\nh|*1..1*\n\
             |*entities*: `List<EXTRACT_ENTITY_MANIFEST>`\n|===\n",
        )
        .unwrap();
        dir
    }

    fn citation_findings(citation: &str, spec: &SpecIndex<'_>) -> Vec<Finding> {
        let mut findings = Vec::new();
        check_citations(&[citation], "T-1", spec, &mut findings);
        findings
    }

    /// The register-source convention (#2545): prose fragments pass, citation
    /// clauses resolve, and a register-shaped mix of both is clean when its
    /// citations are real.
    #[test]
    fn register_source_prose_passes_and_citations_resolve() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());
        assert!(
            citation_findings(
                "silence CONFIRMED first-hand (grep of the chapter: zero hits); \
                 RM ehr_extract master04-common_package §Version Specification; \
                 the stalled guide is never authority",
                &spec
            )
            .is_empty()
        );
    }

    /// A register source with NO citation clause at all is accused through
    /// the unknown-component fallback — a silence claim must ground on at
    /// least one resolvable citation (#2545).
    #[test]
    fn register_source_without_any_citation_clause_is_accused() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());
        let out = citation_findings("ALL SOURCES READ FIRST-HAND, nothing cited", &spec);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].message.contains("unknown component"), "{out:?}");
    }

    /// Seeded defects a register source must fail on: a phantom document and
    /// a phantom section in a real document (#2545).
    #[test]
    fn register_source_phantom_document_and_section_fail() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());
        let phantom_doc = citation_findings("RM ehr_extract master99-nonexistent §Anything", &spec);
        assert_eq!(phantom_doc.len(), 1, "{phantom_doc:?}");
        let phantom_section = citation_findings(
            "RM ehr_extract master04-common_package §No Such Heading Anywhere",
            &spec,
        );
        assert_eq!(phantom_section.len(), 1, "{phantom_section:?}");
    }

    /// `{a,b}` brace shorthands expand — every variant must resolve, so a
    /// half-phantom shorthand still fails, naming the missing variant (#2545).
    #[test]
    fn brace_shorthand_expands_and_a_half_phantom_variant_fails() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());
        assert!(
            citation_findings(
                "RM ehr_extract UML/classes/org.openehr.rm.ehr_extract.{extract_manifest}.adoc \
                 §Attributes",
                &spec
            )
            .is_empty()
        );
        let out = citation_findings(
            "RM ehr_extract \
             UML/classes/org.openehr.rm.ehr_extract.{extract_manifest,nonexistent}.adoc",
            &spec,
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(out[0].message.contains("nonexistent"), "{out:?}");
    }

    /// A workspace-shaped fixture: `docs/specs/openehr` beside the vendored
    /// XSD bundle at `crates/openehr-its/schemas/xml`, mirroring the real
    /// layout [`bundle_root`] derives.
    fn workspace_fixture() -> assert_fs::TempDir {
        let dir = assert_fs::TempDir::new().unwrap();
        let docs = dir
            .path()
            .join("docs/specs/openehr/ITS-XML/components/RM/Release-1.0.2");
        let schemas = dir
            .path()
            .join("crates/openehr-its/schemas/xml/components/RM/Release-1.0.2/documents");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(&schemas).unwrap();
        // What `vendor-spec-docs.sh` actually vendors into the docs tree: the
        // upstream README stub, no schemas.
        std::fs::write(docs.join("README.adoc"), "= XML Schemas\n\n== Releases\n").unwrap();
        std::fs::write(
            schemas.join("Composition.xsd"),
            "<xs:schema targetNamespace=\"http://schemas.openehr.org/v1\">\n\
             <xs:element name=\"composition\" type=\"COMPOSITION\"/>\n\
             <xs:complexType name=\"COMPOSITION\"/>\n</xs:schema>\n",
        )
        .unwrap();
        dir
    }

    /// ITS-XML is the one component with TWO roots (issue #1833): the docs
    /// tree carries only prose, the released XSDs are vendored once under
    /// `crates/openehr-its/schemas/xml`, and a citation of an XSD element
    /// must resolve without the bundle being duplicated into the docs tree.
    #[test]
    fn its_xml_citations_resolve_against_the_vendored_schema_bundle() {
        let dir = workspace_fixture();
        let root = dir.path().join("docs/specs/openehr");
        let spec = SpecIndex::new(&root);

        // Both roots are offered for ITS-XML, the docs tree first.
        let roots = spec.component_roots("ITS-XML", "ITS-XML");
        assert_eq!(roots.len(), 2, "{roots:?}");
        // Every other component keeps exactly one.
        assert_eq!(spec.component_roots("RM", "RM").len(), 0, "no RM docs dir");

        // The docs-tree half still resolves.
        assert!(
            citation_findings(
                "ITS-XML components/RM/Release-1.0.2/README.adoc §Releases",
                &spec
            )
            .is_empty()
        );
        // The XSD half resolves — the document AND its declared element, the
        // citation that could not be machine-resolved at all before.
        assert!(
            citation_findings(
                "ITS-XML components/RM/Release-1.0.2/documents/Composition.xsd §composition",
                &spec
            )
            .is_empty()
        );
        assert!(
            citation_findings(
                "ITS-XML components/RM/Release-1.0.2/documents/Composition.xsd §COMPOSITION",
                &spec
            )
            .is_empty()
        );
        // Seeded defect: an element the schema does not declare.
        let out = citation_findings(
            "ITS-XML components/RM/Release-1.0.2/documents/Composition.xsd §invented_element",
            &spec,
        );
        assert_eq!(out.len(), 1, "{out:?}");
        // Seeded defect: the overlay is ITS-XML-only — the same path under
        // another component resolves nowhere.
        let out = citation_findings(
            "ITS-JSON components/RM/Release-1.0.2/documents/Composition.xsd",
            &spec,
        );
        assert_eq!(out.len(), 1, "{out:?}");
    }

    #[test]
    fn xsd_declared_names_reads_every_name_attribute() {
        let names = xsd_declared_names(
            "<xs:element name=\"composition\" type=\"openehr:COMPOSITION\"/>\
             <xs:complexType name='LOCATABLE'><xs:attribute name=\"archetype_node_id\"/>\
             </xs:complexType>",
        );
        // Declared names only — `type=` references are not declarations.
        assert!(names.contains("composition"), "{names:?}");
        assert!(names.contains("locatable"), "{names:?}");
        assert!(names.contains("archetype node id"), "{names:?}");
        assert_eq!(names.len(), 3, "{names:?}");
    }

    #[test]
    fn spec_ref_gate_resolves_documents_and_sections() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());

        // The chapter resolves, and so does a section it declares itself.
        assert!(
            citation_findings(
                "RM ehr_extract master04-common_package §Version Specification",
                &spec
            )
            .is_empty()
        );
        // A section the chapter only carries THROUGH its class-table include
        // resolves too — the included table is part of the document.
        assert!(
            citation_findings(
                "RM ehr_extract master04-common_package §EXTRACT_MANIFEST",
                &spec
            )
            .is_empty()
        );
        // So does a class-table label, and the class export addressed directly.
        assert!(
            citation_findings(
                "RM ehr_extract UML/classes/org.openehr.rm.ehr_extract.extract_manifest.adoc \
                 §Attributes (entities 1..1)",
                &spec
            )
            .is_empty()
        );
        // A component-only citation checks the component directory alone.
        assert!(citation_findings("RM", &spec).is_empty());
    }

    #[test]
    fn spec_ref_gate_refuses_a_phantom_document() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());
        // Seeded defect: the chapter number does not exist. The pre-#1807
        // gate passed this on the `ehr_extract` token alone.
        let findings = citation_findings(
            "RM ehr_extract master99-invented_package §Version Specification",
            &spec,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        let message = &findings.first().unwrap().message;
        assert!(message.contains("no vendored document"), "{message}");
        // Seeded defect: a real chapter name under the WRONG component.
        let findings = citation_findings("BASE ehr_extract master04-common_package", &spec);
        assert_eq!(findings.len(), 1, "{findings:?}");
        // Seeded defect: a component that does not exist at all.
        let findings = citation_findings("NONESUCH master04 §Whatever", &spec);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings
                .first()
                .unwrap()
                .message
                .contains("unknown component"),
            "{findings:?}"
        );
    }

    #[test]
    fn spec_ref_gate_refuses_a_phantom_section() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());
        // Seeded defect: the document resolves, the section does not — the
        // phantom-citation class (#1738): a real chapter plus a §heading that
        // exists nowhere in it.
        let findings = citation_findings(
            "RM ehr_extract master04-common_package §Invented Section",
            &spec,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
        let message = &findings.first().unwrap().message;
        assert!(message.contains("carry no section matching"), "{message}");
        // Every clause of a multi-document citation is resolved, not just the
        // first: the second clause's section is the seeded defect here.
        let findings = citation_findings(
            "RM ehr_extract master04-common_package §Version Specification; \
             RM ehr_extract master04-common_package §Invented Section",
            &spec,
        );
        assert_eq!(findings.len(), 1, "{findings:?}");
    }

    /// A binding `extension`/`unrealized` `source` is a DERIVATION, and every
    /// clause of it is read (issue #1832): before this gate a derivation
    /// written `SM x.adoc … vs ITS-REST y.yaml …` was ONE clause to
    /// [`citation_clauses`], so the ITS-REST half — and any typo in it —
    /// never reached the resolver.
    #[test]
    fn binding_source_gate_reads_every_clause_of_the_derivation() {
        let dir = spec_tree_fixture();
        let spec = SpecIndex::new(dir.path());
        let binding = |source: &str| -> OperationBinding {
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_EHR_SERVICE.create_ehr",
                "its": "its-rest",
                "unrealized": {
                    "reason": "r",
                    "source": source,
                    "ambiguity": "AMB-1"
                }
            }))
            .expect("fixture binding parses")
        };
        let findings = |source: &str| -> Vec<Finding> {
            let mut out = Vec::new();
            check_binding_sources(&binding(source), "b.yaml", &spec, &mut out);
            out
        };

        // Normalized: one `;`-separated clause per citation, each resolving.
        assert!(
            findings(
                "RM ehr_extract master04-common_package §Version Specification; \
                 RM ehr_extract master04-common_package §EXTRACT_MANIFEST"
            )
            .is_empty()
        );
        // The spec-silence flag is the one allowed non-citation clause.
        assert!(
            findings(
                "RM ehr_extract master04-common_package; \
                 no openEHR spec governs this — our own design/extension"
            )
            .is_empty()
        );
        // Seeded defect: a separated fragment that names a document without
        // its component. `citation_clauses` DROPS it unread, so the gate
        // refuses the shape rather than passing it vacuously.
        let out = findings(
            "RM ehr_extract master04-common_package; \
             UML/classes/org.openehr.rm.ehr_extract.extract_manifest.adoc (a gloss)",
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(
            out.first().expect("one finding").message.contains(
                "opens with neither a spec component nor the spec-silence flag nor the register"
            ),
            "{out:?}"
        );
        // Seeded defect: the un-normalized `vs` form, which reads as ONE
        // clause whose token run swallows the connective — it resolves to no
        // document at all, so it cannot pass either.
        let out = findings(
            "RM ehr_extract master04-common_package vs RM ehr_extract master99-invented_package",
        );
        assert_eq!(out.len(), 1, "{out:?}");
        // Seeded defect: normalized shape, phantom document in the SECOND
        // clause — exactly what the old form hid.
        let out = findings(
            "RM ehr_extract master04-common_package; RM ehr_extract master99-invented_package",
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(
            out.first()
                .expect("one finding")
                .message
                .contains("no vendored document"),
            "{out:?}"
        );
    }

    /// Drive one step through one binding and return the gate's findings.
    fn step_argument_findings(
        binding: serde_json::Value,
        with: &serde_json::Value,
    ) -> Vec<Finding> {
        let binding: OperationBinding =
            serde_json::from_value(binding).expect("fixture binding parses");
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "T-ARG", "kind": "functional", "component": "EHR",
            "sm_operation": "I_EHR_CONTRIBUTION.commit_contribution",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": [],
            "flow": [ {
                "step": 1, "call": "commit_contribution",
                "with": with, "expect": "created"
            } ]
        }))
        .expect("fixture case parses");
        let mut set = ArtifactSet::default();
        set.bindings.push((PathBuf::from("b.yaml"), binding));
        set.cases.push((PathBuf::from("c.yaml"), case));
        let mut out = Vec::new();
        check_step_arguments(&set, &mut out);
        out
    }

    /// The bundled-CONTRIBUTION binding: a path param, a declared query
    /// parameter, and the `contribution` body role.
    fn contribution_binding() -> serde_json::Value {
        serde_json::json!({
            "sm_operation": "I_EHR_CONTRIBUTION.commit_contribution",
            "its": "its-rest",
            "request": {
                "method": "POST",
                "path": "/ehr/{ehr_id}/contribution",
                "query": { "dry_run": "${dry_run?}" },
                "body": "contribution"
            },
            "outcomes": { "created": { "status": 201 } }
        })
    }

    /// A key every request path reads is not accused: a path param, a
    /// declared query parameter, and the two keys the bundled-CONTRIBUTION
    /// construct reads — `versions` and `audit`, the client-supplied
    /// committal metadata issue #1818 wired through.
    #[test]
    fn step_arguments_gate_credits_every_read_key() {
        let out = step_argument_findings(
            contribution_binding(),
            &serde_json::json!({
                "ehr_id": "${ehr_id}",
                "dry_run": "true",
                "versions": [{ "data": "${ds:x}", "change_type": "creation" }],
                "audit": { "committer": { "name": "Dr Example" } }
            }),
        );
        assert!(out.is_empty(), "{out:?}");
    }

    /// A `with:` key the driver reads on no request path never reaches the
    /// SUT, so anything the case asserts about it passes vacuously (issue
    /// #1830 — the live instance was the SEC audit case's client-supplied
    /// `audit.time_committed`, dropped for its whole life).
    ///
    /// The seeded defect is that very block on a binding whose body role is
    /// NOT `contribution`: `select_body` resolves the `composition` role
    /// directly, so the single-payload scan that might otherwise have picked
    /// the object up never runs.
    #[test]
    fn step_arguments_gate_accuses_an_unread_payload_key() {
        let out = step_argument_findings(
            serde_json::json!({
                "sm_operation": "I_EHR_CONTRIBUTION.commit_contribution",
                "its": "its-rest",
                "request": {
                    "method": "POST",
                    "path": "/ehr/{ehr_id}/composition",
                    "body": "composition"
                },
                "outcomes": { "created": { "status": 201 } }
            }),
            &serde_json::json!({
                "ehr_id": "${ehr_id}",
                "composition": "${ds:x}",
                "audit": { "time_committed": "1990-01-01T00:00:00Z" }
            }),
        );
        assert_eq!(out.len(), 1, "{out:?}");
        let finding = out.first().expect("one finding");
        assert_eq!(finding.check, CheckId::StepArguments);
        assert!(finding.message.contains("with.audit"), "{finding:?}");
        assert!(finding.message.contains("vacuously"), "{finding:?}");
    }

    /// A key that names nothing on the wire at all.
    #[test]
    fn step_arguments_gate_accuses_a_key_no_form_declares() {
        let out = step_argument_findings(
            contribution_binding(),
            &serde_json::json!({ "ehr_id": "${ehr_id}", "decoration": "x" }),
        );
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(
            out.first()
                .expect("one finding")
                .message
                .contains("with.decoration"),
            "{out:?}"
        );
    }

    /// A `*_text` payload role takes a plain STRING as its body, so the one
    /// non-path string key is the payload, not decoration.
    #[test]
    fn step_arguments_gate_credits_a_text_role_payload() {
        let out = step_argument_findings(
            serde_json::json!({
                "sm_operation": "I_EHR_CONTRIBUTION.commit_contribution",
                "its": "its-rest",
                "request": {
                    "method": "PUT",
                    "path": "/definition/query/{qualified_query_name}",
                    "body": "aql_text"
                },
                "outcomes": { "created": { "status": 200 } }
            }),
            &serde_json::json!({
                "qualified_query_name": "org.openehr.cnf::q",
                "query": "SELECT c FROM EHR e CONTAINS COMPOSITION c"
            }),
        );
        assert!(out.is_empty(), "{out:?}");
    }

    /// A structured body reads its `${…}` members from the step's `with:`.
    #[test]
    fn step_arguments_gate_credits_structured_body_members() {
        let out = step_argument_findings(
            serde_json::json!({
                "sm_operation": "I_EHR_CONTRIBUTION.commit_contribution",
                "its": "its-rest",
                "request": {
                    "method": "POST",
                    "path": "/query/aql",
                    "body": { "q": "${q}", "offset": "${offset?}" }
                },
                "outcomes": { "created": { "status": 200 } }
            }),
            &serde_json::json!({ "q": "SELECT c FROM EHR e", "offset": "0" }),
        );
        assert!(out.is_empty(), "{out:?}");
    }

    /// A `with_<p>` sibling binding is SELECTED BY the step binding `<p>`, so
    /// `<p>` is read even though the variant-less binding's own request never
    /// names it (`HttpDriver::auto_variant`).
    #[test]
    fn step_arguments_gate_credits_the_auto_variant_selector() {
        let variantless: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "request": { "method": "POST", "path": "/ehr" },
            "outcomes": { "created": { "status": 201 } }
        }))
        .expect("fixture binding parses");
        let with_ehr_id: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "its": "its-rest",
            "variant": "with_ehr_id",
            "request": { "method": "PUT", "path": "/ehr/{ehr_id}" },
            "outcomes": { "created": { "status": 201 } }
        }))
        .expect("fixture binding parses");
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "T-VAR", "kind": "functional", "component": "EHR",
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": [],
            "flow": [ {
                "step": 1, "call": "create_ehr",
                "with": { "ehr_id": "11111111-1111-4111-8111-111111111111" },
                "expect": "created"
            } ]
        }))
        .expect("fixture case parses");
        let mut set = ArtifactSet::default();
        set.bindings.push((PathBuf::from("a.yaml"), variantless));
        set.bindings.push((PathBuf::from("b.yaml"), with_ehr_id));
        set.cases.push((PathBuf::from("c.yaml"), case));
        let mut findings = Vec::new();
        check_step_arguments(&set, &mut findings);
        assert!(findings.is_empty(), "{findings:?}");
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

    /// Axis 4 is a DECLARATION: adding served extensions changes no coverage
    /// obligation on the other three axes, and a family may not claim a
    /// released path.
    #[test]
    fn axis4_declares_without_gating() {
        let set = build_set(true);
        let wire = set.wire_surface.as_ref().map(|(_, w)| w).unwrap();
        let mut baseline = Vec::new();
        check_binding_branch_coverage(&set, wire, &mut baseline);
        check_wire_surface_elements(&set, wire, &mut baseline);
        assert!(baseline.is_empty(), "{baseline:?}");

        let declared: WireSurface = serde_json::from_value(serde_json::json!({
            "branches": [ {
                "binding": "I_EHR_SERVICE.create_ehr", "outcome": "already_exists",
                "reason": "coverage_gap",
                "source": "ITS-REST Requests_and_responses.md §HTTP status codes"
            } ],
            "served_extensions": [ {
                "family": "management",
                "routes": ["GET /management/info"],
                "config_gate": "management.enabled",
                "spec_silence": "no released clause governs the URI space beyond the resource set",
                "never_gates": true
            } ]
        }))
        .unwrap();
        let mut with_axis = Vec::new();
        check_binding_branch_coverage(&set, &declared, &mut with_axis);
        check_wire_surface_elements(&set, &declared, &mut with_axis);
        check_served_extensions(&set, &declared, &mut with_axis);
        assert!(
            with_axis.is_empty(),
            "the outward axis must never add an obligation, got: {with_axis:?}"
        );

        // Claiming a released path IS a finding (the binding fixture serves
        // POST /ehr; the mount prefix is stripped at the segment boundary).
        let claiming: WireSurface = serde_json::from_value(serde_json::json!({
            "served_extensions": [ {
                "family": "impostor",
                "routes": ["POST /ferroehr/rest/openehr/v1/ehr"],
                "config_gate": "always on",
                "spec_silence": "s",
                "never_gates": true
            } ]
        }))
        .unwrap();
        let mut findings = Vec::new();
        check_served_extensions(&set, &claiming, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("claims the released")),
            "expected a released-path claim finding, got: {findings:?}"
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

    /// A pinned non-SM ITS-REST operation resolves WITHOUT an SM class export
    /// (there is none — that is the whole point), while an unpinned
    /// `I_ITS_REST_*` reference is a finding naming the pinned table, so the
    /// pseudo-interface cannot become a way around SM resolution.
    #[test]
    fn pinned_pseudo_operation_resolves_and_an_unpinned_one_is_a_finding() {
        let empty_root = assert_fs::TempDir::new().unwrap();

        let spec = SpecIndex::new(empty_root.path());

        let (pinned_name, _) = *NON_SM_REST_OPERATIONS.first().unwrap();
        let pinned = SmOperationRef::parse(pinned_name).unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&pinned, "b.yaml", &spec, &mut findings);
        assert!(findings.is_empty(), "{findings:?}");

        let invented = SmOperationRef::parse("I_ITS_REST_SYSTEM.invented").unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&invented, "b.yaml", &spec, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SmOperation
                    && f.message.contains("NON_SM_REST_OPERATIONS")),
            "expected an unpinned-pseudo-interface finding, got: {findings:?}"
        );

        // A real SM interface is still resolved against the vendored tree.
        let sm = SmOperationRef::parse("I_EHR_SERVICE.create_ehr").unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&sm, "b.yaml", &spec, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("no vendored SM class export")),
            "{findings:?}"
        );
    }

    /// Axis 1 holds a pinned non-SM operation to the same obligation as an SM
    /// one: no binding and no `sm_operations` exception is a finding.
    #[test]
    fn axis1_requires_a_binding_for_a_pinned_non_sm_operation() {
        let set = build_set(false);
        let empty_root = assert_fs::TempDir::new().unwrap();
        let empty = WireSurface::default();
        let (pinned, pinned_source) = *NON_SM_REST_OPERATIONS.first().unwrap();

        let mut findings = Vec::new();
        check_surface_sm_operations(
            &set,
            &SpecIndex::new(empty_root.path()),
            &empty,
            &mut findings,
        );
        assert!(
            findings
                .iter()
                .any(|f| f.artifact == pinned && f.message.contains("non-SM ITS-REST operation")),
            "expected an unbound non-SM operation finding, got: {findings:?}"
        );

        // A cited sm_operations exception suppresses it (as for an SM op).
        let excepted: WireSurface = serde_json::from_value(serde_json::json!({
            "sm_operations": [ {
                "operation": pinned,
                "reason": "coverage_gap",
                "source": pinned_source
            } ]
        }))
        .unwrap();
        let mut findings = Vec::new();
        check_surface_sm_operations(
            &set,
            &SpecIndex::new(empty_root.path()),
            &excepted,
            &mut findings,
        );
        assert!(
            !findings.iter().any(|f| f.artifact == pinned),
            "the exception should suppress the finding, got: {findings:?}"
        );
    }

    /// The pinned table is a MULTI-interface domain, not a single-row special
    /// case: every row parses, carries the reserved prefix and a non-empty
    /// citation, no reference is pinned twice, and more than one reserved
    /// pseudo-interface is represented (System + `ITEM_TAGS` today).
    #[test]
    fn pinned_non_sm_table_is_wellformed_across_several_pseudo_interfaces() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut interfaces: BTreeSet<String> = BTreeSet::new();
        for (name, source) in NON_SM_REST_OPERATIONS {
            let op = SmOperationRef::parse(name)
                .unwrap_or_else(|e| panic!("pinned row {name} does not parse: {e}"));
            assert!(
                op.interface().starts_with(PSEUDO_INTERFACE_PREFIX),
                "pinned row {name} must use the reserved pseudo-interface prefix"
            );
            assert!(!source.trim().is_empty(), "pinned row {name} has no source");
            assert!(seen.insert(*name), "pinned row {name} is listed twice");
            interfaces.insert(op.interface().to_owned());
        }
        assert!(
            interfaces.len() > 1,
            "the table must stay interface-general, got {interfaces:?}"
        );
        assert!(interfaces.contains("I_ITS_REST_ITEM_TAGS"));

        // The reservation holds for EVERY pseudo-interface, not just System:
        // an unpinned ITEM_TAGS reference is still a finding naming the table.
        let empty_root = assert_fs::TempDir::new().unwrap();
        let spec = SpecIndex::new(empty_root.path());
        let invented = SmOperationRef::parse("I_ITS_REST_ITEM_TAGS.folder_tags_get").unwrap();
        let mut findings = Vec::new();
        resolve_sm_operation(&invented, "b.yaml", &spec, &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SmOperation
                    && f.message.contains("NON_SM_REST_OPERATIONS")),
            "expected an unpinned-pseudo-interface finding, got: {findings:?}"
        );
    }

    /// The Axis-1 report renders ONE row per reserved pseudo-interface, with
    /// per-interface counts — the single-pseudo-interface era is over.
    #[test]
    fn axis1_report_groups_rows_per_pseudo_interface() {
        let mut set = build_set(false);
        let tag_binding: OperationBinding = serde_json::from_value(serde_json::json!({
            "sm_operation": "I_ITS_REST_ITEM_TAGS.composition_tags_get",
            "its": "its-rest",
            "request": { "method": "GET", "path": "/ehr/{ehr_id}/composition/{uid_based_id}/tags" },
            "formats": ["canonical-json"],
            "outcomes": { "ok": { "status": 200 } }
        }))
        .unwrap();
        set.bindings.push((PathBuf::from("tags.yaml"), tag_binding));
        let empty_root = assert_fs::TempDir::new().unwrap();

        let report = render_coverage_report(&set, Some(empty_root.path()));
        let tag_ops = NON_SM_REST_OPERATIONS
            .iter()
            .filter(|(name, _)| name.starts_with("I_ITS_REST_ITEM_TAGS."))
            .count();
        assert!(
            report.contains("| I_ITS_REST_SYSTEM (docs-text pinned, non-SM) | 1 | 0 | 0 | 0 |"),
            "{report}"
        );
        assert!(
            report.contains(&format!(
                "| I_ITS_REST_ITEM_TAGS (docs-text pinned, non-SM) | {tag_ops} | 1 | 0 | 0 |"
            )),
            "{report}"
        );
    }

    #[test]
    fn markdown_headings_take_h1_h2_only_and_skip_fences() {
        let doc = "\
[comment]: # (title: Fixture)\n\
\n\
# HTTP Methods\n\
some prose\n\
## Prefer only identifier \n\
```http\n\
# not a heading\n\
```\n\
### too deep\n\
#nospace\n\
# HTTP Methods\n";
        let headings = markdown_section_headings(doc);
        assert_eq!(
            headings,
            vec![
                "HTTP Methods".to_owned(),
                "Prefer only identifier".to_owned()
            ]
        );
    }

    /// The matcher is whitespace-normalized and case-insensitive over the
    /// YAML-folded source strings, and the exclusion table is the only other
    /// way a heading can be accounted for.
    #[test]
    fn axis3_derivation_matches_sources_and_honours_exclusions() {
        let dir = assert_fs::TempDir::new().unwrap();
        for doc in AXIS3_OVERVIEW_DOCS {
            let path = dir.path().join(doc);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "# Covered Section\n\n## Silent Section\n").unwrap();
        }
        let wire: WireSurface = serde_json::from_value(serde_json::json!({
            "elements": [ {
                "id": "x", "description": "d",
                // folded whitespace + different case: still a match
                "source": "ITS-REST Requests_and_responses.md §covered\n  section (the rule)",
                "exception": { "reason": "coverage_gap" }
            } ]
        }))
        .unwrap();

        let derivations = axis3_derivation(&wire, &SpecIndex::new(dir.path()), &[]);
        for derivation in &derivations {
            assert!(!derivation.unreadable);
            assert_eq!(derivation.covered, vec!["Covered Section".to_owned()]);
            assert_eq!(derivation.uncovered, vec!["Silent Section".to_owned()]);
        }

        let excluded = axis3_derivation(
            &wire,
            &SpecIndex::new(dir.path()),
            &[("silent section", "why")],
        );
        for derivation in &excluded {
            assert_eq!(derivation.excluded, vec!["Silent Section".to_owned()]);
            assert!(derivation.uncovered.is_empty());
        }
    }

    /// An unnamed released section is a finding; the exclusion table only
    /// ratchets — an exclusion that is covered, or that names no section at
    /// all, is a finding of its own.
    #[test]
    fn axis3_derivation_findings_and_exclusion_ratchet() {
        let dir = assert_fs::TempDir::new().unwrap();
        for doc in AXIS3_OVERVIEW_DOCS {
            let path = dir.path().join(doc);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "# Covered Section\n\n## Silent Section\n").unwrap();
        }
        let wire: WireSurface = serde_json::from_value(serde_json::json!({
            "elements": [ {
                "id": "x", "description": "d",
                "source": "ITS-REST Requests_and_responses.md §Covered Section",
                "exception": { "reason": "coverage_gap" }
            } ]
        }))
        .unwrap();

        let mut findings = Vec::new();
        check_axis3_section_derivation(&wire, &SpecIndex::new(dir.path()), &[], &mut findings);
        assert!(
            findings
                .iter()
                .any(|f| f.check == CheckId::SurfaceCoverage
                    && f.message.contains("§Silent Section")),
            "expected an uncovered-section finding, got: {findings:?}"
        );

        let mut findings = Vec::new();
        check_axis3_section_derivation(
            &wire,
            &SpecIndex::new(dir.path()),
            &[
                ("Covered Section", "already named by a source"),
                ("No Such Section", "names nothing"),
                ("Silent Section", "no distinct testable wire behaviour"),
            ],
            &mut findings,
        );
        assert!(
            findings.iter().any(|f| f.message.contains("is stale")),
            "expected a stale-exclusion finding, got: {findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("names no section")),
            "expected an unknown-exclusion finding, got: {findings:?}"
        );
        assert!(
            !findings.iter().any(|f| f.message.contains("§Silent")),
            "the honest exclusion must suppress its section finding, got: {findings:?}"
        );
    }

    /// A missing released chapter is itself a finding — the derivation must
    /// never silently pass because it could not read its domain.
    #[test]
    fn axis3_derivation_reports_an_unreadable_chapter() {
        let dir = assert_fs::TempDir::new().unwrap();
        let wire = WireSurface::default();
        let mut findings = Vec::new();
        check_axis3_section_derivation(&wire, &SpecIndex::new(dir.path()), &[], &mut findings);
        assert_eq!(findings.len(), AXIS3_OVERVIEW_DOCS.len(), "{findings:?}");
        assert!(
            findings.iter().all(|f| f.message.contains("not readable")),
            "{findings:?}"
        );
    }
}
