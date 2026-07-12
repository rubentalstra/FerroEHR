[comment]: # (title: Requests and Responses)

The following describes in details how HTTP [headers](#tag/Requests_and_responses/HTTP-headers) and [status codes](#tag/Requests_and_responses/HTTP-status-codes) MUST be used by an openEHR REST API implementation to ensure effective interaction between services and clients in accordance with this specification.

Additionally, general details about resources, [data representation](#tag/Resources/Data-representation) and content negotiation are described in this specification.

> NOTE: Examples primarily use the **JSON** format. However, this does not imply that **XML** is unsupported by the same service endpoint.


# HTTP Methods

HTTP Methods are described by [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#name-overview).
The following subset is used in this specification:

| Method  | Description                                                                              |
|---------|------------------------------------------------------------------------------------------|
| GET     | Transfer a current representation of the target resource.                                |
| HEAD    | Check existence of target resource and return the status, but does not transfer content. |
| POST    | Perform resource-specific processing on the request payload.                             |
| PUT     | Replace all current representations of the target resource with the request payload.     |
| DELETE  | Remove all current representations of the target resource.                               |
| OPTIONS | Describe the communication options for the target resource.                              |

A server receiving an unrecognized or unimplemented method SHOULD respond with the `501 Not Implemented` status code.
If a method is recognized but not allowed for the target resource, the response SHOULD be `405 Method Not Allowed` status code.


# Authentication and authorization

Services SHOULD implement and support an HTTP Authentication and Authorization framework, though this specification does not mandate a specific authentication scheme.
See [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#name-http-authentication) or [Mozilla's HTTP Authentication](https://developer.mozilla.org/en-US/docs/Web/HTTP/Authentication)
for details on implementing an authentication and authorization.

If authentication and authorization are required, services MUST properly use the `WWW-Authenticate` and/or `Proxy-Authenticate` response headers, returning `403 Forbidden`, `401 Unauthorized` or `407 Proxy Authentication` as applicable.
Clients MUST send valid `Authorization` and `Proxy-Authorization` headers in their requests when required.


# HTTP headers

Standard HTTP request and response headers are defined by [RFC 9110](https://tools.ietf.org/html/rfc9110) and the [IANA Message Headers Registry](http://www.iana.org/assignments/message-headers/message-headers.xhtml).

This section outlines the subset of these headers used in the openEHR API, along with custom headers specific to openEHR.

## Deprecated headers

[Prior to Release 1.1.0](https://specifications.openehr.org/releases/ITS-REST/Release-1.0.3/overview.html#tag/Requests_and_responses/HTTP-headers), some openEHR custom headers included special characters that caused compatibility issues with HTTP standards. To ensure compliance and improve interoperability, these headers have been deprecated.

While the deprecated headers remain available for backward compatibility, their use is strongly discouraged. The table below lists the updated header names alongside their deprecated counterparts:

| Deprecated header     | New header            |
|-----------------------|-----------------------|
| openEHR-VERSION       | openehr-version       |
| openEHR-AUDIT_DETAILS | openehr-audit-details |
| openEHR-TEMPLATE_ID   | openehr-template-id   |
| openEHR-uri           | openehr-uri           |
| openEHR-EHR-id        | openehr-ehr-id        |

Some of the `GET` methods had a `Location` response header to indicate the canonical location of a representation. 
However, this was an incorrect use of the header, and it is now deprecated. 
Similarly, the `Location` response header was deprecated from responses of `DELETE` methods.
For more information see [Location header](#tag/Requests_and_responses/HTTP-headers/Location) section.

The `ETag` response header was used without a weakness indicator `W/`. 
This is now deprecated, all `ETag` headers that hold a resource identifier MUST include a weakness indicator `W/`.
For more information see [ETag and Last-Modified](#tag/Requests_and_responses/HTTP-headers/ETag-and-Last-Modified) section.

To ensure forward compatibility, clients are strongly encouraged to always include the `Prefer` request header explicitly. Although the current default behavior is equivalent to `Prefer=minimal`, this might change in the near future to `Prefer=identifier`. Explicitly stating the preference prevents unexpected behavior changes as the default evolves.
For more information see [Prefer only identifier](#tag/Requests_and_responses/Representation-details-negotiation/Prefer-only-identifier) section.

For long-term stability, all new implementations should adopt the updated header names and follow their prescribed usage.

## openehr-version and openehr-audit-details

When it comes to committing content to an openEHR system, for all change-controlled resources (e.g. COMPOSITION, EHR_STATUS, FOLDER, etc.) the services are [performing versioning](https://specifications.openehr.org/releases/RM/latest/common.html#\_change_control_package) under the hood.
The 'native' way of committing is to use a [CONTRIBUTION](https://specifications.openehr.org/releases/RM/latest/common.html#\_contributions) and wrap the content as a [VERSION](https://specifications.openehr.org/releases/RM/latest/common.html#\_version_class).
To keep things simpler and consistent, services MUST also allow `PUT`, `POST` and `DELETE` methods directly on these change-controlled resources.
However, these operations MUST internally be executed using the 'native' way.

In order to allow clients to provide committal metadata, services MUST accept `openehr-version` and `openehr-audit-details` custom request headers.
For clients, it is RECOMMENDED to provision these headers based on [authentication and authorization](#tag/Requests_and_responses/Authentication-and-authorization) runtime data.
Through the `openehr-audit-details` header, clients MAY supply values for the AUDIT_DETAILS attributes `change_type`, `description`, `committer` and `system_id`.
The `time_committed` attribute is always set by the server.
Below is a complex example of these request headers used in a `PUT` action to update a COMPOSITION:

```http
openehr-version: lifecycle_state.code_string="532"
openehr-audit-details: change_type.code_string="251"
openehr-audit-details: description.value="An updated composition contribution description"
openehr-audit-details: committer.name="John Doe",committer.external_ref.id="BC8132EA-8F4A-11E7-BB31-BE2E44B06B34",committer.external_ref.namespace="demographic",committer.external_ref.type="PERSON"
openehr-audit-details: system_id="example.openehr.systemid"
```

None of these headers are mandatory, but whatever is provided it MUST be merged with the default VERSION and VERSION.audit_details attributes on commit runtime.
In particular, when `system_id` is not provided by the client, the server MUST set it to its own configured system identifier.

The list of `code_string` values and their meaning is specified by the [Audit Change Type](https://specifications.openehr.org/releases/TERM/latest/SupportTerminology.html#_audit_change_type) and [Version Lifecycle State](https://specifications.openehr.org/releases/TERM/latest/SupportTerminology.html#_version_lifecycle_state) openEHR terminology vocabularies.

## openehr-item-tag and openehr-version-item-tag

The `openehr-item-tag` and `openehr-version-item-tag` headers provide a lightweight mechanism for associating tags with change-controlled resources (e.g. COMPOSITION, EHR_STATUS, FOLDER, etc.).

- `openehr-item-tag` applies to *VERSIONED_OBJECT* targets.
- `openehr-version-item-tag` applies to a specific target *VERSION* within a VERSIONED_OBJECT.

Below is an example of these headers:
```http
openehr-item-tag: key="category",value="final"
openehr-version-item-tag: key="reviewed",value="true"; key="flag",value="follow-up",target_path="/composition/start_time/value"
```

Both headers act as convenient wrappers around the dedicated ITEM_TAG operations (e.g. retrieving [COMPOSITION tags](ehr.html#tag/COMPOSITION/operation/composition_tags_get)), simplifying client interaction and reducing the number of API calls.

Tag values are provided as a list of key-value pairs, separated by semicolons. They both support the same syntax and schema, having the following attributes: `key`, `value` and `target_path`.
More than one ITEM_TAG may be associated with a single target, in which case they are uniquely identified by their `key` and `target_path` pair attributes.

If the server does not support ITEM_TAGs, these headers will also be unsupported.

#### Usage in Requests
When used as request headers for creation or update operations (`PUT`, `POST`), the header instructs the system which ITEM_TAG list should be associated with the target VERSION or VERSIONED_OBJECT resource (depending on the header name).

Providing an empty value for this header will effectively remove all ITEM_TAGs associated with the given target.

#### Usage in Responses
Servers MAY include the `openehr-item-tag` or `openehr-version-item-tag` header in responses to confirm the actual list of ITEM_TAGs stored on the server side.

When retrieving resources via `GET`, the server MAY also add these headers to the response, containing the list of ITEM_TAGs associated with the target VERSION or VERSIONED_OBJECT (depending on the header name).

## openehr-template-id

The `openehr-template-id` request header MUST be used whenever committing COMPOSITION (via `PUT` or `POST` methods) using a [Simplified Format](#header-alternative-data-formats) which does not support TEMPLATE_ID value under an equivalent `LOCATABLE.archetype_details.template_id` attribute of contained data.

## Location

The `Location` header, as defined in [RFC 9110, Section 10.2.2](https://datatracker.ietf.org/doc/html/rfc9110#field.location), is used by a server to indicate the URL of a newly created resource.
In the openEHR REST API, it is used in `201 Created` responses when a new resource is successfully created (e.g., via `POST` method). 
It MUST NOT be used to indicate an alternate representation of an existing resource (e.g. via `GET` method).

> DEPRECATION: [Prior to Release 1.1.0](https://specifications.openehr.org/releases/ITS-REST/Release-1.0.3/overview.html#tag/Requests_and_responses/HTTP-headers), the `Location` header was used to indicate the canonical location of a representation in a response.
> This usage is now deprecated. 
> The `Location` header MUST ONLY be used for resource creation (e.g., `201 Created`) or redirect responses.

Example of a `Location` response header:

```http
HTTP/1.1 201 Created
Location: https://openEHRSys.example.com/v1/ehr/347a5490-55ee-4da9-b91a-9bba710f730e/composition/8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2
Preference-Applied: return=minimal
```

## openehr-uri

If the service supports generating resource URIs in the format defined by the [DV_EHR_URI](https://specifications.openehr.org/releases/RM/development/data_types.html#_dv_ehr_uri_class) class, it MAY include the openehr-uri response header for applicable openEHR resources.

Example:

```http
HTTP/1.1 200 OK
openehr-uri: ehr:/347a5490-55ee-4da9-b91a-9bba710f730e/compositions/87284370-2D4B-4e3d-A3F3-F303D2F4F34B
```

## Prefer

The `Prefer` request header MAY be used by clients to negotiate the format or content of the resource representation in the response.
For more information, see the [representation details negotiation](#tag/Requests_and_responses/Representation-details-negotiation) section.

## ETag and Last-Modified

The `ETag` and `Last-Modified` headers provide essential information about the state of a resource, enabling clients to manage caching, detect changes, and prevent unintentional overwrites.

The `ETag` (Entity Tag) header acts as a unique identifier for a specific version of a resource. It helps clients determine whether a resource has changed between requests, supporting efficient caching and optimistic concurrency control.

In this specification, the `ETag` value is independent of its resource serialization format (JSON/XML). This differs from standard HTTP behavior, where an `ETag` typically identifies a specific representation of a resource - see [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#field.etag). Therefore, the `ETag` is considered to be of weak-type and should have a weakness indicator `W/` prefix.

> DEPRECATION: [Prior to Release 1.1.0](https://specifications.openehr.org/releases/ITS-REST/Release-1.0.3/overview.html#tag/Requests_and_responses/HTTP-headers), the `ETag` header was used without a weakness indicator `W/`. This usage is now deprecated, but implementations MAY still support it alongside the updated header format that includes the weakness indicator `W/` prefix.

The `ETag` value is usually taken from e.g. VERSIONED_OBJECT.uid.value, VERSION.uid.value, EHR.ehr_id.value, etc. It changes as soon as the resource changes (i.e. when a new version is created).

An example of `ETag` header value format is:

```http
HTTP/1.1 200 OK
ETag: W/"8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2"
```

Servers MAY add additional `ETag` response headers, consisting of an opaque quoted string, possibly prefixed by a weakness indicator.

The `Last-Modified` response HTTP header, indicates the date and time when the resource was last modified.
This helps clients determine freshness and manage conditional requests.
For openEHR resources, this value should be derived from VERSION.commit_audit.time_committed.value.

An example of `Last-Modified` header value format is:

```http
HTTP/1.1 200 OK
Last-Modified: Wed, 22 Jul 2009 19:15:56 GMT
```

Both `ETag` and `Last-Modified` SHOULD be included in responses for VERSION, VERSIONED_OBJECT, or other resources that have versioning or unique state identifiers.

## If-Match and accidental overwrites

The `If-Match` request header SHOULD be used by the clients with state-changing methods (e.g., `PUT`) to prevent accidental overwrites when multiple user agents might be acting in parallel on the same resource (i.e., to prevent the "lost update" problem). This is only required by a small set of versioned resources in this specification, when the `preceding_version_uid` is not part of the endpoint path segment.
If a service receives this header, and the condition evaluates to `false`, it MUST NOT perform the requested method. Instead, it MUST respond with HTTP status code `412 Precondition Failed`, and SHOULD return also latest `version_uid` in the `ETag` response headers. When the service expects `If-Match` for an operation, but the client does not provide it, the service SHOULD respond with `400 Bad Request`. 

Example:
```http
If-Match: "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2"
```

See also details for `If-Match` described by [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#name-if-match).


# HTTP status codes

HTTP Status codes are described by [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#name-status-codes) and by the [IANA Status Code Registry](https://www.iana.org/assignments/http-status-codes/http-status-codes.xhtml).
The following subset is used in this specification:

| Code | Reason-Phrase          | Meaning, use case and details                                                                                                                                              |
| ---- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 200  | OK                     | The request succeeded, payload sent in a 200 response depends on the request method                                                                                        |
| 201  | Created                | The request has been fulfilled and has resulted in one or more new resources being created                                                                                 |
| 204  | No Content             | The request has been fulfilled and there is no additional content to send in the response payload body                                                                     |
| 400  | Bad Request            | The service cannot or will not process the request due to something that is perceived to be a client error (e.g., malformed request syntax, syntactically invalid content) |
| 401  | Unauthorized           | If the service requires authorization, this indicates that the request has not been applied because it lacks valid authentication credentials for the target resource      |
| 403  | Forbidden              | The service understood the request but refuses to authorize it                                                                                                             |
| 404  | Not Found              | The origin service did not find the target resource or is not willing to disclose that one exists                                                                          |
| 405  | Method Not Allowed     | The method received in the request-line is known by the origin service but not supported by the target resource                                                            |
| 406  | Not Acceptable         | The target resource does not have a current representation that would be acceptable to the user                                                                            |
| 408  | Request Timeout        | Request maximum execution time is reached, therefore the server aborted the request                                                                                        |
| 409  | Conflict               | Indicates that the request could not be processed because it might generate a duplicate or a conflict                                                                      |
| 412  | Precondition Failed    | One or more conditions given in the request header fields evaluated to false when tested on the server                                                                     |
| 415  | Unsupported Media Type | The service is refusing the request because the payload is in a format not supported by this method on the target resource                                                 |
| 422  | Unprocessable Entity   | The request was well-formed but was unable to be followed due to semantic errors                                                                                           |
| 500  | Internal Server Error  | The service encountered an unexpected condition that prevented it from fulfilling the request                                                                              |
| 501  | Not Implemented        | The service does not support the functionality required to fulfill the request                                                                                             |

To indicate the status of a request or operation, an appropriate HTTP status code MUST be used as described above. Additional status codes MAY be used as long as they do not conflict with the predefined codes.

Status code `400` indicates normally a bad request, as well as a generic client-side error, used when no other `4xx` error code is appropriate.
The client SHOULD NOT repeat the request without modifications.

For `4xx` and `5xx` status codes, services MAY return additional error details if the `Prefer: return=representation` header is present in the request.
Example error response:

```json
{
    "message": "Error message",
    "code": 90000,
    "errors": [
        {
            "_type": "DV_CODED_TEXT",
            "value": "Error message",
            "defining_code": {
                "terminology_id": {
                    "value": "local"
                },
                "code_string": "9000"
            }
        },
        {
            "_type": "DV_CODED_TEXT",
            "value": "Secondary error message",
            "defining_code": {
                "terminology_id": {
                    "value": "local"
                },
                "code_string": "8000"
            }
        }
    ]
}
```

# Representation details negotiation

When using `POST`, `PUT`, or `DELETE` to create, update, or delete a resource, the client can specify its preferred response format using the `Prefer` header, as defined in [RFC 7240, Section 4.2](https://tools.ietf.org/html/rfc7240#section-4.2).

The service MAY include a `Preference-Applied` header in the response, such as `Preference-Applied: return=minimal` or `Preference-Applied: return=representation`, to indicate that the client's preference has been honored.

If no `Prefer` header is provided, the default behavior is assumed to be `return=minimal`.

## Prefer minimal, identifier or full representation response

Clients MAY use the following preferences to control the verbosity of successful responses:

* `Prefer: return=minimal`  
  Indicates the client prefers a minimal response. 
  The response SHOULD include a `Location` header pointing to the newly created or updated resource. The HTTP status is typically `201 Created`. If no response body is returned, the service SHOULD use `204 No Content`.

* `Prefer: return=identifier`  
  Indicates the client prefers a minimal response that includes only the identifier (e.g., the `uid`) of the affected resource. 
  The response MAY include a `Location` header pointing to the newly created or updated resource. The HTTP status is typically `201 Created` or `200 OK` and the response body SHOULD contain only the identifier of the affected resource.

* `Prefer: return=representation`  
  Indicates the client prefers a full resource representation in the response.  
  The response MAY include a `Location` header, and the response body SHOULD contain the full representation of the resource. The HTTP status is typically `201 Created` or `200 OK`.


## Prefer only identifier 

Clients MAY request a minimal response that includes only the identifier (e.g., the `uid`) of the affected resource by specifying:

```http
PUT https://openEHRSys.example.com/v1/ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2
Prefer: return=identifier
Accept: application/json
```

This is a variant of preference that implies minimal response semantics, but with a non-empty response body (i.e., the status will be `201 Created` or `200 OK`, never `204 No Content`).
The response body SHOULD contain only the identifier of the affected resource formatted according to the `Accept` request header.
For example, when `application/json` is requested as above, the response body will be a single JSON object with a single `uid` attribute.

```http
HTTP/1.1 200 OK
Preference-Applied: return=identifier
Content-Type: application/json
{ 
  "uid": "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::3"
}
```

> **Notice:** Although the current default behavior is `return=minimal`, some server implementations MAY provide a configurable flag to switch the default response preference to `return=identifier`. To ensure forward compatibility, clients are encouraged to always include the `Prefer` request header explicitly. Stating the preference explicitly prevents unexpected behavior changes as the default behavior evolves.

## Prefer resolving Object references

Clients MAY request that object references (e.g., OBJECT_REF) be resolved into full or partial representations by specifying:

```http
Prefer: return=representation, resolve_refs
```

This is particularly useful when retrieving lists of COMPOSITION resources within an EHR, where the default behavior is to return only reference links.
