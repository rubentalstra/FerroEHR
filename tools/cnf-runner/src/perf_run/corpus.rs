// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The seeded corpus for measured runs.
//!
//! It is the `scale_ladder` volume (N EHRs × ~100 committed
//! blood-pressure versions, `corpus/recipes/scale_ladder.md`) plus the
//! STANDING WARD — the per-patient state the journey stages address
//! mid-flight (an episode directory, the GP-data-set chart document, the
//! medicines list, one committed CONTRIBUTION), seeded strictly through
//! the public API (never a database backdoor). The workflow always seeds
//! a freshly composed, empty SUT and tears the stack down afterwards —
//! there is no seed reuse.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::perf_run::client::{PerfClient, location_last_segment, strip_weak_quotes};
use crate::perf_run::pack;
use crate::perf_run::pack::JourneyPack;

/// The stored query the ward dashboard executes continuously (registered
/// once at seeding; `I_DEFINITION_QUERY.store_query` → wire 200).
///
/// NOTE: the namespaced form is the catalogue's own house convention —
/// chosen as the workload constant so the workload runs on SUTs that
/// (non-conformantly) reject the equally spec-valid namespace-less dotted
/// form (ITS-REST `Qualified_query_name.md`: namespace optional, name
/// charset `[a-zA-Z0-9_.-]`); that upstream non-conformance is recorded
/// and catalogue-tested on its own tracker issue, never accommodated in
/// any conformance expectation.
pub(crate) const STORED_QUERY_NAME: &str = "org.openehr.cnf::ward_dashboard";

/// The per-patient blood-pressure trend (the corpus contract's committed
/// series), EHR-scoped via the `$ehr_id` binding — the ad-hoc read.
pub(crate) const ADHOC_AQL: &str = "SELECT c/uid/value, o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude \
     FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2] \
     WHERE e/ehr_id/value = $ehr_id LIMIT 10";

/// The registered dashboard query: the same trend WITHOUT a `$ehr_id`
/// parameter — the stored-query GET scopes by the wire `ehr_id` query
/// parameter (ITS-REST `query_execute_stored_query`: `ehr_id` is the EHR
/// scope; AQL `$name` bindings ride `query_parameters`, which the
/// continuous dashboard pattern does not need).
pub(crate) const STORED_QUERY_AQL: &str = "SELECT c/uid/value, o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude \
     FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2] \
     LIMIT 10";

/// The advanced-AQL analytics read: the same per-patient series ORDERED and
/// truncated server-side (QUERY AQL §ORDER BY, §LIMIT — the advanced query
/// class the functional battery pins in
/// `I_QUERY_SERVICE.execute_ad_hoc_query-order_by_limit`).
pub(crate) const ANALYTICS_AQL: &str = "SELECT c/uid/value AS uid \
     FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2] \
     WHERE e/ehr_id/value = $ehr_id \
     ORDER BY o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude DESC LIMIT 3";

/// The terminology-backed AQL read (QUERY AQL §TERMINOLOGY): the value-set
/// expansion form the functional battery pins in
/// `I_QUERY_SERVICE.execute_ad_hoc_query-terminology_expand_matches`.
pub(crate) const TERMINOLOGY_AQL: &str = "SELECT c/uid/value AS uid \
     FROM EHR e CONTAINS COMPOSITION c \
     WHERE c/category/defining_code/code_string matches TERMINOLOGY('expand', 'openehr', 'composition_category') \
     LIMIT 10";

/// The cross-EHR ward worklist (the population query class).
pub(crate) const WARD_AQL: &str = "SELECT e/ehr_id/value, c/uid/value \
     FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o [openEHR-EHR-OBSERVATION.blood_pressure.v2] \
     WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 130 LIMIT 50";

/// One standing ward patient: the seeded per-patient state journey stages
/// address (uids captured from the seeding responses).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardPatient {
    /// Index into [`SeededCorpus::ehr_ids`].
    pub ehr_index: usize,
    /// The GP-data-set chart document (`OBJECT_VERSION_ID` at seed).
    pub gp_ovid: String,
    /// The medicines list (`OBJECT_VERSION_ID` at seed).
    pub medlist_ovid: String,
    /// The episode directory version (`OBJECT_VERSION_ID` at seed).
    pub directory_ovid: String,
    /// One committed CONTRIBUTION uid (the audit-review target).
    pub contribution_uid: String,
}

/// The seeded corpus index: what the measurement operations address
/// (in-memory for the run; the workflow always seeds fresh).
#[derive(Debug, Serialize, Deserialize)]
pub struct SeededCorpus {
    /// The corpus key this index realizes (e.g. `cnf.scale.10k`).
    pub corpus: String,
    /// Every seeded EHR id.
    pub ehr_ids: Vec<String>,
    /// Seeded compositions as `(ehr index, version_uid)`.
    pub compositions: Vec<(usize, String)>,
    /// The standing ward (`seed_ward` fills it after the scale seed).
    #[serde(default)]
    pub ward: Vec<WardPatient>,
}

/// Whether a provisioning WRITE landed in the created family. Corpus
/// seeding is PROVISIONING, not the conformance instrument: with
/// `Prefer: return=minimal` some SUTs answer 201 Created and others 204
/// No Content with the identifying headers (upstream `EHRbase`'s minimal
/// create). The functional catalogue pins exact status codes; the seeder
/// accepts either, then still demands the identifying header it needs.
fn created(status: StatusCode) -> bool {
    status == StatusCode::CREATED || status == StatusCode::NO_CONTENT
}

/// The volumetric shape of one `cnf.scale.*` corpus key per the
/// `scale_ladder` contract (EHR count; ~100 composition versions each).
///
/// # Errors
/// An unknown scale key.
pub fn scale_shape(corpus_key: &str) -> Result<(usize, usize), String> {
    match corpus_key {
        "cnf.scale.10k" => Ok((10_000, 100)),
        "cnf.scale.100k" => Ok((100_000, 100)),
        "cnf.scale.1m" => Ok((1_000_000, 100)),
        "cnf.scale.10m" => Ok((10_000_000, 100)),
        other => Err(format!("unknown scale corpus key {other:?}")),
    }
}

/// The standing ward size: the journey stripes address a fixed 10k-bed
/// ward regardless of corpus scale (chart reads still address the FULL
/// corpus, so per-EHR volume keeps dominating query cost).
#[must_use]
pub fn ward_size(ehr_count: usize) -> usize {
    ehr_count.min(10_000)
}

/// Seeds the `scale_ladder` corpus through the public API.
///
/// Uploads the
/// blood-pressure OPT (409 on re-run is fine), creates `ehrs` EHRs, commits
/// `versions_per_ehr` [`crate::exec::recipes::bp_series`] compositions
/// into each. Deterministic content; parallel across `workers` threads.
///
/// # Errors
/// A message on any wire outcome outside the bindings' created/exists
/// outcomes, or a transport fault.
#[expect(
    clippy::too_many_lines,
    reason = "one seeding procedure, linear phases"
)]
pub fn seed_scale_ladder(
    client: &PerfClient,
    corpus_key: &str,
    opt_xml: &str,
    ehrs: usize,
    versions_per_ehr: usize,
    workers: usize,
    progress: &(dyn Fn(String) + Sync),
) -> Result<SeededCorpus, String> {
    // Template first — the compositions' constraint carrier.
    let upload = client.request(
        reqwest::Method::POST,
        "/definition/template/adl1.4",
        Some(("application/xml", opt_xml.as_bytes().to_vec())),
        false,
        None,
    )?;
    if upload.status != StatusCode::CREATED && upload.status != StatusCode::CONFLICT {
        return Err(format!(
            "OPT upload returned {} (expected 201/409)",
            upload.status.as_u16()
        ));
    }

    let workers = workers.max(1);

    // Phase 1: EHRs. The FIRST create runs serially: SUTs that lazily
    // create per-principal bookkeeping on the first authenticated write
    // (upstream EHRbase races its internal user-row creation across
    // parallel first contacts — "User already created concurrently",
    // HTTP 500) settle that state once before the fan-out. Identical
    // treatment for every SUT (fairness); a no-op where no such lazy
    // state exists.
    let ehr_slots: Vec<Mutex<Option<String>>> = (0..ehrs).map(|_| Mutex::new(None)).collect();
    let first = client
        .request(reqwest::Method::POST, "/ehr", None, true, None)
        .and_then(|reply| {
            if reply.status != StatusCode::CREATED {
                return Err(format!("create_ehr returned {}", reply.status.as_u16()));
            }
            reply
                .location
                .as_deref()
                .and_then(location_last_segment)
                .ok_or_else(|| "create_ehr: no Location ehr_id".to_owned())
        })
        .map_err(|e| format!("seeding EHRs failed: {e}"))?;
    if let Some(slot) = ehr_slots.first()
        && let Ok(mut guard) = slot.lock()
    {
        *guard = Some(first);
    }
    let next_ehr = AtomicUsize::new(1);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let done = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next_ehr.fetch_add(1, Ordering::Relaxed);
                    if i >= ehrs {
                        break;
                    }
                    let outcome = client
                        .request(reqwest::Method::POST, "/ehr", None, true, None)
                        .and_then(|reply| {
                            if reply.status != StatusCode::CREATED {
                                return Err(format!(
                                    "create_ehr returned {}",
                                    reply.status.as_u16()
                                ));
                            }
                            reply
                                .location
                                .as_deref()
                                .and_then(location_last_segment)
                                .ok_or_else(|| "create_ehr: no Location ehr_id".to_owned())
                        });
                    match outcome {
                        Ok(id) => {
                            if let Some(Ok(mut slot)) = ehr_slots.get(i).map(Mutex::lock) {
                                *slot = Some(id);
                            }
                        }
                        Err(e) => {
                            if let Ok(mut f) = failures.lock() {
                                f.push(e);
                            }
                            break;
                        }
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(1000) {
                        progress(format!("seeded {n}/{ehrs} EHRs"));
                    }
                }
            });
        }
    });
    if let Ok(f) = failures.lock()
        && let Some(first) = f.first()
    {
        return Err(format!("seeding EHRs failed: {first}"));
    }
    let mut ehr_ids = Vec::with_capacity(ehrs);
    for slot in &ehr_slots {
        let id = slot
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .ok_or_else(|| "seeding EHRs left a gap".to_owned())?;
        ehr_ids.push(id);
    }

    // Phase 2: compositions — bp_series(j % 10) into EHR i, uid captured
    // from the ETag exactly as the create_composition binding does.
    let total = ehrs
        .checked_mul(versions_per_ehr)
        .ok_or_else(|| "corpus size overflows".to_owned())?;
    let bodies: Vec<Vec<u8>> = (0..10)
        .map(|k| {
            crate::exec::recipes::bp_series(k)
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::to_vec(&v).map_err(|e| e.to_string()))
        })
        .collect::<Result<_, _>>()?;
    let next_commit = AtomicUsize::new(0);
    let committed: Mutex<Vec<(usize, String)>> = Mutex::new(Vec::with_capacity(total));
    let done = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut local: Vec<(usize, String)> = Vec::new();
                loop {
                    let t = next_commit.fetch_add(1, Ordering::Relaxed);
                    if t >= total {
                        break;
                    }
                    #[expect(
                        clippy::integer_division,
                        reason = "which EHR the t-th commit belongs to: exact integer bucketing"
                    )]
                    let ehr_index = t / versions_per_ehr;
                    let series = t % 10;
                    let Some(body) = bodies.get(series) else {
                        break;
                    };
                    let Some(ehr_id) = ehr_ids.get(ehr_index) else {
                        break;
                    };
                    let outcome = client
                        .request(
                            reqwest::Method::POST,
                            &format!("/ehr/{ehr_id}/composition"),
                            Some(("application/json", body.clone())),
                            true,
                            None,
                        )
                        .and_then(|reply| {
                            if !created(reply.status) {
                                return Err(format!(
                                    "create_composition returned {}",
                                    reply.status.as_u16()
                                ));
                            }
                            reply
                                .etag
                                .as_deref()
                                .map(strip_weak_quotes)
                                .ok_or_else(|| "create_composition: no ETag".to_owned())
                        });
                    match outcome {
                        Ok(uid) => local.push((ehr_index, uid)),
                        Err(e) => {
                            if let Ok(mut f) = failures.lock() {
                                f.push(e);
                            }
                            break;
                        }
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(10_000) {
                        progress(format!("committed {n}/{total} compositions"));
                    }
                }
                if let Ok(mut all) = committed.lock() {
                    all.append(&mut local);
                }
            });
        }
    });
    if let Ok(f) = failures.lock()
        && let Some(first) = f.first()
    {
        return Err(format!("seeding compositions failed: {first}"));
    }
    let mut compositions = committed
        .into_inner()
        .map_err(|error| format!("seeding lock poisoned: {error}"))?;
    if compositions.len() != total {
        return Err(format!(
            "seeded {}/{total} compositions only",
            compositions.len()
        ));
    }
    compositions.sort();
    Ok(SeededCorpus {
        corpus: corpus_key.to_owned(),
        ehr_ids,
        compositions,
        ward: Vec::new(),
    })
}

/// Seeds the standing ward on top of a scale corpus.
///
/// Uploads every pack OPT
/// (409 on re-run is fine), registers the dashboard stored query, then per
/// ward patient commits the GP chart document, the medicines list, the
/// episode directory, and one CONTRIBUTION — capturing every uid the
/// journey stages address. Idempotent: a corpus whose `ward` already
/// covers the target size is left untouched.
///
/// # Errors
/// A message on any unexpected wire outcome or a transport fault.
#[expect(
    clippy::too_many_lines,
    reason = "one seeding procedure, linear phases"
)]
pub fn seed_ward(
    client: &PerfClient,
    corpus: &mut SeededCorpus,
    journey_pack: &JourneyPack,
    workers: usize,
    progress: &(dyn Fn(String) + Sync),
) -> Result<(), String> {
    let target = ward_size(corpus.ehr_ids.len());
    if corpus.ward.len() >= target {
        progress(format!("standing ward already seeded ({target} patients)"));
        return Ok(());
    }

    // Pack OPTs — every journey template's constraint carrier — plus the
    // Simplified-FLAT payload's own OPT when the catalogue commits one.
    let mut opts: Vec<(&str, &str)> = journey_pack
        .templates
        .iter()
        .map(|t| (t.key.as_str(), t.opt_xml.as_str()))
        .collect();
    if let Some(flat) = &journey_pack.aux.flat {
        opts.push((pack::FLAT_OPT_KEY, flat.opt_xml.as_str()));
    }
    // The TDD stage's own OPT: a TDD is an instance of the template-derived
    // TDS (AM OPT2 master02-overview §Purpose of the OPT), so it cannot be
    // interpreted at all without its operational template.
    if let Some(tdd) = &journey_pack.aux.tdd {
        opts.push((pack::TDD_OPT_KEY, tdd.opt_xml.as_str()));
    }
    for (key, opt_xml) in opts {
        let upload = client.request(
            reqwest::Method::POST,
            "/definition/template/adl1.4",
            Some(("application/xml", opt_xml.as_bytes().to_vec())),
            false,
            None,
        )?;
        if upload.status != StatusCode::CREATED && upload.status != StatusCode::CONFLICT {
            return Err(format!(
                "OPT upload for {key} returned {} (expected 201/409)",
                upload.status.as_u16()
            ));
        }
    }
    // PACK PREFLIGHT: every pack example must commit clean once (a
    // scratch EHR) before any window opens — an RM-invalid generated
    // payload is an instrument-ground defect and must fail seeding
    // loudly, never surface as silent error arrivals inside a measured
    // window.
    let scratch = client.request(reqwest::Method::POST, "/ehr", None, true, None)?;
    let scratch_ehr = if scratch.status == StatusCode::CREATED {
        scratch
            .location
            .as_deref()
            .and_then(location_last_segment)
            .ok_or_else(|| "preflight EHR: no Location ehr_id".to_owned())?
    } else {
        return Err(format!(
            "preflight EHR create returned {}",
            scratch.status.as_u16()
        ));
    };
    for (index, template) in journey_pack.templates.iter().enumerate() {
        let body = pack::composition_body(template, 0, u64::try_from(index).unwrap_or(0))?;
        let reply = client.request(
            reqwest::Method::POST,
            &format!("/ehr/{scratch_ehr}/composition"),
            Some(("application/json", body)),
            true,
            None,
        )?;
        if !created(reply.status) {
            return Err(format!(
                "pack preflight: template {} example returned {} — the committed payload                  ground is invalid for this SUT; fix the pack (or the SUT's validation)                  before measuring",
                template.key,
                reply.status.as_u16()
            ));
        }
    }
    // The Simplified-FLAT payload rides the same preflight: its FLAT paths
    // are template-derived, so a mismatch against the OPT is an
    // instrument-ground defect exactly like an RM-invalid example.
    if let Some(flat) = &journey_pack.aux.flat {
        let reply = client.request_negotiated(
            reqwest::Method::POST,
            &format!("/ehr/{scratch_ehr}/composition"),
            Some(("application/openehr.wt.flat+json", pack::flat_body(flat)?)),
            true,
            None,
            None,
            &[("openehr-template-id", flat.template_id.clone())],
        )?;
        if !created(reply.status) {
            return Err(format!(
                "pack preflight: the Simplified-FLAT payload returned {} — the committed payload \
                 ground is invalid for this SUT; fix the pack (or the SUT's validation) before \
                 measuring",
                reply.status.as_u16()
            ));
        }
    }
    // The TDD payload rides the same preflight: its body is matched to the
    // template node tree on the way in, so a mismatch against the OPT is an
    // instrument-ground defect exactly like an RM-invalid example.
    if let Some(tdd) = &journey_pack.aux.tdd {
        let reply = client.request(
            reqwest::Method::POST,
            &format!("/message/tdd/{scratch_ehr}"),
            Some(("application/xml", tdd.document.as_bytes().to_vec())),
            false,
            None,
        )?;
        if !created(reply.status) {
            return Err(format!(
                "pack preflight: the TDD payload returned {} — the committed payload ground is \
                 invalid for this SUT; fix the pack (or the SUT's validation) before measuring",
                reply.status.as_u16()
            ));
        }
    }
    progress(format!(
        "pack preflight: {} template examples committed clean",
        journey_pack.templates.len()
    ));

    // The dashboard stored query (`store_query` → wire 200).
    let stored = client.request(
        reqwest::Method::PUT,
        &format!("/definition/query/{STORED_QUERY_NAME}"),
        Some(("text/plain", STORED_QUERY_AQL.as_bytes().to_vec())),
        false,
        None,
    )?;
    if stored.status != StatusCode::OK
        && stored.status != StatusCode::CREATED
        && stored.status != StatusCode::CONFLICT
    {
        return Err(format!(
            "store_query returned {} (expected 200)",
            stored.status.as_u16()
        ));
    }

    let gp_index = journey_pack
        .index_of("cnf.ckm.gp_data_set")
        .ok_or_else(|| "pack has no cnf.ckm.gp_data_set".to_owned())?;
    let medlist_index = journey_pack
        .index_of("cnf.ckm.medicines_list")
        .ok_or_else(|| "pack has no cnf.ckm.medicines_list".to_owned())?;
    let lab_index = journey_pack
        .index_of("cnf.ckm.lab_result")
        .ok_or_else(|| "pack has no cnf.ckm.lab_result".to_owned())?;

    let start = corpus.ward.len();
    let slots: Vec<Mutex<Option<WardPatient>>> =
        (start..target).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let done = AtomicUsize::new(0);
    let workers = workers.max(1);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let offset = next.fetch_add(1, Ordering::Relaxed);
                    let patient = start + offset;
                    if patient >= target {
                        break;
                    }
                    let outcome = seed_one_patient(
                        client,
                        corpus,
                        journey_pack,
                        patient,
                        gp_index,
                        medlist_index,
                        lab_index,
                    );
                    match outcome {
                        Ok(entry) => {
                            if let Some(Ok(mut slot)) = slots.get(offset).map(Mutex::lock) {
                                *slot = Some(entry);
                            }
                        }
                        Err(e) => {
                            if let Ok(mut f) = failures.lock() {
                                f.push(e);
                            }
                            break;
                        }
                    }
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(1000) {
                        progress(format!("ward-seeded {n}/{} patients", target - start));
                    }
                }
            });
        }
    });
    if let Ok(f) = failures.lock()
        && let Some(first) = f.first()
    {
        return Err(format!("ward seeding failed: {first}"));
    }
    for slot in &slots {
        let entry = slot
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .ok_or_else(|| "ward seeding left a gap".to_owned())?;
        corpus.ward.push(entry);
    }
    Ok(())
}

fn seed_one_patient(
    client: &PerfClient,
    corpus: &SeededCorpus,
    journey_pack: &JourneyPack,
    patient: usize,
    gp_index: usize,
    medlist_index: usize,
    lab_index: usize,
) -> Result<WardPatient, String> {
    let ehr_id = corpus
        .ehr_ids
        .get(patient)
        .ok_or_else(|| "ward patient outside the corpus".to_owned())?;
    let arrival = u64::try_from(patient).unwrap_or(u64::MAX);
    let commit = |index: usize| -> Result<String, String> {
        let template = journey_pack
            .get(index)
            .ok_or_else(|| "pack index out of range".to_owned())?;
        let body = pack::composition_body(template, 0, arrival)?;
        let reply = client.request(
            reqwest::Method::POST,
            &format!("/ehr/{ehr_id}/composition"),
            Some(("application/json", body)),
            true,
            None,
        )?;
        if !created(reply.status) {
            return Err(format!(
                "ward commit ({}) returned {}",
                template.key,
                reply.status.as_u16()
            ));
        }
        reply
            .etag
            .as_deref()
            .map(strip_weak_quotes)
            .ok_or_else(|| "ward commit: no ETag".to_owned())
    };
    let gp_ovid = commit(gp_index)?;
    let medlist_ovid = commit(medlist_index)?;

    let directory = client.request(
        reqwest::Method::POST,
        &format!("/ehr/{ehr_id}/directory"),
        Some(("application/json", pack::folder_body(false))),
        true,
        None,
    )?;
    if !created(directory.status) {
        return Err(format!(
            "ward directory create returned {}",
            directory.status.as_u16()
        ));
    }
    let directory_ovid = directory
        .etag
        .as_deref()
        .map(strip_weak_quotes)
        .ok_or_else(|| "ward directory create: no ETag".to_owned())?;

    let lab = journey_pack
        .get(lab_index)
        .ok_or_else(|| "pack index out of range".to_owned())?;
    let contribution = client.request(
        reqwest::Method::POST,
        &format!("/ehr/{ehr_id}/contribution"),
        Some((
            "application/json",
            pack::contribution_body(lab, 0, arrival)?,
        )),
        true,
        None,
    )?;
    if !created(contribution.status) {
        return Err(format!(
            "ward contribution returned {}",
            contribution.status.as_u16()
        ));
    }
    let contribution_uid = contribution
        .location
        .as_deref()
        .and_then(location_last_segment)
        .or_else(|| contribution.etag.as_deref().map(strip_weak_quotes))
        .ok_or_else(|| "ward contribution: no Location/ETag uid".to_owned())?;

    Ok(WardPatient {
        ehr_index: patient,
        gp_ovid,
        medlist_ovid,
        directory_ovid,
        contribution_uid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_shapes_follow_the_ladder() {
        assert_eq!(scale_shape("cnf.scale.10k").unwrap(), (10_000, 100));
        assert_eq!(scale_shape("cnf.scale.10m").unwrap(), (10_000_000, 100));
        assert!(scale_shape("cnf.scale.5k").is_err());
        assert_eq!(ward_size(500), 500);
        assert_eq!(ward_size(1_000_000), 10_000);
    }

    #[test]
    fn a_pre_ward_index_parses_with_an_empty_ward() {
        let index = r#"{"corpus":"cnf.scale.10k","ehr_ids":["a"],"compositions":[[0,"u::s::1"]]}"#;
        let corpus: SeededCorpus = serde_json::from_str(index).unwrap();
        assert!(corpus.ward.is_empty());
    }
}
