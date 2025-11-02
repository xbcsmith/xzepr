# security hardening

This document explains the security hardening measures implemented in XZepr to
protect against common web application vulnerabilities and attacks.

## overview

Phase 3 of the production readiness roadmap focuses on comprehensive security
hardening across multiple layers of the application stack. These measures work
together to provide defense-in-depth protection.

## security layers

### 1. cors (cross-origin resource sharing)

CORS controls which web applications can make requests to the XZepr API from
different origins.

#### how it works

When a browser makes a cross-origin request, it first sends a preflight OPTIONS
request to check if the server allows requests from that origin. The server
responds with CORS headers indicating:

- Which origins are allowed
- Which HTTP methods are permitted
- Which headers can be included
- Whether credentials (cookies, auth headers) are allowed
- How long the browser can cache the preflight response

#### security considerations

**Production Configuration:**

- Explicitly whitelist allowed origins (no wildcards)
- Require HTTPS for all origins except localhost
- Enable credentials only when necessary
- Set appropriate max-age to balance security and performance

**Development Configuration:**

- Allows all origins for easier local development
- Should never be used in production

**Attack Prevention:**

- Prevents unauthorized web applications from accessing the API
- Mitigates CSRF attacks by controlling credential inclusion
- Reduces risk of data exfiltration to malicious sites

#### configuration

```yaml
security:
  cors:
    allowed_origins:
      - https://app.example.com
      - https://admin.example.com
    allowed_methods:
      - GET
      - POST
      - PUT
      - DELETE
    allow_credentials: true
    max_age_seconds: 3600
```

### 2. rate limiting

Rate limiting prevents abuse by restricting the number of requests a client can
make within a time window.

#### architecture

XZepr uses a token bucket algorithm for rate limiting:

1. Each client gets a bucket of tokens
2. Tokens are consumed for each request
3. Tokens refill at a constant rate
4. Requests are rejected when bucket is empty

#### rate tiers

**Anonymous Users:**

- Default: 10 requests per minute
- Identified by IP address
- Lowest limit to prevent abuse

**Authenticated Users:**

- Default: 100 requests per minute
- Identified by JWT subject (user ID)
- Higher limit for legitimate users

**Admin Users:**

- Default: 1000 requests per minute
- Highest limit for administrative operations

**Per-Endpoint Limits:**

Critical endpoints can have custom limits:

- Login: 5 requests per minute (prevent credential stuffing)
- Registration: 3 requests per minute (prevent spam)
- Expensive queries: Custom limits based on resource cost

#### storage backends

**In-Memory Store:**

- Fast and simple
- Suitable for single-instance deployments
- State is lost on restart
- Not suitable for multi-instance deployments

**Redis Store (Recommended for Production):**

- Distributed state across instances
- Persistent across restarts
- Adds Redis as a dependency
- Slight latency overhead

#### attack prevention

- **Brute Force:** Limits login attempts
- **DoS/DDoS:** Prevents resource exhaustion
- **API Abuse:** Prevents scraping and excessive usage
- **Credential Stuffing:** Rate limits authentication endpoints

### 3. input validation

Input validation ensures all user-provided data meets expected format and
constraints before processing.

#### validation layers

**Schema Validation:**

Using the `validator` crate for declarative validation:

```rust
#[derive(Validate)]
struct CreateEventRequest {
    #[validate(length(min = 3, max = 100))]
    name: String,

    #[validate(email)]
    email: String,

    #[validate(range(min = 1, max = 100))]
    priority: u8,
}
```

**Sanitization:**

Active sanitization of input data:

- Remove control characters from strings
- Escape HTML special characters
- Validate URL protocols
- Normalize whitespace

**Size Limits:**

- Request body size: 1MB default (configurable)
- File uploads: 10MB max
- String lengths: 10,000 characters max
- Array lengths: 1,000 items max

#### validation patterns

**Email Validation:**

- RFC 5322 compliant regex
- Prevents email injection attacks

**URL Validation:**

- Must start with http:// or https://
- Blocks dangerous protocols (javascript:, data:, vbscript:)
- Prevents XSS through URL injection

**Alphanumeric Validation:**

- Allows letters, numbers, and common separators
- Blocks special characters that could enable injection

**UUID/ULID Validation:**

- Validates format before parsing
- Prevents injection through ID parameters

#### attack prevention

- **SQL Injection:** Validated input reduces attack surface
- **XSS:** HTML sanitization prevents script injection
- **Path Traversal:** Input validation blocks directory traversal
- **Command Injection:** Sanitization prevents shell command injection

### 4. graphql security

GraphQL-specific security measures protect against query-based attacks.

#### authentication and authorization

**Authentication Check:**

```rust
async fn protected_field(ctx: &Context<'_>) -> Result<String> {
    require_auth(ctx)?;
    Ok("Protected data".to_string())
}
```

**Role-Based Access Control:**

```rust
async fn admin_field(ctx: &Context<'_>) -> Result<String> {
    require_roles(ctx, &["admin"])?;
    Ok("Admin data".to_string())
}
```

**Permission-Based Access Control:**

```rust
async fn write_field(ctx: &Context<'_>) -> Result<String> {
    require_permissions(ctx, &["events:write"])?;
    Ok("Write operation".to_string())
}
```

#### query complexity analysis

Complex queries can be used for DoS attacks by requesting deeply nested or
expensive data.

**Complexity Limits:**

- Maximum complexity: 100 (production: 50)
- Maximum depth: 10 (production: 8)
- Configurable per-environment

**How It Works:**

1. Parse the GraphQL query
2. Calculate complexity score based on:
   - Number of fields requested
   - Nesting depth
   - Expected result size
3. Reject queries exceeding limits

#### attack prevention

- **Query DoS:** Complexity limits prevent expensive queries
- **Nested Queries:** Depth limits prevent stack overflow
- **Batch Attacks:** Combined with rate limiting
- **Unauthorized Access:** Auth checks on sensitive fields

### 5. security headers

HTTP security headers instruct browsers to enable protective features.

#### content security policy (CSP)

Controls which resources the browser can load:

```
default-src 'self';
script-src 'self';
style-src 'self';
img-src 'self' https:;
connect-src 'self';
frame-ancestors 'none';
```

**Protection Against:**

- XSS attacks by restricting script sources
- Clickjacking by controlling frame embedding
- Data injection by whitelisting resource origins

#### strict transport security (HSTS)

Forces HTTPS connections:

```
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
```

**Protection Against:**

- Protocol downgrade attacks
- Man-in-the-middle attacks
- SSL stripping attacks

#### x-frame-options

Prevents the page from being embedded in frames:

```
X-Frame-Options: DENY
```

**Protection Against:**

- Clickjacking attacks
- UI redressing attacks

#### x-content-type-options

Prevents MIME type sniffing:

```
X-Content-Type-Options: nosniff
```

**Protection Against:**

- MIME confusion attacks
- Content type manipulation

#### referrer-policy

Controls referrer information sent with requests:

```
Referrer-Policy: strict-origin-when-cross-origin
```

**Protection Against:**

- Information leakage through referrer headers
- Privacy violations

#### permissions-policy

Controls browser features:

```
Permissions-Policy: geolocation=(), microphone=(), camera=()
```

**Protection Against:**

- Unauthorized access to device features
- Privacy violations

## defense in depth

Multiple security layers work together to provide comprehensive protection:

```
┌─────────────────────────────────────┐
│         Client Browser              │
│  (Security Headers, CSP)            │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         TLS/HTTPS Layer             │
│  (Encryption, HSTS)                 │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         CORS Layer                  │
│  (Origin Validation)                │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Rate Limiting               │
│  (Abuse Prevention)                 │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Authentication              │
│  (JWT Validation)                   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Authorization               │
│  (Role/Permission Checks)           │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Input Validation            │
│  (Sanitization, Format Checks)      │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Business Logic              │
│  (Application Code)                 │
└─────────────────────────────────────┘
```

Each layer provides protection even if another layer is bypassed.

## configuration strategies

### development environment

```rust
let config = SecurityConfig {
    cors: CorsConfig::permissive(),
    rate_limit: RateLimitConfig::permissive(),
    validation: ValidationConfig::permissive(),
    headers: SecurityHeadersConfig::permissive(),
};
```

- Loose restrictions for easier development
- All origins allowed for CORS
- High rate limits
- Relaxed validation rules

### staging environment

```rust
let config = SecurityConfig {
    cors: CorsConfig::from_env(),
    rate_limit: RateLimitConfig::from_env(),
    validation: ValidationConfig::default(),
    headers: SecurityHeadersConfig::default(),
};
```

- Production-like configuration
- Environment-based settings
- Suitable for integration testing

### production environment

```rust
let config = SecurityConfig {
    cors: CorsConfig::production()?,
    rate_limit: RateLimitConfig::production(),
    validation: ValidationConfig::production(),
    headers: SecurityHeadersConfig::production(),
};
```

- Strictest security settings
- Validates configuration on startup
- Fails fast if misconfigured

## monitoring and alerts

Security measures should be monitored to detect attacks:

### metrics to track

- Rate limit rejections per minute
- Failed authentication attempts
- CORS policy violations
- Input validation failures
- GraphQL query complexity violations

### alert thresholds

- Spike in rate limit rejections (potential DoS)
- High authentication failure rate (credential stuffing)
- Unusual query complexity patterns (reconnaissance)
- Multiple validation failures from same IP (probing)

### logging

All security events should be logged with:

- Timestamp
- Client identifier (IP, user ID)
- Event type
- Result (allowed/blocked)
- Context (endpoint, payload size)

## performance considerations

Security measures add overhead:

### cors

- Minimal overhead
- Preflight responses are cached
- No significant performance impact

### rate limiting

**In-Memory:**

- Very fast (nanoseconds)
- No network latency

**Redis:**

- Additional network round-trip (1-5ms)
- Acceptable for most use cases
- Can use connection pooling

### input validation

- CPU overhead for regex matching
- Negligible for typical payloads
- Can be significant for large arrays/strings

### security headers

- Added to every response
- Minimal overhead (bytes of data)
- Cached by browsers

### graphql complexity

- Parsing and analysis overhead
- Computed once per query
- Much cheaper than executing expensive queries

## testing security measures

### unit tests

Test individual security components:

```rust
#[test]
fn test_cors_rejects_invalid_origin() {
    let config = CorsConfig::production().unwrap();
    assert!(!config.allowed_origins.contains(&"http://evil.com"));
}
```

### integration tests

Test security in realistic scenarios:

```rust
#[tokio::test]
async fn test_rate_limit_enforcement() {
    // Make 11 requests (limit is 10)
    for i in 1..=11 {
        let response = client.get("/api/events").send().await;
        if i <= 10 {
            assert_eq!(response.status(), StatusCode::OK);
        } else {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }
    }
}
```

### security tests

Automated security testing:

- OWASP ZAP scanning
- SQL injection probing
- XSS attack simulation
- CSRF testing
- Rate limit bypass attempts

## best practices

### cors

- Never use wildcard origins in production
- Use HTTPS for all production origins
- Enable credentials only when necessary
- Set max-age to balance caching and flexibility

### rate limiting

- Use Redis for multi-instance deployments
- Set endpoint-specific limits for critical paths
- Monitor rate limit metrics
- Implement progressive backoff for repeated violations

### input validation

- Validate all user input at API boundaries
- Sanitize before storage, not just display
- Use allowlists over denylists
- Fail securely (reject invalid input)

### graphql security

- Apply authentication to all sensitive fields
- Use role/permission checks for authorization
- Set conservative complexity limits
- Monitor query patterns for abuse

### security headers

- Use strict CSP in production
- Enable HSTS preload for public services
- Review permissions-policy for your use case
- Test headers with security scanners

## common vulnerabilities prevented

### owasp top 10

1. **Injection:** Input validation and sanitization
2. **Broken Authentication:** JWT with proper validation
3. **Sensitive Data Exposure:** HSTS, secure headers
4. **XML External Entities:** N/A (no XML parsing)
5. **Broken Access Control:** Role/permission checks
6. **Security Misconfiguration:** Secure defaults, validation
7. **XSS:** CSP, input sanitization, output encoding
8. **Insecure Deserialization:** Input validation
9. **Using Components with Known Vulnerabilities:** Dependency updates
10. **Insufficient Logging:** Security event logging

## conclusion

Security hardening is an ongoing process. The measures implemented in Phase 3
provide a strong foundation, but security requires:

- Regular security audits
- Dependency updates
- Monitoring and alerting
- Incident response planning
- Security training for team

Defense in depth means that even if one layer fails, others provide protection.
