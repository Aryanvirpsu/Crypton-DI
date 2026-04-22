//! Postgres-backed context loader.
//!
//! Given a `user_id` (and optional `credential_id`), this module fetches the
//! slow-moving signals the engine needs from the Crypton identity schema —
//! credential status, credential age, registered UA, sign-count, and audit
//! history counts — and returns a [`ContextPatch`] the caller can merge into
//! a freshly-built [`RiskContext`].
//!
//! The schema matches the migrations in
//! `services/crypton-identity/migrations/`:
//!   - `users(id, username, created_at)`
//!   - `credentials(id, user_id, sign_count, user_agent, status, created_at, last_used_at)`
//!   - `audit_logs(user_id, actor, action, outcome, created_at)`
//!
//! The loader does **not** touch Redis — that is [`fred_store`]'s job — nor
//! does it build a full [`RiskContext`]; the caller still owns request-shaped
//! fields like JWT, IP, UA, and nonce.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::context::CredentialStatus;
use crate::error::RiskEngineError;

// ─────────────────────────────────────────────────────────────────────────────
// ContextPatch
// ─────────────────────────────────────────────────────────────────────────────

/// Fields loaded from Postgres that the caller applies to a [`RiskContext`].
///
/// Every field is optional because the DB may be degraded or the row may
/// not exist (brand-new user mid-registration). The caller should set
/// `db_signals_degraded = true` on the context when this loader returns an
/// `Err`; missing-but-ok rows are represented by `Option::None` fields.
#[derive(Debug, Clone, Default)]
pub struct ContextPatch {
    pub credential_status: Option<CredentialStatus>,
    pub credential_created_at: Option<DateTime<Utc>>,
    pub credential_last_used_at: Option<DateTime<Utc>>,
    pub credential_sign_count_prev: Option<u64>,
    pub credential_registered_ua: Option<String>,
    pub credential_count_for_user: Option<u32>,
    pub account_age_days: Option<u32>,
    pub prior_audit_event_count: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// SqlxContextLoader
// ─────────────────────────────────────────────────────────────────────────────

/// Loader that owns a [`PgPool`] and fetches a [`ContextPatch`] per request.
#[derive(Clone)]
pub struct SqlxContextLoader {
    pool: PgPool,
}

impl SqlxContextLoader {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Borrow the pool for callers that want to run their own queries
    /// alongside the loader (e.g. recovery_requests lookups).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Fetch the patch for a (user, credential) pair. `credential_id` may be
    /// `None` for actions like Register where no credential exists yet; in
    /// that case the credential-scoped fields stay `None` and only the user
    /// and audit counts are populated.
    pub async fn load(
        &self,
        user_id: Uuid,
        credential_id: Option<Uuid>,
    ) -> Result<ContextPatch, RiskEngineError> {
        let mut patch = ContextPatch::default();

        // ── Credential row (optional) ────────────────────────────────────────
        if let Some(cid) = credential_id {
            let row = sqlx::query_as::<_, CredentialRow>(
                "SELECT status, sign_count, user_agent, created_at, last_used_at
                 FROM credentials
                 WHERE id = $1 AND user_id = $2",
            )
            .bind(cid)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;

            if let Some(r) = row {
                patch.credential_status = Some(map_status(&r.status));
                patch.credential_sign_count_prev = Some(r.sign_count.max(0) as u64);
                patch.credential_registered_ua = r.user_agent;
                patch.credential_created_at = Some(r.created_at);
                patch.credential_last_used_at = r.last_used_at;
            }
        }

        // ── User + credential counts (single round-trip) ─────────────────────
        let summary = sqlx::query_as::<_, UserSummaryRow>(
            "SELECT
                u.created_at AS user_created_at,
                (SELECT COUNT(*)::bigint FROM credentials c WHERE c.user_id = u.id) AS credential_count,
                (SELECT COUNT(*)::bigint FROM audit_logs a WHERE a.user_id = u.id) AS audit_count
             FROM users u
             WHERE u.id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        if let Some(s) = summary {
            let age_days = (Utc::now() - s.user_created_at).num_days().max(0) as u32;
            patch.account_age_days = Some(age_days);
            patch.credential_count_for_user = Some(s.credential_count.max(0) as u32);
            patch.prior_audit_event_count = Some(s.audit_count.max(0) as u32);
        }

        Ok(patch)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Row types (private)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct CredentialRow {
    status: String,
    sign_count: i64,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct UserSummaryRow {
    user_created_at: DateTime<Utc>,
    credential_count: i64,
    audit_count: i64,
}

fn map_status(s: &str) -> CredentialStatus {
    match s {
        "active" => CredentialStatus::Active,
        "revoked" => CredentialStatus::Revoked,
        "lost" => CredentialStatus::Lost,
        _ => CredentialStatus::Active, // fail-open: unknown status treated as active so the engine still scores normally
    }
}

fn pg_err(e: sqlx::Error) -> RiskEngineError {
    // Source-preserving wrap — SQLx's rich error chain (Database, Io,
    // Protocol, …) is walkable from `tracing::error!(error = ?e)` without
    // us having to stringify it pre-wrap.
    RiskEngineError::store_with_source("postgres operation failed", e)
}
