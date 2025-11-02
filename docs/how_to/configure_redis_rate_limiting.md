# Configure Redis Rate Limiting

This guide explains how to set up Redis-backed rate limiting for distributed
deployments of XZepr.

## Prerequisites

- Redis server (version 6.0 or higher recommended)
- XZepr application configured with production settings
- Access to application configuration files

## Why Redis Rate Limiting

In production deployments with multiple application instances, in-memory rate
limiting is insufficient because each instance maintains its own state. This
allows clients to bypass rate limits by distributing requests across instances.

Redis-backed rate limiting solves this by:

- **Shared state** - All instances check the same Redis database
- **Atomic operations** - Uses Lua scripts for race-condition-free counting
- **Distributed enforcement** - Rate limits apply across all instances
- **Sliding window** - More accurate than fixed window counting

## Configuration Steps

### Step 1: Install and Configure Redis

Install Redis on your server or use a managed service:

```bash
# Ubuntu/Debian
sudo apt-get install redis-server

# Start Redis
sudo systemctl start redis
sudo systemctl enable redis
```

For production, configure Redis with:

```conf
# /etc/redis/redis.conf
bind 127.0.0.1
port 6379
requirepass your_secure_password_here
maxmemory 256mb
maxmemory-policy allkeys-lru
```

Restart Redis after configuration changes:

```bash
sudo systemctl restart redis
```

### Step 2: Enable Redis Rate Limiting in Configuration

Edit your `config/production.yaml`:

```yaml
security:
  rate_limit:
    use_redis: true
    anonymous_rpm: 10
    authenticated_rpm: 100
    admin_rpm: 1000
    per_endpoint:
      /auth/login: 5
      /auth/register: 3
      /api/v1/events: 50
```

### Step 3: Set Redis Connection URL

Set the Redis connection URL as an environment variable:

```bash
# For local Redis without password
export XZEPR__REDIS_URL="redis://127.0.0.1:6379"

# For Redis with password
export XZEPR__REDIS_URL="redis://:your_password@127.0.0.1:6379"

# For Redis on remote host
export XZEPR__REDIS_URL="redis://:password@redis.example.com:6379/0"

# For Redis Cluster
export XZEPR__REDIS_URL="redis://node1:6379,node2:6379,node3:6379"
```

### Step 4: Verify Redis Connection

Start the application and check logs for Redis connection status:

```bash
cargo run --release
```

Look for log messages:

```text
INFO xzepr::api::router: Using Redis-backed rate limiting at redis://127.0.0.1:6379
```

If Redis connection fails, you'll see:

```text
ERROR xzepr::api::router: Failed to connect to Redis: Connection refused. Falling back to in-memory rate limiting
```

## Testing Rate Limiting

### Test Basic Rate Limiting

Use curl to test rate limiting:

```bash
# Test anonymous rate limit (10 requests/minute)
for i in {1..15}; do
  curl -i http://localhost:8080/api/v1/events
  echo "Request $i"
done
```

After the 10th request, you should receive:

```text
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 60
Retry-After: 60
```

### Test Endpoint-Specific Limits

Test the login endpoint with a lower limit:

```bash
# Test login endpoint (5 requests/minute)
for i in {1..8}; do
  curl -i -X POST http://localhost:8080/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","password":"test"}'
  echo "Request $i"
done
```

### Test Distributed Rate Limiting

Start multiple instances and verify they share rate limit state:

```bash
# Terminal 1
XZEPR__SERVER__PORT=8080 cargo run --release

# Terminal 2
XZEPR__SERVER__PORT=8081 cargo run --release
```

Test that rate limits apply across both instances:

```bash
# Send 5 requests to instance 1
for i in {1..5}; do
  curl http://localhost:8080/api/v1/events
done

# Send 5 more to instance 2 - should start rejecting
for i in {1..5}; do
  curl http://localhost:8081/api/v1/events
done
```

## Monitoring Redis Rate Limiting

### Check Redis Keys

Connect to Redis and inspect rate limiting keys:

```bash
redis-cli

# List all rate limit keys
KEYS ratelimit:*

# Check a specific key
ZCARD ratelimit:ip:192.168.1.100
ZRANGE ratelimit:ip:192.168.1.100 0 -1 WITHSCORES
```

### Monitor Rate Limit Metrics

XZepr exposes rate limit rejection metrics:

```bash
curl http://localhost:8080/metrics | grep rate_limit
```

Look for:

```text
xzepr_rate_limit_rejections_total{endpoint="/api/v1/events",client_id="ip:192.168.1.100"} 5
```

### Check Application Logs

Rate limit rejections are logged:

```text
WARN xzepr::api::middleware::rate_limit: Rate limit exceeded
    key = "ip:192.168.1.100"
    path = "/api/v1/events"
    limit = 10
```

## Troubleshooting

### Redis Connection Errors

**Problem**: `Failed to connect to Redis: Connection refused`

**Solutions**:

1. Verify Redis is running:

   ```bash
   redis-cli ping
   # Should return: PONG
   ```

2. Check Redis is listening on the correct port:

   ```bash
   netstat -tlnp | grep redis
   ```

3. Verify firewall rules allow connection

4. Check Redis password matches configuration

### High Memory Usage

**Problem**: Redis memory usage grows over time

**Solutions**:

1. Configure `maxmemory` in redis.conf:

   ```conf
   maxmemory 256mb
   maxmemory-policy allkeys-lru
   ```

2. Rate limit keys automatically expire after the window period

3. Monitor memory usage:

   ```bash
   redis-cli info memory
   ```

### Performance Issues

**Problem**: Rate limiting adds latency to requests

**Solutions**:

1. Use Redis on the same network segment as application servers

2. Enable Redis pipelining for better performance

3. Consider Redis Cluster for higher throughput

4. Monitor Redis latency:

   ```bash
   redis-cli --latency
   ```

### Rate Limits Not Shared Across Instances

**Problem**: Each instance has independent rate limits

**Solutions**:

1. Verify `use_redis: true` in configuration

2. Check all instances use the same `XZEPR__REDIS_URL`

3. Verify Redis is not running in cluster mode without proper configuration

4. Check logs for Redis connection messages on all instances

## Production Recommendations

### Redis Configuration

```conf
# High availability
save 900 1
save 300 10
save 60 10000

# Performance
tcp-backlog 511
timeout 0
tcp-keepalive 300

# Security
requirepass strong_password_here
rename-command CONFIG ""
rename-command FLUSHDB ""
rename-command FLUSHALL ""

# Memory
maxmemory 512mb
maxmemory-policy allkeys-lru
```

### Environment Variables

```bash
# Production environment file
export XZEPR__REDIS_URL="redis://:${REDIS_PASSWORD}@redis.internal:6379/0"
export XZEPR__SECURITY__RATE_LIMIT__USE_REDIS=true
export XZEPR__SECURITY__RATE_LIMIT__ANONYMOUS_RPM=10
export XZEPR__SECURITY__RATE_LIMIT__AUTHENTICATED_RPM=100
export XZEPR__SECURITY__RATE_LIMIT__ADMIN_RPM=1000
```

### High Availability Setup

For production deployments, use Redis Sentinel or Redis Cluster:

```yaml
# Redis Sentinel example
XZEPR__REDIS_URL: "redis://sentinel1:26379,sentinel2:26379,sentinel3:26379/mymaster"
```

### Monitoring and Alerts

Set up alerts for:

- Redis connection failures
- High rate limit rejection rates
- Redis memory usage above 80%
- Redis latency above 10ms

## Security Considerations

- **Never expose Redis to the public internet**
- **Always use strong passwords** for Redis authentication
- **Use TLS** for Redis connections in production
- **Restrict Redis commands** using `rename-command`
- **Monitor for suspicious rate limit patterns**

## Next Steps

- Configure Prometheus alerts for rate limiting metrics
- Set up Redis monitoring with Redis Exporter
- Implement custom rate limit tiers for different API keys
- Configure per-user rate limits based on subscription level

## Related Documentation

- [Security Configuration](../reference/security_configuration.md)
- [Monitoring and Metrics](../how_to/setup_monitoring.md)
- [Production Deployment](../how_to/deploy_production.md)
