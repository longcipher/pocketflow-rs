#!/bin/bash

# PocketFlow-RS Workspace Publishing Script
# 
# This script publishes all crates in the workspace to crates.io in the correct order,
# respecting dependency relationships between crates.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
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

# Function to check if we're in the right directory
check_workspace() {
    if [[ ! -f "Cargo.toml" ]] || ! grep -q "\[workspace\]" Cargo.toml; then
        print_error "This script must be run from the workspace root directory"
        exit 1
    fi
}

# Function to check if cargo is available
check_cargo() {
    if ! command -v cargo &> /dev/null; then
        print_error "cargo is not installed or not in PATH"
        exit 1
    fi
}

# Function to check if user is logged in to crates.io
check_cargo_login() {
    print_info "Checking cargo login status..."
    if ! cargo login --help &> /dev/null; then
        print_error "Unable to access cargo login. Please ensure you're logged in to crates.io"
        print_info "Run: cargo login <your-api-token>"
        exit 1
    fi
}

# Function to run tests for a crate
run_tests() {
    local crate=$1
    print_info "Running tests for $crate..."
    
    if cargo test --package "$crate" --all-features; then
        print_success "Tests passed for $crate"
    else
        print_error "Tests failed for $crate"
        return 1
    fi
}

# Function to run linting for the workspace
run_lint() {
    print_info "Running lint checks..."
    
    if command -v just &> /dev/null && [[ -f "Justfile" ]]; then
        just lint
    else
        print_warning "just command or Justfile not found, running individual checks..."
        cargo fmt --all -- --check
        cargo clippy --all -- -D warnings
    fi
}

# Function to check if a crate version exists on crates.io
check_version_exists() {
    local crate=$1
    local version
    version=$(cargo pkgid --package "$crate" | sed 's/.*#//')
    
    print_info "Checking if $crate version $version already exists on crates.io..."
    
    # Use cargo search to check if the specific version exists
    if cargo search "$crate" --limit 1 | grep -q "= \"$version\""; then
        print_warning "$crate version $version already exists on crates.io"
        return 0
    else
        print_info "$crate version $version is new, proceeding with publish"
        return 1
    fi
}

# Function to publish a single crate
publish_crate() {
    local crate=$1
    local dry_run=${2:-false}
    
    print_info "Publishing $crate..."
    
    # Check if version already exists (skip if it does)
    if check_version_exists "$crate"; then
        print_warning "Skipping $crate as version already exists"
        return 0
    fi
    
    # Run tests first
    if ! run_tests "$crate"; then
        print_error "Skipping $crate due to test failures"
        return 1
    fi
    
    # Publish the crate
    local publish_cmd="cargo publish --package $crate"
    if [[ "$dry_run" == "true" ]]; then
        publish_cmd="$publish_cmd --dry-run"
        print_info "Dry run: $publish_cmd"
    else
        print_info "Executing: $publish_cmd"
    fi
    
    if $publish_cmd; then
        if [[ "$dry_run" == "true" ]]; then
            print_success "Dry run successful for $crate"
        else
            print_success "Published $crate successfully"
            # Wait a bit for crates.io to process the new crate
            print_info "Waiting 30 seconds for crates.io to process $crate..."
            sleep 30
        fi
    else
        print_error "Failed to publish $crate"
        return 1
    fi
}

# Function to get current version from Cargo.toml
get_workspace_version() {
    grep "version.*=" Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

# Main publishing function
publish_workspace() {
    local dry_run=${1:-false}
    
    # Define publish order based on dependency graph
    local crates=(
        "pocketflow-core"      # Base crate, no internal dependencies
        "pocketflow-tools"     # Depends on: pocketflow-core
        "pocketflow-mcp"       # Depends on: pocketflow-core
        "pocketflow-cognitive" # Depends on: pocketflow-core, pocketflow-mcp
        "pocketflow-agent"     # Depends on: pocketflow-core, pocketflow-tools
    )
    
    local workspace_version
    workspace_version=$(get_workspace_version)
    
    if [[ "$dry_run" == "true" ]]; then
        print_info "=== DRY RUN MODE ==="
        print_info "This will simulate publishing all crates without actually publishing them"
    fi
    
    print_info "Publishing PocketFlow-RS workspace (version: $workspace_version)"
    print_info "Publish order: ${crates[*]}"
    
    local failed_crates=()
    
    for crate in "${crates[@]}"; do
        print_info "----------------------------------------"
        if publish_crate "$crate" "$dry_run"; then
            print_success "$crate processed successfully"
        else
            print_error "Failed to process $crate"
            failed_crates+=("$crate")
        fi
    done
    
    print_info "========================================"
    
    if [[ ${#failed_crates[@]} -eq 0 ]]; then
        if [[ "$dry_run" == "true" ]]; then
            print_success "Dry run completed successfully for all crates!"
        else
            print_success "All crates published successfully!"
        fi
    else
        print_error "Failed to publish the following crates: ${failed_crates[*]}"
        exit 1
    fi
}

# Function to show help
show_help() {
    echo "PocketFlow-RS Publishing Script"
    echo ""
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -h, --help     Show this help message"
    echo "  -d, --dry-run  Run in dry-run mode (simulate publishing without actually publishing)"
    echo "  -t, --test     Run tests for all crates without publishing"
    echo "  -l, --lint     Run lint checks for the workspace"
    echo "  -c, --check    Run all checks (lint + tests) without publishing"
    echo ""
    echo "Examples:"
    echo "  $0                    # Publish all crates to crates.io"
    echo "  $0 --dry-run          # Simulate publishing without actually publishing"
    echo "  $0 --test             # Run tests for all crates"
    echo "  $0 --check            # Run lint and tests"
    echo ""
    echo "Crate publish order (respecting dependencies):"
    echo "  1. pocketflow-core"
    echo "  2. pocketflow-tools"
    echo "  3. pocketflow-mcp"
    echo "  4. pocketflow-cognitive"
    echo "  5. pocketflow-agent"
}

# Parse command line arguments
DRY_RUN=false
TEST_ONLY=false
LINT_ONLY=false
CHECK_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -d|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -t|--test)
            TEST_ONLY=true
            shift
            ;;
        -l|--lint)
            LINT_ONLY=true
            shift
            ;;
        -c|--check)
            CHECK_ONLY=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Main execution
main() {
    print_info "Starting PocketFlow-RS publishing script..."
    
    # Pre-flight checks
    check_workspace
    check_cargo
    
    if [[ "$LINT_ONLY" == "true" ]]; then
        print_info "Running lint checks only..."
        run_lint
        print_success "Lint checks completed"
        exit 0
    fi
    
    if [[ "$TEST_ONLY" == "true" ]]; then
        print_info "Running tests for all crates..."
        local crates=("pocketflow-core" "pocketflow-tools" "pocketflow-mcp" "pocketflow-cognitive" "pocketflow-agent")
        for crate in "${crates[@]}"; do
            run_tests "$crate"
        done
        print_success "All tests completed"
        exit 0
    fi
    
    if [[ "$CHECK_ONLY" == "true" ]]; then
        print_info "Running lint and tests..."
        run_lint
        local crates=("pocketflow-core" "pocketflow-tools" "pocketflow-mcp" "pocketflow-cognitive" "pocketflow-agent")
        for crate in "${crates[@]}"; do
            run_tests "$crate"
        done
        print_success "All checks completed"
        exit 0
    fi
    
    # Check login status if not dry run
    if [[ "$DRY_RUN" == "false" ]]; then
        check_cargo_login
    fi
    
    # Run pre-publish checks
    print_info "Running pre-publish checks..."
    run_lint
    
    # Publish workspace
    publish_workspace "$DRY_RUN"
    
    if [[ "$DRY_RUN" == "false" ]]; then
        print_success "PocketFlow-RS workspace published successfully!"
        print_info "You can view the published crates at:"
        print_info "  - https://crates.io/crates/pocketflow-core"
        print_info "  - https://crates.io/crates/pocketflow-tools"
        print_info "  - https://crates.io/crates/pocketflow-mcp"
        print_info "  - https://crates.io/crates/pocketflow-cognitive"
        print_info "  - https://crates.io/crates/pocketflow-agent"
    fi
}

# Run main function
main "$@"
