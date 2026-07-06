[comment]: # (title: Resources)

For the openEHR API, a Resource is an instance object of a specific openEHR class (type) that can be identified, addressed, handled or managed by the service.
The following are examples of openEHR resources and their types:

- top-level content structure types, such as COMPOSITION, EHR_STATUS, FOLDER, PARTY, etc.
- version containers, such as VERSIONED_COMPOSITION, VERSIONED_EHR_STATUS, etc. or their containing VERSION
- other non-versioned resources, such as EHR, CONTRIBUTION, RESULT_SET, etc.
- definitions, such as TEMPLATE, ARCHETYPE, QUERY

Types defined by openEHR specification are always capitalized when used in this specification.
For a list of all available types or a particular type definition please refer to the [class index](https://specifications.openehr.org/classes).

# Resource identification

An important aspect of a RESTful service is to be able to address resources via URIs/URLs using HTTP verbs (methods).
Each resource has an identifier which is unique within the system for that resource type, usually consisting of at least a globally unique identifier and sometimes associated also with a version identifier.
Once the resource is persisted by the service, the identifier is assigned and will never change for that resource instance.

Depending on the resource type and on the use-cases, there are several identifier types (and formats):

- a `versioned_object_uid` for identifying a VERSIONED_OBJECT (i.e. a version container), stored under VERSIONED_OBJECT.uid.value, in a form of a HIER_OBJECT_ID,
- a `version_uid` for identifying a VERSION from the version container, stored under VERSION.uid.value, in a form of an OBJECT_VERSION_ID,
- an `ehr_id` to identify an EHR,
- a `template_id` to identify a Template definition.
- a `qualified_query_name` to identify Query definition.

The `version_uid` is the unique identifier of a VERSION, in the lexical form of `object_id :: creating_system_id :: version_tree_id`,
where the `object_id` has the same value as the containing VERSIONED_OBJECT identifier taken from VERSIONED_OBJECT.uid.value (i.e. the `versioned_object_uid`).

As examples of identifiers, the following request:

```http
GET https://openEHRSys.example.com/v1/ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1
```

is using an URI that contains an EHR identifier `7d44b88c-4199-4bad-97dc-d78268e01398`, taken from EHR.ehr_id.value,
and a COMPOSITION identifier `8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::1`, taken from COMPOSITION.uid.value,
serviced by an openEHR API (of version `v1`), located at `https://openEHRSys.example.com`.

There are circumstances when resources can be uniquely identified by two different identifiers. For example, assuming that a VERSIONED_COMPOSITION container has the _versioned_object_uid_ identifier `8849182c-82ad-4088-a07f-48ead4180515`, and the latest version is `5` (taken from _version_tree_id_), then that COMPOSITION can be identified either by following explicit URI:

```http
https://openEHRSys.example.com/v1/ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com:5
```

using the _version_uid_ identifier, as well as by the following implicit URI:

```http
https://openEHRSys.example.com/v1/ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/8849182c-82ad-4088-a07f-48ead4180515
```

However, the latter will only reference the same resource as the former as long as there are no more changes or modifications on that specific version container (i.e. the latest version remains `5`).

- **NOTE:** Since [RM Release 1.0.4](https://specifications.openehr.org/releases/RM/Release-1.0.4) it is strongly recommended that the inherited attribute `uid` be populated in COMPOSITIONs, using the UID copied from the object_id() of the `uid` field of the enclosing VERSION object.
  For example, the ORIGINAL_VERSION.uid `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2` would be copied to the `uid` field of the COMPOSITION.
  See [Levels of identification](https://specifications.openehr.org/releases/BASE/latest/architecture_overview.html#\_levels_of_identification) chapter of Architecture Overview for more details.

# Data representation

Services MUST support at least one of the openEHR **XML** or **JSON** formats for resource representation.
Alternative formats MAY be also supported, such as a **Simplified Data Template (SDT)** format.
These formats and their associated negotiation protocol is described below.

## XML Format

When resources representation is serialized as XML, the request payload as well as the result MUST be valid against [published XSDs](https://specifications.openehr.org/releases/ITS-XML/latest).

A client MAY use the header `Content-Type: application/xml` in the requests to specify the XML payload format.
If the service cannot process the request payload as XML format is not supported, it MUST respond with HTTP status code `415 Unsupported Media Type`.

The client SHOULD use the `Accept: application/xml` request header in order to specify the expected XML response format.
If the service cannot fulfill this aspect of the request, it MUST respond with HTTP status code `406 Not Acceptable`.
Proper header `Content-Type: application/xml` MUST be present in the response of the service
unless the response has no content body (HTTP status code `204`).

## JSON Format

When resources representation is serialized as JSON, the request payload as well as the result SHOULD be valid against [published JSON-Schemas](https://specifications.openehr.org/releases/ITS-JSON/latest).

> Note: The JSON-Schema project is under development.

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

One example is the `_type` attribute, which should be used to specify the RM type whenever polymorphism
is involved, or when the underlying definition in RM type is abstract
(dynamic type is different from the static type).
This follows same rule as for XML typing.
The value of this attribute MUST be the uppercase class name from the RM specification. For example:

```json
{
    "_type": "DV_TEXT",
    "value": "Hello world!"
}
```

The RM attributes (even required ones) that are `Null`, empty list or empty arrays SHOULD be absent when serialized as JSON.

The order of attributes in the resource MAY follow the order of attributes in the RM specification of the type of the resource, but this is not mandatory.

A client MAY use the header `Content-Type: application/json` in the requests to specify the JSON payload format.
If the service cannot process the request payload as JSON format is not supported, it MUST respond with HTTP status code `415 Unsupported Media Type`.

The client SHOULD use the `Accept: application/json` request header in order to specify the expected JSON response format.
If the service cannot fulfill this aspect of the request, it MUST respond with HTTP status code `406 Not Acceptable`.
Proper header `Content-Type: application/json` MUST be present in the response of the service
unless the response has no content body (HTTP status code `204`).

## Alternative data formats

Creating data instances according to canonical XML or JSON format is not always straightforward,
particularly for developers with minimal exposure to openEHR, and various alternatives
have been used in the past to simplify the job of content creation and committal for application developers.

There is an initiative exploring and documenting these alternative formats, which specifications can be consulted at
[Simplified Data Template (SDT)](simplified_data_template.html) page.

> Note: The specification of Simplified Data Template is under development.

In order to use these formats, content negotiation SHOULD be done in the same manner as for the canonical XML or JSON format above,
but instead of `application/xml` or `application/json`, clients and servers MUST use:

- `application/openehr.wt.flat+json` for the simplified IM Simplified Data Template (simSDT) as JSON,

based on the 'FLAT' version of the ['web template' format](https://www.ehrscape.com/reference.html), originally created by Marand for the Better platform.
More information can be found also in their ['examples'](https://www.ehrscape.com/examples.html) page, as well as in their [open-source implementation](https://github.com/better-care/web-template)
and [conformance tests](https://github.com/better-care/web-template-tests).

```
EHRbase also has support for this format, and provides information in their [documentation](https://ehrbase.readthedocs.io/en/latest/02_getting_started/05_load_data/index.html#flat-format). 
```

- `application/openehr.wt.structured+json` for the structured IM Simplified Data Template (structSDT) as JSON,

based on the 'STRUCTURED' version of the ['web template' format](https://www.ehrscape.com/reference.html) originally created by Marand for the Better platform
(see also their ['examples'](https://www.ehrscape.com/examples.html)).

- `text/plain` for all textual ADL2 templates or AQL queries,
- `application/openehr.nc.flat+json` for near-canonical RM Simplified Data Template (ncSDT) as JSON, based on the

[ECISFLAT format](https://github.com/ethercis/ethercis/blob/master/doc/flat%20json.md), originally devised for the EtherCIS project,

- `application/openehr.tds2+xml` for TDS simplification of RM as XML,

based on the 'TDS/TDD' format originally created by [Ocean Health Systems](http://oceanhealthsystems.com/)
(see [this wiki page](https://openehr.atlassian.net/wiki/spaces/spec/pages/30408770/Template+Data+Schema+TDS+Specification+and+associated+Template+Data+Document+TDD)
and the [TDD2canonical](https://github.com/openEHR/openEHR-TDD2canonical) project).

> Note: Other alternative formats may be added in the future, depending on innovative impact, simplicity, popular demand or adoption rate.

Current alternative formats might not be supported once they become obsolete or superseded by newer formats.

A client MAY use the header `Content-Type` in the requests to specify the simplified payload format.
If the service cannot process the request payload as the simplified format is not supported, it MUST respond with HTTP status code `415 Unsupported Media Type`.

The client SHOULD use the `Accept` request header in order to specify the expected simplified response format.
If the service cannot fulfill this aspect of the request, it MUST respond with HTTP status code `406 Not Acceptable`.
Proper header `Content-Type` MUST be present in the response of the service
unless the response has no content body (HTTP status code `204`).

## Datetime format

The format of the date, time and datetime types should comply with the [ISO 8601 Date and Time](https://en.wikipedia.org/wiki/ISO\_8601) format specification.
The ISO 8601 semantics assumed by openEHR is defined in the classes of the [base.foundation_types.time package](https://specifications.openehr.org/releases/BASE/latest/foundation_types.html#\_time_types).
It is strongly recommended using the _extended_ format for dates, times and datetimes.
This greatly aids human readability, and reduces the need for special date/time parsing and formatting.

HTTP query parameters and path segments that are dates, datetimes, or times, MUST always use the _extended_ ISO 8601 format.
The general form of a datetime is `YYYY-MM-DDThh:mm:ss.sss[Z|±hh:mm]`, e.g. `2016-06-23T13:42:16.117+02:00`.\
Timezone SHOULD be only supplied when needed, otherwise the local timezone is assumed.

Any date, datetime or time value provided as part of the HTTP message body, when creating or updating a resource (e.g. a DV_DATE_TIME value inside the COMPOSITION content),
will be preserved as it was sent by the client, and passed to the underlying backend engine as is.
Retrieval or querying those resources SHOULD return date, datetime, or time values in the (original) format provided by underlying backend engine, avoiding any format change.
