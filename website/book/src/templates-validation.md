# Templates & validation

A template is what tells FerroEHR what clinical data to accept. Before you can
commit a composition, you upload the **Operational Template (OPT)** it conforms
to; from that template the server derives everything it needs to validate
incoming data and to describe the data's shape to client applications. This
chapter covers uploading and retrieving templates, the derived WebTemplate, the
convenience FLAT and STRUCTURED composition formats, and how validation behaves
on commit. If templates and archetypes are new to you, read the
[openEHR primer](concepts/openehr-primer.md) first.

## Uploading a template

FerroEHR ingests templates in the **OPT 1.4** XML format. Upload one with
`Content-Type: application/xml`:

```shell
curl -u ferroehr:ferroehr \
  -H 'Content-Type: application/xml' \
  --data-binary @vital_signs.opt \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/template/adl1.4
```

The upload takes OPT XML and nothing else: a request declaring another
payload type (`Content-Type: application/json`, say) is refused with **415
Unsupported Media Type** before the template is parsed. `text/xml` works the
same as `application/xml`, and omitting the header altogether is fine — the
endpoint has only one body format.

A successful upload returns **201 Created**. Add
`Prefer: return=identifier` when you only need the id back — the response
body is then the JSON identifier object:

```json
{ "template_id": "vital_signs" }
```

(`return=representation` returns the stored OPT XML; the default is an empty
body with the id in `ETag`/`Location`.) Uploading a template whose id
already exists returns **409 Conflict** — templates are immutable once loaded.
Template ids are compared **case-insensitively** (the stored casing is
preserved), so uploading a case-variant of an existing id — `Vital_Signs`
against a stored `vital_signs` — is also a **409 Conflict**, not a second
template.
On upload the server checks the template itself for artefact validity — that
its constraints are internally consistent per the openEHR archetype model:
reference-model conformance of every constrained type and attribute,
occurrence/cardinality consistency, terminology code definedness and
language consistency, archetype-identifier well-formedness, constraint
pattern validity (temporal and duration patterns, boolean satisfiability,
assumed values inside their own constraints), and more. An invalid template
is rejected with **400 Bad Request** carrying the specific archetype-model
rule code (e.g. `VCARM`, `VATID`, `Pattern_validity`) and a human-readable
detail.

ADL2 artefacts (archetypes, templates, operational templates) are accepted
as `text/plain` source on `…/definition/template/adl2` and validated by the
full ADL2 engine: the source is parsed, then checked against the AOM2
validity catalogue (phase 1 basic integrity, reference-model conformance, and
— for a specialised artefact whose parent is already loaded — specialisation
conformance). An invalid artefact is rejected with **422 Unprocessable
Entity** whose error body lists the offending rule codes in `validationErrors`
(the S-codes for a source that cannot be parsed, the V-codes for a
validation-phase failure, e.g. `VARD`, `VCORM`, `VACSD`). A duplicate ADL2
template id returns **409 Conflict** on this endpoint. Deleting an ADL2
template that committed compositions still reference is also a
**409 Conflict** naming the reference count — the same never-orphan guard the
OPT 1.4 delete applies; delete the compositions first.

A loaded ADL2 template is retrieved in either of two representations, chosen
by `Accept`: the stored ADL2 **source** (`text/plain`, the default) or the
**operational template as JSON** (`application/json`). A partial
`template_id` resolves to the latest matching version:

```shell
# The ADL2 source, verbatim
curl -u ferroehr:ferroehr -H 'Accept: text/plain' \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/template/adl2/openEHR-EHR-COMPOSITION.t_vitals.v1.0.0

# The operational template as JSON
curl -u ferroehr:ferroehr -H 'Accept: application/json' \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/template/adl2/openEHR-EHR-COMPOSITION.t_vitals.v1
```

List and retrieve loaded templates:

```shell
# List all templates
curl -u ferroehr:ferroehr \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/template/adl1.4

# Filter and page the list
curl -u ferroehr:ferroehr \
  'http://localhost:8080/ferroehr/rest/openehr/v1/definition/template/adl1.4?template_id=vital*&offset=0&fetch=20'

# Get the canonical OPT XML for one template
curl -u ferroehr:ferroehr -H 'Accept: application/xml' \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/template/adl1.4/vital_signs
```

The ADL 1.4 list accepts three filters — `template_id`, `concept`, and
`version`, each a glob pattern (`*` wildcards, e.g.
`template_id=vital*`) — plus `offset` (rows to skip, default 0) and `fetch`
(maximum rows; absent or 0 = all). The ADL2 list accepts the same
`offset`/`fetch` pagination.

> [!NOTE]
> When the `version` parameter is absent, the list collapses to the **latest
> version** of each template (the highest `.vN` axis of its `template_id`).
> Pass `version=*` to list every stored version, or a partial glob
> (`version=*.v1*`) to select specific ones.

## The WebTemplate

The **WebTemplate** is a JSON description of a template that is far easier for
application code to consume than raw OPT XML — it lists every field with its
path, type, cardinality, allowed values, and labels, which is exactly what you
need to render a form or map data. Request it with the WebTemplate media type:

```shell
curl -u ferroehr:ferroehr \
  -H 'Accept: application/openehr.wt+json' \
  http://localhost:8080/ferroehr/rest/openehr/v1/definition/template/adl1.4/vital_signs
```

FerroEHR follows the widely used Better `web-template` semantics (format
version `2.3`), so tooling built for that model works unchanged.

`Accept: application/json` on the same URL returns that identical WebTemplate
document — it is the only JSON representation of a template — but labelled
`Content-Type: application/json`, the type you asked for. Use
`Accept: application/xml` for the canonical OPT instead.

You can also fetch an **example composition** for a template — a skeleton
instance you can fill in — from
`GET /definition/template/adl1.4/{template_id}/example`, choosing the input or
output form and the level of detail. The same example endpoint is available for
ADL2 templates at `GET /definition/template/adl2/{template_id}/example`: the
stored operational template is turned into a WebTemplate and walked into an
example composition, served in any of canonical JSON/XML, FLAT, or STRUCTURED
(via `Accept`), with the same `type` (`input`/`output`) and `detail_level`
(`required`/`medium`/`complete`) query parameters.

## Composition formats

When committing or retrieving a composition, the canonical openEHR JSON (or XML)
is always available, but two flatter formats are offered for convenience, keyed
to a template:

- **FLAT (simSDT)** — `application/openehr.wt.flat+json`. The whole composition
  as a single flat map of `path|attribute → value`, which is compact and easy
  to produce from a form. For example:

  ```json
  {
    "vital_signs/blood_pressure/any_event:0/systolic|magnitude": 120,
    "vital_signs/blood_pressure/any_event:0/systolic|unit": "mm[Hg]",
    "vital_signs/blood_pressure/any_event:0/diastolic|magnitude": 80,
    "vital_signs/blood_pressure/any_event:0/diastolic|unit": "mm[Hg]",
    "vital_signs/language|code": "en",
    "vital_signs/language|terminology": "ISO_639-1"
  }
  ```

- **STRUCTURED (structSDT)** — `application/openehr.wt.structured+json`. The
  same data as a nested JSON tree that mirrors the template structure, rather
  than a flat map.

Send the matching `Content-Type` when committing, or the matching `Accept` when
retrieving, and the server converts between the flat/structured form and the
canonical composition. These formats are a Better/EHRbase interoperability
convenience; the canonical JSON and XML remain the openEHR-standard wire format.
They work against both ADL 1.4 and ADL2 templates: a FLAT or STRUCTURED commit
keyed to an ADL2-registered template resolves and is validated against that
template's archetype constraints exactly as an ADL 1.4 commit is.

> [!NOTE]
> The FLAT and STRUCTURED formats are always relative to a template — the paths
> are template paths. Use them for form-driven capture; use canonical JSON/XML
> for full-fidelity exchange and archival.

### Optional RM attributes (`_`-prefixed keys)

Beyond the template's own fields, FLAT and STRUCTURED carry the optional
reference-model attributes as `_`-prefixed path segments, round-tripping in
both directions. Indexed families take an `:n` suffix; sub-fields ride the
usual `|attribute` pipes:

```json
{
  "vital_signs/blood_pressure/_uid": "9fcc1c70-…",
  "vital_signs/blood_pressure/_link:0|type": "problem",
  "vital_signs/blood_pressure/_link:0|target": "ehr://…",
  "vital_signs/blood_pressure/any_event:0/systolic/_null_flavour|code": "253",
  "vital_signs/blood_pressure/any_event:0/systolic/_normal_range/lower|magnitude": 90,
  "vital_signs/blood_pressure/_other_participation:0|function": "witness",
  "vital_signs/blood_pressure/_other_participation:0|name": "Dr. Marcus Johnson"
}
```

The supported family: `_uid`, `_link:n`, `_feeder_audit`, `_null_flavour`,
`_null_reason`, `_mapping:n`, `_normal_range`, `_other_reference_ranges:n`,
`_accuracy`, `_language`, `_encoding`, `_charset`, `_provider`,
`_other_participation:n`, `_work_flow_id`, `_guideline_id`, `_expiry_time`,
`_wf_definition`, `_instruction_details`, `_identifier:n`, and `_thumbnail`.

An ACTION's `_instruction_details` carries exactly three suffixes on the field
itself — the instruction's path within its composition, that composition's uid,
and the activity id:

```json
{
  "encounter/procedure/_instruction_details|path": "/content[openEHR-EHR-INSTRUCTION.request.v1]",
  "encounter/procedure/_instruction_details|composition_uid": "4cdc3017-d8c5-4cd3-9900-f3bb7171d006",
  "encounter/procedure/_instruction_details|activity_id": "activities[at0001]"
}
```

An interval event additionally carries `|sample_count` (the number of samples
the interval summarises) alongside its `/width` and `/math_function` fields.

An element that records *why* a value is missing carries `_null_flavour` (and
optionally `_null_reason`) **instead of** a value — the reference model makes
the two mutually exclusive — and both directions preserve it:

```json
{
  "vital_signs/blood_pressure/any_event:0/systolic/_null_flavour|code": "253",
  "vital_signs/blood_pressure/any_event:0/systolic/_null_flavour|value": "unknown",
  "vital_signs/blood_pressure/any_event:0/systolic/_null_flavour|terminology": "openehr",
  "vital_signs/blood_pressure/any_event:0/systolic/_null_reason": "not asked"
}
```

### Who the entry is about: `subject`

Every entry (observation, evaluation, instruction, action, admin entry)
records a subject. It defaults to the owner of the EHR, and while it stays the
default it does not appear on the wire at all. When an entry is about someone
else — a relative, a donor, a fetus — spell it out with the party suffixes:

```json
{
  "family_history/family_history/subject|name": "Susan Doe",
  "family_history/family_history/subject|id": "199",
  "family_history/family_history/subject|id_scheme": "HOSPITAL-NS",
  "family_history/family_history/subject|id_namespace": "HOSPITAL-NS",
  "family_history/family_history/subject/relationship|code": "10",
  "family_history/family_history/subject/relationship|value": "mother",
  "family_history/family_history/subject/relationship|terminology": "openehr"
}
```

A `/relationship` sub-path makes the subject a *related party*; additional
identifiers ride the `/_identifier:n` family; `|_type: "PARTY_SELF"` marks a
subject that is the EHR owner yet still carries an external reference.

### Context fields: `ctx/` shortcut or full path

The composition's event context is normally set through the `ctx/` shortcuts
(`ctx/time`, `ctx/setting`, `ctx/language`, …). The equivalent full paths are
accepted on input too, and take precedence over the shortcut defaults:
`…/context/start_time`, `…/context/setting`, and an entry's `…/language` and
`…/encoding`. A bare `…/context/setting|code` is resolved against the openEHR
*setting* value set, exactly as `ctx/setting` is. Output always uses the
`ctx/` form for these, so a round trip through FLAT is stable. A path the
Simplified Formats specification does not define is rejected with an error —
never silently dropped.

### Embedding canonical JSON with `|raw`

When one node needs full fidelity inside an otherwise-FLAT commit, write the
node's canonical JSON verbatim under the `|raw` suffix:

```json
{
  "vital_signs/blood_pressure/any_event:0/systolic|raw": {
    "_type": "DV_QUANTITY", "magnitude": 120, "unit": "mm[Hg]"
  }
}
```

The embedded object **must** carry `_type` (without it, the key is treated as
a normal leaf). `|raw` is write-only: retrieval always decomposes to regular
FLAT keys.

### Coded text and open value sets

A coded field whose template value set is **open** accepts free text under
the `|other` suffix. `|other` may not be combined with `|code`, `|value`,
`|terminology`, or `|preferred_term` on the same leaf, and is rejected when
the value set is closed.

### Duplicate node names

When a template contains sibling nodes with the same name, the generated
WebTemplate/FLAT path ids are disambiguated with underscore suffixes counted
from 1 — `blood_pressure`, `blood_pressure_1`, `blood_pressure_2` — as the
specification prescribes. There is no vendor-compatibility mode: the
specification's numbering is the only numbering, and vendor-only DV_QUANTITY
suffixes such as `|unit_system` and `|unit_display_name` are not accepted. If
your tooling was written against another server's path ids, map them on the
client side.

## Validation on commit

Every composition is validated against its template at commit time — this is
where the template earns its keep. The server checks:

- **structure** — required sections and fields are present, and cardinality and
  occurrence constraints are respected;
- **leaf values** — data types, units, value ranges, string patterns, decimal
  precision, and date/time constraints match the template;
- **terminology** — coded values are members of the value sets the template
  binds, using the bundled openEHR terminology or a configured external FHIR
  terminology server (see [Terminology servers](beyond-core/terminology.md)).

If a composition is well-formed but breaks its template, the commit fails with
**422 Unprocessable Entity** and a `validationErrors` list — one entry per
offending node, as `"<path>: <message>"` — so a client can show the user exactly
what to fix. A syntactically malformed request instead gets **400 Bad Request**.
The error shapes are described in
[Content negotiation & errors](using-the-api/content-negotiation.md).

## Next

- [Using the API → Resource walkthroughs](using-the-api/resources.md) — the
  full composition create/update/delete lifecycle.
- [Querying with AQL](querying-aql.md) — reading the data back out by path.
