use uuid::Uuid;

/// Redis key for a pending WebAuthn registration challenge.
/// Format: `webauthn:register:challenge:<uuid>`
pub fn challenge_key(challenge_id: Uuid) -> String {
    format!("webauthn:register:challenge:{}", challenge_id)
}

/// Redis key for a pending WebAuthn login challenge.
/// Format: `webauthn:login:challenge:<uuid>`
pub fn login_key(challenge_id: Uuid) -> String {
    format!("webauthn:login:challenge:{}", challenge_id)
}
