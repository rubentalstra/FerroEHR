---
name: contribution-xml-read-not-a-released-document
description: CATALOGUE bin — the CONTRIBUTION canonical-XML READ has no released document form either; AMB-165's "the READ side is not affected and is exercised" claim is false, and a 406 is the docs-text answer
metadata:
  type: project
---

`I_EHR_CONTRIBUTION.get_contribution-xml` / `-has_contribution-xml` expect
`ok` + a body matching `^\s*<` on `Accept: application/xml`; the SUT answers
**406**, which the binding maps to no outcome (errored/inconclusive).

**The release does not require a 200 here.** ITS-REST
`specifications/docs/overview/Resources.md` §XML Format: "The client SHOULD use
the `Accept: application/xml` request header … **If the service cannot fulfill
this aspect of the request, it MUST respond with HTTP status code `406 Not
Acceptable`**", and responses in canonical XML "MUST conform to the [published
XSDs]". §Data representation makes XML a per-service choice ("Services MUST
support at least one of the openEHR **XML** or **JSON** canonical formats").
Verified first-hand in the vendored bundles
(`crates/openehr-its/schemas/xml/its-xml-{1.0.2-nsv1,2.0.0-nsv2}`): the ONLY
globally declared document elements are `composition`, `version`, `items`,
`template`, `extract`, `extract_request`, `versioned_object`, `archetype` —
there is **no `contribution` element** (only the `CONTRIBUTION` complexType in
RM `Common.xsd`). The released OAS is not decisive: `Accept_LOCATABLE` /
`ContentType_LOCATABLE` list `application/xml`, but every 200 `content:` map in
the release binds `application/json` only (composition_get included).

So the read side sits in the SAME position as the commit envelope AMB-165
already declined: media type declared, document withheld. The case cores admit
it ("BODY FLOOR … no released sentence assigns this response's document ROOT
NAME") and then assert a floor anyway — writing spec, not testing it.

**How to apply:** attribute such rows CATALOGUE, never app. Fix = extend
AMB-165 (`artifacts/registers/ambiguities.yaml`, `disposition: report_only`,
#1605) to the READ branch, drop `canonical-xml` from
`bindings/its-rest/I_EHR_CONTRIBUTION.{get,has}_contribution.yaml` `formats:`,
record the exception in `vocab/wire_surface.yaml` (its lines ~503-515 currently
assert the opposite). Related open modelling gap: `statement.json`
`tech_profiles.formats` is party-GLOBAL, so a party that offers XML for some
resources is judged as offering it for all.
