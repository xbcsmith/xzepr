// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! XZepr event tracking server.
//!
//! The binary performs runtime bootstrap and delegates all HTTP route composition
//! to the canonical API router.

use anyhow::{bail, Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tracing::{error, info, warn};
use xzepr::api::middleware::JwtMiddlewareState;
use xzepr::api::rest::auth::AuthState;
use xzepr::api::router::{build_production_router, RouterConfig};
use xzepr::application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};
use xzepr::auth::jwt::{Algorithm, JwtConfig, JwtService};
use xzepr::auth::provisioning::UserProvisioningService;
use xzepr::infrastructure::database::{
    PostgresEventReceiverGroupRepository, PostgresEventReceiverRepository, PostgresEventRepository,
    PostgresUserRepository,
};
use xzepr::infrastructure::messaging::producer::KafkaEventPublisher;
use xzepr::{Settings, TopicManager};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting XZepr Event Tracking Server");

    let settings = Settings::new().context("Failed to load configuration")?;
    validate_runtime_security(&settings)?;
    log_runtime_settings(&settings);

    let db_pool = connect_database(&settings).await?;
    ensure_kafka_topic(&settings).await;

    let event_repo = Arc::new(PostgresEventRepository::new(db_pool.clone()));
    let receiver_repo = Arc::new(PostgresEventReceiverRepository::new(db_pool.clone()));
    let group_repo = Arc::new(PostgresEventReceiverGroupRepository::new(db_pool.clone()));
    let user_repo = Arc::new(PostgresUserRepository::new(db_pool));

    let event_publisher = create_event_publisher(&settings);

    let event_handler = if let Some(ref publisher) = event_publisher {
        EventHandler::with_publisher(event_repo.clone(), receiver_repo.clone(), publisher.clone())
    } else {
        EventHandler::new(event_repo.clone(), receiver_repo.clone())
    };

    let receiver_handler = if let Some(ref publisher) = event_publisher {
        EventReceiverHandler::with_publisher(receiver_repo.clone(), publisher.clone())
    } else {
        EventReceiverHandler::new(receiver_repo.clone())
    }
    .with_integrity_repositories(event_repo.clone(), group_repo.clone());

    let group_handler = if let Some(ref publisher) = event_publisher {
        EventReceiverGroupHandler::with_publisher(
            group_repo.clone(),
            receiver_repo.clone(),
            publisher.clone(),
        )
    } else {
        EventReceiverGroupHandler::new(group_repo.clone(), receiver_repo.clone())
    }
    .with_user_repository(user_repo.clone())
    .with_event_repository(event_repo.clone());

    let api_state = xzepr::api::rest::events::AppState {
        event_handler,
        event_receiver_handler: receiver_handler,
        event_receiver_group_handler: group_handler,
    };

    let jwt_service = Arc::new(
        JwtService::from_config(jwt_config_from_settings(&settings)?)
            .context("Failed to initialize JWT service")?,
    );
    let jwt_state = JwtMiddlewareState::new(jwt_service.as_ref().clone());
    let provisioning_service = Arc::new(UserProvisioningService::new(user_repo));
    let auth_state = AuthState::new(jwt_service, None, None, provisioning_service);
    let router_config = RouterConfig::from_settings(&settings)
        .context("Failed to build router config from runtime settings")?;
    let app = build_production_router(api_state, auth_state, jwt_state, router_config).await;

    let addr = SocketAddr::from((
        settings
            .server
            .host
            .parse::<std::net::IpAddr>()
            .unwrap_or_else(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
        settings.server.port,
    ));

    log_ready(addr, settings.server.enable_https);

    if settings.server.enable_https {
        let tls_config = load_tls_config(&settings.tls.cert_path, &settings.tls.key_path)
            .await
            .context("Failed to load TLS configuration")?;

        info!("Starting HTTPS server on https://{}", addr);
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .context("Server error")?;
    } else {
        warn!("TLS/HTTPS disabled - running in HTTP mode");
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

async fn connect_database(settings: &Settings) -> Result<PgPool> {
    info!("Connecting to database");
    let db_pool = PgPool::connect(&settings.database.url)
        .await
        .context("Failed to connect to database")?;

    info!("Running database migrations");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .context("Failed to run database migrations")?;

    sqlx::query("SELECT 1")
        .fetch_one(&db_pool)
        .await
        .context("Failed to verify database connection")?;

    info!("Database connection established and verified");
    Ok(db_pool)
}

async fn ensure_kafka_topic(settings: &Settings) {
    info!("Ensuring Kafka topic exists");
    let topic_manager = match TopicManager::new(&settings.kafka.brokers) {
        Ok(manager) => manager,
        Err(e) => {
            warn!(error = %e, "Failed to create Kafka topic manager");
            return;
        }
    };

    match topic_manager
        .ensure_topic_exists(
            &settings.kafka.default_topic,
            settings.kafka.default_topic_partitions,
            settings.kafka.default_topic_replication_factor,
        )
        .await
    {
        Ok(true) => info!(topic = %settings.kafka.default_topic, "Created Kafka topic"),
        Ok(false) => info!(topic = %settings.kafka.default_topic, "Kafka topic already exists"),
        Err(e) => warn!(
            error = %e,
            "Failed to ensure Kafka topic exists; continuing with degraded messaging"
        ),
    }
}

fn create_event_publisher(settings: &Settings) -> Option<Arc<KafkaEventPublisher>> {
    info!("Initializing Kafka event publisher");
    match KafkaEventPublisher::new(&settings.kafka.brokers, &settings.kafka.default_topic) {
        Ok(publisher) => Some(Arc::new(publisher)),
        Err(e) => {
            warn!(
                error = %e,
                "Failed to initialize Kafka event publisher; event publication disabled"
            );
            None
        }
    }
}

fn jwt_config_from_settings(settings: &Settings) -> Result<JwtConfig> {
    let algorithm = match settings.auth.jwt.algorithm.to_uppercase().as_str() {
        "RS256" => Algorithm::RS256,
        "HS256" => Algorithm::HS256,
        other => bail!("Unsupported JWT algorithm: {}", other),
    };

    Ok(JwtConfig {
        access_token_expiration_seconds: settings.auth.jwt.access_token_expiration_seconds,
        refresh_token_expiration_seconds: settings.auth.jwt.refresh_token_expiration_seconds,
        issuer: settings.auth.jwt.issuer.clone(),
        audience: settings.auth.jwt.audience.clone(),
        algorithm,
        private_key_path: settings.auth.jwt.private_key_path.clone(),
        public_key_path: settings.auth.jwt.public_key_path.clone(),
        secret_key: settings.auth.jwt.secret_key.clone(),
        enable_token_rotation: settings.auth.jwt.enable_token_rotation,
        leeway_seconds: settings.auth.jwt.leeway_seconds,
    })
}

fn validate_runtime_security(settings: &Settings) -> Result<()> {
    let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
    if env == "production" {
        settings
            .validate_production()
            .context("Production configuration validation failed")?;
    }

    Ok(())
}

fn log_runtime_settings(settings: &Settings) {
    info!("Configuration loaded successfully");
    info!(
        "  Server: {}:{}",
        settings.server.host, settings.server.port
    );
    info!("  HTTPS: {}", settings.server.enable_https);
    info!("  Database: {}", mask_password(&settings.database.url));
    info!("  Kafka: {}", settings.kafka.brokers);
}

fn log_ready(addr: SocketAddr, https_enabled: bool) {
    let scheme = if https_enabled { "https" } else { "http" };
    info!("=================================================");
    info!("XZepr Event Tracking Server Ready");
    info!("=================================================");
    info!("Health check:       {}://{}/health", scheme, addr);
    info!("API status:         {}://{}/api/v1/status", scheme, addr);
    info!("GraphQL endpoint:   {}://{}/graphql", scheme, addr);
    info!(
        "GraphQL Playground: {}://{}/graphql/playground",
        scheme, addr
    );
    info!("GraphQL health:     {}://{}/graphql/health", scheme, addr);
    info!("=================================================");
}

async fn load_tls_config(cert_path: &str, key_path: &str) -> Result<RustlsConfig> {
    // Canonicalize paths to resolve symlinks and make them absolute.
    let cert_canonical = std::fs::canonicalize(cert_path)
        .with_context(|| format!("Failed to canonicalize TLS cert path: {}", cert_path))?;
    let key_canonical = std::fs::canonicalize(key_path)
        .with_context(|| format!("Failed to canonicalize TLS key path: {}", key_path))?;

    info!("Loading TLS certificate from: {}", cert_canonical.display());
    info!("Loading TLS private key from: {}", key_canonical.display());

    // On Unix platforms, verify the private key file is not world-readable.
    #[cfg(unix)]
    check_private_key_permissions(&key_canonical)?;

    RustlsConfig::from_pem_file(&cert_canonical, &key_canonical)
        .await
        .context("Failed to load TLS certificate and key")
}

/// Verify that a private key file is not world-readable.
///
/// Emits a warning if the file permissions are broader than owner-only (0o600).
/// Returns an error if the key is readable by group or others in production.
///
/// # Errors
///
/// Returns an error on Unix if the file metadata cannot be read, or if the
/// environment is `production` and the key file has group- or world-readable bits set.
#[cfg(unix)]
fn check_private_key_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;
    let mode = metadata.permissions().mode();

    // Check group-readable (0o040) or world-readable (0o004) bits.
    if mode & 0o044 != 0 {
        let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
        if env == "production" {
            anyhow::bail!(
                "Private key file {} has insecure permissions ({:#o}). \
                 Use `chmod 600` to restrict access.",
                path.display(),
                mode & 0o777
            );
        } else {
            warn!(
                "Private key file {} has permissive permissions ({:#o}). \
                 Restrict with `chmod 600` before deploying to production.",
                path.display(),
                mode & 0o777
            );
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            error!(error = %e, "Failed to listen for Ctrl+C signal");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => error!(error = %e, "Failed to listen for terminate signal"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal, shutting down gracefully");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down gracefully");
        },
    }
}

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
