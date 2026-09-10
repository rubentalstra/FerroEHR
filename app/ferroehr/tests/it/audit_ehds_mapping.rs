// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! The EHDS Annex II 3.2 logging elements, asserted against the access-event
//! model and against BOTH renderings the audit trail leaves the server in.
//!
//! Annex II 3.2 of Regulation (EU) 2025/327 lists five things the European
//! logging software component must record "on every access event or group of
//! events" (<https://eur-lex.europa.eu/eli/reg/2025/327/oj>). The published
//! mapping from each of them to a field lives on the audit page of the
//! documentation site; this module is what stops that page from becoming a
//! claim nobody checks.
//!
//! Two of the five are recorded and rendered, one is partial, and two are
//! gaps. **The gaps are asserted as gaps**: a test that expects the absence
//! fails the day the field arrives, which is the point — the page and the
//! code then have to move together, and a gap cannot quietly close while the
//! documentation still calls it open.
//!
//! No openEHR spec governs the read-side access log — our own
//! design/extension. openEHR specifies the write-side `AUDIT_DETAILS` only
//! (`docs/specs/openehr/RM/docs/common/` §`AUDIT_DETAILS` Class).

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this module's \
              fixture helpers; a panicking fixture is the intended shape (the \
              Rust Book ch11), as in `multimedia_s3`"
)]

use jiff::Timestamp;

use ferroehr::system_log::event::{
    AccessDomain, AuditEvent, EventActionCode, EventOutcome, EventType, ObjectClass,
};
use ferroehr::system_log::fhir;
use ferroehr::system_log::message::{AuditContext, AuditMessage};

fn ctx() -> AuditContext {
    AuditContext {
        source_id: "ferroehr".to_owned(),
        enterprise_site_id: "site-1".to_owned(),
        server_ip: "10.42.23.77".to_owned(),
        value_if_missing: "UNKNOWN".to_owned(),
    }
}

/// One clinical read by a named person, with everything the model can carry.
fn access_event() -> AuditEvent {
    let mut e = AuditEvent::new(
        EventActionCode::Read,
        ObjectClass::Composition,
        EventOutcome::Success,
    );
    "dr.jansen".clone_into(&mut e.user_id);
    e.client_ip = Some("10.0.0.9".to_owned());
    e.object_id = Some("8fa1::ferroehr::1".to_owned());
    e.event_type = Some(EventType::RestOperation("composition_get"));
    e.token_id = Some("jti-1".to_owned());
    e.purpose = Some("TREAT".to_owned());
    e.legal_basis = Some("gdpr-9-2-h".to_owned());
    e.result_count = Some(1);
    e.timestamp = "2026-07-10T08:30:00Z".parse().expect("a fixed instant");
    e
}

/// The event as its two wire renderings: DICOM PS3.15 XML and the FHIR R4
/// `AuditEvent` the BALP feed carries.
fn renderings(event: &AuditEvent) -> (String, String) {
    let xml = AuditMessage::build(event, &ctx(), Some("patient-42"))
        .to_xml()
        .expect("the DICOM rendering builds");
    let json = serde_json::to_string(
        &fhir::to_fhir(event, &ctx(), Some("patient-42")).expect("the FHIR rendering builds"),
    )
    .expect("the FHIR resource serializes");
    (xml, json)
}

/// (b) "identification of the specific natural person or persons having
/// accessed the personal electronic health data".
#[test]
fn element_b_the_person_who_accessed_is_recorded_and_rendered() {
    let event = access_event();
    assert_eq!(event.user_id, "dr.jansen");
    let (xml, json) = renderings(&event);
    assert!(
        xml.contains("dr.jansen"),
        "the accessing person must reach the DICOM ActiveParticipant: {xml}"
    );
    assert!(
        json.contains("dr.jansen"),
        "the accessing person must reach the FHIR agent: {json}"
    );
}

/// (d) "the time and date of access".
#[test]
fn element_d_the_time_of_access_is_recorded_and_rendered() {
    let event = access_event();
    let stamp: Timestamp = "2026-07-10T08:30:00Z".parse().expect("a fixed instant");
    assert_eq!(event.timestamp, stamp);
    let (xml, json) = renderings(&event);
    assert!(
        xml.contains("2026-07-10T08:30:00"),
        "the event time must reach the DICOM EventDateTime: {xml}"
    );
    assert!(
        json.contains("2026-07-10T08:30:00"),
        "the event time must reach the FHIR recorded element: {json}"
    );
}

/// (c) "the categories of data accessed" — partial, and the test says which
/// half holds.
///
/// The record carries the resource class and the pseudonymisation domain, so
/// "what kind of thing was read, and was it clinical or identifying" is
/// answerable. What it does not carry is the Annex I priority category, which
/// is a different vocabulary from openEHR's resource classes.
#[test]
fn element_c_the_resource_class_and_domain_stand_in_for_the_category() {
    let event = access_event();
    assert_eq!(event.object, ObjectClass::Composition);
    assert_eq!(event.domain, AccessDomain::Ehr);
    assert_eq!(
        AccessDomain::of(ObjectClass::Demographic),
        AccessDomain::Demographic,
        "reading a party is recorded in the demographic domain, which is what \
         separates 'who saw the identity' from 'who read the record'"
    );
    let (xml, json) = renderings(&event);
    // The DICOM ParticipantObject type/role codes carry the class; the FHIR
    // entity does the same.
    assert!(
        xml.contains("ParticipantObjectIdentification"),
        "the object the event touched must be rendered: {xml}"
    );
    assert!(
        json.contains("entity"),
        "the FHIR rendering must carry the entity: {json}"
    );
}

/// (a) "identification of the healthcare provider or other individuals having
/// accessed" — a GAP, asserted as one.
///
/// Read beside (b), this element asks for the ORGANISATION or other entity on
/// whose behalf the access happened, distinct from the natural person (b)
/// names. The model records the authenticated principal and nothing about the
/// organisation, so nothing can render it.
#[test]
fn element_a_the_accessing_organisation_is_not_recorded_yet() {
    let event = access_event();
    let fields = format!("{event:?}");
    assert!(
        !fields.to_lowercase().contains("organisation")
            && !fields.to_lowercase().contains("organization"),
        "an organisation field has appeared on the access event — Annex II 3.2(a) \
         is no longer a gap, so update the mapping table on the audit page and \
         this test together: {fields}"
    );
}

/// (e) "the origin or origins of data" — a GAP, asserted as one.
///
/// The origin of the DATA is not the origin of the request: the record knows
/// the client address it was asked from, and nothing about where the content
/// it served came from. openEHR models that provenance as `FEEDER_AUDIT` on
/// the content itself (RM common `master04` §Feeder Audit), which the access
/// log does not read.
#[test]
fn element_e_the_origin_of_the_data_is_not_recorded_yet() {
    let event = access_event();
    let fields = format!("{event:?}");
    assert!(
        !fields.to_lowercase().contains("origin"),
        "an origin field has appeared on the access event — Annex II 3.2(e) is \
         no longer a gap, so update the mapping table on the audit page and this \
         test together: {fields}"
    );
    // The client address is present and is deliberately NOT the answer.
    assert_eq!(event.client_ip.as_deref(), Some("10.0.0.9"));
}

/// The purpose of use is recorded but reaches neither rendering, and only one
/// of the two could carry it.
///
/// FHIR R4 `AuditEvent` defines `agent.purposeOfUse`
/// (<https://hl7.org/fhir/R4/auditevent.html>) and this rendering does not
/// populate it — a real gap. The DICOM Audit Message schema of PS3.15 §A.5
/// defines no purpose element at all: its `AuditMessage` carries
/// `EventIdentification`, `ActiveParticipant`, `AuditSourceIdentification`
/// and `ParticipantObjectIdentification`, and none of them has one
/// (<https://dicom.nema.org/medical/dicom/current/output/chtml/part15/sect_A.5.html>).
/// So the DICOM side is a limit of the format rather than of this code, and
/// the assertion below pins both halves for what each of them is.
#[test]
fn the_declared_purpose_is_stored_but_reaches_neither_rendering() {
    let event = access_event();
    assert_eq!(event.purpose.as_deref(), Some("TREAT"));
    let (xml, json) = renderings(&event);
    assert!(
        !xml.contains("TREAT") && !xml.contains("PurposeOfUse"),
        "the DICOM rendering has grown a purpose — update the mapping table and \
         this test together: {xml}"
    );
    assert!(
        !json.contains("TREAT") && !json.contains("purposeOfUse"),
        "the FHIR rendering has grown a purposeOfUse — update the mapping table \
         and this test together: {json}"
    );
}

/// The trail separates the two pseudonymisation domains, which is what makes
/// "who read this person's identifying data" answerable on its own.
#[test]
fn every_resource_class_answers_which_domain_it_touched() {
    for (class, domain) in [
        (ObjectClass::Ehr, AccessDomain::Ehr),
        (ObjectClass::Composition, AccessDomain::Ehr),
        (ObjectClass::Contribution, AccessDomain::Ehr),
        (ObjectClass::Directory, AccessDomain::Ehr),
        (ObjectClass::Query, AccessDomain::Ehr),
        (ObjectClass::Extract, AccessDomain::Ehr),
        (ObjectClass::Demographic, AccessDomain::Demographic),
        (ObjectClass::Template, AccessDomain::System),
        (ObjectClass::ApplicationActivity, AccessDomain::System),
        (ObjectClass::Authentication, AccessDomain::System),
    ] {
        assert_eq!(
            AccessDomain::of(class),
            domain,
            "{class:?} must answer which pseudonymisation domain it touched"
        );
    }
}
