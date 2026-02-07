#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="hack-assembler:latest"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check and start Podman machine on macOS
check_podman() {
    # Check if podman is available
    if ! command -v podman &> /dev/null; then
        echo -e "${RED}Error: Podman not found${NC}"
        echo "Install with: brew install podman"
        exit 1
    fi
    
    # On macOS, ensure podman machine is running
    if [[ "$OSTYPE" == "darwin"* ]]; then
        if ! podman machine list --format "{{.Running}}" 2>/dev/null | grep -q "true"; then
            echo -e "${YELLOW}Podman machine not running. Starting...${NC}"
            
            # Check if any machine exists
            if ! podman machine list --format "{{.Name}}" 2>/dev/null | grep -q .; then
                echo -e "${YELLOW}No Podman machine found. Initializing...${NC}"
                podman machine init
            fi
            
            # Start the machine
            podman machine start
            
            echo -e "${GREEN}✓ Podman machine started${NC}"
        fi
    fi
}

print_usage() {
    cat << EOF
Hack Assembler - Containerized Build & Run Script

USAGE:
    ./assemble.sh build              Build the container image
    ./assemble.sh <file.asm>         Assemble a single file
    ./assemble.sh <file1.asm> ...    Assemble multiple files
    ./assemble.sh test               Run all tests
    ./assemble.sh shell              Open shell in container

EXAMPLES:
    ./assemble.sh build
    ./assemble.sh Add.asm
    ./assemble.sh *.asm -v
    ./assemble.sh test

EOF
}

build_image() {
    check_podman
    echo -e "${YELLOW}Building Hack Assembler container...${NC}"
    podman build -t "$IMAGE_NAME" -f "$SCRIPT_DIR/Containerfile" "$SCRIPT_DIR"
    echo -e "${GREEN}✓ Build complete${NC}"
}

run_tests() {
    check_podman
    echo -e "${YELLOW}Running tests...${NC}"
    cd "$SCRIPT_DIR"
    podman run --rm -it \
        -v "$SCRIPT_DIR:/build:ro" \
        docker.io/rust:alpine \
        sh -c "cd /build && cargo test --release"
    echo -e "${GREEN}✓ All tests passed${NC}"
}

run_assembler() {
    check_podman
    
    # Check if image exists
    if ! podman image exists "$IMAGE_NAME"; then
        echo -e "${YELLOW}Image not found. Building...${NC}"
        build_image
    fi

    # Get absolute path of current directory
    WORKSPACE="$(pwd)"
    
    # Run assembler with mounted workspace
    podman run --rm \
        -v "$WORKSPACE:/workspace:rw" \
        "$IMAGE_NAME" "$@"
}

open_shell() {
    check_podman
    
    if ! podman image exists "$IMAGE_NAME"; then
        echo -e "${YELLOW}Image not found. Building...${NC}"
        build_image
    fi
    
    WORKSPACE="$(pwd)"
    podman run --rm -it \
        -v "$WORKSPACE:/workspace:rw" \
        --entrypoint /bin/sh \
        "$IMAGE_NAME"
}

# Main logic
if [ $# -eq 0 ]; then
    print_usage
    exit 1
fi

case "$1" in
    build)
        build_image
        ;;
    test)
        run_tests
        ;;
    shell)
        open_shell
        ;;
    -h|--help|help)
        print_usage
        ;;
    *)
        run_assembler "$@"
        ;;
esac
