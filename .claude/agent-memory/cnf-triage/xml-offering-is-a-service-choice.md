---
name: xml-offering-is-a-service-choice
description: AMB-167 "DOCUMENTED" answers WHICH document if XML is offered — it never makes OFFERING XML mandatory; §Data representation keeps that a service choice
metadata:
  type: project
---

Two orthogonal questions the AMB-167 handling paragraph conflates:

1. **Must the service OFFER XML on this resource?** NO, for every resource —
   ITS-REST `docs/overview/Resources.md` §"Data representation": "Services MUST
   support **at least one** of the openEHR **XML** or **JSON** canonical formats
   for resource representation." §"XML Format" then designates the refusal
   answer: "If the service cannot fulfill this aspect of the request, it MUST
   respond with HTTP status code `406 Not Acceptable`." A 406 on
   `Accept: application/xml` is therefore a CONFORMANT answer anywhere.
2. **If offered, is the document defined?** That is the DOCUMENTED vs UNDEFINED
   split (published global `xs:element` inventory).

AMB-167's own `source:` states (1) verbatim, but its `handling:` then says "The
DOCUMENTED rows stay ungated and unconditional" — that sentence is the defect:
it turns a service choice into an obligation. Any `-xml` case with no offering
arm over-asserts.

Live instance (2026-07-28): `I_ITS_REST_VERSIONED_PARTY.versioned_party_get-xml`
errored on a spec-correct 406 while `I_EHR_COMPOSITION.get_versioned_composition-xml`
and the bare-PARTY `-xml` rows pass — the server serves XML for those resources
and not for the demographic VERSIONED_* container. Per-resource asymmetry is
not a conformance defect. CATALOGUE bin: add an offering arm + a mirror
`-xml_not_acceptable` case (coverage ratchets up, never down).
