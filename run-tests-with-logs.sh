#!/bin/bash

# Script to run integration tests while showing docker-compose logs
# This provides an alternative way to see docker-compose output during tests

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
    echo -e "${BLUE}[TEST-RUNNER]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[TEST-RUNNER]${NC} $1"
}

print_error() {
    echo -e "${RED}[TEST-RUNNER]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[TEST-RUNNER]${NC} $1"
}

# Cleanup function
cleanup() {
    print_status "Cleaning up..."
    if [ ! -z "$LOGS_PID" ]; then
        kill $LOGS_PID 2>/dev/null || true
    fi
    make down 2>/dev/null || true
    exit
}

# Set up signal handlers for cleanup
trap cleanup SIGINT SIGTERM EXIT

print_status "Starting docker-compose services..."

# Clean up any existing containers
make down 2>/dev/null || true

# Start docker-compose in the background
print_status "Starting services with: make run"
make run &
COMPOSE_PID=$!

# Wait a moment for services to start
sleep 5

# Start following logs in the background
print_status "Starting to follow docker-compose logs..."
docker compose logs -f &
LOGS_PID=$!

# Wait a bit more for services to be ready
print_status "Waiting for services to be ready..."
sleep 15

# Check if services are running
if ! docker compose ps | grep -q "Up"; then
    print_error "Services failed to start properly"
    docker compose ps
    exit 1
fi

print_success "Services are running. Starting integration tests..."

# Run the integration tests
print_status "Running integration tests..."
if cargo test --workspace --test e2e -- --nocapture; then
    print_success "Integration tests passed!"
    TEST_RESULT=0
else
    print_error "Integration tests failed!"
    TEST_RESULT=1
fi

print_status "Test run completed with exit code: $TEST_RESULT"

# Cleanup will be handled by the trap
exit $TEST_RESULT
