# Kafka Security Best Practices

## Overview

This guide provides comprehensive security best practices for configuring and operating XZepr with Kafka/Redpanda using SASL/SCRAM authentication. Following these practices ensures secure credential management, network security, and operational safety.

## Credential Management

### Never Hardcode Credentials

**DO NOT** hardcode credentials in source code:

```rust
// WRONG - Never do this
let config = KafkaAuthConfig::scram_sha256_ssl(
    "my-username".to_string(),
    "my-password".to_string(),
    Some("/path/to/ca.pem".to_string()),
)?;
```

**CORRECT** - Load from environment variables or secure configuration:

```rust
// Load from environment
let auth_config = KafkaAuthConfig::from_env()?;

// Or from secure configuration management
let auth_config = load_config_from_vault()?;
```

### Environment Variable Best Practices

**Use environment variables for local development and testing only**. For production, use dedicated secret management solutions.

**Minimum required variables**:

```bash
export KAFKA_SECURITY_PROTOCOL=SASL_SSL
export KAFKA_SASL_MECHANISM=SCRAM-SHA-256
export KAFKA_SASL_USERNAME=your-username
export KAFKA_SASL_PASSWORD=your-password
export KAFKA_SSL_CA_LOCATION=/path/to/ca-cert.pem
```

**Security considerations**:

1. Never commit `.env` files containing credentials to version control
2. Add `.env` to `.gitignore`
3. Use different credentials for each environment (dev, staging, production)
4. Rotate credentials regularly (at least every 90 days)
5. Use strong, randomly generated passwords (minimum 32 characters)

### Secure Configuration Storage

#### Development and Testing

For local development, use environment variables or encrypted configuration files:

```bash
# Option 1: Environment variables (loaded from .env file)
source .env

# Option 2: Pass-through from secure storage
export KAFKA_SASL_PASSWORD=$(security find-generic-password -s kafka-password -w)
```

#### Production Deployments

**Kubernetes Secrets**:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: kafka-credentials
type: Opaque
stringData:
  KAFKA_SASL_USERNAME: your-username
  KAFKA_SASL_PASSWORD: your-secure-password
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: xzepr
spec:
  template:
    spec:
      containers:
      - name: xzepr
        env:
        - name: KAFKA_SASL_USERNAME
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: KAFKA_SASL_USERNAME
        - name: KAFKA_SASL_PASSWORD
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: KAFKA_SASL_PASSWORD
```

**HashiCorp Vault**:

```rust
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

async fn load_kafka_credentials() -> Result<(String, String), Error> {
    let client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address("https://vault.example.com")
            .token(std::env::var("VAULT_TOKEN")?)
            .build()?
    )?;

    let secret = client.kv2::read("secret/kafka/credentials").await?;
    let username = secret["username"].as_str().unwrap().to_string();
    let password = secret["password"].as_str().unwrap().to_string();

    Ok((username, password))
}
```

**AWS Secrets Manager**:

```rust
use aws_sdk_secretsmanager::Client;

async fn load_kafka_credentials() -> Result<(String, String), Error> {
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let response = client
        .get_secret_value()
        .secret_id("kafka/credentials")
        .send()
        .await?;

    let secret_string = response.secret_string().unwrap();
    let credentials: serde_json::Value = serde_json::from_str(secret_string)?;

    Ok((
        credentials["username"].as_str().unwrap().to_string(),
        credentials["password"].as_str().unwrap().to_string(),
    ))
}
```

### Credential Rotation

**Rotation frequency**:

- Production: Every 90 days (minimum)
- Staging: Every 180 days
- Development: Every 365 days or on personnel changes

**Rotation procedure**:

1. Generate new credentials in Kafka/Redpanda
2. Update credentials in secret management system
3. Perform rolling restart of XZepr instances
4. Verify all instances connect successfully
5. Remove old credentials from Kafka/Redpanda after grace period (24-48 hours)
6. Update documentation and audit logs

**Automated rotation with Kubernetes**:

```bash
#!/bin/bash
# rotate-kafka-credentials.sh

NEW_PASSWORD=$(openssl rand -base64 32)

# Update in Kafka
kafka-configs --bootstrap-server kafka:9092 \
  --alter --add-config "SCRAM-SHA-256=[password=$NEW_PASSWORD]" \
  --entity-type users --entity-name xzepr-user

# Update Kubernetes secret
kubectl create secret generic kafka-credentials \
  --from-literal=KAFKA_SASL_USERNAME=xzepr-user \
  --from-literal=KAFKA_SASL_PASSWORD="$NEW_PASSWORD" \
  --dry-run=client -o yaml | kubectl apply -f -

# Rolling restart
kubectl rollout restart deployment/xzepr
kubectl rollout status deployment/xzepr
```

## Network Security

### TLS/SSL Configuration

**Always use TLS/SSL in production**. Never use `PLAINTEXT` or `SASL_PLAINTEXT` security protocols.

**Minimum TLS version**: TLS 1.2 (TLS 1.3 recommended)

**Certificate validation**:

```rust
use xzepr::infrastructure::messaging::config::{
    KafkaAuthConfig,
    SecurityProtocol,
    SaslMechanism,
};

// CORRECT - Full certificate validation
let config = KafkaAuthConfig::scram_sha256_ssl(
    username,
    password,
    Some("/path/to/ca-cert.pem".to_string()),
)?;

// WRONG - Disabling certificate validation (never do this in production)
// This is a security risk and should only be used in isolated test environments
```

**Certificate management**:

1. Use certificates from trusted Certificate Authorities (CA)
2. Store CA certificates securely (read-only access, encrypted at rest)
3. Rotate certificates before expiration (monitor with 30-day advance warning)
4. Use separate certificates for each environment
5. Implement certificate pinning for critical environments

**Redpanda TLS configuration example**:

```yaml
redpanda:
  kafka_api_tls:
    enabled: true
    cert_file: /etc/redpanda/certs/server.crt
    key_file: /etc/redpanda/certs/server.key
    truststore_file: /etc/redpanda/certs/ca.crt
    require_client_auth: true
```

### Network Segmentation

**Isolate Kafka/Redpanda traffic**:

1. Deploy Kafka/Redpanda in private network/VPC
2. Use network policies to restrict access
3. Only allow connections from authorized services
4. Use firewall rules to block unauthorized access

**Kubernetes Network Policy example**:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: kafka-access
spec:
  podSelector:
    matchLabels:
      app: redpanda
  ingress:
  - from:
    - podSelector:
        matchLabels:
          app: xzepr
    ports:
    - protocol: TCP
      port: 9093
```

### Connection Security

**Client configuration**:

```rust
// Enable all security features
let mut client_config = rdkafka::ClientConfig::new();

// Authentication
auth_config.apply_to_client_config(&mut client_config)?;

// Security settings
client_config.set("security.protocol", "SASL_SSL");
client_config.set("sasl.mechanism", "SCRAM-SHA-256");

// SSL settings
client_config.set("ssl.ca.location", "/path/to/ca.pem");
client_config.set("ssl.cipher.suites", "TLS_AES_256_GCM_SHA384");
client_config.set("ssl.endpoint.identification.algorithm", "https");

// Connection limits
client_config.set("connections.max.idle.ms", "540000");
client_config.set("socket.connection.setup.timeout.ms", "30000");
```

## Access Control

### Kafka ACLs

**Principle of least privilege**: Grant only the minimum permissions required.

**Producer permissions** (XZepr requires these):

```bash
# Allow write to specific topic
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-user \
  --operation Write --topic events

# Allow describe topic (for metadata)
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-user \
  --operation Describe --topic events

# Allow create topic (if dynamic topic creation is enabled)
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-user \
  --operation Create --cluster
```

**Consumer permissions** (if XZepr consumes events):

```bash
# Allow read from topic
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-consumer \
  --operation Read --topic events

# Allow read consumer group
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-consumer \
  --operation Read --group xzepr-consumer-group
```

**Review ACLs regularly**:

```bash
# List all ACLs
kafka-acls --bootstrap-server kafka:9092 --list

# Remove unnecessary ACLs
kafka-acls --bootstrap-server kafka:9092 \
  --remove --allow-principal User:old-user \
  --operation All --topic '*'
```

### User Management

**Separate users for different purposes**:

- `xzepr-producer`: Write events only
- `xzepr-consumer`: Read events only
- `xzepr-admin`: Topic management (used by operators, not application)

**User lifecycle**:

1. Create users with strong passwords
2. Grant minimum required permissions
3. Document user purpose and ownership
4. Review permissions quarterly
5. Remove users when no longer needed

## Monitoring and Auditing

### Security Monitoring

**Monitor authentication failures**:

```rust
use tracing::{warn, error};

// Log authentication attempts (without credentials)
match KafkaEventPublisher::with_auth(&brokers, &topic, auth_config.as_ref()) {
    Ok(publisher) => {
        tracing::info!(
            "Kafka authentication successful",
            security_protocol = ?auth_config.security_protocol,
            username = ?auth_config.sasl_config.as_ref().map(|s| &s.username),
        );
    }
    Err(e) => {
        tracing::error!(
            "Kafka authentication failed",
            error = ?e,
            security_protocol = ?auth_config.security_protocol,
            // Never log the password
        );
    }
}
```

**Metrics to track**:

- Authentication success/failure rate
- Connection attempts by source IP
- Certificate expiration dates
- ACL violations
- Unusual traffic patterns

**Alerting rules**:

1. Multiple authentication failures from same source
2. Certificate expiring within 30 days
3. ACL violations
4. Unexpected network connections
5. Configuration changes

### Audit Logging

**Enable Kafka audit logging**:

```yaml
# Redpanda configuration
redpanda:
  audit_enabled: true
  audit_log_topics:
    - audit-events
  audit_log_num_partitions: 12
  audit_log_replication_factor: 3
```

**Log security events in XZepr**:

```rust
use tracing::info;

// Log credential rotation
info!(
    "Kafka credentials rotated",
    username = username,
    timestamp = chrono::Utc::now().to_rfc3339(),
);

// Log configuration changes
info!(
    "Kafka security protocol changed",
    old_protocol = old_protocol,
    new_protocol = new_protocol,
    changed_by = operator_id,
);
```

**Audit log retention**:

- Production: 1 year minimum (or per regulatory requirements)
- Staging: 90 days
- Development: 30 days

## Incident Response

### Security Incident Types

1. Credential compromise
2. Unauthorized access
3. Certificate expiration
4. ACL misconfiguration
5. Network breach

### Incident Response Procedures

#### Credential Compromise

**Immediate actions** (within 1 hour):

1. Revoke compromised credentials immediately
2. Generate new credentials
3. Update all services with new credentials
4. Review access logs for unauthorized activity
5. Document incident timeline

**Follow-up actions** (within 24 hours):

1. Investigate root cause
2. Review all credentials and rotate if necessary
3. Update security procedures
4. Conduct security training
5. Report to security team

**Example revocation**:

```bash
# Revoke credentials immediately
kafka-configs --bootstrap-server kafka:9092 \
  --alter --delete-config "SCRAM-SHA-256" \
  --entity-type users --entity-name compromised-user

# Create new user
kafka-configs --bootstrap-server kafka:9092 \
  --alter --add-config "SCRAM-SHA-256=[password=$(openssl rand -base64 32)]" \
  --entity-type users --entity-name new-user

# Update ACLs
kafka-acls --bootstrap-server kafka:9092 \
  --remove --allow-principal User:compromised-user --force
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:new-user \
  --operation Write --topic events
```

#### Unauthorized Access

1. Identify source of unauthorized access
2. Block IP address or network
3. Review and tighten ACLs
4. Audit all recent access
5. Investigate potential data breach

#### Certificate Expiration

**Prevention** (automated monitoring):

```bash
#!/bin/bash
# check-cert-expiration.sh

CERT_FILE="/path/to/ca-cert.pem"
EXPIRY_DATE=$(openssl x509 -enddate -noout -in "$CERT_FILE" | cut -d= -f2)
EXPIRY_EPOCH=$(date -d "$EXPIRY_DATE" +%s)
NOW_EPOCH=$(date +%s)
DAYS_UNTIL_EXPIRY=$(( ($EXPIRY_EPOCH - $NOW_EPOCH) / 86400 ))

if [ $DAYS_UNTIL_EXPIRY -lt 30 ]; then
    echo "WARNING: Certificate expires in $DAYS_UNTIL_EXPIRY days"
    # Send alert
fi
```

**Renewal procedure**:

1. Generate new certificate from CA
2. Deploy new certificate to all nodes
3. Update CA bundle if necessary
4. Perform rolling restart
5. Verify all connections successful
6. Remove old certificate after grace period

### Communication Plan

**Internal notification**:

- Security team: Immediately
- Engineering team: Within 1 hour
- Management: Within 4 hours

**External notification** (if data breach):

- Customers: Per regulatory requirements
- Regulators: Per legal requirements
- Public disclosure: Per company policy

## Security Checklist

Use this checklist for security audits and compliance reviews:

### Authentication

- [ ] SASL/SCRAM authentication enabled
- [ ] Strong passwords used (minimum 32 characters)
- [ ] Credentials stored in secure secret management system
- [ ] No hardcoded credentials in source code
- [ ] Credentials not committed to version control
- [ ] Debug implementation redacts sensitive data

### Network Security

- [ ] TLS/SSL enabled (SASL_SSL security protocol)
- [ ] Valid certificates from trusted CA
- [ ] Certificate validation enabled
- [ ] TLS 1.2 or higher enforced
- [ ] Network segmentation implemented
- [ ] Firewall rules configured

### Access Control

- [ ] ACLs configured with least privilege
- [ ] Separate users for different purposes
- [ ] ACLs reviewed quarterly
- [ ] Unused users removed

### Monitoring

- [ ] Authentication failures logged
- [ ] Security metrics tracked
- [ ] Alerts configured for security events
- [ ] Audit logging enabled
- [ ] Logs retained per policy

### Operations

- [ ] Credential rotation schedule defined
- [ ] Certificate expiration monitoring active
- [ ] Incident response procedures documented
- [ ] Security training completed
- [ ] Regular security audits scheduled

### Compliance

- [ ] Regulatory requirements met
- [ ] Security policies documented
- [ ] Access reviews completed
- [ ] Data retention policies implemented
- [ ] Third-party security assessments completed

## Common Security Pitfalls

### Pitfall 1: Logging Sensitive Data

**WRONG**:

```rust
tracing::info!("Connecting to Kafka with password: {}", password);
```

**CORRECT**:

```rust
tracing::info!("Connecting to Kafka", username = username);
// Password is never logged
```

### Pitfall 2: Disabling Certificate Validation

**WRONG**:

```rust
client_config.set("enable.ssl.certificate.verification", "false");
```

**CORRECT**:

```rust
client_config.set("ssl.ca.location", "/path/to/ca-cert.pem");
// Certificate validation enabled by default
```

### Pitfall 3: Using Weak Security Protocols

**WRONG**:

```rust
let config = KafkaAuthConfig::new(
    SecurityProtocol::Plaintext, // Unencrypted
    None,
    None,
)?;
```

**CORRECT**:

```rust
let config = KafkaAuthConfig::new(
    SecurityProtocol::SaslSsl, // Encrypted and authenticated
    Some(sasl_config),
    Some(ssl_config),
)?;
```

### Pitfall 4: Committing Credentials

**WRONG**:

```bash
# .env file committed to git
KAFKA_SASL_PASSWORD=my-secret-password
```

**CORRECT**:

```bash
# .gitignore
.env
*.env
secrets/

# .env.example (template without real credentials)
KAFKA_SASL_USERNAME=your-username-here
KAFKA_SASL_PASSWORD=your-password-here
```

### Pitfall 5: Insufficient Error Handling

**WRONG**:

```rust
let config = KafkaAuthConfig::from_env().unwrap();
```

**CORRECT**:

```rust
let config = KafkaAuthConfig::from_env()
    .map_err(|e| {
        tracing::error!("Failed to load Kafka config", error = ?e);
        e
    })?;
```

## Resources

### Internal Documentation

- Configuration Guide: `docs/how_to/configure_kafka_authentication.md`
- Implementation Plan: `docs/explanations/kafka_sasl_scram_authentication_plan.md`
- API Reference: `docs/reference/api_specification.md`

### External Documentation

- Kafka Security: https://kafka.apache.org/documentation/#security
- Redpanda Security: https://docs.redpanda.com/docs/security/
- SASL/SCRAM RFC: https://tools.ietf.org/html/rfc5802
- TLS Best Practices: https://wiki.mozilla.org/Security/Server_Side_TLS

### Security Standards

- NIST Cybersecurity Framework
- CIS Kafka Benchmark
- OWASP Secure Coding Practices
- PCI DSS (if applicable)
- GDPR (if applicable)

## Conclusion

Security is an ongoing process, not a one-time configuration. Regularly review and update security practices, monitor for threats, and respond quickly to incidents. Following these best practices ensures XZepr operates securely with Kafka/Redpanda in production environments.
