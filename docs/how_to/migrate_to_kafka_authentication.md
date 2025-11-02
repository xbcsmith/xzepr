# Migrate to Kafka Authentication

## Overview

This guide provides step-by-step instructions for migrating from unauthenticated Kafka connections to SASL/SCRAM authenticated connections. Following this guide ensures a safe, validated migration with minimal downtime and clear rollback procedures.

## Target Audience

- DevOps engineers deploying XZepr
- System administrators managing Kafka/Redpanda infrastructure
- Platform engineers responsible for security compliance

## Prerequisites

Before beginning migration, ensure you have:

- XZepr version with authentication support installed
- Access to Kafka/Redpanda admin credentials
- Ability to create SASL users in Kafka/Redpanda
- Access to secret management system (Kubernetes Secrets, Vault, or AWS Secrets Manager)
- Staging environment for testing
- Rollback plan approved
- Maintenance window scheduled (if required)

## Pre-Migration Checklist

### Environment Preparation

- [ ] Verify Kafka/Redpanda version supports SASL/SCRAM
- [ ] Verify Kafka/Redpanda cluster has SSL/TLS enabled
- [ ] Confirm certificate chain is valid and not expired
- [ ] Obtain CA certificate for SSL validation
- [ ] Test connectivity to Kafka/Redpanda from XZepr hosts
- [ ] Document current configuration
- [ ] Back up current configuration files

### Credentials Preparation

- [ ] Generate strong passwords (minimum 32 characters, random)
- [ ] Create SASL users in Kafka/Redpanda
- [ ] Configure Kafka ACLs for new users
- [ ] Store credentials in secret management system
- [ ] Verify credentials work with test connection
- [ ] Document credential rotation schedule

### Infrastructure Preparation

- [ ] Set up monitoring and alerting for authentication events
- [ ] Configure log aggregation for security events
- [ ] Prepare rollback scripts
- [ ] Create deployment runbook
- [ ] Schedule maintenance window (if needed)
- [ ] Notify stakeholders of migration timeline

### Testing Preparation

- [ ] Test authentication in staging environment
- [ ] Verify application functionality with authentication
- [ ] Test credential rotation procedures
- [ ] Validate rollback procedures
- [ ] Document test results

## Migration Procedure

### Step 1: Create SASL User in Kafka/Redpanda

Create a dedicated user for XZepr in Kafka/Redpanda:

```bash
# For Kafka
kafka-configs --bootstrap-server kafka:9092 \
  --alter --add-config "SCRAM-SHA-256=[password=$(openssl rand -base64 32)]" \
  --entity-type users --entity-name xzepr-producer

# For Redpanda
rpk acl user create xzepr-producer \
  --password "$(openssl rand -base64 32)" \
  --mechanism SCRAM-SHA-256
```

Save the generated password securely immediately.

### Step 2: Configure Kafka ACLs

Grant minimum required permissions to the XZepr user:

```bash
# Allow write to events topic
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-producer \
  --operation Write --topic events

# Allow describe topic
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-producer \
  --operation Describe --topic events

# If dynamic topic creation is needed
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-producer \
  --operation Create --cluster
```

Verify ACLs:

```bash
kafka-acls --bootstrap-server kafka:9092 \
  --list --principal User:xzepr-producer
```

### Step 3: Store Credentials in Secret Management

Choose your secret management system:

#### Option A: Kubernetes Secrets

```bash
kubectl create secret generic kafka-credentials \
  --namespace xzepr \
  --from-literal=KAFKA_SECURITY_PROTOCOL=SASL_SSL \
  --from-literal=KAFKA_SASL_MECHANISM=SCRAM-SHA-256 \
  --from-literal=KAFKA_SASL_USERNAME=xzepr-producer \
  --from-literal=KAFKA_SASL_PASSWORD="<generated-password>" \
  --from-literal=KAFKA_SSL_CA_LOCATION=/etc/kafka/certs/ca-cert.pem
```

#### Option B: HashiCorp Vault

```bash
vault kv put secret/xzepr/kafka \
  security_protocol=SASL_SSL \
  sasl_mechanism=SCRAM-SHA-256 \
  sasl_username=xzepr-producer \
  sasl_password="<generated-password>" \
  ssl_ca_location=/etc/kafka/certs/ca-cert.pem
```

#### Option C: AWS Secrets Manager

```bash
aws secretsmanager create-secret \
  --name xzepr/kafka/credentials \
  --secret-string '{
    "security_protocol": "SASL_SSL",
    "sasl_mechanism": "SCRAM-SHA-256",
    "sasl_username": "xzepr-producer",
    "sasl_password": "<generated-password>",
    "ssl_ca_location": "/etc/kafka/certs/ca-cert.pem"
  }'
```

### Step 4: Update XZepr Configuration

#### For Environment Variables

Update your deployment configuration to inject environment variables:

**Kubernetes Deployment**:

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
        env:
        - name: KAFKA_SECURITY_PROTOCOL
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: KAFKA_SECURITY_PROTOCOL
        - name: KAFKA_SASL_MECHANISM
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: KAFKA_SASL_MECHANISM
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
        - name: KAFKA_SSL_CA_LOCATION
          valueFrom:
            secretKeyRef:
              name: kafka-credentials
              key: KAFKA_SSL_CA_LOCATION
        volumeMounts:
        - name: kafka-certs
          mountPath: /etc/kafka/certs
          readOnly: true
      volumes:
      - name: kafka-certs
        secret:
          secretName: kafka-ca-cert
```

**Docker Compose**:

```yaml
services:
  xzepr:
    image: xzepr:latest
    environment:
      KAFKA_SECURITY_PROTOCOL: SASL_SSL
      KAFKA_SASL_MECHANISM: SCRAM-SHA-256
      KAFKA_SASL_USERNAME: xzepr-producer
      KAFKA_SASL_PASSWORD: ${KAFKA_SASL_PASSWORD}
      KAFKA_SSL_CA_LOCATION: /etc/kafka/certs/ca-cert.pem
    volumes:
      - ./certs/ca-cert.pem:/etc/kafka/certs/ca-cert.pem:ro
```

#### For YAML Configuration

Update your `config.yaml`:

```yaml
kafka:
  brokers:
    - kafka-1:9093
    - kafka-2:9093
    - kafka-3:9093
  topic: events
  auth:
    security_protocol: SASL_SSL
    sasl:
      mechanism: SCRAM-SHA-256
      username: xzepr-producer
      password: ${KAFKA_SASL_PASSWORD}
    ssl:
      ca_location: /etc/kafka/certs/ca-cert.pem
```

### Step 5: Deploy Updated Configuration

#### Blue-Green Deployment (Recommended)

1. Deploy new version with authentication to green environment
2. Verify connectivity and functionality
3. Switch traffic to green environment
4. Monitor for issues
5. Decommission blue environment after validation period

```bash
# Deploy to green environment
kubectl apply -f deployment-green.yaml

# Verify health
kubectl get pods -l app=xzepr,environment=green
kubectl logs -l app=xzepr,environment=green

# Switch traffic
kubectl patch service xzepr -p '{"spec":{"selector":{"environment":"green"}}}'

# Monitor
kubectl logs -f -l app=xzepr,environment=green
```

#### Rolling Update

1. Update deployment configuration
2. Apply changes (Kubernetes will perform rolling update)
3. Monitor pod rollout
4. Verify all pods are healthy

```bash
# Apply updated configuration
kubectl apply -f deployment.yaml

# Monitor rollout
kubectl rollout status deployment/xzepr

# Verify pods are running
kubectl get pods -l app=xzepr

# Check logs for authentication success
kubectl logs -l app=xzepr --tail=100 | grep -i "kafka.*auth"
```

#### Canary Deployment

1. Deploy small percentage with authentication
2. Monitor metrics and errors
3. Gradually increase percentage
4. Complete rollout if successful

```bash
# Deploy 10% canary
kubectl apply -f deployment-canary.yaml

# Monitor canary metrics
kubectl logs -l app=xzepr,version=canary

# If successful, increase to 50%
kubectl scale deployment xzepr-canary --replicas=5

# If successful, complete rollout
kubectl apply -f deployment.yaml
```

### Step 6: Verify Migration

#### Check Application Health

```bash
# Kubernetes
kubectl get pods -l app=xzepr
kubectl describe pod <pod-name>

# Docker
docker ps | grep xzepr
docker logs <container-id>
```

#### Verify Authentication

```bash
# Check logs for successful authentication
kubectl logs -l app=xzepr | grep -i "kafka.*authentication.*success"

# Check for authentication errors
kubectl logs -l app=xzepr | grep -i "kafka.*authentication.*fail"
```

#### Test Event Publishing

```bash
# Send test event
curl -X POST http://xzepr:8080/api/events \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "test.migration",
    "payload": {"status": "testing"}
  }'

# Verify event in Kafka
kafka-console-consumer --bootstrap-server kafka:9093 \
  --topic events \
  --consumer.config /etc/kafka/consumer.properties \
  --from-beginning \
  --max-messages 1
```

#### Check Monitoring

- Verify metrics are being collected
- Check authentication success rate (should be 100%)
- Verify no authentication errors
- Check event publish rate is normal
- Verify latency is within acceptable range

### Step 7: Post-Migration Validation

- [ ] All XZepr instances connected successfully
- [ ] Authentication metrics showing 100% success rate
- [ ] Event publishing working normally
- [ ] No errors in application logs
- [ ] No errors in Kafka/Redpanda logs
- [ ] Monitoring and alerting working
- [ ] Performance metrics within acceptable range
- [ ] Security audit logs capturing authentication events

## Rollback Procedures

If issues occur during migration, follow these rollback procedures:

### Immediate Rollback (Within Maintenance Window)

If issues are detected immediately:

```bash
# Kubernetes - rollback to previous deployment
kubectl rollout undo deployment/xzepr

# Verify rollback
kubectl rollout status deployment/xzepr
kubectl get pods -l app=xzepr

# Check logs
kubectl logs -l app=xzepr --tail=100
```

### Partial Rollback (After Maintenance Window)

If issues are detected after initial deployment:

1. Scale down new deployment
2. Scale up previous deployment
3. Verify functionality
4. Investigate issues

```bash
# Scale down authenticated version
kubectl scale deployment xzepr-auth --replicas=0

# Scale up unauthenticated version
kubectl scale deployment xzepr-noauth --replicas=3

# Verify
kubectl get pods -l app=xzepr
```

### Configuration Rollback

If only configuration needs to be reverted:

```bash
# Kubernetes - update secret to remove authentication
kubectl create secret generic kafka-credentials \
  --namespace xzepr \
  --from-literal=KAFKA_SECURITY_PROTOCOL=PLAINTEXT \
  --dry-run=client -o yaml | kubectl apply -f -

# Restart pods to pick up new configuration
kubectl rollout restart deployment/xzepr
```

### Emergency Rollback

In case of critical failure:

1. Stop all XZepr instances
2. Revert configuration to pre-migration state
3. Remove authentication requirement from Kafka ACLs (temporary)
4. Restart XZepr with unauthenticated configuration
5. Investigate root cause
6. Plan remediation

```bash
# Emergency: Allow unauthenticated access temporarily
kafka-configs --bootstrap-server kafka:9092 \
  --alter --add-config "allow.everyone.if.no.acl.found=true" \
  --entity-type brokers --entity-default

# Rollback deployment
kubectl rollout undo deployment/xzepr

# After issue resolved, re-enable authentication requirement
kafka-configs --bootstrap-server kafka:9092 \
  --alter --delete-config "allow.everyone.if.no.acl.found" \
  --entity-type brokers --entity-default
```

## Common Migration Issues

### Issue 1: Authentication Failure

**Symptoms**: Logs show "Authentication failed" errors

**Causes**:
- Incorrect username or password
- SASL mechanism mismatch
- User not created in Kafka

**Resolution**:

```bash
# Verify user exists
kafka-configs --bootstrap-server kafka:9092 \
  --describe --entity-type users --entity-name xzepr-producer

# Test credentials
kafka-console-producer --bootstrap-server kafka:9093 \
  --topic test-topic \
  --producer.config /tmp/producer.properties

# Recreate user if needed
kafka-configs --bootstrap-server kafka:9092 \
  --alter --add-config "SCRAM-SHA-256=[password=<new-password>]" \
  --entity-type users --entity-name xzepr-producer
```

### Issue 2: SSL Certificate Validation Failure

**Symptoms**: Logs show "SSL certificate validation failed"

**Causes**:
- CA certificate not found
- CA certificate expired
- CA certificate incorrect
- Hostname verification failed

**Resolution**:

```bash
# Verify certificate file exists
ls -la /etc/kafka/certs/ca-cert.pem

# Verify certificate is valid
openssl x509 -in /etc/kafka/certs/ca-cert.pem -noout -dates

# Test SSL connection
openssl s_client -connect kafka:9093 -CAfile /etc/kafka/certs/ca-cert.pem

# Verify certificate chain
openssl verify -CAfile /etc/kafka/certs/ca-cert.pem /etc/kafka/certs/server-cert.pem
```

### Issue 3: ACL Permission Denied

**Symptoms**: Logs show "Not authorized to access topics"

**Causes**:
- ACLs not configured
- ACLs configured for wrong principal
- Insufficient permissions

**Resolution**:

```bash
# List current ACLs
kafka-acls --bootstrap-server kafka:9092 \
  --list --principal User:xzepr-producer

# Add missing permissions
kafka-acls --bootstrap-server kafka:9092 \
  --add --allow-principal User:xzepr-producer \
  --operation Write --topic events

# Verify permissions
kafka-acls --bootstrap-server kafka:9092 \
  --list --topic events
```

### Issue 4: Connection Timeout

**Symptoms**: Logs show "Connection timeout" or "Unable to connect"

**Causes**:
- Network connectivity issues
- Firewall blocking port
- Wrong broker address
- Wrong port number

**Resolution**:

```bash
# Test network connectivity
telnet kafka 9093

# Test with netcat
nc -zv kafka 9093

# Verify DNS resolution
nslookup kafka

# Check firewall rules
iptables -L -n | grep 9093
```

### Issue 5: Environment Variables Not Set

**Symptoms**: XZepr uses unauthenticated connection despite configuration

**Causes**:
- Environment variables not injected
- Secret not mounted
- Wrong secret name

**Resolution**:

```bash
# Verify environment variables in pod
kubectl exec <pod-name> -- env | grep KAFKA

# Verify secret exists
kubectl get secret kafka-credentials

# Verify secret content
kubectl get secret kafka-credentials -o yaml

# Describe pod to check volume mounts
kubectl describe pod <pod-name>
```

## Staged Rollout Strategy

For large deployments, use a staged rollout:

### Stage 1: Development Environment (Week 1)

- Deploy authentication to dev environment
- Test all functionality
- Validate monitoring and alerting
- Document any issues
- Refine procedures

### Stage 2: Staging Environment (Week 2)

- Deploy authentication to staging
- Run full test suite
- Perform load testing
- Test credential rotation
- Validate rollback procedures
- Get sign-off from QA team

### Stage 3: Production Canary (Week 3)

- Deploy to 10% of production instances
- Monitor for 24 hours
- Check metrics and logs
- If successful, increase to 25%
- Monitor for another 24 hours

### Stage 4: Production Rollout (Week 4)

- Increase to 50% of production instances
- Monitor for 24 hours
- If successful, complete rollout to 100%
- Monitor for 72 hours
- Remove old unauthenticated configuration

### Stage 5: Cleanup (Week 5)

- Remove temporary ACLs
- Clean up old configurations
- Update documentation
- Conduct post-migration review
- Schedule credential rotation

## Verification Steps

After migration is complete, verify:

### Functional Verification

- [ ] All XZepr instances running
- [ ] All instances authenticated successfully
- [ ] Events publishing to Kafka
- [ ] Event consumers receiving events
- [ ] API endpoints responding normally
- [ ] Health checks passing

### Security Verification

- [ ] Authentication metrics showing 100% success
- [ ] No unauthenticated connections
- [ ] ACLs enforced correctly
- [ ] SSL/TLS encryption active
- [ ] Credentials stored securely
- [ ] Audit logs capturing authentication events

### Performance Verification

- [ ] Latency within acceptable range
- [ ] Throughput matches pre-migration levels
- [ ] No connection pooling issues
- [ ] Memory usage normal
- [ ] CPU usage normal
- [ ] No authentication-related bottlenecks

### Monitoring Verification

- [ ] Authentication metrics collected
- [ ] Alerts configured and tested
- [ ] Dashboards updated
- [ ] Log aggregation working
- [ ] Audit logs flowing
- [ ] Certificate expiration monitoring active

## Post-Migration Tasks

After successful migration:

1. **Documentation**
   - Update runbooks with new authentication procedures
   - Document credential rotation schedule
   - Update disaster recovery procedures
   - Update troubleshooting guides

2. **Monitoring**
   - Create authentication metrics dashboard
   - Set up alerting for authentication failures
   - Configure certificate expiration alerts
   - Monitor ACL violations

3. **Security**
   - Schedule first credential rotation
   - Review and tighten ACLs
   - Conduct security audit
   - Update security documentation

4. **Training**
   - Train operators on new procedures
   - Document common issues and resolutions
   - Create incident response procedures
   - Schedule security awareness training

5. **Cleanup**
   - Remove old unauthenticated configuration
   - Clean up temporary files
   - Archive migration documentation
   - Conduct post-migration review

## References

### Internal Documentation

- Configuration Guide: `docs/how_to/configure_kafka_authentication.md`
- Security Best Practices: `docs/how_to/kafka_security_best_practices.md`
- Security Checklist: `docs/reference/kafka_security_checklist.md`
- Implementation Plan: `docs/explanations/kafka_sasl_scram_authentication_plan.md`

### External Resources

- Kafka Security: https://kafka.apache.org/documentation/#security
- Redpanda Security: https://docs.redpanda.com/docs/security/
- Kubernetes Secrets: https://kubernetes.io/docs/concepts/configuration/secret/
- HashiCorp Vault: https://www.vaultproject.io/docs

## Support

If you encounter issues during migration:

1. Check the Common Migration Issues section above
2. Review application logs for error details
3. Consult the Security Best Practices guide
4. Contact the platform team for assistance

## Conclusion

Following this migration guide ensures a safe, validated transition to authenticated Kafka connections. The staged approach, comprehensive testing, and clear rollback procedures minimize risk and ensure business continuity.

Remember to document your specific migration experience and update this guide with any lessons learned for future migrations.
