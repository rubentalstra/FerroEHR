---
name: authz-class-is-explicit-sm-deferral
description: RBAC class assignment is EXPLICIT SM deferral (not silence), so a 403 is never an app defect — and semantics cases must drive an authorized principal
metadata:
  type: project
---

Which role may call an operation is **explicitly deferred to the
implementation** by the released SM, so a role-based 403 can never be an
application-bin defect; and the same clause is the positive ground for driving
a semantics case as an authorized principal.

`SM/docs/openehr_platform/master02-overview.adoc` §Global Conventions →
§Functional Style lists "approach to access control and authorisation" among the
dimensions where "In real implementations, different choices will be made … only
the resulting semantics do [need to be replicated]", and then:

> "Authentication and authorisation is assumed to have been dealt with before
> any particular call has been made by a combination of standard authentication
> technologies (e.g. OAuth, RFC 7235) and role-based access control."

ITS-REST `specifications/docs/overview/Requests_and_responses.md`
§Authentication and authorization matches: "this specification does not mandate a
specific authentication scheme"; its only MUST is the 401/403/407 header
mechanics. The released Admin API `docs/admin/Description.md` carries no
privilege language at all.

**How to apply.** (1) A red row whose only delta is `expected <semantics>,
observed forbidden` is never the application bin — derive the class question as
own-design. (2) An SM operation's pre/postconditions are specified for a caller
who has ALREADY passed authorization, so a case whose declared outcomes are
semantics-only MUST drive a principal the deployment authorizes; the `on:`
key is ISO/IEC 9646 selection data, not a spec-derived expectation, so changing
it is not the banned "adjust the expectation to match the SUT". (3) The
catalogue's own established pattern is
`schedule/admin/I_ADMIN_SERVICE.physical_ehr_delete-delete_existing.yaml`:
`on: admin` on the destructive step, default `sut` on the verification step.
(4) But a class move ALWAYS costs coverage — the newly-refused principal branch
needs its own case, and any `-readonly_forbidden` row on the moved route stops
isolating the read-only restriction (the Admin gate now refuses first).

See [[definition-delete-routes-are-admin-class]].
