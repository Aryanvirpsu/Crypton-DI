//! # ip_reputation
//!
//! IP intelligence signals: VPN / proxy / privacy-relay / abuse scoring.
//!
//! Populates `RiskContext::ip_is_vpn`, `ip_is_proxy`, `ip_is_relay`,
//! `ip_abuse_confidence`. These are separate from `ip_asn_type` because a
//! single IP can legitimately be `AsnType::Residential` (home ISP) and also
//! flagged as a VPN exit node (residential VPN services are common).
//!
//! ## Provider-agnostic by design
//!
//! The engine must not know whether the host uses MaxMind, IPinfo, AbuseIPDB,
//! ip2location, or an internal threat feed. All a host has to do is
//! implement the [`IpReputationStore`] trait. A zero-dependency
//! [`NoopIpReputation`] is provided for tests and deployments that haven't
//! wired a real provider yet.
//!
//! ## Where the default impls live
//!
//! - `NoopIpReputation` is always available.
//! - MaxMind Anonymous IP DB support lives at
//!   [`super::geoip::GeoIpReader`] when the `geoip` feature is on. You'd
//!   wrap it in a tiny adapter that implements this trait.
//! - AbuseIPDB / IPinfo are typically HTTP APIs — keep those implementations
//!   out of this crate; they belong in the host service where the HTTP
//!   client is already configured.

use std::net::IpAddr;

use async_trait::async_trait;

use crate::error::RiskEngineError;

/// Classification bundle for a single IP. Every field is `Option` so a
/// provider that only supplies some signals (e.g., a VPN list with no abuse
/// score) can still participate without lying about fields it cannot fill.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IpReputation {
    pub is_vpn: Option<bool>,
    pub is_proxy: Option<bool>,
    pub is_relay: Option<bool>,
    /// 0–100 abuse confidence, higher = more reported abuse. Providers that
    /// only give a boolean reputation should leave this `None` and set
    /// `is_proxy`/`is_vpn` instead of pretending to have a score.
    pub abuse_confidence: Option<u8>,
}

impl IpReputation {
    /// Convenience: `true` if any of the anonymisation flags is `Some(true)`.
    /// A `None` flag is NOT treated as "false" — the distinction between
    /// "provider said no" and "provider didn't answer" is preserved.
    pub fn is_anonymised(&self) -> bool {
        self.is_vpn == Some(true)
            || self.is_proxy == Some(true)
            || self.is_relay == Some(true)
    }
}

/// Async trait any IP intelligence provider implements. The engine adapter
/// calls `classify` during context enrichment and stitches the result onto
/// `RiskContext`.
///
/// Implementations MUST respect a budget — the adapter typically wraps the
/// call in `tokio::time::timeout(Duration::from_millis(50), …)` so a slow
/// provider cannot stall request ingestion. A timeout surfaces as
/// [`RiskEngineError::Timeout`] and the engine fails open.
#[async_trait]
pub trait IpReputationStore: Send + Sync {
    async fn classify(&self, ip: IpAddr) -> Result<IpReputation, RiskEngineError>;
}

/// Always returns `IpReputation::default()` — every field `None`. Useful for
/// tests, local dev, and deployments that haven't wired a real provider.
/// Never rejects, never errors.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopIpReputation;

#[async_trait]
impl IpReputationStore for NoopIpReputation {
    async fn classify(&self, _ip: IpAddr) -> Result<IpReputation, RiskEngineError> {
        Ok(IpReputation::default())
    }
}

/// Static allow/deny classification backed by two lists. Intended for tests
/// and tiny deployments; production should use a real provider.
///
/// An IP is classified by exact match against each list. No CIDR support —
/// if you need CIDR, your provider is big enough to warrant its own adapter.
#[derive(Debug, Default, Clone)]
pub struct StaticIpReputation {
    pub known_vpn: Vec<IpAddr>,
    pub known_proxy: Vec<IpAddr>,
    pub known_relay: Vec<IpAddr>,
}

#[async_trait]
impl IpReputationStore for StaticIpReputation {
    async fn classify(&self, ip: IpAddr) -> Result<IpReputation, RiskEngineError> {
        Ok(IpReputation {
            is_vpn: Some(self.known_vpn.contains(&ip)),
            is_proxy: Some(self.known_proxy.contains(&ip)),
            is_relay: Some(self.known_relay.contains(&ip)),
            abuse_confidence: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn noop_returns_all_none() {
        let r = NoopIpReputation
            .classify(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))
            .await
            .unwrap();
        assert_eq!(r, IpReputation::default());
        assert!(!r.is_anonymised());
    }

    #[tokio::test]
    async fn static_matches_exact() {
        let vpn = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let store = StaticIpReputation {
            known_vpn: vec![vpn],
            ..Default::default()
        };
        let hit = store.classify(vpn).await.unwrap();
        assert_eq!(hit.is_vpn, Some(true));
        assert!(hit.is_anonymised());

        let miss = store
            .classify(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)))
            .await
            .unwrap();
        assert_eq!(miss.is_vpn, Some(false));
        assert!(!miss.is_anonymised());
    }

    #[test]
    fn is_anonymised_only_true_when_any_flag_true() {
        let r = IpReputation {
            is_vpn: Some(false),
            is_proxy: None,
            is_relay: None,
            abuse_confidence: Some(10),
        };
        assert!(!r.is_anonymised());

        let r2 = IpReputation {
            is_vpn: None,
            is_proxy: Some(true),
            ..Default::default()
        };
        assert!(r2.is_anonymised());
    }
}
