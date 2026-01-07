#!/bin/bash

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_info() {
    echo -e "${YELLOW}ℹ $1${NC}"
}

# Check if Docker is installed
check_docker() {
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    print_success "Docker is installed"
}

# Build Docker image
build_docker_image() {
    print_header "Building Docker Image"

    if docker build -t blog-v2:latest .; then
        print_success "Docker image built successfully"
    else
        print_error "Failed to build Docker image"
        exit 1
    fi
}

# Run Docker container locally
run_local() {
    print_header "Running Local Docker Container"

    print_info "Starting container on port 8080..."
    docker run -p 8080:8080 -e PORT=8080 --name blog-v2-local blog-v2:latest
}

# Test local deployment
test_local() {
    print_header "Testing Local Deployment"

    print_info "Building Docker image..."
    if ! docker build -t blog-v2:test .; then
        print_error "Build failed"
        exit 1
    fi

    print_info "Starting test container..."
    docker run -d -p 8080:8080 -e PORT=8080 --name blog-v2-test blog-v2:test

    # Wait for container to be ready
    print_info "Waiting for container to start (10 seconds)..."
    sleep 10

    # Test endpoint
    print_info "Testing health endpoint..."
    if curl -f http://localhost:8080/ > /dev/null 2>&1; then
        print_success "Health check passed!"
        print_info "Your blog is running at http://localhost:8080"
        print_info "Press Ctrl+C to stop the container"

        # Show logs
        docker logs -f blog-v2-test
    else
        print_error "Health check failed"
        docker logs blog-v2-test
        docker stop blog-v2-test
        docker rm blog-v2-test
        exit 1
    fi
}

# Clean up Docker resources
cleanup() {
    print_header "Cleaning Up Docker Resources"

    # Stop running containers
    if [ "$(docker ps -q -f name=blog-v2)" ]; then
        print_info "Stopping running containers..."
        docker stop $(docker ps -q -f name=blog-v2) 2>/dev/null || true
    fi

    # Remove containers
    if [ "$(docker ps -aq -f name=blog-v2)" ]; then
        print_info "Removing containers..."
        docker rm $(docker ps -aq -f name=blog-v2) 2>/dev/null || true
    fi

    # Remove images
    if [ "$(docker images -q blog-v2)" ]; then
        print_info "Removing images..."
        docker rmi $(docker images -q blog-v2) 2>/dev/null || true
    fi

    print_success "Cleanup complete"
}

# Git commit and push for deployment
push_to_github() {
    print_header "Pushing to GitHub"

    read -p "Enter commit message: " commit_msg

    if [ -z "$commit_msg" ]; then
        print_error "Commit message cannot be empty"
        return 1
    fi

    print_info "Adding files..."
    git add . || { print_error "git add failed"; return 1; }

    print_info "Committing changes..."
    git commit -m "$commit_msg" || { print_error "git commit failed"; return 1; }

    print_info "Pushing to GitHub..."
    git push origin main || { print_error "git push failed"; return 1; }

    print_success "Successfully pushed to GitHub!"
    print_info "Cloudflare Pages will automatically deploy your changes"
}

# Show deployment status
show_status() {
    print_header "Deployment Status"

    print_info "Local Docker information:"
    docker --version

    print_info "\nRunning containers:"
    docker ps --filter "name=blog-v2" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" || echo "No blog-v2 containers running"

    print_info "\nDocker images:"
    docker images --filter "reference=blog-v2*" --format "table {{.Repository}}:{{.Tag}}\t{{.Size}}" || echo "No blog-v2 images found"

    print_info "\nGit status:"
    git status --short || echo "Not a git repository"
}

# Display usage information
show_usage() {
    cat << EOF
${BLUE}Blog v2 Deployment Helper${NC}

Usage: ./deploy.sh [COMMAND]

Commands:
    build           Build Docker image locally
    test            Build and test Docker container locally
    run             Run Docker container (continuous mode)
    clean           Stop and remove Docker containers and images
    push            Commit and push code to GitHub (triggers Cloudflare deployment)
    status          Show deployment status
    help            Show this help message

Examples:
    ./deploy.sh build          # Build the Docker image
    ./deploy.sh test           # Test locally
    ./deploy.sh run            # Run container interactively
    ./deploy.sh push           # Push to GitHub for automatic deployment
    ./deploy.sh clean          # Clean up Docker resources
    ./deploy.sh status         # Check deployment status

Prerequisites:
    - Docker installed
    - Git configured
    - GitHub repository set up
    - Cloudflare Pages connected to your GitHub repo

Deployment Workflow:
    1. Make changes to your blog locally
    2. Run: ./deploy.sh test           (test changes locally)
    3. Run: ./deploy.sh push           (push to GitHub)
    4. Cloudflare automatically deploys your changes!

For more information, see DEPLOYMENT_GUIDE.md
EOF
}

# Main script logic
main() {
    local command=${1:-help}

    case $command in
        build)
            check_docker
            build_docker_image
            ;;
        test)
            check_docker
            test_local
            ;;
        run)
            check_docker
            build_docker_image
            run_local
            ;;
        clean)
            cleanup
            ;;
        push)
            push_to_github
            ;;
        status)
            show_status
            ;;
        help|--help|-h)
            show_usage
            ;;
        *)
            print_error "Unknown command: $command"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
