// src/main.rs
//! XZepr Event Tracking Server
//!
//! Production-ready event tracking server with REST API, GraphQL, authentication,
//! and real-time event streaming with Redpanda.

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Mutex;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{error, info, Level};
use xzepr::{
    api::graphql::{create_schema, graphql_handler, graphql_health, graphql_playground},
    application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler},
    auth::api_key::UserRepository,
    domain::entities::{
        event::Event, event_receiver::EventReceiver, event_receiver_group::EventReceiverGroup,
    },
    domain::repositories::{
        event_receiver_group_repo::{EventReceiverGroupRepository, FindEventReceiverGroupCriteria},
        event_receiver_repo::{EventReceiverRepository, FindEventReceiverCriteria},
        event_repo::{EventRepository, FindEventCriteria},
    },
    domain::value_objects::{EventId, EventReceiverGroupId, EventReceiverId},
    PostgresApiKeyRepository, PostgresUserRepository, Settings,
};

use xzepr::api::graphql::Schema;

/// Unified application state for production use
#[derive(Clone)]
pub struct AppState {
    // Authentication and database
    pub db_pool: PgPool,
    pub user_repo: Arc<PostgresUserRepository>,
    pub api_key_repo: Arc<PostgresApiKeyRepository>,
    // Domain handlers
    pub event_handler: EventHandler,
    pub event_receiver_handler: EventReceiverHandler,
    pub event_receiver_group_handler: EventReceiverGroupHandler,
    // GraphQL schema
    pub graphql_schema: Schema,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting XZepr Event Tracking Server");

    // Load configuration
    let settings = Settings::new().context("Failed to load configuration")?;
    info!("Configuration loaded successfully");
    info!(
        "  Server: {}:{}",
        settings.server.host, settings.server.port
    );
    info!("  HTTPS: {}", settings.server.enable_https);
    info!("  Database: {}", mask_password(&settings.database.url));
    info!("  Kafka: {}", settings.kafka.brokers);

    // Connect to database
    info!("Connecting to database...");
    let db_pool = PgPool::connect(&settings.database.url)
        .await
        .context("Failed to connect to database")?;
    info!("Database connection established");

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .context("Failed to run database migrations")?;
    info!("Database migrations completed");

    // Test database connection
    sqlx::query("SELECT 1")
        .fetch_one(&db_pool)
        .await
        .context("Failed to verify database connection")?;
    info!("Database health check passed");

    // Initialize authentication repositories
    let user_repo = Arc::new(PostgresUserRepository::new(db_pool.clone()));
    let api_key_repo = Arc::new(PostgresApiKeyRepository::new(db_pool.clone()));

    // Initialize domain repositories (using mock for now)
    // TODO: Replace with PostgreSQL implementations when available
    info!("Initializing event repositories (in-memory mode)...");
    let event_repo = Arc::new(MockEventRepository::new());
    let receiver_repo = Arc::new(MockEventReceiverRepository::new());
    let group_repo = Arc::new(MockEventReceiverGroupRepository::new());

    // Create application handlers
    let event_handler = EventHandler::new(event_repo, receiver_repo.clone());
    let receiver_handler = EventReceiverHandler::new(receiver_repo.clone());
    let group_handler = EventReceiverGroupHandler::new(group_repo, receiver_repo);

    // Create GraphQL schema
    let schema = create_schema(
        Arc::new(receiver_handler.clone()),
        Arc::new(group_handler.clone()),
    );

    // Create unified application state
    let app_state = AppState {
        db_pool: db_pool.clone(),
        user_repo,
        api_key_repo,
        event_handler,
        event_receiver_handler: receiver_handler,
        event_receiver_group_handler: group_handler,
        graphql_schema: schema,
    };

    // Build the unified router
    let app = build_router(app_state);

    // Determine bind address
    let addr = SocketAddr::from((
        settings
            .server
            .host
            .parse::<std::net::IpAddr>()
            .unwrap_or_else(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
        settings.server.port,
    ));

    info!("=================================================");
    info!("XZepr Event Tracking Server Ready");
    info!("=================================================");
    info!("Health check:       http://{}/health", addr);
    info!("API status:         http://{}/api/v1/status", addr);
    info!("GraphQL endpoint:   http://{}/graphql", addr);
    info!("GraphQL Playground: http://{}/graphql/playground", addr);
    info!("GraphQL health:     http://{}/graphql/health", addr);
    info!("=================================================");

    // Start server with graceful shutdown
    if settings.server.enable_https {
        info!("TLS/HTTPS enabled");
        info!("Certificate: {}", settings.tls.cert_path);
        info!("Private key: {}", settings.tls.key_path);

        // Load TLS configuration
        let tls_config = load_tls_config(&settings.tls.cert_path, &settings.tls.key_path)
            .await
            .context("Failed to load TLS configuration")?;

        info!("Starting HTTPS server on https://{}", addr);

        // Start HTTPS server
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .context("Server error")?;
    } else {
        info!("TLS/HTTPS disabled - running in HTTP mode");
        info!("Starting HTTP server on http://{}", addr);

        // Start HTTP server
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .context("Failed to bind to address")?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("Server error")?;
    }

    info!("Server shutdown complete");
    Ok(())
}

/// Build the unified application router with all routes and middleware
fn build_router(state: AppState) -> Router {
    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    // Create tracing layer for request logging
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(LatencyUnit::Millis),
        );

    // Build unified router with single state type
    Router::new()
        // Root routes
        .route("/", get(root_handler))
        .route("/health", get(health_check))
        // GraphQL routes
        .route("/graphql", post(graphql_handler_wrapper))
        .route("/graphql/playground", get(graphql_playground_wrapper))
        .route("/graphql/health", get(graphql_health_wrapper))
        // API routes
        .route("/api/v1/status", get(api_status))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/events", post(create_event_wrapper))
        .route("/api/v1/events/:id", get(get_event_wrapper))
        .route("/api/v1/receivers", post(create_event_receiver_wrapper))
        .route("/api/v1/receivers", get(list_event_receivers_wrapper))
        .route("/api/v1/receivers/:id", get(get_event_receiver_wrapper))
        .route("/api/v1/receivers/:id", put(update_event_receiver_wrapper))
        .route(
            "/api/v1/receivers/:id",
            delete(delete_event_receiver_wrapper),
        )
        .route("/api/v1/groups", post(create_event_receiver_group_wrapper))
        .route("/api/v1/groups/:id", get(get_event_receiver_group_wrapper))
        .route(
            "/api/v1/groups/:id",
            put(update_event_receiver_group_wrapper),
        )
        .route(
            "/api/v1/groups/:id",
            delete(delete_event_receiver_group_wrapper),
        )
        .with_state(state)
        .layer(ServiceBuilder::new().layer(trace_layer).layer(cors))
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    // Check database connection
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.db_pool).await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    let status = if db_status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(json!({
            "status": db_status,
            "service": "xzepr",
            "version": env!("CARGO_PKG_VERSION"),
            "components": {
                "database": db_status,
            }
        })),
    )
}

/// Root handler - API information
async fn root_handler() -> impl IntoResponse {
    Json(json!({
        "service": "XZepr Event Tracking Server",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "High-performance event tracking with real-time streaming",
        "endpoints": {
            "health": "/health",
            "graphql": "/graphql",
            "graphql_playground": "/graphql/playground",
            "api": "/api/v1",
        }
    }))
}

/// API status endpoint
async fn api_status() -> impl IntoResponse {
    Json(json!({
        "status": "operational",
        "api_version": "v1",
        "features": {
            "authentication": ["local", "oidc", "api_key"],
            "authorization": "rbac",
            "event_tracking": true,
            "graphql": true,
            "real_time_streaming": true
        }
    }))
}

/// Load TLS configuration from certificate and key files
async fn load_tls_config(
    cert_path: &str,
    key_path: &str,
) -> Result<axum_server::tls_rustls::RustlsConfig> {
    use axum_server::tls_rustls::RustlsConfig;

    info!("Loading TLS certificate from: {}", cert_path);
    info!("Loading TLS private key from: {}", key_path);

    let config = RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .context("Failed to load TLS certificate and key")?;

    info!("TLS configuration loaded successfully");
    Ok(config)
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal, shutting down gracefully...");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down gracefully...");
        },
    }
}

/// Mask password in database URL for logging
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(proto_end) = url.find("://") {
            let proto = &url[..proto_end + 3];
            let after_at = &url[at_pos..];
            if let Some(colon_pos) = url[proto_end + 3..at_pos].find(':') {
                let username = &url[proto_end + 3..proto_end + 3 + colon_pos];
                return format!("{}{}:***{}", proto, username, after_at);
            }
        }
    }
    url.to_string()
}

/// Wrapper for GraphQL handler that extracts schema from AppState
async fn graphql_handler_wrapper(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> axum::response::Response {
    use xzepr::api::graphql::handlers::GraphQLRequest;

    // Parse the GraphQL request
    let graphql_req: GraphQLRequest = match serde_json::from_value(req) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid GraphQL request: {}", e)})),
            )
                .into_response();
        }
    };

    // Call the actual handler with schema from state
    graphql_handler(State(state.graphql_schema), Json(graphql_req)).await
}

/// Wrapper for GraphQL playground
async fn graphql_playground_wrapper(State(state): State<AppState>) -> impl IntoResponse {
    graphql_playground(State(state.graphql_schema)).await
}

/// Wrapper for GraphQL health check
async fn graphql_health_wrapper(State(_state): State<AppState>) -> impl IntoResponse {
    graphql_health().await
}

// ============================================================================
// API Handler Wrappers
// ============================================================================
// These wrappers convert from main::AppState to api::rest::events::AppState

/// Convert main AppState to API AppState
fn to_api_state(state: &AppState) -> xzepr::api::rest::events::AppState {
    xzepr::api::rest::events::AppState {
        event_handler: state.event_handler.clone(),
        event_receiver_handler: state.event_receiver_handler.clone(),
        event_receiver_group_handler: state.event_receiver_group_handler.clone(),
    }
}

async fn create_event_wrapper(
    State(state): State<AppState>,
    body: Bytes,
) -> axum::response::Response {
    use xzepr::api::rest::events::create_event;
    let api_state = to_api_state(&state);
    let json_result = serde_json::from_slice(&body);
    match json_result {
        Ok(json) => create_event(State(api_state), Json(json))
            .await
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid JSON: {}", e)})),
        )
            .into_response(),
    }
}

async fn get_event_wrapper(
    State(state): State<AppState>,
    path: Path<String>,
) -> axum::response::Response {
    use xzepr::api::rest::events::get_event;
    let api_state = to_api_state(&state);
    get_event(State(api_state), path).await.into_response()
}

async fn create_event_receiver_wrapper(
    State(state): State<AppState>,
    body: Bytes,
) -> axum::response::Response {
    use xzepr::api::rest::events::create_event_receiver;
    let api_state = to_api_state(&state);
    let json_result = serde_json::from_slice(&body);
    match json_result {
        Ok(json) => create_event_receiver(State(api_state), Json(json))
            .await
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid JSON: {}", e)})),
        )
            .into_response(),
    }
}

async fn list_event_receivers_wrapper(
    State(state): State<AppState>,
    query: Query<xzepr::api::rest::dtos::EventReceiverQueryParams>,
) -> axum::response::Response {
    use xzepr::api::rest::events::list_event_receivers;
    let api_state = to_api_state(&state);
    list_event_receivers(State(api_state), query)
        .await
        .into_response()
}

async fn get_event_receiver_wrapper(
    State(state): State<AppState>,
    path: Path<String>,
) -> axum::response::Response {
    use xzepr::api::rest::events::get_event_receiver;
    let api_state = to_api_state(&state);
    get_event_receiver(State(api_state), path)
        .await
        .into_response()
}

async fn update_event_receiver_wrapper(
    State(state): State<AppState>,
    path: Path<String>,
    body: Bytes,
) -> axum::response::Response {
    use xzepr::api::rest::events::update_event_receiver;
    let api_state = to_api_state(&state);
    let json_result = serde_json::from_slice(&body);
    match json_result {
        Ok(json) => update_event_receiver(State(api_state), path, Json(json))
            .await
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid JSON: {}", e)})),
        )
            .into_response(),
    }
}

async fn delete_event_receiver_wrapper(
    State(state): State<AppState>,
    path: Path<String>,
) -> axum::response::Response {
    use xzepr::api::rest::events::delete_event_receiver;
    let api_state = to_api_state(&state);
    delete_event_receiver(State(api_state), path)
        .await
        .into_response()
}

async fn create_event_receiver_group_wrapper(
    State(state): State<AppState>,
    body: Bytes,
) -> axum::response::Response {
    use xzepr::api::rest::events::create_event_receiver_group;
    let api_state = to_api_state(&state);
    let json_result = serde_json::from_slice(&body);
    match json_result {
        Ok(json) => create_event_receiver_group(State(api_state), Json(json))
            .await
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid JSON: {}", e)})),
        )
            .into_response(),
    }
}

async fn get_event_receiver_group_wrapper(
    State(state): State<AppState>,
    path: Path<String>,
) -> axum::response::Response {
    use xzepr::api::rest::events::get_event_receiver_group;
    let api_state = to_api_state(&state);
    get_event_receiver_group(State(api_state), path)
        .await
        .into_response()
}

async fn update_event_receiver_group_wrapper(
    State(state): State<AppState>,
    path: Path<String>,
    body: Bytes,
) -> axum::response::Response {
    use xzepr::api::rest::events::update_event_receiver_group;
    let api_state = to_api_state(&state);
    let json_result = serde_json::from_slice(&body);
    match json_result {
        Ok(json) => update_event_receiver_group(State(api_state), path, Json(json))
            .await
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid JSON: {}", e)})),
        )
            .into_response(),
    }
}

async fn delete_event_receiver_group_wrapper(
    State(state): State<AppState>,
    path: Path<String>,
) -> axum::response::Response {
    use xzepr::api::rest::events::delete_event_receiver_group;
    let api_state = to_api_state(&state);
    delete_event_receiver_group(State(api_state), path)
        .await
        .into_response()
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct UserInfo {
    id: String,
    username: String,
    email: String,
}

/// Login endpoint
async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    info!(username = %request.username, "Login attempt");

    // Find user by username
    let user = match state.user_repo.find_by_username(&request.username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            error!(username = %request.username, "User not found");
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid credentials"})),
            );
        }
        Err(err) => {
            error!(username = %request.username, error = %err, "Database error during user lookup");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Authentication service unavailable"})),
            );
        }
    };

    // Check if user is enabled
    if !user.enabled {
        error!(username = %request.username, "User account disabled");
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Account disabled"})),
        );
    }

    // Verify password
    match user.verify_password(&request.password) {
        Ok(true) => {
            info!(
                username = %request.username,
                user_id = %user.id,
                "Login successful"
            );

            // Generate a simple JWT-like token (for demo purposes)
            // TODO: Implement proper JWT token generation
            let token = format!("xzepr_token_{}", user.id);

            let user_info = UserInfo {
                id: user.id.to_string(),
                username: user.username,
                email: user.email,
            };

            (
                StatusCode::OK,
                Json(json!({"token": token, "user": user_info})),
            )
        }
        Ok(false) => {
            error!(username = %request.username, "Password verification failed");
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid credentials"})),
            )
        }
        Err(err) => {
            error!(
                username = %request.username,
                error = %err,
                "Password verification error"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Authentication service unavailable"})),
            )
        }
    }
}

// ============================================================================
// Mock Repository Implementations
// ============================================================================
// These are in-memory implementations for development and testing.
// TODO: Replace with PostgreSQL implementations for production use.

/// Mock event repository that stores events in memory
pub struct MockEventRepository {
    events: Arc<Mutex<HashMap<EventId, Event>>>,
}

impl Default for MockEventRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventRepository {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventRepository for MockEventRepository {
    async fn save(&self, event: &Event) -> xzepr::error::Result<()> {
        let mut events = self.events.lock().unwrap();
        events.insert(event.id(), event.clone());
        info!("Saved event: {} ({})", event.name(), event.id());
        Ok(())
    }

    async fn find_by_id(&self, id: EventId) -> xzepr::error::Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events.get(&id).cloned())
    }

    async fn find_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id)
            .cloned()
            .collect())
    }

    async fn find_by_success(&self, success: bool) -> xzepr::error::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.success() == success)
            .cloned()
            .collect())
    }

    async fn find_by_name(&self, name: &str) -> xzepr::error::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.name().contains(name))
            .cloned()
            .collect())
    }

    async fn find_by_platform_id(&self, platform_id: &str) -> xzepr::error::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.platform_id() == platform_id)
            .cloned()
            .collect())
    }

    async fn find_by_package(&self, package: &str) -> xzepr::error::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.package() == package)
            .cloned()
            .collect())
    }

    async fn list(&self, _limit: usize, _offset: usize) -> xzepr::error::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events.values().cloned().collect())
    }

    async fn count(&self) -> xzepr::error::Result<usize> {
        let events = self.events.lock().unwrap();
        Ok(events.len())
    }

    async fn count_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<usize> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id)
            .count())
    }

    async fn count_successful_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<usize> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id && e.success())
            .count())
    }

    async fn delete(&self, id: EventId) -> xzepr::error::Result<()> {
        let mut events = self.events.lock().unwrap();
        events.remove(&id);
        Ok(())
    }

    async fn find_latest_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        let mut filtered: Vec<Event> = events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id)
            .cloned()
            .collect();
        filtered.sort_by_key(|b| std::cmp::Reverse(b.created_at()));
        Ok(filtered.into_iter().next())
    }

    async fn find_latest_successful_by_receiver_id(
        &self,
        receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<Option<Event>> {
        let events = self.events.lock().unwrap();
        let mut filtered: Vec<Event> = events
            .values()
            .filter(|e| e.event_receiver_id() == receiver_id && e.success())
            .cloned()
            .collect();
        filtered.sort_by_key(|b| std::cmp::Reverse(b.created_at()));
        Ok(filtered.into_iter().next())
    }

    async fn find_by_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> xzepr::error::Result<Vec<Event>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .filter(|e| e.created_at() >= start_time && e.created_at() <= end_time)
            .cloned()
            .collect())
    }

    async fn find_by_criteria(
        &self,
        _criteria: FindEventCriteria,
    ) -> xzepr::error::Result<Vec<Event>> {
        // Simplified implementation
        self.list(1000, 0).await
    }
}

/// Mock event receiver repository
pub struct MockEventReceiverRepository {
    receivers: Arc<Mutex<HashMap<EventReceiverId, EventReceiver>>>,
}

impl Default for MockEventReceiverRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventReceiverRepository {
    pub fn new() -> Self {
        Self {
            receivers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventReceiverRepository for MockEventReceiverRepository {
    async fn save(&self, receiver: &EventReceiver) -> xzepr::error::Result<()> {
        let mut receivers = self.receivers.lock().unwrap();
        receivers.insert(receiver.id(), receiver.clone());
        info!(
            "Saved event receiver: {} ({})",
            receiver.name(),
            receiver.id()
        );
        Ok(())
    }

    async fn find_by_id(&self, id: EventReceiverId) -> xzepr::error::Result<Option<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers.get(&id).cloned())
    }

    async fn find_by_name(&self, name: &str) -> xzepr::error::Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .filter(|r| r.name().contains(name))
            .cloned()
            .collect())
    }

    async fn find_by_type(&self, type_name: &str) -> xzepr::error::Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .filter(|r| r.receiver_type() == type_name)
            .cloned()
            .collect())
    }

    async fn find_by_type_and_version(
        &self,
        type_name: &str,
        version: &str,
    ) -> xzepr::error::Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .filter(|r| r.receiver_type() == type_name && r.version() == version)
            .cloned()
            .collect())
    }

    async fn find_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> xzepr::error::Result<Option<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .find(|r| r.fingerprint() == fingerprint)
            .cloned())
    }

    async fn list(
        &self,
        _limit: usize,
        _offset: usize,
    ) -> xzepr::error::Result<Vec<EventReceiver>> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers.values().cloned().collect())
    }

    async fn count(&self) -> xzepr::error::Result<usize> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers.len())
    }

    async fn update(&self, receiver: &EventReceiver) -> xzepr::error::Result<()> {
        self.save(receiver).await
    }

    async fn delete(&self, id: EventReceiverId) -> xzepr::error::Result<()> {
        let mut receivers = self.receivers.lock().unwrap();
        receivers.remove(&id);
        Ok(())
    }

    async fn exists_by_name_and_type(
        &self,
        name: &str,
        type_name: &str,
    ) -> xzepr::error::Result<bool> {
        let receivers = self.receivers.lock().unwrap();
        Ok(receivers
            .values()
            .any(|r| r.name() == name && r.receiver_type() == type_name))
    }

    async fn find_by_criteria(
        &self,
        _criteria: FindEventReceiverCriteria,
    ) -> xzepr::error::Result<Vec<EventReceiver>> {
        self.list(1000, 0).await
    }
}

/// Mock event receiver group repository
pub struct MockEventReceiverGroupRepository {
    groups: Arc<Mutex<HashMap<EventReceiverGroupId, EventReceiverGroup>>>,
}

impl Default for MockEventReceiverGroupRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventReceiverGroupRepository {
    pub fn new() -> Self {
        Self {
            groups: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventReceiverGroupRepository for MockEventReceiverGroupRepository {
    async fn save(&self, group: &EventReceiverGroup) -> xzepr::error::Result<()> {
        let mut groups = self.groups.lock().unwrap();
        groups.insert(group.id(), group.clone());
        info!(
            "Saved event receiver group: {} ({})",
            group.name(),
            group.id()
        );
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: EventReceiverGroupId,
    ) -> xzepr::error::Result<Option<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.get(&id).cloned())
    }

    async fn find_by_name(&self, name: &str) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups
            .values()
            .filter(|g| g.name().contains(name))
            .cloned()
            .collect())
    }

    async fn find_by_type(&self, type_name: &str) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups
            .values()
            .filter(|g| g.group_type() == type_name)
            .cloned()
            .collect())
    }

    async fn find_by_type_and_version(
        &self,
        type_name: &str,
        version: &str,
    ) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups
            .values()
            .filter(|g| g.group_type() == type_name && g.version() == version)
            .cloned()
            .collect())
    }

    async fn find_enabled(&self) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.values().filter(|g| g.enabled()).cloned().collect())
    }

    async fn find_disabled(&self) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.values().filter(|g| !g.enabled()).cloned().collect())
    }

    async fn find_by_event_receiver_id(
        &self,
        _receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        // Simplified - would need to track group membership
        self.list(1000, 0).await
    }

    async fn list(
        &self,
        _limit: usize,
        _offset: usize,
    ) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.values().cloned().collect())
    }

    async fn count(&self) -> xzepr::error::Result<usize> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.len())
    }

    async fn count_enabled(&self) -> xzepr::error::Result<usize> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.values().filter(|g| g.enabled()).count())
    }

    async fn count_disabled(&self) -> xzepr::error::Result<usize> {
        let groups = self.groups.lock().unwrap();
        Ok(groups.values().filter(|g| !g.enabled()).count())
    }

    async fn update(&self, group: &EventReceiverGroup) -> xzepr::error::Result<()> {
        self.save(group).await
    }

    async fn delete(&self, id: EventReceiverGroupId) -> xzepr::error::Result<()> {
        let mut groups = self.groups.lock().unwrap();
        groups.remove(&id);
        Ok(())
    }

    async fn enable(&self, id: EventReceiverGroupId) -> xzepr::error::Result<()> {
        if let Some(mut group) = self.find_by_id(id).await? {
            group.enable();
            self.update(&group).await?;
        }
        Ok(())
    }

    async fn disable(&self, id: EventReceiverGroupId) -> xzepr::error::Result<()> {
        if let Some(mut group) = self.find_by_id(id).await? {
            group.disable();
            self.update(&group).await?;
        }
        Ok(())
    }

    async fn exists_by_name_and_type(
        &self,
        name: &str,
        type_name: &str,
    ) -> xzepr::error::Result<bool> {
        let groups = self.groups.lock().unwrap();
        Ok(groups
            .values()
            .any(|g| g.name() == name && g.group_type() == type_name))
    }

    async fn add_event_receiver_to_group(
        &self,
        _group_id: EventReceiverGroupId,
        _receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<()> {
        // Simplified implementation
        Ok(())
    }

    async fn remove_event_receiver_from_group(
        &self,
        _group_id: EventReceiverGroupId,
        _receiver_id: EventReceiverId,
    ) -> xzepr::error::Result<()> {
        // Simplified implementation
        Ok(())
    }

    async fn get_group_event_receivers(
        &self,
        _group_id: EventReceiverGroupId,
    ) -> xzepr::error::Result<Vec<EventReceiverId>> {
        // Simplified implementation
        Ok(Vec::new())
    }

    async fn find_by_criteria(
        &self,
        _criteria: FindEventReceiverGroupCriteria,
    ) -> xzepr::error::Result<Vec<EventReceiverGroup>> {
        self.list(1000, 0).await
    }
}
