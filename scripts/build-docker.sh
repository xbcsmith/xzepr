#!/bin/bash

# XZEPR Docker Build Script
# This script builds the XZEPR server Docker image using Red Hat UBI 9

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
IMAGE_NAME="xzepr"
IMAGE_TAG="latest"
BUILD_CONTEXT="."
DOCKERFILE="Dockerfile"
PUSH_IMAGE=false
REGISTRY=""
BUILD_ARGS=""
PLATFORM=""

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

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build XZEPR server Docker image using Red Hat UBI 9 base image.

OPTIONS:
    -n, --name NAME         Image name (default: xzepr)
    -t, --tag TAG          Image tag (default: latest)
    -r, --registry URL     Registry URL for pushing
    -p, --push             Push image to registry after build
    -f, --dockerfile FILE  Dockerfile path (default: Dockerfile)
    -c, --context PATH     Build context path (default: .)
    --platform PLATFORM    Target platform (e.g., linux/amd64,linux/arm64)
    --build-arg ARG=VALUE  Pass build arguments
    --no-cache             Don't use cache when building
    --help                 Show this help message

EXAMPLES:
    # Basic build
    $0

    # Build with custom tag
    $0 -t v1.0.0

    # Build and push to registry
    $0 -r registry.example.com -p -t v1.0.0

    # Multi-platform build
    $0 --platform linux/amd64,linux/arm64 -t latest

    # Build with custom build args
    $0 --build-arg RUST_VERSION=1.75 -t dev

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--name)
            IMAGE_NAME="$2"
            shift 2
            ;;
        -t|--tag)
            IMAGE_TAG="$2"
            shift 2
            ;;
        -r|--registry)
            REGISTRY="$2"
            shift 2
            ;;
        -p|--push)
            PUSH_IMAGE=true
            shift
            ;;
        -f|--dockerfile)
            DOCKERFILE="$2"
            shift 2
            ;;
        -c|--context)
            BUILD_CONTEXT="$2"
            shift 2
            ;;
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        --build-arg)
            if [[ -n "$BUILD_ARGS" ]]; then
                BUILD_ARGS="$BUILD_ARGS --build-arg $2"
            else
                BUILD_ARGS="--build-arg $2"
            fi
            shift 2
            ;;
        --no-cache)
            BUILD_ARGS="$BUILD_ARGS --no-cache"
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Construct full image name
if [[ -n "$REGISTRY" ]]; then
    FULL_IMAGE_NAME="${REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"
else
    FULL_IMAGE_NAME="${IMAGE_NAME}:${IMAGE_TAG}"
fi

# Validate build context
if [[ ! -d "$BUILD_CONTEXT" ]]; then
    print_error "Build context directory does not exist: $BUILD_CONTEXT"
    exit 1
fi

# Validate Dockerfile
if [[ ! -f "$BUILD_CONTEXT/$DOCKERFILE" ]]; then
    print_error "Dockerfile does not exist: $BUILD_CONTEXT/$DOCKERFILE"
    exit 1
fi

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed or not in PATH"
    exit 1
fi

# Check if Docker daemon is running
if ! docker info &> /dev/null; then
    print_error "Docker daemon is not running"
    exit 1
fi

print_info "Starting XZEPR Docker build..."
print_info "Image: $FULL_IMAGE_NAME"
print_info "Context: $BUILD_CONTEXT"
print_info "Dockerfile: $DOCKERFILE"

# Build Docker command
DOCKER_CMD="docker build"

if [[ -n "$PLATFORM" ]]; then
    DOCKER_CMD="$DOCKER_CMD --platform $PLATFORM"
fi

if [[ -n "$BUILD_ARGS" ]]; then
    DOCKER_CMD="$DOCKER_CMD $BUILD_ARGS"
fi

DOCKER_CMD="$DOCKER_CMD -f $BUILD_CONTEXT/$DOCKERFILE -t $FULL_IMAGE_NAME $BUILD_CONTEXT"

# Execute build
print_info "Executing: $DOCKER_CMD"
if eval "$DOCKER_CMD"; then
    print_success "Docker image built successfully: $FULL_IMAGE_NAME"
else
    print_error "Docker build failed"
    exit 1
fi

# Show image size
IMAGE_SIZE=$(docker images --format "table {{.Size}}" "$FULL_IMAGE_NAME" | tail -n 1)
print_info "Image size: $IMAGE_SIZE"

# Push image if requested
if [[ "$PUSH_IMAGE" == true ]]; then
    if [[ -z "$REGISTRY" ]]; then
        print_error "Cannot push image: no registry specified"
        exit 1
    fi

    print_info "Pushing image to registry..."
    if docker push "$FULL_IMAGE_NAME"; then
        print_success "Image pushed successfully: $FULL_IMAGE_NAME"
    else
        print_error "Failed to push image"
        exit 1
    fi
fi

# Security scan (if available)
if command -v docker scan &> /dev/null; then
    print_info "Running security scan..."
    docker scan "$FULL_IMAGE_NAME" || print_warning "Security scan failed or not available"
fi

# Show final information
print_success "Build completed!"
echo
echo "Image Details:"
echo "  Name: $FULL_IMAGE_NAME"
echo "  Size: $IMAGE_SIZE"
echo
echo "To run the container:"
echo "  docker run -d -p 8443:8443 --name xzepr-server $FULL_IMAGE_NAME"
echo
echo "To run with docker-compose:"
echo "  docker-compose -f docker-compose.prod.yaml up -d"
echo
