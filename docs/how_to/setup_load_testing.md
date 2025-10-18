# How to Set Up Load Testing

This guide walks you through setting up comprehensive load testing for the XZepr event tracking server using k6, Apache Bench, and other tools.

## Prerequisites

- XZepr server running locally or in staging
- Docker (for k6 and infrastructure)
- Basic understanding of HTTP load testing
- API key for authentication

## Overview

We'll set up:

1. k6 load testing framework
2. Test scenarios (baseline, stress, spike, soak)
3. Performance metrics collection
4. Continuous load testing in CI/CD

## Step 1: Install k6

### Using Docker (Recommended)

```bash
docker pull grafana/k6:latest
```

### Using Package Manager

**macOS:**
```bash
brew install k6
```

**Linux:**
```bash
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

**Windows:**
```powershell
choco install k6
```

## Step 2: Create Test Directory Structure

```bash
mkdir -p tests/load/{scenarios,data,results}
```

Structure:
```text
tests/load/
├── scenarios/
│   ├── baseline.js
│   ├── stress.js
│   ├── spike.js
│   └── soak.js
├── data/
│   ├── test-events.json
│   └── users.json
├── results/
│   └── .gitkeep
└── README.md
```

## Step 3: Create Baseline Test

Create `tests/load/scenarios/baseline.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const eventCreationTime = new Trend('event_creation_duration');

// Test configuration
export let options = {
    stages: [
        { duration: '1m', target: 10 },   // Ramp up to 10 users
        { duration: '3m', target: 10 },   // Stay at 10 users
        { duration: '1m', target: 0 },    // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<500', 'p(99)<1000'],
        http_req_failed: ['rate<0.01'],
        errors: ['rate<0.05'],
        event_creation_duration: ['p(95)<300'],
    },
};

// Environment variables
const BASE_URL = __ENV.BASE_URL || 'http://localhost:8042';
const API_KEY = __ENV.API_KEY;

if (!API_KEY) {
    throw new Error('API_KEY environment variable is required');
}

// Test data
const eventNames = ['build', 'deploy', 'test', 'release', 'rollback'];
const platforms = ['linux-x86_64', 'darwin-arm64', 'windows-x86_64'];
const packages = ['app-server', 'web-ui', 'api-gateway', 'worker'];

function randomElement(array) {
    return array[Math.floor(Math.random() * array.length)];
}

function generateEvent() {
    return {
        name: randomElement(eventNames),
        version: `1.${Math.floor(Math.random() * 100)}.0`,
        release: `release-${Math.floor(Math.random() * 1000)}`,
        platform_id: randomElement(platforms),
        package: randomElement(packages),
        success: Math.random() > 0.1, // 90% success rate
        description: `Test event from k6 load test`,
    };
}

export default function() {
    // Test 1: Health check
    {
        let res = http.get(`${BASE_URL}/health`);
        check(res, {
            'health check status is 200': (r) => r.status === 200,
        });
    }

    // Test 2: Create event
    {
        let event = generateEvent();
        let payload = JSON.stringify(event);

        let start = Date.now();
        let res = http.post(`${BASE_URL}/api/v1/events`, payload, {
            headers: {
                'Content-Type': 'application/json',
                'X-API-Key': API_KEY,
            },
        });
        let duration = Date.now() - start;

        eventCreationTime.add(duration);

        let success = check(res, {
            'event created': (r) => r.status === 201,
            'response has id': (r) => {
                try {
                    return JSON.parse(r.body).id !== undefined;
                } catch {
                    return false;
                }
            },
        });

        errorRate.add(!success);

        if (!success) {
            console.error(`Failed to create event: ${res.status} - ${res.body}`);
        }
    }

    // Test 3: Query events (if we have an ID)
    // Add your query tests here

    sleep(1);
}

export function handleSummary(data) {
    return {
        'results/baseline-summary.json': JSON.stringify(data, null, 2),
        stdout: textSummary(data, { indent: ' ', enableColors: true }),
    };
}
```

## Step 4: Create Stress Test

Create `tests/load/scenarios/stress.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export let options = {
    stages: [
        { duration: '2m', target: 50 },    // Ramp to 50
        { duration: '2m', target: 100 },   // Ramp to 100
        { duration: '2m', target: 200 },   // Ramp to 200
        { duration: '2m', target: 300 },   // Ramp to 300
        { duration: '2m', target: 400 },   // Ramp to 400
        { duration: '5m', target: 400 },   // Stay at 400
        { duration: '2m', target: 0 },     // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<2000'],  // More lenient for stress test
        http_req_failed: ['rate<0.05'],
        errors: ['rate<0.1'],
    },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8042';
const API_KEY = __ENV.API_KEY;

export default function() {
    let event = {
        name: 'stress-test-event',
        version: '1.0.0',
        release: 'stress',
        platform_id: 'linux-x86_64',
        package: 'stress-package',
        success: true,
    };

    let res = http.post(`${BASE_URL}/api/v1/events`, JSON.stringify(event), {
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': API_KEY,
        },
    });

    let success = check(res, {
        'status is 2xx or 429 or 503': (r) =>
            (r.status >= 200 && r.status < 300) ||
            r.status === 429 || // Rate limited (expected under stress)
            r.status === 503,   // Service unavailable (expected under stress)
    });

    errorRate.add(!success);

    sleep(0.5);
}
```

## Step 5: Create Spike Test

Create `tests/load/scenarios/spike.js`:

```javascript
import http from 'k6/http';
import { check } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export let options = {
    stages: [
        { duration: '10s', target: 10 },   // Normal load
        { duration: '30s', target: 500 },  // Sudden spike
        { duration: '3m', target: 500 },   // Sustained spike
        { duration: '30s', target: 10 },   // Drop back
        { duration: '2m', target: 10 },    // Recovery
        { duration: '10s', target: 0 },    // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<3000'],
        http_req_failed: ['rate<0.1'],
    },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8042';
const API_KEY = __ENV.API_KEY;

export default function() {
    let event = {
        name: 'spike-test',
        version: '1.0.0',
        release: 'spike',
        platform_id: 'linux-x86_64',
        package: 'spike-package',
        success: true,
    };

    let res = http.post(`${BASE_URL}/api/v1/events`, JSON.stringify(event), {
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': API_KEY,
        },
        timeout: '10s',
    });

    check(res, {
        'request completed': (r) => r.status !== 0,
    });
}
```

## Step 6: Create Soak Test

Create `tests/load/scenarios/soak.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Counter } from 'k6/metrics';

const errorRate = new Rate('errors');
const errorCounter = new Counter('error_count');

export let options = {
    stages: [
        { duration: '5m', target: 50 },    // Ramp up
        { duration: '4h', target: 50 },    // Soak (4 hours)
        { duration: '5m', target: 0 },     // Ramp down
    ],
    thresholds: {
        http_req_duration: ['p(95)<1000'],
        http_req_failed: ['rate<0.01'],
        errors: ['rate<0.02'],
    },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8042';
const API_KEY = __ENV.API_KEY;

export default function() {
    let event = {
        name: 'soak-test',
        version: '1.0.0',
        release: 'soak',
        platform_id: 'linux-x86_64',
        package: 'soak-package',
        success: true,
        timestamp: new Date().toISOString(),
    };

    let res = http.post(`${BASE_URL}/api/v1/events`, JSON.stringify(event), {
        headers: {
            'Content-Type': 'application/json',
            'X-API-Key': API_KEY,
        },
    });

    let success = check(res, {
        'event created': (r) => r.status === 201,
    });

    if (!success) {
        errorCounter.add(1);
        console.error(`Error at ${new Date().toISOString()}: ${res.status}`);
    }

    errorRate.add(!success);

    sleep(2);
}
```

## Step 7: Run Tests

### Run Baseline Test

```bash
# Set environment variables
export BASE_URL="http://localhost:8042"
export API_KEY="your-api-key-here"

# Run test
k6 run tests/load/scenarios/baseline.js
```

### Run with Docker

```bash
docker run --rm -i \
  -e BASE_URL="http://host.docker.internal:8042" \
  -e API_KEY="your-api-key-here" \
  -v $(pwd)/tests/load:/scripts \
  grafana/k6:latest run /scripts/scenarios/baseline.js
```

### Run All Tests

```bash
#!/bin/bash
# tests/load/run-all.sh

export BASE_URL="${BASE_URL:-http://localhost:8042}"
export API_KEY="${API_KEY:?API_KEY is required}"

echo "Running baseline test..."
k6 run tests/load/scenarios/baseline.js

echo "Running stress test..."
k6 run tests/load/scenarios/stress.js

echo "Running spike test..."
k6 run tests/load/scenarios/spike.js

# Soak test takes 4+ hours
# echo "Running soak test..."
# k6 run tests/load/scenarios/soak.js
```

## Step 8: Analyze Results

k6 provides several output formats:

### JSON Output

```bash
k6 run --out json=results/baseline.json tests/load/scenarios/baseline.js
```

### InfluxDB + Grafana

Start InfluxDB and Grafana:

```yaml
# docker-compose.monitoring.yaml
version: '3.8'

services:
  influxdb:
    image: influxdb:2.7
    ports:
      - "8086:8086"
    environment:
      - DOCKER_INFLUXDB_INIT_MODE=setup
      - DOCKER_INFLUXDB_INIT_USERNAME=admin
      - DOCKER_INFLUXDB_INIT_PASSWORD=password123
      - DOCKER_INFLUXDB_INIT_ORG=xzepr
      - DOCKER_INFLUXDB_INIT_BUCKET=k6
    volumes:
      - influxdb-data:/var/lib/influxdb2

  grafana:
    image: grafana/grafana:10.0.0
    ports:
      - "3000:3000"
    environment:
      - GF_AUTH_ANONYMOUS_ENABLED=true
      - GF_AUTH_ANONYMOUS_ORG_ROLE=Admin
    volumes:
      - grafana-data:/var/lib/grafana
      - ./config/grafana/dashboards:/etc/grafana/provisioning/dashboards

volumes:
  influxdb-data:
  grafana-data:
```

Run k6 with InfluxDB output:

```bash
k6 run --out influxdb=http://localhost:8086/k6 tests/load/scenarios/baseline.js
```

## Step 9: Set Performance Targets

Define SLAs in your test options:

```javascript
export let options = {
    thresholds: {
        // HTTP errors should be less than 1%
        http_req_failed: ['rate<0.01'],

        // 95% of requests should be below 500ms
        http_req_duration: ['p(95)<500'],

        // 99% of requests should be below 1s
        'http_req_duration{name:CreateEvent}': ['p(99)<1000'],

        // Requests per second should be above 100
        http_reqs: ['rate>100'],
    },
};
```

## Step 10: Continuous Load Testing

Add to GitHub Actions:

```yaml
# .github/workflows/load-test.yaml
name: Load Test

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:

jobs:
  load-test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Start XZepr
        run: |
          docker-compose up -d
          sleep 30  # Wait for startup

      - name: Create API Key
        id: apikey
        run: |
          API_KEY=$(docker-compose exec -T xzepr \
            ./admin apikey create --user-id test-user --name ci-test)
          echo "::add-mask::$API_KEY"
          echo "API_KEY=$API_KEY" >> $GITHUB_OUTPUT

      - name: Run k6 baseline test
        uses: grafana/k6-action@v0.3.0
        with:
          filename: tests/load/scenarios/baseline.js
        env:
          BASE_URL: http://localhost:8042
          API_KEY: ${{ steps.apikey.outputs.API_KEY }}

      - name: Upload results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: k6-results
          path: tests/load/results/

      - name: Check thresholds
        run: |
          if grep -q '"failed":true' tests/load/results/baseline-summary.json; then
            echo "Load test thresholds failed"
            exit 1
          fi
```

## Troubleshooting

### High Error Rate

If you see high error rates:

1. Check server logs for errors
2. Monitor resource usage (CPU, memory)
3. Check database connection pool
4. Verify rate limits aren't too restrictive

### Slow Response Times

If response times are slow:

1. Check database query performance
2. Look for N+1 query problems
3. Check for blocking operations
4. Profile the application
5. Review database indexes

### Connection Errors

If you see connection errors:

1. Increase server connection limits
2. Check network configuration
3. Verify firewall rules
4. Check for connection pool exhaustion

## Best Practices

1. **Start small** - Begin with baseline tests before stress testing
2. **Use realistic data** - Test with production-like data volumes
3. **Monitor resources** - Watch CPU, memory, disk, network
4. **Test incrementally** - Gradually increase load
5. **Test regularly** - Run tests on schedule, not just before release
6. **Test in staging** - Don't load test production without planning
7. **Document baselines** - Record baseline performance metrics
8. **Set alerts** - Alert on performance degradation
9. **Test failure modes** - Test how system recovers from failures
10. **Share results** - Make results visible to the team

## Next Steps

- Set up Prometheus for real-time metrics
- Create custom Grafana dashboards
- Add distributed tracing analysis
- Implement chaos engineering tests
- Set up automated performance regression detection
- Create runbooks for performance issues
