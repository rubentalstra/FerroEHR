# Templates & validation

A template is what tells EHRbase-rs what clinical data to accept. Before you can
commit a composition, you upload the **Operational Template (OPT)** it conforms
to; from that template the server derives everything it needs to validate
incoming data and to describe the data's shape to client applications. This
chapter covers uploading and retrieving templates, the derived WebTemplate, the
convenience FLAT and STRUCTURED composition formats, and how validation behaves
on commit. If templates and archetypes are new to you, read the
[openEHR primer](concepts/openehr-primer.md) first.

## Uploading a template

EHRbase-rs ingests templates in the **OPT 1.4** XML format. Upload one with
`Content-Type: application/xml`:

```shell
curl -u ehrbase:ehrbase \
  -H 'Content-Type: application/xml' \
  --data-binary @vital_signs.opt \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4
```

A successful upload returns **201 Created**. Uploading a template whose id
already exists returns **409 Conflict** — templates are immutable once loaded.
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
as `text/plain` source on `…/definition/template/adl2` and validated at
registration: mandatory sections, header versions, root type and node-id
rules, specialisation depth, terminology language consistency, code
definedness, value-set validity. A duplicate ADL2 template id returns
**409 Conflict** on this endpoint.

List and retrieve loaded templates:

```shell
# List all templates
curl -u ehrbase:ehrbase \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4

# Get the canonical OPT XML for one template
curl -u ehrbase:ehrbase -H 'Accept: application/xml' \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4/vital_signs
```

## The WebTemplate

The **WebTemplate** is a JSON description of a template that is far easier for
application code to consume than raw OPT XML — it lists every field with its
path, type, cardinality, allowed values, and labels, which is exactly what you
need to render a form or map data. Request it with the WebTemplate media type:

```shell
curl -u ehrbase:ehrbase \
  -H 'Accept: application/openehr.wt+json' \
  http://localhost:8080/ehrbase/rest/openehr/v1/definition/template/adl1.4/vital_signs
```

EHRbase-rs follows the widely used Better `web-template` semantics (format
version `2.3`), so tooling built for that model works unchanged.

You can also fetch an **example composition** for a template — a skeleton
instance you can fill in — from
`GET /definition/template/adl1.4/{template_id}/example`, choosing the input or
output form and the level of detail.

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

> [!NOTE]
> The FLAT and STRUCTURED formats are always relative to a template — the paths
> are template paths. Use them for form-driven capture; use canonical JSON/XML
> for full-fidelity exchange and archival.

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
