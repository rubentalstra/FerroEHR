[comment]: # (title: Qualified query name)

Stored queries are identified by their name, used as `qualified_query_name`, and an optional `version` number.

Usually a `qualified_query_name` has a format of `[{namespace}::]{query-name}`.

The `namespace` is optional, and when used it should be in a form of a reverse domain name, which allows for separation of use of stored queries by teams, companies, etc.
The `query-name` may include any combination of characters, matched by the pattern `[a-zA-Z0-9_.-]`. 

Examples of valid `qualified_query_name`:
- `org.openehr::my_compositions`
- `my_compositions`
- `ehr::all_influenza_vacc_candidates`

The `version` identifier is in the format specified by  [SEMVER](https://semver.org/) style (i.e. `major.minor.patch`).
When only a partial `version` pattern is supplied, or when `version` is not supplied at all, the system must use the latest `version` with the supplied prefix - i.e. if only `major` or `major.minor` is used, then the latest query version matching supplied prefix will be used.

