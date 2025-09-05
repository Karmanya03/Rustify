# Dockerfile optimized for Render.com deployment
FROM rust:1.75 as builder

# Set the working directory
WORKDIR /app

# Copy the entire project (workspace)
COPY ../ ./

# Build the web-backend binary
WORKDIR /app/web-backend
RUN cargo build --release --bin rustify-web-backend

# Runtime stage with minimal image
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/rustify-web-backend ./rustify-web-backend

# Copy static files (frontend) - adjust path for proper serving
COPY --from=builder /app/dist ../dist

# Expose port (Render.com uses 10000 by default)
EXPOSE 10000

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["./rustify-web-backend"]
