[comment]: # (title: Resources)

For the openEHR API, a **resource** is an instance object of a specific openEHR class (type) that can be identified, addressed, handled or managed by the service.
Examples of openEHR resources include:

- top-level content structure: COMPOSITION, EHR_STATUS, FOLDER, PARTY, etc.
- versioned entities: VERSIONED_COMPOSITION, VERSIONED_EHR_STATUS, etc. or their contained VERSION instances.
- non-versioned resources: EHR, CONTRIBUTION, RESULT_SET, etc.
- definitions: TEMPLATE, ARCHETYPE, QUERY.

In this specification, types defined by openEHR are always capitalized. 
For a complete list of available types, refer to the [class index](https://specifications.openehr.org/classes).

# Resource identification

A key aspect of a RESTful service is the ability to address resources via URIs/URLs using HTTP methods.
Each resource has a unique identifier within the system for its type, typically including a globally unique identifier and, in some cases, a version identifier.
Once assigned, a resource identifier remains unchanged throughout its lifetime.

## Identifier types

Depending on the resource type and use case, different identifiers (and formats) are used:

- a `versioned_object_uid` for identifying a VERSIONED_OBJECT (i.e. a version container), stored under VERSIONED_OBJECT.uid.value, in a form of a HIER_OBJECT_ID,
- a `version_uid` for identifying a VERSION from the version container, stored under VERSION.uid.value, in a form of an OBJECT_VERSION_ID,
- an `ehr_id` to identify an EHR,
- a `template_id` to identify a Template definition.
- a `qualified_query_name` to identify Query definition.

The `version_uid` uniquely identifies a VERSION, in the lexical form of `object_id :: creating_system_id :: version_tree_id`. Here, the `object_id` matches the VERSIONED_OBJECT identifier, taken from VERSIONED_OBJECT.uid.value (i.e. the `versioned_object_uid`).

Consider the following request:

```http
GET https://openEHRSys.example.com/v1/ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1
```

- the EHR identifier is `7d44b88c-4199-4bad-97dc-d78268e01398`, taken from EHR.ehr_id.value,
- the COMPOSITION identifier `8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1`, taken from COMPOSITION.uid.value, which also implies that the VERSIONED_OBJECT identifier is `8849182c-82ad-4088-a07f-48ead4180515` and the latest version is `1`,
- the request is made to an openEHR API (of version `v1`), located at `https://openEHRSys.example.com`.

> NOTE: Since [RM Release 1.0.4](https://specifications.openehr.org/releases/RM/Release-1.0.4) it is strongly recommended that the inherited `uid` attribute in COMPOSITION objects be populated using the `uid` copied from the enclosing VERSION object.
  For example, if an ORIGINAL_VERSION.uid is `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2`, then this value should be copied to the COMPOSITION.uid attribute.
  See [Levels of identification](https://specifications.openehr.org/releases/BASE/latest/architecture_overview.html#\_levels_of_identification) section of Architecture Overview for more details.

## Multiple identifiers for the same resource

There are circumstances when openEHR resources can be uniquely identified by two different identifiers. 
For example, if a VERSIONED_COMPOSITION container has the _versioned_object_uid_ identifier `8849182c-82ad-4088-a07f-48ead4180515`, and the latest version is `5` (taken from _version_tree_id_), then the COMPOSITION can be accessed using either:

```http
GET https://openEHRSys.example.com/v1/ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com:5
```

as **explicit version reference**, using the _version_uid_ identifier,

```http
GET https://openEHRSys.example.com/v1/ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/8849182c-82ad-4088-a07f-48ead4180515
```
as **implicit latest version reference**.

However, the implicit URI will only resolve to the same resource as the explicit versioned URI as long as no new versions are created (i.e. the latest version remains `5`).


# Data representation

Services MUST support at least one of the openEHR **XML** or **JSON** canonical formats for resource representation.
Additionally, the **Simplified Formats** format SHOULD be supported, while other alternative formats MAY be supported as well, depending on the use case.
These formats and their negotiation protocols are described below.

> NOTE: Examples of data instances, serialized in the preferred format, can be retrieved from the server using the dedicated resource endpoint described in the [Get example data by template](definition.html#tag/ADL1.4/operation/definition_template_adl1.4_example_get) section of the Definition API.

## XML Format

When resources are serialized in **canonical XML** format, both request payloads and responses MUST conform to the [published XSDs](https://specifications.openehr.org/releases/ITS-XML/latest).

A client MAY use the header `Content-Type: application/xml` in the requests to specify the XML payload format.
If the service cannot process the request payload as XML format, it MUST respond with HTTP status code `415 Unsupported Media Type`.

The client SHOULD use the `Accept: application/xml` request header to specify the expected XML response format.
If the service cannot fulfill this aspect of the request, it MUST respond with HTTP status code `406 Not Acceptable`.
Proper header `Content-Type: application/xml` MUST be present in the response of the service
unless the response has no content body (HTTP status code `204`).

## JSON Format

When resources are serialized as **canonical JSON**, the request payload as well as the result SHOULD be valid against [published JSON-Schemas](https://specifications.openehr.org/releases/ITS-JSON/latest).

> NOTE: The JSON-Schema project is under development.

Attribute names must be lowercase _snake_case_ names as specified in the equivalent RM type. For example:

```json
{
    "category": {
        "value": "event",
        "defining_code": {
            "terminology_id": {
                "value": "openehr"
            },
            "code_string": "433"
        }
    }
}
```

Metadata attributes (those that are not also RM attributes) will always be prefixed by a `'_'`.

One example is the `_type` attribute, which should be used to specify the RM type whenever polymorphism is involved, or when the underlying definition in RM type is abstract (dynamic type is different from the static type).
This follows the same rule as for XML typing.
The value of this attribute MUST be the uppercase class name from the RM specification. For example:

```json
{
    "_type": "DV_TEXT",
    "value": "Hello world!"
}
```

The RM attributes (even required ones) that are `Null` or an empty list (array) SHOULD be absent when serialized as JSON.

The order of attributes in the resource MAY follow the order of attributes in the RM specification of the type of the resource, but this is not mandatory.

A client MAY use the header `Content-Type: application/json` in the requests to specify the JSON payload format.
If the service cannot process the request payload as JSON format, it MUST respond with HTTP status code `415 Unsupported Media Type`.

The client SHOULD use the `Accept: application/json` request header to specify the expected JSON response format.
If the service cannot fulfill this aspect of the request, it MUST respond with HTTP status code `406 Not Acceptable`.
Proper header `Content-Type: application/json` MUST be present in the response of the service unless the response has no content body (HTTP status code `204`).

## Simplified Formats

The detailed specifications of these formats can be consulted at [Simplified Formats](simplified_formats.html) page.

To use these formats, content negotiation SHOULD be done in the same manner as for the canonical XML or JSON format above, but instead of `application/xml` or `application/json`, clients and servers MUST use:

- `application/openehr.wt.flat+json` for the Simplified Flat JSON format,
- `application/openehr.wt.structured+json` for the Simplified Structured JSON format,
- `application/openehr.wt+json` for the Operational Template definition as Web Template JSON format.

> NOTE: The `application/openehr.wt.flat.schema+json` and `application/openehr.wt.structured.schema+json` which were historically available in earlier versions of this specification, are now deprecated and will be removed in a future version of the specification. The equivalent formats `application/openehr.wt.flat+json` and `application/openehr.wt.structured+json` SHOULD be used instead.

A client MAY use the header `Content-Type` in the requests to specify the simplified payload format.
If the service cannot process the request payload as the simplified format is not supported, it MUST respond with HTTP status code `415 Unsupported Media Type`.

The client SHOULD use the `Accept` request header to specify the expected simplified response format.
If the service cannot fulfill this aspect of the request, it MUST respond with HTTP status code `406 Not Acceptable`.
Proper header `Content-Type` MUST be present in the response of the service unless the response has no content body (HTTP status code `204`).

## Alternative data formats

Creating data instances according to canonical XML or JSON format is not always straightforward, particularly for developers with minimal exposure to openEHR.
Various alternatives have been used in the past to simplify the job of content creation and committal for application developers.

Legacy or experimental formats: 

- `application/openehr.nc.flat+json` based on the [ECISFLAT format](https://github.com/ethercis/ethercis/blob/master/doc/flat%20json.md), 
- `application/openehr.tds2+xml` for TDS simplification of RM as XML (see [this wiki page](https://openehr.atlassian.net/wiki/spaces/spec/pages/30408770/Template+Data+Schema+TDS+Specification+and+associated+Template+Data+Document+TDD) and the [TDD2canonical](https://github.com/openEHR/openEHR-TDD2canonical) project),
- `application/openehr.wt.flat.schema+json` deprecated in favour of `application/openehr.wt.flat+json`,
- `application/openehr.wt.structured.schema+json` deprecated in favour of `application/openehr.wt.structured+json`.

Other alternative formats may be added in the future, depending on innovative impact, simplicity, popular demand, or adoption rate.
Some of these formats might not be supported once they become obsolete or superseded by newer formats.

## Datetime format

The format of the date, time and datetime types should comply with the [ISO 8601 Date and Time](https://en.wikipedia.org/wiki/ISO\_8601) format specification.
The ISO 8601 semantics assumed by openEHR is defined in the classes of the [base.foundation_types.time package](https://specifications.openehr.org/releases/BASE/latest/foundation_types.html#\_time_types).
It is strongly recommended using the _extended_ format for dates, times and datetimes.
This greatly aids human readability, and reduces the need for special date/time parsing and formatting.

HTTP query parameters and path segments that are dates, datetimes, or times, MUST always use the _extended_ ISO 8601 format.
The general form of a datetime is `YYYY-MM-DDThh:mm:ss.sss[Z|±hh:mm]`, e.g. `2016-06-23T13:42:16.117+02:00`.\
Timezone SHOULD be only supplied when needed, otherwise the local timezone is assumed.

Any date, datetime or time value provided as part of the HTTP message body, when creating or updating a resource (e.g. a DV_DATE_TIME value inside the COMPOSITION content), will be preserved as it was sent by the client, and passed to the underlying backend engine as is.
Retrieval or querying those resources SHOULD return date, datetime, or time values in the (original) format provided by underlying backend engine, avoiding any format change.
