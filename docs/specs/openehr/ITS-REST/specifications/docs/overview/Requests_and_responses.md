[comment]: # (title: Requests and Responses)

The following describes in details how HTTP [headers](#tag/Requests_and_responses/HTTP-headers) and [status codes](#tag/Requests_and_responses/HTTP-status-codes)
MUST be used by an openEHR REST API implementation in order to achieve good interaction between services and clients in the spirit of this specification.

Furthermore, general details about resources, [data representation](#tag/Resources/Data-representation) and content negotiation are described in this specification.

> NOTE: **JSON** format is used in many of the example below, but this does not imply that **XML** might not be supported by the same service endpoint.


# HTTP Methods

HTTP Methods are described by [RFC 7231](https://tools.ietf.org/html/rfc7231#section-4)
and by [IANA HTTP Method Registry](https://www.iana.org/assignments/http-methods/http-methods.xhtml).
The following subset is being used in this specification:

| Method  | Description                                                                              |
| ------- | ---------------------------------------------------------------------------------------- |
| GET     | Transfer a current representation of the target resource.                                |
| HEAD    | Check existence of target resource and return the status, but does not transfer content. |
| POST    | Perform resource-specific processing on the request payload.                             |
| PUT     | Replace all current representations of the target resource with the request payload.     |
| DELETE  | Remove all current representations of the target resource.                               |
| OPTIONS | Describe the communication options for the target resource.                              |


# Authentication and authorization

Services SHOULD implement and support an HTTP Authentication and Authorization framework
(which can support various schemes) but there is no assumption or recommendation being made
in this specification about which authentication scheme should be used by services and clients.
See [RFC 7235](https://tools.ietf.org/html/rfc7235) or [Mozilla's HTTP Authentication](https://developer.mozilla.org/en-US/docs/Web/HTTP/Authentication)
for details on this subject.

If an Authentication and Authorisation framework is present, services MUST properly use `WWW-Authenticate` and/or
`Proxy-Authenticate` response headers and return HTTP status code `403 Forbidden` or `401 Unauthorized` or
`407 Proxy Authentication` whenever applicable, and clients MUST properly use `Authorization` and `Proxy-Authorization`
in their request headers.


# HTTP headers

Standard HTTP Request and Response headers are described by [RFC 7231](https://tools.ietf.org/html/rfc7231)
and by the [IANA Message Headers](http://www.iana.org/assignments/message-headers/message-headers.xhtml).
The following describes the use of a subset of these headers, as well as the custom headers used by an openEHR API.

## openEHR-VERSION and openEHR-AUDIT_DETAILS

When it comes to committing content to an openEHR system, for all change-controlled resources (e.g. COMPOSITION, EHR_STATUS, FOLDER, etc.) the services are
[performing versioning](https://specifications.openehr.org/releases/RM/latest/common.html#\_change_control_package) under the hood.
The 'native' way of committing is to use a [CONTRIBUTION](https://specifications.openehr.org/releases/RM/latest/common.html#\_contributions)
and wrap the content as a [VERSION](https://specifications.openehr.org/releases/RM/latest/common.html#\_version_class).
To keep things simpler and consistent, services MUST also allow `PUT`, `POST` and `DELETE` methods directly on these change-controlled resources.
However, these operations MUST internally be executed using the 'native' way.

In order to allow clients to provide committal metadata, services MUST accept `openEHR-VERSION` and `openEHR-AUDIT_DETAILS` custom request headers.
For clients, it is RECOMMENDED to provision these headers based on [authentication and authorization](#tag/Requests_and_responses/Authentication-and-authorization) runtime data.
Below is a complex example of these request headers used in a `PUT` action to update a COMPOSITION:

```http
openEHR-VERSION.lifecycle_state: code_string="532"
openEHR-AUDIT_DETAILS.change_type: code_string="251"
openEHR-AUDIT_DETAILS.description: value="An updated composition contribution description"
openEHR-AUDIT_DETAILS.committer: name="John Doe", external_ref.id="BC8132EA-8F4A-11E7-BB31-BE2E44B06B34", external_ref.namespace="demographic", external_ref.type="PERSON"
```

None of these headers are mandatory, but whatever is provided it MUST be merged with the default VERSION and VERSION.audit_details attributes on commit runtime.

Below a list of `code_string` values and their meaning (taken from [openEHR terminology](https://raw.githubusercontent.com/openEHR/terminology/Release-2.0.0/openEHR_RM/en/openehr_terminology.xml)):

| header                            | code | value / meaning |
| --------------------------------- | ---- | --------------- |
| openEHR-VERSION.lifecycle_state   | 532  | complete        |
| openEHR-VERSION.lifecycle_state   | 553  | incomplete      |
| openEHR-VERSION.lifecycle_state   | 523  | deleted         |
| openEHR-AUDIT_DETAILS.change_type | 249  | creation        |
| openEHR-AUDIT_DETAILS.change_type | 250  | amendment       |
| openEHR-AUDIT_DETAILS.change_type | 251  | modification    |
| openEHR-AUDIT_DETAILS.change_type | 252  | synthesis       |
| openEHR-AUDIT_DETAILS.change_type | 523  | deleted         |
| openEHR-AUDIT_DETAILS.change_type | 666  | attestation     |
| openEHR-AUDIT_DETAILS.change_type | 253  | unknown         |

## If-Match and accidental overwrites

The `If-Match` request header SHOULD be used by the clients to prevent accidental overwrites when multiple user
agents might be acting in parallel on the same resource. This is only required by a small set of versioned resources of this specification.
In case a service receives this header, and the condition evaluates to `false`, it MUST NOT perform the requested method and instead MUST respond with
HTTP status code `412 Precondition Failed`, and SHOULD return also latest `version_uid` in the `Location` and `ETag` response headers.

Example:
```http
If-Match: "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2"
```

See also details for `If-Match` described by [RFC 7232](https://tools.ietf.org/html/rfc7232#section-3.1).

## openEHR-TEMPLATE_ID

The `openEHR-TEMPLATE_ID` request header MUST be used whenever committing COMPOSITION (via `PUT` or `POST` methods)
using a [simplified data format](#header-alternative-data-formats) which does not support TEMPLATE_ID value
under an equivalent `LOCATABLE.archetype_details.template_id` attribute of contained data.

## Location and openEHR-uri

The `Location` response header indicates the resource location (URL).
According to [RFC 7231](https://tools.ietf.org/html/rfc7231#section-7.1.2), it is used to refer to a specific resource in relation to the response.
The type of relationship is defined by the combination of request method and status code semantics.
The identifiers part of this URI-reference SHOULD comply with [resource identification](#tag/Resources/Resource-identification) semantics of this specification.

Services MUST return this header whenever a create or update operation was performed, but it MAY
return this header on other operation or action. Example:

```http
Location: https://openEHRSys.example.com/v1/ehr/347a5490-55ee-4da9-b91a-9bba710f730e/composition/8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2
```

See [representation details negotiation](#tag/Requests_and_responses/Representation-details-negotiation) section
for more details on how use this header.

If services have support to generate resource URL as specified by the DV_URI/DV_EHR_URI format, then they MAY send also `openEHR-uri` response header. Example:

```http
openEHR-uri: ehr:/347a5490-55ee-4da9-b91a-9bba710f730e/compositions/87284370-2D4B-4e3d-A3F3-F303D2F4F34B
```

## Prefer

The `Prefer` request header MAY be used by clients for resource representation negotiation.
See more details on [representation details negotiation](#tag/Requests_and_responses/Representation-details-negotiation) section.

## ETag and Last-Modified

These headers are mainly related to whether or not a returned resource may be cacheable.
Their purpose is to give clients information about the state of the requested resources.

According to [RFC 7232](https://tools.ietf.org/html/rfc7232#section-2.3),

> The "ETag" header field in a response provides the current entity-tag
for the selected representation, as determined at the conclusion of
handling the request.  An entity-tag is an opaque validator for
differentiating between multiple representations of the same
resource, regardless of whether those multiple representations are
due to resource state changes over time, content negotiation
resulting in multiple representations being valid at the same time,
or both.

The `ETag` response HTTP header contains a string token that the server associates with a resource in order to
uniquely identify the state of that resource over its lifetime. The value of the token changes as soon as the resource changes.
An example of such header value format is:

```http
ETag: "8849182c-82ad-4088-a07f-48ead4180515::openEHRSys.example.com::2"
```

Servers MAY choose their own format for this header, but the recommended value is the unique identifier of the requested resource
(e.g. VERSIONED_OBJECT.uid.value, VERSION.uid.value, EHR.ehr_id.value, etc).

The `Last-Modified` response HTTP header contains the datetime of the last modification of targeted resource
which should be taken from VERSION.commit_audit.time_committed.value.
An example of such header value format is:

```http
Last-Modified: Wed, 22 Jul 2009 19:15:56 GMT
```

These two headers SHOULD be present in all responses targeting VERSION, VERSIONED_OBJECT or other resources
that have similar unique identifier.


# HTTP status codes 

HTTP Status codes are described by [RFC 7231](https://tools.ietf.org/html/rfc7231#section-6)
and by the [IANA Status Code Registry](https://www.iana.org/assignments/http-status-codes/http-status-codes.xhtml).
The following subset of the is being used in this specification:

| Code | Reason-Phrase          | Meaning, usecase and details                                                                                                                                               |
| ---- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 200  | OK                     | The request succeeded, payload sent in a 200 response depends on the request method                                                                                        |
| 201  | Created                | The request has been fulfilled and has resulted in one or more new resources being created                                                                                 |
| 204  | No content             | The request has been fulfilled and there is no additional content to send in the response payload body                                                                     |
| 400  | Bad request            | The service cannot or will not process the request due to something that is perceived to be a client error (e.g., malformed request syntax, syntactically invalid content) |
| 401  | Unauthorized           | If the service requires authorization, this indicates that the request has not been applied because it lacks valid authentication credentials for the target resource      |
| 403  | Forbidden              | The service understood the request but refuses to authorize it                                                                                                             |
| 404  | Not found              | The origin service did not find the target resource or is not willing to disclose that one exists                                                                          |
| 405  | Method Not Allowed     | The method received in the request-line is known by the origin service but not supported by the target resource                                                            |
| 406  | Not Acceptable         | The target resource does not have a current representation that would be acceptable to the user                                                                            |
| 408  | Request Timeout        | Request maximum execution time is reached, therefore the server aborted the request                                                                                        |
| 409  | Conflict               | Indicates that the request could not be processed because it might generate a duplicate or a conflict                                                                      |
| 412  | Precondition Failed    | One or more conditions given in the request header fields evaluated to false when tested on the server                                                                     |
| 415  | Unsupported Media Type | The service is refusing the request because the payload is in a format not supported by this method on the target resource                                                 |
| 422  | Unprocessable Entity   | The request was well-formed but was unable to be followed due to semantic errors                                                                                           |
| 500  | Internal Server Error  | The service encountered an unexpected condition that prevented it from fulfilling the request                                                                              |
| 501  | Not Implemented        | The service does not support the functionality required to fulfill the request                                                                                             |

To indicate the status of the request, or the executed operation, one of the HTTP status codes MUST be used, as described [above](#header-glossary-and-conventions).
Meaning of these codes may be further detailed (nuanced) in this specification by particular responses.
If required, other status codes MAY be used by implementations as long as their usecase is not conflicting or overlapping with above codes.

Status code `400` indicates a bad request or generic client-side error, used when no other `4xx` error code is appropriate.
The client SHOULD NOT repeat the request without modifications.

In case of errors (HTTP status codes `400`-`500`), the services MAY return more details (if `Prefer: return=representation` request header is present).
Example:

> some codes/messages <https://github.com/ppazos/cabolabs-ehrserver/wiki/API-error-codes-and-messages> and
> <http://veratechnas1.synology.me:13606/InstanceValidator/rules.html>

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

When using the `POST` and `PUT` HTTP methods to create or update a resource, the service SHOULD give clients the option of returning either a complete
representation of the (modified) resource, or a minimal or no content in the payload response (assuming the operation was successfully completed).
See [RFC 7240](https://tools.ietf.org/html/rfc7240#section-4.2) for more details on how achieve this using `Prefer` header.
The client MAY choose any of the following:

- send `Prefer: return=minimal` to inform the service that prefers only a minimal response to a successful request.

A `Location` header indicating the direct URL to access the resource MUST be part of the service response.
If there is no payload content to be returned, the service SHOULD use HTTP status code `204 No Content`.

- send `Prefer: return=representation` to inform the service that prefers a full representation response to a successful request.

A `Location` header indicating the direct URL to access the resource MAY be part of the service response,
and the payload content SHOULD include a full representation.

In case no `Prefer` header is present in request, the default response policy is `return=minimal`.

Another preference is related to following and resolving OBJECT_REF identifiers. Under some circumstances a client
MAY indicate that prefers response containing full or partial resource representation instead of references to resources using OBJECT_REF.
A typical case is a list of COMPOSITIONs part of an EHR, which, strictly following RM specification, should always return
list of OBJECT_REF, but sometimes clients prefers to get COMPOSITIONs.
Services that have this capability implemented SHOULD accept and honor `Prefer: resolve_refs` request header.

```http
Prefer: return=representation, resolve_refs
```
