# implement security hardening

This guide shows you how to implement and configure the security hardening
measures in your XZepr deployment.

## prerequisites

- XZepr server installed
- Access to configuration files
- Basic understanding of web security concepts

## step 1: configure cors

### production configuration

Edit your configuration file to set allowed origins:

```yaml
security:
  cors:
    allowed_origins:
      - https://app.example.com
      - https://admin.example.com
    allow_credentials: true
    max_age_seconds: 3600
```

Or set environment variables:

```bash
export XZEPR__SECURITY__CORS__ALLOWED_ORIGINS="https://app.example.com,https://admin.example.com"
export XZEPR__SECURITY__CORS__ALLOW_CREDENTIALS=true
export XZEPR__SECURITY__CORS__MAX_AGE_SECONDS=3600
```

### apply cors middleware

In your application setup:

```rust
use xzepr::api::middleware::cors::production_cors_layer;

let cors = production_cors_layer()?;
let app = Router::new()
    .route("/api/events", get(list_events))
    .layer(cors);
```

### verify cors configuration

Test with curl:

```bash
# Should succeed with allowed origin
curl -H "Origin: https://app.example.com" \
     -H "Access-Control-Request-Method: POST" \
     -X OPTIONS \
     https://your-api.com/api/events

# Should fail with disallowed origin
curl -H "Origin: https://evil.com" \
     -H "Access-Control-Request-Method: POST" \
     -X OPTIONS \
     https://your-api.com/api/events
```

Check for these headers in successful response:

- `Access-Control-Allow-Origin: https://app.example.com`
- `Access-Control-Allow-Methods: GET, POST, PUT, DELETE`
- `Access-Control-Max-Age: 3600`

## step 2: enable rate limiting

### choose storage backend

**For single instance (development/small deployments):**

```rust
use xzepr::api::middleware::rate_limit::{
    RateLimitConfig, RateLimiterState, InMemoryRateLimitStore
};

let config = RateLimitConfig::from_env();
let store = Arc::new(InMemoryRateLimitStore::new());
let limiter = RateLimiterState::new(config, store);
```

**For multi-instance (production):**

```rust
// TODO: Implement Redis store
// use xzepr::api::middleware::rate_limit::RedisRateLimitStore;
//
// let redis_client = redis::Client::open("redis://127.0.0.1/")?;
// let store = Arc::new(RedisRateLimitStore::new(redis_client));
// let limiter = RateLimiterState::new(config, store);
```

### configure rate limits

```bash
# Anonymous users
export XZEPR__SECURITY__RATE_LIMIT__ANONYMOUS_RPM=10

# Authenticated users
export XZEPR__SECURITY__RATE_LIMIT__AUTHENTICATED_RPM=100

# Admin users
export XZEPR__SECURITY__RATE_LIMIT__ADMIN_RPM=1000
```

### apply rate limiting middleware

```rust
use axum::middleware;
use xzepr::api::middleware::rate_limit::rate_limit_middleware;

let app = Router::new()
    .route("/api/events", get(list_events))
    .layer(middleware::from_fn_with_state(
        limiter.clone(),
        rate_limit_middleware
    ));
```

### add per-endpoint limits

For critical endpoints:

```rust
let config = RateLimitConfig::from_env()
    .with_endpoint_limit("/auth/login", 5)
    .with_endpoint_limit("/auth/register", 3);
```

### verify rate limiting

Test with curl:

```bash
# Make 11 requests quickly (limit is 10)
for i in {1..11}; do
  curl -w "\n%{http_code}\n" https://your-api.com/api/events
done

# First 10 should return 200
# 11th should return 429 (Too Many Requests)
```

Check response headers:

- `X-RateLimit-Limit: 10`
- `X-RateLimit-Remaining: 5`
- `X-RateLimit-Reset: 45`

## step 3: implement input validation

### add validation to request types

Use the validator crate:

```rust
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateEventRequest {
    #[validate(length(min = 3, max = 100))]
    pub name: String,

    #[validate(email)]
    pub email: String,

    #[validate(range(min = 1, max = 100))]
    pub priority: u8,

    #[validate(url)]
    pub webhook_url: Option<String>,
}
```

### validate in handlers

```rust
use xzepr::api::middleware::validation::validate_request;

async fn create_event(
    Json(payload): Json<CreateEventRequest>
) -> Result<Json<Event>, ValidationErrorResponse> {
    // Validate the request
    validate_request(&payload)?;

    // Process the request
    let event = event_service.create(payload).await?;
    Ok(Json(event))
}
```

### add body size limits

```rust
use xzepr::api::middleware::validation::body_size_limit_middleware;

let app = Router::new()
    .route("/api/events", post(create_event))
    .layer(middleware::from_fn(body_size_limit_middleware));
```

### use sanitization helpers

```rust
use xzepr::api::middleware::validation::sanitize;

// Sanitize user input
let clean_name = sanitize::sanitize_string(&user_input);

// Validate email
if !sanitize::validate_email(&email) {
    return Err("Invalid email format");
}

// Sanitize URL
let safe_url = sanitize::sanitize_url(&url)?;
```

## step 4: secure graphql

### add authentication checks

Update your GraphQL resolvers:

```rust
use xzepr::api::graphql::guards::require_auth;

#[Object]
impl Query {
    async fn protected_data(&self, ctx: &Context<'_>) -> Result<String> {
        // Require authentication
        require_auth(ctx)?;

        Ok("Protected data".to_string())
    }
}
```

### add role-based authorization

```rust
use xzepr::api::graphql::guards::require_roles;

#[Object]
impl Query {
    async fn admin_data(&self, ctx: &Context<'_>) -> Result<String> {
        // Require admin role
        require_roles(ctx, &["admin"])?;

        Ok("Admin data".to_string())
    }
}
```

### add permission-based authorization

```rust
use xzepr::api::graphql::guards::require_permissions;

#[Object]
impl Mutation {
    async fn create_event(&self, ctx: &Context<'_>) -> Result<Event> {
        // Require write permission
        require_permissions(ctx, &["events:write"])?;

        // Create event
        Ok(event_service.create().await?)
    }
}
```

### configure query complexity limits

```rust
use xzepr::api::graphql::guards::ComplexityConfig;

let complexity_config = ComplexityConfig::production();

let schema = Schema::build(Query, Mutation, EmptySubscription)
    .data(complexity_config)
    .finish();
```

Set via environment:

```bash
export XZEPR__GRAPHQL__MAX_COMPLEXITY=50
export XZEPR__GRAPHQL__MAX_DEPTH=8
export XZEPR__GRAPHQL__ENFORCE_COMPLEXITY=true
```

## step 5: enable security headers

### apply security headers middleware

```rust
use xzepr::api::middleware::security_headers::security_headers_middleware;

let app = Router::new()
    .route("/api/events", get(list_events))
    .layer(middleware::from_fn(security_headers_middleware));
```

### customize for production

```rust
use xzepr::api::middleware::security_headers::{
    SecurityHeadersConfig,
    security_headers_middleware_with_config
};

let config = SecurityHeadersConfig::production();

let app = Router::new()
    .route("/api/events", get(list_events))
    .layer(middleware::from_fn(move |req, next| {
        let config = config.clone();
        async move {
            security_headers_middleware_with_config(config, req, next).await
        }
    }));
```

### customize content security policy

```bash
export XZEPR__SECURITY__HEADERS__CSP_DIRECTIVES="default-src 'self'; script-src 'self' https://cdn.example.com"
```

### configure hsts

```bash
export XZEPR__SECURITY__HEADERS__ENABLE_HSTS=true
export XZEPR__SECURITY__HEADERS__HSTS_MAX_AGE=31536000
export XZEPR__SECURITY__HEADERS__HSTS_INCLUDE_SUBDOMAINS=true
export XZEPR__SECURITY__HEADERS__HSTS_PRELOAD=false
```

### verify security headers

Test with curl:

```bash
curl -I https://your-api.com/api/events
```

Look for these headers:

```
Content-Security-Policy: default-src 'self'; script-src 'self'
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
Referrer-Policy: strict-origin-when-cross-origin
```

## step 6: combine all middleware

Apply all security layers in correct order:

```rust
use axum::{Router, routing::get, middleware};
use xzepr::api::middleware::{
    cors::production_cors_layer,
    rate_limit::{rate_limit_middleware, RateLimiterState},
    validation::body_size_limit_middleware,
    security_headers::security_headers_middleware,
    jwt::jwt_auth_middleware,
};

async fn build_app() -> Result<Router, Box<dyn std::error::Error>> {
    // Initialize rate limiter
    let limiter = RateLimiterState::default_with_config(
        RateLimitConfig::from_env()
    );

    // Build router with all security layers
    let app = Router::new()
        .route("/api/events", get(list_events))
        // Security headers (outermost)
        .layer(middleware::from_fn(security_headers_middleware))
        // CORS
        .layer(production_cors_layer()?)
        // Rate limiting
        .layer(middleware::from_fn_with_state(
            limiter,
            rate_limit_middleware
        ))
        // Body size limits
        .layer(middleware::from_fn(body_size_limit_middleware))
        // JWT authentication
        .layer(middleware::from_fn(jwt_auth_middleware));

    Ok(app)
}
```

## step 7: test security configuration

### automated testing

Create integration tests:

```rust
#[tokio::test]
async fn test_cors_policy() {
    let app = build_test_app().await;

    // Test allowed origin
    let response = app
        .oneshot(Request::builder()
            .uri("/api/events")
            .header("Origin", "https://app.example.com")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert!(response.headers()
        .get("access-control-allow-origin")
        .is_some());
}

#[tokio::test]
async fn test_rate_limiting() {
    let app = build_test_app().await;

    // Make requests up to limit
    for _ in 0..10 {
        let response = app.clone()
            .oneshot(Request::get("/api/events").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // Next request should be rate limited
    let response = app
        .oneshot(Request::get("/api/events").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
```

### manual security testing

Use security scanning tools:

```bash
# OWASP ZAP
zap-cli quick-scan https://your-api.com

# Nikto
nikto -h https://your-api.com

# SSL Labs
# Visit: https://www.ssllabs.com/ssltest/
```

## step 8: monitor security events

### set up logging

Configure structured logging for security events:

```rust
tracing::warn!(
    event = "rate_limit_exceeded",
    ip = %client_ip,
    endpoint = %path,
    "Rate limit exceeded"
);

tracing::warn!(
    event = "authentication_failed",
    ip = %client_ip,
    reason = %error,
    "Authentication failed"
);
```

### configure alerts

Set up alerts for security events:

- Rate limit violations spike
- Authentication failures increase
- CORS violations
- Input validation failures
- Unusual query complexity patterns

### metrics to track

Export Prometheus metrics:

- `security_rate_limit_rejections_total`
- `security_auth_failures_total`
- `security_cors_violations_total`
- `security_validation_errors_total`

## troubleshooting

### cors issues

**Problem:** Browser blocks requests with CORS error

**Solution:**
1. Check that origin is in allowed_origins list
2. Verify origin uses HTTPS in production
3. Check that credentials are enabled if needed
4. Clear browser cache

### rate limiting issues

**Problem:** Legitimate users getting rate limited

**Solution:**
1. Increase rate limits for authenticated users
2. Implement per-user tracking (not just IP)
3. Use Redis for distributed rate limiting
4. Add endpoint-specific exceptions

### validation errors

**Problem:** Valid requests rejected by validation

**Solution:**
1. Check validation rules match expected format
2. Review max lengths and ranges
3. Check for encoding issues (UTF-8)
4. Log rejected payloads for analysis

### security header conflicts

**Problem:** CSP blocking legitimate resources

**Solution:**
1. Review CSP directives
2. Add specific domains to allowlist
3. Use browser dev tools to see violations
4. Test in staging first

## best practices

1. **Start strict:** Begin with strict settings and relax if needed
2. **Monitor first:** Deploy with monitoring before enforcing
3. **Test thoroughly:** Use automated tests for security config
4. **Document exceptions:** Keep track of why rules are relaxed
5. **Regular audits:** Review security configuration regularly
6. **Stay updated:** Keep dependencies current for security patches

## next steps

- Implement Redis backend for rate limiting
- Set up security monitoring dashboards
- Configure automated security scanning
- Create incident response procedures
- Train team on security best practices
