# Running the XZEPR Server

This guide explains how to start, stop, and manage the XZEPR event tracking
server.

## Prerequisites

Before running the server, ensure you have:

1. **PostgreSQL running** with the xzepr database
2. **Database migrations applied**
3. **Configuration files** in place
4. **TLS certificates** generated (if using HTTPS)

## Quick Start

### Start All Services

```bash
# Start PostgreSQL, Keycloak, and Redpanda
docker compose -f docker-compose.services.yaml up -d

# Run database migrations
make db-migrate

# Start the XZEPR server
cargo run --bin xzepr
```

The server will start on `https://127.0.0.1:8443` by default.

### Verify Server is Running

```bash
# Check health endpoint
curl -k https://localhost:8443/health

# Expected response:
# {
#   "status": "healthy",
#   "service": "xzepr",
#   "version": "0.1.0",
#   "components": {
#     "database": "healthy"
#   }
# }
```

## Configuration

### Environment Variables

Override configuration using environment variables:

```bash
# Database connection
export XZEPR__DATABASE__URL="postgres://xzepr:password@localhost:5432/xzepr"

# Server settings
export XZEPR__SERVER__HOST="0.0.0.0"
export XZEPR__SERVER__PORT="8443"
export XZEPR__SERVER__ENABLE_HTTPS="true"

# Authentication
export XZEPR__AUTH__JWT_SECRET="your-secret-key-min-32-chars"
export XZEPR__AUTH__ENABLE_LOCAL_AUTH="true"
export XZEPR__AUTH__ENABLE_OIDC="true"

# TLS certificates
export XZEPR__TLS__CERT_PATH="certs/cert.pem"
export XZEPR__TLS__KEY_PATH="certs/key.pem"

# Kafka/Redpanda
export XZEPR__KAFKA__BROKERS="localhost:19092"
```

### Configuration Files

The server loads configuration from:

1. `config/default.yaml` - Base configuration
2. `config/development.yaml` or `config/production.yaml` - Environment-specific
3. Environment variables - Highest priority

Set environment with:

```bash
export RUST_ENV=development  # or production
```

## Running Modes

### Development Mode

```bash
# Run with hot reload
make dev-watch

# Or run directly
cargo run --bin xzepr
```

**Features:**

- Detailed logging
- Auto-reload on code changes (with cargo-watch)
- Development JWT secrets
- Local binding (127.0.0.1)

### Production Mode

```bash
# Build release binary
cargo build --release

# Run the binary
export RUST_ENV=production
./target/release/xzepr
```

**Features:**

- Optimized performance
- Production logging levels
- Secure defaults
- Network binding (0.0.0.0)

## TLS/HTTPS Configuration

### Generate Self-Signed Certificates

For development:

```bash
make certs-generate
```

This creates:

- `certs/cert.pem` - Certificate
- `certs/key.pem` - Private key

### Use Custom Certificates

For production with valid certificates:

```bash
# Set paths to your certificates
export XZEPR__TLS__CERT_PATH="/path/to/cert.pem"
export XZEPR__TLS__KEY_PATH="/path/to/key.pem"
```

### Disable HTTPS

For local development only:

```bash
export XZEPR__SERVER__ENABLE_HTTPS="false"
cargo run --bin xzepr
```

Server will run on HTTP at `http://127.0.0.1:8443`

## Logging

### Log Levels

Control logging with the `RUST_LOG` environment variable:

```bash
# Debug level (verbose)
export RUST_LOG=debug
cargo run --bin xzepr

# Info level (default)
export RUST_LOG=info
cargo run --bin xzepr

# Warning level (minimal)
export RUST_LOG=warn
cargo run --bin xzepr

# Module-specific logging
export RUST_LOG=xzepr=debug,tower_http=info
cargo run --bin xzepr
```

### Log Output

Logs include:

- Timestamp
- Log level
- Thread ID
- File and line number
- Request/response details

Example log entry:

```text
2025-10-16T22:20:11.545448Z INFO ThreadId(01) src/main.rs:41: Starting XZEPR Event Tracking Server
2025-10-16T22:20:14.210480Z INFO ThreadId(12) request{method=GET uri=/health}: finished processing request latency=0 ms status=200
```

## Health Checks

### Health Endpoint

```bash
curl -k https://localhost:8443/health
```

**Response codes:**

- `200 OK` - All components healthy
- `503 Service Unavailable` - One or more components unhealthy

### API Status

```bash
curl -k https://localhost:8443/api/v1/status
```

Shows API version and enabled features.

## Available Endpoints

### Public Endpoints

- `GET /` - API information
- `GET /health` - Health check
- `GET /api/v1/status` - API status

### Authentication Endpoints (Coming Soon)

- `POST /api/v1/auth/login` - Local authentication
- `GET /api/v1/auth/oidc/callback` - OIDC callback

### Event Endpoints (Coming Soon)

- `POST /api/v1/events` - Create event
- `GET /api/v1/events` - List events
- `GET /api/v1/events/:id` - Get event by ID
- `DELETE /api/v1/events/:id` - Delete event

## Process Management

### Running in Background

```bash
# Start in background
nohup cargo run --bin xzepr > xzepr.log 2>&1 &
echo $! > xzepr.pid

# Check status
ps -p $(cat xzepr.pid)

# Stop server
kill $(cat xzepr.pid)

# View logs
tail -f xzepr.log
```

### Using systemd

Create `/etc/systemd/system/xzepr.service`:

```ini
[Unit]
Description=XZEPR Event Tracking Server
After=network.target postgresql.service

[Service]
Type=simple
User=xzepr
WorkingDirectory=/opt/xzepr
Environment="RUST_LOG=info"
Environment="RUST_ENV=production"
ExecStart=/opt/xzepr/target/release/xzepr
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

Manage with systemd:

```bash
# Start service
sudo systemctl start xzepr

# Enable on boot
sudo systemctl enable xzepr

# Check status
sudo systemctl status xzepr

# View logs
sudo journalctl -u xzepr -f
```

## Graceful Shutdown

The server handles shutdown signals gracefully:

- `Ctrl+C` (SIGINT)
- `SIGTERM`

On receiving a signal, the server:

1. Stops accepting new connections
2. Completes in-flight requests
3. Closes database connections
4. Exits cleanly

## Troubleshooting

### Server Won't Start

**Check database connection:**

```bash
psql postgres://xzepr:password@localhost:5432/xzepr -c "SELECT 1"
```

**Check if port is in use:**

```bash
sudo lsof -i :8443
```

**Check certificate files:**

```bash
ls -la certs/cert.pem certs/key.pem
```

### Connection Refused

**Verify server is listening:**

```bash
netstat -tlnp | grep 8443
```

**Check firewall rules:**

```bash
sudo firewall-cmd --list-all
```

### Database Errors

**Run migrations:**

```bash
make db-migrate
```

**Check database status:**

```bash
docker compose -f docker-compose.services.yaml ps postgres
```

### TLS Certificate Errors

**Regenerate certificates:**

```bash
rm -rf certs/
make certs-generate
```

**Use HTTP mode temporarily:**

```bash
export XZEPR__SERVER__ENABLE_HTTPS="false"
cargo run --bin xzepr
```

## Performance Tuning

### Database Connection Pool

Configure in `config/production.yaml`:

```yaml
database:
  url: "postgres://xzepr:password@localhost:5432/xzepr"
  max_connections: 100
  min_connections: 10
  connect_timeout: 30
  idle_timeout: 600
```

### Worker Threads

Set Tokio worker threads:

```bash
export TOKIO_WORKER_THREADS=8
cargo run --bin xzepr
```

### Log Levels in Production

```bash
export RUST_LOG=info,tower_http=warn
```

## Monitoring

### Metrics (Coming Soon)

Prometheus metrics will be available at:

```bash
curl -k https://localhost:8443/metrics
```

### Request Tracing

All HTTP requests are traced with:

- Method
- URI
- Status code
- Latency

Example:

```text
INFO request{method=GET uri=/health}: finished processing request latency=0 ms status=200
```

## Next Steps

- [Authentication Setup](authentication.md) - Configure auth providers
- [API Usage](../reference/api.md) - API endpoint documentation
- [Deployment Guide](deployment.md) - Production deployment
