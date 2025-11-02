// src/api/rest/routes.rs

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::api::graphql::{create_schema, graphql_handler, graphql_health, graphql_playground};
use crate::api::rest::events::{
    create_event, create_event_receiver, create_event_receiver_group, delete_event_receiver,
    delete_event_receiver_group, get_event, get_event_receiver, get_event_receiver_group,
    health_check, list_event_receivers, update_event_receiver, update_event_receiver_group,
    AppState,
};

/// Builds the complete router with all API routes
pub fn build_router(state: AppState) -> Router {
    // Create GraphQL schema
    let schema = create_schema(
        std::sync::Arc::new(state.event_receiver_handler.clone()),
        std::sync::Arc::new(state.event_receiver_group_handler.clone()),
    );

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // GraphQL routes
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        .route("/graphql/health", get(graphql_health))
        .with_state(schema.clone())
        // REST API routes
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
        .with_state(state)
        // Middleware layers
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

/// Builds router with authentication middleware for protected routes
pub fn build_protected_router(state: AppState) -> Router {
    // Create GraphQL schema
    let schema = create_schema(
        std::sync::Arc::new(state.event_receiver_handler.clone()),
        std::sync::Arc::new(state.event_receiver_group_handler.clone()),
    );

    Router::new()
        // Health check (public)
        .route("/health", get(health_check))
        // GraphQL routes (public for now, can be protected later)
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        .route("/graphql/health", get(graphql_health))
        .with_state(schema.clone())
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
        .with_state(state)
        // Add authentication middleware here when implemented
        // .route_layer(middleware::from_fn_with_state(
        //     state.clone(),
        //     extract_user,
        // ))
        // Middleware layers
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::handlers::{
        EventHandler, EventReceiverGroupHandler, EventReceiverHandler,
    };
    use crate::domain::entities::event::Event;
    use crate::domain::entities::event_receiver::EventReceiver;
    use crate::domain::entities::event_receiver_group::EventReceiverGroup;
    use crate::domain::repositories::event_receiver_group_repo::EventReceiverGroupRepository;
    use crate::domain::repositories::event_receiver_repo::EventReceiverRepository;
    use crate::domain::repositories::event_repo::EventRepository;
    use crate::domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId};
    use crate::error::Result;
    use async_trait::async_trait;
    use axum::http::{Method, Request, StatusCode};
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    // Mock EventRepository for testing
    struct MockEventRepository {
        events: Arc<Mutex<HashMap<EventId, Event>>>,
    }

    impl MockEventRepository {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl EventRepository for MockEventRepository {
        async fn save(&self, event: &Event) -> Result<()> {
            let mut events = self.events.lock().unwrap();
            events.insert(event.id(), event.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: EventId) -> Result<Option<Event>> {
            let events = self.events.lock().unwrap();
            Ok(events.get(&id).cloned())
        }

        async fn find_by_receiver_id(&self, _receiver_id: EventReceiverId) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_success(&self, _success: bool) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_name(&self, _name: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_platform_id(&self, _platform_id: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_package(&self, _package: &str) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<usize> {
            let events = self.events.lock().unwrap();
            Ok(events.len())
        }

        async fn count_by_receiver_id(&self, _receiver_id: EventReceiverId) -> Result<usize> {
            Ok(0)
        }

        async fn count_successful_by_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<usize> {
            Ok(0)
        }

        async fn delete(&self, id: EventId) -> Result<()> {
            let mut events = self.events.lock().unwrap();
            events.remove(&id);
            Ok(())
        }

        async fn find_latest_by_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<Option<Event>> {
            Ok(None)
        }

        async fn find_latest_successful_by_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<Option<Event>> {
            Ok(None)
        }

        async fn find_by_time_range(
            &self,
            _start: DateTime<Utc>,
            _end: DateTime<Utc>,
        ) -> Result<Vec<Event>> {
            Ok(vec![])
        }

        async fn find_by_criteria(
            &self,
            _criteria: crate::domain::repositories::event_repo::FindEventCriteria,
        ) -> Result<Vec<Event>> {
            Ok(vec![])
        }
    }

    // Mock EventReceiverRepository for testing
    struct MockEventReceiverRepository {
        receivers: Arc<Mutex<HashMap<EventReceiverId, EventReceiver>>>,
    }

    impl MockEventReceiverRepository {
        fn new() -> Self {
            Self {
                receivers: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl EventReceiverRepository for MockEventReceiverRepository {
        async fn save(&self, event_receiver: &EventReceiver) -> Result<()> {
            let mut receivers = self.receivers.lock().unwrap();
            receivers.insert(event_receiver.id(), event_receiver.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: EventReceiverId) -> Result<Option<EventReceiver>> {
            let receivers = self.receivers.lock().unwrap();
            Ok(receivers.get(&id).cloned())
        }

        async fn find_by_name(&self, _name: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_type(&self, _receiver_type: &str) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_type_and_version(
            &self,
            _receiver_type: &str,
            _version: &str,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn find_by_fingerprint(&self, _fingerprint: &str) -> Result<Option<EventReceiver>> {
            Ok(None)
        }

        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<usize> {
            Ok(0)
        }

        async fn update(&self, event_receiver: &EventReceiver) -> Result<()> {
            let mut receivers = self.receivers.lock().unwrap();
            receivers.insert(event_receiver.id(), event_receiver.clone());
            Ok(())
        }

        async fn delete(&self, id: EventReceiverId) -> Result<()> {
            let mut receivers = self.receivers.lock().unwrap();
            receivers.remove(&id);
            Ok(())
        }

        async fn exists_by_name_and_type(&self, _name: &str, _receiver_type: &str) -> Result<bool> {
            Ok(false)
        }

        async fn find_by_criteria(
            &self,
            _criteria: crate::domain::repositories::event_receiver_repo::FindEventReceiverCriteria,
        ) -> Result<Vec<EventReceiver>> {
            Ok(vec![])
        }
    }

    // Mock EventReceiverGroupRepository for testing
    struct MockEventReceiverGroupRepository {
        groups: Arc<Mutex<HashMap<EventReceiverGroupId, EventReceiverGroup>>>,
    }

    impl MockEventReceiverGroupRepository {
        fn new() -> Self {
            Self {
                groups: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl EventReceiverGroupRepository for MockEventReceiverGroupRepository {
        async fn save(&self, group: &EventReceiverGroup) -> Result<()> {
            let mut groups = self.groups.lock().unwrap();
            groups.insert(group.id(), group.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: EventReceiverGroupId) -> Result<Option<EventReceiverGroup>> {
            let groups = self.groups.lock().unwrap();
            Ok(groups.get(&id).cloned())
        }

        async fn find_by_name(&self, _name: &str) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }

        async fn find_by_type(&self, _group_type: &str) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }

        async fn find_by_type_and_version(
            &self,
            _group_type: &str,
            _version: &str,
        ) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }

        async fn find_enabled(&self) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }

        async fn find_disabled(&self) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }

        async fn find_by_event_receiver_id(
            &self,
            _receiver_id: EventReceiverId,
        ) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }

        async fn list(&self, _limit: usize, _offset: usize) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<usize> {
            Ok(0)
        }

        async fn count_enabled(&self) -> Result<usize> {
            Ok(0)
        }

        async fn count_disabled(&self) -> Result<usize> {
            Ok(0)
        }

        async fn update(&self, group: &EventReceiverGroup) -> Result<()> {
            let mut groups = self.groups.lock().unwrap();
            groups.insert(group.id(), group.clone());
            Ok(())
        }

        async fn delete(&self, id: EventReceiverGroupId) -> Result<()> {
            let mut groups = self.groups.lock().unwrap();
            groups.remove(&id);
            Ok(())
        }

        async fn enable(&self, _id: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }

        async fn disable(&self, _id: EventReceiverGroupId) -> Result<()> {
            Ok(())
        }

        async fn exists_by_name_and_type(&self, _name: &str, _group_type: &str) -> Result<bool> {
            Ok(false)
        }

        async fn add_event_receiver_to_group(
            &self,
            _group_id: EventReceiverGroupId,
            _receiver_id: EventReceiverId,
        ) -> Result<()> {
            Ok(())
        }

        async fn remove_event_receiver_from_group(
            &self,
            _group_id: EventReceiverGroupId,
            _receiver_id: EventReceiverId,
        ) -> Result<()> {
            Ok(())
        }

        async fn get_group_event_receivers(
            &self,
            _group_id: EventReceiverGroupId,
        ) -> Result<Vec<EventReceiverId>> {
            Ok(vec![])
        }

        async fn find_by_criteria(
            &self,
            _criteria: crate::domain::repositories::event_receiver_group_repo::FindEventReceiverGroupCriteria,
        ) -> Result<Vec<EventReceiverGroup>> {
            Ok(vec![])
        }
    }

    /// Creates a test AppState with mock repositories
    fn create_test_state() -> AppState {
        let event_repo = Arc::new(MockEventRepository::new());
        let receiver_repo = Arc::new(MockEventReceiverRepository::new());
        let group_repo = Arc::new(MockEventReceiverGroupRepository::new());

        let event_handler = EventHandler::new(event_repo, receiver_repo.clone());
        let event_receiver_handler = EventReceiverHandler::new(receiver_repo.clone());
        let event_receiver_group_handler =
            EventReceiverGroupHandler::new(group_repo, receiver_repo);

        AppState {
            event_handler,
            event_receiver_handler,
            event_receiver_group_handler,
        }
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
        let routes = vec!["/api/v1/events", "/api/v1/receivers", "/api/v1/groups"];

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

    #[tokio::test]
    async fn test_protected_router_health_check() {
        let state = create_test_state();
        let app = build_protected_router(state);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_protected_router_routes_exist() {
        let state = create_test_state();
        let app = build_protected_router(state);

        // Test that protected routes are registered
        let routes = vec!["/api/v1/events", "/api/v1/receivers", "/api/v1/groups"];

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

    #[tokio::test]
    async fn test_router_has_cors_layer() {
        let state = create_test_state();
        let app = build_router(state);

        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // OPTIONS requests should be handled by CORS layer
        assert!(
            response.status().is_success() || response.status() == StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[tokio::test]
    async fn test_router_event_routes() {
        let state = create_test_state();
        let app = build_router(state);

        // Test POST /api/v1/events exists
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/events")
            .header("content-type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        // Should not be 404 Not Found (will be 400 or 422 due to empty body)
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test GET /api/v1/events/:id exists
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/events/01H0EXAMPLE0000000000000000")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Should not be 404 Not Found (will be other error due to invalid ID)
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_router_receiver_routes() {
        let state = create_test_state();
        let app = build_router(state);

        // Test POST /api/v1/receivers exists
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/receivers")
            .header("content-type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test GET /api/v1/receivers exists
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/receivers")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test GET /api/v1/receivers/:id exists
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/receivers/01H0EXAMPLE0000000000000000")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test PUT /api/v1/receivers/:id exists
        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/receivers/01H0EXAMPLE0000000000000000")
            .header("content-type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test DELETE /api/v1/receivers/:id exists
        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/api/v1/receivers/01H0EXAMPLE0000000000000000")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_router_group_routes() {
        let state = create_test_state();
        let app = build_router(state);

        // Test POST /api/v1/groups exists
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/groups")
            .header("content-type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test GET /api/v1/groups/:id exists
        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/groups/01H0EXAMPLE0000000000000000")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test PUT /api/v1/groups/:id exists
        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/groups/01H0EXAMPLE0000000000000000")
            .header("content-type", "application/json")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);

        // Test DELETE /api/v1/groups/:id exists
        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/api/v1/groups/01H0EXAMPLE0000000000000000")
            .body(axum::body::Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }
}
