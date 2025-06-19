#!/bin/bash

# Test script to verify Prometheus metrics endpoint
# This script starts the connector and tests the metrics endpoint

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$PROJECT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[METRICS-TEST]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[METRICS-TEST]${NC} $1"
}

print_error() {
    echo -e "${RED}[METRICS-TEST]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[METRICS-TEST]${NC} $1"
}

# Cleanup function
cleanup() {
    print_status "Cleaning up..."
    make down 2>/dev/null || true
    exit
}

# Set up signal handlers for cleanup
trap cleanup SIGINT SIGTERM EXIT

print_status "Building the project..."
make build

print_status "Starting docker-compose services..."
make down 2>/dev/null || true
make run &
COMPOSE_PID=$!

print_status "Waiting for services to start..."
sleep 20

# Check if services are running
if ! docker compose ps | grep -q "Up"; then
    print_error "Services failed to start properly"
    docker compose ps
    exit 1
fi

print_success "Services are running!"

# Test health endpoint
print_status "Testing health endpoint..."
if curl -s http://localhost:3000/health | grep -q "OK"; then
    print_success "Health endpoint is working"
else
    print_error "Health endpoint is not working"
    exit 1
fi

# Test metrics endpoint
print_status "Testing metrics endpoint..."
METRICS_RESPONSE=$(curl -s http://localhost:3000/metrics)

if [ $? -eq 0 ]; then
    print_success "Metrics endpoint is accessible"
else
    print_error "Failed to access metrics endpoint"
    exit 1
fi

# Verify expected metrics are present
print_status "Verifying metrics content..."

# Check for required metrics
REQUIRED_METRICS=(
    "mrcon_running_servers_total"
    "mrcon_collection_servers"
    "mrcon_tasks_started_total"
    "mrcon_task_restarts_total"
    "mrcon_task_failures_total"
)

ALL_PRESENT=true
for metric in "${REQUIRED_METRICS[@]}"; do
    if echo "$METRICS_RESPONSE" | grep -q "$metric"; then
        print_success "Found metric: $metric"
    else
        print_error "Missing metric: $metric"
        ALL_PRESENT=false
    fi
done

if [ "$ALL_PRESENT" = true ]; then
    print_success "All required metrics are present!"
else
    print_error "Some metrics are missing"
    exit 1
fi

# Show some sample metrics
print_status "Sample metrics output:"
echo "$METRICS_RESPONSE" | head -20

# Test with specific metric queries
print_status "Testing specific metrics..."

# Check running servers count
RUNNING_SERVERS=$(echo "$METRICS_RESPONSE" | grep "^mrcon_running_servers_total" | awk '{print $2}')
if [ -n "$RUNNING_SERVERS" ]; then
    print_success "Running servers count: $RUNNING_SERVERS"
else
    print_warning "Could not extract running servers count"
fi

# Check tasks started
TASKS_STARTED=$(echo "$METRICS_RESPONSE" | grep "^mrcon_tasks_started_total" | awk '{print $2}')
if [ -n "$TASKS_STARTED" ]; then
    print_success "Tasks started total: $TASKS_STARTED"
else
    print_warning "Could not extract tasks started count"
fi

# Test metrics format (should be Prometheus format)
if echo "$METRICS_RESPONSE" | grep -q "^# HELP"; then
    print_success "Metrics are in proper Prometheus format"
else
    print_warning "Metrics might not be in proper Prometheus format"
fi

# Test that metrics endpoint returns proper content type
print_status "Testing content type..."
CONTENT_TYPE=$(curl -s -I http://localhost:3000/metrics | grep -i "content-type" | tr -d '\r')
if echo "$CONTENT_TYPE" | grep -q "text/plain"; then
    print_success "Content type is correct: $CONTENT_TYPE"
else
    print_warning "Content type might not be optimal: $CONTENT_TYPE"
fi

# Save metrics to file for inspection
print_status "Saving metrics output to metrics-output.txt"
echo "$METRICS_RESPONSE" > metrics-output.txt
print_success "Metrics saved to metrics-output.txt"

print_success "All metrics tests passed!"
print_status "You can now access:"
print_status "  Health endpoint: http://localhost:3000/health"
print_status "  Metrics endpoint: http://localhost:3000/metrics"
print_status ""
print_status "Press Ctrl+C to stop the services"

# Keep running until interrupted
wait $COMPOSE_PID
