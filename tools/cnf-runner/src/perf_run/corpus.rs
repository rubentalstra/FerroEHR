//! The seeded corpus: the `scale_ladder` volume (N EHRs × ~100 committed
//! blood-pressure versions, `corpus/recipes/scale_ladder.md`) plus the
//! STANDING WARD — the per-patient state the journey stages address
//! mid-flight (an episode directory, the GP-data-set chart document, the
//! medicines list, one committed CONTRIBUTION), seeded strictly through
//! the public API (never a database backdoor) and persisted as a sidecar
//! index so re-runs can skip re-seeding.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::perf_run::client::{PerfClient, location_last_segment, strip_weak_quotes};
use crate::perf_run::pack;
use crate::perf_run::pack::JourneyPack;

/// The stored query the ward dashboard executes continuously (registered
/// once at seeding; `I_DEFINITION_QUERY.store_query` → wire 200).
pub(crate) const STORED_QUERY_NAME: &str = "cnf.ward_dashboard";

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

/// The seeded corpus index: what the measurement operations address.
/// Persisted as a sidecar JSON so a re-run can skip re-seeding.
#[derive(Debug, Serialize, Deserialize)]
pub struct SeededCorpus {
    /// The corpus key this index realizes (e.g. `cnf.scale.10k`).
    pub corpus: String,
    /// Every seeded EHR id.
    pub ehr_ids: Vec<String>,
    /// Seeded compositions as `(ehr index, version_uid)`.
    pub compositions: Vec<(usize, String)>,
    /// The standing ward (empty in a pre-journey sidecar; `seed_ward`
    /// fills it).
    #[serde(default)]
    pub ward: Vec<WardPatient>,
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

/// Seed the `scale_ladder` corpus through the public API: upload the
/// blood-pressure OPT (409 on re-run is fine), create `ehrs` EHRs, commit
/// `versions_per_ehr` [`crate::exec::recipes::bp_series`] compositions
/// into each. Deterministic content; parallel across `workers` threads.
///
/// # Errors
/// A message on any wire outcome outside the bindings' created/exists
/// outcomes, or a transport fault.
#[allow(clippy::too_many_lines)] // one seeding procedure, linear phases
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
    match upload.status {
        201 | 409 => {}
        other => return Err(format!("OPT upload returned {other} (expected 201/409)")),
    }

    let workers = workers.max(1);

    // Phase 1: EHRs.
    let ehr_slots: Vec<Mutex<Option<String>>> = (0..ehrs).map(|_| Mutex::new(None)).collect();
    let next_ehr = AtomicUsize::new(0);
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
                            if reply.status != 201 {
                                return Err(format!("create_ehr returned {}", reply.status));
                            }
                            reply
                                .location
                                .as_deref()
                                .and_then(location_last_segment)
                                .ok_or_else(|| "create_ehr: no Location ehr_id".to_owned())
                        });
                    match outcome {
                        Ok(id) => {
                            if let Ok(mut slot) = ehr_slots[i].lock() {
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
                            if reply.status != 201 {
                                return Err(format!(
                                    "create_composition returned {}",
                                    reply.status
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
        .map_err(|_| "seeding lock poisoned".to_owned())?;
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

/// Seed the standing ward on top of a scale corpus: upload every pack OPT
/// (409 on re-run is fine), register the dashboard stored query, then per
/// ward patient commit the GP chart document, the medicines list, the
/// episode directory, and one CONTRIBUTION — capturing every uid the
/// journey stages address. Idempotent per sidecar: a corpus whose `ward`
/// already covers the target size is left untouched.
///
/// # Errors
/// A message on any unexpected wire outcome or a transport fault.
#[allow(clippy::too_many_lines)] // one seeding procedure, linear phases
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

    // Pack OPTs — every journey template's constraint carrier.
    for template in &journey_pack.templates {
        let upload = client.request(
            reqwest::Method::POST,
            "/definition/template/adl1.4",
            Some(("application/xml", template.opt_xml.as_bytes().to_vec())),
            false,
            None,
        )?;
        match upload.status {
            201 | 409 => {}
            other => {
                return Err(format!(
                    "OPT upload for {} returned {other} (expected 201/409)",
                    template.key
                ));
            }
        }
    }
    // The dashboard stored query (`store_query` → wire 200).
    let stored = client.request(
        reqwest::Method::PUT,
        &format!("/definition/query/{STORED_QUERY_NAME}"),
        Some(("text/plain", STORED_QUERY_AQL.as_bytes().to_vec())),
        false,
        None,
    )?;
    match stored.status {
        200 | 201 | 409 => {}
        other => return Err(format!("store_query returned {other} (expected 200)")),
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
                            if let Ok(mut slot) = slots[offset].lock() {
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

#[allow(clippy::too_many_arguments)] // one patient's linear seeding chain
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
        if reply.status != 201 {
            return Err(format!(
                "ward commit ({}) returned {}",
                template.key, reply.status
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
    if directory.status != 201 {
        return Err(format!(
            "ward directory create returned {}",
            directory.status
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
    if contribution.status != 201 {
        return Err(format!(
            "ward contribution returned {}",
            contribution.status
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
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
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
    fn a_pre_journey_sidecar_still_parses_without_a_ward() {
        let sidecar =
            r#"{"corpus":"cnf.scale.10k","ehr_ids":["a"],"compositions":[[0,"u::s::1"]]}"#;
        let corpus: SeededCorpus = serde_json::from_str(sidecar).unwrap();
        assert!(corpus.ward.is_empty());
    }
}
