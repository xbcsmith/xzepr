// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! XZepr event tracking server.
//!
//! The binary performs runtime bootstrap and delegates all HTTP route composition
//! to the canonical API router.

use anyhow::{bail, Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::signal;
use tracing::{error, info, warn};
use xzepr::api::middleware::opa::{OpaMiddlewareState, ResourceContextBuilders};
use xzepr::api::middleware::resource_context::{
    EventContextBuilder, EventReceiverContextBuilder, EventReceiverGroupContextBuilder,
};
use xzepr::api::middleware::JwtMiddlewareState;
use xzepr::api::rest::auth::AuthState;
use xzepr::api::router::{build_production_router, RouterConfig};
use xzepr::application::handlers::{EventHandler, EventReceiverGroupHandler, EventReceiverHandler};
use xzepr::auth::jwt::{Algorithm, JwtConfig, JwtService};
use xzepr::auth::oidc::{
    InMemoryOidcSessionStore, OidcCallbackHandler, OidcClient, OidcConfig, OidcSessionStore,
    RedisOidcSessionStore,
};
use xzepr::auth::provisioning::UserProvisioningService;
use xzepr::infrastructure::audit::AuditLogger;
use xzepr::infrastructure::config::OidcSessionStoreBackend;
use xzepr::infrastructure::database::{
    PostgresEventReceiverGroupRepository, PostgresEventReceiverRepository, PostgresEventRepository,
    PostgresUserRepository,
};
use xzepr::infrastructure::messaging::producer::KafkaEventPublisher;
use xzepr::infrastructure::messaging::TopicManager;
use xzepr::infrastructure::PrometheusMetrics;
use xzepr::opa::client::OpaClient;
use xzepr::opa::types::OpaFailSafeMode;
use xzepr::Settings;

/// Build the authentication state, wiring OIDC components when enabled.
///
/// When `settings.auth.enable_oidc` is true, builds `OidcConfig`, performs
/// OIDC provider discovery, constructs the callback handler, and creates an
/// in-memory session store with TTL from settings. When OIDC is disabled,
/// returns a minimal state with `NullOidcSessionStore`.
///
/// # Arguments
///
/// * `settings` - Validated runtime settings
/// * `jwt_service` - Initialized JWT service
/// * `provisioning_service` - User provisioning service
///
/// # Errors
///
/// Returns an error if OIDC is enabled but provider discovery fails or
/// the Keycloak configuration is missing.
async fn build_auth_state<R: xzepr::domain::repositories::user_repo::UserRepository + 'static>(
    settings: &Settings,
    jwt_service: Arc<JwtService>,
    provisioning_service: Arc<UserProvisioningService<R>>,
) -> Result<AuthState<R>> {
    if !settings.auth.enable_oidc {
        info!("OIDC authentication disabled");
        return Ok(
            AuthState::new(jwt_service, None, None, provisioning_service)
                .with_local_auth_enabled(settings.auth.enable_local_auth),
        );
    }

    let keycloak = settings
        .auth
        .keycloak
        .as_ref()
        .context("OIDC is enabled but [auth.keycloak] configuration is missing")?;

    info!(
        issuer = %keycloak.issuer_url,
        "Initializing OIDC client"
    );

    let oidc_config = OidcConfig::keycloak(
        keycloak.issuer_url.clone(),
        keycloak.client_id.clone(),
        keycloak.client_secret.clone(),
        keycloak.redirect_url.clone(),
    );

    let oidc_client = Arc::new(
        OidcClient::new(oidc_config)
            .await
            .context("Failed to initialize OIDC client; provider discovery may have failed")?,
    );

    let callback_handler = Arc::new(OidcCallbackHandler::new(oidc_client.clone()));

    let session_ttl = Duration::from_secs(keycloak.session_ttl_seconds);
    // Use 10x the per-user limit as a total-capacity ceiling for pending sessions.
    let max_pending = keycloak.max_sessions_per_user.saturating_mul(10).max(100);
    let session_store: Arc<dyn OidcSessionStore> = match keycloak.session_store.backend {
        OidcSessionStoreBackend::Memory => {
            let store = Arc::new(InMemoryOidcSessionStore::new_with_principal_limit(
                max_pending,
                keycloak.max_sessions_per_user,
                session_ttl,
            ));
            store.clone().spawn_cleanup_task(Duration::from_secs(60));
            store
        }
        OidcSessionStoreBackend::Redis => Arc::new(
            RedisOidcSessionStore::new(
                keycloak.session_store.redis_url.as_deref().context(
                    "OIDC Redis session store requires auth.keycloak.session_store.redis_url",
                )?,
                keycloak.session_store.key_prefix.clone(),
                keycloak.max_sessions_per_user,
            )
            .await
            .context("Failed to initialize Redis-backed OIDC session store")?,
        ),
    };

    let allowed_redirect_hosts = keycloak.allowed_redirect_hosts.clone();

    info!(
        local_auth_enabled = settings.auth.enable_local_auth,
        session_ttl_seconds = keycloak.session_ttl_seconds,
        max_pending,
        allowed_hosts = ?allowed_redirect_hosts,
        "OIDC authentication enabled"
    );

    Ok(AuthState::new_with_oidc(
        jwt_service,
        oidc_client,
        callback_handler,
        session_store as Arc<dyn OidcSessionStore>,
        provisioning_service,
        allowed_redirect_hosts,
        session_ttl,
    )
    .with_local_auth_enabled(settings.auth.enable_local_auth))
}

/// Build the OPA middleware state when OPA authorization is enabled.
///
/// Reads the OPA configuration from `settings.opa`, constructs an `OpaClient`,
/// audit logger, resource context builders, and returns a fully initialized
/// [`OpaMiddlewareState`]. Returns `Ok(None)` when OPA is disabled.
///
/// When OPA is enabled, client construction errors fail startup instead of
/// silently disabling authorization. This keeps configured policy enforcement
/// fail-safe for the canonical runtime path.
///
/// # Arguments
///
/// * `settings` - Validated runtime settings
/// * `event_repo` - Event repository for context building
/// * `receiver_repo` - Event receiver repository for context building
/// * `group_repo` - Event receiver group repository for context building
/// * `metrics` - Prometheus metrics (shared with router)
async fn build_opa_state(
    settings: &Settings,
    event_repo: Arc<PostgresEventRepository>,
    receiver_repo: Arc<PostgresEventReceiverRepository>,
    group_repo: Arc<PostgresEventReceiverGroupRepository>,
    metrics: Option<Arc<PrometheusMetrics>>,
) -> Result<Option<OpaMiddlewareState>> {
    let opa_config = match &settings.opa {
        Some(cfg) if cfg.enabled => cfg.clone(),
        _ => {
            info!("OPA authorization disabled");
            return Ok(None);
        }
    };

    let fail_safe_mode = opa_config.fail_safe_mode;

    let opa_client = Arc::new(
        OpaClient::new(opa_config)
            .context("Failed to build OPA client while OPA authorization is enabled")?,
    );

    let audit_logger = Arc::new(AuditLogger::new());

    let metrics = metrics.unwrap_or_else(|| {
        // SAFETY: Default PrometheusMetrics registry is always valid.
        Arc::new(PrometheusMetrics::default())
    });

    let context_builders = ResourceContextBuilders {
        event: Arc::new(EventContextBuilder::new(
            event_repo.clone(),
            receiver_repo.clone(),
            group_repo.clone(),
        )),
        receiver: Arc::new(EventReceiverContextBuilder::new(
            receiver_repo.clone(),
            group_repo.clone(),
        )),
        group: Arc::new(EventReceiverGroupContextBuilder::new(group_repo.clone())),
    };

    let is_production = std::env::var("RUST_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    if is_production && fail_safe_mode == OpaFailSafeMode::FailOpenDevelopment {
        warn!(
            "OPA fail_safe_mode is fail_open_development but RUST_ENV=production; \
             this combination is rejected by validate_production() and should not be reachable. \
             Startup will fail to prevent accidental fail-open authorization."
        );
        bail!(
            "OPA fail_open_development is invalid in production; refusing to start with fail-open authorization"
        );
    }

    if fail_safe_mode == OpaFailSafeMode::LegacyRbacFallback {
        warn!(
            "OPA fail_safe_mode is legacy_rbac_fallback; \
             built-in RBAC will be used as a fallback when OPA is unavailable"
        );
    }

    opa_client
        .health_check()
        .await
        .context("OPA health check failed while OPA authorization is enabled")?;

    info!(
        url = %opa_client.config().url,
        fail_safe_mode = ?fail_safe_mode,
        is_production,
        "OPA authorization enabled"
    );

    Ok(Some(OpaMiddlewareState::new(
        opa_client,
        audit_logger,
        metrics,
        context_builders,
        fail_safe_mode,
        is_production,
    )))
}

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
    let auth_state = build_auth_state(&settings, jwt_service, provisioning_service).await?;
    let router_config = RouterConfig::from_settings(&settings)
        .context("Failed to build router config from runtime settings")?;
    let opa_state = build_opa_state(
        &settings,
        event_repo.clone(),
        receiver_repo.clone(),
        group_repo.clone(),
        router_config.metrics.clone(),
    )
    .await?;
    let app =
        build_production_router(api_state, auth_state, jwt_state, router_config, opa_state).await;

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
