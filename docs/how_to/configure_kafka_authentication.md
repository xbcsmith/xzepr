# How to Configure Kafka Authentication

This guide explains how to configure XZepr to connect to Kafka clusters with
authentication enabled using SASL/SCRAM mechanisms.

## Overview

XZepr supports multiple Kafka authentication mechanisms to connect to secured
Kafka clusters:

- **PLAINTEXT** - No authentication (default, for development only)
- **SASL_PLAINTEXT** - SASL authentication without SSL/TLS encryption
- **SASL_SSL** - SASL authentication with SSL/TLS encryption (recommended for
  production)
- **SSL** - SSL/TLS authentication using certificates

Supported SASL mechanisms:

- **SCRAM-SHA-256** - Recommended for most use cases
- **SCRAM-SHA-512** - Higher security requirements
- **PLAIN** - Simple username/password (less secure)
- **GSSAPI** - Kerberos authentication
- **OAUTHBEARER** - OAuth 2.0 authentication

## Prerequisites

Before configuring authentication:

1. Kafka cluster with authentication enabled
2. Valid credentials (username and password for SASL)
3. SSL certificates (if using SSL/TLS)
4. Network access to Kafka brokers

## Configuration Methods

XZepr supports two methods for configuring Kafka authentication:

1. **Environment Variables** - Recommended for production deployments
2. **YAML Configuration Files** - Useful for development and testing

### Method 1: Environment Variables

Environment variables provide the most secure and flexible way to configure
Kafka authentication, especially in containerized environments.

#### SASL/SCRAM-SHA-256 with SSL (Recommended)

```bash
# Security protocol
export XZEPR_KAFKA_SECURITY_PROTOCOL="SASL_SSL"

# SASL configuration
export XZEPR_KAFKA_SASL_MECHANISM="SCRAM-SHA-256"
export XZEPR_KAFKA_SASL_USERNAME="your-kafka-username"
export XZEPR_KAFKA_SASL_PASSWORD="your-kafka-password"

# SSL configuration (paths to certificate files)
export XZEPR_KAFKA_SSL_CA_LOCATION="/path/to/ca-cert.pem"
export XZEPR_KAFKA_SSL_CERTIFICATE_LOCATION="/path/to/client-cert.pem"
export XZEPR_KAFKA_SSL_KEY_LOCATION="/path/to/client-key.pem"

# Kafka brokers
export XZEPR_KAFKA_BROKERS="broker1.example.com:9093,broker2.example.com:9093"
export XZEPR_KAFKA_TOPIC="xzepr-events"
```

#### SASL/SCRAM-SHA-512 with SSL

```bash
export XZEPR_KAFKA_SECURITY_PROTOCOL="SASL_SSL"
export XZEPR_KAFKA_SASL_MECHANISM="SCRAM-SHA-512"
export XZEPR_KAFKA_SASL_USERNAME="your-kafka-username"
export XZEPR_KAFKA_SASL_PASSWORD="your-kafka-password"
export XZEPR_KAFKA_SSL_CA_LOCATION="/path/to/ca-cert.pem"
```

#### SASL/PLAIN without SSL (Not Recommended for Production)

```bash
export XZEPR_KAFKA_SECURITY_PROTOCOL="SASL_PLAINTEXT"
export XZEPR_KAFKA_SASL_MECHANISM="PLAIN"
export XZEPR_KAFKA_SASL_USERNAME="your-kafka-username"
export XZEPR_KAFKA_SASL_PASSWORD="your-kafka-password"
```

#### SSL Only (Certificate-Based Authentication)

```bash
export XZEPR_KAFKA_SECURITY_PROTOCOL="SSL"
export XZEPR_KAFKA_SSL_CA_LOCATION="/path/to/ca-cert.pem"
export XZEPR_KAFKA_SSL_CERTIFICATE_LOCATION="/path/to/client-cert.pem"
export XZEPR_KAFKA_SSL_KEY_LOCATION="/path/to/client-key.pem"
```

#### No Authentication (Development Only)

```bash
export XZEPR_KAFKA_SECURITY_PROTOCOL="PLAINTEXT"
# No credentials needed
```

### Method 2: YAML Configuration Files

Add authentication configuration to your YAML configuration file
(e.g., `config/production.yaml`):

#### SASL/SCRAM-SHA-256 with SSL

```yaml
kafka:
  brokers: "broker1.example.com:9093,broker2.example.com:9093"
  topic: "xzepr-events"
  auth:
    security_protocol: "SASL_SSL"
    sasl:
      mechanism: "SCRAM-SHA-256"
      username: "your-kafka-username"
      password: "your-kafka-password"
    ssl:
      ca_location: "/path/to/ca-cert.pem"
      certificate_location: "/path/to/client-cert.pem"
      key_location: "/path/to/client-key.pem"
```

#### SASL/SCRAM-SHA-512 with SSL

```yaml
kafka:
  brokers: "broker1.example.com:9093,broker2.example.com:9093"
  topic: "xzepr-events"
  auth:
    security_protocol: "SASL_SSL"
    sasl:
      mechanism: "SCRAM-SHA-512"
      username: "your-kafka-username"
      password: "your-kafka-password"
    ssl:
      ca_location: "/path/to/ca-cert.pem"
```

#### No Authentication

```yaml
kafka:
  brokers: "localhost:9092"
  topic: "xzepr-events"
  auth:
    security_protocol: "PLAINTEXT"
```

## Security Best Practices

### Credential Management

1. **Never commit credentials to version control**

   ```bash
   # Add to .gitignore
   echo "config/*production*.yaml" >> .gitignore
   echo "config/*secrets*.yaml" >> .gitignore
   ```

2. **Use environment variables in production**

   Environment variables are more secure than config files because they:
   - Are not stored on disk
   - Can be managed by secrets management systems
   - Are easier to rotate

3. **Use secrets management systems**

   For production deployments, use:
   - Kubernetes Secrets
   - HashiCorp Vault
   - AWS Secrets Manager
   - Azure Key Vault
   - Google Secret Manager

4. **Rotate credentials regularly**

   Implement a credential rotation policy:
   - Change passwords every 90 days
   - Update SSL certificates before expiration
   - Use automated rotation where possible

### SSL/TLS Configuration

1. **Always use SSL in production**

   ```bash
   # Prefer SASL_SSL over SASL_PLAINTEXT
   export XZEPR_KAFKA_SECURITY_PROTOCOL="SASL_SSL"
   ```

2. **Verify certificate paths**

   Ensure certificate files exist and have correct permissions:

   ```bash
   # Verify files exist
   ls -l /path/to/ca-cert.pem
   ls -l /path/to/client-cert.pem
   ls -l /path/to/client-key.pem

   # Set correct permissions
   chmod 600 /path/to/client-key.pem
   chmod 644 /path/to/client-cert.pem
   chmod 644 /path/to/ca-cert.pem
   ```

3. **Use valid certificates**

   - Use certificates from trusted CAs
   - Verify certificate expiration dates
   - Monitor certificate expiration

### Authentication Mechanism Selection

Choose the appropriate mechanism for your use case:

| Mechanism | Security Level | Use Case |
|-----------|---------------|----------|
| PLAINTEXT | None | Development only |
| SASL_PLAINTEXT | Low | Development/testing |
| SASL_SSL (PLAIN) | Medium | Internal networks |
| SASL_SSL (SCRAM-SHA-256) | High | Production (recommended) |
| SASL_SSL (SCRAM-SHA-512) | Highest | High security requirements |
| SSL (mTLS) | High | Certificate-based auth |

## Verification Steps

After configuring authentication, verify the connection:

### 1. Check Configuration Loading

Start XZepr with verbose logging:

```bash
export RUST_LOG="debug,xzepr=trace"
./target/release/server
```

Look for log messages indicating successful authentication configuration:

```text
[INFO xzepr::infrastructure::messaging] Loading Kafka authentication config
[DEBUG xzepr::infrastructure::messaging] Security protocol: SASL_SSL
[DEBUG xzepr::infrastructure::messaging] SASL mechanism: SCRAM-SHA-256
[INFO xzepr::infrastructure::messaging] Kafka authentication configured successfully
```

### 2. Test Connection

Use the admin CLI to verify the Kafka connection:

```bash
# This will attempt to create the topic and verify connectivity
./target/release/admin verify-kafka
```

### 3. Publish Test Event

Create a test event to verify message publishing:

```bash
# Get authentication token
TOKEN=$(curl -X POST https://localhost:8443/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}' \
  -k -s | jq -r '.token')

# Create event receiver
RECEIVER=$(curl -X POST https://localhost:8443/api/v1/event-receivers \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Test Receiver", "description": "Auth test"}' \
  -k -s | jq -r '.id')

# Create test event
curl -X POST https://localhost:8443/api/v1/events \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "auth-test",
    "version": "1.0.0",
    "release": "2024.01",
    "platform_id": "test",
    "package": "xzepr",
    "description": "Authentication test event",
    "success": true,
    "event_receiver_id": "'$RECEIVER'"
  }' -k
```

### 4. Verify Message in Kafka

If you have access to Kafka tools, verify the message was published:

```bash
# Using kafka-console-consumer (if available)
kafka-console-consumer \
  --bootstrap-server broker1.example.com:9093 \
  --topic xzepr-events \
  --from-beginning \
  --consumer.config /path/to/consumer.properties
```

Or use Redpanda Console (if available):

```text
Navigate to: http://localhost:8081/topics/xzepr-events
```

## Troubleshooting Common Issues

### Issue: Authentication Failed

**Symptoms**:

```text
ERROR: Authentication failed: Invalid credentials
ERROR: SASL authentication failed
```

**Solutions**:

1. Verify credentials are correct:

   ```bash
   echo $XZEPR_KAFKA_SASL_USERNAME
   echo $XZEPR_KAFKA_SASL_PASSWORD
   ```

2. Check Kafka broker allows the SASL mechanism:

   ```bash
   # Verify broker configuration supports SCRAM-SHA-256
   kafka-configs --describe --entity-type brokers --entity-name 0
   ```

3. Ensure user exists in Kafka:

   ```bash
   # List Kafka SCRAM users
   kafka-configs --describe --entity-type users
   ```

### Issue: SSL Certificate Verification Failed

**Symptoms**:

```text
ERROR: SSL handshake failed
ERROR: Certificate verification failed
ERROR: unable to get local issuer certificate
```

**Solutions**:

1. Verify certificate files exist:

   ```bash
   ls -l $XZEPR_KAFKA_SSL_CA_LOCATION
   ls -l $XZEPR_KAFKA_SSL_CERTIFICATE_LOCATION
   ls -l $XZEPR_KAFKA_SSL_KEY_LOCATION
   ```

2. Check certificate validity:

   ```bash
   openssl x509 -in $XZEPR_KAFKA_SSL_CERTIFICATE_LOCATION -text -noout
   openssl x509 -in $XZEPR_KAFKA_SSL_CA_LOCATION -text -noout
   ```

3. Verify certificate chain:

   ```bash
   openssl verify -CAfile $XZEPR_KAFKA_SSL_CA_LOCATION \
     $XZEPR_KAFKA_SSL_CERTIFICATE_LOCATION
   ```

4. Check certificate permissions:

   ```bash
   chmod 600 $XZEPR_KAFKA_SSL_KEY_LOCATION
   ```

### Issue: Connection Timeout

**Symptoms**:

```text
ERROR: Connection timeout
ERROR: Failed to resolve broker addresses
```

**Solutions**:

1. Verify broker addresses and ports:

   ```bash
   echo $XZEPR_KAFKA_BROKERS
   # Should be: "host1:port1,host2:port2"
   ```

2. Test network connectivity:

   ```bash
   telnet broker1.example.com 9093
   nc -zv broker1.example.com 9093
   ```

3. Check firewall rules allow outbound connections

4. Verify DNS resolution:

   ```bash
   nslookup broker1.example.com
   dig broker1.example.com
   ```

### Issue: Missing Configuration

**Symptoms**:

```text
ERROR: Missing SASL credentials
ERROR: SSL configuration required but certificate paths not provided
```

**Solutions**:

1. Verify all required environment variables are set:

   ```bash
   # For SASL_SSL, you need:
   env | grep XZEPR_KAFKA
   ```

2. Check YAML configuration syntax:

   ```bash
   # Validate YAML syntax
   yamllint config/production.yaml
   ```

3. Ensure configuration file is loaded:

   ```bash
   export XZEPR_CONFIG_FILE="config/production.yaml"
   ```

### Issue: Permission Denied

**Symptoms**:

```text
ERROR: Authorization failed
ERROR: Topic authorization failed
```

**Solutions**:

1. Verify Kafka ACLs grant access to the user:

   ```bash
   kafka-acls --list --authorizer-properties \
     zookeeper.connect=localhost:2181 \
     --topic xzepr-events
   ```

2. Grant necessary permissions:

   ```bash
   kafka-acls --add \
     --allow-principal User:your-kafka-username \
     --operation Write \
     --topic xzepr-events
   ```

## Docker Configuration

### Using Environment Variables

Create a `.env` file for Docker Compose:

```bash
# .env.production
XZEPR_KAFKA_SECURITY_PROTOCOL=SASL_SSL
XZEPR_KAFKA_SASL_MECHANISM=SCRAM-SHA-256
XZEPR_KAFKA_SASL_USERNAME=kafka-user
XZEPR_KAFKA_SASL_PASSWORD=kafka-password
XZEPR_KAFKA_SSL_CA_LOCATION=/app/certs/ca-cert.pem
XZEPR_KAFKA_BROKERS=broker1.example.com:9093
```

Update `docker-compose.yaml`:

```yaml
services:
  xzepr:
    image: xzepr:latest
    env_file:
      - .env.production
    volumes:
      - ./certs:/app/certs:ro
    environment:
      - RUST_LOG=info,xzepr=debug
```

Start with Docker Compose:

```bash
docker compose --env-file .env.production up -d
```

### Using Docker Secrets

For production Docker Swarm deployments:

```bash
# Create secrets
echo "kafka-user" | docker secret create kafka_username -
echo "kafka-password" | docker secret create kafka_password -

# Update docker-compose.yaml
services:
  xzepr:
    secrets:
      - kafka_username
      - kafka_password
    environment:
      - XZEPR_KAFKA_SASL_USERNAME_FILE=/run/secrets/kafka_username
      - XZEPR_KAFKA_SASL_PASSWORD_FILE=/run/secrets/kafka_password

secrets:
  kafka_username:
    external: true
  kafka_password:
    external: true
```

## Kubernetes Configuration

### Using Kubernetes Secrets

Create a secret for Kafka credentials:

```bash
kubectl create secret generic kafka-credentials \
  --from-literal=username=kafka-user \
  --from-literal=password=kafka-password \
  --namespace=xzepr
```

Reference in deployment:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xzepr
spec:
  template:
    spec:
      containers:
      - name: xzepr
        image: xzepr:latest
        env:
        - name: XZEPR_KAFKA_SECURITY_PROTOCOL
          value: "SASL_SSL"
        - name: XZEPR_KAFKA_SASL_MECHANISM
          value: "SCRAM-SHA-256"
        - name: XZEPR_KAFKA_SASL_USERNAME
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: username
        - name: XZEPR_KAFKA_SASL_PASSWORD
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: password
        - name: XZEPR_KAFKA_SSL_CA_LOCATION
          value: "/app/certs/ca-cert.pem"
        volumeMounts:
        - name: kafka-certs
          mountPath: /app/certs
          readOnly: true
      volumes:
      - name: kafka-certs
        secret:
          secretName: kafka-ssl-certs
```

## Performance Considerations

### Connection Pooling

XZepr maintains a pool of Kafka connections. Configure pool size based on load:

```yaml
kafka:
  connection_pool_size: 10  # Default
  connection_timeout_ms: 5000
  request_timeout_ms: 30000
```

### Message Batching

Enable batching for better throughput:

```yaml
kafka:
  batch_size: 1000
  linger_ms: 100
```

### Compression

Enable compression to reduce network usage:

```yaml
kafka:
  compression_type: "snappy"  # Options: none, gzip, snappy, lz4, zstd
```

## Migration from Unauthenticated Kafka

If you are migrating from an unauthenticated Kafka setup:

1. **Add authentication configuration** (environment variables or YAML)
2. **Update Kafka broker configuration** to enable SASL/SSL
3. **Create Kafka users** with appropriate ACLs
4. **Test in development** environment first
5. **Rolling deployment** to production:
   - Deploy new configuration to one instance
   - Verify successful connection
   - Roll out to remaining instances
6. **Monitor logs** for authentication errors
7. **Update monitoring alerts** if needed

## Additional Resources

- [Kafka SASL/SCRAM Authentication](https://kafka.apache.org/documentation/#security_sasl_scram)
- [Kafka SSL Configuration](https://kafka.apache.org/documentation/#security_ssl)
- [librdkafka Configuration](https://github.com/edenhill/librdkafka/blob/master/CONFIGURATION.md)
- [XZepr Configuration Reference](../reference/configuration.md)
- [XZepr Event Publication](../explanations/event_publication_implementation.md)

## Summary

This guide covered:

- Supported authentication mechanisms
- Environment variable and YAML configuration
- Security best practices
- Verification steps
- Troubleshooting common issues
- Docker and Kubernetes deployment
- Migration from unauthenticated setup

For additional help:

- Check logs: `RUST_LOG=debug ./target/release/server`
- Review configuration: Verify all required settings are present
- Test connectivity: Use network tools to verify broker access
- Consult Kafka broker logs for server-side errors

Authentication ensures secure communication between XZepr and your Kafka
cluster, protecting your event data in transit and at rest.
