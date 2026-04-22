//! # signals
//!
//! Enrichment helpers that resolve raw request data (IP, email, headers, …)
//! into the typed fields the engine consumes on `RiskContext`. Everything in
//! this module runs *before* `engine::evaluate()` — the engine itself never
//! calls out to enrichment sources.
//!
//! ## Sources by category
//!
//! - [`geo`] — elapsed-hours helper for time decay.
//! - [`fingerprint`] — JWT/request fingerprint hashing.
//! - [`sanctions`] — OFAC sanctioned-country lookup (bundled).
//! - [`disposable_email`] — disposable-email-domain lookup (bundled).
//! - [`client_sdk`] — HTTP header parsers for the client-side fingerprinting SDK.
//! - [`ip_reputation`] — VPN / proxy / relay classifier trait.
//! - [`breach`] — HIBP k-anonymity breached-credential checker trait.
//! - [`geoip`] *(feature = `geoip`)* — MaxMind GeoLite2 reader.

pub mod breach;
pub mod client_sdk;
pub mod disposable_email;
pub mod fingerprint;
pub mod geo;
pub mod ip_reputation;
pub mod metrics;
pub mod sanctions;

#[cfg(feature = "geoip")]
pub mod geoip;
