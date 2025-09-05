# Dockerfile optimized for Render.com deployment
FROM rust:1.80-slim-bookworm as builder

# Set the working directory
WORKDIR /app

# Copy only the web-backend project
COPY . ./

# Build the web-backend binary
RUN cargo build --release

# Runtime stage with minimal image
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/web-backend ./web-backend

# Copy static files (frontend)
COPY ../dist ./dist

# Expose port (Render.com uses 10000 by default)
EXPOSE 10000

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["./web-backend"]
