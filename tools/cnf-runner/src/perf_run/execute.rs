//! The per-stage wire realization.
//!
//! Each [`crate::perf::PerfOp`] maps to
//! its committed ITS-REST operation binding (`artifacts/bindings/its-rest/`
//! — create EHR / commit COMPOSITION / directory / contribution in the
//! created family with the uid via `ETag`/`Location`, versioned update via
//! `If-Match`, ad-hoc and stored query 200, template list/get 200, tags
//! 200). The driver sends `Prefer: return=minimal` on its writes, so the
//! created family accepts BOTH `201` and `204` (ITS-REST overview
//! `Requests_and_responses` §Prefer: "typically `201 Created`. If no
//! response body is returned, the service SHOULD use `204 No Content`").
//! Anything else observed counts as an error arrival.
//!
//! Dependent stages resolve prerequisites from the module's `CaptureStore` — the
//! journey-instance state earlier stages captured (a fresh EHR's id, a
//! commit's version uid) — falling back to the standing ward's seeded
//! state ([`crate::perf_run::corpus::WardPatient`]). NOTHING BLOCKS: a
//! prerequisite genuinely absent at fire time (the SUT has not landed the
//! earlier stage) is an honest error observation — that IS the
//! measurement.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::collections::HashMap;
use std::sync::Mutex;

use reqwest::StatusCode;

use crate::perf::PerfOp;
use crate::perf_run::client::{
    PerfClient, PerfPrincipals, location_last_segment, object_uid_of, strip_weak_quotes,
};
use crate::perf_run::corpus::{
    ADHOC_AQL, ANALYTICS_AQL, STORED_QUERY_NAME, SeededCorpus, TERMINOLOGY_AQL, WARD_AQL,
};
use crate::perf_run::pack::{self, JourneyPack};
use crate::perf_run::schedule::{PlannedArrival, WardDoc};

/// Per-journey-instance captured state (fresh-EHR journeys) — written by
/// creates/commits, read by the instance's later stages, dropped at the
/// instance's last in-window stage.
#[derive(Debug, Default, Clone)]
struct JourneyState {
    ehr_id: Option<String>,
    last_commit_ovid: Option<String>,
    directory_ovid: Option<String>,
    contribution_uid: Option<String>,
    status_ovid: Option<String>,
    /// The demographic PARTY this instance registered: its
    /// `versioned_object_uid` and the latest `OBJECT_VERSION_ID` (the
    /// If-Match the amendment chains on).
    party_uid: Option<String>,
    party_ovid: Option<String>,
    /// The `PARTY_RELATIONSHIP` this instance committed (extension route).
    relationship_uid: Option<String>,
}

/// Per-patient rolling version state (standing-ward journeys): the latest
/// known `OBJECT_VERSION_ID` per ward document, advanced by each versioned
/// update so successive corrections chain `If-Match` correctly.
#[derive(Debug, Default)]
#[expect(
    clippy::struct_field_names,
    reason = "each field IS an ovid of a distinct document"
)]
struct PatientState {
    gp_ovid: Option<String>,
    medlist_ovid: Option<String>,
    directory_ovid: Option<String>,
    status_ovid: Option<String>,
}

/// The capture store: sharded mutexes (journeys by instance id, patients
/// by index) — contention stays negligible against second-scale stage
/// spacing.
#[derive(Debug)]
pub(crate) struct CaptureStore {
    journeys: Vec<Mutex<HashMap<u64, JourneyState>>>,
    patients: Vec<Mutex<HashMap<usize, PatientState>>>,
}

const SHARDS: usize = 64;

impl CaptureStore {
    pub(crate) fn new() -> Self {
        Self {
            journeys: (0..SHARDS).map(|_| Mutex::new(HashMap::new())).collect(),
            patients: (0..SHARDS).map(|_| Mutex::new(HashMap::new())).collect(),
        }
    }

    // NOTE: a poisoned shard is RECOVERED, never dropped (#1853) — the guarded
    // value is a plain per-id map with no cross-entry invariant a panic
    // elsewhere can break, so `None` now means only "no such shard".
    fn journey<R>(&self, id: u64, f: impl FnOnce(&mut JourneyState) -> R) -> Option<R> {
        let shard = usize::try_from(id).unwrap_or(0) % SHARDS;
        let mut map = self
            .journeys
            .get(shard)?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Some(f(map.entry(id).or_default()))
    }

    fn patient<R>(&self, index: usize, f: impl FnOnce(&mut PatientState) -> R) -> Option<R> {
        let shard = index % SHARDS;
        let mut map = self
            .patients
            .get(shard)?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Some(f(map.entry(index).or_default()))
    }

    fn drop_journey(&self, id: u64) {
        let shard = usize::try_from(id).unwrap_or(0) % SHARDS;
        if let Some(mutex) = self.journeys.get(shard)
            && let Ok(mut map) = mutex.lock()
        {
            map.remove(&id);
        }
    }
}

/// Record the observed wire status (the failure-sampling channel: a
/// mismatched arrival reports WHAT the SUT answered, not just that it
/// mismatched).
///
/// The recorded channel is a bare `u16` because it is RENDERED into the run
/// record; the returned status stays typed so every caller compares against
/// a [`StatusCode`] constant.
fn note(observed: &mut Option<u16>, status: StatusCode) -> StatusCode {
    *observed = Some(status.as_u16());
    // A 429 anywhere invalidates the whole run: see
    // `crate::perf_run::rate_limited_observed`.
    if status == StatusCode::TOO_MANY_REQUESTS {
        crate::perf_run::note_rate_limited();
    }
    status
}

/// Whether a `Prefer: return=minimal` write landed in the created family.
/// ITS-REST overview `Requests_and_responses` §Prefer: the status is
/// "typically `201 Created`. If no response body is returned, the service
/// SHOULD use `204 No Content`" — both are conformant (upstream `EHRbase`
/// answers 204; this SUT answers 201). The identifying `ETag`/`Location`
/// is still demanded by each arm. Mirrors the seeder's acceptance
/// ([`crate::perf_run::corpus`]).
fn created(status: StatusCode) -> bool {
    status == StatusCode::CREATED || status == StatusCode::NO_CONTENT
}

/// Whether a `Prefer: return=minimal` versioned UPDATE landed: the same
/// §Prefer clause makes an empty body `204 No Content`, while a served
/// representation is `200 OK`.
fn updated(status: StatusCode) -> bool {
    status == StatusCode::OK || status == StatusCode::NO_CONTENT
}

/// Deterministic corpus addressing: a large odd stride cycles the pools.
fn stride(arrival: u64) -> u64 {
    arrival
        .checked_mul(2_654_435_761)
        .unwrap_or(arrival)
        .max(arrival)
}

/// Execute one planned arrival against the SUT.
///
/// Returns whether the wire outcome matched the binding's expected kind.
///
/// # Errors
/// A transport fault or an unresolvable prerequisite — both count as error
/// observations at the call site, never run failures.
#[expect(
    clippy::too_many_lines,
    reason = "one match arm per closed-vocabulary operation"
)]
pub(crate) fn perform(
    principals: &PerfPrincipals,
    arrival_index: u64,
    planned: &PlannedArrival,
    corpus: &SeededCorpus,
    journey_pack: &JourneyPack,
    captures: &CaptureStore,
    observed: &mut Option<u16>,
) -> Result<bool, String> {
    let offset_s = planned.at.as_secs();
    let journey = planned.journey;
    // The principal the operation is driven by. The schedule never plans an
    // arrival whose principal the party's ixit leaves undeclared, so this
    // resolution failing is an instrument defect, recorded as an honest
    // error arrival rather than a run failure.
    let principal = planned.op.principal();
    let client = principals.client(principal).ok_or_else(|| {
        format!(
            "the ixit declares no instance for the {principal:?} principal that {} needs",
            planned.op.as_str()
        )
    })?;

    // The EHR the stage addresses: the instance's fresh EHR, the standing
    // ward patient, or (read-only fallbacks) a corpus stride.
    let ehr_id: String = if let Some(patient) = planned.patient {
        corpus
            .ehr_ids
            .get(corpus.ward.get(patient).map_or(patient, |w| w.ehr_index))
            .cloned()
            .ok_or_else(|| "ward patient outside the corpus".to_owned())?
    } else if planned.op == PerfOp::EhrCreate {
        String::new() // created below
    } else {
        captures
            .journey(journey, |s| s.ehr_id.clone())
            .flatten()
            .ok_or_else(|| "prerequisite EHR not yet created (SUT stall)".to_owned())?
    };
    let ward = planned.patient.and_then(|p| corpus.ward.get(p));

    let ok = match planned.op {
        PerfOp::EhrCreate => {
            let reply = client.request(reqwest::Method::POST, "/ehr", None, true, None)?;
            if created(note(observed, reply.status))
                && let Some(id) = reply.location.as_deref().and_then(location_last_segment)
            {
                captures.journey(journey, |s| s.ehr_id = Some(id));
                true
            } else {
                false
            }
        }
        PerfOp::EhrRead => {
            let target = if ehr_id.is_empty() {
                // audit_review addresses the ward; fresh journeys their own
                return Err("ehr_read without a resolved EHR".to_owned());
            } else {
                ehr_id
            };
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{target}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::EhrStatusRead => {
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/ehr_status"),
                None,
                false,
                None,
            )?;
            if note(observed, reply.status) == StatusCode::OK
                && let Some(ovid) = reply.etag.as_deref().map(strip_weak_quotes)
            {
                if let Some(patient) = planned.patient {
                    captures.patient(patient, |s| s.status_ovid = Some(ovid));
                } else {
                    captures.journey(journey, |s| s.status_ovid = Some(ovid));
                }
            }
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::EhrStatusUpdate => {
            // If-Match from the journey's own status read (the ADT flow
            // reads before updating); an unread status is a stall.
            let preceding = planned
                .patient
                .and_then(|p| captures.patient(p, |s| s.status_ovid.clone()))
                .flatten()
                .or_else(|| {
                    captures
                        .journey(journey, |s| s.status_ovid.clone())
                        .flatten()
                })
                .ok_or_else(|| "prerequisite EHR_STATUS read has not landed".to_owned())?;
            let reply = client.request(
                reqwest::Method::PUT,
                &format!("/ehr/{ehr_id}/ehr_status"),
                Some(("application/json", pack::ehr_status_body(offset_s))),
                true,
                Some(&preceding),
            )?;
            let ok = updated(note(observed, reply.status));
            let ovid = if ok {
                reply.etag.as_deref().map(strip_weak_quotes)
            } else {
                refresh_current_ovid(client, &format!("/ehr/{ehr_id}/ehr_status"))
            };
            if let Some(ovid) = ovid {
                if let Some(patient) = planned.patient {
                    captures.patient(patient, |s| s.status_ovid = Some(ovid));
                } else {
                    captures.journey(journey, |s| s.status_ovid = Some(ovid));
                }
            }
            ok
        }
        PerfOp::CompositionCommit => {
            let template = planned
                .template
                .and_then(|i| journey_pack.get(i))
                .ok_or_else(|| "commit stage without a pack template".to_owned())?;
            let body = pack::composition_body(template, offset_s, arrival_index)?;
            let reply = client.request(
                reqwest::Method::POST,
                &format!("/ehr/{ehr_id}/composition"),
                Some(("application/json", body)),
                true,
                None,
            )?;
            if created(note(observed, reply.status))
                && let Some(uid) = reply.etag.as_deref().map(strip_weak_quotes)
            {
                captures.journey(journey, |s| s.last_commit_ovid = Some(uid));
                true
            } else {
                false
            }
        }
        PerfOp::CompositionRead => {
            // A committed-corpus read: the scale pool, stride-addressed.
            let n = corpus.compositions.len().max(1);
            let index = usize::try_from(stride(arrival_index)).unwrap_or(usize::MAX) % n;
            let (ehr_index, uid) = corpus
                .compositions
                .get(index)
                .ok_or_else(|| "corpus has no compositions".to_owned())?;
            let read_ehr = corpus
                .ehr_ids
                .get(*ehr_index)
                .ok_or_else(|| "corpus composition references a missing EHR".to_owned())?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{read_ehr}/composition/{uid}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::CompositionReadCurrent => {
            // The journey's own document: the instance's last commit, else
            // the ward chart (versioned-object read → latest version).
            let uid = captures
                .journey(journey, |s| s.last_commit_ovid.clone())
                .flatten()
                .map(|ovid| object_uid_of(&ovid))
                .or_else(|| ward.map(|w| object_uid_of(&w.gp_ovid)))
                .ok_or_else(|| "prerequisite commit has not landed (SUT stall)".to_owned())?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/composition/{uid}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::CompositionRevisionHistory => {
            let uid = current_doc_object_uid(planned, captures, ward)
                .ok_or_else(|| "no document for revision history".to_owned())?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/versioned_composition/{uid}/revision_history"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::CompositionUpdate => {
            let template = planned
                .template
                .and_then(|i| journey_pack.get(i))
                .ok_or_else(|| "update stage without a pack template".to_owned())?;
            let patient = planned
                .patient
                .ok_or_else(|| "versioned update addresses a ward patient".to_owned())?;
            // The rolling latest ovid (seeded, advanced per correction).
            let preceding = captures
                .patient(patient, |s| match planned.doc {
                    WardDoc::MedList => s.medlist_ovid.clone(),
                    WardDoc::Gp => s.gp_ovid.clone(),
                })
                .flatten()
                .or_else(|| {
                    ward.map(|w| match planned.doc {
                        WardDoc::MedList => w.medlist_ovid.clone(),
                        WardDoc::Gp => w.gp_ovid.clone(),
                    })
                })
                .ok_or_else(|| "no seeded ward document to update".to_owned())?;
            let object_uid = object_uid_of(&preceding);
            let body = pack::composition_body(template, offset_s, arrival_index)?;
            let reply = client.request(
                reqwest::Method::PUT,
                &format!("/ehr/{ehr_id}/composition/{object_uid}"),
                Some(("application/json", body)),
                true,
                Some(&preceding),
            )?;
            let ok = updated(note(observed, reply.status));
            let next = if ok {
                reply.etag.as_deref().map(strip_weak_quotes)
            } else {
                // Conflict/failure: re-resolve the current version so the
                // NEXT amendment chains correctly (see `refresh_current_ovid`).
                refresh_current_ovid(client, &format!("/ehr/{ehr_id}/composition/{object_uid}"))
            };
            if let Some(next) = next {
                captures.patient(patient, |s| match planned.doc {
                    WardDoc::MedList => s.medlist_ovid = Some(next),
                    WardDoc::Gp => s.gp_ovid = Some(next),
                });
            }
            ok
        }
        PerfOp::CompositionDelete => {
            // Deletes the journey's own commit (the deletion journey
            // commits first) — never a shared ward document.
            let preceding = captures
                .journey(journey, |s| s.last_commit_ovid.clone())
                .flatten()
                .ok_or_else(|| "prerequisite commit has not landed (SUT stall)".to_owned())?;
            let reply = client.request(
                reqwest::Method::DELETE,
                &format!("/ehr/{ehr_id}/composition/{preceding}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::NO_CONTENT
        }
        PerfOp::DirectoryCreate => {
            // Fresh-EHR journeys create their episode tree; the standing
            // ward already has one (seeded), so a 409/400 on re-create is
            // a real error — admission journeys always run on fresh EHRs.
            let reply = client.request(
                reqwest::Method::POST,
                &format!("/ehr/{ehr_id}/directory"),
                Some(("application/json", pack::folder_body(false))),
                true,
                None,
            )?;
            let ok = created(note(observed, reply.status));
            if ok && let Some(ovid) = reply.etag.as_deref().map(strip_weak_quotes) {
                captures.journey(journey, |s| s.directory_ovid = Some(ovid));
            }
            ok
        }
        PerfOp::DirectoryRead => {
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/directory"),
                None,
                false,
                None,
            )?;
            if note(observed, reply.status) == StatusCode::OK
                && let Some(patient) = planned.patient
                && let Some(ovid) = reply.etag.as_deref().map(strip_weak_quotes)
            {
                captures.patient(patient, |s| s.directory_ovid = Some(ovid));
            }
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::DirectoryUpdate => {
            let preceding = planned
                .patient
                .and_then(|p| captures.patient(p, |s| s.directory_ovid.clone()))
                .flatten()
                .or_else(|| ward.map(|w| w.directory_ovid.clone()))
                .or_else(|| {
                    captures
                        .journey(journey, |s| s.directory_ovid.clone())
                        .flatten()
                })
                .ok_or_else(|| "no directory version to update".to_owned())?;
            let reply = client.request(
                reqwest::Method::PUT,
                &format!("/ehr/{ehr_id}/directory"),
                Some(("application/json", pack::folder_body(true))),
                true,
                Some(&preceding),
            )?;
            let ok = updated(note(observed, reply.status));
            let next = if ok {
                reply.etag.as_deref().map(strip_weak_quotes)
            } else {
                refresh_current_ovid(client, &format!("/ehr/{ehr_id}/directory"))
            };
            if let Some(next) = next {
                if let Some(patient) = planned.patient {
                    captures.patient(patient, |s| s.directory_ovid = Some(next));
                } else {
                    captures.journey(journey, |s| s.directory_ovid = Some(next));
                }
            }
            ok
        }
        PerfOp::ContributionCommit => {
            let template = planned
                .template
                .and_then(|i| journey_pack.get(i))
                .ok_or_else(|| "contribution stage without a pack template".to_owned())?;
            let body = pack::contribution_body(template, offset_s, arrival_index)?;
            let reply = client.request(
                reqwest::Method::POST,
                &format!("/ehr/{ehr_id}/contribution"),
                Some(("application/json", body)),
                true,
                None,
            )?;
            if created(note(observed, reply.status)) {
                let uid = reply
                    .location
                    .as_deref()
                    .and_then(location_last_segment)
                    .or_else(|| reply.etag.as_deref().map(strip_weak_quotes));
                if let Some(uid) = uid {
                    captures.journey(journey, |s| s.contribution_uid = Some(uid));
                }
                true
            } else {
                false
            }
        }
        PerfOp::ContributionRead => {
            let uid = captures
                .journey(journey, |s| s.contribution_uid.clone())
                .flatten()
                .or_else(|| ward.map(|w| w.contribution_uid.clone()))
                .ok_or_else(|| "no contribution to inspect".to_owned())?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/contribution/{uid}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::AdhocQuery => {
            let body = serde_json::json!({
                "q": ADHOC_AQL,
                "query_parameters": { "ehr_id": ehr_id }
            });
            let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
            let reply = client.request(
                reqwest::Method::POST,
                "/query/aql",
                Some(("application/json", bytes)),
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::WardQuery => {
            let body = serde_json::json!({ "q": WARD_AQL });
            let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
            let reply = client.request(
                reqwest::Method::POST,
                "/query/aql",
                Some(("application/json", bytes)),
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::StoredQueryExecute => {
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/query/{STORED_QUERY_NAME}?ehr_id={ehr_id}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::TemplateList => {
            let reply = client.request(
                reqwest::Method::GET,
                "/definition/template/adl1.4",
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::TemplateGet => {
            // Stride across the pack (integration engines poll them all).
            let n = journey_pack.templates.len().max(1);
            let index = usize::try_from(stride(arrival_index)).unwrap_or(usize::MAX) % n;
            let template = journey_pack
                .get(index)
                .ok_or_else(|| "pack is empty".to_owned())?;
            let encoded = urlencoding::encode(&template.template_id);
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/definition/template/adl1.4/{encoded}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::TagsPut => {
            let uid = current_doc_object_uid(planned, captures, ward)
                .ok_or_else(|| "no document to tag".to_owned())?;
            let reply = client.request(
                reqwest::Method::PUT,
                &format!("/ehr/{ehr_id}/composition/{uid}/tags"),
                Some(("application/json", pack::tags_body(offset_s))),
                false,
                None,
            )?;
            // 200 (stored collection), 201 (first collection) or 204 (no
            // content) — each is the successful full-collection replace.
            let status = note(observed, reply.status);
            updated(status) || status == StatusCode::CREATED
        }
        PerfOp::TagsRead => {
            let uid = current_doc_object_uid(planned, captures, ward)
                .ok_or_else(|| "no document to read tags from".to_owned())?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/composition/{uid}/tags"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::CompositionVersionRead => {
            // The ORIGINAL_VERSION envelope (the signature carrier): the
            // instance's own commit, else the ward chart's seeded version.
            let ovid = current_version_uid(planned, captures, ward)
                .ok_or_else(|| "no committed version to read".to_owned())?;
            let vo_uid = object_uid_of(&ovid);
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/versioned_composition/{vo_uid}/version/{ovid}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::CompositionCommitFlat => {
            let flat = journey_pack
                .aux
                .flat
                .as_ref()
                .ok_or_else(|| "the pack carries no Simplified-FLAT payload".to_owned())?;
            let reply = client.request_negotiated(
                reqwest::Method::POST,
                &format!("/ehr/{ehr_id}/composition"),
                Some(("application/openehr.wt.flat+json", pack::flat_body(flat)?)),
                true,
                None,
                None,
                // ITS-REST overview Requests_and_responses §openehr-template-id
                // — a Simplified-Format commit names the template it is
                // constrained by, the format carrying no archetype details.
                &[("openehr-template-id", flat.template_id.clone())],
            )?;
            if created(note(observed, reply.status))
                && let Some(uid) = reply.etag.as_deref().map(strip_weak_quotes)
            {
                captures.journey(journey, |s| s.last_commit_ovid = Some(uid));
                true
            } else {
                false
            }
        }
        PerfOp::CompositionReadFlat => {
            let ovid = current_version_uid(planned, captures, ward)
                .ok_or_else(|| "no committed version to read as FLAT".to_owned())?;
            let reply = client.request_negotiated(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/composition/{ovid}"),
                None,
                false,
                None,
                Some("application/openehr.wt.flat+json"),
                &[],
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::PartyCreate => {
            let person = journey_pack
                .aux
                .person
                .as_ref()
                .ok_or_else(|| "the pack carries no PERSON payload".to_owned())?;
            let reply = client.request(
                reqwest::Method::POST,
                "/demographic/person",
                Some((
                    "application/json",
                    pack::person_body(person, arrival_index)?,
                )),
                true,
                None,
            )?;
            if created(note(observed, reply.status))
                && let Some(ovid) = reply
                    .etag
                    .as_deref()
                    .map(strip_weak_quotes)
                    .or_else(|| reply.location.as_deref().and_then(location_last_segment))
            {
                let uid = object_uid_of(&ovid);
                captures.journey(journey, |s| {
                    s.party_uid = Some(uid);
                    s.party_ovid = Some(ovid);
                });
                true
            } else {
                false
            }
        }
        PerfOp::PartyRead => {
            let uid = captures
                .journey(journey, |s| s.party_uid.clone())
                .flatten()
                .ok_or_else(|| "prerequisite PARTY has not landed (SUT stall)".to_owned())?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/demographic/person/{uid}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::PartyUpdate => {
            let amended = journey_pack
                .aux
                .person_amended
                .as_ref()
                .ok_or_else(|| "the pack carries no amended PERSON payload".to_owned())?;
            let (uid, preceding) = captures
                .journey(journey, |s| s.party_uid.clone().zip(s.party_ovid.clone()))
                .flatten()
                .ok_or_else(|| "prerequisite PARTY has not landed (SUT stall)".to_owned())?;
            let reply = client.request(
                reqwest::Method::PUT,
                &format!("/demographic/person/{uid}"),
                Some((
                    "application/json",
                    pack::person_body(amended, arrival_index)?,
                )),
                true,
                Some(&preceding),
            )?;
            let ok = updated(note(observed, reply.status));
            if ok && let Some(next) = reply.etag.as_deref().map(strip_weak_quotes) {
                captures.journey(journey, |s| s.party_ovid = Some(next));
            }
            ok
        }
        PerfOp::PartyRelationshipCreate => {
            let relationship = journey_pack
                .aux
                .party_relationship
                .as_ref()
                .ok_or_else(|| "the pack carries no PARTY_RELATIONSHIP payload".to_owned())?;
            let source = captures
                .journey(journey, |s| s.party_uid.clone())
                .flatten()
                .ok_or_else(|| "prerequisite PARTY has not landed (SUT stall)".to_owned())?;
            let reply = client.request(
                reqwest::Method::POST,
                "/demographic/party_relationship",
                Some((
                    "application/json",
                    pack::party_relationship_body(relationship, &source)?,
                )),
                true,
                None,
            )?;
            if created(note(observed, reply.status))
                && let Some(ovid) = reply
                    .etag
                    .as_deref()
                    .map(strip_weak_quotes)
                    .or_else(|| reply.location.as_deref().and_then(location_last_segment))
            {
                captures.journey(journey, |s| s.relationship_uid = Some(object_uid_of(&ovid)));
                true
            } else {
                false
            }
        }
        PerfOp::PartyRelationshipRead => {
            let uid = captures
                .journey(journey, |s| s.relationship_uid.clone())
                .flatten()
                .ok_or_else(|| {
                    "prerequisite PARTY_RELATIONSHIP has not landed (SUT stall)".to_owned()
                })?;
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/demographic/party_relationship/{uid}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::TemplateExample => {
            let n = journey_pack.templates.len().max(1);
            let index = usize::try_from(stride(arrival_index)).unwrap_or(usize::MAX) % n;
            let template = journey_pack
                .get(index)
                .ok_or_else(|| "pack is empty".to_owned())?;
            let encoded = urlencoding::encode(&template.template_id);
            let reply = client.request(
                reqwest::Method::GET,
                // The two query parameters the released operation declares
                // (`type`, `detail_level`).
                &format!(
                    "/definition/template/adl1.4/{encoded}/example?type=input&detail_level=required"
                ),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::TemplateAdl2List => {
            let reply = client.request(
                reqwest::Method::GET,
                "/definition/template/adl2",
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::ArchetypeAdl2List => {
            // EXTENSION route (register AMB-37) — no openEHR spec governs it;
            // it loads the ADL 2 archetype listing this product serves of its
            // own design.
            let reply = client.request(
                reqwest::Method::GET,
                "/definition/archetype/adl2",
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::AdminContributionReport => {
            // EXTENSION route (register AMB-33) — no openEHR spec governs it.
            // The EHR service is the only versioned-content service this
            // arrival reports on (SM platform_service.adoc); the count is a
            // pure read and mutates nothing.
            let reply = client.request(
                reqwest::Method::GET,
                "/admin/report/contribution/count?a_service=Ehr",
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::EhrExtractExport => {
            // EXTENSION route (register AMB-34) — no openEHR spec governs it.
            // A pure read: the EHR's content as a `List<EXTRACT>`.
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/message/export/{ehr_id}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::TddImport => {
            // EXTENSION route (register AMB-34) — no openEHR spec governs it.
            // The document converts against its operational template and
            // commits through the ordinary validated COMPOSITION path, so this
            // arrival is a real write.
            let tdd = journey_pack
                .aux
                .tdd
                .as_ref()
                .ok_or_else(|| "the pack carries no TDD payload".to_owned())?;
            let reply = client.request(
                reqwest::Method::POST,
                &format!("/message/tdd/{ehr_id}"),
                Some(("application/xml", tdd.document.as_bytes().to_vec())),
                false,
                None,
            )?;
            created(note(observed, reply.status))
        }
        PerfOp::AnalyticsQuery => {
            let body = serde_json::json!({
                "q": ANALYTICS_AQL,
                "query_parameters": { "ehr_id": ehr_id }
            });
            let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
            let reply = client.request(
                reqwest::Method::POST,
                "/query/aql",
                Some(("application/json", bytes)),
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::TerminologyQuery => {
            let body = serde_json::json!({ "q": TERMINOLOGY_AQL });
            let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
            let reply = client.request(
                reqwest::Method::POST,
                "/query/aql",
                Some(("application/json", bytes)),
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::SystemOptions => {
            let reply = client.request(reqwest::Method::OPTIONS, "/", None, false, None)?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::SmartConfigurationRead => {
            // Addressed at the PLATFORM base the ixit's `smart` lane names —
            // a different path root from the openEHR REST base (ITS-REST
            // docs/smart_app_launch/master04-service_discovery.adoc
            // §Service Discovery).
            let reply = client.request(
                reqwest::Method::GET,
                "/.well-known/smart-configuration",
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::OK
        }
        PerfOp::UnauthenticatedProbe => {
            // The DENY branch is the measured outcome: a credential-less
            // read must be refused, so 401 is the arrival's success and
            // anything else — 200 above all — is an error arrival.
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == StatusCode::UNAUTHORIZED
        }
        PerfOp::ReadonlyWriteDenied => {
            let template = planned
                .template
                .and_then(|i| journey_pack.get(i))
                .ok_or_else(|| "denied-write stage without a pack template".to_owned())?;
            let body = pack::composition_body(template, offset_s, arrival_index)?;
            let reply = client.request(
                reqwest::Method::POST,
                &format!("/ehr/{ehr_id}/composition"),
                Some(("application/json", body)),
                true,
                None,
            )?;
            // 403 is the arrival's success: the write is refused, so the
            // measured population is untouched by this probe.
            note(observed, reply.status) == StatusCode::FORBIDDEN
        }
    };

    if planned.last {
        captures.drop_journey(journey);
    }
    Ok(ok)
}

/// The versioned-object uid of the document a governance stage addresses:
/// the journey's own last commit, else the ward chart.
/// Re-resolve a ward document's CURRENT version after a failed versioned
/// update — what every real EHR client does on an optimistic-concurrency
/// conflict (re-read, then amend from the fresh version). Without it a
/// single lost race (409) or timed-out update leaves the tracked
/// `OBJECT_VERSION_ID` stale forever and every later update on that
/// patient fails — an instrument-made error cascade the first corrected-
/// pack ladder measured as a false knee (398/409 update failures at one
/// rung while the SUT answered every stale If-Match correctly). The
/// refresh rides INSIDE the failed arrival (its latency is that arrival's
/// honest conflict cost); the arrival still records as an error.
fn refresh_current_ovid(client: &PerfClient, path: &str) -> Option<String> {
    let reply = client
        .request(reqwest::Method::GET, path, None, false, None)
        .ok()?;
    if reply.status == StatusCode::OK {
        reply.etag.as_deref().map(strip_weak_quotes)
    } else {
        None
    }
}

/// The `OBJECT_VERSION_ID` of the version a provenance stage addresses:
/// the journey's own last commit, else the ward chart's seeded version.
fn current_version_uid(
    planned: &PlannedArrival,
    captures: &CaptureStore,
    ward: Option<&crate::perf_run::corpus::WardPatient>,
) -> Option<String> {
    captures
        .journey(planned.journey, |s| s.last_commit_ovid.clone())
        .flatten()
        .or_else(|| {
            ward.map(|w| match planned.doc {
                WardDoc::MedList => w.medlist_ovid.clone(),
                WardDoc::Gp => w.gp_ovid.clone(),
            })
        })
}

fn current_doc_object_uid(
    planned: &PlannedArrival,
    captures: &CaptureStore,
    ward: Option<&crate::perf_run::corpus::WardPatient>,
) -> Option<String> {
    captures
        .journey(planned.journey, |s| s.last_commit_ovid.clone())
        .flatten()
        .map(|ovid| object_uid_of(&ovid))
        .or_else(|| {
            ward.map(|w| match planned.doc {
                WardDoc::MedList => object_uid_of(&w.medlist_ovid),
                WardDoc::Gp => object_uid_of(&w.gp_ovid),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capture_store_scopes_journeys_and_drops_them() {
        let store = CaptureStore::new();
        store.journey(7, |s| s.ehr_id = Some("e-7".to_owned()));
        store.journey(7 + 64, |s| s.ehr_id = Some("e-71".to_owned()));
        assert_eq!(
            store.journey(7, |s| s.ehr_id.clone()).flatten().as_deref(),
            Some("e-7")
        );
        assert_eq!(
            store
                .journey(7 + 64, |s| s.ehr_id.clone())
                .flatten()
                .as_deref(),
            Some("e-71")
        );
        store.drop_journey(7);
        assert_eq!(store.journey(7, |s| s.ehr_id.clone()).flatten(), None);
        // patient state rolls forward
        store.patient(3, |s| s.gp_ovid = Some("g::s::1".to_owned()));
        store.patient(3, |s| s.gp_ovid = Some("g::s::2".to_owned()));
        assert_eq!(
            store.patient(3, |s| s.gp_ovid.clone()).flatten().as_deref(),
            Some("g::s::2")
        );
    }

    /// The failure-sampling channel is a RECORDED wire number: `window.rs`
    /// renders it into the run's progress record, so `note` must leave a bare
    /// `u16` there however the status is held while comparing.
    #[test]
    fn the_failure_sampling_channel_records_a_bare_wire_number() {
        let mut observed = None;
        let returned = note(&mut observed, StatusCode::NOT_FOUND);
        assert_eq!(returned, StatusCode::NOT_FOUND, "the caller compares typed");
        assert_eq!(observed, Some(404), "the recorded channel stays a number");
        assert_eq!(
            observed.map(|status| format!("unexpected wire status {status}")),
            Some("unexpected wire status 404".to_owned())
        );
    }

    /// The two `Prefer: return=minimal` acceptance families, pinned against
    /// the neighbouring codes a numeric comparison could have confused.
    #[test]
    fn the_prefer_minimal_families_accept_exactly_their_codes() {
        assert!(created(StatusCode::CREATED) && created(StatusCode::NO_CONTENT));
        assert!(!created(StatusCode::OK) && !created(StatusCode::ACCEPTED));
        assert!(updated(StatusCode::OK) && updated(StatusCode::NO_CONTENT));
        assert!(!updated(StatusCode::CREATED) && !updated(StatusCode::RESET_CONTENT));
    }

    #[test]
    fn strides_cycle_the_pool() {
        let a = stride(1) % 97;
        let b = stride(2) % 97;
        assert_ne!(a, b);
    }
}
