[comment]: # (title: Query types)

## Single EHR queries

A common use-case is to execute queries within a specific EHR. 

This is achieved by supplying a `ehr_id` query parameter or setting a `openEHR-EHR-id` request header.


## Population queries

Population queries are queries which are executed on several EHRs in the same request.
These queries are not referring or using any `ehr_id` parameter to constrain the scope on a specific EHR. 

Examples of use-cases can be:
- Ward lists,
- Explore correlations between patients in a pandemic situation,
- Research, e.g. epidemiology, population health.


## Stored queries

Stored queries are queries which have their definition stored (registered) on the server. 
These are identified by their [qualified name and version](#tag/Qualified_query_name).

Using stored queries has several of advantages:
- separation of responsibilities (some users/developers write queries, others just call/execute them and consume the responses),
- no need to pass long query string over the network.

Queries can be stored or, once stored, their definition can be retrieved using the [definition endpoint](definition.html#tag/Query).


## Ad-hoc queries

As opposed to stored queries, ad-hoc type queries does not have their definitions stored on the server, neither any associated identifier.
These queries will be executed as-is, as part request body or query parameter, by the [Execute ad-hoc AQL](#tag/Query/operation/query_execute_adhoc_query) operation endpoint. 
