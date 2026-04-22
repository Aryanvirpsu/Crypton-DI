//! # signals::metrics
//!
//! Central taxonomy for enrichment-side metrics. One function per emit site
//! so the label space stays small and reviewable, and so the `metrics`
//! feature flag is threaded through exactly one place per source.
//!
//! ## Emitted series
//!
//! | Metric                                           | Kind      | Labels                       |
//! |--------------------------------------------------|-----------|------------------------------|
//! | `risk_engine_enrichment_classification_total`    | counter   | `source`, `result`           |
//! | `risk_engine_enrichment_duration_seconds`        | histogram | `source`                     |
//! | `risk_engine_enrichment_errors_total`            | counter   | `source`                     |
//!
//! ### Label values
//!
//! - `source` ∈ `{geo, ip_reputation, breach, sanctions, disposable_email, client_sdk}`
//! - `result` ∈ `{matched, not_matched, unknown}` — `unknown` covers missing
//!   inputs (e.g., no IP in the request) so a `0/0` denominator in dashboards
//!   doesn't silently drop requests.
//!
//! ## No-op when the feature is off
//!
//! All helpers compile to a no-op under `default-features = false`.
//! Call them unconditionally — the `metrics` feature flips every call site
//! on together.

/// Stable identifier for an enrichment source. Mapped to the `source`
/// Prometheus label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnrichmentSource {
    Geo,
    IpReputation,
    Breach,
    Sanctions,
    DisposableEmail,
    ClientSdk,
}

impl EnrichmentSource {
    pub const fn as_label(self) -> &'static str {
        match self {
            EnrichmentSource::Geo => "geo",
            EnrichmentSource::IpReputation => "ip_reputation",
            EnrichmentSource::Breach => "breach",
            EnrichmentSource::Sanctions => "sanctions",
            EnrichmentSource::DisposableEmail => "disposable_email",
            EnrichmentSource::ClientSdk => "client_sdk",
        }
    }
}

/// Outcome of a classification call. Distinct from a *lookup success*: a
/// source that returned "no, this IP is not a VPN" is `NotMatched`, not
/// `Unknown`. `Unknown` is reserved for "we couldn't evaluate" (missing
/// input, timeout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationResult {
    Matched,
    NotMatched,
    Unknown,
}

impl ClassificationResult {
    pub const fn as_label(self) -> &'static str {
        match self {
            ClassificationResult::Matched => "matched",
            ClassificationResult::NotMatched => "not_matched",
            ClassificationResult::Unknown => "unknown",
        }
    }
}

/// Bump the classification counter. Call this from the adapter after each
/// enrichment resolution.
#[inline]
pub fn record_classification(
    _source: EnrichmentSource,
    _result: ClassificationResult,
) {
    #[cfg(feature = "metrics")]
    metrics::counter!(
        "risk_engine_enrichment_classification_total",
        "source" => _source.as_label(),
        "result" => _result.as_label(),
    )
    .increment(1);
}

/// Record the wall-clock duration of an enrichment lookup. Intended to be
/// called with `Instant::elapsed().as_secs_f64()` immediately after the
/// upstream call returns (including error paths).
#[inline]
pub fn record_duration(_source: EnrichmentSource, _seconds: f64) {
    #[cfg(feature = "metrics")]
    metrics::histogram!(
        "risk_engine_enrichment_duration_seconds",
        "source" => _source.as_label(),
    )
    .record(_seconds);
}

/// Bump the enrichment-errors counter when a lookup itself fails (timeout,
/// network error, corrupt DB record). The corresponding `record_classification`
/// call should use `ClassificationResult::Unknown` to keep the two series
/// aligned.
#[inline]
pub fn record_error(_source: EnrichmentSource) {
    #[cfg(feature = "metrics")]
    metrics::counter!(
        "risk_engine_enrichment_errors_total",
        "source" => _source.as_label(),
    )
    .increment(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_labels_are_stable() {
        // Downstream dashboards key off these exact strings — locking them
        // prevents an accidental rename from silently breaking queries.
        assert_eq!(EnrichmentSource::Geo.as_label(), "geo");
        assert_eq!(EnrichmentSource::IpReputation.as_label(), "ip_reputation");
        assert_eq!(EnrichmentSource::Breach.as_label(), "breach");
        assert_eq!(EnrichmentSource::Sanctions.as_label(), "sanctions");
        assert_eq!(
            EnrichmentSource::DisposableEmail.as_label(),
            "disposable_email"
        );
        assert_eq!(EnrichmentSource::ClientSdk.as_label(), "client_sdk");
    }

    #[test]
    fn result_labels_are_stable() {
        assert_eq!(ClassificationResult::Matched.as_label(), "matched");
        assert_eq!(ClassificationResult::NotMatched.as_label(), "not_matched");
        assert_eq!(ClassificationResult::Unknown.as_label(), "unknown");
    }

    #[test]
    fn helpers_are_noop_without_metrics_feature() {
        // These should never panic regardless of feature state.
        record_classification(EnrichmentSource::Geo, ClassificationResult::Matched);
        record_duration(EnrichmentSource::Breach, 0.015);
        record_error(EnrichmentSource::IpReputation);
    }
}
