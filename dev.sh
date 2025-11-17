#!/bin/bash

# Development script for nullae project
# Runs both API server and frontend development server

set -e

echo "Starting nullae development environment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to cleanup background processes
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    if [ -n "$API_PID" ]; then
        echo "Stopping API server (PID: $API_PID)"
        kill $API_PID 2>/dev/null || true
    fi
    if [ -n "$FRONTEND_PID" ]; then
        echo "Stopping frontend server (PID: $FRONTEND_PID)"
        kill $FRONTEND_PID 2>/dev/null || true
    fi
    exit 0
}

# Set trap to cleanup on exit
trap cleanup SIGINT SIGTERM EXIT

# Check if required tools are installed
check_dependencies() {
    echo -e "${BLUE}Checking dependencies...${NC}"

    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: cargo is not installed${NC}"
        exit 1
    fi

    if ! command -v trunk &> /dev/null; then
        echo -e "${RED}Error: trunk is not installed. Install with: cargo install trunk${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ All dependencies found${NC}"
}

# Build and start API server
start_api() {
    echo -e "${BLUE}Building and starting API server...${NC}"

    # Build the API
    cargo build --bin nullae-api

    # Start API server in background
    ./target/debug/nullae-api &
    API_PID=$!

    # Wait for API to start
    sleep 3

    if ps -p $API_PID > /dev/null; then
        echo -e "${GREEN}✓ API server started (PID: $API_PID)${NC}"
        echo -e "  API available at: http://127.0.0.1:3000"
    else
        echo -e "${RED}✗ Failed to start API server${NC}"
        exit 1
    fi
}

# Start frontend development server
start_frontend() {
    echo -e "${BLUE}Starting frontend development server...${NC}"

    cd crates/nullae-ui

    # Start trunk dev server in background
    trunk serve --open &
    FRONTEND_PID=$!

    cd ../..

    # Wait a bit for frontend to start
    sleep 2

    if ps -p $FRONTEND_PID > /dev/null; then
        echo -e "${GREEN}✓ Frontend server started (PID: $FRONTEND_PID)${NC}"
        echo -e "  Frontend available at: http://127.0.0.1:8080"
    else
        echo -e "${RED}✗ Failed to start frontend server${NC}"
        exit 1
    fi
}

# Test API endpoint
test_api() {
    echo -e "${BLUE}Testing API endpoint...${NC}"

    if curl -s -X POST http://127.0.0.1:3000/api/v1/short \
        -H "Content-Type: application/json" \
        -d '{"url":"https://example.com"}' \
        --connect-timeout 5 > /dev/null; then
        echo -e "${GREEN}✓ API endpoint is responding${NC}"
    else
        echo -e "${YELLOW}⚠ API endpoint not responding yet, will retry...${NC}"
        sleep 2
        # Retry once
        if curl -s -X POST http://127.0.0.1:3000/api/v1/short \
            -H "Content-Type: application/json" \
            -d '{"url":"https://example.com"}' \
            --connect-timeout 5 > /dev/null; then
            echo -e "${GREEN}✓ API endpoint is now responding${NC}"
        else
            echo -e "${RED}✗ API endpoint failed to respond${NC}"
        fi
    fi
}

# Main execution
main() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}    nullae Development Setup    ${NC}"
    echo -e "${BLUE}================================${NC}"

    check_dependencies
    start_api
    test_api
    start_frontend

    echo -e "\n${GREEN}🎉 Development environment is ready!${NC}"
    echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
    echo -e "\n${BLUE}Services:${NC}"
    echo -e "  API:      http://127.0.0.1:3000"
    echo -e "  Frontend: http://127.0.0.1:8080"

    # Wait for user to press Ctrl+C
    wait
}

# Run main function
main
