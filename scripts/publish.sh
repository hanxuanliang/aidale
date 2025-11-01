#!/bin/bash
# Automated script to publish all Aidale crates to crates.io
# Usage: ./scripts/publish.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored messages
info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to wait for crates.io to update index
wait_for_index() {
    local crate_name=$1
    local wait_time=30
    info "Waiting ${wait_time}s for crates.io to update index for ${crate_name}..."
    sleep $wait_time
}

# Check if cargo login has been done
info "Checking if logged in to crates.io..."
if ! cargo login --help &> /dev/null; then
    error "cargo login failed. Please run 'cargo login' first."
    exit 1
fi

# Crates to publish in dependency order
crates=(
    "aidale-core"
    "aidale-provider"
    "aidale-layer"
    "aidale-plugin"
    "aidale"
)

info "Starting publication of Aidale crates..."
echo ""

# Pre-flight checks
info "Running pre-flight checks..."
for crate in "${crates[@]}"; do
    info "Checking ${crate}..."
    cd "$crate"

    # Run tests
    if ! cargo test --quiet; then
        error "Tests failed for ${crate}"
        exit 1
    fi

    # Check for warnings
    if ! cargo check --quiet; then
        error "Cargo check failed for ${crate}"
        exit 1
    fi

    cd ..
done

info "All pre-flight checks passed!"
echo ""

# Ask for confirmation
read -p "Ready to publish all crates. Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    warn "Publication cancelled."
    exit 0
fi

# Publish each crate
for crate in "${crates[@]}"; do
    info "Publishing ${crate}..."
    cd "$crate"

    # Try to publish
    if cargo publish; then
        info "Successfully published ${crate}"
    else
        error "Failed to publish ${crate}"
        cd ..
        exit 1
    fi

    cd ..

    # Wait for index update (except for the last crate)
    if [ "$crate" != "aidale" ]; then
        wait_for_index "$crate"
    fi

    echo ""
done

info "🎉 All crates published successfully!"
info "View at: https://crates.io/crates/aidale"
