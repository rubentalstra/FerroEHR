// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The run orchestration: select → execute (the interpreter over the live
//! driver) → record — producing the `results.json` outcomes the party layer
//! emits and the verdict pipeline consumes.
//!
//! Interpreter-coverage accounting is first-class: every case that cannot
//! be interpreter-run is a REGISTERED EXCEPTION with its reason (the
//! ≥90%-interpreter-run gate is computed, never asserted).

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use crate::artifacts::ArtifactSet;
use crate::exec::driver::HttpDriver;
use crate::exec::{CaseRecord, RowOutcome, run_case};
use crate::ids::{CapabilityName, InstanceName, SmOperationRef};
use crate::ixit::Ixit;
use crate::model::assertion::assertion_refs;
use crate::model::case::{CaseCore, PartyRelationshipRequirement};
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
    /// One record per executed case×format, in execution order.
    pub records: Vec<CaseRecord>,
    /// Cases the interpreter could not drive, each with its reason.
    pub exceptions: Vec<(crate::ids::CaseId, Exception)>,
    /// Cases the interpreter drove end-to-end.
    pub interpreter_run: usize,
    /// All active assertion-machinery cases considered.
    pub considered: usize,
    /// The `restapi_specs_version` the System OPTIONS manifest served, when
    /// the campaign drove that exchange — an independent confirmation of the
    /// party's declared `spec_versions.its_rest`, never a source of truth
    /// (the released `Options` schema has no `required` list; a divergence
    /// becomes a static-review finding, not a re-declaration).
    pub restapi_specs_version: Option<String>,
}

impl RunReport {
    /// The interpreter-coverage fraction (the ≥90% gate input).
    #[must_use]
    pub fn interpreter_coverage(&self) -> f64 {
        if self.considered == 0 {
            return 1.0;
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "case counts << 2^52"
        )]
        {
            self.interpreter_run as f64 / self.considered as f64
        }
    }
}

/// Whether every operation the case's flow calls is unrealized on this ITS.
///
/// Shared with the `claim-completeness` gate ([`crate::validate`]), which
/// needs the same catalogue-side predicate to tell a case that can carry
/// executed evidence from one that will always resolve excused.
pub(crate) fn fully_unrealized(set: &ArtifactSet, case: &CaseCore) -> Option<String> {
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

/// The binding a flow step drives — the driver's variant-aware selection, so
/// selection-time guards judge exactly the realization the driver will send.
fn step_binding<'a>(
    set: &'a ArtifactSet,
    case: &CaseCore,
    step: &crate::model::case::FlowStep,
) -> Option<&'a crate::model::binding::OperationBinding> {
    let op = if step.call.contains('.') {
        SmOperationRef::parse(&step.call).ok()?
    } else {
        case.sm_operation.as_ref()?.sibling(&step.call)
    };
    let mut bindings = set.bindings.iter().map(|(_, b)| b);
    if let Some(variant) = step.variant.as_deref()
        && let Some(exact) = bindings
            .clone()
            .find(|b| b.sm_operation == op && b.variant.as_deref() == Some(variant))
    {
        return Some(exact);
    }
    bindings.find(|b| b.sm_operation == op && b.variant.is_none())
}

/// The OPERATION-level spec-version floors this party does not meet
/// (`OperationBinding::applies`, issue #629 — the field was deserialized and
/// read by nothing).
///
/// A binding declares a floor when the WIRE itself arrived in a later
/// release: driving it against a party that declares an earlier one asks a
/// server for an endpoint or request form its release never defined, which is
/// a selection question (ISO/IEC 9646), not a conformance failure. The case is
/// therefore not-applicable with the citation, exactly as an undeclared option
/// branch or an undeclared ixit fact is.
///
/// This is the OPERATION level only. A release that merely dates how an
/// ANSWER must look (the `W/` weakness indicator, the read/DELETE `Location`
/// restriction) puts its floor on the header expectation instead, so the
/// operation stays driven and only that one rule is out of scope — see
/// [`crate::model::binding::HeaderExpectation`].
fn unmet_binding_floors(
    set: &ArtifactSet,
    case: &CaseCore,
    versions: &crate::party::SpecVersions,
) -> Vec<String> {
    let mut unmet = Vec::new();
    for step in &case.flow {
        let Some(binding) = step_binding(set, case, step) else {
            continue;
        };
        let Some(applies) = &binding.applies else {
            continue;
        };
        if applies.satisfied_by(versions) {
            continue;
        }
        let declared: Vec<String> = applies
            .entries()
            .into_iter()
            .map(|(component, range)| format!("{} {}", component.token(), range.raw()))
            .collect();
        let citation = format!("{} requires {}", binding.sm_operation, declared.join(", "));
        if !unmet.contains(&citation) {
            unmet.push(citation);
        }
    }
    unmet
}

/// The `served_extensions` family + adjudicating register entry of the first
/// EXTENSION binding the case's flow drives, if any — the marker that the case
/// verifies a route no openEHR specification governs (our own
/// design/extension). The register id travels with it so the not-applicable
/// citation is register-linked like every other excused row.
fn extension_family(set: &ArtifactSet, case: &CaseCore) -> Option<String> {
    let anchor = case.sm_operation.as_ref()?;
    for step in &case.flow {
        let op = if step.call.contains('.') {
            SmOperationRef::parse(&step.call).ok()?
        } else {
            anchor.sibling(&step.call)
        };
        if let Some(decl) = set
            .bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| b.sm_operation == op)
            .and_then(|b| b.extension.as_ref())
        {
            return Some(format!("{}; {}", decl.family, decl.ambiguity));
        }
    }
    None
}

/// The capabilities the catalogue's own cases put a verdict on when they drive
/// the `family` extension route — the ONLY way a party statement can say it
/// serves a route no openEHR specification governs.
fn capabilities_claiming_family(set: &ArtifactSet, family: &str) -> Vec<CapabilityName> {
    let mut claiming: Vec<CapabilityName> = Vec::new();
    for (_, case) in &set.cases {
        if !extension_family(set, case)
            .is_some_and(|marker| marker.starts_with(&format!("{family};")))
        {
            continue;
        }
        for capability in &case.capabilities {
            if !claiming.contains(capability) {
                claiming.push(capability.clone());
            }
        }
    }
    claiming
}

/// Why THIS party cannot have a `requires.import` case driven: the extract
/// replay is an EXTENSION route (ITS-REST 1.1.0 publishes no MESSAGE /
/// EHR-Extract API at all — register AMB-34), so a party that claims none of
/// the capabilities that family's cases gate has no import to precondition
/// with.
///
/// The scoping is the same law the extension arm of
/// [`selection_exception`] applies to a case's FLOW, moved to its
/// PRECONDITION: the case's own subject is a released read, and driving it
/// against a party that serves no import route would record a red row for a
/// ground that party never offered to establish. Excused at SELECTION time —
/// never as a drive-time provisioning refusal, which reads like a SUT defect.
///
/// # Errors
/// An interpreter defect: one of the SM operation anchors this arm is written
/// against is not a well-formed `I_<INTERFACE>.<operation>` reference.
fn unservable_import(
    set: &ArtifactSet,
    statement: Option<&crate::party::Statement>,
    case: &CaseCore,
) -> Result<Option<String>, String> {
    if !matches!(
        case.requires.import,
        Some(crate::model::case::ImportRequirement::Received { .. })
    ) {
        return Ok(None);
    }
    // Either receiving situation of master06 §Copying drives the same family;
    // whichever binding the catalogue realizes names it.
    unservable_provisioning(
        set,
        statement,
        &[
            "I_EHR_EXTRACT_SERVICE.import_ehr_extract",
            "I_EHR_EXTRACT_SERVICE.import_ehr",
        ],
        "requires.import",
        "the received version this case reads cannot exist here",
    )
}

/// Why THIS party cannot have a `requires.party_relationship` case driven:
/// the relationship create is an EXTENSION route (ITS-REST 1.1.0 surfaces no
/// `PARTY_RELATIONSHIP` resource — register AMB-32), exactly as
/// [`unservable_import`]'s extract replay is, so a party that claims none of
/// the capabilities that family's cases gate has no relationship to
/// precondition with.
///
/// # Errors
/// An interpreter defect: an SM operation anchor this arm is written against
/// is not a well-formed `I_<INTERFACE>.<operation>` reference.
fn unservable_party_relationship(
    set: &ArtifactSet,
    statement: Option<&crate::party::Statement>,
    case: &CaseCore,
) -> Result<Option<String>, String> {
    if !matches!(
        case.requires.party_relationship,
        Some(PartyRelationshipRequirement::Exists { .. })
    ) {
        return Ok(None);
    }
    unservable_provisioning(
        set,
        statement,
        &["I_DEMOGRAPHIC_SERVICE.create_party_relationship"],
        "requires.party_relationship",
        "the relationship this case reads cannot exist here",
    )
}

/// The shared scoping for a PRECONDITION that provisions over an EXTENSION
/// route: a party claiming none of the capabilities that family's cases gate
/// serves no such route, so the precondition cannot be established there.
///
/// The scoping is the same law the extension arm of [`selection_exception`]
/// applies to a case's FLOW, moved to its PRECONDITION: the case's own
/// subject is a released operation, and driving it against a party that
/// serves no provisioning route would record a red row for a ground that
/// party never offered to establish. Excused at SELECTION time — never as a
/// drive-time provisioning refusal, which reads like a SUT defect.
///
/// `operations` are the SM operations whose realized binding declares the
/// family (the first one the catalogue realizes decides); `requirement` names
/// the precondition for the citation, and `consequence` says what the case
/// therefore cannot read.
///
/// # Errors
/// An interpreter defect: an `operations` anchor is not a well-formed
/// `I_<INTERFACE>.<operation>` reference. A malformed anchor matches no
/// binding, so swallowing the parse would silently turn this whole arm off —
/// every `requires`-provisioned case would then DRIVE against a party that
/// serves no such route and record a red row for a ground it never offered to
/// establish, which is the opposite of what this selection law exists to do.
fn unservable_provisioning(
    set: &ArtifactSet,
    statement: Option<&crate::party::Statement>,
    operations: &[&str],
    requirement: &str,
    consequence: &str,
) -> Result<Option<String>, String> {
    let Some(statement) = statement else {
        return Ok(None);
    };
    let mut parsed: Vec<SmOperationRef> = Vec::with_capacity(operations.len());
    for call in operations {
        parsed.push(SmOperationRef::parse(call).map_err(|e| {
            format!("interpreter defect: selection-law SM operation anchor {call:?}: {e}")
        })?);
    }
    let Some(decl) = parsed.iter().find_map(|op| {
        set.bindings
            .iter()
            .map(|(_, b)| b)
            .find(|b| b.sm_operation == *op)
            .and_then(|b| b.extension.as_ref())
    }) else {
        return Ok(None);
    };
    let claiming = capabilities_claiming_family(set, &decl.family);
    if claiming
        .iter()
        .any(|c| statement.claims.capabilities.contains(c))
    {
        return Ok(None);
    }
    Ok(Some(format!(
        "{requirement} provisions over the {} extension routes ({}): the ICS claims none of \
         the capabilities those routes' cases gate ({}), and no openEHR specification governs \
         them, so {consequence}",
        decl.family,
        decl.ambiguity,
        claiming
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )))
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
            IxitField::DumpLocation => ixit.dump_location.is_none(),
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

/// The instances a case's flow addresses: every `on:` selector, and the
/// default `sut` when any step carries none.
pub(crate) fn addressed_instances(case: &CaseCore) -> Vec<InstanceName> {
    let mut named: Vec<InstanceName> = Vec::new();
    let mut any_default = false;
    for step in &case.flow {
        match &step.on {
            Some(name) if !named.contains(name) => named.push(name.clone()),
            Some(_) => {}
            None => any_default = true,
        }
    }
    if any_default && let Ok(default) = InstanceName::parse("sut") {
        named.insert(0, default);
    }
    named
}

/// Why THIS party's terminology declaration does not satisfy the case's
/// `requires.terminology` — the selection guard for every terminology-backed
/// behaviour.
///
/// Released ITS-REST 1.1.0 surfaces no terminology resource (the nine
/// `I_TERMINOLOGY_SERVICE` rows of `vocab/wire_surface.yaml` record that
/// boundary), so which terminology servers a deployment is wired to, which
/// namespaces they answer for, and what it does with a value set it cannot
/// resolve are all deployment facts no released operation discloses. They are
/// therefore IXIT declarations, and an undeclared one costs COVERAGE, never
/// correctness: the case is not-applicable with the citation rather than
/// driven against a server the party never seeded.
fn unsatisfied_terminology(case: &CaseCore, ixit: &Ixit) -> Option<String> {
    let required = case.requires.terminology.as_ref()?;
    for name in addressed_instances(case) {
        // An undeclared instance is already the `undeclared_instances` guard's
        // business; skip it here so one case never reports two citations.
        let Some(instance) = ixit.instance(&name) else {
            continue;
        };
        let Some(lane) = ixit.terminology_of(instance) else {
            return Some(format!(
                "instance {name}: the ixit declares no `terminology` posture — the case needs a \
                 deployment wired to a terminology query server (BASE master12 §Binding \
                 Terminology Value-sets to Archetypes), and no released operation discloses one"
            ));
        };
        if let Some(posture) = required.posture
            && lane.posture != posture
        {
            return Some(format!(
                "instance {name}: the case needs the `{}` unresolvable-value-set posture and this \
                 deployment declares `{}` (register AMB-172 — a deployment realizes exactly one)",
                posture.token(),
                lane.posture.token()
            ));
        }
        for namespace in &required.served {
            match lane.server_for(namespace) {
                Some(server) if server.is_reachable() => {}
                Some(server) => {
                    return Some(format!(
                        "instance {name}: terminology namespace {namespace} is declared on server \
                         '{}', which the ixit declares unreachable — the case needs it answered",
                        server.name
                    ));
                }
                None => {
                    return Some(format!(
                        "instance {name}: no declared terminology server answers for {namespace} \
                         — the party seeded no such namespace"
                    ));
                }
            }
        }
        for namespace in &required.unreachable {
            match lane.server_for(namespace) {
                Some(server) if server.is_reachable() => {
                    return Some(format!(
                        "instance {name}: terminology namespace {namespace} is declared reachable \
                         on server '{}' — the case needs the terminology-server-down branch, \
                         which only a declared-unreachable server provides",
                        server.name
                    ));
                }
                Some(_) => {}
                None => {
                    return Some(format!(
                        "instance {name}: no declared terminology server answers for {namespace} \
                         — the party declares no such unreachable namespace"
                    ));
                }
            }
        }
        if let Some(minimum) = required.distinct_servers {
            let count = lane.distinct_reachable_servers(&required.served);
            if count < minimum {
                return Some(format!(
                    "instance {name}: the case needs {minimum} distinct reachable terminology \
                     servers across its namespaces and the ixit declares {count} (BASE master12 \
                     §Overview — several terminologies served at the same time)"
                ));
            }
        }
    }
    None
}

/// Why THIS party's declared openEHR generation set does not satisfy the
/// case's `requires.spec_profile` — the selection guard for every behaviour
/// whose expectation rests on which generation set a deployment runs.
///
/// No released openEHR operation discloses the generation set a server
/// implements, and the release strategy
/// (<https://specifications.openehr.org/governance/release_strategy>) makes a
/// minor release a compatible superset — so the RELEASED and development sets
/// accept different surface while looking identical on every wire the release
/// defines. The set is therefore an IXIT declaration, per instance first with
/// the party default filling in ([`Ixit::spec_profile_of`]), and an undeclared
/// or differently-declared one costs COVERAGE, never correctness. A
/// multi-instance case states a per-instance need in `requires.instances`;
/// the case-level `requires.spec_profile` binds every addressed instance.
fn unsatisfied_spec_profile(case: &CaseCore, ixit: &Ixit) -> Option<String> {
    let per_instance = case.requires.instances.as_ref();
    for name in addressed_instances(case) {
        // An undeclared instance is already the `undeclared_instances` guard's
        // business; skip it here so one case never reports two citations.
        let Some(instance) = ixit.instance(&name) else {
            continue;
        };
        let required = per_instance
            .and_then(|map| map.get(&name))
            .and_then(|requires| requires.spec_profile)
            .or(case.requires.spec_profile);
        let Some(required) = required else {
            continue;
        };
        match ixit.spec_profile_of(instance) {
            None => {
                return Some(format!(
                    "instance {name}: the ixit declares no `spec_profile` — the case's \
                     expectation rests on the `{}` generation set, and no released operation \
                     discloses which set a deployment runs (openEHR release strategy: a minor \
                     release is a compatible superset, so the sets differ only in accepted \
                     surface)",
                    required.token()
                ));
            }
            Some(declared) if declared != required => {
                return Some(format!(
                    "instance {name}: the case needs the `{}` specification generation set and \
                     this deployment declares `{}` — one running server implements exactly one",
                    required.token(),
                    declared.token()
                ));
            }
            Some(_) => {}
        }
    }
    None
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

fn not_applicable_record(case: &CaseCore, citation: &str) -> CaseRecord {
    CaseRecord {
        case: case.id.clone(),
        format: None,
        rows: vec![RowOutcome::NotApplicable {
            citation: citation.to_owned(),
        }],
        rows_driven: 0,
        rows_total: crate::exec::row_count(case),
    }
}

/// The drive-time selection law (ISO/IEC 9646 ICS-driven selection + the
/// ixit declaration law): the FIRST ground that excuses `case` on this
/// party/deployment, or `None` when the case drives. Each arm carries its
/// citation inside the returned [`Exception`]; the caller records the same
/// citation as the case's single not-applicable row.
/// The EXTENSION arm of [`selection_exception`], covering both places an
/// extension route can enter a case: its FLOW (the case drives the route) and
/// its PRECONDITION (a received EHR-Extract, a provisioned party
/// relationship).
///
/// A route no openEHR specification governs is our own design/extension, so it
/// is behaviour only a party that CLAIMS the capability answers for. Driving it
/// at another vendor's SUT would publish failures for routes that vendor never
/// offered to serve — the published comparison must be honest in both
/// directions, and a spurious red row is not honesty.
///
/// # Errors
/// An interpreter defect propagated from the provisioning arms (a malformed
/// SM operation anchor).
fn unserved_extension(
    set: &ArtifactSet,
    statement: Option<&crate::party::Statement>,
    case: &CaseCore,
) -> Result<Option<String>, String> {
    if let Some(stmt) = statement
        && let Some(family) = extension_family(set, case)
        && !case
            .capabilities
            .iter()
            .any(|c| stmt.claims.capabilities.contains(c))
    {
        return Ok(Some(format!(
            "extension realization ({family}): the ICS claims none of this case's \
             capabilities, and no openEHR specification governs the route — ISO/IEC 9646 \
             test selection"
        )));
    }
    let citation = match unservable_import(set, statement, case)? {
        Some(citation) => Some(citation),
        None => unservable_party_relationship(set, statement, case)?,
    };
    Ok(citation.map(|citation| format!("{citation} — ISO/IEC 9646 test selection")))
}

/// Why the ICS puts `case` outside this party's test scope: it gates only
/// capabilities the statement does not claim.
///
/// ISO/IEC 9646 selects the test cases a party is answerable for from its ICS,
/// and the CNF profiles are exactly that list — "A profile may be defined
/// logically as a particular list of platform components and capabilities"
/// (CNF profiles `master02-overview.adoc` §Overview). The verdict pipeline
/// already selects on the same predicate ([`crate::verdict`] step 2), so a
/// driven row on an unclaimed capability can never reach a verdict: it lands
/// in the record as a failure against a surface the party never offered to
/// serve, and the record and the verdict then disagree about what a red row
/// means.
///
/// A case declaring no capability at all gates nothing and is never excused
/// here; without a statement (the statement-blind sweep) nothing is selected
/// away.
fn unclaimed_capabilities(
    statement: Option<&crate::party::Statement>,
    case: &CaseCore,
) -> Option<String> {
    let statement = statement?;
    if case.capabilities.is_empty()
        || case
            .capabilities
            .iter()
            .any(|c| statement.claims.capabilities.contains(c))
    {
        return None;
    }
    Some(format!(
        "the ICS claims none of the capabilities this case gates ({}) — CNF profiles \
         master02-overview.adoc §Overview (a profile IS the list of capabilities a solution \
         specifies); ISO/IEC 9646 test selection",
        case.capabilities
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// # Errors
/// An interpreter defect propagated from the extension arm (a malformed SM
/// operation anchor in the selection law).
fn selection_exception(
    set: &ArtifactSet,
    ixit: &Ixit,
    statement: Option<&crate::party::Statement>,
    case: &CaseCore,
) -> Result<Option<Exception>, String> {
    if let Some(citation) = fully_unrealized(set, case) {
        return Ok(Some(Exception::Unrealized(citation)));
    }
    if let Some(citation) = unserved_extension(set, statement, case)? {
        return Ok(Some(Exception::Unrealized(citation)));
    }
    // The general form of the arm above, and the same predicate the verdict
    // pipeline selects on: a case gating only unclaimed capabilities is out of
    // the scope this party's own ICS declares, so it is excused HERE with the
    // citation rather than driven into a red row no verdict will ever read.
    if let Some(citation) = unclaimed_capabilities(statement, case) {
        return Ok(Some(Exception::Guarded(citation)));
    }
    // An option branch the party statement does not declare is not this
    // SUT's behaviour — driving it records a spurious failure the verdict
    // pipeline would excuse anyway (`verdict::effective_outcome`); excuse it
    // at drive time with the same citation.
    if let Some(stmt) = statement
        && let Some(tag) = &case.option
        && !stmt.options.contains(tag)
    {
        return Ok(Some(Exception::Unrealized(format!(
            "option {tag}: the ICS does not declare this register branch \
             (statement.options) — ISO/IEC 9646 test selection"
        ))));
    }
    // Case-level spec-version floors (`CaseCore.applies`): a behaviour the
    // spec dates to a release the party does not declare is out of scope for
    // it — `Applies::satisfied_by`, the one polarity every consumer of the
    // floor uses (`verdict` selection re-applies the same predicate).
    // Driving such a case records a spurious failure against behaviour the
    // party never claimed (the 2026-07-28 java run drove 127 red rows and 13
    // spuriously green ones this way).
    if let Some(stmt) = statement
        && !case.applies.satisfied_by(&stmt.spec_versions)
    {
        let declared: Vec<String> = case
            .applies
            .entries()
            .into_iter()
            .map(|(component, range)| format!("{} {}", component.token(), range.raw()))
            .collect();
        return Ok(Some(Exception::Unrealized(format!(
            "case version floor unmet ({}) — the party's declared spec versions do not \
             satisfy the case's applies ranges; ISO/IEC 9646 test selection",
            declared.join(", ")
        ))));
    }
    // Operation-level spec-version floors (`OperationBinding.applies`): a
    // wire a later release introduced is not this party's behaviour to
    // answer for — the same selection question the option branch is, with
    // the binding's own declared range as the citation.
    if let Some(stmt) = statement {
        let unmet = unmet_binding_floors(set, case, &stmt.spec_versions);
        if !unmet.is_empty() {
            return Ok(Some(Exception::Unrealized(format!(
                "operation version floor unmet ({}) — the party's declared spec versions \
                 predate the release that introduced this wire; ISO/IEC 9646 test selection",
                unmet.join("; ")
            ))));
        }
    }
    // The SMART lane is a party declaration, exactly like the ixit facts
    // below: the CDR is a SMART resource server that never issues tokens
    // (ITS-REST docs/smart_app_launch/master06-authentication.adoc
    // §Supported Authentication Flows), so a chosen `scope` claim exists
    // only where the party declares a trusted test issuer to mint against.
    // Undeclared => not-applicable with the citation, never a spurious
    // failure against a deployment that legitimately does not run SMART.
    if needs_smart_lane(case) && ixit.smart.is_none() {
        return Ok(Some(Exception::Guarded(
            "the ixit declares no `smart` lane — the case needs a SMART-enabled \
             deployment and a minted, scope-carrying access token, neither of which any \
             released operation discloses or provides; ISO/IEC 9646 test selection"
                .to_owned(),
        )));
    }
    // The terminology deployment is a party declaration exactly like the
    // SMART lane above: released ITS-REST surfaces no terminology resource,
    // so nothing on the wire says which terminology servers a deployment
    // holds open or how it treats a value set it cannot resolve. Undeclared
    // or differently declared => not-applicable with the citation, never a
    // red row against a deployment that legitimately runs the other posture.
    if let Some(citation) = unsatisfied_terminology(case, ixit) {
        return Ok(Some(Exception::Guarded(format!(
            "{citation}; ISO/IEC 9646 test selection"
        ))));
    }
    // The openEHR specification generation set is the same class of party
    // declaration: no released operation discloses which one a deployment
    // runs, and one running server implements exactly one.
    if let Some(citation) = unsatisfied_spec_profile(case, ixit) {
        return Ok(Some(Exception::Guarded(format!(
            "{citation}; ISO/IEC 9646 test selection"
        ))));
    }
    // A flow step addressing an instance this party does not declare has no
    // ground to run on (the deployment or principal simply does not exist
    // here).
    let missing_instances = undeclared_instances(case, ixit);
    if !missing_instances.is_empty() {
        return Ok(Some(Exception::Guarded(format!(
            "the ixit declares no instance {} — the case's flow addresses it with `on:` and \
             this party runs no such deployment/principal; ISO/IEC 9646 test selection",
            missing_instances.join(", ")
        ))));
    }
    // A case reading a party-declared SUT fact this ixit does not carry
    // cannot be driven: the fact is not on the wire, so the alternative to a
    // declaration is a guess.
    let missing = undeclared_ixit_facts(case, ixit);
    if !missing.is_empty() {
        return Ok(Some(Exception::Guarded(format!(
            "the ixit declares no {} — the case reads it as ${{ixit:…}} and no released \
             operation discloses the value; ISO/IEC 9646 test selection",
            missing.join(", ")
        ))));
    }
    // Global-state grounds (an empty template list, a globally-absent
    // artefact) hold only on an exclusively-owned SUT; on a shared instance
    // the case is not-applicable, never a false verdict.
    if matches!(
        case.requires.server,
        Some(crate::vocab::ServerState::Exclusive)
    ) && !ixit
        .environment
        .as_ref()
        .is_some_and(|env| env.exclusive_server)
    {
        return Ok(Some(Exception::Unrealized(
            "requires.server: exclusive — the ixit declares a shared SUT instance \
             (environment.exclusive_server: false); the global-state ground cannot \
             be established"
                .to_owned(),
        )));
    }
    Ok(None)
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
/// The one not-applicable record shape every drive-time exclusion produces.
pub fn execute(
    set: &ArtifactSet,
    ixit: &Ixit,
    statement: Option<&crate::party::Statement>,
) -> Result<RunReport, String> {
    let mut report = RunReport::default();
    // Exclusive-server cases (global-state grounds like an empty template
    // list) run FIRST: on a freshly reset, exclusively-owned SUT their
    // ground holds only before other cases provision templates/queries.
    let mut ordered: Vec<&CaseCore> = set.cases.iter().map(|(_, c)| c).collect();
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
        if let Some(exception) = selection_exception(set, ixit, statement, case)? {
            let citation = match &exception {
                Exception::Unrealized(c)
                | Exception::ContentGeneration(c)
                | Exception::Guarded(c)
                | Exception::Status(c) => c.clone(),
            };
            report.records.push(not_applicable_record(case, &citation));
            report.exceptions.push((case.id.clone(), exception));
            continue;
        }
        let runnable = if matches!(case.kind, crate::vocab::CaseKind::Content) {
            // One executor serves both: a content row is a generate→commit→
            // expect functional execution over the synthesized flow.
            synthesize_content_case(case)
        } else {
            case.clone()
        };
        let mut driver = HttpDriver::new(set, ixit, statement.map(|s| &s.spec_versions))?;
        let record = run_case(&runnable, runnable.formats.first().copied(), &mut driver)?;
        report.interpreter_run += 1;
        report.records.push(record);
        if let Some(version) = driver.take_observed_restapi_specs_version() {
            report.restapi_specs_version.get_or_insert(version);
        }
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
    let mut ordered: Vec<&CaseCore> = set.cases.iter().map(|(_, c)| c).collect();
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

/// Whether a decision-table row's `violates` list names a MANDATORY-attribute
/// breach of the RM schema — the refusal the release puts on the 400 row.
///
/// ITS-REST `specifications/responses/422.yaml` scopes 422 to content that
/// "could be converted to a resource", while `responses/400.yaml` covers
/// "syntactically invalid header, parameter or content"; a body missing a
/// member the release's own request-body schema lists as `required:` never
/// converts, so it cannot reach the 422 branch. The discriminator is the
/// authored `rm_schema:` clause.
fn refused_at_parse(columns: &[String], row: &[serde_json::Value]) -> bool {
    // NOTE: register AMB-209 is the home of the boundary — a `rm_schema:`
    // clause about a VALUE's lexical form (a non-RFC-3986 `DV_URI.value`)
    // converts first, so only the mandatory-attribute class refuses at parse.
    columns
        .iter()
        .position(|column| column == "violates")
        .and_then(|index| row.get(index))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|violations| {
            violations
                .iter()
                .filter_map(serde_json::Value::as_str)
                .any(|violation| {
                    violation.starts_with("rm_schema:") && violation.contains("mandatory")
                })
        })
}

/// Synthesizes the functional execution of a content case.
///
/// The decision
/// table becomes a matrix (rows drive `${row.*}`), the flow is one commit of
/// the generated instance against the constraint context's template, and the
/// per-row `expected` column is the outcome expectation: `accepted` →
/// `created`, and `rejected` → `bad_request` or `validation_failed` by
/// `refused_at_parse`.
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
                            _ if refused_at_parse(&columns, row) => "bad_request",
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
    synthesized.sm_operation = SmOperationRef::parse("I_EHR_COMPOSITION.create_composition").ok();
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
mod tests {
    use super::*;

    /// The authored `rejected` token covers TWO wire outcomes, and the row's
    /// own `violates` list is the discriminator: a missing mandatory attribute
    /// never converts to the resource (`responses/400.yaml`, "syntactically
    /// invalid … content"), while a value that converts and then fails a
    /// constraint, an RM invariant, or its lexical form is the 422 branch
    /// (`responses/422.yaml`, "could be converted to a resource"). Row 1 is
    /// the malformed-URI class issue #1899 adjudicated onto the 422 side and
    /// register AMB-209 records.
    #[test]
    fn a_rejected_row_splits_on_its_violation_class() {
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "CONT-DV_URI-validate_open", "kind": "content", "component": "CONTENT",
            "rm_class": "DV_URI",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "decision_table": {
                "columns": ["value", "expected", "violates"],
                "rows": [
                    [null, "rejected", ["rm_schema: value is mandatory"]],
                    ["xyz", "rejected", ["rm_schema: value is not a valid RFC 3986 URI"]],
                    ["x", "rejected", ["constraint(pattern)"]],
                    ["y", "rejected", ["rm_invariant(DV_URI.Value_valid)"]],
                    ["z", "rejected", ["iso8601"]],
                    ["ftp://ftp.is.co.za/rfc/rfc1808.txt", "accepted", []]
                ]
            }
        }))
        .unwrap();
        let synthesized = synthesize_content_case(&case);
        let matrix = synthesized
            .parameters
            .as_ref()
            .and_then(|p| p.matrix.as_ref())
            .expect("the decision table becomes a parameters matrix");
        let column = matrix
            .columns
            .iter()
            .position(|c| c == "expected")
            .expect("the reserved `expected` column survives synthesis");
        let kinds: Vec<&str> = matrix
            .rows
            .iter()
            .map(|row| match row.get(column) {
                Some(crate::model::case::MatrixCell::Literal(serde_json::Value::String(s))) => {
                    s.as_str()
                }
                other => panic!("row expectation is not a literal kind: {other:?}"),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "bad_request",
                "validation_failed",
                "validation_failed",
                "validation_failed",
                "validation_failed",
                "created",
            ]
        );
    }

    /// `OperationBinding.applies` is LIVE (issue #629): a binding declaring a
    /// spec-version floor the party does not meet takes its cases out of
    /// scope with the binding's own declared range as the citation, and a
    /// binding without a floor is untouched.
    #[test]
    fn operation_version_floors_are_enforced_at_selection() {
        let floored: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_DEFINITION_ADL14.list_opts",
                "its": "its-rest",
                "applies": { "its_rest": ">=1.1.0" },
                "request": { "method": "GET", "path": "/definition/template/adl1.4" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let case: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-floored", "kind": "functional", "component": "DEFINITION_ADL14",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "expect": "ok" }]
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings
            .push((std::path::PathBuf::from("b.yaml"), floored));

        let old = crate::party::SpecVersions {
            its_rest: Some("1.0.3".to_owned()),
            ..crate::party::SpecVersions::default()
        };
        let unmet = unmet_binding_floors(&set, &case, &old);
        assert_eq!(unmet.len(), 1, "{unmet:?}");
        assert!(unmet[0].contains("I_DEFINITION_ADL14.list_opts"));
        assert!(unmet[0].contains(">=1.1.0"));

        let current = crate::party::SpecVersions {
            its_rest: Some("1.1.0".to_owned()),
            ..crate::party::SpecVersions::default()
        };
        assert!(unmet_binding_floors(&set, &case, &current).is_empty());

        // An undeclared floor never narrows selection.
        let unfloored: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_DEFINITION_ADL14.list_opts",
                "its": "its-rest",
                "request": { "method": "GET", "path": "/definition/template/adl1.4" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let mut plain = ArtifactSet::default();
        plain
            .bindings
            .push((std::path::PathBuf::from("b.yaml"), unfloored));
        assert!(unmet_binding_floors(&plain, &case, &old).is_empty());
    }

    /// An EXTENSION realization is party-scoped selection, not a global one:
    /// the family + register id travel in the marker so the citation is
    /// register-linked, and a case that drives no extension binding is
    /// untouched — an ordinary released-wire case can never be excused this
    /// way.
    #[test]
    fn extension_realizations_are_marked_with_their_family_and_register_entry() {
        let extension: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_PARTY_RELATIONSHIP.get_party_relationship",
                "its": "its-rest",
                "extension": {
                    "family": "party-relationship",
                    "reason": "the release surfaces no PARTY_RELATIONSHIP resource",
                    "source": "SM i_party_relationship.adoc vs ITS-REST demographic.openapi.yaml",
                    "ambiguity": "AMB-32"
                },
                "request": { "method": "GET", "path": "/demographic/party_relationship/{versioned_object_uid}" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let released: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_DEFINITION_ADL14.list_opts",
                "its": "its-rest",
                "request": { "method": "GET", "path": "/definition/template/adl1.4" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings
            .push((std::path::PathBuf::from("e.yaml"), extension));
        set.bindings
            .push((std::path::PathBuf::from("r.yaml"), released));

        let on_extension: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-extension", "kind": "functional", "component": "DEMOGRAPHIC",
            "sm_operation": "I_PARTY_RELATIONSHIP.get_party_relationship",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["PartyRelationshipOperations"],
            "flow": [{ "step": 1, "call": "get_party_relationship", "expect": "ok" }]
        }))
        .unwrap();
        let marker = extension_family(&set, &on_extension).expect("an extension marker");
        assert!(marker.contains("party-relationship"), "{marker}");
        assert!(
            marker.contains("AMB-32"),
            "the citation must stay register-linked: {marker}"
        );

        let on_released: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-released", "kind": "functional", "component": "DEFINITION_ADL14",
            "sm_operation": "I_DEFINITION_ADL14.list_opts",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "list_opts", "expect": "ok" }]
        }))
        .unwrap();
        assert!(extension_family(&set, &on_released).is_none());
    }

    /// A `requires.import` case is party-scoped on the capabilities the
    /// IMPORT family's cases gate, not on its own released-read ones: the
    /// precondition is established over an extension route, so a party that
    /// serves none has no received version for the read to serve.
    #[test]
    fn an_import_precondition_is_scoped_to_the_party_that_serves_the_family() {
        let import: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_EHR_EXTRACT_SERVICE.import_ehr_extract",
                "its": "its-rest",
                "extension": {
                    "family": "message-extract",
                    "reason": "the release publishes no MESSAGE API",
                    "source": "SM master09 vs the released ITS-REST groups",
                    "ambiguity": "AMB-34"
                },
                "request": { "method": "POST", "path": "/message/import/{an_ehr_id}" },
                "outcomes": { "updated": { "status": 204 } }
            }))
            .unwrap();
        let released: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_EHR_COMPOSITION.get_versioned_composition",
                "its": "its-rest",
                "request": { "method": "GET", "path": "/ehr/{ehr_id}/versioned_composition/{versioned_object_uid}/version/{version_uid}" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        // The catalogue's own import case is what says which capability the
        // family is claimed under.
        let importer: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-import", "kind": "functional", "component": "MESSAGING",
            "sm_operation": "I_EHR_EXTRACT_SERVICE.import_ehr_extract",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["EhrExtract"],
            "flow": [{ "step": 1, "call": "import_ehr_extract", "expect": "updated" }]
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings
            .push((std::path::PathBuf::from("i.yaml"), import));
        set.bindings
            .push((std::path::PathBuf::from("r.yaml"), released));
        set.cases
            .push((std::path::PathBuf::from("i-case.yaml"), importer));

        let reader: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-read", "kind": "functional", "component": "EHR_COMPOSITION",
            "sm_operation": "I_EHR_COMPOSITION.get_versioned_composition",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["Versioning"],
            "requires": {
                "ehr": { "commits": "none" },
                "import": {
                    "extract": "cnf.messaging.ehr_extract.v1",
                    "container": "X_VERSIONED_COMPOSITION"
                }
            },
            "flow": [{ "step": 1, "call": "get_versioned_composition", "expect": "ok" }]
        }))
        .unwrap();

        let statement = |caps: &[&str]| -> crate::party::Statement {
            serde_json::from_value(serde_json::json!({
                "product": { "name": "p", "version": "1", "vendor": "v", "identifier": "i" },
                "schedule_release": "CNF-2.0",
                "spec_versions": { "rm": "1.2.0", "its_rest": "1.1.0" },
                "claims": { "capabilities": caps, "profiles": ["CORE"] },
                "tech_profiles": [ { "its": "its-rest", "formats": ["canonical-json"] } ],
                "options": []
            }))
            .unwrap()
        };
        let serving = statement(&["EhrExtract", "Versioning"]);
        assert!(
            unservable_import(&set, Some(&serving), &reader)
                .expect("well-formed anchors")
                .is_none(),
            "a party claiming the family's capability drives the case"
        );

        // Claims the READ capability but not the import family — the case's
        // own capabilities must not be what decides this.
        let read_only = statement(&["Versioning"]);
        let citation = unservable_import(&set, Some(&read_only), &reader)
            .expect("well-formed anchors")
            .expect("excused with a citation");
        assert!(citation.contains("message-extract"), "{citation}");
        assert!(
            citation.contains("AMB-34"),
            "the citation stays register-linked: {citation}"
        );

        // A case with no import precondition is untouched.
        let plain: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-plain", "kind": "functional", "component": "EHR_COMPOSITION",
            "sm_operation": "I_EHR_COMPOSITION.get_versioned_composition",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["Versioning"],
            "flow": [{ "step": 1, "call": "get_versioned_composition", "expect": "ok" }]
        }))
        .unwrap();
        assert!(
            unservable_import(&set, Some(&read_only), &plain)
                .expect("well-formed anchors")
                .is_none()
        );
    }

    /// `requires.party_relationship` provisions over the SAME extension seam
    /// as `requires.import` (register AMB-32: ITS-REST 1.1.0 surfaces no
    /// `PARTY_RELATIONSHIP` resource), so a party claiming none of that
    /// family's capabilities is excused at SELECTION time with the citation —
    /// never driven into a provisioning refusal that reads like a SUT defect.
    #[test]
    fn a_party_relationship_precondition_is_scoped_at_selection() {
        let create: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_DEMOGRAPHIC_SERVICE.create_party_relationship",
                "its": "its-rest",
                "extension": {
                    "family": "party-relationship",
                    "reason": "the release surfaces no PARTY_RELATIONSHIP resource",
                    "source": "SM docs/UML/classes/i_demographic_service.adoc",
                    "ambiguity": "AMB-32"
                },
                "request": { "method": "POST", "path": "/demographic/party_relationship" },
                "outcomes": { "created": { "status": 201 } }
            }))
            .unwrap();
        let released: crate::model::binding::OperationBinding =
            serde_json::from_value(serde_json::json!({
                "sm_operation": "I_PARTY.get_party",
                "its": "its-rest",
                "request": { "method": "GET", "path": "/demographic/party/{party_id}" },
                "outcomes": { "ok": { "status": 200 } }
            }))
            .unwrap();
        // The family's own case is what says which capability claims it.
        let creator: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-rel", "kind": "functional", "component": "DEMOGRAPHIC",
            "sm_operation": "I_DEMOGRAPHIC_SERVICE.create_party_relationship",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["PartyRelationships"],
            "flow": [{ "step": 1, "call": "create_party_relationship", "expect": "created" }]
        }))
        .unwrap();
        let mut set = ArtifactSet::default();
        set.bindings
            .push((std::path::PathBuf::from("c.yaml"), create));
        set.bindings
            .push((std::path::PathBuf::from("r.yaml"), released));
        set.cases
            .push((std::path::PathBuf::from("c-case.yaml"), creator));

        let reader: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-party-read", "kind": "functional", "component": "DEMOGRAPHIC",
            "sm_operation": "I_PARTY.get_party",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["Demographics"],
            "requires": {
                "party_relationship": {
                    "source": "cnf.demographic.person.v1",
                    "target": "cnf.demographic.organisation.v1",
                    "relationship": "cnf.demographic.party_relationship.v1"
                }
            },
            "flow": [{ "step": 1, "call": "get_party", "expect": "ok" }]
        }))
        .unwrap();
        let statement = |caps: &[&str]| -> crate::party::Statement {
            serde_json::from_value(serde_json::json!({
                "product": { "name": "p", "version": "1", "vendor": "v", "identifier": "i" },
                "schedule_release": "CNF-2.0",
                "spec_versions": { "rm": "1.2.0", "its_rest": "1.1.0" },
                "claims": { "capabilities": caps, "profiles": ["CORE"] },
                "tech_profiles": [ { "its": "its-rest", "formats": ["canonical-json"] } ],
                "options": []
            }))
            .unwrap()
        };
        let serving = statement(&["PartyRelationships", "Demographics"]);
        assert!(
            unservable_party_relationship(&set, Some(&serving), &reader)
                .expect("well-formed anchors")
                .is_none(),
            "a party claiming the family's capability drives the case"
        );

        // Claims the READ capability but not the relationship family — the
        // case's own capabilities must not be what decides this.
        let read_only = statement(&["Demographics"]);
        let citation = unservable_party_relationship(&set, Some(&read_only), &reader)
            .expect("well-formed anchors")
            .expect("excused with a citation");
        assert!(citation.contains("party-relationship"), "{citation}");
        assert!(
            citation.contains("AMB-32"),
            "the citation stays register-linked: {citation}"
        );

        // `party_relationship: none` provisions nothing and is untouched.
        let plain: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-plain-party", "kind": "functional", "component": "DEMOGRAPHIC",
            "sm_operation": "I_PARTY.get_party",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["Demographics"],
            "requires": { "party_relationship": "none" },
            "flow": [{ "step": 1, "call": "get_party", "expect": "ok" }]
        }))
        .unwrap();
        assert!(
            unservable_party_relationship(&set, Some(&read_only), &plain)
                .expect("well-formed anchors")
                .is_none()
        );
    }

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

    /// The terminology posture is a DECLARED deployment fact: an undeclared
    /// lane, a mismatched posture, an unseeded namespace, a reachable server
    /// where the case needs the down branch, and too few simultaneous servers
    /// are each a selection outcome with its own citation — never a driven
    /// guess.
    #[test]
    fn terminology_requirements_are_selected_against_the_declaration() {
        let case = |requirement: serde_json::Value| -> CaseCore {
            serde_json::from_value(serde_json::json!({
                "id": "X-terminology", "kind": "functional", "component": "QUERY",
                "sm_operation": "I_QUERY_SERVICE.execute_ad_hoc_query",
                "test_purpose": "t", "description": "d", "spec_refs": [],
                "requires": { "server": "any", "terminology": requirement },
                "flow": [{ "step": 1, "call": "execute_ad_hoc_query", "expect": "ok" }]
            }))
            .unwrap()
        };
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } },
            "terminology": {
                "posture": "fail_open",
                "servers": [
                    { "name": "sct", "namespaces": ["urn:cnf:sct"] },
                    { "name": "loinc", "namespaces": ["urn:cnf:loinc"] },
                    { "name": "down", "reachable": false, "namespaces": ["urn:cnf:down"] }
                ]
            }
        }))
        .unwrap();

        // A case with no terminology requirement is untouched.
        let plain: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "X-plain", "kind": "functional", "component": "QUERY",
            "sm_operation": "I_QUERY_SERVICE.execute_ad_hoc_query",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "flow": [{ "step": 1, "call": "execute_ad_hoc_query", "expect": "ok" }]
        }))
        .unwrap();
        assert!(unsatisfied_terminology(&plain, &ixit).is_none());

        // Satisfied: served namespaces on two distinct reachable servers.
        assert!(
            unsatisfied_terminology(
                &case(serde_json::json!({
                    "posture": "fail_open",
                    "served": ["urn:cnf:sct", "urn:cnf:loinc"],
                    "distinct_servers": 2
                })),
                &ixit
            )
            .is_none()
        );

        // A party declaring no lane at all.
        let undeclared: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://x", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        let citation = unsatisfied_terminology(
            &case(serde_json::json!({ "served": ["urn:cnf:sct"] })),
            &undeclared,
        )
        .expect("undeclared lane is a selection outcome");
        assert!(
            citation.contains("declares no `terminology` posture"),
            "{citation}"
        );

        // The other posture.
        let citation = unsatisfied_terminology(
            &case(serde_json::json!({ "posture": "fail_closed" })),
            &ixit,
        )
        .expect("posture mismatch is a selection outcome");
        assert!(citation.contains("fail_closed"), "{citation}");

        // A namespace no declared server answers for.
        assert!(
            unsatisfied_terminology(
                &case(serde_json::json!({ "served": ["urn:cnf:absent"] })),
                &ixit
            )
            .is_some_and(|c| c.contains("urn:cnf:absent"))
        );

        // The down branch needs a DECLARED-unreachable server.
        assert!(
            unsatisfied_terminology(
                &case(serde_json::json!({ "unreachable": ["urn:cnf:down"] })),
                &ixit
            )
            .is_none()
        );
        assert!(
            unsatisfied_terminology(
                &case(serde_json::json!({ "unreachable": ["urn:cnf:sct"] })),
                &ixit
            )
            .is_some_and(|c| c.contains("declared reachable"))
        );

        // Simultaneity is counted over DISTINCT reachable servers.
        assert!(
            unsatisfied_terminology(
                &case(serde_json::json!({
                    "served": ["urn:cnf:sct"], "distinct_servers": 2
                })),
                &ixit
            )
            .is_some_and(|c| c.contains("2 distinct reachable"))
        );
    }

    /// The generation set is a DECLARED deployment fact: an undeclared set, a
    /// mismatched set, and a per-instance requirement against a per-instance
    /// declaration are each a selection outcome with its own citation — never
    /// a driven guess.
    #[test]
    fn spec_profile_requirements_are_selected_against_the_declaration() {
        let case = |requires: serde_json::Value| -> CaseCore {
            serde_json::from_value(serde_json::json!({
                "id": "X-profile", "kind": "functional", "component": "EHR_COMPOSITION",
                "sm_operation": "I_EHR_COMPOSITION.get_composition_at_version",
                "test_purpose": "t", "description": "d", "spec_refs": [],
                "requires": requires,
                "flow": [
                    { "step": 1, "call": "get_composition_at_version", "expect": "ok" },
                    { "step": 2, "call": "get_composition_at_version",
                      "on": "sut_stable", "expect": "conflict" }
                ]
            }))
            .unwrap()
        };
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://x", "auth": { "mode": "none" } },
                "sut_stable": { "base_url": "http://y", "auth": { "mode": "none" },
                                "spec_profile": "stable" }
            },
            "spec_profile": "development"
        }))
        .unwrap();

        // A case with no generation-set requirement is untouched.
        assert!(unsatisfied_spec_profile(&case(serde_json::json!({})), &ixit).is_none());

        // Satisfied: the case-level need matches the party default, and the
        // per-instance need matches the instance's own declaration.
        assert!(
            unsatisfied_spec_profile(
                &case(serde_json::json!({
                    "spec_profile": "development",
                    "instances": { "sut_stable": { "spec_profile": "stable" } }
                })),
                &ixit
            )
            .is_none()
        );

        // A per-instance requirement is read on its own, with no case-level
        // form present, and its mismatch names the instance.
        assert!(
            unsatisfied_spec_profile(
                &case(serde_json::json!({
                    "instances": { "sut_stable": { "spec_profile": "development" } }
                })),
                &ixit
            )
            .is_some_and(|c| c.contains("sut_stable") && c.contains("exactly one"))
        );

        // A mismatched case-level declaration names both sets.
        let citation = unsatisfied_spec_profile(
            &case(serde_json::json!({ "spec_profile": "stable" })),
            &ixit,
        )
        .expect("set mismatch is a selection outcome");
        assert!(
            citation.contains("`stable`") && citation.contains("`development`"),
            "{citation}"
        );

        // A party declaring no set at all.
        let undeclared: Ixit = serde_json::from_value(serde_json::json!({
            "instances": {
                "sut": { "base_url": "http://x", "auth": { "mode": "none" } },
                "sut_stable": { "base_url": "http://y", "auth": { "mode": "none" } }
            }
        }))
        .unwrap();
        let citation = unsatisfied_spec_profile(
            &case(serde_json::json!({ "spec_profile": "development" })),
            &undeclared,
        )
        .expect("undeclared set is a selection outcome");
        assert!(
            citation.contains("declares no `spec_profile`"),
            "{citation}"
        );
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
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "case counts << 2^52, so the coverage ratio is exact enough"
        )]
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

    fn statement(capabilities: &[&str]) -> crate::party::Statement {
        serde_json::from_value(serde_json::json!({
            "product": { "name": "p", "version": "1", "vendor": "v", "identifier": "i" },
            "schedule_release": "CNF-2.0",
            "spec_versions": { "rm": "1.2.0", "its_rest": "1.1.0" },
            "claims": { "capabilities": capabilities, "profiles": ["CORE"] },
            "tech_profiles": [ { "its": "its-rest", "formats": ["canonical-json"] } ],
            "options": []
        }))
        .unwrap()
    }

    /// Capability scoping is a DRIVE-TIME selection law, not a verdict-layer
    /// afterthought: a case gating only capabilities the ICS does not claim is
    /// recorded not-applicable with its citation, so the record and the
    /// verdict agree about what a red row means. A case sharing ONE claimed
    /// capability still drives, a capability-less case is never excused this
    /// way, and the statement-blind sweep selects nothing away.
    #[test]
    fn a_case_gating_only_unclaimed_capabilities_is_selected_away() {
        let signing: CaseCore = serde_json::from_value(serde_json::json!({
            "id": "SIG-VERSION-ehr_status_signature", "kind": "functional", "component": "EHR",
            "sm_operation": "I_EHR_STATUS.get_ehr_status",
            "test_purpose": "t", "description": "d", "spec_refs": [],
            "capabilities": ["Signing"],
            "flow": [{ "step": 1, "call": "get_ehr_status", "expect": "ok" }]
        }))
        .unwrap();
        let claimed = statement(&["EhrOperations", "Signing"]);
        let unclaimed = statement(&["EhrOperations"]);

        let citation = unclaimed_capabilities(Some(&unclaimed), &signing)
            .expect("an unclaimed capability takes the case out of scope");
        assert!(citation.contains("Signing"), "{citation}");
        assert!(citation.contains("ISO/IEC 9646"), "{citation}");
        assert!(unclaimed_capabilities(Some(&claimed), &signing).is_none());
        assert!(unclaimed_capabilities(None, &signing).is_none());

        let mut partly = signing.clone();
        partly.capabilities = vec![
            CapabilityName::parse("Signing").unwrap(),
            CapabilityName::parse("EhrOperations").unwrap(),
        ];
        assert!(unclaimed_capabilities(Some(&unclaimed), &partly).is_none());

        let mut capability_less = signing;
        capability_less.capabilities.clear();
        assert!(unclaimed_capabilities(Some(&unclaimed), &capability_less).is_none());
    }
}
