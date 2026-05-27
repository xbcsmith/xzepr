# Configuration

This directory contains configuration files for the XZepr event tracking server.

## Overview

XZepr uses a layered configuration approach:

1. **Default configuration** (`default.yaml`) - Base settings
2. **Environment-specific configuration** (`development.yaml`,
   `production.yaml`) - Override defaults
3. **Environment variables** - Highest priority, override all file-based config

## Configuration Files

### default.yaml

Contains sensible defaults for all required settings. This file is always loaded
first.

### development.yaml

Development-specific settings optimized for local development:

- Local host binding
- HS256 JWT signing with `auth.jwt.secret_key` for local-only use
- Development secrets (not for production!)

### production.yaml

Production deployment settings with placeholders for sensitive values. **Never
commit actual production secrets to version control!**

## Configuration Structure

```yaml
server:
  host: "0.0.0.0" # Bind address
  port: 8443 # HTTPS port
  enable_https: true # Enable TLS

database:
  url: "postgres://..." # PostgreSQL connection string

auth:
  enable_local_auth: true # Enable username/password auth
  enable_oidc: true # Enable Keycloak OIDC
  jwt:
    access_token_expiration_seconds: 900 # Access token lifetime
    refresh_token_expiration_seconds: 604800 # Refresh token lifetime
    issuer: "xzepr" # Token issuer
    audience: "xzepr-api" # Token audience
    algorithm: "RS256" # Production signing algorithm
    private_key_path: "/etc/xzepr/keys/jwt_rsa" # RS256 private key
    public_key_path: "/etc/xzepr/keys/jwt_rsa.pub" # RS256 public key
    secret_key: null # HS256 local-only signing secret
    enable_token_rotation: true # Rotate refresh tokens
    leeway_seconds: 60 # Clock skew allowance
  keycloak:
    issuer_url: "..." # Keycloak realm URL
    client_id: "..." # OAuth2 client ID
    client_secret: "..." # OAuth2 client secret
    redirect_url: "..." # OAuth2 callback URL

tls:
  cert_path: "..." # TLS certificate path
  key_path: "..." # TLS private key path

kafka:
  brokers: "..." # Redpanda/Kafka broker addresses
```

## Environment Variables

Override any configuration value using environment variables with the prefix
`XZEPR__`:

```bash
# Format: XZEPR__SECTION__SUBSECTION__KEY=value
export XZEPR__DATABASE__URL="postgres://user:pass@host:5432/db"
export XZEPR__AUTH__JWT__ALGORITHM="RS256"
export XZEPR__AUTH__JWT__PRIVATE_KEY_PATH="/etc/xzepr/keys/jwt_rsa"
export XZEPR__AUTH__JWT__PUBLIC_KEY_PATH="/etc/xzepr/keys/jwt_rsa.pub"
export XZEPR__AUTH__JWT__ACCESS_TOKEN_EXPIRATION_SECONDS="900"
export XZEPR__SERVER__PORT="9443"

# Nested sections use double underscores
export XZEPR__AUTH__KEYCLOAK__CLIENT_SECRET="secret"
```

## Environment Selection

Set the `RUST_ENV` environment variable to choose which config file to load:

```bash
# Use development.yaml
export RUST_ENV=development

# Use production.yaml
export RUST_ENV=production

# Defaults to development if not set
```

## Usage Examples

### Development

```bash
# Uses config/development.yaml by default
cargo run --bin xzepr

# Or explicitly set environment
RUST_ENV=development cargo run --bin xzepr
```

### Production

```bash
# Set environment
export RUST_ENV=production

# Override sensitive values and signing key locations
export XZEPR__AUTH__JWT__ALGORITHM="RS256"
export XZEPR__AUTH__JWT__PRIVATE_KEY_PATH="/etc/xzepr/keys/jwt_rsa"
export XZEPR__AUTH__JWT__PUBLIC_KEY_PATH="/etc/xzepr/keys/jwt_rsa.pub"
export XZEPR__DATABASE__URL="postgres://xzepr:$DB_PASSWORD@postgres:5432/xzepr"
export XZEPR__AUTH__KEYCLOAK__CLIENT_SECRET="$KEYCLOAK_SECRET"

# Run application
./target/release/xzepr
```

### Admin CLI

The admin tool uses the same configuration system:

```bash
# Uses development config by default
make admin ARGS="create-user -u admin -e admin@example.com -p password -r Admin"

# Use production config
RUST_ENV=production make admin ARGS="list-users"
```

## Security Best Practices

### JWT Signing Material

- **Production algorithm**: Use `auth.jwt.algorithm: "RS256"` with separate
  `auth.jwt.private_key_path` and `auth.jwt.public_key_path` values
- **Local-only algorithm**: Use `auth.jwt.secret_key` only with `HS256` for
  development or tests
- **Key generation**: Generate RSA keys with your platform key management tool
  or secrets manager workflow
- **Storage**: Store signing keys in a secrets manager or mounted secret volume,
  never directly in committed configuration files

### Database Credentials

- Use strong passwords (16+ characters)
- Store in environment variables
- Use connection pooling in production
- Enable SSL/TLS for database connections

### Keycloak Secrets

- Generate unique client secrets for each environment
- Rotate secrets regularly
- Use Keycloak's admin console to manage clients
- Never commit secrets to version control

### TLS Certificates

- Use valid certificates from trusted CA in production
- Store private keys securely with restricted permissions (chmod 600)
- Consider using cert-manager for Kubernetes deployments
- Rotate certificates before expiration

## Configuration Validation

The application validates configuration on startup and will fail fast if
required values are missing or invalid.

Common validation errors:

```text
Error: missing JWT signing material for RS256
-> Set XZEPR__AUTH__JWT__PRIVATE_KEY_PATH and XZEPR__AUTH__JWT__PUBLIC_KEY_PATH

Error: invalid database URL
-> Check XZEPR__DATABASE__URL format

Error: TLS certificate not found
-> Generate certs: make certs-generate
```

## Development Setup

```bash
# 1. Ensure default config exists
ls config/default.yaml

# 2. Generate TLS certificates
make certs-generate

# 3. Start services
docker compose -f docker-compose.services.yaml up -d

# 4. Run migrations
make db-migrate

# 5. Run application
cargo run --bin xzepr
```

## Docker Deployment

When running in Docker, configuration can be provided via:

1. **Mounted config files**:

   ```yaml
   volumes:
     - ./config:/app/config:ro
   ```

2. **Environment variables**:

   ```yaml
   environment:
     - XZEPR__AUTH__JWT__ALGORITHM=RS256
     - XZEPR__AUTH__JWT__PRIVATE_KEY_PATH=/run/secrets/jwt_private_key
     - XZEPR__AUTH__JWT__PUBLIC_KEY_PATH=/run/secrets/jwt_public_key
     - XZEPR__DATABASE__URL=${DATABASE_URL}
   ```

3. **Docker secrets** (recommended for production):

   ```yaml
   secrets:
     - jwt_private_key
     - jwt_public_key
     - db_password
   ```

## Troubleshooting

### Missing configuration file

```text
Error: configuration file not found
```

**Solution**: Ensure `config/default.yaml` exists in the working directory.

### Invalid YAML syntax

```text
Error: while parsing a flow mapping
```

**Solution**: Validate YAML syntax with `yamllint config/*.yaml`

### Environment variable not applied

**Check**:

1. Variable name format: `XZEPR__SECTION__KEY` (double underscores)
2. No spaces around `=` sign
3. Variable is exported: `export XZEPR__...`

### Configuration precedence

Remember the loading order:

1. `default.yaml` (lowest priority)
2. `$RUST_ENV.yaml` (e.g., `development.yaml`)
3. Environment variables (highest priority)

## References

- [config-rs Documentation](https://docs.rs/config/)
- [Rust Serde Deserialization](https://serde.rs/)
- [12-Factor App Config](https://12factor.net/config)
