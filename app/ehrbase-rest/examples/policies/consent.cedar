// Illustrative embedded-Cedar policy set for the ehrbase-rs ABAC layer,
// reproducing the *shape* of EHRbase v1's external consent checks
// (`has_consent_patient` + `has_consent_template`) against the shipped schema.
// No openEHR spec governs consent-based authorization — our own extension.
//
// Cedar is DENY-BY-DEFAULT and `forbid` overrides `permit`. v1 ran each
// configured policy independently and required ALL to return 200 (logical AND);
// because multiple Cedar `permit` policies are OR-combined, the AND is expressed
// here as a SINGLE `permit` whose `when` clause carries both checks. Real
// deployments replace the bodies with their own consent rule (or a set backed by
// their own entity store).

// has_consent_patient  ∧  has_consent_template
//
// NOTE (Cedar strict validation): every optional-attribute access is guarded by
// `resource has <attr> &&` *immediately to its left*, so the policy passes
// strict schema validation at load. The "absent OR matches" shape is therefore
// `!(resource has X) || (resource has X && … resource.X …)`.
permit (
    principal,
    action,
    resource
)
when {
    // has_consent_patient: the caller's organization is known, and — when the
    // resource concerns a patient — it is the patient the caller is authorized
    // for (the caller's `patient` claim; the local subject gate enforces the
    // same invariant before the engine is even consulted).
    principal has organization &&
    (!(resource has patient) ||
        (resource has patient && principal has patient
            && principal.patient == resource.patient))
    &&
    // has_consent_template: when the resource carries a template, it must be on
    // the deployment's allow-list. Extend this set for your templates.
    (!(resource has template) ||
        (resource has template &&
            ["org.openehr::vital_signs.v1", "org.openehr::encounter.v1"].contains(resource.template)))
};

// Example belt-and-suspenders override: a template can be explicitly revoked
// regardless of any permit above.
forbid (
    principal,
    action,
    resource
)
when {
    resource has template && resource.template == "org.openehr::revoked.v1"
};
