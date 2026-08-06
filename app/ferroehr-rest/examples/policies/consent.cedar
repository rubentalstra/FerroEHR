// Illustrative embedded-Cedar policy set for the FerroEHR ABAC layer.
//
// No openEHR spec governs consent-based authorization — our own extension. The
// vocabulary is NIST SP 800-162's: a decision over subject attributes (who is
// asking) and resource attributes (what is being touched).
//
// Cedar is DENY-BY-DEFAULT and `forbid` overrides `permit`. Multiple `permit`
// policies are OR-combined, so a rule set that must hold CONJUNCTIVELY — every
// condition, not any one — belongs in a SINGLE `permit` whose `when` carries all
// of them, as the first policy below does. Adding a second `permit` widens
// access; narrowing is what `forbid` is for.
//
// NOTE (Cedar strict validation): every optional-attribute access is guarded by
// `<entity> has <attr> &&` immediately to its left, so the set passes schema
// validation at load. The "absent OR matches" shape is therefore
// `!(x has A) || (x has A && … x.A …)`. An unguarded read is refused AT LOAD,
// so the engine never runs a policy it cannot evaluate.

// The clinical rule: the caller's organization is known, the patient matches when
// one is in play, and the template is on the deployment's allow-list.
permit (
    principal,
    action,
    resource
)
when {
    principal has organization &&
    // When the resource concerns a patient it must be the patient this caller is
    // authorized for. The local subject gate enforces the same invariant before
    // the engine is consulted, so this is the second of two independent checks.
    (!(resource has patient) ||
        (resource has patient && principal has patient
            && principal.patient == resource.patient))
    &&
    // When the resource carries a template it must be on the allow-list. Replace
    // this set with your deployment's templates.
    (!(resource has template) ||
        (resource has template &&
            ["org.openehr::vital_signs.v1", "org.openehr::encounter.v1"].contains(resource.template)))
};

// The principal carries the caller's ROLES (as the RBAC gate sees them,
// upper-cased) and their OAuth2 SCOPES, verbatim — so role- and scope-aware rules
// are expressible here, not only attribute ones.
//
// This policy lets a break-glass role reach any patient, bypassing the patient
// match above but NOT the forbid rules below. A deployment that wants no such
// role deletes it.
permit (
    principal,
    action,
    resource
)
when {
    principal.roles.contains("EMERGENCY_ACCESS")
};

// A read-scoped client must not reach a write action, whatever its roles say.
// `forbid` overrides every `permit`, so this is a restriction rather than a
// grant — the same direction as the read-only role in the RBAC tier.
forbid (
    principal,
    action,
    resource
)
when {
    principal.scopes.contains("openehr/*.r") &&
    !principal.scopes.contains("openehr/*.w") &&
    action in [
        Action::"composition.create",
        Action::"composition.update",
        Action::"composition.delete",
        Action::"ehr.create",
        Action::"ehr_status.update",
        Action::"contribution.create"
    ]
};

// An explicitly revoked template is refused regardless of any permit above.
forbid (
    principal,
    action,
    resource
)
when {
    resource has template && resource.template == "org.openehr::revoked.v1"
};
