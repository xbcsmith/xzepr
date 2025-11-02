#!/bin/bash
# test_demo.sh - XZepr API Demo Script
#
# This script demonstrates the functionality described in the curl examples
# and Python script by using our Rust implementation.
#
# Usage: ./test_demo.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
API_BASE="http://localhost:8042"
SERVER_PID=""

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Function to check if server is running
check_server() {
    if curl -s "${API_BASE}/health" > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Function to start server
start_server() {
    print_step "Starting XZepr server..."

    if check_server; then
        print_status "Server is already running"
        return 0
    fi

    # Build the server first
    print_status "Building XZepr server..."
    cargo build --bin server --release

    # Start server in background
    print_status "Starting server on ${API_BASE}..."
    cargo run --bin server --release &
    SERVER_PID=$!

    # Wait for server to start
    for i in {1..30}; do
        if check_server; then
            print_status "Server started successfully (PID: ${SERVER_PID})"
            return 0
        fi
        sleep 1
    done

    print_error "Failed to start server"
    return 1
}

# Function to stop server
stop_server() {
    if [ -n "$SERVER_PID" ]; then
        print_step "Stopping server (PID: ${SERVER_PID})..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        print_status "Server stopped"
    fi
}

# Function to cleanup on exit
cleanup() {
    stop_server
}
trap cleanup EXIT

# Function to make API calls and extract ID
api_call() {
    local method=$1
    local endpoint=$2
    local data=$3

    if [ -n "$data" ]; then
        curl -s -X "$method" \
             -H "Content-Type: application/json" \
             -d "$data" \
             "${API_BASE}${endpoint}"
    else
        curl -s -X "$method" \
             -H "Content-Type: application/json" \
             "${API_BASE}${endpoint}"
    fi
}

# Function to extract ID from response
extract_id() {
    echo "$1" | jq -r '.data // .id // empty'
}

# Main demo function
run_demo() {
    print_step "XZepr API Demo - Following examples from 03-curl.md"
    echo

    # Health check
    print_step "1. Health Check"
    health_response=$(api_call GET "/health")
    echo "Health: $(echo "$health_response" | jq -r '.status')"
    echo

    # Create event receiver (following the curl example)
    print_step "2. Creating Event Receiver (as per curl example)"
    receiver_data='{
        "name": "foobar",
        "type": "foo.bar",
        "version": "1.1.3",
        "description": "The event receiver of Brixton",
        "schema": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                }
            }
        }
    }'

    print_status "Creating event receiver with data:"
    echo "$receiver_data" | jq .

    receiver_response=$(api_call POST "/api/v1/receivers" "$receiver_data")
    receiver_id=$(extract_id "$receiver_response")

    if [ -n "$receiver_id" ]; then
        print_status "✓ Event receiver created with ID: $receiver_id"
        echo "Response: $receiver_response" | jq .
    else
        print_error "✗ Failed to create event receiver"
        echo "Response: $receiver_response"
        return 1
    fi
    echo

    # Create event (following the curl example)
    print_step "3. Creating Event (as per curl example)"
    event_data=$(cat <<EOF
{
    "name": "magnificent",
    "version": "7.0.1",
    "release": "2023.11.16",
    "platform_id": "linux",
    "package": "docker",
    "description": "blah",
    "payload": {
        "name": "joe"
    },
    "success": true,
    "event_receiver_id": "$receiver_id"
}
EOF
)

    print_status "Creating event with receiver ID: $receiver_id"
    echo "$event_data" | jq .

    event_response=$(api_call POST "/api/v1/events" "$event_data")
    event_id=$(extract_id "$event_response")

    if [ -n "$event_id" ]; then
        print_status "✓ Event created with ID: $event_id"
        echo "Response: $event_response" | jq .
    else
        print_error "✗ Failed to create event"
        echo "Response: $event_response"
        return 1
    fi
    echo

    # Create event receiver group (following the curl example)
    print_step "4. Creating Event Receiver Group (as per curl example)"
    group_data=$(cat <<EOF
{
    "name": "the_clash",
    "type": "foo.bar",
    "version": "3.3.3",
    "description": "The only event receiver group that matters",
    "enabled": true,
    "event_receiver_ids": ["$receiver_id"]
}
EOF
)

    print_status "Creating event receiver group with receiver ID: $receiver_id"
    echo "$group_data" | jq .

    group_response=$(api_call POST "/api/v1/groups" "$group_data")
    group_id=$(extract_id "$group_response")

    if [ -n "$group_id" ]; then
        print_status "✓ Event receiver group created with ID: $group_id"
        echo "Response: $group_response" | jq .
    else
        print_error "✗ Failed to create event receiver group"
        echo "Response: $group_response"
        return 1
    fi
    echo

    # Query operations (following the curl examples)
    print_step "5. Querying Created Resources"

    # Get event
    print_status "Getting event: $event_id"
    event_detail=$(api_call GET "/api/v1/events/$event_id")
    echo "Event details:"
    echo "$event_detail" | jq .
    echo

    # Get event receiver
    print_status "Getting event receiver: $receiver_id"
    receiver_detail=$(api_call GET "/api/v1/receivers/$receiver_id")
    echo "Event receiver details:"
    echo "$receiver_detail" | jq .
    echo

    # Get event receiver group
    print_status "Getting event receiver group: $group_id"
    group_detail=$(api_call GET "/api/v1/groups/$group_id")
    echo "Event receiver group details:"
    echo "$group_detail" | jq .
    echo

    # Additional demo: Generate more sample data
    print_step "6. Generating Additional Sample Data"

    # Create CDEvents-style receivers (like the Python script)
    cdevents_types=(
        "dev.cdevents.pipelinerun.started.0.2.0"
        "dev.cdevents.pipelinerun.queued.0.2.0"
        "dev.cdevents.artifact.packaged.0.2.0"
        "dev.cdevents.artifact.published.0.2.0"
        "dev.cdevents.build.started.0.2.0"
        "dev.cdevents.build.finished.0.2.0"
    )

    created_receivers=()

    for event_type in "${cdevents_types[@]}"; do
        # Parse name and version from type (simplified)
        name=$(echo "$event_type" | cut -d'.' -f1-3 | tr '.' '-')
        version=$(echo "$event_type" | rev | cut -d'.' -f1-3 | rev)
        description="CDEvents $(echo "$event_type" | cut -d'.' -f3) event receiver"

        receiver_data=$(cat <<EOF
{
    "name": "$name",
    "type": "$event_type",
    "version": "$version",
    "description": "$description",
    "schema": {
        "type": "object",
        "properties": {
            "subject": {"type": "object"},
            "customData": {"type": "object"}
        }
    }
}
EOF
)

        response=$(api_call POST "/api/v1/receivers" "$receiver_data")
        new_receiver_id=$(extract_id "$response")

        if [ -n "$new_receiver_id" ]; then
            print_status "✓ Created CDEvents receiver: $name ($new_receiver_id)"
            created_receivers+=("$new_receiver_id")

            # Create a sample event for this receiver
            sample_event_data=$(cat <<EOF
{
    "name": "sample-$(echo "$name" | tr 'A-Z' 'a-z')",
    "version": "1.0.0",
    "release": "$(date +%Y.%m.%d)",
    "platform_id": "kubernetes",
    "package": "helm",
    "description": "Sample $name event",
    "payload": {
        "subject": {
            "id": "sample-subject-id",
            "type": "$event_type"
        },
        "customData": {
            "generated": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        }
    },
    "success": $([ $((RANDOM % 4)) -ne 0 ] && echo "true" || echo "false"),
    "event_receiver_id": "$new_receiver_id"
}
EOF
)

            event_response=$(api_call POST "/api/v1/events" "$sample_event_data")
            sample_event_id=$(extract_id "$event_response")

            if [ -n "$sample_event_id" ]; then
                print_status "  ✓ Created sample event: $sample_event_id"
            fi
        else
            print_warning "✗ Failed to create receiver: $name"
        fi
    done
    echo

    # Create a group with all CDEvents receivers
    if [ ${#created_receivers[@]} -gt 0 ]; then
        print_step "7. Creating CDEvents Group"

        # Convert array to JSON array
        receivers_json=$(printf '%s\n' "${created_receivers[@]}" | jq -R . | jq -s .)

        cdevents_group_data=$(cat <<EOF
{
    "name": "cdevents-pipeline",
    "type": "dev.cdevents.pipeline",
    "version": "0.2.0",
    "description": "CDEvents pipeline event receivers group",
    "enabled": true,
    "event_receiver_ids": $receivers_json
}
EOF
)

        cdevents_group_response=$(api_call POST "/api/v1/groups" "$cdevents_group_data")
        cdevents_group_id=$(extract_id "$cdevents_group_response")

        if [ -n "$cdevents_group_id" ]; then
            print_status "✓ CDEvents group created: $cdevents_group_id"
            echo "Group contains ${#created_receivers[@]} receivers"
        fi
    fi
    echo

    # List all receivers
    print_step "8. Listing All Event Receivers"
    receivers_list=$(api_call GET "/api/v1/receivers?limit=20&offset=0")
    receiver_count=$(echo "$receivers_list" | jq -r '.pagination.total // 0')
    print_status "Total event receivers: $receiver_count"

    if [ "$receiver_count" -gt 0 ]; then
        echo "Receivers:"
        echo "$receivers_list" | jq -r '.data[]? | "- \(.name) (\(.type)) - \(.id)"'
    fi
    echo

    print_step "Demo completed successfully!"
    print_status "✓ Created event receiver: $receiver_id"
    print_status "✓ Created event: $event_id"
    print_status "✓ Created event receiver group: $group_id"
    print_status "✓ Created ${#created_receivers[@]} additional CDEvents receivers"
    print_status "✓ Total receivers in system: $receiver_count"
    echo
    print_status "All functionality from 03-curl.md and generate_epr_events.py has been demonstrated!"
}

# Main execution
main() {
    echo "XZepr Event Tracking Server - Demo Script"
    echo "========================================"
    echo

    # Check dependencies
    if ! command -v jq &> /dev/null; then
        print_error "jq is required but not installed. Please install jq to run this demo."
        exit 1
    fi

    if ! command -v cargo &> /dev/null; then
        print_error "cargo is required but not installed. Please install Rust to run this demo."
        exit 1
    fi

    # Start server and run demo
    if start_server; then
        sleep 2  # Give server a moment to fully initialize
        run_demo
    else
        print_error "Failed to start server. Demo cannot continue."
        exit 1
    fi
}

# Run main function
main "$@"
