// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/api/middleware/rbac.rs
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub user_id: UserId,
    pub username: String,
    pub roles: Vec<Role>,
}

impl AuthenticatedUser {
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.roles.iter().any(|role| role.has_permission(permission))
    }

    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }
}

// Extract user from request
pub async fn extract_user(
    State(auth_service): State<Arc<AuthService>>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Get token from Authorization header or cookie
    let token = extract_token(&req)?;

    // Verify token and get user
    let user = auth_service.verify_and_get_user(&token).await?;

    // Check if user is enabled
    if !user.enabled {
        return Err(ApiError::Forbidden("User account is disabled".to_string()));
    }

    // Store authenticated user in request extensions
    req.extensions_mut().insert(AuthenticatedUser {
        user_id: user.id,
        username: user.username.clone(),
        roles: user.roles.clone(),
    });

    Ok(next.run(req).await)
}

// Require specific permission
pub fn require_permission(permission: Permission) -> impl Fn(Request, Next) -> _ {
    move |req: Request, next: Next| {
        let permission = permission.clone();
        async move {
            let user = req
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(ApiError::Unauthorized)?;

            if !user.has_permission(&permission) {
                return Err(ApiError::Forbidden(format!(
                    "Missing required permission: {:?}",
                    permission
                )));
            }

            Ok(next.run(req).await)
        }
    }
}

// Require any of the specified roles
pub fn require_roles(required_roles: Vec<Role>) -> impl Fn(Request, Next) -> _ {
    move |req: Request, next: Next| {
        let required_roles = required_roles.clone();
        async move {
            let user = req
                .extensions()
                .get::<AuthenticatedUser>()
                .ok_or(ApiError::Unauthorized)?;

            if !user.has_any_role(&required_roles) {
                return Err(ApiError::Forbidden(
                    "Insufficient role privileges".to_string()
                ));
            }

            Ok(next.run(req).await)
        }
    }
}

fn extract_token(req: &Request) -> Result<String, ApiError> {
    // Try Authorization header first
    if let Some(auth_header) = req.headers().get("Authorization") {
        let auth_str = auth_header.to_str()
            .map_err(|_| ApiError::Unauthorized)?;

        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            return Ok(token.to_string());
        }
    }

    // Try API key header
    if let Some(api_key) = req.headers().get("X-API-Key") {
        return Ok(api_key.to_str()
            .map_err(|_| ApiError::Unauthorized)?
            .to_string());
    }

    Err(ApiError::Unauthorized)
}
