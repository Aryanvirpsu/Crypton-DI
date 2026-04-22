use sha2::{Digest, Sha256};
use std::net::IpAddr;

/// Compute a deterministic fingerprint from the IP address and user-agent string.
///
/// `sha256(ip_string || "|" || ua_string)` — lowercase hex.
///
/// This is stored in Redis at JWT issue time (`jwt:fingerprint:{token_hash}`)
/// and compared on each subsequent request carrying that JWT. A mismatch
/// indicates the token may be replayed from a different client.
///
/// Design notes:
/// - We do NOT use the raw JWT as the Redis key (too large, leaks token).
/// - The Redis key is `jwt:fingerprint:{key_hash}` where `key_hash` is
///   `sha256(raw_jwt)`. This is computed by the adapter, not here.
/// - We use IP + UA (not just IP) because mobile users legitimately switch
///   IPs (cell tower handoffs). UA changes mid-session are more anomalous.
pub fn compute_jwt_fingerprint(ip: Option<IpAddr>, ua: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ip.map(|i| i.to_string()).unwrap_or_default().as_bytes());
    hasher.update(b"|");
    hasher.update(ua.unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute the Redis key under which a JWT's fingerprint is stored.
///
/// The raw JWT token is hashed so we never store the bearer token as a key.
pub fn jwt_fingerprint_key(raw_jwt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_jwt.as_bytes());
    format!("jwt:fingerprint:{}", hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn same_inputs_produce_same_fingerprint() {
        let ip = Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
        let ua = Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120");
        let a = compute_jwt_fingerprint(ip, ua);
        let b = compute_jwt_fingerprint(ip, ua);
        assert_eq!(a, b);
    }

    #[test]
    fn different_ip_produces_different_fingerprint() {
        let ua = Some("Mozilla/5.0 Chrome/120");
        let ip1 = Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
        let ip2 = Some(IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)));
        assert_ne!(
            compute_jwt_fingerprint(ip1, ua),
            compute_jwt_fingerprint(ip2, ua)
        );
    }

    #[test]
    fn different_ua_produces_different_fingerprint() {
        let ip = Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));
        assert_ne!(
            compute_jwt_fingerprint(ip, Some("Chrome/120")),
            compute_jwt_fingerprint(ip, Some("Firefox/115"))
        );
    }

    #[test]
    fn jwt_key_is_deterministic() {
        let k1 = jwt_fingerprint_key("eyJhbGci.test.token");
        let k2 = jwt_fingerprint_key("eyJhbGci.test.token");
        assert_eq!(k1, k2);
        assert!(k1.starts_with("jwt:fingerprint:"));
    }
}
