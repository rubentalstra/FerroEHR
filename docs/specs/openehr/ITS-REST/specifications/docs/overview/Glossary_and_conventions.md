[comment]: # (title: Glossary and conventions)

Throughout this specification, a set of short terms is being used as described below:

| <div style="width:180px">Term</div> | Description                                                                                                                                                                                                                                        |
|-------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| API                                 | Application Programmer Interface                                                                                                                                                                                                                   |
| REST                                | [Representational state transfer](https://en.wikipedia.org/wiki/Representational_state_transfer), a type of Web service that allows clients to access and manipulate textual representations of Web resources.                                     |
| OAS                                 | An acronym for [OpenAPI Specification](https://spec.openapis.org/oas/v3.0.3)                                                                                                                                                                       |
| AQL                                 | openEHR's [Archetype Query Language](https://specifications.openehr.org/releases/QUERY/latest/AQL.html)                                                                                                                                            |
| RM                                  | openEHR's [Reference Model](https://specifications.openehr.org/releases/RM/latest)                                                                                                                                                                 |
| SEMVER                              | [Semantic Versioning 2.0.0](https://semver.org/)                                                                                                                                                                                                   |
| `UUID`                              | A universally unique identifier as described by [RFC 4122](https://tools.ietf.org/html/rfc4122) (e.g. 8849182c-82ad-4088-a07f-48ead4180515)                                                                                                        |
| `ehr_id`                            | The value for an EHR identifier, stored under EHR.ehr_id.value, in a form of a HIER_OBJECT_ID, usually an `UUID` or a `GUID` (e.g. 7d44b88c-4199-4bad-97dc-d78268e01398)                                                                           |
| `versioned_object_uid`              | The value of a VERSIONED_OBJECT unique identifier, stored under VERSIONED_OBJECT.uid.value, in a form of a HIER_OBJECT_ID (e.g. 8849182c-82ad-4088-a07f-48ead4180515)                                                                              |
| `version_uid`                       | The value of a VERSION unique identifier, stored under VERSION.uid.value, in a form of an OBJECT_VERSION_ID (e.g. 8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2)                                                                 |
| `uid_based_id`                      | An abstract identifier: it can take a form of an OBJECT_VERSION_ID identifier taken from VERSION.uid.value (i.e. a `version_uid`), or a form of a HIER_OBJECT_ID identifier taken from VERSIONED_OBJECT.uid.value (i.e. a `versioned_object_uid`). |
| `preceding_version_uid`             | The value of a previous VERSION unique identifier, used usually for PUT or DELETE methods (e.g. 8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1)                                                                                   |
| `version_at_time`                   | Time specifier used to retrieve the VERSION at a given time; the value is in the extended ISO 8601 format (e.g. 2015-01-20T19:30:22.765+01:00)                                                                                                     |
<br>

### OpenAPI Specification files

This openEHR REST specifications can be downloaded as YAML files in [OpenAPI Specification 3.0](https://spec.openapis.org/oas/v3.0.3) format.

For each API there are two flavours provided: 
- a file optimized for code generators, with the file name having the `-codegen.openapi.yaml` suffix; this file can be used with tools like [OpenAPI Generator](https://github.com/openapitools/openapi-generator), or [Swagger codegen](https://github.com/swagger-api/swagger-codegen),
- a file optimized for data validation, with the file name having the `-codegen.openapi.yaml` suffix; this file can be used by (mock-)servers or applications to validate openEHR request and response payloads.

The main difference between these files (aside of schema model names) is that the codegen flavour is defining and using inheritance in model schemas, with the use of `allOf` property and discriminators, whereas the validation variant is flattening all these requirements (each model contains all RM-inherited properties), and is using `oneOf` property to define union-types. See [Polymorphism](https://spec.openapis.org/oas/v3.0.3#composition-and-inheritance-polymorphism) specification. 

The latest OpenAPI Specification files are available in GitHub at [openEHR/specifications-ITS-REST/computable/OAS](https://github.com/openEHR/specifications-ITS-REST/tree/master/computable/OAS).
