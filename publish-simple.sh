#!/bin/bash

# PocketFlow-RS Simple Publishing Script
# A simplified version that handles workspace publishing with dependency version management

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Get workspace version
get_workspace_version() {
    grep "version.*=" Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

# Update internal dependency versions before publishing
update_dependency_versions() {
    local version="$1"
    
    print_info "Updating internal dependency versions to $version..."
    
    # Update pocketflow-tools dependencies
    sed -i.bak "s|pocketflow-core = { workspace = true }|pocketflow-core = { version = \"$version\", path = \"../pocketflow-core\" }|g" pocketflow-tools/Cargo.toml
    
    # Update pocketflow-mcp dependencies  
    sed -i.bak "s|pocketflow-core = { workspace = true }|pocketflow-core = { version = \"$version\", path = \"../pocketflow-core\" }|g" pocketflow-mcp/Cargo.toml
    
    # Update pocketflow-cognitive dependencies
    sed -i.bak "s|pocketflow-core = { workspace = true }|pocketflow-core = { version = \"$version\", path = \"../pocketflow-core\" }|g" pocketflow-cognitive/Cargo.toml
    sed -i.bak "s|pocketflow-mcp = { workspace = true }|pocketflow-mcp = { version = \"$version\", path = \"../pocketflow-mcp\" }|g" pocketflow-cognitive/Cargo.toml
    
    # Update pocketflow-agent dependencies
    sed -i.bak "s|pocketflow-core = { workspace = true }|pocketflow-core = { version = \"$version\", path = \"../pocketflow-core\" }|g" pocketflow-agent/Cargo.toml
    sed -i.bak "s|pocketflow-tools = { workspace = true }|pocketflow-tools = { version = \"$version\", path = \"../pocketflow-tools\" }|g" pocketflow-agent/Cargo.toml
    
    print_success "Updated dependency versions"
}

# Restore workspace dependencies
restore_workspace_dependencies() {
    print_info "Restoring workspace dependencies..."
    
    # Restore from backup files
    for crate in pocketflow-tools pocketflow-mcp pocketflow-cognitive pocketflow-agent; do
        if [[ -f "$crate/Cargo.toml.bak" ]]; then
            mv "$crate/Cargo.toml.bak" "$crate/Cargo.toml"
        fi
    done
    
    print_success "Restored workspace dependencies"
}

# Publish a single crate
publish_crate() {
    local crate=$1
    local dry_run=${2:-false}
    
    print_info "Publishing $crate..."
    
    if [[ "$dry_run" == "true" ]]; then
        cargo publish --package "$crate" --allow-dirty --dry-run
        print_success "Dry run successful for $crate"
    else
        if cargo publish --package "$crate" --allow-dirty; then
            print_success "Published $crate successfully"
            # Wait for crates.io to process
            print_info "Waiting 30 seconds for crates.io to process $crate..."
            sleep 30
        else
            print_error "Failed to publish $crate"
            return 1
        fi
    fi
}

# Main function
main() {
    local dry_run=${1:-false}
    
    # Check if we're in workspace root
    if [[ ! -f "Cargo.toml" ]] || ! grep -q "\[workspace\]" Cargo.toml; then
        print_error "This script must be run from the workspace root directory"
        exit 1
    fi
    
    local version
    version=$(get_workspace_version)
    
    print_info "PocketFlow-RS Publishing Script"
    print_info "Workspace version: $version"
    
    if [[ "$dry_run" == "true" ]]; then
        print_warning "Running in DRY-RUN mode - no actual publishing will occur"
    fi
    
    # Define crates in dependency order
    local crates=(
        "pocketflow-core"
        "pocketflow-tools"
        "pocketflow-mcp"
        "pocketflow-cognitive"
        "pocketflow-agent"
    )
    
    # Setup trap to restore dependencies on exit
    trap restore_workspace_dependencies EXIT
    
    # Update dependencies for publishing
    update_dependency_versions "$version"
    
    # Publish each crate
    local failed_crates=()
    
    for crate in "${crates[@]}"; do
        print_info "----------------------------------------"
        if publish_crate "$crate" "$dry_run"; then
            print_success "$crate published successfully"
        else
            print_error "Failed to publish $crate"
            failed_crates+=("$crate")
        fi
    done
    
    print_info "========================================"
    
    if [[ ${#failed_crates[@]} -eq 0 ]]; then
        if [[ "$dry_run" == "true" ]]; then
            print_success "Dry run completed successfully for all crates!"
        else
            print_success "All crates published successfully!"
            print_info "Published crates:"
            for crate in "${crates[@]}"; do
                print_info "  - https://crates.io/crates/$crate"
            done
        fi
    else
        print_error "Failed to publish: ${failed_crates[*]}"
        exit 1
    fi
}

# Parse arguments
case "${1:-}" in
    --dry-run|-d)
        main true
        ;;
    --help|-h)
        echo "PocketFlow-RS Simple Publishing Script"
        echo ""
        echo "Usage: $0 [--dry-run|--help]"
        echo ""
        echo "Options:"
        echo "  --dry-run, -d   Simulate publishing without actually publishing"
        echo "  --help, -h      Show this help message"
        echo ""
        echo "This script will:"
        echo "1. Update internal dependency versions in Cargo.toml files"
        echo "2. Publish crates in dependency order"
        echo "3. Restore workspace dependencies on completion"
        ;;
    "")
        main false
        ;;
    *)
        echo "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac
