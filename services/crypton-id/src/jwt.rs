use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sqlx::{self, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};
use sqlx::PgPool;

/// Key-id, carried in every token's header — not yet used to select
/// between multiple keys (there is exactly one signing key today), but
/// present from the start so a future key rotation has a seam to key off
/// of instead of needing to add one retroactively. See DECISIONS.md.
pub const JWT_KID: &str = "v1";

// ── Claims ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub cred_id: String, // credential/device uuid
    pub iat: u64,
    pub exp: u64,
}

pub async fn verify_device_active(
    pool: &PgPool,
    cred_uuid: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row = sqlx::query("SELECT status FROM credentials WHERE id = $1 AND user_id = $2")
        .bind(cred_uuid)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| AppError::unauthorized("device_not_active"))?;

    let active = row
        .and_then(|r| r.try_get::<String, _>("status").ok())
        .map(|s| s == "active")
        .unwrap_or(false);

    if !active {
        return Err(AppError::unauthorized("device_not_active"));
    }
    Ok(())
}

pub fn issue_jwt(
    user_id: Uuid,
    username: &str,
    credential_id: Uuid,
    signing_key: &EncodingKey,
) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(e.to_string()))?
        .as_secs();

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_owned(),
        cred_id: credential_id.to_string(),
        iat: now,
        exp: now + 3600, // 1 hour
    };

    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(JWT_KID.to_string());

    encode(&header, &claims, signing_key)
        .map_err(|e| AppError::internal(format!("jwt_encode_error: {e}")))
}

/// The single verification choke point every call site in this service
/// must go through — `AuthUser`'s extractor, the optional-auth helper in
/// auth_webauthn.rs, `logout`'s re-decode, and oauth.rs's bearer-token
/// check all delegate here instead of each calling `jsonwebtoken::decode`
/// with their own copy of the algorithm/key. One place to get the
/// algorithm and key right is what makes the Ed25519 migration (and any
/// future one) a one-site change instead of an easy-to-miss four-site one.
///
/// Only ever needs `state.jwt_decoding_key` — the public half. Nothing
/// reachable from this function can mint a token; see
/// `services/crypton-id/src/routes/auth_webauthn.rs`'s login handler
/// (the only production call site for `issue_jwt`) for the one place that
/// can.
pub fn verify_token(state: &AppState, token: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::EdDSA);
    // jsonwebtoken's default Validation also accepts HS256; pin explicitly
    // to EdDSA so a token that somehow carries a different `alg` header
    // (e.g. a pre-migration HS256 token, or an attacker-crafted `alg:
    // none`) is rejected outright rather than silently attempted.
    validation.algorithms = vec![Algorithm::EdDSA];

    decode::<Claims>(token, &state.jwt_decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::unauthorized(format!("invalid_token: {e}")))
}

// ── AuthUser extractor ────────────────────────────────────────────────────────
// Use this as a handler parameter on any route that requires authentication.
// Returns 401 if the header is missing, malformed, or the token is invalid/expired.

pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
    pub cred_id: Uuid,
}

pub struct AdminUser {
    pub user_id: Uuid,
    pub username: String,
    pub cred_id: Uuid,
    pub role: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("missing_authorization_header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::unauthorized("invalid_authorization_scheme"))?;

        let claims = verify_token(state, token)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::unauthorized("malformed_subject_claim"))?;

        let cred_uuid = Uuid::parse_str(&claims.cred_id)
            .map_err(|_| AppError::unauthorized("malformed_cred_id"))?;

        // Check JWT denylist (logout invalidation) before any DB work.
        if let Ok(mut conn) = state.redis_conn().await {
            if let Ok(true) =
                crate::redis::sessions::is_denied(&mut conn, &claims.sub, claims.iat).await
            {
                return Err(AppError::unauthorized("session_invalidated"));
            }
        }

        // If a DB pool is available, verify the credential is still active.
        // Revoked and not-found are both reported as the same 401 to avoid
        // leaking whether a device was revoked vs never existed.
        if let Some(ref pool) = state.db {
            verify_device_active(pool, cred_uuid, user_id).await?;
        }

        Ok(AuthUser {
            user_id,
            username: claims.username,
            cred_id: cred_uuid,
        })
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        let db = state
            .db
            .as_ref()
            .ok_or_else(|| AppError::unauthorized("admin_authorization_required"))?;

        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(auth.user_id)
            .fetch_optional(db)
            .await
            .map_err(|_| AppError::unauthorized("admin_authorization_required"))?;

        let role = role.ok_or_else(|| AppError::forbidden("admin_authorization_required"))?;
        if role != "admin" && role != "super_admin" {
            return Err(AppError::forbidden("admin_authorization_required"));
        }

        Ok(AdminUser {
            user_id: auth.user_id,
            username: auth.username,
            cred_id: auth.cred_id,
            role,
        })
    }
}

/// Fixed, non-secret Ed25519 test keypairs — generated once with
/// `openssl genpkey -algorithm ed25519` and committed here deliberately
/// (never used for anything but `cargo test`). Shared by this module's own
/// tests and by the `test_state()` fixtures in routes/devices.rs,
/// routes/admin.rs, and routes/recovery.rs, so there is exactly one place
/// a test keypair is defined, not four.
#[cfg(test)]
pub mod test_keys {
    pub const SIGNING_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIPCsvnOOwpwO3ny53kAPCm5XlSE5EGIvrgZbm/+R9Wn2\n-----END PRIVATE KEY-----\n";
    pub const PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEAh206LonzWwDNeN7FFpFDgkFBuXf/NqtYZulHoqvWIt8=\n-----END PUBLIC KEY-----\n";

    /// A second, unrelated keypair — for proving a token signed by the
    /// wrong key (or a caller who only has this public key) cannot pass
    /// verification against `PUBLIC_KEY_PEM`.
    pub const OTHER_SIGNING_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIDFfh6j6XSEabG1I0g5ZYbnicE9hTC4SI0nTDJ/N1Vyi\n-----END PRIVATE KEY-----\n";
    pub const OTHER_PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----\nMCowBQYDK2VwAyEALCtWDmj1os2qAPBbDOPBEAX97jclvjNWkNlVXJTkJHA=\n-----END PUBLIC KEY-----\n";
}

// ── Unit tests ────────────────────────────────────────────────────────────────
// Pure JWT issue/verify/expiry logic — no DB or Redis required, so these run
// anywhere `cargo test` runs, including plain `cargo test --lib` with no
// services up.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jwt::test_keys::*;
    use jsonwebtoken::DecodingKey;

    fn signing_key() -> EncodingKey {
        EncodingKey::from_ed_pem(SIGNING_KEY_PEM.as_bytes()).unwrap()
    }

    fn decoding_key() -> DecodingKey {
        DecodingKey::from_ed_pem(PUBLIC_KEY_PEM.as_bytes()).unwrap()
    }

    fn eddsa_validation() -> Validation {
        Validation::new(Algorithm::EdDSA)
    }

    #[test]
    fn issue_jwt_roundtrips_claims() {
        let user_id = Uuid::new_v4();
        let cred_id = Uuid::new_v4();
        let token = issue_jwt(user_id, "alice", cred_id, &signing_key()).unwrap();

        let data = decode::<Claims>(&token, &decoding_key(), &eddsa_validation())
            .expect("token should decode with the matching public key");

        assert_eq!(data.claims.sub, user_id.to_string());
        assert_eq!(data.claims.username, "alice");
        assert_eq!(data.claims.cred_id, cred_id.to_string());
        assert_eq!(data.claims.exp, data.claims.iat + 3600);
        assert_eq!(data.header.alg, Algorithm::EdDSA);
        assert_eq!(data.header.kid.as_deref(), Some(JWT_KID));
    }

    #[test]
    fn decode_fails_with_wrong_public_key() {
        let token = issue_jwt(Uuid::new_v4(), "bob", Uuid::new_v4(), &signing_key()).unwrap();

        let wrong_key = DecodingKey::from_ed_pem(OTHER_PUBLIC_KEY_PEM.as_bytes()).unwrap();
        let result = decode::<Claims>(&token, &wrong_key, &eddsa_validation());

        assert!(
            result.is_err(),
            "a token signed by one keypair must not verify against a different keypair's public key"
        );
    }

    /// Core TD-1 property (see DECISIONS.md): a verifier holding only a
    /// public key must be structurally unable to mint a token that the
    /// *real* public key will accept — not just "produces a token that
    /// happens to fail verification," but "cannot even construct a
    /// signing key from public material in the first place."
    #[test]
    fn public_key_pem_cannot_be_used_as_a_signing_key() {
        let attempted_signing_key = EncodingKey::from_ed_pem(PUBLIC_KEY_PEM.as_bytes());
        assert!(
            attempted_signing_key.is_err(),
            "an Ed25519 SPKI public key must not parse as a PKCS8 signing key — \
             a verifier holding only PUBLIC_KEY_PEM has no way to mint a valid token"
        );
    }

    /// A token forged with a *different* keypair's private key (the
    /// closest a verifier-only party could get to "minting" something)
    /// still fails against the real public key — the end-to-end version
    /// of the property above.
    #[test]
    fn token_signed_by_a_different_private_key_is_rejected() {
        let forged_signing_key =
            EncodingKey::from_ed_pem(OTHER_SIGNING_KEY_PEM.as_bytes()).unwrap();
        let forged_token = issue_jwt(
            Uuid::new_v4(),
            "attacker",
            Uuid::new_v4(),
            &forged_signing_key,
        )
        .unwrap();

        let result = decode::<Claims>(&forged_token, &decoding_key(), &eddsa_validation());
        assert!(
            result.is_err(),
            "a forged token signed by a different key must be rejected"
        );
    }

    #[test]
    fn tampered_claims_are_rejected() {
        let token = issue_jwt(Uuid::new_v4(), "alice", Uuid::new_v4(), &signing_key()).unwrap();
        // Flip one character in the payload segment — same shape a
        // bit-flip or a naive "edit the JSON and re-base64" attack would
        // produce; the signature no longer matches the (now different)
        // payload.
        let mut parts: Vec<&str> = token.split('.').collect();
        let mut payload = parts[1].to_string();
        let last = payload.pop().unwrap();
        payload.push(if last == 'A' { 'B' } else { 'A' });
        parts[1] = &payload;
        let tampered = parts.join(".");

        let result = decode::<Claims>(&tampered, &decoding_key(), &eddsa_validation());
        assert!(
            result.is_err(),
            "a tampered token must fail signature verification"
        );
    }

    #[test]
    fn decode_rejects_expired_token() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Hand-build claims that expired well outside jsonwebtoken's default
        // 60s leeway — issue_jwt always mints exp = iat + 3600, so we bypass
        // it here to exercise the expiry check itself. (A first version of
        // this test used `exp: now - 10`, which is INSIDE the default 60s
        // leeway and therefore validates successfully — that was a bug in
        // the test, not the code; caught by actually running `cargo test`.)
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            username: "carol".into(),
            cred_id: Uuid::new_v4().to_string(),
            iat: now - 3720,
            exp: now - 120,
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(JWT_KID.to_string());
        let token = encode(&header, &claims, &signing_key()).unwrap();

        let result = decode::<Claims>(&token, &decoding_key(), &eddsa_validation());

        assert!(result.is_err(), "an expired token must fail validation");
        assert_eq!(
            result.unwrap_err().kind(),
            &jsonwebtoken::errors::ErrorKind::ExpiredSignature
        );
    }

    #[test]
    fn decode_accepts_token_at_the_expiry_boundary() {
        // jsonwebtoken's default Validation allows a small leeway (60s), so
        // a token that expired a couple of seconds ago should still pass —
        // this pins that documented behaviour rather than assuming it.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            username: "dave".into(),
            cred_id: Uuid::new_v4().to_string(),
            iat: now - 3602,
            exp: now - 2,
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(JWT_KID.to_string());
        let token = encode(&header, &claims, &signing_key()).unwrap();

        let result = decode::<Claims>(&token, &decoding_key(), &eddsa_validation());

        assert!(
            result.is_ok(),
            "default 60s leeway should cover a 2s-expired token"
        );
    }
}
