#!/bin/bash

# XZEPR Health Check Script
# This script performs health checks for the XZEPR server container

set -euo pipefail

# Configuration
HEALTH_ENDPOINT="${HEALTH_ENDPOINT:-https://localhost:8443/health}"
TIMEOUT="${HEALTH_TIMEOUT:-10}"
MAX_RETRIES="${MAX_RETRIES:-3}"
RETRY_DELAY="${RETRY_DELAY:-2}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_success() {
    echo -e "${GREEN}[HEALTHY]${NC} $1"
}

print_error() {
    echo -e "${RED}[UNHEALTHY]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to check if curl is available
check_curl() {
    if ! command -v curl &> /dev/null; then
        print_error "curl command not found. Please install curl."
        exit 1
    fi
}

# Function to perform HTTP health check
http_health_check() {
    local endpoint="$1"
    local timeout="$2"

    # Perform the health check request
    local response
    local http_code

    response=$(curl -f -s -k \
        --max-time "$timeout" \
        --connect-timeout 5 \
        --retry 0 \
        -w "HTTP_CODE:%{http_code}" \
        "$endpoint" 2>/dev/null || echo "CURL_FAILED")

    if [[ "$response" == "CURL_FAILED" ]]; then
        return 1
    fi

    # Extract HTTP status code
    http_code=$(echo "$response" | grep -o 'HTTP_CODE:[0-9]*' | cut -d: -f2)

    # Check if HTTP status is successful (2xx)
    if [[ "$http_code" -ge 200 && "$http_code" -lt 300 ]]; then
        return 0
    else
        print_error "HTTP status code: $http_code"
        return 1
    fi
}

# Function to check database connectivity (if endpoint provides it)
check_database_status() {
    local endpoint="$1/db"
    local timeout="$2"

    local response
    response=$(curl -f -s -k \
        --max-time "$timeout" \
        --connect-timeout 5 \
        "$endpoint" 2>/dev/null || echo "FAILED")

    if [[ "$response" != "FAILED" ]]; then
        print_success "Database connectivity: OK"
        return 0
    else
        print_warning "Database status check failed or not available"
        return 1
    fi
}

# Function to check Kafka/Redpanda connectivity (if endpoint provides it)
check_messaging_status() {
    local endpoint="$1/messaging"
    local timeout="$2"

    local response
    response=$(curl -f -s -k \
        --max-time "$timeout" \
        --connect-timeout 5 \
        "$endpoint" 2>/dev/null || echo "FAILED")

    if [[ "$response" != "FAILED" ]]; then
        print_success "Messaging system connectivity: OK"
        return 0
    else
        print_warning "Messaging status check failed or not available"
        return 1
    fi
}

# Function to check memory usage
check_memory_usage() {
    if command -v free &> /dev/null; then
        local memory_usage
        memory_usage=$(free | grep Mem | awk '{printf "%.1f", $3/$2 * 100.0}')

        if (( $(echo "$memory_usage > 90" | bc -l) )); then
            print_warning "High memory usage: ${memory_usage}%"
            return 1
        else
            print_success "Memory usage: ${memory_usage}%"
            return 0
        fi
    else
        print_warning "Memory check not available (free command not found)"
        return 0
    fi
}

# Function to check disk space
check_disk_space() {
    if command -v df &> /dev/null; then
        local disk_usage
        disk_usage=$(df /app 2>/dev/null | tail -1 | awk '{print $5}' | sed 's/%//')

        if [[ -n "$disk_usage" ]] && [[ "$disk_usage" -gt 90 ]]; then
            print_warning "High disk usage: ${disk_usage}%"
            return 1
        else
            print_success "Disk usage: ${disk_usage:-unknown}%"
            return 0
        fi
    else
        print_warning "Disk check not available (df command not found)"
        return 0
    fi
}

# Function to perform comprehensive health check
comprehensive_health_check() {
    local success=true

    echo "Performing comprehensive health check..."
    echo "Endpoint: $HEALTH_ENDPOINT"
    echo "Timeout: ${TIMEOUT}s"
    echo ""

    # Primary HTTP health check
    if http_health_check "$HEALTH_ENDPOINT" "$TIMEOUT"; then
        print_success "Primary health check: PASSED"
    else
        print_error "Primary health check: FAILED"
        success=false
    fi

    # Additional checks (optional, may fail gracefully)
    check_database_status "$HEALTH_ENDPOINT" "$TIMEOUT" || true
    check_messaging_status "$HEALTH_ENDPOINT" "$TIMEOUT" || true

    # System resource checks
    check_memory_usage || true
    check_disk_space || true

    return $success
}

# Main health check function with retries
main_health_check() {
    local attempt=1
    local success=false

    while [[ $attempt -le $MAX_RETRIES ]]; do
        echo "Health check attempt $attempt/$MAX_RETRIES"

        if comprehensive_health_check; then
            success=true
            break
        fi

        if [[ $attempt -lt $MAX_RETRIES ]]; then
            echo "Waiting ${RETRY_DELAY}s before retry..."
            sleep "$RETRY_DELAY"
        fi

        ((attempt++))
    done

    if [[ "$success" == true ]]; then
        print_success "XZEPR server is healthy"
        exit 0
    else
        print_error "XZEPR server is unhealthy after $MAX_RETRIES attempts"
        exit 1
    fi
}

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Perform health checks for XZEPR server.

OPTIONS:
    -e, --endpoint URL      Health check endpoint (default: $HEALTH_ENDPOINT)
    -t, --timeout SECONDS   Request timeout (default: $TIMEOUT)
    -r, --retries COUNT     Maximum retry attempts (default: $MAX_RETRIES)
    -d, --delay SECONDS     Delay between retries (default: $RETRY_DELAY)
    --simple               Perform simple health check only
    --help                 Show this help message

EXAMPLES:
    # Basic health check
    $0

    # Custom endpoint and timeout
    $0 -e https://xzepr.example.com/health -t 15

    # With custom retry settings
    $0 -r 5 -d 3

    # Simple check without additional diagnostics
    $0 --simple

ENVIRONMENT VARIABLES:
    HEALTH_ENDPOINT        Override default health endpoint
    HEALTH_TIMEOUT         Override default timeout
    MAX_RETRIES           Override default retry count
    RETRY_DELAY           Override default retry delay

EOF
}

# Parse command line arguments
SIMPLE_CHECK=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--endpoint)
            HEALTH_ENDPOINT="$2"
            shift 2
            ;;
        -t|--timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        -r|--retries)
            MAX_RETRIES="$2"
            shift 2
            ;;
        -d|--delay)
            RETRY_DELAY="$2"
            shift 2
            ;;
        --simple)
            SIMPLE_CHECK=true
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Validate numeric parameters
if ! [[ "$TIMEOUT" =~ ^[0-9]+$ ]] || [[ "$TIMEOUT" -le 0 ]]; then
    print_error "Invalid timeout value: $TIMEOUT"
    exit 1
fi

if ! [[ "$MAX_RETRIES" =~ ^[0-9]+$ ]] || [[ "$MAX_RETRIES" -le 0 ]]; then
    print_error "Invalid retry count: $MAX_RETRIES"
    exit 1
fi

if ! [[ "$RETRY_DELAY" =~ ^[0-9]+$ ]] || [[ "$RETRY_DELAY" -lt 0 ]]; then
    print_error "Invalid retry delay: $RETRY_DELAY"
    exit 1
fi

# Check dependencies
check_curl

# Perform health check
if [[ "$SIMPLE_CHECK" == true ]]; then
    # Simple check - just HTTP status
    if http_health_check "$HEALTH_ENDPOINT" "$TIMEOUT"; then
        print_success "XZEPR server is responding"
        exit 0
    else
        print_error "XZEPR server is not responding"
        exit 1
    fi
else
    # Comprehensive check with retries
    main_health_check
fi
