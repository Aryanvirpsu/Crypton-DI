//! # enrichment
//!
//! Single entry point that stitches every Phase-C enrichment source onto a
//! `RiskContext` before `evaluate()` runs.
//!
//! The engine itself never calls out to IP reputation feeds, GeoIP databases,
//! HIBP, or sanctions lists — those live in [`crate::signals`] as pure
//! helpers. This module is the thin orchestration layer that:
//!
//! 1. Holds optional references to each configured source.
//! 2. For each request, calls the relevant sources under a per-step timeout.
//! 3. Writes results onto [`RiskContext`] fields.
//! 4. Emits Phase-D classification / duration / error metrics.
//! 5. Fails open on every error — a source that times out leaves the field
//!    `None` and lets the engine handle absence the way it already does.
//!
//! ## Usage
//!
//! ```ignore
//! // Adapter startup
//! let pipeline = Pipeline::builder()
//!     .with_ip_reputation(Arc::new(MyVpnFeed))
//!     .with_disposable_domains(Arc::new(DisposableDomains::builtin()))
//!     .with_geoip(Arc::new(GeoIpReader::open("GeoLite2-City.mmdb", Some("GeoLite2-ASN.mmdb"))?))
//!     .build();
//!
//! // Per-request, inside the adapter, BEFORE evaluate():
//! pipeline.enrich_context(&mut ctx, email.as_deref()).await;
//! let decision = evaluate(ctx, &store).await;
//!
//! // Optional side channel for password-bearing flows:
//! let breached = pipeline.check_password_breached(password).await;
//! ```
//!
//! ## Scope
//!
//! - `enrich_context` handles every field whose value can be derived from
//!   what's already on `RiskContext` (request_ip, last_login_ip, tenant_id).
//! - Password-bearing checks (HIBP) are exposed as a separate method so
//!   plaintext passwords never land on `RiskContext`.
//! - Client-SDK header parsing stays in [`crate::signals::client_sdk`] —
//!   headers are HTTP-layer and belong in the Axum adapter, not here.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use crate::context::{AsnType, RiskContext};
use crate::signals::breach::BreachChecker;
use crate::signals::disposable_email::{extract_domain, DisposableDomains};
use crate::signals::ip_reputation::IpReputationStore;
use crate::signals::metrics::{
    record_classification, record_duration, record_error, ClassificationResult,
    EnrichmentSource,
};
use crate::signals::sanctions::is_sanctioned;

#[cfg(feature = "geoip")]
use crate::signals::geoip::GeoIpReader;

// ─────────────────────────────────────────────────────────────────────────────
// Config
// ─────────────────────────────────────────────────────────────────────────────

/// Per-step timeout for each enrichment source. Conservative defaults so a
/// slow upstream (HIBP outage, GeoIP library hang) cannot stall the request.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Budget for each individual source call. A miss becomes a `Timeout`
    /// error which is surfaced as an `unknown` classification and the field
    /// stays `None`.
    pub per_step_timeout: Duration,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            // 50 ms is generous — GeoIP lookups are ~5 µs in-process,
            // IP-rep APIs are usually < 30 ms p99. If a source consistently
            // pushes this ceiling, the source is the problem.
            per_step_timeout: Duration::from_millis(50),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Orchestrator that owns handles to each configured enrichment source and
/// applies them to a `RiskContext`. Every source is optional — a deployment
/// can ship with, say, only sanctions + disposable-email and skip the rest.
///
/// Cloneable via `Arc` internals; share a single instance across the service.
#[derive(Clone)]
pub struct Pipeline {
    ip_reputation: Option<Arc<dyn IpReputationStore>>,
    breach: Option<Arc<dyn BreachChecker>>,
    disposable_domains: Option<Arc<DisposableDomains>>,
    /// Host-supplied country-code resolver. Signature: `ip -> Option<ISO alpha-2>`.
    /// When the `geoip` feature is active the builder's `with_geoip` wires
    /// [`GeoIpReader`] into this slot. Hosts that resolve country codes
    /// through a different provider (IPinfo HTTP, ip2location, internal
    /// feed) can supply their own closure via [`Builder::with_country_resolver`].
    country_resolver: Option<Arc<dyn CountryResolver>>,
    /// Optional ASN-type resolver for populating `ip_asn_type`.
    asn_resolver: Option<Arc<dyn AsnResolver>>,
    config: PipelineConfig,
}

/// Trait for resolving an IP to an ISO 3166-1 alpha-2 country code.
/// Kept sync because the canonical source is a local MaxMind DB; if a host
/// uses an HTTP provider they should wrap the call in a `tokio::task::spawn_blocking`
/// or pre-resolve with a synchronous cache layer.
pub trait CountryResolver: Send + Sync {
    fn resolve_country(&self, ip: std::net::IpAddr) -> Option<String>;
}

/// Trait for resolving an IP to an [`AsnType`] bucket.
pub trait AsnResolver: Send + Sync {
    fn resolve_asn(&self, ip: std::net::IpAddr) -> Option<AsnType>;
}

#[cfg(feature = "geoip")]
impl CountryResolver for GeoIpReader {
    fn resolve_country(&self, ip: std::net::IpAddr) -> Option<String> {
        self.lookup_country_code(ip)
    }
}

#[cfg(feature = "geoip")]
impl AsnResolver for GeoIpReader {
    fn resolve_asn(&self, ip: std::net::IpAddr) -> Option<AsnType> {
        self.lookup_asn_type(ip)
    }
}

impl Pipeline {
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Apply every configured enrichment to `ctx`. Safe to call unconditionally —
    /// when a source is not configured, the matching field is left untouched.
    ///
    /// `email` is passed in separately because it usually comes from the
    /// request body / user profile, not from an existing `RiskContext` field.
    pub async fn enrich_context(&self, ctx: &mut RiskContext, email: Option<&str>) {
        // ── Country resolution (sanctions + geo) ─────────────────────────────
        if ctx.is_sanctioned_country.is_none() {
            if let Some(ip) = ctx.request_ip {
                let country = self.resolve_country(ip);
                if let Some(code) = country.as_deref() {
                    let sanctioned = is_sanctioned(code);
                    ctx.is_sanctioned_country = Some(sanctioned);
                    record_classification(
                        EnrichmentSource::Sanctions,
                        bool_result(sanctioned),
                    );
                } else {
                    record_classification(
                        EnrichmentSource::Sanctions,
                        ClassificationResult::Unknown,
                    );
                }
            } else {
                record_classification(
                    EnrichmentSource::Sanctions,
                    ClassificationResult::Unknown,
                );
            }
        }

        // ── ASN type ─────────────────────────────────────────────────────────
        if ctx.ip_asn_type.is_none() {
            if let (Some(ip), Some(resolver)) = (ctx.request_ip, self.asn_resolver.as_ref()) {
                let start = Instant::now();
                let asn = resolver.resolve_asn(ip);
                record_duration(EnrichmentSource::Geo, start.elapsed().as_secs_f64());
                ctx.ip_asn_type = asn.clone();
                record_classification(
                    EnrichmentSource::Geo,
                    match asn {
                        Some(AsnType::Unknown) | None => ClassificationResult::Unknown,
                        Some(_) => ClassificationResult::Matched,
                    },
                );
            }
        }

        // ── IP reputation (vpn/proxy/relay/abuse) ────────────────────────────
        if let Some(store) = self.ip_reputation.as_ref() {
            if let Some(ip) = ctx.request_ip {
                let start = Instant::now();
                let result = tokio::time::timeout(
                    self.config.per_step_timeout,
                    store.classify(ip),
                )
                .await;
                record_duration(
                    EnrichmentSource::IpReputation,
                    start.elapsed().as_secs_f64(),
                );

                match result {
                    Ok(Ok(rep)) => {
                        if ctx.ip_is_vpn.is_none() {
                            ctx.ip_is_vpn = rep.is_vpn;
                        }
                        if ctx.ip_is_proxy.is_none() {
                            ctx.ip_is_proxy = rep.is_proxy;
                        }
                        if ctx.ip_is_relay.is_none() {
                            ctx.ip_is_relay = rep.is_relay;
                        }
                        if ctx.ip_abuse_confidence.is_none() {
                            ctx.ip_abuse_confidence = rep.abuse_confidence;
                        }
                        let matched = rep.is_anonymised()
                            || rep.abuse_confidence.map(|c| c > 0).unwrap_or(false);
                        record_classification(
                            EnrichmentSource::IpReputation,
                            if matched {
                                ClassificationResult::Matched
                            } else {
                                ClassificationResult::NotMatched
                            },
                        );
                    }
                    Ok(Err(e)) => {
                        warn!(error = %e, "ip_reputation lookup failed");
                        record_error(EnrichmentSource::IpReputation);
                        record_classification(
                            EnrichmentSource::IpReputation,
                            ClassificationResult::Unknown,
                        );
                    }
                    Err(_) => {
                        warn!(
                            timeout_ms = self.config.per_step_timeout.as_millis() as u64,
                            "ip_reputation lookup timed out"
                        );
                        record_error(EnrichmentSource::IpReputation);
                        record_classification(
                            EnrichmentSource::IpReputation,
                            ClassificationResult::Unknown,
                        );
                    }
                }
            }
        }

        // ── Disposable email ─────────────────────────────────────────────────
        if ctx.email_domain_disposable.is_none() {
            if let Some(addr) = email {
                let start = Instant::now();
                let disposable = self.check_disposable(addr);
                record_duration(
                    EnrichmentSource::DisposableEmail,
                    start.elapsed().as_secs_f64(),
                );
                match disposable {
                    Some(is_disp) => {
                        ctx.email_domain_disposable = Some(is_disp);
                        record_classification(
                            EnrichmentSource::DisposableEmail,
                            bool_result(is_disp),
                        );
                    }
                    None => record_classification(
                        EnrichmentSource::DisposableEmail,
                        ClassificationResult::Unknown,
                    ),
                }
            }
        }

        debug!("enrichment pipeline completed");
    }

    /// Check whether a password has appeared in a known breach. Separate
    /// method so plaintext passwords never need to land on `RiskContext`.
    /// Returns `None` when no breach checker is configured or when the
    /// lookup times out.
    pub async fn check_password_breached(&self, password: &str) -> Option<bool> {
        let checker = self.breach.as_ref()?;
        let start = Instant::now();
        let result =
            tokio::time::timeout(self.config.per_step_timeout, checker.is_breached(password))
                .await;
        record_duration(EnrichmentSource::Breach, start.elapsed().as_secs_f64());

        match result {
            Ok(Ok(is_b)) => {
                record_classification(EnrichmentSource::Breach, bool_result(is_b));
                Some(is_b)
            }
            Ok(Err(e)) => {
                warn!(error = %e, "breach lookup failed");
                record_error(EnrichmentSource::Breach);
                record_classification(EnrichmentSource::Breach, ClassificationResult::Unknown);
                None
            }
            Err(_) => {
                warn!(
                    timeout_ms = self.config.per_step_timeout.as_millis() as u64,
                    "breach lookup timed out"
                );
                record_error(EnrichmentSource::Breach);
                record_classification(EnrichmentSource::Breach, ClassificationResult::Unknown);
                None
            }
        }
    }

    // ── internal helpers ─────────────────────────────────────────────────────

    fn resolve_country(&self, ip: std::net::IpAddr) -> Option<String> {
        let resolver = self.country_resolver.as_ref()?;
        resolver.resolve_country(ip)
    }

    fn check_disposable(&self, email: &str) -> Option<bool> {
        let set = self.disposable_domains.as_ref()?;
        let domain = extract_domain(email)?;
        Some(set.contains(&domain))
    }
}

#[inline]
fn bool_result(b: bool) -> ClassificationResult {
    if b {
        ClassificationResult::Matched
    } else {
        ClassificationResult::NotMatched
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Builder
// ─────────────────────────────────────────────────────────────────────────────

/// Construction helper. Every source is optional and additive. The builder
/// is the only way to construct a [`Pipeline`] — having the struct fields
/// private keeps the "unconfigured sources fail open" invariant enforceable.
#[derive(Default)]
pub struct Builder {
    ip_reputation: Option<Arc<dyn IpReputationStore>>,
    breach: Option<Arc<dyn BreachChecker>>,
    disposable_domains: Option<Arc<DisposableDomains>>,
    country_resolver: Option<Arc<dyn CountryResolver>>,
    asn_resolver: Option<Arc<dyn AsnResolver>>,
    config: PipelineConfig,
}

impl Builder {
    pub fn with_ip_reputation(mut self, store: Arc<dyn IpReputationStore>) -> Self {
        self.ip_reputation = Some(store);
        self
    }

    pub fn with_breach_checker(mut self, checker: Arc<dyn BreachChecker>) -> Self {
        self.breach = Some(checker);
        self
    }

    pub fn with_disposable_domains(mut self, set: Arc<DisposableDomains>) -> Self {
        self.disposable_domains = Some(set);
        self
    }

    /// Provide a generic country resolver (e.g., IPinfo, in-house feed).
    /// When the `geoip` feature is enabled and [`Self::with_geoip`] has
    /// been called, this is set automatically — you only need this for
    /// non-MaxMind sources.
    pub fn with_country_resolver(mut self, resolver: Arc<dyn CountryResolver>) -> Self {
        self.country_resolver = Some(resolver);
        self
    }

    pub fn with_asn_resolver(mut self, resolver: Arc<dyn AsnResolver>) -> Self {
        self.asn_resolver = Some(resolver);
        self
    }

    /// Wire a [`GeoIpReader`] in as both the country resolver and the ASN
    /// resolver in one call. Requires the `geoip` feature.
    #[cfg(feature = "geoip")]
    pub fn with_geoip(mut self, reader: Arc<GeoIpReader>) -> Self {
        self.country_resolver = Some(reader.clone());
        self.asn_resolver = Some(reader);
        self
    }

    pub fn with_config(mut self, config: PipelineConfig) -> Self {
        self.config = config;
        self
    }

    pub fn build(self) -> Pipeline {
        Pipeline {
            ip_reputation: self.ip_reputation,
            breach: self.breach,
            disposable_domains: self.disposable_domains,
            country_resolver: self.country_resolver,
            asn_resolver: self.asn_resolver,
            config: self.config,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::RiskAction;
    use crate::signals::breach::StaticBreachChecker;
    use crate::signals::ip_reputation::{
        IpReputation, IpReputationStore, NoopIpReputation, StaticIpReputation,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use std::net::{IpAddr, Ipv4Addr};
    use uuid::Uuid;

    fn base_ctx(ip: Option<IpAddr>) -> RiskContext {
        RiskContext {
            request_id: Uuid::new_v4(),
            evaluated_at: Utc::now(),
            user_id: Uuid::new_v4(),
            username: "u".into(),
            credential_id: None,
            action: RiskAction::Login,
            resource_id: None,
            credential_status: crate::context::CredentialStatus::Active,
            credential_created_at: Utc::now(),
            credential_last_used_at: None,
            credential_sign_count_prev: 0,
            credential_sign_count_new: 1,
            credential_registered_ua: None,
            credential_count_for_user: 1,
            prior_audit_event_count: 0,
            last_login_ip: None,
            last_login_at: None,
            last_login_geo: None,
            jwt_issued_at: None,
            jwt_expires_at: None,
            jwt_fingerprint_stored: None,
            jwt_fingerprint_current: None,
            request_ip: ip,
            request_ua: None,
            request_geo: None,
            ip_asn_type: None,
            login_attempts_5m: None,
            failed_login_attempts_1h: None,
            actions_executed_5m: None,
            recovery_requests_24h: None,
            device_revocations_1h: None,
            registrations_from_ip_10m: None,
            active_session_count: None,
            account_locked: false,
            recovery_pending: false,
            oauth_authorize_ip: None,
            nonce_present: false,
            nonce_already_used: false,
            org_risk_level: crate::context::OrgRiskLevel::Normal,
            redis_signals_degraded: false,
            db_signals_degraded: false,
            tenant_id: "t".into(),
            org_risk_score: None,
            cluster_membership: None,
            threshold_shift: None,
            org_active_cluster_count: 0,
            login_attempts_1m: None,
            login_attempts_1h: None,
            login_attempts_24h: None,
            client_timestamp: None,
            device_fingerprint_hash: None,
            ja3_fingerprint: None,
            known_ip_for_user: None,
            ip_is_vpn: None,
            ip_is_proxy: None,
            ip_is_relay: None,
            ip_abuse_confidence: None,
            geo_allowed_countries: None,
            is_sanctioned_country: None,
            webdriver_detected: None,
            captcha_score: None,
            screen_resolution: None,
            touch_capable: None,
            account_age_days: None,
            email_verified: None,
            email_domain_disposable: None,
            breached_credential: None,
            user_typical_hours: None,
            accept_language: None,
            previous_accept_language: None,
            device_trust_level: None,
            policy: std::sync::Arc::new(crate::policy_config::PolicyConfig::default()),
        }
    }

    struct CountryStub(&'static str);
    impl CountryResolver for CountryStub {
        fn resolve_country(&self, _: IpAddr) -> Option<String> {
            Some(self.0.into())
        }
    }

    struct AsnStub(AsnType);
    impl AsnResolver for AsnStub {
        fn resolve_asn(&self, _: IpAddr) -> Option<AsnType> {
            Some(self.0.clone())
        }
    }

    #[tokio::test]
    async fn empty_pipeline_is_a_noop() {
        let pipeline = Pipeline::builder().build();
        let mut ctx = base_ctx(Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))));
        pipeline.enrich_context(&mut ctx, Some("a@gmail.com")).await;
        // No source configured → every field stays `None`.
        assert_eq!(ctx.is_sanctioned_country, None);
        assert_eq!(ctx.ip_is_vpn, None);
        assert_eq!(ctx.email_domain_disposable, None);
        assert_eq!(ctx.ip_asn_type, None);
    }

    #[tokio::test]
    async fn sanctioned_country_populates_via_country_resolver() {
        let pipeline = Pipeline::builder()
            .with_country_resolver(Arc::new(CountryStub("IR")))
            .build();
        let mut ctx = base_ctx(Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        pipeline.enrich_context(&mut ctx, None).await;
        assert_eq!(ctx.is_sanctioned_country, Some(true));
    }

    #[tokio::test]
    async fn clean_country_is_not_sanctioned() {
        let pipeline = Pipeline::builder()
            .with_country_resolver(Arc::new(CountryStub("US")))
            .build();
        let mut ctx = base_ctx(Some(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        pipeline.enrich_context(&mut ctx, None).await;
        assert_eq!(ctx.is_sanctioned_country, Some(false));
    }

    #[tokio::test]
    async fn sanctions_skipped_when_no_ip() {
        let pipeline = Pipeline::builder()
            .with_country_resolver(Arc::new(CountryStub("IR")))
            .build();
        let mut ctx = base_ctx(None);
        pipeline.enrich_context(&mut ctx, None).await;
        // No IP → no resolution.
        assert_eq!(ctx.is_sanctioned_country, None);
    }

    #[tokio::test]
    async fn asn_resolver_populates_ip_asn_type() {
        let pipeline = Pipeline::builder()
            .with_asn_resolver(Arc::new(AsnStub(AsnType::Datacenter)))
            .build();
        let mut ctx = base_ctx(Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        pipeline.enrich_context(&mut ctx, None).await;
        assert_eq!(ctx.ip_asn_type, Some(AsnType::Datacenter));
    }

    #[tokio::test]
    async fn ip_reputation_vpn_hit() {
        let vpn = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let store: Arc<dyn IpReputationStore> = Arc::new(StaticIpReputation {
            known_vpn: vec![vpn],
            ..Default::default()
        });
        let pipeline = Pipeline::builder().with_ip_reputation(store).build();
        let mut ctx = base_ctx(Some(vpn));
        pipeline.enrich_context(&mut ctx, None).await;
        assert_eq!(ctx.ip_is_vpn, Some(true));
        assert_eq!(ctx.ip_is_proxy, Some(false));
    }

    #[tokio::test]
    async fn ip_reputation_noop_marks_not_matched() {
        let pipeline = Pipeline::builder()
            .with_ip_reputation(Arc::new(NoopIpReputation))
            .build();
        let mut ctx = base_ctx(Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        pipeline.enrich_context(&mut ctx, None).await;
        // Noop returns every field None — fields stay None.
        assert_eq!(ctx.ip_is_vpn, None);
    }

    struct SlowStore;
    #[async_trait]
    impl IpReputationStore for SlowStore {
        async fn classify(
            &self,
            _ip: IpAddr,
        ) -> Result<IpReputation, crate::error::RiskEngineError> {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(IpReputation::default())
        }
    }

    #[tokio::test]
    async fn ip_reputation_timeout_fails_open() {
        let pipeline = Pipeline::builder()
            .with_ip_reputation(Arc::new(SlowStore))
            .with_config(PipelineConfig {
                per_step_timeout: Duration::from_millis(5),
            })
            .build();
        let mut ctx = base_ctx(Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        pipeline.enrich_context(&mut ctx, None).await;
        // Timeout leaves all rep fields untouched.
        assert_eq!(ctx.ip_is_vpn, None);
        assert_eq!(ctx.ip_is_proxy, None);
    }

    #[tokio::test]
    async fn disposable_email_match() {
        let pipeline = Pipeline::builder()
            .with_disposable_domains(Arc::new(DisposableDomains::builtin()))
            .build();
        let mut ctx = base_ctx(None);
        pipeline
            .enrich_context(&mut ctx, Some("bad@mailinator.com"))
            .await;
        assert_eq!(ctx.email_domain_disposable, Some(true));
    }

    #[tokio::test]
    async fn disposable_email_legit_not_flagged() {
        let pipeline = Pipeline::builder()
            .with_disposable_domains(Arc::new(DisposableDomains::builtin()))
            .build();
        let mut ctx = base_ctx(None);
        pipeline
            .enrich_context(&mut ctx, Some("ok@gmail.com"))
            .await;
        assert_eq!(ctx.email_domain_disposable, Some(false));
    }

    #[tokio::test]
    async fn preset_fields_are_respected() {
        // Simulate adapter already having resolved the value from a cache.
        let mut ctx = base_ctx(Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))));
        ctx.is_sanctioned_country = Some(true);
        ctx.ip_is_vpn = Some(true);

        let pipeline = Pipeline::builder()
            .with_country_resolver(Arc::new(CountryStub("US")))
            .with_ip_reputation(Arc::new(NoopIpReputation))
            .build();
        pipeline.enrich_context(&mut ctx, None).await;

        // Values set by caller must not be overwritten by enrichment.
        assert_eq!(ctx.is_sanctioned_country, Some(true));
        assert_eq!(ctx.ip_is_vpn, Some(true));
    }

    #[tokio::test]
    async fn check_password_breached_hits() {
        let pipeline = Pipeline::builder()
            .with_breach_checker(Arc::new(StaticBreachChecker::from_plaintext([
                "hunter2",
            ])))
            .build();
        assert_eq!(pipeline.check_password_breached("hunter2").await, Some(true));
        assert_eq!(
            pipeline
                .check_password_breached("long-unique-value-not-in-corpus")
                .await,
            Some(false)
        );
    }

    #[tokio::test]
    async fn check_password_without_checker_returns_none() {
        let pipeline = Pipeline::builder().build();
        assert_eq!(pipeline.check_password_breached("anything").await, None);
    }
}
