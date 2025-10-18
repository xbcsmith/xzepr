// src/main.rs
use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
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
    auth::api_key::UserRepository,
    PostgresApiKeyRepository,
    PostgresUserRepository,
    Settings
};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub user_repo: Arc<PostgresUserRepository>,
    pub api_key_repo: Arc<PostgresApiKeyRepository>,
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

    info!("Starting XZEPR Event Tracking Server");

    // Load configuration
    let settings = Settings::new().context("Failed to load configuration")?;
    info!("Configuration loaded successfully");

    // Connect to database
    info!("Connecting to database at {}", settings.database.url);
    let db_pool = PgPool::connect(&settings.database.url)
        .await
        .context("Failed to connect to database")?;
    info!("Database connection established");

    // Test database connection
    sqlx::query("SELECT 1")
        .fetch_one(&db_pool)
        .await
        .context("Failed to verify database connection")?;
    info!("Database health check passed");

    // Initialize repositories
    let user_repo = Arc::new(PostgresUserRepository::new(db_pool.clone()));
    let api_key_repo = Arc::new(PostgresApiKeyRepository::new(db_pool.clone()));

    // Create application state
    let state = AppState {
        db_pool: db_pool.clone(),
        user_repo,
        api_key_repo,
    };

    // Build router with all routes and middleware
    let app = build_router(state);

    // Determine bind address
    let addr = SocketAddr::from((
        settings
            .server
            .host
            .parse::<std::net::IpAddr>()
            .unwrap_or_else(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
        settings.server.port,
    ));

    info!("Server listening on {}", addr);
    info!("Health check available at: http://{}/health", addr);
    info!("API endpoints available at: http://{}/api/v1/*", addr);

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

/// Build the application router with all routes and middleware
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

    // Build the router with all routes
    Router::new()
        .route("/health", get(health_check))
        .route("/", get(root_handler))
        .route("/api/v1/status", get(api_status))
        .route("/api/v1/auth/login", post(login))
        // TODO: Add event routes
        // .route("/api/v1/events", post(create_event).get(list_events))
        // .route("/api/v1/events/:id", get(get_event).delete(delete_event))
        .layer(ServiceBuilder::new().layer(trace_layer).layer(cors))
        .with_state(state)
}

/// Health check endpoint
async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
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
        "service": "XZEPR Event Tracking Server",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "High-performance event tracking with real-time streaming",
        "endpoints": {
            "health": "/health",
            "api": "/api/v1",
            "documentation": "/api/v1/docs"
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
