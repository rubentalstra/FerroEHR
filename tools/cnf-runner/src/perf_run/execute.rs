//! The per-stage wire realization: each [`crate::perf::PerfOp`] maps to
//! its committed ITS-REST operation binding (`artifacts/bindings/its-rest/`
//! — create EHR 201, commit COMPOSITION 201 with uid via `ETag`, versioned
//! update via `If-Match`, directory 201/200/204, contribution 201, ad-hoc
//! and stored query 200, template list/get 200, tags 200). Anything else
//! observed counts as an error arrival.
//!
//! Dependent stages resolve prerequisites from [`CaptureStore`] — the
//! journey-instance state earlier stages captured (a fresh EHR's id, a
//! commit's version uid) — falling back to the standing ward's seeded
//! state ([`crate::perf_run::corpus::WardPatient`]). NOTHING BLOCKS: a
//! prerequisite genuinely absent at fire time (the SUT has not landed the
//! earlier stage) is an honest error observation — that IS the
//! measurement.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::perf::PerfOp;
use crate::perf_run::client::{
    PerfClient, location_last_segment, object_uid_of, strip_weak_quotes,
};
use crate::perf_run::corpus::{ADHOC_AQL, STORED_QUERY_NAME, SeededCorpus, WARD_AQL};
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
}

/// Per-patient rolling version state (standing-ward journeys): the latest
/// known `OBJECT_VERSION_ID` per ward document, advanced by each versioned
/// update so successive corrections chain `If-Match` correctly.
#[derive(Debug, Default)]
#[allow(clippy::struct_field_names)] // each field IS an ovid of a distinct document
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

    fn journey<R>(&self, id: u64, f: impl FnOnce(&mut JourneyState) -> R) -> Option<R> {
        let shard = usize::try_from(id).unwrap_or(0) % SHARDS;
        let mut map = self.journeys.get(shard)?.lock().ok()?;
        Some(f(map.entry(id).or_default()))
    }

    fn patient<R>(&self, index: usize, f: impl FnOnce(&mut PatientState) -> R) -> Option<R> {
        let shard = index % SHARDS;
        let mut map = self.patients.get(shard)?.lock().ok()?;
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
fn note(observed: &mut Option<u16>, status: u16) -> u16 {
    *observed = Some(status);
    status
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
#[allow(clippy::too_many_lines)] // one match arm per closed-vocabulary operation
pub(crate) fn perform(
    client: &PerfClient,
    arrival_index: u64,
    planned: &PlannedArrival,
    corpus: &SeededCorpus,
    journey_pack: &JourneyPack,
    captures: &CaptureStore,
    observed: &mut Option<u16>,
) -> Result<bool, String> {
    let offset_s = planned.at.as_secs();
    let journey = planned.journey;

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
            if note(observed, reply.status) == 201
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
            note(observed, reply.status) == 200
        }
        PerfOp::EhrStatusRead => {
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/ehr_status"),
                None,
                false,
                None,
            )?;
            if note(observed, reply.status) == 200
                && let Some(ovid) = reply.etag.as_deref().map(strip_weak_quotes)
            {
                if let Some(patient) = planned.patient {
                    captures.patient(patient, |s| s.status_ovid = Some(ovid));
                } else {
                    captures.journey(journey, |s| s.status_ovid = Some(ovid));
                }
            }
            note(observed, reply.status) == 200
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
            let ok = matches!(note(observed, reply.status), 200 | 204);
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
            if note(observed, reply.status) == 201
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
            note(observed, reply.status) == 200
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
            note(observed, reply.status) == 200
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
            note(observed, reply.status) == 200
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
            let ok = matches!(note(observed, reply.status), 200 | 204);
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
            note(observed, reply.status) == 204
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
            if note(observed, reply.status) == 201
                && let Some(ovid) = reply.etag.as_deref().map(strip_weak_quotes)
            {
                captures.journey(journey, |s| s.directory_ovid = Some(ovid));
            }
            note(observed, reply.status) == 201
        }
        PerfOp::DirectoryRead => {
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/ehr/{ehr_id}/directory"),
                None,
                false,
                None,
            )?;
            if note(observed, reply.status) == 200
                && let Some(patient) = planned.patient
                && let Some(ovid) = reply.etag.as_deref().map(strip_weak_quotes)
            {
                captures.patient(patient, |s| s.directory_ovid = Some(ovid));
            }
            note(observed, reply.status) == 200
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
            let ok = matches!(note(observed, reply.status), 200 | 204);
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
            if note(observed, reply.status) == 201 {
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
            note(observed, reply.status) == 200
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
            note(observed, reply.status) == 200
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
            note(observed, reply.status) == 200
        }
        PerfOp::StoredQueryExecute => {
            let reply = client.request(
                reqwest::Method::GET,
                &format!("/query/{STORED_QUERY_NAME}?ehr_id={ehr_id}"),
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == 200
        }
        PerfOp::TemplateList => {
            let reply = client.request(
                reqwest::Method::GET,
                "/definition/template/adl1.4",
                None,
                false,
                None,
            )?;
            note(observed, reply.status) == 200
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
            note(observed, reply.status) == 200
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
            // 200 (stored collection) or 204 (no content) — both are the
            // successful full-collection replace.
            matches!(note(observed, reply.status), 200 | 201 | 204)
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
            note(observed, reply.status) == 200
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
    if reply.status == 200 {
        reply.etag.as_deref().map(strip_weak_quotes)
    } else {
        None
    }
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
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
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

    #[test]
    fn strides_cycle_the_pool() {
        let a = stride(1) % 97;
        let b = stride(2) % 97;
        assert_ne!(a, b);
    }
}
