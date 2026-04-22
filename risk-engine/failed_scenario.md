# Failed Test Scenario: Clean Login

## Test Case: `full_simulation_summary` (tests/simulation.rs)

### Scenario Description
**Scenario 1: Clean Login**
- **Description:** A clean, low-risk login from a known device with no suspicious activity
- **Context Configuration:** Uses `default_context()` which should represent a standard, low-risk login scenario
- **Expected Behavior:** Decision should be `Allow` with a low score (< 30)

### Context Details (from default_context)
```rust
RiskContext {
    request_id: Uuid::new_v4(),
    evaluated_at: Utc::now(),
    user_id: Uuid::parse_str(TEST_USER_ID).unwrap(),
    username: "test_user".into(),
    credential_id: Some(Uuid::new_v4()),
    action: RiskAction::Login,
    resource_id: None,
    credential_status: CredentialStatus::Active,
    credential_created_at: Utc::now() - Duration::days(30),
    credential_last_used_at: Some(Utc::now() - Duration::hours(12)),
    credential_sign_count_prev: 100,
    credential_sign_count_new: 101,
    credential_registered_ua: Some("Mozilla/5.0 Chrome/120".into()),
    credential_count_for_user: 2,
    prior_audit_event_count: 100,
    last_login_ip: Some(TestIps::home_us()),
    last_login_at: Some(Utc::now() - Duration::hours(24)),
    last_login_geo: Some(TestGeos::new_york()),
    jwt_issued_at: Some(Utc::now() - Duration::minutes(5)),
    jwt_expires_at: Some(Utc::now() + Duration::minutes(55)),
    jwt_fingerprint_stored: Some("test_fingerprint".into()),
    jwt_fingerprint_current: Some("test_fingerprint".into()),
    request_ip: Some(TestIps::home_us()),
    request_ua: Some("Mozilla/5.0 Chrome/120".into()),
    request_geo: Some(TestGeos::new_york()),
    ip_asn_type: Some(AsnType::Residential),
    login_attempts_5m: Some(1),
    failed_login_attempts_1h: Some(0),
    actions_executed_5m: Some(0),
    recovery_requests_24h: Some(0),
    device_revocations_1h: Some(0),
    registrations_from_ip_10m: Some(0),
    active_session_count: Some(1),
    account_locked: false,
    recovery_pending: false,
    oauth_authorize_ip: None,
    nonce_present: true,
    nonce_already_used: false,
    org_risk_level: OrgRiskLevel::Normal,
    redis_signals_degraded: false,
    db_signals_degraded: false,
}
```

### Engine Output
```
1. Clean Login ... ✓ Score: 32, Decision: Challenge
```

### Expected vs Actual
- **Expected Decision:** `Allow` (score should be < 30)
- **Actual Decision:** `Challenge` (score = 32, which is in Challenge range 30-59)
- **Score Breakdown Expected:**
  - Base components (D+S+N+B): 0 (no risk factors)
  - Multipliers: 1.0 (normal action and org level)
  - Absolute adder: 0 (fingerprints match)
  - Final score: 0 → Allow

### Analysis
The engine is producing a score of 32 instead of the expected 0, causing a `Challenge` decision instead of `Allow`. This suggests that some risk factor is being applied that shouldn't be present in a "clean" login scenario.

### Possible Issues
1. **JWT Fingerprint Logic:** Although set to match, there might be a bug in the comparison
2. **Velocity Scoring:** `login_attempts_5m: Some(1)` might be triggering unexpected scoring
3. **Context Field Mismatch:** Some field in the default context is not as expected
4. **Scoring Bug:** The risk calculation logic has an error elevating the score

### Debug Steps
1. Add detailed score breakdown logging to see which component is contributing the 32 points
2. Verify JWT fingerprint matching logic
3. Check velocity scoring for low attempt counts
4. Compare with working individual test scenarios