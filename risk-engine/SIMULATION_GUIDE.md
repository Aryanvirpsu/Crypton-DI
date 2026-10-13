# Risk Engine Simulation Guide

## Overview

This document demonstrates how the **Crypton-DI Risk Engine** evaluates authentication requests and produces risk decisions using concrete test data examples.

---

## Core Formula

```
R = ((D + S + N + B) * M_action * M_org) + A

Where:
  D  = Device trust score       (0–25)
  S  = Session anomaly score    (0–25)
  N  = Network risk score       (0–20)
  B  = Behavioral velocity      (0–20)
  M_action = Action multiplier  (1.0–3.0)
  M_org = Org risk multiplier   (1.0–2.0)
  A = Absolute adder            (0–30)
  Final Score = clamp(0, 100)
```

## Decision Mapping

| Score Range | Decision  | Action                           |
|-------------|-----------|----------------------------------|
| 0–29        | **Allow** | Proceed without friction         |
| 30–49       | **Challenge** | StepUpWebAuthn or ReverifyDevice |
| 50–59       | **Challenge** | ReLogin (higher bracket)         |
| 60–84       | **Hold**  | AdminApproval (202 Accepted)     |
| 85–100      | **Deny**  | Hard block, possible lock        |

---

## Test Scenarios

### Scenario 1: Clean Login ✅

**Input RiskContext:**
```json
{
  "action": "login",
  "credential_status": "Active",
  "credential_age_days": 30,
  "last_login_ip": "1.2.3.4",
  "request_ip": "1.2.3.4",
  "request_country": "US",
  "last_login_country": "US",
  "login_attempts_5m": 1,
  "failed_login_attempts_1h": 0,
  "ip_asn_type": "Residential",
  "jwt_fingerprint_match": true,
  "nonce_already_used": false,
  "redis_signals_degraded": false
}
```

**Score Breakdown:**
- Device (D): 2/25 (normal credential, known device)
- Session (S): 3/25 (valid JWT, matching fingerprint)
- Network (N): 0/20 (residential ISP, same country)
- Behavioral (B): 0/20 (low velocity, 1 attempt)
- Base Score: 5

**Calculations:**
```
Scaled = floor((5) * 1.0 * 1.0) = 5
Final Score = 5 + 0 = 5
```

**Output Decision:**
```
{
  "decision": "Allow",
  "score": 5,
  "required_action": null,
  "triggered_gates": [],
  "triggered_rules": []
}
```

**Explanation:** ✓ User has established pattern, familiar device, normal velocity.

---

### Scenario 2: Brute Force Attack 🚨

**Input RiskContext:**
```json
{
  "action": "login",
  "credential_status": "Active",
  "login_attempts_5m": 15,
  "failed_login_attempts_1h": 10,
  "login_attempts_1h": 20,
  "last_login_ip": "1.2.3.4",
  "request_ip": "5.6.7.8",
  "ip_asn_type": "Datacenter",
  "actions_executed_5m": 0,
  "recovery_requests_24h": 0
}
```

**Score Breakdown:**
- Device (D): 5/25 (some risk)
- Session (S): 5/25 (unusual pattern)
- Network (N): 10/20 (datacenter IP)
- Behavioral (B): 20/20 (15 attempts in 5m = MAXIMUM)
- Base Score: 40

**Calculations:**
```
M_action = 2.0 (Login is sensitive)
Scaled = floor(40 * 2.0 * 1.0) = 80
Final Score = 80 + 0 = 80
```

**Output Decision:**
```
{
  "decision": "Hold",
  "score": 80,
  "required_action": "AdminApproval",
  "triggered_gates": [],
  "triggered_rules": [
    "velocity_login_5m_threshold",
    "datacenter_ip_detected"
  ]
}
```

**Explanation:** ⚠️ Extreme velocity + unusual network = hold for admin review.

---

### Scenario 3: New Credential + Sensitive Action ❌

**Input RiskContext:**
```json
{
  "action": "ActionExecute",
  "action_name": "add_admin",
  "credential_created_at": "2026-04-11T10:02:00Z",
  "now": "2026-04-11T10:04:00Z",
  "credential_count_for_user": 1,
  "prior_audit_event_count": 0,
  "credential_status": "Active"
}
```

**Score Breakdown:**
- Device (D): 25/25 (brand new credential)
- Session (S): 10/25 (no history)
- Network (N): 5/20 (normal)
- Behavioral (B): 0/20 (zero violations)
- Base Score: 40

**Calculations:**
```
M_action = 3.0 (ActionExecute is highly sensitive)
Scaled = floor(40 * 3.0 * 1.0) = 120, clamped to 255
Final Score = min(120 + 0, 100) = 100 (clamped)

HARD GATE FIRES: "New credential + privileged action"
```

**Output Decision:**
```
{
  "decision": "Deny",
  "score": 100,
  "required_action": null,
  "triggered_gates": ["new_cred_privileged_action"],
  "lock_account": false,
  "hold_reason": "New credential cannot immediately execute privileged actions"
}
```

**Explanation:** ❌ Immediate DENY via hard gate. No new credential can escalate privileges.

---

### Scenario 4: Tor Exit Node on Sensitive Action ❌

**Input RiskContext:**
```json
{
  "action": "DeviceRevoke",
  "ip_asn_type": "Tor",
  "last_login_ip": "1.2.3.4",
  "request_ip": "9.10.11.12",
  "nonce_present": true,
  "nonce_already_used": false
}
```

**Score Breakdown:**
- (Not computed; hard gate fires first)

**Policy Gate:**
```
HARD_GATE = "Tor + sensitive_action"
Condition: ip_asn_type == Tor AND action in [DeviceRevoke, RecoveryStart, ...]
Result: IMMEDIATE DENY
```

**Output Decision:**
```
{
  "decision": "Deny",
  "score": 100,
  "required_action": null,
  "triggered_gates": ["tor_sensitive_action"],
  "lock_account": false,
  "hold_reason": "Tor exit node attempting sensitive action"
}
```

**Explanation:** ❌ Policy blocks Tor + sensitive because revocation is account-critical.

---

### Scenario 5: VPN + Geographic Anomaly ⚠️

**Input RiskContext:**
```json
{
  "action": "Login",
  "credential_status": "Active",
  "ip_asn_type": "Vpn",
  "request_geo": {
    "country": "AU",
    "city": "Sydney",
    "lat": -33.8688,
    "lon": 151.2093
  },
  "last_login_geo": {
    "country": "US",
    "city": "New York",
    "lat": 40.7128,
    "lon": -74.0060
  },
  "last_login_at": "2026-04-10T14:00:00Z",
  "evaluated_at": "2026-04-11T10:00:00Z",
  "login_attempts_5m": 1
}
```

**Score Breakdown:**
- Device (D): 3/25 (normal)
- Session (S): 8/25 (geo anomaly: 11,000 km jump + VPN)
- Network (N): 15/20 (VPN detected)
- Behavioral (B): 0/20 (low velocity)
- Base Score: 26

**Calculations:**
```
M_action = 2.0 (Login)
Scaled = floor(26 * 2.0 * 1.0) = 52
Final Score = 52 + 0 = 52
```

**Output Decision:**
```
{
  "decision": "Challenge",
  "score": 52,
  "required_action": "ReLogin",
  "triggered_rules": [
    "geographic_anomaly_detected",
    "vpn_on_login"
  ]
}
```

**Explanation:** ⚠️ Geographic jump + VPN = suspicious enough to require ReLogin.

---

### Scenario 6: Revoked Credential ❌

**Input RiskContext:**
```json
{
  "action": "Login",
  "credential_status": "Revoked",
  "credential_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Policy Gate:**
```
HARD_GATE = "revoked_credential"
Condition: credential_status == Revoked
Result: IMMEDIATE DENY
```

**Output Decision:**
```
{
  "decision": "Deny",
  "score": 100,
  "required_action": null,
  "triggered_gates": ["revoked_credential"],
  "lock_account": false,
  "hold_reason": "Credential has been revoked"
}
```

**Explanation:** ❌ No authentication with revoked credentials, ever.

---

### Scenario 7: Redis Degraded + Sensitive Action ⚠️

**Input RiskContext:**
```json
{
  "action": "DeviceRevoke",
  "redis_signals_degraded": true,
  "db_signals_degraded": false,
  "credential_status": "Active",
  "last_login_at": "2026-04-10T14:00:00Z"
}
```

**Score Computation:**
```
Base signals available: device, network (partial)
Missing signals: Redis velocity, JWT fingerprint, nonce check

Policy: On degraded + sensitive → force CHALLENGE
(Reason: cannot verify full context, so require step-up)
```

**Output Decision:**
```
{
  "decision": "Challenge",
  "score": 35,
  "required_action": "StepUpWebAuthn",
  "signals_degraded": true,
  "signals_missing": ["velocity_counters", "jwt_fingerprint_store"],
  "hold_reason": "Redis service unavailable; escalating to WebAuthn"
}
```

**Explanation:** ⚠️ System degrades gracefully: still challenge, but allow if WebAuthn passes.

---

### Scenario 8: Nonce Replay Attack ❌

**Input RiskContext:**
```json
{
  "action": "Login",
  "nonce": "abc123def456",
  "nonce_present": true,
  "nonce_already_used": true
}
```

**Policy Gate:**
```
HARD_GATE = "nonce_replay"
Condition: nonce_already_used == true
Result: IMMEDIATE DENY
```

**Output Decision:**
```
{
  "decision": "Deny",
  "score": 100,
  "required_action": null,
  "triggered_gates": ["nonce_replay"],
  "lock_account": true,
  "hold_reason": "Nonce replay detected – possible account compromise"
}
```

**Explanation:** ❌ Replay attack = instant lock. This nonce was already consumed.

---

### Scenario 9: Recovery Complete Without Pending ❌

**Input RiskContext:**
```json
{
  "action": "RecoveryComplete",
  "recovery_pending": false,
  "recovery_requests_24h": 0
}
```

**Policy Gate:**
```
HARD_GATE = "recovery_without_pending"
Condition: action == RecoveryComplete AND recovery_pending == false
Result: IMMEDIATE DENY
```

**Output Decision:**
```
{
  "decision": "Deny",
  "score": 100,
  "required_action": null,
  "triggered_gates": ["recovery_not_initiated"],
  "hold_reason": "Attempted recovery without active recovery request"
}
```

**Explanation:** ❌ Cannot complete a recovery that was never requested.

---

### Scenario 10: JWT Fingerprint Mismatch ⚠️

**Input RiskContext:**
```json
{
  "action": "DeviceRevoke",
  "credential_status": "Active",
  "jwt_fingerprint_stored": "aabbcc1122334455",
  "jwt_fingerprint_current": "ddeeff6677889900",
  "jwt_issued_at": "2026-04-11T09:00:00Z",
  "jwt_expires_at": "2026-04-11T10:00:00Z",
  "nonce_already_used": false
}
```

**Score Breakdown:**
- Device (D): 5/25 (normal)
- Session (S): 25/25 (fingerprint mismatch = MAX SESSION RISK)
- Network (N): 0/20 (normal)
- Behavioral (B): 0/20 (normal)
- Base Score: 30

**Calculations:**
```
M_action = 2.0 (DeviceRevoke is sensitive)
Scaled = floor(30 * 2.0 * 1.0) = 60
Final Score = 60 + 0 = 60
```

**Output Decision:**
```
{
  "decision": "Hold",
  "score": 60,
  "required_action": "AdminApproval",
  "triggered_rules": ["jwt_fingerprint_mismatch"],
  "hold_reason": "JWT fingerprint mismatch – possible token hijacking?"
}
```

**Explanation:** ⚠️ Possible token hijacking. Hold for admin review.

---

## Running the Simulation

### Option 1: Run All Tests

```bash
cd risk-engine
cargo test --test simulation full_simulation_summary -- --nocapture
```

Output shows 10 scenarios with pass/fail results:

```
✓ Score: 5, Decision: Allow
✓ Score: 80, Decision: Hold (BLOCKING)
✓ Score: 100, Decision: Deny
✓ Score: 100, Decision: Deny (POLICY GATE)
...
```

### Option 2: Run Individual Scenario Tests

```bash
cargo test --test simulation sim_scenario_1_clean_login -- --nocapture
cargo test --test simulation sim_scenario_2_brute_force -- --nocapture
# ... etc
```

### Option 3: Examine the Test Code

See [tests/simulation.rs](../tests/simulation.rs) for the full mock store implementation and context builders.

---

## Key Insights

### 1. Hard Gates Fire First
If **any** hard gate matches, the result is **Deny (score 100)** immediately:
- Revoked credential
- Tor on sensitive action
- Nonce replay
- Recovery without pending
- New credential on privileged action

### 2. Scoring is Deterministic
Every score is traceable:
- Device trust, session anomaly, network risk, behavioral velocity are independent
- Multipliers are applied based on action sensitivity and org risk
- No randomness, no ML — pure rules

### 3. Graceful Degradation
If Redis is unavailable:
- On **sensitive** actions: force CHALLENGE (deny if user fails WebAuthn)
- On **non-sensitive** actions: allow if all device/network checks pass

### 4. Escalation Memory
Recent decisions influence current decision:
- 3 recent CHALLENGEs → escalate to HOLD
- Repeated violations → escalate to DENY

### 5. Org-Graph Multiplier
When organization is under attack (org_risk_level = UnderAttack):
- M_org becomes 2.0 (vs. 1.0 normal)
- All scores are doubled, thresholds shift

---

## Adding Custom Scenarios

To add your own test scenario, modify [tests/simulation.rs](../tests/simulation.rs):

```rust
#[tokio::test]
async fn sim_scenario_11_custom() {
    let mut ctx = default_context();
    ctx.your_field = some_value;
    
    let store = MockStore::default();
    let decision = evaluate(ctx, &store).await;
    
    println!("Score: {}, Decision: {}", decision.score, decision.decision);
    assert_eq!(decision.decision, Decision::YourExpectedDecision);
}
```

---

## Architecture Diagram

```
Request (RiskContext)
        ↓
┌───────────────────────────────────────────┐
│  Step 1: evaluate_hard_gates()            │ Fast path
│  ❌ Match? → DENY(100)                    │ (immediate)
└───────────────────────────────────────────┘
        ↓ (no match)
┌───────────────────────────────────────────┐
│  Step 2: compute_score()                  │
│  D = device_score(ctx)    [0..25]         │
│  S = session_score(ctx)   [0..25]         │
│  N = network_score(ctx)   [0..20]         │
│  B = velocity_score(ctx)  [0..20]         │
│  Base = D + S + N + B     [0..90]         │
│  Scaled = Base * M_action * M_org         │
│  Final = min(Scaled + A, 100)             │
└───────────────────────────────────────────┘
        ↓
┌───────────────────────────────────────────┐
│  Step 3: score_to_decision()              │
│  0–29   → Allow                           │
│  30–49  → Challenge (StepUpWebAuthn)     │
│  50–59  → Challenge (ReLogin)            │
│  60–84  → Hold (AdminApproval)           │
│  85–100 → Deny                           │
└───────────────────────────────────────────┘
        ↓
┌───────────────────────────────────────────┐
│  Step 4: apply_escalation_memory()        │
│  recent_decisions[3x Challenge] → Hold    │
│  recent_decisions[3x Hold] → Deny         │
└───────────────────────────────────────────┘
        ↓
       RiskDecision (with score, action, gates, rules)
```

---

## Summary

The **Crypton-DI Risk Engine**:
- ✅ Evaluates 82 input fields
- ✅ Fires 11+ hard policy gates
- ✅ Computes 4-component risk score
- ✅ Maps to 5 decision levels (Allow, 2× Challenge types, Hold, Deny)
- ✅ Applies escalation memory
- ✅ Fails open on service degradation
- ✅ **Zero external dependencies** — fully standalone

See [log.txt](../log.txt) for the complete technical reference.
