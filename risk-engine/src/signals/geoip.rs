//! # geoip
//!
//! MaxMind GeoIP2 / GeoLite2 City + ASN lookup.
//!
//! Feature-gated behind `geoip` so the core crate stays dependency-free.
//! Enable with:
//!
//! ```toml
//! risk-engine = { version = "0.1", features = ["geoip"] }
//! ```
//!
//! The reader is loaded once at startup from a local `.mmdb` file
//! (GeoLite2-City.mmdb + GeoLite2-ASN.mmdb) and queried synchronously on
//! each request. Lookups are ~5µs on modern hardware — fast enough to run
//! in the hot path without offloading to a thread pool.
//!
//! ## Populates
//!
//! - `RiskContext::request_geo` (lat, lon, country, city)
//! - `RiskContext::last_login_geo`
//! - `RiskContext::ip_asn_type` (via heuristic ASN-name classification)
//!
//! ## Updating the database
//!
//! MaxMind ships a new GeoLite2 release twice a week. A background job on the
//! host service should `mmdb` download + atomic-rename the file, then send a
//! `SIGHUP` (or a hot-reload signal of your choice) which triggers a new
//! [`GeoIpReader::open`] call; swap the `Arc` in your adapter state.

use crate::context::{AsnType, GeoPoint};
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use maxminddb::{geoip2, MaxMindDBError, Reader};

/// Typed error set for GeoIP lookups. Kept separate from [`RiskEngineError`]
/// because the engine itself never constructs one of these — only the host
/// adapter does.
///
/// [`RiskEngineError`]: crate::error::RiskEngineError
#[derive(Debug, thiserror::Error)]
pub enum GeoIpError {
    #[error("geoip database open failed at {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: MaxMindDBError,
    },
    #[error("geoip city database missing — city lookups disabled")]
    NoCityDb,
    #[error("geoip asn database missing — asn lookups disabled")]
    NoAsnDb,
}

/// Thread-safe reader that bundles a City DB and an optional ASN DB. Clone
/// via `Arc` — internally it holds `maxminddb::Reader` which is `Sync`.
///
/// Either database may be absent; in that case the corresponding lookup
/// method returns `None`. Callers that rely on ASN classification should
/// make sure they've provided `asn_db_path`.
pub struct GeoIpReader {
    city: Reader<Vec<u8>>,
    asn: Option<Reader<Vec<u8>>>,
}

impl GeoIpReader {
    /// Open the City database. ASN DB is optional. Both are memory-mapped
    /// via `Reader::open_readfile`, so the process must keep the underlying
    /// files on disk for the reader's lifetime.
    pub fn open(
        city_db_path: impl AsRef<Path>,
        asn_db_path: Option<impl AsRef<Path>>,
    ) -> Result<Self, GeoIpError> {
        let city_path = city_db_path.as_ref().to_path_buf();
        let city = Reader::open_readfile(&city_path).map_err(|source| GeoIpError::Open {
            path: city_path,
            source,
        })?;

        let asn = match asn_db_path {
            Some(p) => {
                let p = p.as_ref().to_path_buf();
                let r = Reader::open_readfile(&p).map_err(|source| GeoIpError::Open {
                    path: p,
                    source,
                })?;
                Some(r)
            }
            None => None,
        };

        Ok(Self { city, asn })
    }

    /// Resolve a `GeoPoint` from an IP. Returns `None` when the IP is absent
    /// from the DB or lacks a location record (e.g., bogon addresses, some
    /// mobile carrier /24s).
    pub fn lookup_geo(&self, ip: IpAddr) -> Option<GeoPoint> {
        let record: geoip2::City = self.city.lookup(ip).ok()?;

        let country_code = record
            .country
            .as_ref()
            .and_then(|c| c.iso_code.map(|s| s.to_string()))?;

        let location = record.location.as_ref()?;
        let lat = location.latitude?;
        let lon = location.longitude?;

        let city = record
            .city
            .as_ref()
            .and_then(|c| c.names.as_ref())
            .and_then(|n| n.get("en").map(|s| s.to_string()));

        Some(GeoPoint {
            lat,
            lon,
            country_code,
            city,
        })
    }

    /// Resolve just the ISO-3166 alpha-2 country code — cheaper than the
    /// full `GeoPoint` because it skips location/city decoding. Useful for
    /// sanctions screening where only the country is needed.
    pub fn lookup_country_code(&self, ip: IpAddr) -> Option<String> {
        let record: geoip2::Country = self.city.lookup(ip).ok()?;
        record.country?.iso_code.map(|s| s.to_string())
    }

    /// Resolve an ASN classification. Requires the ASN database to have been
    /// provided at [`Self::open`] time; otherwise returns `None`.
    ///
    /// The mapping from ASN-org-string to [`AsnType`] is a conservative
    /// heuristic — MaxMind does not ship a field that answers "is this
    /// datacenter" directly. For higher precision, layer an IP-intelligence
    /// feed (see [`super::ip_reputation`]) on top.
    pub fn lookup_asn_type(&self, ip: IpAddr) -> Option<AsnType> {
        let asn_db = self.asn.as_ref()?;
        let record: geoip2::Asn = asn_db.lookup(ip).ok()?;
        let org = record.autonomous_system_organization?;
        Some(classify_asn_org(org))
    }
}

/// Heuristic classifier for ASN organisation strings. Callers get a coarse
/// bucket; attackers can still find gaps, which is why [`AsnType::Unknown`]
/// is a valid answer and the engine does not hard-ban on it.
///
/// Kept as a free function so it can be unit-tested without needing a
/// MaxMind database on disk.
pub fn classify_asn_org(org: &str) -> AsnType {
    let lower = org.to_ascii_lowercase();

    // Tor: typically embedded "tor" as a word — guard against false positives
    // like "motorola" by requiring whitespace or boundary.
    if lower.contains(" tor ") || lower.starts_with("tor ") || lower.ends_with(" tor")
    {
        return AsnType::Tor;
    }

    // Privacy relays (Apple Private Relay, Cloudflare WARP, iCloud).
    if lower.contains("private relay")
        || lower.contains("icloud")
        || lower.contains("cloudflare warp")
    {
        return AsnType::Relay;
    }

    // Commercial VPN tells.
    for vpn_name in [
        "nordvpn",
        "expressvpn",
        "mullvad",
        "protonvpn",
        "surfshark",
        "private internet access",
        "ipvanish",
        "cyberghost",
        "tunnelbear",
        "vpn unlimited",
    ] {
        if lower.contains(vpn_name) {
            return AsnType::Vpn;
        }
    }
    if lower.contains(" vpn") || lower.ends_with(" vpn") || lower.starts_with("vpn ") {
        return AsnType::Vpn;
    }

    // Open proxy / anonymizer.
    if lower.contains("proxy") || lower.contains("anonymizer") {
        return AsnType::Proxy;
    }

    // Major cloud + datacenter.
    for dc in [
        "amazon",
        "aws",
        "microsoft",
        "azure",
        "google cloud",
        "googleusercontent",
        "digitalocean",
        "linode",
        "ovh",
        "hetzner",
        "vultr",
        "oracle cloud",
        "alibaba cloud",
        "contabo",
        "scaleway",
    ] {
        if lower.contains(dc) {
            return AsnType::Datacenter;
        }
    }

    // Generic hosting.
    if lower.contains("hosting") || lower.contains("server") || lower.contains("datacenter") {
        return AsnType::Hosting;
    }

    // Corporate / enterprise — usually lacks the telltales above but can
    // contain "inc", "corp", "llc". Treat as Corporate only when we have a
    // clear tell; otherwise Unknown.
    for corp in ["verizon business", "at&t services", "deutsche telekom business"] {
        if lower.contains(corp) {
            return AsnType::Corporate;
        }
    }

    // ISPs / residential — if nothing else matched, assume residential only
    // when the string contains a known ISP signal.
    for isp in [
        "comcast",
        "spectrum",
        "att internet",
        "verizon fios",
        "virgin media",
        "sky broadband",
        "deutsche telekom",
        "telefonica",
        "orange",
        "bt group",
        "vodafone",
    ] {
        if lower.contains(isp) {
            return AsnType::Residential;
        }
    }

    AsnType::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_tor() {
        assert_eq!(classify_asn_org("Some Tor relay"), AsnType::Tor);
        // Negative: doesn't grab "motorola".
        assert_ne!(classify_asn_org("Motorola Solutions"), AsnType::Tor);
    }

    #[test]
    fn classifies_private_relays() {
        assert_eq!(classify_asn_org("Apple Private Relay"), AsnType::Relay);
        assert_eq!(classify_asn_org("Cloudflare WARP"), AsnType::Relay);
        assert_eq!(
            classify_asn_org("iCloud Private Relay"),
            AsnType::Relay
        );
    }

    #[test]
    fn classifies_commercial_vpns() {
        for s in [
            "NordVPN S.A.",
            "ExpressVPN",
            "Mullvad VPN AB",
            "Proton VPN",
        ] {
            assert_eq!(classify_asn_org(s), AsnType::Vpn, "{s}");
        }
    }

    #[test]
    fn classifies_major_cloud() {
        for s in [
            "Amazon.com, Inc.",
            "Microsoft Corporation",
            "Google Cloud",
            "DigitalOcean, LLC",
            "Hetzner Online GmbH",
            "OVH SAS",
        ] {
            assert_eq!(classify_asn_org(s), AsnType::Datacenter, "{s}");
        }
    }

    #[test]
    fn classifies_residential_isps() {
        for s in ["Comcast Cable Communications", "Virgin Media", "Vodafone"] {
            assert_eq!(classify_asn_org(s), AsnType::Residential, "{s}");
        }
    }

    #[test]
    fn unknown_falls_through() {
        assert_eq!(classify_asn_org(""), AsnType::Unknown);
        assert_eq!(classify_asn_org("Completely Made Up Org"), AsnType::Unknown);
    }
}
