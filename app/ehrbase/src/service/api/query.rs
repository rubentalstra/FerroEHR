//! `QueryService` — ad-hoc + stored AQL execution (ITS-REST QUERY API), on the
//! [`crate::aql`] engine. Stored queries are resolved through the existing
//! `stored_query` store, then executed exactly like an ad-hoc query.

use async_trait::async_trait;
use serde_json::Value;

use ehrbase_rest::{AqlQueryRequest, QueryOutcome, QueryService};
use ehrbase_sm::SmError;

use crate::service::EhrbaseService;

#[async_trait]
impl QueryService for EhrbaseService {
    async fn query_execute_adhoc(
        &self,
        aql: String,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        self.execute_aql(&aql, None, &request).await
    }

    async fn query_execute_stored(
        &self,
        qualified_query_name: String,
        version: Option<String>,
        request: AqlQueryRequest,
    ) -> Result<QueryOutcome, SmError> {
        // Resolve the stored AQL text (exact/partial SEMVER, or the latest) via
        // the DEFINITION store, then execute it.
        let stored = self
            .get_stored_query(&qualified_query_name, version.as_deref())
            .await?;
        let aql = stored
            .get("q")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                SmError::exception(format!(
                    "stored query `{qualified_query_name}` has no query text"
                ))
            })?
            .to_owned();
        self.execute_aql(&aql, Some(&qualified_query_name), &request)
            .await
    }
}
