# Setup Monitoring and Metrics

This guide explains how to configure Prometheus monitoring, security metrics,
and observability for XZepr in production.

## Prerequisites

- XZepr application configured with production settings
- Prometheus server (version 2.30 or higher)
- Grafana for visualization (optional but recommended)
- Access to application configuration files

## Overview

XZepr provides comprehensive monitoring through:

- **Prometheus metrics** - HTTP, security, and application metrics
- **Structured logging** - JSON-formatted logs with tracing
- **Health checks** - Component health status endpoints
- **Distributed tracing** - Jaeger integration for request tracking

## Configuration Steps

### Step 1: Enable Monitoring in Configuration

Edit your `config/production.yaml`:

```yaml
security:
  monitoring:
    metrics_enabled: true
    tracing_enabled: true
    log_level: "info"
    jaeger_endpoint: "http://jaeger:14268/api/traces"
```

### Step 2: Configure Logging

Set up structured logging with environment variables:

```bash
# Set log level
export RUST_LOG="info,xzepr=debug"

# Enable JSON logging for production
export XZEPR__LOG_FORMAT="json"

# Log output destination
export XZEPR__LOG_OUTPUT="stdout"
```

### Step 3: Verify Metrics Endpoint

Start the application and check the metrics endpoint:

```bash
cargo run --release
```

Test the metrics endpoint:

```bash
curl http://localhost:8080/metrics
```

You should see Prometheus-formatted metrics:

```text
# HELP xzepr_http_requests_total Total number of HTTP requests
# TYPE xzepr_http_requests_total counter
xzepr_http_requests_total{method="GET",path="/api/v1/events",status="200"} 42

# HELP xzepr_auth_failures_total Total number of authentication failures
# TYPE xzepr_auth_failures_total counter
xzepr_auth_failures_total{reason="invalid_token",client_id="ip:192.168.1.100"} 5

# HELP xzepr_rate_limit_rejections_total Total number of rate limit rejections
# TYPE xzepr_rate_limit_rejections_total counter
xzepr_rate_limit_rejections_total{endpoint="/api/v1/events",client_id="ip:192.168.1.100"} 3
```

## Configure Prometheus

### Step 1: Install Prometheus

Download and install Prometheus:

```bash
# Download Prometheus
wget https://github.com/prometheus/prometheus/releases/download/v2.45.0/prometheus-2.45.0.linux-amd64.tar.gz
tar xvfz prometheus-*.tar.gz
cd prometheus-*
```

### Step 2: Configure Prometheus Scraping

Create `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'xzepr'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
    scrape_timeout: 5s
```

For multiple instances:

```yaml
scrape_configs:
  - job_name: 'xzepr'
    static_configs:
      - targets:
        - 'xzepr-01.internal:8080'
        - 'xzepr-02.internal:8080'
        - 'xzepr-03.internal:8080'
    labels:
      environment: 'production'
      service: 'xzepr'
```

### Step 3: Start Prometheus

```bash
./prometheus --config.file=prometheus.yml
```

Access Prometheus UI at `http://localhost:9090`

## Available Metrics

### Security Metrics

#### Authentication Metrics

```text
xzepr_auth_failures_total{reason, client_id}
xzepr_auth_success_total{method, user_id}
```

Example queries:

```promql
# Authentication failure rate
rate(xzepr_auth_failures_total[5m])

# Failed authentications by reason
sum by (reason) (xzepr_auth_failures_total)

# Auth success rate
rate(xzepr_auth_success_total[5m])
```

#### Rate Limiting Metrics

```text
xzepr_rate_limit_rejections_total{endpoint, client_id}
```

Example queries:

```promql
# Rate limit rejection rate
rate(xzepr_rate_limit_rejections_total[5m])

# Top endpoints being rate limited
topk(10, sum by (endpoint) (xzepr_rate_limit_rejections_total))

# Clients hitting rate limits
sum by (client_id) (xzepr_rate_limit_rejections_total)
```

#### CORS and Validation Metrics

```text
xzepr_cors_violations_total{origin, endpoint}
xzepr_validation_errors_total{endpoint, field}
xzepr_graphql_complexity_violations_total{client_id}
```

Example queries:

```promql
# CORS violations by origin
sum by (origin) (xzepr_cors_violations_total)

# Validation errors by field
sum by (field) (xzepr_validation_errors_total)

# GraphQL complexity violations
rate(xzepr_graphql_complexity_violations_total[5m])
```

### Application Metrics

#### HTTP Request Metrics

```text
xzepr_http_requests_total{method, path, status}
xzepr_http_request_duration_seconds{method, path, status}
```

Example queries:

```promql
# Request rate
rate(xzepr_http_requests_total[5m])

# Request rate by status code
sum by (status) (rate(xzepr_http_requests_total[5m]))

# 95th percentile latency
histogram_quantile(0.95, rate(xzepr_http_request_duration_seconds_bucket[5m]))

# Average latency by endpoint
sum by (path) (rate(xzepr_http_request_duration_seconds_sum[5m]))
/
sum by (path) (rate(xzepr_http_request_duration_seconds_count[5m]))
```

#### Connection Metrics

```text
xzepr_active_connections
```

Example queries:

```promql
# Current active connections
xzepr_active_connections

# Maximum connections over time
max_over_time(xzepr_active_connections[1h])
```

### System Metrics

```text
xzepr_uptime_seconds
xzepr_info{version}
```

Example queries:

```promql
# Server uptime in hours
xzepr_uptime_seconds / 3600

# Application version
xzepr_info
```

## Configure Alerting

### Step 1: Create Alert Rules

Create `alerts.yml`:

```yaml
groups:
  - name: xzepr_security
    interval: 30s
    rules:
      # High authentication failure rate
      - alert: HighAuthFailureRate
        expr: rate(xzepr_auth_failures_total[5m]) > 10
        for: 2m
        labels:
          severity: warning
          category: security
        annotations:
          summary: "High authentication failure rate"
          description: "Auth failures: {{ $value }} per second"

      # Sustained rate limit rejections
      - alert: HighRateLimitRejections
        expr: rate(xzepr_rate_limit_rejections_total[5m]) > 5
        for: 5m
        labels:
          severity: warning
          category: security
        annotations:
          summary: "High rate of rate limit rejections"
          description: "Rate limit rejections: {{ $value }} per second"

      # CORS violations detected
      - alert: CorsViolations
        expr: increase(xzepr_cors_violations_total[10m]) > 10
        for: 1m
        labels:
          severity: info
          category: security
        annotations:
          summary: "CORS violations detected"
          description: "Origin {{ $labels.origin }} has {{ $value }} violations"

  - name: xzepr_application
    interval: 30s
    rules:
      # High error rate
      - alert: HighErrorRate
        expr: |
          sum(rate(xzepr_http_requests_total{status=~"5.."}[5m]))
          /
          sum(rate(xzepr_http_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
          category: application
        annotations:
          summary: "High HTTP error rate"
          description: "Error rate: {{ $value | humanizePercentage }}"

      # High latency
      - alert: HighLatency
        expr: |
          histogram_quantile(0.95,
            rate(xzepr_http_request_duration_seconds_bucket[5m])
          ) > 1.0
        for: 5m
        labels:
          severity: warning
          category: performance
        annotations:
          summary: "High request latency"
          description: "95th percentile: {{ $value }}s"

      # Service down
      - alert: ServiceDown
        expr: up{job="xzepr"} == 0
        for: 1m
        labels:
          severity: critical
          category: availability
        annotations:
          summary: "XZepr service is down"
          description: "Instance {{ $labels.instance }} is unreachable"
```

### Step 2: Configure Alertmanager

Create `alertmanager.yml`:

```yaml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'severity']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'team-notifications'
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
    - match:
        category: security
      receiver: 'security-team'

receivers:
  - name: 'team-notifications'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK'
        channel: '#alerts'
        title: 'XZepr Alert'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_KEY'

  - name: 'security-team'
    email_configs:
      - to: 'security@example.com'
        from: 'alerts@example.com'
        smarthost: 'smtp.example.com:587'
```

## Setup Grafana Dashboards

### Step 1: Install Grafana

```bash
# Ubuntu/Debian
sudo apt-get install -y grafana

# Start Grafana
sudo systemctl start grafana-server
sudo systemctl enable grafana-server
```

Access Grafana at `http://localhost:3000` (default credentials: admin/admin)

### Step 2: Add Prometheus Data Source

1. Navigate to Configuration → Data Sources
2. Click "Add data source"
3. Select "Prometheus"
4. Set URL to `http://localhost:9090`
5. Click "Save & Test"

### Step 3: Create Security Dashboard

Create a dashboard with these panels:

#### Authentication Panel

```promql
# Auth failures by reason
sum by (reason) (rate(xzepr_auth_failures_total[5m]))
```

#### Rate Limiting Panel

```promql
# Rate limit rejections over time
sum(rate(xzepr_rate_limit_rejections_total[5m]))
```

#### Request Rate Panel

```promql
# Requests per second by status
sum by (status) (rate(xzepr_http_requests_total[5m]))
```

#### Latency Panel

```promql
# P50, P95, P99 latency
histogram_quantile(0.50, rate(xzepr_http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.95, rate(xzepr_http_request_duration_seconds_bucket[5m]))
histogram_quantile(0.99, rate(xzepr_http_request_duration_seconds_bucket[5m]))
```

### Sample Dashboard JSON

Import this dashboard configuration:

```json
{
  "dashboard": {
    "title": "XZepr Security Monitoring",
    "panels": [
      {
        "title": "Authentication Failures",
        "targets": [
          {
            "expr": "sum(rate(xzepr_auth_failures_total[5m]))"
          }
        ]
      },
      {
        "title": "Rate Limit Rejections",
        "targets": [
          {
            "expr": "sum(rate(xzepr_rate_limit_rejections_total[5m]))"
          }
        ]
      }
    ]
  }
}
```

## Structured Logging

### Log Format

XZepr logs are structured in JSON format:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "WARN",
  "target": "xzepr::api::middleware::rate_limit",
  "fields": {
    "event": "rate_limit_exceeded",
    "client_id": "ip:192.168.1.100",
    "endpoint": "/api/v1/events",
    "limit": 10
  },
  "message": "Rate limit exceeded"
}
```

### Query Logs

Use `jq` to parse JSON logs:

```bash
# Filter by log level
cat logs/xzepr.log | jq 'select(.level == "ERROR")'

# Count events by type
cat logs/xzepr.log | jq -r '.fields.event' | sort | uniq -c

# Find rate limit events
cat logs/xzepr.log | jq 'select(.fields.event == "rate_limit_exceeded")'

# Extract authentication failures
cat logs/xzepr.log | jq 'select(.fields.event == "authentication_failed")'
```

### Log Aggregation

Forward logs to a centralized system:

```bash
# To Elasticsearch
filebeat -e -c filebeat.yml

# To Loki
promtail -config.file=promtail.yml

# To CloudWatch
aws logs put-log-events --log-group-name xzepr --log-stream-name production
```

## Distributed Tracing

### Configure Jaeger

Start Jaeger all-in-one:

```bash
docker run -d --name jaeger \
  -p 5775:5775/udp \
  -p 6831:6831/udp \
  -p 6832:6832/udp \
  -p 5778:5778 \
  -p 16686:16686 \
  -p 14268:14268 \
  -p 14250:14250 \
  -p 9411:9411 \
  jaegertracing/all-in-one:latest
```

Configure XZepr to send traces:

```yaml
security:
  monitoring:
    tracing_enabled: true
    jaeger_endpoint: "http://localhost:14268/api/traces"
```

Access Jaeger UI at `http://localhost:16686`

## Health Check Endpoint

XZepr provides a health check endpoint at `/health`:

```bash
curl http://localhost:8080/health
```

Response:

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "components": [
    {
      "name": "database",
      "status": "healthy",
      "response_time_ms": 5
    },
    {
      "name": "redis",
      "status": "healthy",
      "response_time_ms": 2
    }
  ]
}
```

Use for:

- Kubernetes liveness/readiness probes
- Load balancer health checks
- Monitoring system checks

## Troubleshooting

### Metrics Not Appearing

**Problem**: Prometheus shows no data

**Solutions**:

1. Verify metrics endpoint is accessible:
   ```bash
   curl http://localhost:8080/metrics
   ```

2. Check Prometheus targets page: `http://localhost:9090/targets`

3. Verify scrape configuration in `prometheus.yml`

4. Check firewall rules between Prometheus and XZepr

### High Cardinality Issues

**Problem**: Too many unique label combinations

**Solutions**:

1. Avoid high-cardinality labels like user IDs or timestamps
2. Use `client_id` hash instead of full identifier
3. Aggregate paths to reduce endpoint combinations
4. Set retention policies in Prometheus

### Missing Logs

**Problem**: Logs not appearing in aggregation system

**Solutions**:

1. Check log level is set correctly
2. Verify log format matches parser expectations
3. Check log shipping agent status
4. Verify network connectivity to log aggregator

## Production Recommendations

### Metrics Retention

Configure Prometheus retention:

```bash
./prometheus \
  --config.file=prometheus.yml \
  --storage.tsdb.retention.time=30d \
  --storage.tsdb.retention.size=50GB
```

### High Availability

Run multiple Prometheus instances with federation:

```yaml
# prometheus-federated.yml
scrape_configs:
  - job_name: 'federate'
    scrape_interval: 15s
    honor_labels: true
    metrics_path: '/federate'
    params:
      'match[]':
        - '{job="xzepr"}'
    static_configs:
      - targets:
        - 'prometheus-1:9090'
        - 'prometheus-2:9090'
```

### Monitoring Best Practices

1. **Set appropriate scrape intervals** - Balance between freshness and load
2. **Use recording rules** - Pre-calculate expensive queries
3. **Configure retention policies** - Manage storage costs
4. **Enable remote write** - Long-term storage in Thanos or Cortex
5. **Monitor the monitors** - Alert on Prometheus/Grafana issues

## Next Steps

- Configure custom metrics for business logic
- Set up log-based alerting
- Integrate with incident management system
- Create runbooks for common alerts
- Implement SLO monitoring

## Related Documentation

- [Configure Redis Rate Limiting](configure_redis_rate_limiting.md)
- [Security Configuration](../reference/security_configuration.md)
- [Production Deployment](deploy_production.md)
