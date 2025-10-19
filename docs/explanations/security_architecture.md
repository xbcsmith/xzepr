# Security Architecture

This document explains the security architecture, defense-in-depth approach, and
hardening strategies implemented in XZepr.

## Overview

XZepr implements a comprehensive security architecture based on defense-in-depth
principles, ensuring multiple layers of protection for the event tracking system.
The architecture is designed to protect against common attack vectors while
maintaining performance and usability.

## Security Principles

### Defense in Depth

Multiple independent layers of security controls work together:

1. **Network Layer** - TLS encryption, firewall rules, network segmentation
2. **Application Layer** - Authentication, authorization, input validation
3. **Data Layer** - Encryption at rest, secure key management
4. **Monitoring Layer** - Real-time threat detection and logging

### Principle of Least Privilege

- Users have minimum permissions needed for their role
- API tokens scoped to specific resources
- Service accounts with limited capabilities
- Role-based access control (RBAC) enforced at application level

### Secure by Default

- Production configurations require explicit security settings
- No wildcards or permissive defaults in production
- Strong security headers enabled automatically
- Rate limiting active out of the box

### Zero Trust Architecture

- All requests authenticated and authorized
- No implicit trust based on network location
- Continuous validation of identity and context
- Principle applied across all components

## Security Layers

### Layer 1: Transport Security

**TLS/HTTPS Enforcement**

All production traffic uses TLS 1.3 with strong cipher suites:

```yaml
server:
  tls:
    enabled: true
    cert_path: /etc/xzepr/tls/cert.pem
    key_path: /etc/xzepr/tls/key.pem
    min_version: "1.3"
```

**HTTP Security Headers**

Applied to all responses via middleware:

- **Content-Security-Policy (CSP)** - Prevents XSS attacks
- **Strict-Transport-Security (HSTS)** - Enforces HTTPS
- **X-Frame-Options** - Prevents clickjacking
- **X-Content-Type-Options** - Prevents MIME sniffing
- **Referrer-Policy** - Controls referrer information leakage
- **Permissions-Policy** - Restricts browser features

Implementation in `src/api/middleware/security_headers.rs`:

```rust
pub struct SecurityHeadersConfig {
    enable_csp: bool,
    csp_directives: HashMap<String, String>,
    enable_hsts: bool,
    hsts_max_age: u32,
    // ... additional headers
}
```

### Layer 2: CORS Protection

**Origin Validation**

Strict CORS policy prevents unauthorized cross-origin requests:

```yaml
security:
  cors:
    allowed_origins:
      - https://app.example.com
      - https://admin.example.com
    allow_credentials: true
    max_age_seconds: 3600
```

**CORS Middleware Flow**

1. Extract request origin header
2. Validate against allowed origins list
3. Reject requests from unauthorized origins
4. Add appropriate CORS headers to allowed requests
5. Log violations for security monitoring

Implementation uses `tower-http` CORS layer with strict validation.

### Layer 3: Rate Limiting

**Multi-Tier Rate Limiting**

Different rate limits based on user tier and endpoint:

```yaml
security:
  rate_limit:
    anonymous_rpm: 10        # Unauthenticated users
    authenticated_rpm: 100   # Authenticated users
    admin_rpm: 1000          # Admin users
    per_endpoint:
      /auth/login: 5         # Stricter for auth endpoints
      /api/v1/events: 50     # Per-endpoint overrides
```

**Token Bucket Algorithm**

Uses token bucket algorithm for smooth rate limiting:

- Tokens represent allowed requests
- Bucket refills at constant rate
- Requests consume tokens
- Rejected when bucket empty

**Distributed Rate Limiting**

Redis-backed store ensures consistent limits across instances:

```rust
pub trait RateLimitStore: Send + Sync {
    async fn check_rate_limit(
        &self,
        key: &str,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitStatus, String>;
}
```

Lua script ensures atomic operations:

```lua
-- Remove old entries outside window
redis.call('ZREMRANGEBYSCORE', key, 0, now - window)

-- Count current requests
local current = redis.call('ZCARD', key)

if current < limit then
    -- Allow request
    redis.call('ZADD', key, now, now)
    return {1, limit - current - 1, window}
else
    -- Reject request
    return {0, 0, reset_after}
end
```

### Layer 4: Authentication

**Multi-Provider Authentication**

Supports multiple authentication providers:

1. **Local Authentication** - Username/password with bcrypt hashing
2. **JWT Tokens** - Stateless token authentication with RSA signing
3. **OAuth 2.0** - Google, GitHub, Microsoft integration
4. **OpenID Connect** - Generic OIDC provider support
5. **API Keys** - Long-lived keys for service accounts

**JWT Token Structure**

```rust
pub struct JwtClaims {
    pub sub: String,           // User ID
    pub email: String,         // User email
    pub roles: Vec<String>,    // User roles
    pub exp: u64,              // Expiration time
    pub iat: u64,              // Issued at
    pub iss: String,           // Issuer
}
```

**Token Security**

- RSA-256 signing with 2048-bit keys
- Short expiration times (1 hour default)
- Refresh tokens for extended sessions
- Token revocation via Redis blacklist

### Layer 5: Authorization (RBAC)

**Role Hierarchy**

```text
Admin
  └── Full system access
      └── Manage users, roles, system config

User
  └── Standard access
      └── Create/read own resources

Viewer
  └── Read-only access
      └── View resources only

Anonymous
  └── Minimal access
      └── Public endpoints only
```

**Permission Model**

```rust
pub struct Permission {
    pub resource: String,    // events, users, receivers
    pub action: String,      // create, read, update, delete
    pub scope: Scope,        // own, team, global
}
```

**Authorization Flow**

1. Extract user identity from JWT claims
2. Load user roles and permissions
3. Check if user has required permission
4. Verify resource ownership if scoped
5. Grant or deny access

Implementation in `src/auth/rbac.rs`.

### Layer 6: Input Validation

**Multi-Layer Validation**

Validation occurs at multiple levels:

1. **Schema Validation** - JSON schema validation for API requests
2. **Type Validation** - Rust type system enforces correctness
3. **Business Rules** - Domain-specific validation logic
4. **Database Constraints** - Final validation at persistence layer

**Body Size Limits**

Prevents DoS attacks via large payloads:

```yaml
security:
  validation:
    max_body_size: 1048576  # 1 MB
    max_string_length: 10000
    max_array_length: 1000
```

**GraphQL Query Complexity**

Prevents expensive queries:

```rust
let schema = Schema::build(query, mutation, subscription)
    .limit_depth(10)
    .limit_complexity(100)
    .finish();
```

**SQL Injection Prevention**

- Parameterized queries via SQLx
- No dynamic SQL construction
- Input sanitization for search terms
- Database permissions restricted

## Security Monitoring

### Security Events

All security-relevant events are logged and recorded:

```rust
pub struct SecurityMonitor {
    fn record_auth_failure(&self, client_id: &str, reason: &str);
    fn record_rate_limit_rejection(&self, client_id: &str, endpoint: &str);
    fn record_cors_violation(&self, origin: &str, endpoint: &str);
    fn record_validation_error(&self, endpoint: &str, field: &str);
}
```

### Metrics Collection

Prometheus metrics for security analysis:

```text
xzepr_auth_failures_total{reason, client_id}
xzepr_rate_limit_rejections_total{endpoint, client_id}
xzepr_cors_violations_total{origin, endpoint}
xzepr_validation_errors_total{endpoint, field}
```

### Threat Detection

Automated detection of suspicious patterns:

- **Brute Force** - Multiple failed auth attempts
- **Rate Limit Abuse** - Consistent rate limit hits
- **CORS Probing** - Multiple CORS violations
- **Input Attacks** - Validation error patterns
- **Token Theft** - Token usage from multiple IPs

### Audit Logging

All sensitive operations logged:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "event": "user_role_changed",
  "actor": "admin@example.com",
  "target": "user123",
  "changes": {
    "roles": ["user", "admin"]
  },
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0..."
}
```

## Key Management

### RSA Key Pairs

JWT signing keys generated and stored securely:

```bash
# Generate private key
openssl genpkey -algorithm RSA -out private.pem -pkeyopt rsa_keygen_bits:2048

# Extract public key
openssl rsa -pubout -in private.pem -out public.pem
```

**Key Rotation**

- Keys rotated every 90 days
- Old keys retained for token validation
- Graceful transition period for active tokens

### Secret Storage

**Development**

- Secrets in local files (not committed)
- Environment variables for configuration

**Production**

- Kubernetes Secrets for container deployments
- HashiCorp Vault for enterprise deployments
- AWS Secrets Manager for cloud deployments
- Environment variables injected at runtime

Configuration:

```yaml
security:
  jwt:
    private_key_path: /run/secrets/jwt_private_key
    public_key_path: /run/secrets/jwt_public_key
```

## Threat Model

### Threats Mitigated

**External Threats**

1. **DDoS Attacks** - Rate limiting and connection limits
2. **Brute Force** - Rate limiting on auth endpoints
3. **SQL Injection** - Parameterized queries
4. **XSS Attacks** - CSP headers and input sanitization
5. **CSRF Attacks** - Token-based authentication
6. **Man-in-the-Middle** - TLS encryption
7. **Replay Attacks** - Short-lived JWT tokens with expiration

**Internal Threats**

1. **Privilege Escalation** - Strict RBAC enforcement
2. **Data Exfiltration** - Audit logging and monitoring
3. **Unauthorized Access** - Authentication on all endpoints
4. **Resource Abuse** - Rate limiting per user tier

### Attack Vectors

**Network Level**

- TLS/HTTPS enforcement prevents eavesdropping
- HSTS prevents protocol downgrade
- Network segmentation isolates services

**Application Level**

- Input validation prevents injection attacks
- Authentication prevents unauthorized access
- Rate limiting prevents abuse
- CORS prevents unauthorized origins

**Data Level**

- Encryption at rest protects stored data
- Access controls limit data exposure
- Audit logs track data access

## Security Configuration

### Development Mode

Permissive settings for local development:

```rust
impl SecurityConfig {
    pub fn development() -> Self {
        Self {
            cors: CorsSecurityConfig {
                allowed_origins: vec!["*".to_string()],
                allow_credentials: false,
                max_age_seconds: 3600,
            },
            rate_limit: RateLimitSecurityConfig {
                use_redis: false,
                anonymous_rpm: 10000,
                authenticated_rpm: 10000,
                admin_rpm: 10000,
                per_endpoint: HashMap::new(),
            },
            headers: SecurityHeadersConfig {
                enable_hsts: false,
                enable_csp: true,
                // ... relaxed settings
            },
        }
    }
}
```

### Production Mode

Strict security settings:

```rust
impl SecurityConfig {
    pub fn production() -> Self {
        Self {
            cors: CorsSecurityConfig {
                allowed_origins: vec![
                    "https://app.example.com".to_string(),
                    "https://admin.example.com".to_string(),
                ],
                allow_credentials: true,
                max_age_seconds: 3600,
            },
            rate_limit: RateLimitSecurityConfig {
                use_redis: true,
                anonymous_rpm: 10,
                authenticated_rpm: 100,
                admin_rpm: 1000,
                per_endpoint: HashMap::from([
                    ("/auth/login".to_string(), 5),
                    ("/auth/register".to_string(), 3),
                ]),
            },
            headers: SecurityHeadersConfig {
                enable_hsts: true,
                hsts_max_age: 31536000, // 1 year
                enable_csp: true,
                // ... strict settings
            },
        }
    }
}
```

### Environment-Based Configuration

Override via environment variables:

```bash
export XZEPR__SECURITY__RATE_LIMIT__USE_REDIS=true
export XZEPR__SECURITY__CORS__ALLOWED_ORIGINS="https://app.example.com,https://admin.example.com"
export XZEPR__SECURITY__HEADERS__ENABLE_HSTS=true
```

## Incident Response

### Detection

1. **Automated Monitoring** - Prometheus alerts on anomalies
2. **Log Analysis** - Structured logs analyzed for patterns
3. **User Reports** - Security issue reporting channel

### Response Workflow

1. **Identify** - Determine scope and severity
2. **Contain** - Rate limit or block attacker
3. **Investigate** - Analyze logs and metrics
4. **Remediate** - Fix vulnerability or configuration
5. **Recover** - Restore normal operations
6. **Learn** - Update security measures

### Automated Response

Rate limiting provides automatic protection:

- Attackers automatically throttled
- Sustained attacks blocked after threshold
- Legitimate users minimally impacted

## Compliance Considerations

### Data Protection

- GDPR compliance for EU users
- User data deletion on request
- Data retention policies enforced
- Audit trail for data access

### Security Standards

- OWASP Top 10 mitigations
- CWE/SANS Top 25 protections
- Industry best practices followed

## Future Enhancements

### Planned Security Features

1. **Mutual TLS** - Client certificate authentication
2. **WAF Integration** - Web application firewall support
3. **Anomaly Detection** - ML-based threat detection
4. **Hardware Security Module** - HSM for key storage
5. **Zero-Knowledge Proofs** - Enhanced privacy features

### Research Areas

- Homomorphic encryption for secure computation
- Differential privacy for analytics
- Blockchain-based audit logs
- Quantum-resistant cryptography

## Security Checklist

### Deployment Checklist

- [ ] TLS certificates installed and valid
- [ ] HSTS enabled with appropriate max-age
- [ ] CORS allowed origins configured (no wildcards)
- [ ] Rate limiting enabled with Redis
- [ ] JWT keys generated and securely stored
- [ ] Database credentials in secret management system
- [ ] Security monitoring and alerting configured
- [ ] Audit logging enabled and forwarded
- [ ] Firewall rules configured
- [ ] Regular security updates scheduled

### Ongoing Maintenance

- [ ] Review security logs weekly
- [ ] Update dependencies monthly
- [ ] Rotate JWT keys quarterly
- [ ] Security audit annually
- [ ] Penetration testing annually
- [ ] Incident response drill annually

## Related Documentation

- [How to Configure Redis Rate Limiting](../how_to/configure_redis_rate_limiting.md)
- [How to Setup Monitoring](../how_to/setup_monitoring.md)
- [Security Configuration Reference](../reference/security_configuration.md)
- [Authentication Guide](../how_to/configure_authentication.md)

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
- [CIS Controls](https://www.cisecurity.org/controls/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
