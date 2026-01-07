# Stage 1: Builder
FROM rust:latest as builder

WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy source code and assets
COPY src ./src
COPY posts ./posts
COPY templates ./templates

# Build release binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/BlogWebsite /app/BlogWebsite

# Copy static assets and templates
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/posts ./posts

# Create output directory
RUN mkdir -p /app/output

# Expose port
EXPOSE 8080

# Set environment variable
ENV PORT=8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/ || exit 1

# Run application
CMD ["./BlogWebsite"]
