use uuid::Uuid;

/// Redis key for a pending WebAuthn registration challenge.
/// Format: `webauthn:register:challenge:<uuid>`
pub fn challenge_key(challenge_id: Uuid) -> String {
    format!("webauthn:register:challenge:{}", challenge_id)
}
