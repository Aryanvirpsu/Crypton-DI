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

/// Key for a protected-action challenge (separate namespace from login).
/// Format: `webauthn:action:challenge:<uuid>`
pub fn action_key(challenge_id: Uuid) -> String {
    format!("webauthn:action:challenge:{}", challenge_id)
}

/// Key for a recovery-authorized device enrollment grant.
/// Format: `webauthn:recovery:enroll:<token>`
pub fn recovery_enrollment_key(token: &str) -> String {
    format!("webauthn:recovery:enroll:{}", token)
}
