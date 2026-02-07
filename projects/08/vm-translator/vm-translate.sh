#!/bin/bash
# vm-translate.sh - Wrapper script for containerized vm-translator
#
# Usage:
#   ./vm-translate.sh build          # Build container image
#   ./vm-translate.sh test           # Run tests
#   ./vm-translate.sh shell          # Interactive shell
#   ./vm-translate.sh <args>         # Translate VM files
#
# Examples:
#   ./vm-translate.sh SimpleAdd.vm
#   ./vm-translate.sh -v StackTest.vm
#   ./vm-translate.sh FunctionCalls/FibonacciElement/
#
# Requires: Podman (or Docker with CONTAINER_ENGINE=docker)

set -euo pipefail

# ==============================================================================
# Configuration
# ==============================================================================

# Container configuration
IMAGE_NAME="vm-translator"
IMAGE_TAG="2.0.1"
FULL_IMAGE="${IMAGE_NAME}:${IMAGE_TAG}"

# Use podman by default, fall back to docker
CONTAINER_ENGINE="${CONTAINER_ENGINE:-podman}"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ==============================================================================
# Helper Functions
# ==============================================================================

# Print colored message
print_msg() {
    local color=$1
    shift
    echo -e "${color}$*${NC}"
}

# Print info message
info() {
    print_msg "$BLUE" "[INFO]" "$@"
}

# Print success message
success() {
    print_msg "$GREEN" "[SUCCESS]" "$@"
}

# Print warning message
warn() {
    print_msg "$YELLOW" "[WARN]" "$@"
}

# Print error message
error() {
    print_msg "$RED" "[ERROR]" "$@"
}

# Check if container engine is available
check_container_engine() {
    if ! command -v "$CONTAINER_ENGINE" &> /dev/null; then
        error "Container engine '$CONTAINER_ENGINE' not found"
        echo ""
        echo "Please install Podman:"
        echo "  macOS: brew install podman"
        echo "  Linux: sudo apt install podman (or equivalent)"
        echo ""
        echo "Or set CONTAINER_ENGINE=docker to use Docker instead"
        exit 1
    fi

    info "Using container engine: $CONTAINER_ENGINE"
}

# Check if podman machine is running (macOS only)
check_podman_machine() {
    if [[ "$CONTAINER_ENGINE" == "podman" ]] && [[ "$OSTYPE" == "darwin"* ]]; then
        # Check if any machine exists (look for lines with podman-machine)
        if ! podman machine list 2>/dev/null | grep -q "podman-machine"; then
            warn "No podman machine found"
            info "To set up podman machine:"
            echo "  podman machine init"
            echo "  podman machine start"
            echo ""
            info "Or use native binary: cargo build --release"
            exit 1
        fi

        # Check if machine is running
        if ! podman machine list 2>/dev/null | grep -q "Currently running"; then
            warn "Podman machine not running, attempting to start..."
            podman machine start || {
                error "Failed to start podman machine"
                echo "Try manually: podman machine start"
                echo ""
                info "Or use native binary: cargo build --release"
                exit 1
            }
            success "Podman machine started"
        fi
    fi
}

# Build container image
build_image() {
    info "Building container image: $FULL_IMAGE"

    # Navigate to script directory (where Containerfile is)
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    cd "$SCRIPT_DIR"

    # Build with progress
    $CONTAINER_ENGINE build \
        -t "$FULL_IMAGE" \
        -f Containerfile \
        . || {
        error "Build failed"
        exit 1
    }

    success "Build complete: $FULL_IMAGE"

    # Show image size
    local size
    size=$($CONTAINER_ENGINE images "$FULL_IMAGE" --format "{{.Size}}")
    info "Image size: $size"
}

# Run tests in container
run_tests() {
    info "Running tests in container..."

    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    cd "$SCRIPT_DIR"

    # Run tests during build (already done in Containerfile)
    # But we can also run them separately if needed
    $CONTAINER_ENGINE run --rm \
        -v "$SCRIPT_DIR:/build:ro" \
        -w /build \
        rust:1.83-alpine \
        sh -c "apk add --no-cache musl-dev && cargo test" || {
        error "Tests failed"
        exit 1
    }

    success "All tests passed"
}

# Start interactive shell in container
run_shell() {
    info "Starting interactive shell..."

    $CONTAINER_ENGINE run --rm -it \
        -v "$(pwd):/workspace" \
        --entrypoint /bin/sh \
        "$FULL_IMAGE"
}

# Run vm-translator with arguments
run_translator() {
    local args=("$@")

    # Check if image exists
    if ! $CONTAINER_ENGINE image exists "$FULL_IMAGE"; then
        warn "Image $FULL_IMAGE not found, building..."
        build_image
    fi

    # Run container with arguments
    $CONTAINER_ENGINE run --rm \
        -v "$(pwd):/workspace" \
        "$FULL_IMAGE" \
        "${args[@]}"
}

# Show help
show_help() {
    cat << EOF
vm-translate.sh - Containerized VM Translator Wrapper (Project 08)

USAGE:
    ./vm-translate.sh COMMAND [OPTIONS]

COMMANDS:
    build               Build container image
    test                Run tests in container
    shell               Start interactive shell
    help                Show this help message

TRANSLATION (pass through to vm-translator):
    ./vm-translate.sh <vm-file>         Translate single file
    ./vm-translate.sh <directory>/      Translate directory (bootstrap + all .vm files)
    ./vm-translate.sh -v <vm-file>      Verbose mode

EXAMPLES:
    # Build image
    ./vm-translate.sh build

    # Translate single file (no bootstrap)
    ./vm-translate.sh SimpleAdd.vm

    # Translate directory (with bootstrap code)
    ./vm-translate.sh FunctionCalls/FibonacciElement/

    # Verbose output
    ./vm-translate.sh -v StackTest.vm

    # Interactive debugging
    ./vm-translate.sh shell

ENVIRONMENT:
    CONTAINER_ENGINE    Container engine to use (default: podman)
                       Set to 'docker' to use Docker instead

    Example: CONTAINER_ENGINE=docker ./vm-translate.sh build

REQUIREMENTS:
    - Podman (or Docker)
    - macOS: Podman machine must be running
      (Script will auto-start if not running)

IMAGE INFO:
    Name: $FULL_IMAGE
    Size: ~12MB (Alpine + static binary)
    User: Non-root (vmuser:1000)
    Workspace: /workspace (volume mounted from \$(pwd))

PROJECT 08 FEATURES:
    - Bootstrap code generation (SP=256, call Sys.init)
    - Program flow: label, goto, if-goto
    - Function commands: function, call, return
    - Directory mode for multi-file programs
    - Static variable scoping per file

For more information, see README.md
EOF
}

# ==============================================================================
# Main Script
# ==============================================================================

main() {
    # Check prerequisites
    check_container_engine
    check_podman_machine

    # Parse command
    if [[ $# -eq 0 ]]; then
        show_help
        exit 0
    fi

    case "$1" in
        build)
            build_image
            ;;
        test)
            run_tests
            ;;
        shell)
            run_shell
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            # Pass through to vm-translator
            run_translator "$@"
            ;;
    esac
}

# Run main function with all arguments
main "$@"
