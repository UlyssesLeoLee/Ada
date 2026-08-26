//! `telemetry!` macro — thin wrapper around [`tracing::info_span!`].
//!
//! Other Ada crates can use the `layer:` keyword to attach a structured
//! [`crate::AdaLayer`] tag to a span without having to spell out the
//! span name and the `layer = ...` field by hand. The macro expands
//! to a plain [`tracing::info_span!`] call, so the rest of the
//! `tracing` API (fields, parents, follows-from) is available as usual.
//!
//! # Examples
//!
//! ```ignore
//! use ada_core::{telemetry, AdaLayer};
//!
//! // Layer-tagged span
//! let _span = telemetry!(layer: AdaLayer::Nerve, "canvas executed");
//!
//! // Pass-through to `tracing::info_span!` for everything else
//! let _span = telemetry!("plain span", tenant_id = %tenant);
//! ```

/// Build a `tracing::info_span!` with an optional `layer:` prefix.
///
/// Forms accepted:
///
/// - `telemetry!(layer: <AdaLayer>, name, fields...)` — the macro
///   inserts the canonical span name `"ada.telemetry"` and attaches
///   `layer = <AdaLayer>.as_str()` automatically.
/// - `telemetry!(args...)` — pass-through to
///   [`tracing::info_span!`] verbatim.
#[macro_export]
macro_rules! telemetry {
    (layer: $layer:expr, $($rest:tt)*) => {
        ::tracing::info_span!(
            "ada.telemetry",
            layer = $layer.as_str(),
            $($rest)*
        )
    };
    ($($rest:tt)*) => {
        ::tracing::info_span!($($rest)*)
    };
}

#[cfg(test)]
mod tests {
    use crate::AdaLayer;

    #[test]
    fn layer_keyword_form_compiles() {
        let span = crate::telemetry!(layer: AdaLayer::Nerve, "test");
        drop(span);
    }

    #[test]
    fn passthrough_form_compiles() {
        let span = crate::telemetry!("plain", foo = 42_u64);
        drop(span);
    }
}
