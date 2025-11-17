# OPA Bundle Server Setup Guide

## Overview

This guide explains how to set up and configure an OPA (Open Policy Agent) bundle server for XZepr's authorization policies. The bundle server enables centralized policy management, versioning, and distribution to OPA instances.

## Prerequisites

- Docker and Docker Compose installed
- Access to XZepr repository
- Basic understanding of OPA and Rego policies
- Web server (nginx, Apache, or similar) for hosting bundles

## What is an OPA Bundle?

An OPA bundle is a compressed archive containing:

- Rego policy files (.rego)
- Policy data files (JSON/YAML)
- Manifest file describing bundle contents
- Signatures for verification (optional)

Bundles enable:

- Centralized policy management
- Version control for policies
- Atomic policy updates
- Policy rollback capability
- Policy signing and verification

## Bundle Structure

The XZepr OPA bundle follows this structure:

```text
xzepr-policies/
├── .manifest
├── policies/
│   ├── authz.rego
│   ├── event_receiver.rego
│   ├── event_receiver_group.rego
│   ├── event.rego
│   └── helpers.rego
└── data/
    ├── roles.json
    └── permissions.json
```

### Manifest File

The `.manifest` file describes the bundle:

```json
{
  "revision": "v1.2.3",
  "roots": ["policies", "data"],
  "metadata": {
    "name": "xzepr-authz-policies",
    "description": "XZepr authorization policies for OPA",
    "organization": "xzepr",
    "created_at": "2024-01-15T10:30:00Z"
  }
}
```

## Step 1: Create Bundle Directory Structure

Create the directory structure for your policies:

```bash
# Create bundle directory
mkdir -p /opt/opa-bundles/xzepr-policies/{policies,data}

# Navigate to bundle directory
cd /opt/opa-bundles/xzepr-policies
```

## Step 2: Copy Policy Files

Copy the Rego policy files from the XZepr repository:

```bash
# Copy all policy files
cp /path/to/xzepr/config/opa/policies/*.rego policies/

# Verify files are copied
ls -la policies/
```

Expected files:

- `authz.rego` - Main authorization logic
- `event_receiver.rego` - Event receiver permissions
- `event_receiver_group.rego` - Group permissions
- `event.rego` - Event permissions
- `helpers.rego` - Utility functions

## Step 3: Create Data Files

Create the data files that define roles and permissions:

```bash
# Create roles.json
cat > data/roles.json <<'EOF'
{
  "roles": {
    "admin": {
      "description": "Full system access",
      "permissions": ["*"]
    },
    "owner": {
      "description": "Resource owner",
      "permissions": [
        "event_receiver:read",
        "event_receiver:update",
        "event_receiver:delete",
        "event:read",
        "event:create",
        "event:delete",
        "group:manage_members"
      ]
    },
    "member": {
      "description": "Group member",
      "permissions": [
        "event_receiver:read",
        "event:read",
        "event:create"
      ]
    },
    "viewer": {
      "description": "Read-only access",
      "permissions": [
        "event_receiver:read",
        "event:read"
      ]
    }
  }
}
EOF

# Create permissions.json
cat > data/permissions.json <<'EOF'
{
  "permissions": {
    "event_receiver:read": {
      "description": "View event receiver details"
    },
    "event_receiver:update": {
      "description": "Modify event receiver configuration"
    },
    "event_receiver:delete": {
      "description": "Delete event receiver"
    },
    "event:read": {
      "description": "View events"
    },
    "event:create": {
      "description": "Create new events"
    },
    "event:delete": {
      "description": "Delete events"
    },
    "group:manage_members": {
      "description": "Add or remove group members"
    }
  }
}
EOF
```

## Step 4: Create Manifest File

Generate the manifest file:

```bash
cat > .manifest <<EOF
{
  "revision": "$(date +%Y%m%d-%H%M%S)",
  "roots": ["policies", "data"],
  "metadata": {
    "name": "xzepr-authz-policies",
    "description": "XZepr authorization policies for OPA",
    "organization": "xzepr",
    "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  }
}
EOF
```

## Step 5: Build the Bundle

Use OPA CLI to build and verify the bundle:

```bash
# Build the bundle
opa build -b . -o xzepr-policies.tar.gz

# Verify bundle structure
tar -tzf xzepr-policies.tar.gz

# Test bundle syntax
opa test policies/*.rego
```

Expected output:

```text
PASS: 15/15
```

## Step 6: Set Up Bundle Server

### Option A: Simple HTTP Server (Development)

For development and testing, use Python's built-in HTTP server:

```bash
# Navigate to bundles directory
cd /opt/opa-bundles

# Start HTTP server on port 8888
python3 -m http.server 8888

# Bundle will be available at:
# http://localhost:8888/xzepr-policies.tar.gz
```

### Option B: Nginx (Production)

For production, use nginx to serve bundles:

```bash
# Install nginx
sudo apt-get update
sudo apt-get install nginx

# Create nginx configuration
sudo tee /etc/nginx/sites-available/opa-bundles <<'EOF'
server {
    listen 8888;
    server_name opa-bundles.example.com;

    root /opt/opa-bundles;

    location / {
        autoindex on;
        add_header Cache-Control "no-cache, no-store, must-revalidate";
        add_header Pragma "no-cache";
        add_header Expires "0";
    }

    location ~ \.tar\.gz$ {
        add_header Content-Type "application/gzip";
        add_header Content-Disposition "attachment";
    }

    access_log /var/log/nginx/opa-bundles-access.log;
    error_log /var/log/nginx/opa-bundles-error.log;
}
EOF

# Enable site
sudo ln -s /etc/nginx/sites-available/opa-bundles /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Restart nginx
sudo systemctl restart nginx
```

### Option C: Docker Container

Run a simple bundle server using Docker:

```bash
# Create Dockerfile
cat > Dockerfile <<'EOF'
FROM nginx:alpine

COPY xzepr-policies.tar.gz /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 8888
EOF

# Create nginx.conf
cat > nginx.conf <<'EOF'
events {
    worker_connections 1024;
}

http {
    server {
        listen 8888;
        root /usr/share/nginx/html;

        location / {
            autoindex on;
        }
    }
}
EOF

# Build and run
docker build -t xzepr-opa-bundle-server .
docker run -d -p 8888:8888 --name opa-bundles xzepr-opa-bundle-server
```

## Step 7: Configure OPA to Use Bundle Server

Update the XZepr configuration to point to the bundle server:

```yaml
opa:
  enabled: true
  url: "http://localhost:8181"
  timeout_seconds: 5
  policy_path: "/v1/data/xzepr/authz/allow"
  bundle_url: "http://localhost:8888/xzepr-policies.tar.gz"
  cache_ttl_seconds: 300
```

## Step 8: Configure OPA Bundle Loading

Update the OPA configuration to download bundles:

```yaml
# config/opa/config.yaml
services:
  bundle-server:
    url: http://localhost:8888

bundles:
  xzepr-policies:
    service: bundle-server
    resource: /xzepr-policies.tar.gz
    polling:
      min_delay_seconds: 60
      max_delay_seconds: 120

decision_logs:
  console: true
```

## Step 9: Start OPA with Bundle Configuration

Start OPA with the bundle configuration:

```bash
# Using Docker
docker run -d \
  --name xzepr-opa \
  -p 8181:8181 \
  -v $(pwd)/config/opa/config.yaml:/config.yaml \
  openpolicyagent/opa:latest \
  run --server --config-file=/config.yaml

# Using OPA binary
opa run --server --config-file=config/opa/config.yaml
```

## Step 10: Verify Bundle Loading

Check that OPA loaded the bundle successfully:

```bash
# Check OPA status
curl http://localhost:8181/health

# Check loaded policies
curl http://localhost:8181/v1/policies

# Test authorization query
curl -X POST http://localhost:8181/v1/data/xzepr/authz/allow \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "user": {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "roles": ["owner"]
      },
      "action": "event_receiver:read",
      "resource": {
        "type": "event_receiver",
        "owner_id": "550e8400-e29b-41d4-a716-446655440000"
      }
    }
  }'
```

Expected response:

```json
{
  "result": true
}
```

## Bundle Versioning

### Semantic Versioning

Use semantic versioning for your bundles:

- `v1.0.0` - Initial release
- `v1.1.0` - New features (backward compatible)
- `v1.0.1` - Bug fixes
- `v2.0.0` - Breaking changes

### Version Script

Create a script to automate bundle versioning:

```bash
#!/bin/bash
# build-bundle.sh

VERSION=${1:-"dev"}
BUNDLE_NAME="xzepr-policies-${VERSION}.tar.gz"

# Update manifest with version
cat > .manifest <<EOF
{
  "revision": "${VERSION}",
  "roots": ["policies", "data"],
  "metadata": {
    "name": "xzepr-authz-policies",
    "description": "XZepr authorization policies for OPA",
    "organization": "xzepr",
    "version": "${VERSION}",
    "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  }
}
EOF

# Build bundle
opa build -b . -o "${BUNDLE_NAME}"

echo "Bundle built: ${BUNDLE_NAME}"
```

Usage:

```bash
# Build development bundle
./build-bundle.sh dev

# Build release bundle
./build-bundle.sh v1.2.3
```

## Bundle Signing (Optional)

For production, sign bundles to ensure integrity:

```bash
# Generate signing key
openssl genrsa -out private_key.pem 2048
openssl rsa -in private_key.pem -pubout -out public_key.pem

# Build signed bundle
opa build -b . \
  --signing-key private_key.pem \
  --signing-alg RS256 \
  -o xzepr-policies-signed.tar.gz

# Configure OPA to verify signatures
cat > config.yaml <<EOF
services:
  bundle-server:
    url: http://localhost:8888

bundles:
  xzepr-policies:
    service: bundle-server
    resource: /xzepr-policies-signed.tar.gz
    signing:
      keyid: "xzepr-signing-key"
      scope: "read"
    verification:
      keyid: "xzepr-signing-key"
      publickey: "$(cat public_key.pem | base64 -w0)"
      scope: "read"
EOF
```

## Monitoring Bundle Updates

Monitor OPA bundle updates:

```bash
# Check bundle status
curl http://localhost:8181/v1/status

# Watch OPA logs for bundle updates
docker logs -f xzepr-opa | grep bundle
```

Expected log output:

```text
Bundle loaded and activated successfully. name=xzepr-policies
```

## Troubleshooting

### Bundle Not Loading

Check OPA logs:

```bash
docker logs xzepr-opa
```

Common issues:

1. Bundle server unreachable
2. Invalid bundle format
3. Policy syntax errors
4. Network connectivity issues

### Policy Syntax Errors

Test policies locally:

```bash
# Check syntax
opa check policies/

# Run tests
opa test policies/

# Evaluate specific query
opa eval -d policies/ -i input.json 'data.xzepr.authz.allow'
```

### Bundle Server Not Responding

Check server status:

```bash
# Test bundle download
curl -I http://localhost:8888/xzepr-policies.tar.gz

# Check nginx logs
sudo tail -f /var/log/nginx/opa-bundles-error.log
```

## Production Considerations

### High Availability

Deploy multiple bundle servers behind a load balancer:

```yaml
services:
  bundle-server:
    url: http://bundle-lb.example.com

bundles:
  xzepr-policies:
    service: bundle-server
    resource: /xzepr-policies.tar.gz
    polling:
      min_delay_seconds: 60
      max_delay_seconds: 120
```

### CDN Distribution

Use a CDN for global bundle distribution:

- Upload bundles to S3/GCS/Azure Storage
- Enable CDN (CloudFront, Cloud CDN, Azure CDN)
- Configure OPA to pull from CDN URL

### Automated Deployment

Integrate bundle building into CI/CD:

```yaml
# .github/workflows/opa-bundle.yaml
name: Build OPA Bundle

on:
  push:
    paths:
      - 'config/opa/policies/**'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install OPA
        run: |
          curl -L -o opa https://openpolicyagent.org/downloads/latest/opa_linux_amd64
          chmod +x opa
          sudo mv opa /usr/local/bin/

      - name: Test policies
        run: opa test config/opa/policies/

      - name: Build bundle
        run: |
          cd config/opa/policies
          opa build -b . -o xzepr-policies.tar.gz

      - name: Upload to S3
        run: |
          aws s3 cp xzepr-policies.tar.gz s3://xzepr-opa-bundles/
```

## Related Documentation

- [OPA Policy Development Guide](opa_policy_development.md)
- [OPA Authorization Architecture](../explanation/opa_authorization_architecture.md)
- [Group Membership API Reference](../reference/group_membership_api.md)

## References

- OPA Bundle Documentation: https://www.openpolicyagent.org/docs/latest/management-bundles/
- OPA Bundle API: https://www.openpolicyagent.org/docs/latest/rest-api/#bundles-api
- OPA Signing: https://www.openpolicyagent.org/docs/latest/management-bundles/#signing
