//! The single `WebTemplate` resolution seam.

use std::sync::Arc;

use async_trait::async_trait;

use openehr_flat::WebTemplate;
use crate::error::{CallStatusType, SmError};

/// The single `WebTemplate` resolution seam (W2-K / finding F-13-02).
///
/// A stored OPT 1.4 template has exactly **one** built [`WebTemplate`]
/// representation, owned and cached by the service (one `moka` cache keyed by
/// template id). Every consumer — composition validation, the FLAT/STRUCTURED
/// (simSDT/structSDT) converters, and the Better `wt+json` template GET — goes
/// through this method, so the `WebTemplate` a composition is validated against
/// is byte-identical to the one its FLAT round-trip uses. The REST layer holds
/// no cache of its own and never re-fetches/re-parses OPT XML.
///
/// An unknown template id resolves as `Unprocessable` (→ ITS-REST `422`): on a
/// composition commit an unknown referenced template is a *semantic* error
/// (`422_COMPOSITION.yaml` — "the underlying template is not known"; CNF
/// `create_composition-event_bad_opt`).
#[async_trait]
pub trait WebTemplateService: Send + Sync {
    /// Resolve the (service-cached) [`WebTemplate`] for a stored operational
    /// template.
    async fn web_template(&self, _template_id: &str) -> Result<Arc<WebTemplate>, SmError> {
        Err(SmError::new(CallStatusType::NotImplemented, "not implemented"))
    }
}
