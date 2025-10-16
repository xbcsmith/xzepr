# Configuration Reference

This document provides a complete reference for all XZEPR configuration options.

## Overview

XZEPR uses a layered configuration system with three levels of precedence:

1. **Default configuration** (`config/default.yaml`) - Lowest priority
2. **Environment-specific configuration** (`config/{environment}.yaml`) - Medium
   priority
3. **Environment variables** - Highest priority

## Configuration Files

### File Locations

Configuration files are located in the `config/` directory:

```text
config/
├── default.yaml        # Base configuration
├── development.yaml    # Development overrides
└── production.yaml     # Production overrides
```

### Environment Selection

Set the `RUST_ENV` environment variable to choose which configuration file to
load:

```bash
export RUST_ENV=development  # Loads config/development.yaml
export RUST_ENV=production   # Loads config/production.yaml
```

Default: `development`

## Configuration Sections

### Server Configuration

```yaml
server:
  host: "0.0.0.0"
  port: 8443
  enable_https: true
```

#### server.host

- **Type:** String
- **Default:** `"0.0.0.0"`
- **Description:** IP address to bind the server to
- **Values:**
  - `"0.0.0.0"` - Bind to all interfaces
  - `"127.0.0.1"` - Localhost only
  - Specific IP address

#### server.port

- **Type:** Integer
- **Default:** `8443`
- **Description:** Port number for the server
- **Range:** 1-65535
- **Common values:**
  - `8443` - HTTPS (recommended)
  - `8080` - HTTP alternative
  - `3000` - Alternative development port

#### server.enable_https

- **Type:** Boolean
- **Default:** `true`
- **Description:** Enable TLS/HTTPS
- **Values:**
  - `true` - Use HTTPS (requires certificates)
  - `false` - Use HTTP (development only)

### Database Configuration

```yaml
database:
  url: "postgres://xzepr:password@localhost:5432/xzepr"
```

#### database.url

- **Type:** String
- **Required:** Yes
- **Description:** PostgreSQL connection string
- **Format:** `postgres://username:password@host:port/database`
- **Example:** `postgres://xzepr:password@localhost:5432/xzepr`
- **Security:** Should be overridden with environment variable in production

### Authentication Configuration

```yaml
auth:
  jwt_secret: "your-secret-key-min-32-chars"
  jwt_expiration_hours: 24
  enable_local_auth: true
  enable_oidc: false
  keycloak:
    issuer_url: "http://localhost:8080/realms/xzepr"
    client_id: "xzepr-client"
    client_secret: "change-me-in-production"
    redirect_url: "https://localhost:8443/auth/callback"
```

#### auth.jwt_secret

- **Type:** String
- **Required:** Yes
- **Description:** Secret key for signing JWT tokens
- **Minimum length:** 32 characters
- **Security:** Must be cryptographically random in production
- **Generation:** `openssl rand -base64 32`
- **Important:** Never commit to version control

#### auth.jwt_expiration_hours

- **Type:** Integer
- **Default:** `24`
- **Description:** JWT token lifetime in hours
- **Range:** 1-168 (1 hour to 7 days)
- **Recommended:**
  - Development: 168 (7 days)
  - Production: 24 (1 day)

#### auth.enable_local_auth

- **Type:** Boolean
- **Default:** `true`
- **Description:** Enable local username/password authentication

#### auth.enable_oidc

- **Type:** Boolean
- **Default:** `false`
- **Description:** Enable OpenID Connect authentication

#### auth.keycloak.issuer_url

- **Type:** String
- **Required:** If OIDC is enabled
- **Description:** Keycloak realm URL
- **Format:** `https://keycloak.example.com/realms/{realm-name}`
- **Example:** `http://localhost:8080/realms/xzepr`

#### auth.keycloak.client_id

- **Type:** String
- **Required:** If OIDC is enabled
- **Description:** OAuth2 client identifier
- **Example:** `xzepr-client`

#### auth.keycloak.client_secret

- **Type:** String
- **Required:** If OIDC is enabled
- **Description:** OAuth2 client secret
- **Security:** Must be kept confidential

#### auth.keycloak.redirect_url

- **Type:** String
- **Required:** If OIDC is enabled
- **Description:** OAuth2 callback URL
- **Format:** Must match registered redirect URI in Keycloak
- **Example:** `https://localhost:8443/auth/callback`

### TLS Configuration

```yaml
tls:
  cert_path: "certs/cert.pem"
  key_path: "certs/key.pem"
```

#### tls.cert_path

- **Type:** String
- **Required:** If HTTPS is enabled
- **Description:** Path to TLS certificate file
- **Format:** PEM format
- **Default:** `certs/cert.pem`

#### tls.key_path

- **Type:** String
- **Required:** If HTTPS is enabled
- **Description:** Path to TLS private key file
- **Format:** PEM format
- **Default:** `certs/key.pem`
- **Security:** File should have 600 permissions

### Kafka Configuration

```yaml
kafka:
  brokers: "localhost:19092"
```

#### kafka.brokers

- **Type:** String
- **Default:** `"localhost:19092"`
- **Description:** Comma-separated list of Kafka/Redpanda broker addresses
- **Format:** `host:port[,host:port]`
- **Examples:**
  - Single broker: `"localhost:19092"`
  - Multiple brokers: `"broker1:9092,broker2:9092,broker3:9092"`

## Environment Variables

Override any configuration value using environment variables with the `XZEPR__`
prefix.

### Naming Convention

Format: `XZEPR__SECTION__SUBSECTION__KEY`

- Use double underscores (`__`) to separate levels
- All uppercase
- Maps directly to YAML structure

### Examples

```bash
# Server configuration
export XZEPR__SERVER__HOST="0.0.0.0"
export XZEPR__SERVER__PORT="9443"
export XZEPR__SERVER__ENABLE_HTTPS="true"

# Database configuration
export XZEPR__DATABASE__URL="postgres://user:pass@db:5432/xzepr"

# Authentication configuration
export XZEPR__AUTH__JWT_SECRET="my-super-secret-key-at-least-32-chars"
export XZEPR__AUTH__JWT_EXPIRATION_HOURS="48"
export XZEPR__AUTH__ENABLE_LOCAL_AUTH="true"
export XZEPR__AUTH__ENABLE_OIDC="true"

# Keycloak configuration (nested)
export XZEPR__AUTH__KEYCLOAK__ISSUER_URL="https://auth.example.com/realms/prod"
export XZEPR__AUTH__KEYCLOAK__CLIENT_ID="xzepr-prod"
export XZEPR__AUTH__KEYCLOAK__CLIENT_SECRET="secret-value"
export XZEPR__AUTH__KEYCLOAK__REDIRECT_URL="https://xzepr.example.com/auth/callback"

# TLS configuration
export XZEPR__TLS__CERT_PATH="/etc/xzepr/tls/cert.pem"
export XZEPR__TLS__KEY_PATH="/etc/xzepr/tls/key.pem"

# Kafka configuration
export XZEPR__KAFKA__BROKERS="kafka1:9092,kafka2:9092"
```

## Configuration Precedence

When the same setting is defined in multiple places, this is the order of
precedence (highest to lowest):

1. Environment variables
2. Environment-specific YAML file
3. Default YAML file

Example: If `server.port` is defined in all three places:

```yaml
# config/default.yaml
server:
  port: 8443
```

```yaml
# config/production.yaml
server:
  port: 8080
```

```bash
# Environment variable
export XZEPR__SERVER__PORT="9000"
```

Result: Server will use port **9000** (environment variable wins)

## Configuration by Environment

### Development Configuration

`config/development.yaml`:

```yaml
server:
  host: "127.0.0.1"
  port: 8443
  enable_https: true

database:
  url: "postgres://xzepr:password@localhost:5432/xzepr"

auth:
  jwt_secret: "dev-secret-not-for-production-use"
  jwt_expiration_hours: 168 # 7 days for convenience
  enable_local_auth: true
  enable_oidc: true

tls:
  cert_path: "certs/cert.pem"
  key_path: "certs/key.pem"

kafka:
  brokers: "localhost:19092"
```

### Production Configuration

`config/production.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8443
  enable_https: true

database:
  url: "postgres://xzepr:CHANGE_ME@postgres:5432/xzepr"

auth:
  jwt_secret: "CHANGE_ME_TO_SECURE_RANDOM_STRING"
  jwt_expiration_hours: 24
  enable_local_auth: true
  enable_oidc: true
  keycloak:
    issuer_url: "https://keycloak.example.com/realms/xzepr"
    client_id: "xzepr-client"
    client_secret: "CHANGE_ME_TO_KEYCLOAK_CLIENT_SECRET"
    redirect_url: "https://xzepr.example.com/auth/callback"

tls:
  cert_path: "/etc/xzepr/tls/cert.pem"
  key_path: "/etc/xzepr/tls/key.pem"

kafka:
  brokers: "redpanda-0:9092"
```

## Security Best Practices

### Secrets Management

Never commit secrets to version control:

- JWT secret
- Database passwords
- Keycloak client secrets
- TLS private keys

Use environment variables or secrets management:

```bash
# Read from secrets management system
export XZEPR__AUTH__JWT_SECRET=$(vault read -field=value secret/xzepr/jwt_secret)
export XZEPR__DATABASE__URL=$(vault read -field=value secret/xzepr/db_url)
```

### JWT Secret

Generate secure JWT secret:

```bash
openssl rand -base64 32
```

Requirements:

- Minimum 32 characters
- Cryptographically random
- Unique per environment
- Rotated regularly

### Database Credentials

Best practices:

- Use strong passwords (16+ characters)
- Different credentials per environment
- Limit database user permissions
- Enable SSL/TLS for connections
- Rotate credentials regularly

### TLS Certificates

Production requirements:

- Valid certificates from trusted CA
- Private key file permissions: 600
- Regular renewal before expiration
- Use cert-manager for Kubernetes

## Validation

Configuration is validated on startup. Common errors:

### Missing Required Fields

```text
Error: missing field `jwt_secret`
```

**Solution:** Set `XZEPR__AUTH__JWT_SECRET` environment variable

### Invalid Values

```text
Error: invalid value for server.port: 0
```

**Solution:** Use valid port number (1-65535)

### File Not Found

```text
Error: TLS certificate not found: certs/cert.pem
```

**Solution:** Generate certificates or correct path

## Troubleshooting

### Configuration Not Loading

Check file existence:

```bash
ls -la config/default.yaml
ls -la config/development.yaml
```

Check YAML syntax:

```bash
yamllint config/*.yaml
```

### Environment Variables Not Applied

Verify format:

```bash
env | grep XZEPR__
```

Ensure double underscores and uppercase.

### Wrong Configuration Used

Check environment:

```bash
echo $RUST_ENV
```

Verify loading order in logs.

## Complete Example

Full production configuration using environment variables:

```bash
#!/bin/bash
# Production environment configuration

# Environment selection
export RUST_ENV=production

# Server
export XZEPR__SERVER__HOST="0.0.0.0"
export XZEPR__SERVER__PORT="8443"
export XZEPR__SERVER__ENABLE_HTTPS="true"

# Database
export XZEPR__DATABASE__URL="postgres://xzepr:${DB_PASSWORD}@postgres.prod:5432/xzepr"

# Authentication
export XZEPR__AUTH__JWT_SECRET="${JWT_SECRET}"
export XZEPR__AUTH__JWT_EXPIRATION_HOURS="24"
export XZEPR__AUTH__ENABLE_LOCAL_AUTH="true"
export XZEPR__AUTH__ENABLE_OIDC="true"

# Keycloak
export XZEPR__AUTH__KEYCLOAK__ISSUER_URL="https://auth.example.com/realms/prod"
export XZEPR__AUTH__KEYCLOAK__CLIENT_ID="xzepr-prod"
export XZEPR__AUTH__KEYCLOAK__CLIENT_SECRET="${KEYCLOAK_SECRET}"
export XZEPR__AUTH__KEYCLOAK__REDIRECT_URL="https://xzepr.example.com/auth/callback"

# TLS
export XZEPR__TLS__CERT_PATH="/etc/xzepr/tls/cert.pem"
export XZEPR__TLS__KEY_PATH="/etc/xzepr/tls/key.pem"

# Kafka
export XZEPR__KAFKA__BROKERS="kafka1.prod:9092,kafka2.prod:9092,kafka3.prod:9092"

# Start server
./target/release/xzepr
```

## See Also

- [Running the Server](../how_to/running_server.md) - How to use configuration
- [Environment Variables Reference](environment_variables.md) - Complete
  variable list
- [Deployment Guide](../how_to/deployment.md) - Production deployment
