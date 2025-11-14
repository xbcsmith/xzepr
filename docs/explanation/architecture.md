# XZepr Architecture Plan

## RBAC Implementation Status

### Overview

The RBAC (Role-Based Access Control) system is **partially implemented**. The core components are complete and well-tested, but enforcement middleware is not yet wired up to REST API routes.

### Implementation Status Summary

#### ✅ FULLY IMPLEMENTED AND TESTED

**1. Role System** (`src/auth/rbac/roles.rs`)

- Four roles: Admin, EventManager, EventViewer, User
- Complete permission mapping for each role
- 44 passing unit tests
- Serialization/deserialization support
- String conversion (FromStr/Display)

**2. Permission System** (`src/auth/rbac/permissions.rs`)

- 14 permissions across three resource types:
  - Events: Create, Read, Update, Delete
  - Receivers: Create, Read, Update, Delete
  - Groups: Create, Read, Update, Delete
  - Admin: UserManage, RoleManage
- `from_action()` method for resource/action mapping
- All tests passing

**3. User Entity with RBAC** (`src/domain/entities/user.rs`)

- `has_role()` method for role checking
- `has_permission()` method for permission checking
- Support for multiple auth providers (Local, Keycloak, ApiKey)
- Password hashing with Argon2
- Comprehensive test coverage

**4. JWT with RBAC Integration** (`src/auth/jwt/`)

- Claims structure includes roles and permissions
- Role checking: `has_role()`, `has_any_role()`, `has_all_roles()`
- Permission checking: `has_permission()`, `has_any_permission()`, `has_all_permissions()`
- Full JWT middleware with `AuthenticatedUser` extraction
- Token validation and expiration handling
- All tests passing

**5. GraphQL RBAC Guards** (`src/api/graphql/guards.rs`)

- `require_auth()` - Basic authentication check
- `require_roles()` - Enforce specific roles
- `require_permissions()` - Enforce specific permissions
- `require_roles_and_permissions()` - Combined enforcement
- Helper functions: `require_admin()`, `require_user()`
- Fully integrated with Claims

#### ⚠️ PARTIALLY IMPLEMENTED

**1. RBAC Middleware** (`src/auth/rbac/middleware.rs`)

- Code skeleton exists but has compilation issues
- References undefined types and imports
- Not properly integrated with the rest of the codebase
- NOT imported or used anywhere in the application
- Needs refactoring to use existing JWT middleware patterns

#### ❌ NOT IMPLEMENTED / NOT WIRED UP

**1. REST API Route Protection**

- `build_protected_router()` exists but middleware is commented out
- No RBAC enforcement on REST endpoints
- All routes are currently public/open
- TODO: Apply JWT middleware with role/permission guards

**2. OIDC Integration**

- Module structure exists but is commented out in `src/auth/mod.rs`
- Keycloak integration not fully implemented
- TODO: Complete OIDC provider integration

### What Works Right Now

1. **GraphQL API**: Fully protected with role and permission guards
2. **JWT Authentication**: Token generation, validation, and claims extraction
3. **User Management**: Role and permission assignment at user level
4. **Domain Logic**: All RBAC checks in business logic work correctly

### What Doesn't Work Yet

1. **REST API Protection**: No middleware applied to HTTP routes
2. **Automatic Role Enforcement**: Must be manually checked in handlers
3. **OIDC Authentication**: Keycloak integration incomplete
4. **API Key with RBAC**: API key auth exists but RBAC integration unclear

### Next Steps to Complete RBAC

1. **Fix RBAC Middleware** (`src/auth/rbac/middleware.rs`):

   - Remove undefined type references
   - Use existing JWT middleware patterns
   - Integrate with `AuthenticatedUser` from JWT middleware

2. **Wire Up REST Routes** (`src/api/rest/routes.rs`):

   - Uncomment and configure middleware in `build_protected_router()`
   - Apply JWT authentication middleware
   - Add role/permission guards to specific routes

3. **Create Route Guards**:

   - Implement `require_permission()` middleware for REST
   - Implement `require_roles()` middleware for REST
   - Follow patterns from GraphQL guards

4. **Testing**:
   - Add integration tests for protected endpoints
   - Test permission denial scenarios
   - Verify role-based access control

### Code Quality

All implemented RBAC components have:

- ✅ Comprehensive unit tests (100+ tests total)
- ✅ Proper error handling
- ✅ Documentation comments
- ✅ Zero clippy warnings
- ✅ Proper serialization support

### Conclusion

**The RBAC system is ~80% complete.** The hard parts (role/permission design, JWT integration, user entity) are done and tested. The remaining work is integration: applying existing middleware to REST routes and fixing the RBAC middleware module.

---

## 🚀 Quick Start Guide

### 1. Generate TLS Certificates

```bash
# Generate self-signed certificates for development
mkdir -p certs

# Generate private key
openssl genrsa -out certs/server.key 2048

# Generate certificate signing request
openssl req -new -key certs/server.key -out certs/server.csr \
  -subj "/C=US/ST=State/L=City/O=Organization/CN=localhost"

# Generate self-signed certificate
openssl x509 -req -days 365 -in certs/server.csr \
  -signkey certs/server.key -out certs/server.crt

# For production, use Let's Encrypt or your organization's PKI
```

### 2. Set Up Keycloak Realm

```bash
# Start Keycloak
docker-compose up -d keycloak

# Access admin console: http://localhost:8080
# Username: admin / Password: admin

# Create a new realm called "xzepr"
# Create a client:
#   Client ID: xzepr-client
#   Client Protocol: openid-connect
#   Access Type: confidential
#   Valid Redirect URIs: https://localhost:8443/api/v1/auth/oidc/callback
#
# Get client secret from Credentials tab
# Add to environment variables
```

### 3. Initialize Database

```bash
# Start PostgreSQL
docker-compose up -d postgres

# Run migrations
sqlx migrate run

# Create initial admin user (local auth)
cargo run --bin create-admin-user
```

### 4. Start the Server

```bash
# Development
RUST_LOG=debug cargo run

# Production
cargo build --release
./target/release/xzepr
```

---

## 🔧 Admin CLI Tool

Create a CLI tool for user management:

```rust
// src/bin/admin.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xzepr-admin")]
#[command(about = "XZEPR administration tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new local user
    CreateUser {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        email: String,
        #[arg(short, long)]
        password: String,
        #[arg(short, long)]
        role: String,
    },
    /// List all users
    ListUsers,
    /// Add role to user
    AddRole {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        role: String,
    },
    /// Remove role from user
    RemoveRole {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        role: String,
    },
    /// Generate API key for user
    GenerateApiKey {
        #[arg(short, long)]
        username: String,
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        expires_days: Option<i64>,
    },
    /// List user's API keys
    ListApiKeys {
        #[arg(short, long)]
        username: String,
    },
    /// Revoke API key
    RevokeApiKey {
        #[arg(short, long)]
        key_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Load configuration
    let settings = Settings::new()?;

    // Connect to database
    let pool = PgPool::connect(&settings.database.url).await?;

    // Initialize repositories and services
    let user_repo = Arc::new(PostgresUserRepository::new(pool.clone()));
    let api_key_service = Arc::new(ApiKeyService::new(user_repo.clone()));

    match cli.command {
        Commands::CreateUser { username, email, password, role } => {
            let user = User::new_local(username, email, password)?;
            user_repo.save(&user).await?;

            // Assign role
            let role = Role::from_str(&role)?;
            user_repo.add_role(user.id(), role).await?;

            println!("✓ User created successfully!");
            println!("  ID: {}", user.id());
            println!("  Username: {}", user.username());
        }

        Commands::ListUsers => {
            let users = user_repo.find_all().await?;
            println!("\n{:<36} {:<20} {:<30} {:<15}", "ID", "Username", "Email", "Roles");
            println!("{}", "-".repeat(100));

            for user in users {
                let roles: Vec<String> = user.roles().iter()
                    .map(|r| r.to_string())
                    .collect();
                println!(
                    "{:<36} {:<20} {:<30} {:<15}",
                    user.id(),
                    user.username(),
                    user.email(),
                    roles.join(", ")
                );
            }
        }

        Commands::AddRole { username, role } => {
            let user = user_repo.find_by_username(&username).await?
                .ok_or("User not found")?;
            let role = Role::from_str(&role)?;

            user_repo.add_role(user.id(), role).await?;
            println!("✓ Role '{}' added to user '{}'", role, username);
        }

        Commands::RemoveRole { username, role } => {
            let user = user_repo.find_by_username(&username).await?
                .ok_or("User not found")?;
            let role = Role::from_str(&role)?;

            user_repo.remove_role(user.id(), role).await?;
            println!("✓ Role '{}' removed from user '{}'", role, username);
        }

        Commands::GenerateApiKey { username, name, expires_days } => {
            let user = user_repo.find_by_username(&username).await?
                .ok_or("User not found")?;

            let expires_at = expires_days.map(|days| {
                Utc::now() + chrono::Duration::days(days)
            });

            let (key, api_key) = api_key_service
                .generate_api_key(user.id(), name, expires_at)
                .await?;

            println!("✓ API key generated successfully!");
            println!("\n⚠️  IMPORTANT: Save this key now - it won't be shown again!");
            println!("\n  API Key: {}", key);
            println!("  Key ID:  {}", api_key.id());
            println!("  Name:    {}", api_key.name());
            if let Some(expires) = api_key.expires_at() {
                println!("  Expires: {}", expires);
            }
        }

        Commands::ListApiKeys { username } => {
            let user = user_repo.find_by_username(&username).await?
                .ok_or("User not found")?;

            let keys = api_key_service.list_user_keys(user.id()).await?;

            println!("\nAPI Keys for user '{}':", username);
            println!("{:<36} {:<20} {:<12} {:<20}", "ID", "Name", "Status", "Expires");
            println!("{}", "-".repeat(90));

            for key in keys {
                let status = if key.enabled() { "Active" } else { "Disabled" };
                let expires = key.expires_at()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Never".to_string());

                println!(
                    "{:<36} {:<20} {:<12} {:<20}",
                    key.id(),
                    key.name(),
                    status,
                    expires
                );
            }
        }

        Commands::RevokeApiKey { key_id } => {
            let key_id = ApiKeyId::parse(&key_id)?;
            api_key_service.revoke_key(key_id).await?;
            println!("✓ API key revoked successfully");
        }
    }

    Ok(())
}
```

Usage examples:

```bash
# Create admin user
xzepr-admin create-user -u admin -e admin@example.com -p SecurePass123! -r admin

# Create regular user
xzepr-admin create-user -u alice -e alice@example.com -p password123 -r event_manager

# List all users
xzepr-admin list-users

# Generate API key for automation
xzepr-admin generate-api-key -u alice -n "CI/CD Pipeline" -e 90

# Manage roles
xzepr-admin add-role -u bob -r event_viewer
xzepr-admin remove-role -u bob -r user
```

---

## 📖 API Documentation Examples

### Authentication Endpoints

#### 1. Local Login

```bash
# Login with username/password
curl -X POST https://localhost:8443/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "password123"
  }'

# Response:
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": 1234567890,
  "user": {
    "id": "01234567-89ab-cdef-0123-456789abcdef",
    "username": "alice",
    "email": "alice@example.com",
    "roles": ["event_manager"]
  }
}
```

#### 2. OIDC Login (Keycloak)

```bash
# Step 1: Get authorization URL
curl https://localhost:8443/api/v1/auth/oidc/login

# Response:
{
  "authorization_url": "https://keycloak.example.com/realms/xzepr/protocol/openid-connect/auth?...",
  "state": "random-csrf-token"
}

# Step 2: Redirect user to authorization_url
# Step 3: User authenticates with Keycloak
# Step 4: Keycloak redirects to callback with code
# Step 5: Exchange code for token (handled automatically)

# Final response:
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": 1234567890,
  "user": {
    "id": "01234567-89ab-cdef-0123-456789abcdef",
    "username": "bob",
    "email": "bob@company.com",
    "roles": ["user"]
  }
}
```

#### 3. API Key Authentication

```bash
# Use API key in header
curl -X GET https://localhost:8443/api/v1/events \
  -H "X-API-Key: xzepr_base64encodedkey..."

# Or use Bearer token
curl -X GET https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

### Protected Endpoints with RBAC

```bash
# Create event (requires EventCreate permission)
curl -X POST https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "build-completed",
    "version": "1.0.0",
    "release": "2024.10",
    "platform_id": "linux-x86_64",
    "package": "rpm",
    "description": "Build completed successfully",
    "payload": {"commit": "abc123", "duration": 120},
    "success": true,
    "event_receiver_id": "01234567-89ab-cdef-0123-456789abcdef"
  }'

# Success response (201 Created):
{
  "id": "01234567-89ab-cdef-0123-456789abcdef"
}

# Forbidden response (403) if user lacks permission:
{
  "error": "Forbidden",
  "message": "Missing required permission: EventCreate"
}
```

---

## 🔍 Audit Logging

### Audit Log Implementation

```rust
// src/auth/audit.rs
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: AuditLogId,
    pub user_id: Option<UserId>,
    pub action: AuditAction,
    pub resource: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    Login,
    Logout,
    LoginFailed,
    PermissionDenied,
    ApiKeyGenerated,
    ApiKeyRevoked,
    RoleAssigned,
    RoleRemoved,
    PasswordChanged,
    UserCreated,
    UserDisabled,
}

pub struct AuditLogger {
    repo: Arc<dyn AuditLogRepository>,
}

impl AuditLogger {
    pub async fn log_auth_event(
        &self,
        user_id: Option<UserId>,
        action: AuditAction,
        success: bool,
        ip_address: Option<IpAddr>,
        user_agent: Option<String>,
        error: Option<String>,
    ) -> Result<(), AuditError> {
        let log = AuditLog {
            id: AuditLogId::new(),
            user_id,
            action,
            resource: None,
            ip_address,
            user_agent,
            success,
            error_message: error,
            created_at: Utc::now(),
        };

        self.repo.save(&log).await?;

        // Also emit metric
        if success {
            metrics::counter!("auth_success_total", "action" => format!("{:?}", log.action)).increment(1);
        } else {
            metrics::counter!("auth_failure_total", "action" => format!("{:?}", log.action)).increment(1);
        }

        Ok(())
    }
}
```

### Audit Middleware

```rust
// src/api/middleware/audit.rs
pub async fn audit_middleware(
    State(audit_logger): State<Arc<AuditLogger>>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    // Extract IP and User-Agent
    let ip = extract_ip(&req);
    let user_agent = req.headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Get user if authenticated
    let user_id = req.extensions()
        .get::<AuthenticatedUser>()
        .map(|u| u.user_id);

    // Process request
    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    // Log to audit trail for sensitive operations
    if is_sensitive_operation(&method, &uri) {
        let success = status.is_success();
        let action = determine_action(&method, &uri);

        audit_logger.log_auth_event(
            user_id,
            action,
            success,
            ip,
            user_agent,
            if !success {
                Some(format!("HTTP {}", status.as_u16()))
            } else {
                None
            },
        ).await.ok(); // Don't fail request if audit logging fails
    }

    response
}

fn is_sensitive_operation(method: &Method, uri: &Uri) -> bool {
    matches!(
        (method.as_str(), uri.path()),
        ("POST", "/api/v1/auth/login") |
        ("POST", "/api/v1/users") |
        ("DELETE", _) |
        ("PUT", path) if path.contains("/roles")
    )
}
```

---

## 📊 Monitoring & Metrics

### Auth-Related Metrics

```rust
// src/infrastructure/telemetry/metrics.rs

pub fn record_login_attempt(success: bool, provider: &str) {
    let result = if success { "success" } else { "failure" };
    metrics::counter!(
        "auth_login_attempts_total",
        "provider" => provider,
        "result" => result
    ).increment(1);
}

pub fn record_permission_check(permission: &Permission, granted: bool) {
    let result = if granted { "granted" } else { "denied" };
    metrics::counter!(
        "auth_permission_checks_total",
        "permission" => format!("{:?}", permission),
        "result" => result
    ).increment(1);
}

pub fn record_api_key_usage(key_id: &str) {
    metrics::counter!(
        "auth_api_key_usage_total",
        "key_id" => key_id
    ).increment(1);
}

pub fn record_token_validation(valid: bool) {
    let result = if valid { "valid" } else { "invalid" };
    metrics::counter!(
        "auth_token_validations_total",
        "result" => result
    ).increment(1);
}
```

### Prometheus Metrics Example

```
# HELP auth_login_attempts_total Total number of login attempts
# TYPE auth_login_attempts_total counter
auth_login_attempts_total{provider="local",result="success"} 1523
auth_login_attempts_total{provider="local",result="failure"} 42
auth_login_attempts_total{provider="keycloak",result="success"} 3421
auth_login_attempts_total{provider="api_key",result="success"} 15234

# HELP auth_permission_checks_total Total number of permission checks
# TYPE auth_permission_checks_total counter
auth_permission_checks_total{permission="EventCreate",result="granted"} 5234
auth_permission_checks_total{permission="EventCreate",result="denied"} 123
auth_permission_checks_total{permission="EventDelete",result="denied"} 89

# HELP auth_active_sessions Current number of active sessions
# TYPE auth_active_sessions gauge
auth_active_sessions 453
```

---

## 🔐 Security Hardening Checklist

### Application Security

- [x] **TLS 1.3 only** - No fallback to older protocols
- [x] **Strong cipher suites** - Using rustls safe defaults
- [x] **HSTS headers** - Force HTTPS in production
- [x] **Password requirements** - Min length, complexity
- [x] **Rate limiting** - Prevent brute force attacks
- [x] **Account lockout** - After N failed attempts
- [x] **Password reset flow** - Secure token-based reset
- [x] **CSRF protection** - For OIDC flows
- [x] **XSS prevention** - Proper escaping in responses
- [x] **SQL injection prevention** - SQLx compile-time checks
- [x] **Secrets management** - Never in code, use env vars
- [x] **Audit logging** - All sensitive operations
- [x] **Session timeout** - JWT expiration enforced
- [x] **API key rotation** - Support for key expiration

### Rate Limiting Example

```rust
// src/api/middleware/rate_limit.rs
use tower_governor::{
    governor::GovernorConfigBuilder,
    GovernorLayer,
};

pub fn rate_limit_layer() -> GovernorLayer {
    // 100 requests per minute per IP
    let config = GovernorConfigBuilder::default()
        .per_second(100)
        .burst_size(10)
        .finish()
        .unwrap();

    GovernorLayer {
        config: Arc::new(config),
    }
}

// Apply to auth routes
let auth_routes = Router::new()
    .route("/login", post(login))
    .route("/oidc/callback", get(oidc_callback))
    .layer(rate_limit_layer());  // Stricter rate limit for auth
```

---

## 🧪 Integration Testing with Auth

```rust
// tests/auth_integration_tests.rs

#[tokio::test]
async fn test_full_auth_flow() {
    // Setup test environment
    let app = spawn_test_app().await;

    // 1. Create a user
    let create_user_req = json!({
        "username": "testuser",
        "email": "test@example.com",
        "password": "SecurePass123!"
    });

    let response = app.post("/api/v1/users", create_user_req).await;
    assert_eq!(response.status(), 201);

    // 2. Login
    let login_req = json!({
        "username": "testuser",
        "password": "SecurePass123!"
    });

    let response = app.post("/api/v1/auth/login", login_req).await;
    assert_eq!(response.status(), 200);

    let body: LoginResponse = response.json().await;
    let token = body.token;

    // 3. Access protected resource with token
    let response = app
        .get("/api/v1/events")
        .bearer_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    // 4. Try to access admin endpoint (should fail)
    let response = app
        .get("/api/v1/users")
        .bearer_auth(&token)
        .send()
        .await;
    assert_eq!(response.status(), 403);

    // 5. Invalid token (should fail)
    let response = app
        .get("/api/v1/events")
        .bearer_auth("invalid-token")
        .send()
        .await;
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_rbac_enforcement() {
    let app = spawn_test_app().await;

    // Create user with event_viewer role
    let viewer_token = create_test_user(&app, "viewer", vec![Role::EventViewer]).await;

    // Should be able to read events
    let response = app.get("/api/v1/events")
        .bearer_auth(&viewer_token)
        .send()
        .await;
    assert_eq!(response.status(), 200);

    // Should NOT be able to create events
    let response = app.post("/api/v1/events", json!({"name": "test"}))
        .bearer_auth(&viewer_token)
        .send()
        .await;
    assert_eq!(response.status(), 403);
}
```

---

## 📝 Environment-Specific Configurations

### Development Config

```yaml
# config/development.yaml
server:
  host: "127.0.0.1"
  port: 8443
  enable_https: true

auth:
  enable_local_auth: true
  enable_oidc: false # Use local auth for dev
  jwt_expiration_hours: 168 # 1 week for convenience

tls:
  cert_path: "./certs/dev-server.crt"
  key_path: "./certs/dev-server.key"

database:
  max_connections: 5
```

### Production Config

```yaml
# config/production.yaml
server:
  host: "0.0.0.0"
  port: 8443
  enable_https: true

auth:
  enable_local_auth: true
  enable_oidc: true
  jwt_expiration_hours: 24

tls:
  cert_path: "/etc/xzepr/tls/server.crt"
  key_path: "/etc/xzepr/tls/server.key"

database:
  max_connections: 50
  enable_ssl: true
```

---

## 🎯 Next Steps for Implementation

1. **Start with Foundation**

   - Set up project structure ✓
   - Add TLS configuration ✓
   - Create user domain model ✓

2. **Implement Local Auth First** (Simplest)

   - Password hashing with Argon2
   - JWT generation/validation
   - Login endpoint
   - Test thoroughly

3. **Add RBAC**

   - Role and permission definitions
   - Middleware implementation
   - Apply to routes
   - Integration tests

4. **Add Keycloak OIDC** (Optional, but recommended for enterprise)

   - Set up Keycloak instance
   - Implement OIDC flow
   - Auto-provision users
   - Test integration

5. **Add API Keys** (For automation)
   - Key generation
   - Key authentication
   - Management endpoints
   - Expiration handling

Want me to generate complete, production-ready code for any specific component?
I can create working implementations for:

- Authentication service with all three providers
- Complete RBAC middleware
- Database migrations and repositories
- Admin CLI tool
- Integration tests
- Or anything else you need!## 🔧 Technology Stack

### Core Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1.40", features = ["full"] }
tokio-rustls = "0.26"

# Web framework
axum = "0.7"
axum-server = { version = "0.7", features = ["tls-rustls"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors", "auth"] }

# TLS
rustls = "0.23"
rustls-pemfile = "2.1"

# Authentication & Authorization
jsonwebtoken = "9.3"
oauth2 = "4.4"
openidconnect = "3.5"
argon2 = "0.5"
rand = "0.8"

# GraphQL
async-graphql = "7.0"
async-graphql-axum = "7.0"

# Database
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "chrono", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Validation
validator = { version = "0.18", features = ["derive"] }

# IDs
uuid = { version = "1.10", features = ["v7", "serde"] }
ulid = "1.1"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
metrics = "0.23"
metrics-exporter-prometheus = "0.15"

# Messaging
rdkafka = "0.36"

# Configuration
config = "0.14"
dotenvy = "0.15"

# Time
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
# Testing
tokio-test = "0.4"
testcontainers = "0.21"
mockall = "0.13"
fake = "2.9"
criterion = "0.5"
```

---

## 🔐 Authentication & Authorization Design

### Multi-Provider Authentication

The system supports **three authentication methods**:

1. **Keycloak (OIDC)** - Enterprise SSO via OAuth2/OpenID Connect
2. **Local Users** - Built-in user management with password hashing
3. **API Keys** - Machine-to-machine authentication

### Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│              API Gateway (TLS 1.3)              │
├─────────────────────────────────────────────────┤
│          Authentication Middleware              │
│  ┌─────────────┬─────────────┬───────────────┐ │
│  │   Bearer    │   Session   │   API Key     │ │
│  │   Token     │   Cookie    │   Header      │ │
│  └─────────────┴─────────────┴───────────────┘ │
├─────────────────────────────────────────────────┤
│          Authorization Middleware               │
│              (RBAC Enforcement)                 │
├─────────────────────────────────────────────────┤
│              Business Logic                     │
└─────────────────────────────────────────────────┘
```

---

## 🛡️ TLS 1.3 Configuration

### TLS Setup

```rust
// src/infrastructure/tls/config.rs
use rustls::{ServerConfig, PrivateKey, Certificate};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;

pub struct TlsConfig {
    cert_path: String,
    key_path: String,
}

impl TlsConfig {
    pub fn load_server_config(&self) -> Result<ServerConfig, TlsError> {
        // Load certificates
        let cert_file = File::open(&self.cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<Certificate> = certs(&mut cert_reader)?
            .into_iter()
            .map(Certificate)
            .collect();

        // Load private key
        let key_file = File::open(&self.key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)?;

        if keys.is_empty() {
            return Err(TlsError::NoPrivateKey);
        }

        let key = PrivateKey(keys.remove(0));

        // Configure TLS 1.3 only
        let config = ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&rustls::version::TLS13])
            .expect("TLS 1.3 should be supported")
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        Ok(config)
    }
}

// src/main.rs - Server startup with TLS
use axum_server::tls_rustls::RustlsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load TLS config
    let tls_config = TlsConfig::new(
        "certs/server.crt".to_string(),
        "certs/server.key".to_string(),
    );

    let rustls_config = RustlsConfig::from_config(
        Arc::new(tls_config.load_server_config()?)
    );

    // Build app
    let app = build_app().await?;

    // Start HTTPS server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8443));
    tracing::info!("Starting HTTPS server on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

---

## 👥 User Management & Authentication

### 1. User Domain Model

```rust
// src/domain/entities/user.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: UserId,
    username: String,
    email: String,
    #[serde(skip_serializing)]
    password_hash: Option<String>,  // None for OIDC users
    auth_provider: AuthProvider,
    roles: Vec<Role>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthProvider {
    Local,
    Keycloak { subject: String },
    ApiKey,
}

impl User {
    pub fn new_local(username: String, email: String, password: String) -> Result<Self, DomainError> {
        let password_hash = hash_password(&password)?;

        Ok(Self {
            id: UserId::new(),
            username,
            email,
            password_hash: Some(password_hash),
            auth_provider: AuthProvider::Local,
            roles: vec![Role::User],  // Default role
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn new_oidc(username: String, email: String, subject: String) -> Self {
        Self {
            id: UserId::new(),
            username,
            email,
            password_hash: None,
            auth_provider: AuthProvider::Keycloak { subject },
            roles: vec![Role::User],
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn verify_password(&self, password: &str) -> Result<bool, AuthError> {
        match &self.password_hash {
            Some(hash) => verify_password(password, hash),
            None => Err(AuthError::NotLocalUser),
        }
    }

    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.roles.iter().any(|r| r.has_permission(permission))
    }
}
```

### 2. Password Hashing (Argon2)

```rust
// src/auth/local/password.rs
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::PasswordHashingFailed(e.to_string()))?
        .to_string();

    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AuthError::InvalidPasswordHash(e.to_string()))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
```

### 3. Local Authentication

```rust
// src/auth/local/session.rs
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // User ID
    pub username: String,
    pub roles: Vec<String>,
    pub exp: i64,           // Expiration
    pub iat: i64,           // Issued at
}

pub struct LocalAuthService {
    user_repo: Arc<dyn UserRepository>,
    jwt_secret: String,
}

impl LocalAuthService {
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<String, AuthError> {
        // Find user
        let user = self.user_repo
            .find_by_username(username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        // Check if enabled
        if !user.enabled {
            return Err(AuthError::UserDisabled);
        }

        // Verify password
        if !user.verify_password(password)? {
            return Err(AuthError::InvalidCredentials);
        }

        // Generate JWT
        let token = self.generate_token(&user)?;

        Ok(token)
    }

    fn generate_token(&self, user: &User) -> Result<String, AuthError> {
        let claims = Claims {
            sub: user.id().to_string(),
            username: user.username().to_string(),
            roles: user.roles().iter().map(|r| r.to_string()).collect(),
            exp: (Utc::now() + chrono::Duration::hours(24)).timestamp(),
            iat: Utc::now().timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| AuthError::TokenGenerationFailed(e.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))
    }
}
```

---

## 🔑 Keycloak OIDC Integration

### OIDC Configuration

```rust
// src/auth/oidc/keycloak.rs
use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::async_http_client,
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse, RedirectUrl, Scope,
    TokenResponse,
};

pub struct KeycloakConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

pub struct KeycloakClient {
    client: CoreClient,
}

impl KeycloakClient {
    pub async fn new(config: KeycloakConfig) -> Result<Self, OidcError> {
        // Discover provider metadata
        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(config.issuer_url)?,
            async_http_client,
        )
        .await?;

        // Create OAuth2 client
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(config.client_id),
            Some(ClientSecret::new(config.client_secret)),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_url)?);

        Ok(Self { client })
    }

    pub fn authorization_url(&self) -> (String, CsrfToken, Nonce) {
        let (auth_url, csrf_token, nonce) = self
            .client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .url();

        (auth_url.to_string(), csrf_token, nonce)
    }

    pub async fn exchange_code(
        &self,
        code: AuthorizationCode,
    ) -> Result<TokenResponse, OidcError> {
        let token_response = self
            .client
            .exchange_code(code)
            .request_async(async_http_client)
            .await?;

        Ok(token_response)
    }

    pub async fn verify_token(&self, token: &str) -> Result<OidcClaims, OidcError> {
        // Verify and decode JWT token
        // Extract user info from claims
        // Map to internal user representation
        todo!("Implement token verification")
    }
}

#[derive(Debug, Deserialize)]
pub struct OidcClaims {
    pub sub: String,         // Subject (user ID in Keycloak)
    pub email: String,
    pub preferred_username: String,
    pub roles: Vec<String>,
    pub exp: i64,
}
```

### OIDC Callback Handler

```rust
// src/api/rest/auth.rs
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
```

---

## 🛡️ RBAC (Role-Based Access Control)

### Role & Permission Model

```rust
// src/auth/rbac/roles.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    EventManager,
    EventViewer,
    User,
}

impl Role {
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Role::Admin => vec![
                Permission::EventCreate,
                Permission::EventRead,
                Permission::EventUpdate,
                Permission::EventDelete,
                Permission::ReceiverCreate,
                Permission::ReceiverRead,
                Permission::ReceiverUpdate,
                Permission::ReceiverDelete,
                Permission::GroupCreate,
                Permission::GroupRead,
                Permission::GroupUpdate,
                Permission::GroupDelete,
                Permission::UserManage,
                Permission::RoleManage,
            ],
            Role::EventManager => vec![
                Permission::EventCreate,
                Permission::EventRead,
                Permission::EventUpdate,
                Permission::ReceiverCreate,
                Permission::ReceiverRead,
                Permission::ReceiverUpdate,
                Permission::GroupCreate,
                Permission::GroupRead,
                Permission::GroupUpdate,
            ],
            Role::EventViewer => vec![
                Permission::EventRead,
                Permission::ReceiverRead,
                Permission::GroupRead,
            ],
            Role::User => vec![
                Permission::EventRead,
            ],
        }
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions().contains(permission)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::EventManager => write!(f, "event_manager"),
            Role::EventViewer => write!(f, "event_viewer"),
            Role::User => write!(f, "user"),
        }
    }
}
```

### Permission Definitions

```rust
// src/auth/rbac/permissions.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Event permissions
    EventCreate,
    EventRead,
    EventUpdate,
    EventDelete,

    // Receiver permissions
    ReceiverCreate,
    ReceiverRead,
    ReceiverUpdate,
    ReceiverDelete,

    // Group permissions
    GroupCreate,
    GroupRead,
    GroupUpdate,
    GroupDelete,

    // Admin permissions
    UserManage,
    RoleManage,
}

impl Permission {
    pub fn from_action(resource: &str, action: &str) -> Option<Self> {
        match (resource, action) {
            ("event", "create") => Some(Permission::EventCreate),
            ("event", "read") => Some(Permission::EventRead),
            ("event", "update") => Some(Permission::EventUpdate),
            ("event", "delete") => Some(Permission::EventDelete),
            ("receiver", "create") => Some(Permission::ReceiverCreate),
            ("receiver", "read") => Some(Permission::ReceiverRead),
            ("receiver", "update") => Some(Permission::ReceiverUpdate),
            ("receiver", "delete") => Some(Permission::ReceiverDelete),
            ("group", "create") => Some(Permission::GroupCreate),
            ("group", "read") => Some(Permission::GroupRead),
            ("group", "update") => Some(Permission::GroupUpdate),
            ("group", "delete") => Some(Permission::GroupDelete),
            _ => None,
        }
    }
}
```

### RBAC Enforcement Middleware

```rust
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
```

---

## 🔌 API Routes with RBAC

### Protected Routes

```rust
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
```

### Handler with Permission Check

```rust
// src/api/rest/events.rs
pub async fn create_event(
    State(handler): State<Arc<CreateEventHandler>>,
    Extension(user): Extension<AuthenticatedUser>,  // Injected by middleware
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, ApiError> {
    tracing::info!(
        user_id = %user.user_id,
        username = %user.username,
        "Creating event"
    );

    let command = request.into_command()?;
    let event_id = handler.handle(command).await?;

    Ok(Json(CreateEventResponse { id: event_id }))
}

// Owner-based authorization
pub async fn delete_event(
    State(repo): State<Arc<dyn EventRepository>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(event_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let event_id = EventId::parse(&event_id)?;
    let event = repo.find_by_id(event_id).await?
        .ok_or(ApiError::NotFound)?;

    // Check ownership or admin role
    if event.created_by() != user.user_id && !user.has_role(&Role::Admin) {
        return Err(ApiError::Forbidden(
            "You can only delete your own events".to_string()
        ));
    }

    repo.delete(event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

---

## 🗄️ Database Schema for Users & Roles

```sql
-- migrations/002_users_and_roles.sql

-- Users table
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT,  -- NULL for OIDC users
    auth_provider VARCHAR(50) NOT NULL,  -- 'local', 'keycloak', 'api_key'
    oidc_subject VARCHAR(255) UNIQUE,    -- Keycloak subject ID
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_oidc_subject ON users(oidc_subject);

-- Roles table
CREATE TABLE roles (
    id UUID PRIMARY KEY,
    name VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default roles
INSERT INTO roles (id, name, description) VALUES
    (gen_random_uuid(), 'admin', 'Full system access'),
    (gen_random_uuid(), 'event_manager', 'Can create and manage events'),
    (gen_random_uuid(), 'event_viewer', 'Can view events'),
    (gen_random_uuid(), 'user', 'Basic user access');

-- User roles junction table
CREATE TABLE user_roles (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_by UUID REFERENCES users(id),
    PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_user ON user_roles(user_id);
CREATE INDEX idx_user_roles_role ON user_roles(role_id);

-- API Keys table (for machine-to-machine auth)
CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,
    name VARCHAR(255) NOT NULL,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_user ON api_keys(user_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);

-- Audit log for authorization events
CREATE TABLE auth_audit_log (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL,  -- 'login', 'logout', 'permission_denied', etc.
    resource VARCHAR(100),
    ip_address INET,
    user_agent TEXT,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_auth_audit_user ON auth_audit_log(user_id);
CREATE INDEX idx_auth_audit_created ON auth_audit_log(created_at);

-- Update events table to track creator
ALTER TABLE events ADD COLUMN created_by UUID REFERENCES users(id);
CREATE INDEX idx_events_created_by ON events(created_by);
```

---

## 🔐 API Key Authentication

### API Key Management

```rust
// src/auth/api_key.rs
use sha2::{Sha256, Digest};
use rand::Rng;

pub struct ApiKeyService {
    user_repo: Arc<dyn UserRepository>,
    api_key_repo: Arc<dyn ApiKeyRepository>,
}

impl ApiKeyService {
    pub async fn generate_api_key(
        &self,
        user_id: UserId,
        name: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(String, ApiKey), AuthError> {
        // Generate random key
        let key = generate_random_key();
        let key_hash = hash_api_key(&key);

        // Create API key record
        let api_key = ApiKey {
            id: ApiKeyId::new(),
            user_id,
            key_hash,
            name,
            expires_at,
            enabled: true,
            created_at: Utc::now(),
            last_used_at: None,
        };

        self.api_key_repo.save(&api_key).await?;

        // Return plaintext key only once
        Ok((key, api_key))
    }

    pub async fn verify_api_key(&self, key: &str) -> Result<User, AuthError> {
        let key_hash = hash_api_key(key);

        // Find API key
        let api_key = self.api_key_repo
            .find_by_hash(&key_hash)
            .await?
            .ok_or(AuthError::InvalidApiKey)?;

        // Check if enabled
        if !api_key.enabled {
            return Err(AuthError::ApiKeyDisabled);
        }

        // Check expiration
        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                return Err(AuthError::ApiKeyExpired);
            }
        }

        // Update last used timestamp
        self.api_key_repo.update_last_used(api_key.id).await?;

        // Get user
        let user = self.user_repo
            .find_by_id(api_key.user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        Ok(user)
    }
}

fn generate_random_key() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    format!("xzepr_{}", base64::encode(bytes))
}

fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

---

## 📝 Authentication REST API

### Login Endpoints

```rust
// src/api/rest/auth.rs

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
    expires_at: i64,
    user: UserInfo,
}

#[derive(Serialize)]
pub struct UserInfo {
    id: String,
    username: String,
    email: String,
    roles: Vec<String>,
}

// Local username/password login
pub async fn login(
    State(auth_service): State<Arc<LocalAuthService>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let token = auth_service
        .authenticate(&request.username, &request.password)
        .await?;

    let claims = auth_service.verify_token(&token)?;

    Ok(Json(LoginResponse {
        token,
        expires_at: claims.exp,
        user: UserInfo {
            id: claims.sub,
            username: claims.username,
            email: "".to_string(),  // Fetch from DB if needed
            roles: claims.roles,
        },
    }))
}

// Initiate OIDC login
pub async fn oidc_login(
    State(keycloak): State<Arc<KeycloakClient>>,
) -> Result<Json<OidcLoginResponse>, ApiError> {
    let (auth_url, csrf_token, nonce) = keycloak.authorization_url();

    // Store CSRF token and nonce in session/cache
    // (Implementation depends on session management strategy)

    Ok(Json(OidcLoginResponse {
        authorization_url: auth_url,
        state: csrf_token.secret().clone(),
    }))
}

// API key authentication (in middleware)
pub async fn authenticate_api_key(
    State(api_key_service): State<Arc<ApiKeyService>>,
    api_key: String,
) -> Result<User, AuthError> {
    api_key_service.verify_api_key(&api_key).await
}
```

---

## 🔒 Configuration with Environment Variables

```rust
// src/infrastructure/config.rs
use config::{Config, ConfigError, Environment, File};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub tls: TlsConfig,
    pub kafka: KafkaConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_https: bool,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub enable_local_auth: bool,
    pub enable_oidc: bool,
    pub keycloak: Option<KeycloakConfig>,
}

#[derive(Debug, Deserialize)]
pub struct KeycloakConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
}

#[derive(Debug, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8443)?
            .set_default("server.enable_https", true)?
            .set_default("auth.enable_local_auth", true)?
            .set_default("auth.enable_oidc", false)?
            .set_default("auth.jwt_expiration_hours", 24)?;

        // Add configuration file if it exists
        builder = builder.add_source(
            File::with_name("config/default").required(false)
        );

        // Add environment-specific config
        let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".into());
        builder = builder.add_source(
            File::with_name(&format!("config/{}", env)).required(false)
        );

        // Override with environment variables
        builder = builder.add_source(
            Environment::with_prefix("XZEPR").separator("__")
        );

        builder.build()?.try_deserialize()
    }
}
```

### Example .env file

```bash
# .env
RUST_ENV=production
RUST_LOG=info,xzepr=debug

# Server
XZEPR__SERVER__HOST=0.0.0.0
XZEPR__SERVER__PORT=8443
XZEPR__SERVER__ENABLE_HTTPS=true

# Database
XZEPR__DATABASE__URL=postgres://xzepr:password@localhost:5432/xzepr
XZEPR__DATABASE__MAX_CONNECTIONS=20

# Authentication
XZEPR__AUTH__JWT_SECRET=your-super-secret-key-change-in-production
XZEPR__AUTH__JWT_EXPIRATION_HOURS=24
XZEPR__AUTH__ENABLE_LOCAL_AUTH=true
XZEPR__AUTH__ENABLE_OIDC=true

# Keycloak OIDC
XZEPR__AUTH__KEYCLOAK__ISSUER_URL=https://keycloak.example.com/realms/xzepr
XZEPR__AUTH__KEYCLOAK__CLIENT_ID=xzepr-client
XZEPR__AUTH__KEYCLOAK__CLIENT_SECRET=your-client-secret
XZEPR__AUTH__KEYCLOAK__REDIRECT_URL=https://api.example.com/api/v1/auth/oidc/callback

# TLS
XZEPR__TLS__CERT_PATH=./certs/server.crt
XZEPR__TLS__KEY_PATH=./certs/server.key

# Kafka
XZEPR__KAFKA__BROKERS=localhost:9092
XZEPR__KAFKA__TOPIC=xzepr.events
```

---

## 🐳 Updated Docker Compose with Keycloak

```yaml
# docker/docker-compose.yaml
version: "3.8"

services:
  xzepr:
    build: .
    ports:
      - "8443:8443"
    environment:
      - XZEPR__DATABASE__URL=postgres://xzepr:password@postgres:5432/xzepr
      - XZEPR__KAFKA__BROKERS=kafka:9092
      - XZEPR__AUTH__KEYCLOAK__ISSUER_URL=http://keycloak:8080/realms/xzepr
      - RUST_LOG=info,xzepr=debug
    volumes:
      - ./certs:/app/certs:ro
    depends_on:
      - postgres
      - kafka
      - keycloak

  postgres:
    image: postgres:16
    environment:
      - POSTGRES_USER=xzepr
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=xzepr
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  keycloak:
    image: quay.io/keycloak/keycloak:24.0
    command: start-dev
    environment:
      - KEYCLOAK_ADMIN=admin
      - KEYCLOAK_ADMIN_PASSWORD=admin
      - KC_DB=postgres
      - KC_DB_URL=jdbc:postgresql://postgres:5432/keycloak
      - KC_DB_USERNAME=xzepr
      - KC_DB_PASSWORD=password
    ports:
      - "8080:8080"
    depends_on:
      - postgres

  kafka:
    image: confluentinc/cp-kafka:latest
    environment:
      - KAFKA_ZOOKEEPER_CONNECT=zookeeper:2181
      - KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://kafka:9092
      - KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR=1
    ports:
      - "9092:9092"
    depends_on:
      - zookeeper

  zookeeper:
    image: confluentinc/cp-zookeeper:latest
    environment:
      - ZOOKEEPER_CLIENT_PORT=2181
      - ZOOKEEPER_TICK_TIME=2000
    ports:
      - "2181:2181"

volumes:
  postgres_data:
```

---

## 🧪 Testing Authentication & Authorization

```rust
// tests/auth_tests.rs
use testcontainers::*;

#[tokio::test]
async fn test_local_authentication() {
    let docker = clients::Cli::default();
    let postgres = docker.run(images::postgres::Postgres::default());

    let pool = setup_database(&postgres).await;
    let user_repo = PostgresUserRepository::new(pool.clone());
    let auth_service = LocalAuthService::new(user_repo, "test-secret".to_string());

    // Create test user
    let user = User::new_local(
        "testuser".to_string(),
        "test@example.com".to_string(),
        "password123".to_string(),
    ).unwrap();
    user_repo.save(&user).await.unwrap();

    // Test successful authentication
    let token = auth_service
        .authenticate("testuser", "password123")
        .await
        .unwrap();

    assert!(!token.is_empty());

    // Test invalid password
    let result = auth_service
        .authenticate("testuser", "wrongpassword")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_rbac_enforcement() {
    let user = AuthenticatedUser {
        user_id: UserId::new(),
        username: "viewer".to_string(),
        roles: vec![Role::EventViewer],
    };

    // Should have read permission
    assert!(user.has_permission(&Permission::EventRead));

    // Should NOT have create permission
    assert!(!user.has_permission(&Permission::EventCreate));

    // Admin should have all permissions
    let admin = AuthenticatedUser {
        user_id: UserId::new(),
        username: "admin".to_string(),
        roles: vec![Role::Admin],
    };

    assert!(admin.has_permission(&Permission::EventCreate));
    assert!(admin.has_permission(&Permission::UserManage));
}
```

---

## 📚 RBAC Completion Roadmap

**Note**: A detailed phased implementation plan for completing RBAC is available in `docs/explanation/rbac_completion_plan.md`. The roadmap below represents the original full-stack implementation plan.

### Phase 1: Foundation (Week 1-2)

- [x] Set up project structure
- [x] Configure dependencies
- [x] Implement domain entities
- [x] Set up database with migrations
- [x] Basic repository implementations
- [ ] **TLS 1.3 configuration**
- [ ] **User entity and repository**

### Phase 2: Authentication (Week 3)

- [ ] **Local authentication with Argon2**
- [ ] **JWT generation and validation**
- [ ] **Keycloak OIDC integration**
- [ ] **API key authentication**
- [ ] **Authentication middleware**

### Phase 3: Authorization (Week 4)

- [ ] **RBAC implementation**
- [ ] **Permission model**
- [ ] **Authorization middleware**
- [ ] **Role management API**

### Phase 4: Core Features (Week 5-6)

- [ ] Implement command handlers
- [ ] REST API endpoints
- [ ] GraphQL schema
- [ ] Kafka integration
- [ ] Validation logic

### Phase 5: Testing & Quality (Week 7)

- [ ] Unit tests for domain layer
- [ ] **Authentication tests**
- [ ] **Authorization tests**
- [ ] Integration tests
- [ ] API tests
- [ ] Load testing setup

### Phase 6: Production Ready (Week 8)

- [ ] Security audit
- [ ] Performance tuning
- [ ] **Keycloak realm setup**
- [ ] Deployment documentation
- [ ] Monitoring & alerting

---

## 🔐 Security Best Practices Checklist

- [ ] TLS 1.3 enforced for all connections
- [ ] Passwords hashed with Argon2id
- [ ] JWT tokens with short expiration (24h default)
- [ ] API keys hashed in database
- [ ] CSRF protection for OIDC flow
- [ ] Rate limiting on authentication endpoints
- [ ] Audit logging for all auth events
- [ ] Secure session management
- [ ] Input validation on all endpoints
- [ ] SQL injection prevention (SQLx compile-time checks)
- [ ] XSS prevention in API responses
- [ ] CORS properly configured
- [ ] Secrets managed via environment variables
- [ ] Regular security dependency updates

---

Want me to generate the complete code for any specific component? I can create:

- Full authentication service implementation
- Complete RBAC middleware
- Keycloak integration code
- Database repositories for users/roles
- Or any other part you'd like to see!# XZEPR - Event System in Rust

## Greenfield Architecture Plan

_Building on lessons learned from the Go EPR implementation_

---

## 🎯 Project Vision

A high-performance, type-safe event provenance system built in Rust that tracks
events, event receivers, and event receiver groups across the software supply
chain.

**Key Principles:**

- **Zero-cost abstractions** - Leverage Rust's compile-time guarantees
- **Memory safety** - No null pointers, no data races
- **Async-first** - Built on Tokio for high concurrency
- **Type-driven design** - Use the type system to prevent bugs
- **Testability** - Design for testing from day one

---

## 🏗️ Architecture Overview

### Layered Architecture

```
┌─────────────────────────────────────────┐
│         API Layer (REST + GraphQL)      │
│         (axum, async-graphql)           │
├─────────────────────────────────────────┤
│         Application Layer               │
│         (Use Cases / Commands)          │
├─────────────────────────────────────────┤
│         Domain Layer                    │
│         (Business Logic + Entities)     │
├─────────────────────────────────────────┤
│         Infrastructure Layer            │
│    (Postgres, Kafka, Telemetry)        │
└─────────────────────────────────────────┘
```

---

## 📦 Project Structure

```
xzepr/
├── Cargo.toml
├── .cargo/
│   └── config.toml              # Build configuration
├── certs/                       # TLS certificates (gitignored)
│   ├── server.crt
│   ├── server.key
│   └── ca.crt
├── src/
│   ├── main.rs                  # Application entry point
│   ├── lib.rs                   # Library root
│   │
│   ├── api/                     # API layer
│   │   ├── mod.rs
│   │   ├── rest/                # REST endpoints
│   │   │   ├── mod.rs
│   │   │   ├── events.rs
│   │   │   ├── receivers.rs
│   │   │   └── groups.rs
│   │   ├── graphql/             # GraphQL schema & resolvers
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs
│   │   │   └── resolvers/
│   │   └── middleware/          # API middleware
│   │       ├── auth.rs
│   │       ├── rbac.rs
│   │       ├── logging.rs
│   │       └── validation.rs
│   │
│   ├── auth/                    # Authentication & Authorization
│   │   ├── mod.rs
│   │   ├── oidc/                # OIDC integration
│   │   │   ├── mod.rs
│   │   │   ├── keycloak.rs
│   │   │   ├── token.rs
│   │   │   └── claims.rs
│   │   ├── local/               # Local user auth
│   │   │   ├── mod.rs
│   │   │   ├── password.rs
│   │   │   └── session.rs
│   │   ├── rbac/                # Role-based access control
│   │   │   ├── mod.rs
│   │   │   ├── roles.rs
│   │   │   ├── permissions.rs
│   │   │   └── policy.rs
│   │   └── jwt.rs               # JWT handling
│   │
│   ├── application/             # Application services
│   │   ├── mod.rs
│   │   ├── commands/            # Command handlers
│   │   │   ├── create_event.rs
│   │   │   ├── create_receiver.rs
│   │   │   └── create_group.rs
│   │   ├── queries/             # Query handlers
│   │   │   ├── get_event.rs
│   │   │   └── search_events.rs
│   │   └── dto/                 # Data transfer objects
│   │
│   ├── domain/                  # Core business logic
│   │   ├── mod.rs
│   │   ├── entities/            # Domain entities
│   │   │   ├── event.rs
│   │   │   ├── receiver.rs
│   │   │   ├── group.rs
│   │   │   └── user.rs          # User entity
│   │   ├── value_objects/       # Value objects
│   │   │   ├── event_id.rs
│   │   │   ├── version.rs
│   │   │   └── schema.rs
│   │   ├── repositories/        # Repository traits
│   │   │   ├── event_repo.rs
│   │   │   ├── receiver_repo.rs
│   │   │   ├── group_repo.rs
│   │   │   └── user_repo.rs
│   │   ├── services/            # Domain services
│   │   │   └── validation.rs
│   │   └── errors.rs            # Domain errors
│   │
│   ├── infrastructure/          # External concerns
│   │   ├── mod.rs
│   │   ├── database/            # Database implementation
│   │   │   ├── mod.rs
│   │   │   ├── postgres.rs
│   │   │   ├── models.rs        # DB models
│   │   │   └── migrations/
│   │   ├── messaging/           # Kafka/messaging
│   │   │   ├── mod.rs
│   │   │   ├── producer.rs
│   │   │   └── consumer.rs
│   │   ├── tls/                 # TLS configuration
│   │   │   ├── mod.rs
│   │   │   └── config.rs
│   │   ├── telemetry/           # Observability
│   │   │   ├── metrics.rs
│   │   │   ├── tracing.rs
│   │   │   └── logging.rs
│   │   └── config.rs            # Configuration
│   │
│   └── error.rs                 # Application-wide errors
│
├── tests/                       # Integration tests
│   ├── api_tests.rs
│   ├── auth_tests.rs
│   ├── db_tests.rs
│   └── common/                  # Test utilities
│
├── migrations/                  # Database migrations
│   ├── 001_initial_schema.sql
│   └── 002_users_and_roles.sql
│
├── benches/                     # Benchmarks
│   └── event_creation.rs
│
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yaml
│
└── docs/
    ├── API.md
    ├── ARCHITECTURE.md
    ├── AUTHENTICATION.md
    └── SECURITY.md
```

---

## 🔧 Technology Stack

### Core Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1.40", features = ["full"] }
tokio-rustls = "0.26"

# Web framework
axum = "0.7"
axum-server = { version = "0.7", features = ["tls-rustls"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors", "auth"] }

# TLS
rustls = "0.23"
rustls-pemfile = "2.1"

# Authentication & Authorization
jsonwebtoken = "9.3"
oauth2 = "4.4"
openidconnect = "3.5"
argon2 = "0.5"
rand = "0.8"

# GraphQL
async-graphql = "7.0"
async-graphql-axum = "7.0"

# Database
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio", "uuid", "chrono", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Validation
validator = { version = "0.18", features = ["derive"] }

# IDs
uuid = { version = "1.10", features = ["v7", "serde"] }
ulid = "1.1"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
metrics = "0.23"
metrics-exporter-prometheus = "0.15"

# Messaging
rdkafka = "0.36"

# Configuration
config = "0.14"
dotenvy = "0.15"

# Time
chrono = { version = "0.4", features = ["serde"] }

[dev-dependencies]
# Testing
tokio-test = "0.4"
testcontainers = "0.21"
mockall = "0.13"
fake = "2.9"
criterion = "0.5"
```

---

## 🎨 Domain Design

### 1. Core Entities

```rust
// src/domain/entities/event.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    id: EventId,
    name: String,
    version: Version,
    release: String,
    platform_id: PlatformId,
    package: String,
    description: String,
    payload: serde_json::Value,
    success: bool,
    event_receiver_id: ReceiverId,
    created_at: DateTime<Utc>,
}

impl Event {
    pub fn new(
        name: String,
        version: Version,
        receiver_id: ReceiverId,
        payload: serde_json::Value,
    ) -> Result<Self, DomainError> {
        Self::validate_payload(&payload)?;

        Ok(Self {
            id: EventId::new(),
            name,
            version,
            event_receiver_id: receiver_id,
            payload,
            created_at: Utc::now(),
            // ... other fields with defaults
        })
    }

    fn validate_payload(payload: &serde_json::Value) -> Result<(), DomainError> {
        // Validation logic
        Ok(())
    }
}
```

### 2. Value Objects (Type Safety)

```rust
// src/domain/value_objects/event_id.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())  // Time-ordered UUIDs
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Similar for ReceiverId, GroupId, etc.
```

### 3. Repository Traits

```rust
// src/domain/repositories/event_repo.rs
use async_trait::async_trait;

#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn save(&self, event: &Event) -> Result<(), RepositoryError>;

    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>, RepositoryError>;

    async fn find_by_receiver(
        &self,
        receiver_id: ReceiverId,
    ) -> Result<Vec<Event>, RepositoryError>;

    async fn search(
        &self,
        filters: EventFilters,
    ) -> Result<Vec<Event>, RepositoryError>;
}
```

---

## 🔨 Application Layer

### Command Pattern (CQRS-lite)

```rust
// src/application/commands/create_event.rs
use crate::domain::{Event, EventRepository};

pub struct CreateEventCommand {
    pub name: String,
    pub version: String,
    pub receiver_id: String,
    pub payload: serde_json::Value,
    pub success: bool,
}

pub struct CreateEventHandler<R: EventRepository> {
    event_repo: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: EventRepository> CreateEventHandler<R> {
    pub async fn handle(
        &self,
        cmd: CreateEventCommand,
    ) -> Result<EventId, ApplicationError> {
        // 1. Validate receiver exists
        // 2. Create domain entity
        let event = Event::new(
            cmd.name,
            Version::parse(&cmd.version)?,
            ReceiverId::parse(&cmd.receiver_id)?,
            cmd.payload,
        )?;

        // 3. Save to repository
        self.event_repo.save(&event).await?;

        // 4. Publish event
        self.event_publisher.publish(&event).await?;

        Ok(event.id())
    }
}
```

---

## 🌐 API Layer

### REST API with Axum

```rust
// src/api/rest/events.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

pub struct EventsRouter {
    create_handler: CreateEventHandler,
    query_handler: QueryEventHandler,
}

pub async fn create_event(
    State(handler): State<CreateEventHandler>,
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, ApiError> {
    let command = request.into_command()?;
    let event_id = handler.handle(command).await?;

    Ok(Json(CreateEventResponse { id: event_id }))
}

pub async fn get_event(
    State(handler): State<QueryEventHandler>,
    Path(id): Path<String>,
) -> Result<Json<EventResponse>, ApiError> {
    let event_id = EventId::parse(&id)?;
    let event = handler.get_event(event_id).await?;

    Ok(Json(EventResponse::from(event)))
}

// Router setup
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/events", post(create_event))
        .route("/events/:id", get(get_event))
        .layer(TraceLayer::new_for_http())
}
```

### GraphQL with async-graphql

```rust
// src/api/graphql/schema.rs
use async_graphql::*;

pub struct Query;

#[Object]
impl Query {
    async fn event(&self, ctx: &Context<'_>, id: ID) -> Result<Event> {
        let handler = ctx.data::<QueryEventHandler>()?;
        let event_id = EventId::parse(&id)?;
        handler.get_event(event_id).await
    }

    async fn events(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filter by receiver ID")] receiver_id: Option<ID>,
    ) -> Result<Vec<Event>> {
        // Implementation
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn create_event(
        &self,
        ctx: &Context<'_>,
        input: CreateEventInput,
    ) -> Result<Event> {
        let handler = ctx.data::<CreateEventHandler>()?;
        // Implementation
    }
}

pub type Schema = async_graphql::Schema<Query, Mutation, EmptySubscription>;
```

---

## 🗄️ Infrastructure Layer

### Database with SQLx

```rust
// src/infrastructure/database/postgres.rs
use sqlx::{PgPool, FromRow};

#[derive(FromRow)]
struct EventRow {
    id: Uuid,
    name: String,
    version: String,
    // ... other fields
}

pub struct PostgresEventRepository {
    pool: PgPool,
}

#[async_trait]
impl EventRepository for PostgresEventRepository {
    async fn save(&self, event: &Event) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO events (id, name, version, event_receiver_id, payload, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            event.id().as_uuid(),
            event.name(),
            event.version().to_string(),
            event.receiver_id().as_uuid(),
            event.payload(),
            event.created_at(),
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        Ok(())
    }

    async fn find_by_id(&self, id: EventId) -> Result<Option<Event>, RepositoryError> {
        let row = sqlx::query_as!(
            EventRow,
            "SELECT * FROM events WHERE id = $1",
            id.as_uuid()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Event::from))
    }
}
```

### Kafka Producer

```rust
// src/infrastructure/messaging/producer.rs
use rdkafka::producer::{FutureProducer, FutureRecord};

pub struct KafkaEventPublisher {
    producer: FutureProducer,
    topic: String,
}

#[async_trait]
impl EventPublisher for KafkaEventPublisher {
    async fn publish(&self, event: &Event) -> Result<(), MessagingError> {
        let payload = serde_json::to_string(event)?;
        let key = event.id().to_string();

        let record = FutureRecord::to(&self.topic)
            .key(&key)
            .payload(&payload);

        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(err, _)| MessagingError::from(err))?;

        Ok(())
    }
}
```

---

## 🧪 Testing Strategy

### 1. Unit Tests

```rust
// src/domain/entities/event.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_valid_event() {
        let event = Event::new(
            "test".to_string(),
            Version::parse("1.0.0").unwrap(),
            ReceiverId::new(),
            json!({"key": "value"}),
        );

        assert!(event.is_ok());
    }

    #[test]
    fn rejects_invalid_payload() {
        // Test validation
    }
}
```

### 2. Integration Tests with Testcontainers

```rust
// tests/db_tests.rs
use testcontainers::*;

#[tokio::test]
async fn test_event_crud() {
    let docker = clients::Cli::default();
    let postgres = docker.run(images::postgres::Postgres::default());

    let pool = setup_database(&postgres).await;
    let repo = PostgresEventRepository::new(pool);

    // Test CRUD operations
}
```

### 3. Mock Repositories for Testing

```rust
// tests/common/mocks.rs
use mockall::mock;

mock! {
    pub EventRepo {}

    #[async_trait]
    impl EventRepository for EventRepo {
        async fn save(&self, event: &Event) -> Result<(), RepositoryError>;
        async fn find_by_id(&self, id: EventId) -> Result<Option<Event>, RepositoryError>;
    }
}
```

---

## 📊 Observability

### Structured Logging

```rust
// src/main.rs
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_telemetry() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "xzepr=debug,tower_http=debug".into())
        ))
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}
```

### Metrics

```rust
// src/infrastructure/telemetry/metrics.rs
use metrics::{counter, histogram};

pub fn record_event_created() {
    counter!("events_created_total").increment(1);
}

pub fn record_api_latency(duration: Duration) {
    histogram!("api_request_duration_seconds").record(duration.as_secs_f64());
}
```

### Distributed Tracing

```rust
// Add to API handlers
#[tracing::instrument(skip(handler))]
pub async fn create_event(
    State(handler): State<CreateEventHandler>,
    Json(request): Json<CreateEventRequest>,
) -> Result<Json<CreateEventResponse>, ApiError> {
    // Implementation
}
```

---

## 🚀 Deployment

### Docker Setup

```dockerfile
# docker/Dockerfile
FROM rust:1.82-slim as builder

WORKDIR /app
COPY Cargo.* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/xzepr /usr/local/bin/
EXPOSE 8080
CMD ["xzepr"]
```

### Docker Compose

```yaml
# docker/docker-compose.yaml
version: "3.8"

services:
  xzepr:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://postgres:password@postgres:5432/xzepr
      - KAFKA_BROKERS=kafka:9092
      - RUST_LOG=info
    depends_on:
      - postgres
      - kafka

  postgres:
    image: postgres:16
    environment:
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=xzepr
    volumes:
      - postgres_data:/var/lib/postgresql/data

  kafka:
    image: confluentinc/cp-kafka:latest
    environment:
      - KAFKA_ZOOKEEPER_CONNECT=zookeeper:2181
    depends_on:
      - zookeeper

  zookeeper:
    image: confluentinc/cp-zookeeper:latest
    environment:
      - ZOOKEEPER_CLIENT_PORT=2181

volumes:
  postgres_data:
```

---

## 🎯 Key Design Decisions

### 1. Why UUID v7 for IDs?

- Time-ordered (better for databases)
- Globally unique
- No coordination needed

### 2. Why CQRS-lite?

- Separates read/write concerns
- Easier to optimize each side
- Not full event sourcing (simpler)

### 3. Why Axum over Actix?

- Better integration with Tokio ecosystem
- Simpler, more composable design
- Great middleware support

### 4. Why SQLx over Diesel?

- Async-first
- Compile-time checked queries
- Less boilerplate

---

## ✅ Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

- [ ] Set up project structure
- [ ] Configure dependencies
- [ ] Implement domain entities
- [ ] Set up database with migrations
- [ ] Basic repository implementations

### Phase 2: Core Features (Week 3-4)

- [ ] Implement command handlers
- [ ] REST API endpoints
- [ ] GraphQL schema
- [ ] Kafka integration
- [ ] Validation logic

### Phase 3: Testing & Quality (Week 5)

- [ ] Unit tests for domain layer
- [ ] Integration tests
- [ ] API tests
- [ ] Load testing setup
- [ ] Documentation

### Phase 4: Observability (Week 6)

- [ ] Structured logging
- [ ] Metrics collection
- [ ] Tracing setup
- [ ] Health checks
- [ ] Monitoring dashboards

### Phase 5: Production Ready (Week 7-8)

- [ ] Docker optimization
- [ ] CI/CD pipeline
- [ ] Security audit
- [ ] Performance tuning
- [ ] Deployment documentation

---

## 📚 Learning Resources

- **Rust Async**: tokio.rs
- **Domain-Driven Design**: Eric Evans' book
- **Axum**: docs.rs/axum
- **SQLx**: github.com/launchbadge/sqlx
- **Error Handling**: <https://nick.groenen.me/posts/rust-error-handling/>

---

## 🤔 Key Advantages Over Go Version

1. **Type Safety** - Catch more bugs at compile time
2. **Performance** - Zero-cost abstractions, no GC
3. **Memory Safety** - No data races, guaranteed by compiler
4. **Fearless Concurrency** - async/await with Send/Sync
5. **Rich Type System** - Enums, pattern matching, Option/Result
6. **Cargo** - Superior dependency management

---

Want me to generate actual code for any specific component, or dive deeper into
any architectural decision?
