use sqlx::PgPool;
use uuid::Uuid;

/// Insert an audit log entry. Fail-open: errors are logged but never propagated.
pub async fn log_event(
    db: &PgPool,
    user_id: Option<Uuid>,
    actor: &str,
    credential_id: Option<Uuid>,
    action: &str,
    detail: serde_json::Value,
    outcome: &str,
) {
    let result = sqlx::query(
        "INSERT INTO audit_logs (user_id, actor, credential_id, action, detail, outcome) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user_id)
    .bind(actor)
    .bind(credential_id)
    .bind(action)
    .bind(&detail)
    .bind(outcome)
    .execute(db)
    .await;

    if let Err(e) = result {
        tracing::error!(action, actor, error = %e, "failed to write audit log");
    }
}
