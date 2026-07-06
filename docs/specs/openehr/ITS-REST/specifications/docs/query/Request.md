[comment]: # (title: Request details)

Below is a mostly self-documented AQL query in a request body.

```json
{
    "q": "SELECT o/data[at0002]/events[at0003]/data[at0001]/items[at0004]/value/magnitude AS temperature, o/data[at0002]/events[at0003]/data[at0001]/items[at0004]/value/units AS unit FROM EHR[ehr_id/value='554f896d-faca-4513-bddf-664541146308d'] CONTAINS Observation o[openEHR-EHR-OBSERVATION.body_temperature-zn.v1] WHERE o/data[at0002]/events[at0003]/data[at0001]/items[at0004]/value/magnitude > $temperature AND o/data[at0002]/events[at0003]/data[at0001]/items[at0.63 and name/value='Symptoms']/value/defining_code/code_string=$chills ORDER BY temperature DESC FETCH 3",
    "query_parameters": {
        "temperature": 38.5,
        "chills": "at0.64"
    }
}
```

## GET vs POST

Requests based on the `GET` method have URI length restriction, or some characters might not be allowed and have to be encoded.
Long queries in the `q` parameter and having a long list of `query_parameters` may add up to reach that limit, thus we recommend clients using the `POST` method instead of `GET`.

# Common Headers and Query Parameters 

All query execution requests SHOULD support at least the following parameters:
- `ehr_id` - used to execute the query within a single EHR context: an EHR identifier taken from EHR.ehr_id.value
- `offset` - used for paging: the row number in result to start result-set from (`0`-based); default `0`
- `fetch` - used for paging: the number of rows to fetch (i.e. limit); default depends on the implementation, but cannot be combined with [AQL-top](https://specifications.openehr.org/releases/QUERY/latest/AQL.html#_top)
- other parameter(s) to replace placeholder(s) within the query, here generically named `query_parameters` (see [below](#tag/Request/Common-Headers-and-Query-Parameters/Query-parameters)).

Related request headers:
- `openEHR-EHR-id` - used to execute the query within a single EHR context: an EHR identifier taken from EHR.ehr_id.value

Related response headers:
- `ETag` - A unique identifier of the resultSet


## About the ehr_id parameter

The `ehr_id` parameter SHOULD be supplied by clients when executing single EHR queries and MAY be used by the underlying backend to perform routing, optimizations or similar. It MUST NOT be supplied for 'population queries' and similar multi-patient queries.

Depending on the needs, clients MAY supply it as a query parameter `ehr_id` or alternatively as a request header named `openEHR-EHR-id`.


## Query parameters

Depending on each query definition, various query parameters SHOULD be supported, generically named `query_parameters`
in this specification, but in the real request they will have specific names (e.g. `uid`, `systolic_bp`, etc.)
according to their names in the query definition.

Provided query parameters SHOULD NOT be prefixed with `$` sign. Instead, the server will (whenever necessary)
add the prefix or format queries as valid AQL queries.

As an example, for the following AQL query
```
SELECT c/name/value FROM COMPOSITION c WHERE c/uid/value = $uid
```
named as `'myQuery'`, the client can supply the `uid` as a query parameter:
```
GET https://openEHRSys.example.com/v1/query/myQuery?uid=90910cf0-66a0-4382-b1f8-c0f27e81b42d::openEHRSys.example.com::1
```

As another example, the request
```
GET https://openEHRSys.example.com/v1/query/com.vendor::compositions?temperature_from=36&temperature_unit=Cel
```
will pass query parameters `temperature_from` and `temperature_unit` to the underlying AQL query named `com.vendor::compositions`.

See [AQL-parameters](https://specifications.openehr.org/releases/QUERY/latest/AQL.html#_parameters) specification for more details.

