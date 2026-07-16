//! Subscriber assembly (binding doc §1.3): a reloadable `EnvFilter` + a stdout
//! `fmt` layer (json/pretty/auto) + an optional `OTel` export layer.
//!
//! `tracing` is the single instrumentation API; this module wires the three
//! consumers. The `EnvFilter` sits behind a `reload::Layer` so
//! `/management/loggers` can swap it live; the handle is captured behind
//! type-erased closures into a [`LogReload`]. The `OTel` layer is added **only**
//! when a tracer is supplied (endpoint configured) — absent, it is not in the
//! stack at all (zero overhead).

use std::io::IsTerminal;
use std::sync::Arc;

use crate::telemetry::log_reload::LogReload;
use opentelemetry_sdk::trace::SdkTracer;
use tracing_subscriber::layer::{Layered, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt, reload};

use super::TelemetryError;

/// The subscriber `S` seen by the data layers: the [`Registry`] with the
/// reloadable global `EnvFilter` layer composed onto it. Naming it lets the
/// `fmt`/`OTel` layers be type-erased into a homogeneous `Vec` and added in one
/// `.with()` (sequential `.with()` on boxed `dyn` layers cannot compose).
type Base = Layered<reload::Layer<EnvFilter, Registry>, Registry>;

/// A type-erased data layer over [`Base`].
type BoxedLayer = Box<dyn Layer<Base> + Send + Sync>;

/// Build the global subscriber and install it. Returns the [`LogReload`] handle
/// onto the reloadable filter. Adds the `OTel` layer iff `otel_tracer` is `Some`.
///
/// # Errors
/// Returns [`TelemetryError::Subscriber`] if a global subscriber is already set.
pub fn init_subscriber(
    format: super::config::LogFormat,
    boot_filter: &str,
    otel_tracer: Option<SdkTracer>,
) -> Result<LogReload, TelemetryError> {
    // Reloadable global filter. A bad boot directive falls back to `info` so the
    // process still starts (and logs the problem).
    let env_filter = EnvFilter::try_new(boot_filter).unwrap_or_else(|_| EnvFilter::new("info"));
    let (filter_layer, reload_handle) = reload::Layer::new(env_filter);

    // Data layers, type-erased over `Base` and added in a single `.with(Vec)`.
    let mut layers: Vec<BoxedLayer> = vec![fmt_layer(format)];
    if let Some(tracer) = otel_tracer {
        layers.push(tracing_opentelemetry::layer().with_tracer(tracer).boxed());
    }

    Registry::default()
        .with(filter_layer)
        .with(layers)
        .try_init()
        .map_err(|e| TelemetryError::Subscriber(e.to_string()))?;

    // Type-erase the reload handle (its `S` is `Registry`) behind read/apply
    // closures for `/management/loggers`.
    let read_handle = reload_handle.clone();
    let read = Arc::new(move || {
        read_handle
            .with_current(std::string::ToString::to_string)
            .unwrap_or_default()
    }) as Arc<dyn Fn() -> String + Send + Sync>;

    let apply = Arc::new(move |directives: &str| -> Result<(), String> {
        let new_filter = EnvFilter::try_new(directives).map_err(|e| e.to_string())?;
        reload_handle.reload(new_filter).map_err(|e| e.to_string())
    }) as Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

    Ok(LogReload::new(boot_filter, read, apply))
}

/// The stdout `fmt` layer for the chosen profile, boxed so the subscriber type
/// is uniform. JSON carries `trace_id`/`span_id`/`request_id` via the current
/// span's fields for trace↔log correlation.
fn fmt_layer(format: super::config::LogFormat) -> BoxedLayer {
    use super::config::LogFormat;
    let json = match format {
        LogFormat::Json => true,
        LogFormat::Pretty => false,
        LogFormat::Auto => !std::io::stdout().is_terminal(),
    };
    if json {
        fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .boxed()
    } else {
        // An explicit `pretty` is a human asking for the colored dev output
        // (e.g. `docker compose` logs, where stdout is a pipe, not a TTY) —
        // force ANSI on. Only `auto`-selected pretty keeps TTY detection.
        let ansi = matches!(format, LogFormat::Pretty) || std::io::stdout().is_terminal();
        fmt::layer().with_target(true).with_ansi(ansi).boxed()
    }
}
