# Dockerfile optimized for Render.com deployment
FROM rust:1.83-slim-bookworm as builder

# Set the working directory
WORKDIR /app

# Copy only the web-backend project
COPY . ./

# Build the web-backend binary
RUN cargo build --release

# Runtime stage with minimal image
FROM debian:bookworm-slim

# Install necessary runtime dependencies including Python and yt-dlp
RUN apt-get update && apt-get install -y \
    ca-certificates \
    pkg-config \
    libssl-dev \
    python3 \
    python3-pip \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Install yt-dlp with --break-system-packages flag for Docker environment
RUN pip3 install --no-cache-dir --break-system-packages yt-dlp

# Set working directory
WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/target/release/web-backend ./web-backend

# Copy static files (frontend)
COPY dist ./dist

# Expose port (Render.com uses 10000 by default)
EXPOSE 10000

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["./web-backend"]
