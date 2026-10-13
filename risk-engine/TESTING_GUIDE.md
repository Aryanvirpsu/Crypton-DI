# Risk Engine Testing Module

## Quick Start

The testing module provides a **fluent builder API** and **organized test templates** for testing the risk engine.

### Run All Tests

```bash
cd risk-engine
cargo test --test active_tests -- --nocapture
```

### Run a Specific Category

```bash
# Just login tests
cargo test --test active_tests test_clean_login -- --nocapture

# Just brute force tests
cargo test --test active_tests brute_force -- --nocapture

# Integration tests only
cargo test --test active_tests integration_ -- --nocapture
```

### Run Single Test with Output

```bash
cargo test --test active_tests test_brute_force_high_velocity -- --nocapture
```

---

## Architecture

```
tests/
├── helpers.rs          ← Test utilities (builders, fixtures, assertions)
├── active_tests.rs     ← Main test suite (organized by category)
└── simulation.rs       ← Original simulation examples
```

---

## Using the Fluent Builder

Create test contexts easily with the **fluent builder pattern**:

### Basic Clean Login

```rust
let ctx = RiskContextBuilder::new()
    .action(RiskAction::Login)
    .login_attempts_5m(1)
    .failed_login_attempts_1h(0)
    .build();

let store = TestStore::default();
let decision = evaluate(ctx, &store).await;

assert_eq!(decision.decision, Decision::Allow);
```

### Brute Force Attack

```rust
let ctx = RiskContextBuilder::new()
    .action(RiskAction::Login)
    .login_attempts_5m(15)          // ← many attempts
    .failed_login_attempts_1h(10)   // ← many failures
    .asn_type(AsnType::Datacenter)  // ← suspicious network
    .build();

let decision = evaluate(ctx, &store).await;
assert!(decision.decision.is_blocking());
```

### Geographic Anomaly

```rust
let ctx = RiskContextBuilder::new()
    .request_geo(TestGeos::sydney())           // User now in Sydney
    .last_login_geo(TestGeos::new_york())      // Last login was NYC
    .asn_type(AsnType::Vpn)                    // Using VPN
    .build();

let decision = evaluate(ctx, &store).await;
assert!(decision.score >= 30);  // Should elevate risk
```

### New Privileged Action

```rust
let ctx = RiskContextBuilder::new()
    .action(RiskAction::ActionExecute { 
        action_name: "add_admin".into() 
    })
    .credential_age(Duration::minutes(2))    // ← brand new
    .credential_count(1)                      // ← only credential
    .audit_event_count(0)                     // ← no history
    .build();

let decision = evaluate(ctx, &store).await;
assert_eq!(decision.decision, Decision::Deny);  // Hard gate triggers
```

### Multiple Risk Factors

```rust
let ctx = RiskContextBuilder::new()
    .action(RiskAction::DeviceRevoke)  // sensitive action
    .login_attempts_5m(8)               // moderate velocity
    .asn_type(AsnType::Vpn)             // VPN detected
    .request_ip(TestIps::datacenter())  // datacenter IP
    .jwt_fingerprints(
        "stored_hash",
        "different_hash"                 // fingerprint mismatch
    )
    .build();

let decision = evaluate(ctx, &store).await;
assert!(decision.decision.is_blocking());
```

---

## Built-in Scenarios

Quick access to common test scenarios:

```rust
use helpers::scenarios::*;

// All pre-configured, ready to use:
let ctx = clean_login();                    // ✓ Should Allow
let ctx = brute_force_attack();             // ✓ Should Block
let ctx = new_cred_privileged_action();     // ✓ Should Deny
let ctx = tor_sensitive_action();           // ✓ Should Deny
let ctx = vpn_geo_anomaly();                // ✓ Should Challenge
let ctx = revoked_credential();             // ✓ Should Deny
let ctx = redis_degraded_sensitive();       // ✓ Should Challenge
let ctx = nonce_replay();                   // ✓ Should Deny
let ctx = recovery_not_pending();           // ✓ Should Deny
let ctx = jwt_mismatch();                   // ✓ Should Hold
let ctx = org_under_attack();               // ✓ Score multiplied
```

---

## Assertion Helpers

Simple assertion functions for common patterns:

```rust
use helpers::DecisionAssertions;

// Assert Allow (score < 30)
DecisionAssertions::assert_allow(decision.score, decision.decision);

// Assert Challenge (score 30-59)
DecisionAssertions::assert_challenge(decision.score, decision.decision);

// Assert Hold (score 60-84)
DecisionAssertions::assert_hold(decision.score, decision.decision);

// Assert Deny (score 85-100)
DecisionAssertions::assert_deny(decision.score, decision.decision);

// Assert blocking decision
DecisionAssertions::assert_blocking(decision.score, decision.decision);

// Assert required action is present
DecisionAssertions::assert_has_required_action(&decision);
```

---

## Test Organization by Category

### Category 1: Basic Login Scenarios
Tests normal authentication workflows:
- `test_clean_login_allows()`
- `test_login_with_low_velocity()`
- `test_login_with_moderate_velocity()`

### Category 2: Brute Force & Velocity
Tests high-velocity attack patterns:
- `test_brute_force_high_velocity()`
- `test_brute_force_from_datacenter()`
- `test_excessive_failed_attempts()`

### Category 3: Credential & Policy Gates
Tests credential-based blocks:
- `test_revoked_credential_denies()`
- `test_new_credential_privileged_action()`
- `test_lost_credential_denies()`

### Category 4: Network & Geographic
Tests network and location-based risks:
- `test_tor_exit_sensitive_action_denies()`
- `test_vpn_with_geo_anomaly()`
- `test_same_ip_same_geo_low_risk()`
- `test_impossible_travel_scenario()`

### Category 5: Session & JWT
Tests session and token validation:
- `test_jwt_fingerprint_mismatch_holds()`
- `test_nonce_replay_denies()`

### Category 6: Recovery & Account Management
Tests account recovery flows:
- `test_recovery_without_pending_denies()`
- `test_device_revoke_low_risk()`

### Category 7: Service Degradation
Tests behavior when services are down:
- `test_redis_degraded_sensitive_action_challenges()`
- `test_redis_degraded_non_sensitive_allows()`

### Category 8: Organization Risk
Tests org-level risk multiplication:
- `test_org_under_attack_multiplies_score()`

### Category 9: Builder Tests
Tests the context builder API:
- `test_builder_fluent_api()`
- `test_builder_high_risk_scenario()`

### Category 10: Score Analysis
Tests scoring structure and breakdown:
- `test_score_breakdown_structure()`
- `test_decision_has_factors()`

### Integration Tests
Tests complex multi-factor scenarios:
- `integration_normal_user_flow()`
- `integration_suspicious_to_attack()`

---

## Test Fixtures

Use built-in test fixtures for consistency:

```rust
use helpers::{TestIps, TestGeos};

// Standard test IPs
let home_us = TestIps::home_us();          // 1.2.3.4 (residential)
let home_au = TestIps::home_au();          // 203.0.113.1
let datacenter = TestIps::datacenter();    // 54.239.28.30
let tor_exit = TestIps::tor_exit();        // 5.135.176.65

// Standard locations
let ny = TestGeos::new_york();             // NYC
let sydney = TestGeos::sydney();           // Sydney
let london = TestGeos::london();           // London
let moscow = TestGeos::moscow();           // Moscow
```

---

## Writing Custom Tests

### Template 1: Simple Decision Assertion

```rust
#[tokio::test]
async fn test_my_scenario() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .login_attempts_5m(5)
        .build();
    
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    assert_eq!(decision.decision, Decision::Challenge);
}
```

### Template 2: Score Range Assertion

```rust
#[tokio::test]
async fn test_my_scoring() {
    let ctx = RiskContextBuilder::new()
        .login_attempts_5m(8)
        .asn_type(AsnType::Vpn)
        .build();
    
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    assert!(decision.score >= 30 && decision.score <= 59);
}
```

### Template 3: Multiple Assertions

```rust
#[tokio::test]
async fn test_my_complex_scenario() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::DeviceRevoke)
        .jwt_fingerprints("aabbcc", "ddeeff")  // mismatch
        .asn_type(AsnType::Vpn)
        .request_geo(TestGeos::moscow())
        .last_login_geo(TestGeos::london())
        .build();
    
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Check decision
    assert!(decision.decision.is_blocking());
    
    // Check required action
    DecisionAssertions::assert_has_required_action(&decision);
    
    // Check factors are documented
    assert!(!decision.factors.is_empty());
}
```

---

## Running Tests in CI/CD

### Run All Tests with Coverage

```bash
cargo test --test active_tests --test engine_integration -- --test-threads=1
```

### Run Specific Test Groups

```bash
# All blocking tests
cargo test --test active_tests -- denies | challenges | holds | blocking

# All integration tests
cargo test --test active_tests integration_

# Performance: run once
cargo test --test active_tests -- --nocapture --test-threads=1
```

### Generate Test Report

```bash
cargo test --test active_tests -- --nocapture 2>&1 | tee test_report.txt
```

---

## Builder API Reference

### Context Builder Methods

| Method | Purpose | Example |
|--------|---------|---------|
| `.new()` | Create builder with defaults | `RiskContextBuilder::new()` |
| `.action(a)` | Set the action | `.action(RiskAction::Login)` |
| `.credential_status(s)` | Set credential state | `.credential_status(CredentialStatus::Active)` |
| `.credential_age(d)` | Set credential age | `.credential_age(Duration::days(30))` |
| `.login_attempts_5m(n)` | Set login attempts | `.login_attempts_5m(15)` |
| `.failed_login_attempts_1h(n)` | Set failed attempts | `.failed_login_attempts_1h(10)` |
| `.asn_type(t)` | Set network type | `.asn_type(AsnType::Tor)` |
| `.request_ip(ip)` | Set request IP | `.request_ip(TestIps::home_us())` |
| `.request_geo(g)` | Set request location | `.request_geo(TestGeos::sydney())` |
| `.jwt_fingerprints(s, c)` | Set JWT fingerprints | `.jwt_fingerprints("stored", "current")` |
| `.nonce_replayed()` | Mark nonce as replayed | `.nonce_replayed()` |
| `.redis_degraded()` | Mark Redis down | `.redis_degraded()` |
| `.org_risk_level(l)` | Set org risk | `.org_risk_level(OrgRiskLevel::UnderAttack)` |
| `.build()` | Finalize context | `.build()` |

---

## Common Issues & Solutions

### Issue: Test Fails on Score Range

**Problem:** Your test expects score 30-49 but gets 50.

**Solution:** Check if multipliers changed:
```rust
// Print score breakdown
println!("Score breakdown: {:#?}", decision.score_breakdown);
println!("Triggered rules: {:?}", decision.triggered_rules);
```

### Issue: Policy Gate Fires Unexpectedly

**Problem:** Test expects Challenge but gets Deny.

**Solution:** Check if hard gate is triggering:
```rust
println!("Triggered gates: {:?}", decision.triggered_gates);
```

### Issue: Decision Components Missing

**Problem:** `decision.factors` is empty.

**Solution:** Ensure the decision is not from a hard gate (those don't have factors).

---

## Examples: Real-World Test Cases

### Test: User with Stolen Credential

```rust
#[tokio::test]
async fn test_stolen_credential_detection() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .jwt_fingerprints("original_device", "different_device")
        .request_ip(TestIps::datacenter())
        .last_login_ip(TestIps::home_us())
        .request_geo(TestGeos::moscow())
        .last_login_geo(TestGeos::new_york())
        .login_attempts_5m(8)
        .asn_type(AsnType::Vpn)
        .build();
    
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    assert!(decision.decision.is_blocking());
    assert_eq!(decision.score, 100);  // Multiple red flags
}
```

### Test: Legitimate User from Business Trip

```rust
#[tokio::test]
async fn test_business_trip_geo_change() {
    let ctx = RiskContextBuilder::new()
        .action(RiskAction::Login)
        .credential_age(Duration::days(180))  // Established credential
        .login_attempts_5m(1)                 // Normal velocity
        .request_geo(TestGeos::london())
        .last_login_geo(TestGeos::new_york())
        .asn_type(AsnType::Residential)       // Normal ISP
        .jwt_fingerprints("same_device", "same_device")  // Same device
        .build();
    
    let store = TestStore::default();
    let decision = evaluate(ctx, &store).await;
    
    // Should allow - established user, normal behavior
    assert_eq!(decision.decision, Decision::Allow);
}
```

---

## Next Steps

1. **Study existing tests** - Read through `active_tests.rs` to understand patterns
2. **Add custom tests** - Follow the templates to add your own scenarios
3. **Use builders** - The fluent API makes it easy to create realistic contexts
4. **Run frequently** - Use `cargo test` during development to catch regressions
5. **Iterate** - Adjust test thresholds based on your actual scoring

---

## Performance Tips

- Run tests with `--test-threads=1` for deterministic output
- Use `--nocapture` to see println! statements
- Use `#[ignore]` for slow tests during development
- Create separate test modules for different test families

```rust
#[ignore]
#[tokio::test]
async fn test_slow_scenario() {
    // ...
}

// Run only non-ignored tests:
// cargo test --test active_tests -- --skip slow
```

---

## Summary

This testing module provides:

✅ **Fluent builder** for easy context creation  
✅ **Pre-built scenarios** for common cases  
✅ **Assertion helpers** for consistent checks  
✅ **10 test categories** covering all major scenarios  
✅ **Integration tests** for complex workflows  
✅ **Fixtures** for IPs, geos, and timestamps  
✅ **Easy extensibility** for custom tests  

Ready to test! 🚀
