// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/api/rest/auth.rs

use serde::Deserialize;
use axum::extract::{State, Query};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct OidcCallbackQuery {
    code: String,
    state: String,
}

pub async fn oidc_callback(
    State(keycloak): State<Arc<KeycloakClient>>,
    State(user_repo): State<Arc<dyn UserRepository>>,
    Query(params): Query<OidcCallbackQuery>,
) -> Result<Json<LoginResponse>, ApiError> {
    // Exchange authorization code for tokens
    let token_response = keycloak
        .exchange_code(AuthorizationCode::new(params.code))
        .await?;

    // Verify and extract claims
    let claims = keycloak
        .verify_token(token_response.access_token().secret())
        .await?;

    // Find or create user
    let user = match user_repo.find_by_oidc_subject(&claims.sub).await? {
        Some(user) => user,
        None => {
            // Auto-provision user from OIDC claims
            let new_user = User::new_oidc(
                claims.preferred_username,
                claims.email,
                claims.sub,
            );
            user_repo.save(&new_user).await?;
            new_user
        }
    };

    // Generate internal JWT
    let jwt = generate_internal_jwt(&user)?;

    Ok(Json(LoginResponse { token: jwt }))
}
