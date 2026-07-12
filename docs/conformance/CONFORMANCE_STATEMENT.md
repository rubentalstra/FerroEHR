# ehrbase-rs Conformance Statement (generated)

> Generated from a conformance run (`results.json`) — never hand-asserted.
> The claim on every line is a pure function of the machine profile
> verdicts (`tools/conformance` §profile).

## System under test

| Field | Value |
|---|---|
| Product | ehrbase-rs 3.0.0 |
| SUT | `http://localhost:8080/ehrbase/rest/openehr/v1` |
| Auth mode | basic |
| Run started | 2026-07-12T07:37:07.906259Z |
| Reference corpus | openEHR/specifications-CNF@33251d2a |

## Supported specification versions

| Specification | Version |
|---|---|
| Reference Model (RM) | 1.2.0 |
| ITS-REST contract | development@e8a093e |
| AQL (QUERY) | 1.1.0 |
| Terminology (TERM) | 3.1.0 |

> CNF requires the Conformance Statement to state the supported RM version(s); the minimum required is RM 1.0.2 (`master03-overview.adoc`). This SUT states **RM 1.2.0**.


## External data formats

Declared: XML, JSON (`master03-profiles.adoc` §Other Non-Functional). This run exercised: json, xml.


## Profile claims (machine-computed)

| Profile | Aggregation | Result |
|---|---|---|
| Core | all capabilities | PASS |
| Standard | all capabilities | PASS |
| Options | any optional capability | OBTAINED |

### Non-functional attributes

- Signing (STANDARD): pass
- Anonymous EHRs (CORE + STANDARD): pass

### OPTIONS — obtained optional capabilities

- DemographicApi
- Terminology
- AdminApi
