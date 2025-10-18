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

- Longer JWT expiration for convenience
- Local host binding
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
  jwt_secret: "..." # JWT signing secret (32+ chars)
  jwt_expiration_hours: 24 # JWT token lifetime
  enable_local_auth: true # Enable username/password auth
  enable_oidc: true # Enable Keycloak OIDC
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
export XZEPR__AUTH__JWT_SECRET="my-super-secret-key-at-least-32-chars"
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

# Override sensitive values
export XZEPR__AUTH__JWT_SECRET="$(openssl rand -base64 32)"
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

### JWT Secret

- **Minimum length**: 32 characters
- **Complexity**: Use cryptographically random strings
- **Generation**: `openssl rand -base64 32`
- **Storage**: Store in environment variables or secrets manager, never in
  config files

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

```
Error: missing field `jwt_secret`
→ Set XZEPR__AUTH__JWT_SECRET environment variable

Error: invalid database URL
→ Check XZEPR__DATABASE__URL format

Error: TLS certificate not found
→ Generate certs: make certs-generate
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
     - XZEPR__AUTH__JWT_SECRET=${JWT_SECRET}
     - XZEPR__DATABASE__URL=${DATABASE_URL}
   ```

3. **Docker secrets** (recommended for production):
   ```yaml
   secrets:
     - jwt_secret
     - db_password
   ```

## Troubleshooting

### Missing configuration file

```
Error: configuration file not found
```

**Solution**: Ensure `config/default.yaml` exists in the working directory.

### Invalid YAML syntax

```
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
