---
name: equivalent-refused-on-text-document-grounds
description: RUNNER bin — veredictum 0.1.4 blanket-refuses `equivalent` on any non-JSON body, including OPT-XML/ADL2-text grounds where the comparison is text-vs-text and fully judgeable
metadata:
  type: project
---

Veredictum 0.1.4 (`app/veredictum/src/exec/driver.rs::unjudgeable_on_unparsed_body`,
change #285) routes `Field | Equivalent | ResultSet | InstanceOf | Signature` to
the inconclusive channel whenever `BodyForm::Unparsed` — keyed on the SERVED
media type alone, never on the form of the comparison GROUND. `XmlRoot` and
`Returns` are carved out as "judges the served document text".

**Why that is over-scoped:** a corpus entry with format `opt-xml` /
`adl2-text` / `adl14-text` / `canonical-xml` / `aql-text` resolves to
`Value::String` (`exec/resolve.rs::data_set`), so `equivalent to:
"${ds:…}"` on those endpoints is a text-vs-text comparison — the same class
`returns` already grades. Three rows are affected (all previously passing
under 0.1.0): `I_DEFINITION_ADL14.get_opt-retrieve_single`,
`…-retrieve_specific_version`, `I_DEFINITION_ADL2.get_artefact-retrieve`.

**Spec ground that makes them judgeable and non-optional:** the ADL 1.4 GET's
released 200 declares `application/xml` + `application/openehr.wt+json` ONLY
(`ITS-REST specifications/responses/200_Template_adl1_4_retrieved.yaml`) and
the ADL2 GET's declares `text/plain` + `application/json`
(`…/200_Template_adl2_retrieved.yaml`); the docs text names
`application/openehr.wt+json` as the template's JSON offering
(`specifications/docs/overview/Resources.md` §Simplified Formats) and defines
NO canonical-JSON serialization of an operational template (AMB-58). So there
is no JSON exchange to fall back to, and AMB-111's fixed handling is that
retrieval is VERBATIM. Fix path: scope the `Equivalent` refusal to a JSON
ground (object/array) — or to a case carrying `ignoring`/`server_assigned`
exclusions, which a text compare cannot honour — and keep judging a string
ground. New pinning test belongs in `tests/it/unparsed_bodies.rs`.
