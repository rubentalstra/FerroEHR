[comment]: # (title: Specifications)

The openEHR REST specifications include the following service endpoints: 
 - [System API](system.html)
 - [EHR API](ehr.html)
 - [Demographic API](demographic.html)
 - [Query API](query.html)
 - [Definition API](definition.html)
 - [Admin API](admin.html)

Specifications can be downloaded as YAML files in [OpenAPI Specification 3.0](https://spec.openapis.org/oas/v3.0.3) format.
For each API there are two flavours provided: 
- a file optimized for code generators, with the file name having the `-codegen.openapi.yaml` suffix; this file can be used with tools like [OpenAPI Generator](https://github.com/openapitools/openapi-generator), or [Swagger codegen](https://github.com/swagger-api/swagger-codegen),
- a file optimized for data validation, with the file name having the `-validation.openapi.yaml` suffix; this file can be used by (mock-)servers or applications to validate openEHR request and response payloads.

The main difference between these files (aside from schema model names) is that the codegen flavour is defining and using inheritance in model schemas, with the use of `allOf` property and discriminators, whereas the validation variant is flattening all these requirements (each model contains all RM-inherited properties), and is using `oneOf` property to define union-types. See [Polymorphism](https://spec.openapis.org/oas/v3.0.3#composition-and-inheritance-polymorphism) specification. 

The latest OpenAPI Specification files are available in GitHub at [openEHR/specifications-ITS-REST/computable/OAS](https://github.com/openEHR/specifications-ITS-REST/tree/master/computable/OAS).
