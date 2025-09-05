# Multi-stage Docker build for Rustify
# Stage 1: Build the Rust application
FROM rust:1.80-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the web-backend project files
COPY web-backend/Cargo.toml ./web-backend/
COPY web-backend/src ./web-backend/src

# Build the application
WORKDIR /app/web-backend
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bullseye-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -r -s /bin/false rustify

# Set working directory
WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/web-backend/target/release/web-backend /app/rustify-server

# Copy static files (dist folder)
COPY dist ./dist

# Create necessary directories
RUN mkdir -p /app/logs

# Set ownership
RUN chown -R rustify:rustify /app

# Switch to app user
USER rustify

# Expose port
EXPOSE 10000

# Health check - using wget instead of curl for lighter image
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD timeout 3 bash -c "</dev/tcp/localhost/10000" || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=10000

# Start the application
CMD ["./rustify-server"]
