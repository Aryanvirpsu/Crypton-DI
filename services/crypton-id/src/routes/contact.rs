use axum::{
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{error::{AppError, ApiResponse, AppJson}, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/contact", post(submit_contact))
}

#[derive(Debug, Deserialize)]
pub struct ContactReq {
    pub name: String,
    pub email: String,
    pub company: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ContactResp {
    pub status: &'static str,
}

// Minimal email validation checking for one '@' and a domain part
fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let domain_parts: Vec<&str> = parts[1].split('.').collect();
    domain_parts.len() >= 2 && !domain_parts[0].is_empty() && !domain_parts[1].is_empty()
}

async fn submit_contact(
    AppJson(req): AppJson<ContactReq>,
) -> Result<Json<ApiResponse<ContactResp>>, AppError> {
    let name = req.name.trim();
    let email = req.email.trim();
    let company = req.company.as_deref().map(|c| c.trim()).filter(|c| !c.is_empty());
    let message = req.message.trim();

    if name.is_empty() {
        return Err(AppError::bad_request("name_required"));
    }

    if email.is_empty() || !is_valid_email(email) {
        return Err(AppError::bad_request("invalid_email"));
    }

    if message.is_empty() {
        return Err(AppError::bad_request("message_required"));
    }

    if message.len() > 1000 {
        return Err(AppError::bad_request("message_too_long"));
    }

    // Day 3 Handoff: Safe temporary persistence
    // For Day 2, we are logging the structured payload into the main application trace logs.
    // On Day 3, we will insert the Resend HTTP call right here.
    tracing::info!(
        contact_event = "new_submission",
        name = %name,
        email = %email,
        company = ?company,
        message_length = message.len(),
        "New contact form submission received."
    );

    // In a real Day 3 scenario we might also insert this into a `contact_messages` database table 
    // to ensure no dropped messages if the email provider fails.

    Ok(Json(ApiResponse::success(ContactResp { status: "received" })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_email() {
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name+tag@domain.co.uk"));
    }

    #[test]
    fn test_invalid_email() {
        assert!(!is_valid_email("invalid-email"));
        assert!(!is_valid_email("test@"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("test@example")); 
    }
}
