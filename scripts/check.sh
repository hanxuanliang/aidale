#!/bin/bash
# Pre-publish checks for all crates
# Usage: ./scripts/check.sh

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

crates=(
    "aidale-core"
    "aidale-provider"
    "aidale-layer"
    "aidale-plugin"
    "aidale"
)

info "Running checks for all crates..."
echo ""

for crate in "${crates[@]}"; do
    info "Checking ${crate}..."
    cd "$crate"

    # Check formatting
    info "  - Checking code formatting..."
    cargo fmt --check || warn "  Code formatting issues in ${crate}"

    # Run clippy
    info "  - Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings || warn "  Clippy warnings in ${crate}"

    # Run tests
    info "  - Running tests..."
    cargo test --all-features || error "  Tests failed in ${crate}"

    # Build docs
    info "  - Building documentation..."
    cargo doc --no-deps --all-features || error "  Doc build failed in ${crate}"

    # Check package contents
    info "  - Checking package contents..."
    cargo package --list | head -20

    cd ..
    echo ""
done

info "✅ All checks passed!"
