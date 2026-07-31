//! The `EHR_ACCESS` policy gate — the **spec-grounded** access-decision layer.
//!
//! # Why this leads
//!
//! `EHR_ACCESS` is a mandatory, versioned RM object and the openEHR
//! access-decision authority: "All access decisions to data in the EHR must be
//! made in accordance with the policies and rules in this object" (RM
//! `org.openehr.rm.ehr.ehr_access.adoc` §`EHR_ACCESS` Class). This gate is
//! therefore the **foundational, always-on** access-control layer, built
//! directly on the spec. The crate's RBAC + ABAC + SMART gates
//! ([`crate::extensions::access::authz`], [`crate::extensions::access::pep`]) are
//! our **own enterprise extensions** (SMART aside, which is spec-grounded) — no
//! openEHR spec governs RBAC/ABAC, and the SM places authorisation out of band
//! (SM `openehr_platform/master02-overview.adoc` §General Assumptions). They
//! compose **on top of** this spec base as additive restrictions
//! (AND-composition: a request must clear the `EHR_ACCESS` gate *and* any
//! RBAC/ABAC/SMART policy), never the other way round. Accordingly the gate runs **first** in the pre-dispatch
//! chain (the dispatch mount, `crate::api`), and it is never config-gated: every EHR
//! carries an `EHR_ACCESS`, and the default (`open`, no settings) keeps every
//! existing flow working (BASE `architecture_overview/master07-security.adoc`
//! §Access Control — "sensible defaults").
//!
//! # The three evaluation points
//!
//! 1. **Per-EHR gate** — every authenticated request on an `/ehr/{ehr_id}`-
//!    scoped route (`master07` access list; ITS-REST `401`/`403` discipline,
//!    `ITS-REST/.../Requests_and_responses.md` §Authentication and authorization).
//! 2. **Composition privacy ceiling** — Composition read routes, from the
//!    settings + the target uid alone (`master07` privacy levels).
//! 3. **Gate-keeper preflight** — CONTRIBUTION commits carrying an `EHR_ACCESS`
//!    version (`master07` gate-keeper; RM ehr `master04-ehr_package.adoc`
//!    §EHR Access — settings changes are CONTRIBUTION-wrapped + audited).
//!
//! The concrete scheme evaluated here (`ferroehr.access_control.v1`) is our own
//! design — no openEHR spec governs it;
//! the parsed [`EhrAccessSettings`] live in `ferroehr-sm`.
//!
//! # v1 scope boundary
//!
//! AQL result filtering by privacy level is **not** evaluated in v1 (query
//! execution carries no principal context yet); the per-EHR gate still applies
//! to the REST query surface where an `ehr_id` is bound.
//!
//! The PEP returns `Result<(), Response>` — the deny path is a ready `403`.

use axum::response::{IntoResponse, Response};
use ferroehr::service::ehr::access_types::{AccessLevel, principal_matches};
use ferroehr::service::ehr::access_types::{DefaultAccess, EhrAccessSettings};
use openehr_its::rest::runtime::ApiError;
use uuid::Uuid;

use crate::api::RequestParts;
use crate::extensions::access::authn::{Principal, current_principal};
use crate::overview::error::RestError;
use crate::state::AppState;

/// The `EHR_ACCESS` policy engine: the three spec-grounded evaluation points,
/// as pure decisions over the parsed scheme settings + the authenticated
/// caller. `Ok(())` permits; `Err(reason)` is a denial with its reason. Absent
/// settings (`None`) always permit — the default-open disposition that keeps
/// every existing EHR working (`master07` "sensible defaults").
#[derive(Debug)]
pub struct EhrAccessGate;

impl EhrAccessGate {
    /// The per-EHR gate: `open` (or no settings) permits; `restricted` requires
    /// the caller to match the access list (`master07` §Access Control).
    ///
    /// # Errors
    /// `Err(reason)` when the EHR is restricted and the caller matches no
    /// access-list entry — the reason is the human-readable denial detail.
    pub fn ehr_gate(
        settings: Option<&EhrAccessSettings>,
        subject: Option<&str>,
        roles: &[String],
    ) -> Result<(), String> {
        let Some(settings) = settings else {
            return Ok(());
        };
        match settings.default_access {
            DefaultAccess::Open => Ok(()),
            DefaultAccess::Restricted => {
                if settings.match_principal(subject, roles).is_some() {
                    Ok(())
                } else {
                    Err("EHR access is restricted; the caller is not on the access list".to_owned())
                }
            }
        }
    }

    /// The Composition privacy ceiling: the effective level of the target
    /// versioned Composition must be strictly below the caller's ceiling
    /// (`full` → unbounded; `restricted_below` → `max_level`; no access-list
    /// entry → `default_level + 1`, i.e. the default level stays readable unless
    /// raised) — `master07` §Access Control privacy levels.
    ///
    /// # Errors
    /// `Err(reason)` when the target Composition's privacy level is at or above
    /// the caller's ceiling.
    pub fn privacy_gate(
        settings: Option<&EhrAccessSettings>,
        subject: Option<&str>,
        roles: &[String],
        target_vo_id: &str,
    ) -> Result<(), String> {
        let Some(settings) = settings else {
            return Ok(());
        };
        let level = settings.privacy.level_for(target_vo_id);
        let ceiling = match settings.match_principal(subject, roles) {
            Some(entry) => match entry.access {
                AccessLevel::Full => i64::MAX,
                // `max_level` is present for `restricted_below`; a malformed
                // absent one fails closed at level 0.
                AccessLevel::RestrictedBelow => entry.max_level.unwrap_or(0),
            },
            None => settings.privacy.default_level.saturating_add(1),
        };
        if level >= ceiling {
            Err(format!(
                "composition privacy level {level} is at or above the caller's ceiling {ceiling}"
            ))
        } else {
            Ok(())
        }
    }

    /// The gate-keeper preflight: once a gate-keeper is set, only that principal
    /// may commit a new `EHR_ACCESS` version (`master07` §Access Control; the
    /// settings stay CONTRIBUTION-wrapped + audited — RM ehr
    /// `master04-ehr_package.adoc` §EHR Access).
    ///
    /// # Errors
    /// `Err(reason)` when a gate-keeper is set and the caller is not it.
    pub fn gate_keeper_gate(
        settings: Option<&EhrAccessSettings>,
        subject: Option<&str>,
        roles: &[String],
    ) -> Result<(), String> {
        let Some(settings) = settings else {
            return Ok(());
        };
        let Some(gate_keeper) = &settings.gate_keeper else {
            return Ok(());
        };
        if principal_matches(gate_keeper, subject, roles) {
            Ok(())
        } else {
            Err(format!(
                "only the gate-keeper ({gate_keeper}) may change the EHR_ACCESS settings"
            ))
        }
    }
}

/// Whether `op` reads a Composition (subject to the privacy ceiling). Mirrors
/// the composition-read set the ABAC post-check uses.
fn is_composition_read(op: &str) -> bool {
    op == "composition_get" || op.starts_with("versioned_composition_")
}

/// The target versioned Composition uid for a read op, from the path params
/// (the versioned-object uid, a version uid, or the uid-based id — all
/// normalised to the versioned-object head by `Privacy::level_for`).
fn composition_target(parts: &RequestParts) -> Option<&str> {
    ["versioned_object_uid", "version_uid", "uid_based_id"]
        .into_iter()
        .find_map(|k| parts.path.get(k).map(String::as_str))
}

/// Whether a CONTRIBUTION body carries a version whose data targets `EHR_ACCESS`
/// (the only `EHR_ACCESS` write path — there is no dedicated ITS-REST endpoint).
/// Parsed minimally: an `EHR_ACCESS` create/modify carries the full object, so a
/// `versions[].data._type == "EHR_ACCESS"` is sufficient detection.
fn contribution_targets_ehr_access(parts: &RequestParts) -> bool {
    let Ok(value) = crate::overview::negotiate::json_value(&parts.headers, &parts.body) else {
        // A malformed body: the dispatch will return the 400; nothing to gate.
        return false;
    };
    value
        .get("versions")
        .and_then(|v| v.as_array())
        .is_some_and(|versions| {
            versions.iter().any(|v| {
                v.get("data")
                    .and_then(|d| d.get("_type"))
                    .and_then(|t| t.as_str())
                    == Some("EHR_ACCESS")
            })
        })
}

/// Enforce the `EHR_ACCESS` policy for an EHR-scoped request, after
/// authentication and before dispatch. `Err(response)` short-circuits with a
/// ready `403` (deny) or `500` (settings unavailable — fail-closed, consistent
/// with the ABAC PEP); `Ok(())` lets the request proceed. A route without an
/// `ehr_id` path param is not EHR-scoped and always proceeds.
pub(crate) async fn enforce(
    state: &AppState,
    op: &'static str,
    parts: &RequestParts,
) -> Result<(), Response> {
    let Some(ehr_id_raw) = parts.path.get("ehr_id") else {
        return Ok(());
    };
    let Ok(ehr_id) = Uuid::parse_str(ehr_id_raw) else {
        // A malformed ehr_id is not ours to reject — the dispatch returns 400/404.
        return Ok(());
    };

    let principal = current_principal();
    let subject = principal.as_ref().map(|p| p.subject.as_str());
    let roles = principal.as_ref().map_or(&[][..], |p| p.roles.as_slice());

    let settings = match state
        .backend()
        .current_ehr_access_settings(ferroehr::ids::EhrId(ehr_id))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Err(server_error(
                principal.as_ref(),
                &format!("EHR_ACCESS settings unavailable: {e}"),
            ));
        }
    };
    let settings = settings.as_ref();

    // 1. The per-EHR gate (every EHR-scoped route).
    EhrAccessGate::ehr_gate(settings, subject, roles)
        .map_err(|reason| forbidden(principal.as_ref(), &reason))?;

    // 2. The Composition privacy ceiling (Composition read routes).
    if is_composition_read(op)
        && let Some(target) = composition_target(parts)
    {
        EhrAccessGate::privacy_gate(settings, subject, roles, target)
            .map_err(|reason| forbidden(principal.as_ref(), &reason))?;
    }

    // 3. The gate-keeper preflight (CONTRIBUTION commits touching EHR_ACCESS).
    if op == "contribution_create" && contribution_targets_ehr_access(parts) {
        EhrAccessGate::gate_keeper_gate(settings, subject, roles)
            .map_err(|reason| forbidden(principal.as_ref(), &reason))?;
    }

    Ok(())
}

/// A `403` carrying the principal (so the ATNA audit layer records the deny —
/// ITS-REST `401`/`403` discipline).
fn forbidden(principal: Option<&Principal>, detail: &str) -> Response {
    let mut resp =
        RestError(ApiError::Forbidden(format!("access denied: {detail}"))).into_response();
    if let Some(principal) = principal {
        resp.extensions_mut().insert(principal.clone());
    }
    resp
}

/// A `500` (fail-closed) when the settings cannot be read — an access decision
/// must never proceed on unknown policy.
fn server_error(principal: Option<&Principal>, detail: &str) -> Response {
    let mut resp = RestError(ApiError::Internal(format!(
        "authorization unavailable: {detail}"
    )))
    .into_response();
    if let Some(principal) = principal {
        resp.extensions_mut().insert(principal.clone());
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings(scheme: &serde_json::Value) -> EhrAccessSettings {
        let access = json!({ "_type": "EHR_ACCESS", "settings": scheme });
        EhrAccessSettings::from_ehr_access(&access).expect("our scheme")
    }

    #[test]
    fn absent_settings_allow_everyone() {
        assert!(EhrAccessGate::ehr_gate(None, None, &[]).is_ok());
        assert!(EhrAccessGate::privacy_gate(None, None, &[], "vo").is_ok());
        assert!(EhrAccessGate::gate_keeper_gate(None, None, &[]).is_ok());
    }

    #[test]
    fn open_ehr_admits_anonymous_and_named() {
        let s =
            settings(&json!({ "_type": "FERROEHR_ACCESS_CONTROL_V1", "default_access": "open" }));
        assert!(EhrAccessGate::ehr_gate(Some(&s), None, &[]).is_ok());
        assert!(EhrAccessGate::ehr_gate(Some(&s), Some("bob"), &[]).is_ok());
    }

    #[test]
    fn restricted_ehr_gates_by_access_list() {
        let s = settings(&json!({
            "_type": "FERROEHR_ACCESS_CONTROL_V1",
            "default_access": "restricted",
            "access_list": [
                { "principal": "user:bob", "access": "full" },
                { "principal": "role:nurse", "access": "restricted_below", "max_level": 2 }
            ]
        }));
        // Listed user permitted; listed role permitted; nobody denied.
        assert!(EhrAccessGate::ehr_gate(Some(&s), Some("bob"), &[]).is_ok());
        assert!(EhrAccessGate::ehr_gate(Some(&s), Some("carol"), &["NURSE".to_owned()]).is_ok());
        assert!(EhrAccessGate::ehr_gate(Some(&s), Some("carol"), &[]).is_err());
        assert!(EhrAccessGate::ehr_gate(Some(&s), None, &[]).is_err());
    }

    #[test]
    fn privacy_ceiling_blocks_above_and_admits_below() {
        let s = settings(&json!({
            "_type": "FERROEHR_ACCESS_CONTROL_V1",
            "default_access": "open",
            "access_list": [ { "principal": "role:nurse", "access": "restricted_below", "max_level": 2 } ],
            "privacy": {
                "default_level": 0,
                "composition_overrides": [ { "uid": "high-vo", "level": 3 } ]
            }
        }));
        let nurse = ["NURSE".to_owned()];
        // Nurse: ceiling 2 → level 0 (default) ok, level 3 (override) blocked.
        assert!(EhrAccessGate::privacy_gate(Some(&s), Some("x"), &nurse, "any-vo").is_ok());
        assert!(EhrAccessGate::privacy_gate(Some(&s), Some("x"), &nurse, "high-vo").is_err());
        // No access-list entry under open: ceiling default_level+1 = 1 → level 0
        // ok, the level-3 override blocked.
        assert!(EhrAccessGate::privacy_gate(Some(&s), Some("y"), &[], "any-vo").is_ok());
        assert!(EhrAccessGate::privacy_gate(Some(&s), Some("y"), &[], "high-vo").is_err());
    }

    #[test]
    fn full_access_has_no_privacy_ceiling() {
        let s = settings(&json!({
            "_type": "FERROEHR_ACCESS_CONTROL_V1",
            "default_access": "open",
            "access_list": [ { "principal": "user:chief", "access": "full" } ],
            "privacy": { "default_level": 9, "composition_overrides": [ { "uid": "top", "level": 100 } ] }
        }));
        assert!(EhrAccessGate::privacy_gate(Some(&s), Some("chief"), &[], "top").is_ok());
    }

    #[test]
    fn gate_keeper_admits_only_the_keeper() {
        let s = settings(&json!({
            "_type": "FERROEHR_ACCESS_CONTROL_V1",
            "gate_keeper": "user:alice",
            "default_access": "open"
        }));
        assert!(EhrAccessGate::gate_keeper_gate(Some(&s), Some("alice"), &[]).is_ok());
        assert!(EhrAccessGate::gate_keeper_gate(Some(&s), Some("bob"), &[]).is_err());
        assert!(EhrAccessGate::gate_keeper_gate(Some(&s), None, &[]).is_err());
    }
}
