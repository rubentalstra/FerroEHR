//! Implementations of the generated ITS-REST server traits on [`AppState`].
//!
//! Each submodule provides one `impl {Group}Api for AppState`. In Stage 1
//! (P11) every operation returns [`ApiError::NotImplemented`](openehr_its::rest::runtime::ApiError::NotImplemented);
//! P12 replaces the bodies with the real service logic. The HTTP adapter that
//! extracts requests and calls these methods lives in [`crate::dispatch`].

/// Emit a whole `#[async_trait] impl {Group}Api for AppState` block of
/// `NotImplemented` stubs. The macro emits the `#[async_trait]` attribute itself
/// so that `async_trait` transforms the generated `async fn`s (an attribute
/// placed *around* a `macro_rules!` call runs before the call expands, and would
/// never see the methods).
///
/// Each method spec is `name(ParamsType[, BodyType]) -> ReturnType;`.
///
/// ```ignore
/// stub_api!(EhrApi, {
///     ehr_get_by_id(EhrGetByIdParams) -> serde_json::Value;
///     composition_create(CompositionCreateParams, serde_json::Value) -> serde_json::Value;
///     composition_delete(CompositionDeleteParams) -> ();
/// });
/// ```
macro_rules! stub_api {
    ($trait:path, { $( $name:ident ( $params:ty $(, $body:ty)? ) -> $ret:ty ; )* }) => {
        #[::async_trait::async_trait]
        impl $trait for $crate::state::AppState {
            $(
                async fn $name(
                    &self,
                    _params: $params,
                    $(_body: $body,)?
                ) -> ::core::result::Result<$ret, ::openehr_its::rest::runtime::ApiError> {
                    ::core::result::Result::Err(
                        ::openehr_its::rest::runtime::ApiError::NotImplemented,
                    )
                }
            )*
        }
    };
}
pub(crate) use stub_api;

mod admin;
mod definition;
mod demographic;
mod ehr;
mod query;
