// src/api/rest/routes.rs

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};

use crate::api::rest::events::{
    create_event, get_event, create_event_receiver, get_event_receiver,
    list_event_receivers, update_event_receiver, delete_event_receiver,
    create_event_receiver_group, get_event_receiver_group,
    update_event_receiver_group, delete_event_receiver_group, health_check,
    AppState,
};

/// Builds the complete router with all API routes
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))

        // Event routes
        .route("/api/v1/events", post(create_event))
        .route("/api/v1/events/:id", get(get_event))

        // Event receiver routes
        .route("/api/v1/receivers", post(create_event_receiver))
        .route("/api/v1/receivers", get(list_event_receivers))
        .route("/api/v1/receivers/:id", get(get_event_receiver))
        .route("/api/v1/receivers/:id", put(update_event_receiver))
        .route("/api/v1/receivers/:id", delete(delete_event_receiver))

        // Event receiver group routes
        .route("/api/v1/groups", post(create_event_receiver_group))
        .route("/api/v1/groups/:id", get(get_event_receiver_group))
        .route("/api/v1/groups/:id", put(update_event_receiver_group))
        .route("/api/v1/groups/:id", delete(delete_event_receiver_group))

        // Middleware layers
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Builds router with authentication middleware for protected routes
pub fn build_protected_router(state: AppState) -> Router {
    Router::new()
        // Health check (public)
        .route("/health", get(health_check))

        // Protected event routes
        .route("/api/v1/events", post(create_event))
        .route("/api/v1/events/:id", get(get_event))

        // Protected event receiver routes
        .route("/api/v1/receivers", post(create_event_receiver))
        .route("/api/v1/receivers", get(list_event_receivers))
        .route("/api/v1/receivers/:id", get(get_event_receiver))
        .route("/api/v1/receivers/:id", put(update_event_receiver))
        .route("/api/v1/receivers/:id", delete(delete_event_receiver))

        // Protected event receiver group routes
        .route("/api/v1/groups", post(create_event_receiver_group))
        .route("/api/v1/groups/:id", get(get_event_receiver_group))
        .route("/api/v1/groups/:id", put(update_event_receiver_group))
        .route("/api/v1/groups/:id", delete(delete_event_receiver_group))

        // Add authentication middleware here when implemented
        // .route_layer(middleware::from_fn_with_state(
        //     state.clone(),
        //     extract_user,
        // ))

        // Middleware layers
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;
    use crate::application::handlers::{EventHandler, EventReceiverHandler, EventReceiverGroupHandler};
    use std::sync::Arc;

    // Mock handlers for testing - in real implementation these would be properly initialized
    fn create_test_state() -> AppState {
        // This is a simplified test state - in real implementation,
        // you would properly initialize handlers with mock repositories
        todo!("Implement test state creation with mock repositories")
    }

    #[tokio::test]
    async fn test_health_check_route() {
        let state = create_test_state();
        let app = build_router(state);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_routes_exist() {
        let state = create_test_state();
        let app = build_router(state);

        // Test that routes are registered (this will fail with 405 Method Not Allowed
        // or other errors, but won't fail with 404 Not Found)
        let routes = vec![
            "/api/v1/events",
            "/api/v1/receivers",
            "/api/v1/groups",
        ];

        for route in routes {
            let request = Request::builder()
                .method(Method::GET)
                .uri(route)
                .body(axum::body::Body::empty())
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            // Should not be 404 Not Found
            assert_ne!(response.status(), StatusCode::NOT_FOUND);
        }
    }
}
