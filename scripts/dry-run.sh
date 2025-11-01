#!/bin/bash
# Dry-run publication (package only, don't publish)
# Usage: ./scripts/dry-run.sh

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

crates=(
    "aidale-core"
    "aidale-provider"
    "aidale-layer"
    "aidale-plugin"
    "aidale"
)

info "Running dry-run for all crates..."
echo ""

for crate in "${crates[@]}"; do
    info "Packaging ${crate}..."
    cd "$crate"

    # Package without publishing
    cargo package --allow-dirty

    # Show what would be uploaded
    warn "Package contents for ${crate}:"
    cargo package --list

    cd ..
    echo ""
done

info "✅ Dry-run complete!"
info "Check target/package/ for generated .crate files"
