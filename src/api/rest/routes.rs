// Generated from xzepr-architecture-plan.md
// Section: Multiple sections
// Original line: 0

// src/api/rest/mod.rs
use axum::Router;
use tower_http::cors::CorsLayer;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Public routes
        .route("/health", get(health_check))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/oidc/callback", get(oidc_callback))
        
        // Protected routes - require authentication
        .route("/api/v1/events", post(create_event))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            extract_user,
        ))
        .route_layer(middleware::from_fn(
            require_permission(Permission::EventCreate)
        ))
        
        .route("/api/v1/events/:id", get(get_event))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            extract_user,
        ))
        .route_layer(middleware::from_fn(
            require_permission(Permission::EventRead)
        ))
        
        // Admin-only routes
        .route("/api/v1/users", post(create_user))
        .route("/api/v1/users/:id/roles", put(update_user_roles))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            extract_user,
        ))
        .route_layer(middleware::from_fn(
            require_roles(vec![Role::Admin])
        ))
        
        .layer(CorsLayer::permissive())
        .with_state(state)
}