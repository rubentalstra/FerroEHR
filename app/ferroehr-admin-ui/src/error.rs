// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The console-wide error type, shared by both compilation targets so the UI
//! can render domain errors (CDR status + diagnostic) instead of opaque
//! strings.
//!
//! Implements `FromServerFnError` per the Leptos book (`server/25`).
//!
//! NOTE: the variants carry their cause as text because this type crosses the
//! server-fn boundary — `FromServerFnError` requires `Serialize`/`Deserialize`
//! (Leptos book `server/25`), which no underlying `reqwest` or parser error
//! implements, so an `#[source]` here would be unrepresentable rather than
//! merely inconvenient.

use serde::{Deserialize, Serialize};

/// Every failure a server function can hand the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum AdminUiError {
    /// No console session — the UI redirects to `/login`.
    #[error("not authenticated")]
    Unauthenticated,
    /// Authenticated but the CDR refused (401/403 from the CDR — the
    /// "insufficient scope" surface).
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// A non-2xx CDR answer, normalized: status + the diagnostic the CDR
    /// returned in its error body.
    #[error("CDR answered {status}: {message}")]
    Cdr {
        /// The CDR's HTTP status code.
        status: u16,
        /// The diagnostic extracted from the CDR error body (or the raw body).
        message: String,
    },
    /// The CDR was unreachable (connect/timeout/transport).
    #[error("CDR unreachable: {0}")]
    CdrUnreachable(String),
    /// Bad user input (login form, AQL text, template upload) with the
    /// message to render inline.
    #[error("{0}")]
    Invalid(String),
    /// Console-internal failure (config, session store, serialization).
    #[error("internal error: {0}")]
    Internal(String),
}

impl leptos::server_fn::error::FromServerFnError for AdminUiError {
    type Encoder = leptos::server_fn::codec::JsonEncoding;

    fn from_server_fn_error(value: leptos::server_fn::error::ServerFnErrorErr) -> Self {
        Self::Internal(value.to_string())
    }
}
