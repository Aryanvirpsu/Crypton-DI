//! # Risk Engine Playground
//!
//! This is an interactive simulation tool designed to demonstrate how the risk engine
//! evaluates different scenarios based on the constants in `src/config.rs`.
//!
//! Run this with: `cargo test --test playground -- --nocapture`

mod helpers;

use async_trait::async_trait;
use risk_engine::{
    evaluate,
    context::RiskContext,
    decision::{Decision, ScoreComponent, RiskDecision},
    store::{SignalStore, EscalationEntry, VelocityCounters},
    RiskEngineError,
};
use helpers::scenarios;
use uuid::Uuid;
use std::net::IpAddr;

// ─────────────────────────────────────────────────────────────────────────────
// Simple Mocks for Playground
// ─────────────────────────────────────────────────────────────────────────────

struct MockStore;

#[async_trait]
impl SignalStore for MockStore {
    async fn get_velocity_counters(&self, _u: Uuid, _ip: Option<IpAddr>) -> Result<VelocityCounters, RiskEngineError> { 
        Ok(VelocityCounters::default()) 
    }
    async fn get_recent_decisions(&self, _u: Uuid) -> Result<Vec<EscalationEntry>, RiskEngineError> { 
        Ok(vec![]) 
    }
    async fn record_decision(&self, _u: Uuid, _e: EscalationEntry) -> Result<(), RiskEngineError> { 
        Ok(()) 
    }
    async fn lock_account(&self, _u: Uuid, _t: u64) -> Result<(), RiskEngineError> {
        Ok(())
    }
    async fn block_ip(&self, _ip: IpAddr, _t: u64) -> Result<(), RiskEngineError> {
        Ok(())
    }
    async fn check_and_record_escalation(
        &self,
        _u: Uuid,
        current: &Decision,
        _s: u8,
        _ts: i64,
    ) -> Result<(Decision, bool), RiskEngineError> {
        Ok((current.clone(), false))
    }
    async fn get_jwt_fingerprint(&self, _k: &str) -> Result<Option<String>, RiskEngineError> {
        Ok(None) 
    }
    async fn set_jwt_fingerprint(&self, _k: &str, _v: &str, _t: u64) -> Result<(), RiskEngineError> { 
        Ok(()) 
    }
    async fn is_nonce_used(&self, _n: &str) -> Result<bool, RiskEngineError> { 
        Ok(false) 
    }
    async fn consume_nonce(&self, _n: &str) -> Result<(), RiskEngineError> { 
        Ok(()) 
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Simulation Runner
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn playground_simulation() -> Result<(), Box<dyn std::error::Error>> {
    let mut log_file = std::fs::File::create("risk_playground.log")?;
    use std::io::Write;

    let header = "╔═══════════════════════════════════════════════════════════════════════════════╗\n\
                  ║                        RISK ENGINE SCORING PLAYGROUND                         ║\n\
                  ╚═══════════════════════════════════════════════════════════════════════════════╝\n";
    
    println!("\n\x1b[1;36m{}\x1b[0m", header);
    writeln!(log_file, "{}", header)?;
    
    let sub_header = "  Modify src/config.rs to tweak the weights and re-run this test.\n";
    println!("{}", sub_header);
    writeln!(log_file, "{}", sub_header)?;

    let scenarios = [
        ("Clean Login", scenarios::clean_login()),
        ("Brute Force Attack", scenarios::brute_force_attack()),
        ("New Credential / Privileged Action", scenarios::new_cred_privileged_action()),
        ("Tor + Sensitive Action", scenarios::tor_sensitive_action()),
        ("VPN + Extreme Geo Jump", scenarios::vpn_geo_anomaly()),
        ("JWT Fingerprint Mismatch", scenarios::jwt_mismatch()),
        ("Org Under Attack (Multiplier Blowup)", scenarios::org_under_attack()),
        ("Sybil Registration Risk", scenarios_custom::sybil_registration()),
        ("Bot Discovery (Headless + Webdriver)", scenarios_custom::bot_detection()),
        ("Automated Proxy (Bot + Hosting IP)", scenarios_custom::automated_proxy_bot()),
    ];

    let store = MockStore;

    for (name, ctx) in scenarios {
        let decision = evaluate(ctx.clone(), &store).await;
        print_scenario(name, &ctx, &decision, &mut log_file)?;
    }
    
    println!("\n\x1b[1;32m✔ Simulation complete. Results saved to \x1b[1mrisk_playground.log\x1b[0m\n");
    Ok(())
}

fn print_scenario(name: &str, ctx: &RiskContext, decision: &RiskDecision, file: &mut std::fs::File) -> std::io::Result<()> {
    use std::io::Write;
    
    let mut output = String::new();
    
    output.push_str(&format!("▶ SCENARIO: {}\n", name));
    output.push_str(&format!("  Action: {:?}  |  User: {}\n", ctx.action, ctx.username));
    
    // Decision Header with Bracket Explanation
    let (color, bracket) = match decision.decision {
        Decision::Allow => ("\x1b[32m", "Allow (0-39)"),
        Decision::Challenge => ("\x1b[33m", "Challenge (40-64)"),
        Decision::Hold => ("\x1b[35m", "Hold (65-89)"),
        Decision::Deny => ("\x1b[31m", "Deny (90-100)"),
    };
    
    // Terminal output (with colors)
    println!("\x1b[1;34m▶ SCENARIO: {}\x1b[0m", name);
    println!("\x1b[1m  Action:\x1b[0m {:?}  |  \x1b[1mUser:\x1b[0m {}", ctx.action, ctx.username);
    println!("  \x1b[1mDecision:\x1b[0m {}{:?}\x1b[0m (Score: {}) → \x1b[3m{}\x1b[0m", 
        color, decision.decision, decision.score, bracket);

    // File output (clean text)
    output.push_str(&format!("  Decision: {:?} (Score: {}) → {}\n", decision.decision, decision.score, bracket));

    // D+S+N+B+C Breakdown
    let b = &decision.score_breakdown;
    let b_str = format!("  Breakdown: [D:{} | S:{} | N:{} | B:{} | C:{}]\n", b.d, b.s, b.n, b.b, b.c);
    println!("  \x1b[1mBreakdown:\x1b[0m [\x1b[36mD:{}\x1b[0m | \x1b[36mS:{}\x1b[0m | \x1b[36mN:{}\x1b[0m | \x1b[36mB:{}\x1b[0m | \x1b[36mC:{}\x1b[0m]", 
        b.d, b.s, b.n, b.b, b.c);
    output.push_str(&b_str);
    
    let eq_str = format!("  Equation: (Sum(D+S+N+B+C):{} + OrgBias:{}) * ActMult:{:.1} = Final:{}\n", b.base, b.a, b.m_action, decision.score);
    println!("  \x1b[1mEquation:\x1b[0m (Sum:{} + OrgBias:{}) * ActMult:{:.1} = Final:{}", b.base, b.a, b.m_action, decision.score);
    output.push_str(&eq_str);

    // Hard Gates
    if !decision.triggered_gates.is_empty() {
        println!("  \x1b[1;31m⚠ HARD GATES TRIGGERED:\x1b[0m {:?}", decision.triggered_gates);
        output.push_str(&format!("  ⚠ HARD GATES TRIGGERED: {:?}\n", decision.triggered_gates));
    }

    // Directives
    if decision.lock_account || decision.required_action.is_some() || decision.hold_reason.is_some() {
        println!("  \x1b[1mDirectives:\x1b[0m");
        output.push_str("  Directives:\n");
        if let Some(ra) = &decision.required_action {
            println!("    - \x1b[33mREQUIRED ACTION:\x1b[0m {:?}", ra);
            output.push_str(&format!("    - REQUIRED ACTION: {:?}\n", ra));
        }
        if decision.lock_account {
            println!("    - \x1b[31mLOCK ACCOUNT:\x1b[0m Engine triggered permanent credential lock");
            output.push_str("    - LOCK ACCOUNT: Engine triggered permanent credential lock\n");
        }
        if let Some(hr) = &decision.hold_reason {
            println!("    - \x1b[35mHOLD REASON:\x1b[0m {}", hr);
            output.push_str(&format!("    - HOLD REASON: {}\n", hr));
        }
    }

    // Factors Table
    if !decision.contributing_factors.is_empty() {
        println!("  \x1b[1mContributing Factors:\x1b[0m");
        output.push_str("  Contributing Factors:\n");
        for factor in &decision.contributing_factors {
            let (tag, color) = match factor.component {
                ScoreComponent::Device => ("[D]", "\x1b[36m"),
                ScoreComponent::Session => ("[S]", "\x1b[36m"),
                ScoreComponent::Network => ("[N]", "\x1b[36m"),
                ScoreComponent::Behavioral => ("[B]", "\x1b[36m"),
                ScoreComponent::Correlation => ("[C]", "\x1b[33m"), // Correlation gets a distinct color
                ScoreComponent::Absolute => ("[A]", "\x1b[36m"),
                _ => ("[?]", "\x1b[37m"),
            };
            println!("    - {} {}{:<25}\x1b[0m | \x1b[1m+{:>2}\x1b[0m | {}", tag, color, factor.name, factor.contribution, factor.description);
            output.push_str(&format!("    - {} {:<25} | +{:>2} | {}\n", tag, factor.name, factor.contribution, factor.description));
        }
    } else {
        println!("  \x1b[1mContributing Factors:\x1b[0m None");
        output.push_str("  Contributing Factors: None\n");
    }
    
    if decision.escalated {
        println!("  \x1b[1;35m⚡ ESCALATED:\x1b[0m This decision was promoted based on recent history");
        output.push_str("  ⚡ ESCALATED: This decision was promoted based on recent history\n");
    }

    if decision.signals_degraded {
        println!("  \x1b[1;33m⚠ SIGNALS DEGRADED:\x1b[0m Evaluation continued with missing data");
        output.push_str("  ⚠ SIGNALS DEGRADED: Evaluation continued with missing data\n");
    }
    
    println!("\x1b[30m--------------------------------------------------------------------------------\x1b[0m\n");
    output.push_str("--------------------------------------------------------------------------------\n\n");
    
    file.write_all(output.as_bytes())?;
    Ok(())
}

mod scenarios_custom {
    use super::helpers::{RiskContextBuilder, scenarios};
    use risk_engine::context::{RiskContext, RiskAction};

    pub fn sybil_registration() -> RiskContext {
        RiskContextBuilder::new()
            .action(RiskAction::Register)
            .audit_event_count(0)
            .credential_count(0)
            .build()
    }

    pub fn bot_detection() -> RiskContext {
        let mut ctx = scenarios::clean_login();
        ctx.webdriver_detected = Some(true);
        ctx.request_ua = Some("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/120.0.0.0 Safari/537.36".into());
        ctx.touch_capable = Some(false);
        ctx.captcha_score = Some(0.1);
        ctx
    }

    pub fn automated_proxy_bot() -> RiskContext {
        use risk_engine::context::AsnType;
        let mut ctx = bot_detection();
        ctx.ip_asn_type = Some(AsnType::Hosting);
        ctx
    }
}
