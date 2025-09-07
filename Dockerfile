# Multi-stage Docker build for Rustify
# Stage 1: Build the Rust application
FROM rust:1.83-slim-bookworm AS builder

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
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies including Python, browsers, and yt-dlp
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    wget \
    gnupg \
    python3 \
    python3-pip \
    python3-venv \
    ffmpeg \
    # Browser dependencies for Selenium
    fonts-liberation \
    libasound2 \
    libatk-bridge2.0-0 \
    libatk1.0-0 \
    libatspi2.0-0 \
    libcups2 \
    libdbus-1-3 \
    libdrm2 \
    libgbm1 \
    libgtk-3-0 \
    libnspr4 \
    libnss3 \
    libwayland-client0 \
    libxcomposite1 \
    libxdamage1 \
    libxfixes3 \
    libxkbcommon0 \
    libxrandr2 \
    xvfb \
    # Additional libraries for headless browser (complete set)
    libxss1 \
    libatspi2.0-0 \
    libgdk-pixbuf2.0-0 \
    && rm -rf /var/lib/apt/lists/*

# Install Google Chrome (more reliable than Chromium for Selenium)
RUN wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | gpg --dearmor -o /usr/share/keyrings/googlechrome-linux-keyring.gpg \
    && echo "deb [arch=amd64 signed-by=/usr/share/keyrings/googlechrome-linux-keyring.gpg] http://dl.google.com/linux/chrome/deb/ stable main" > /etc/apt/sources.list.d/google-chrome.list \
    && apt-get update \
    && apt-get install -y google-chrome-stable \
    && rm -rf /var/lib/apt/lists/*

# Create virtual environment and install yt-dlp with browser automation tools
RUN python3 -m venv /opt/venv && \
    /opt/venv/bin/pip install --no-cache-dir --upgrade pip && \
    /opt/venv/bin/pip install --no-cache-dir yt-dlp selenium webdriver-manager beautifulsoup4 requests \
    fake-useragent pysocks stem random-user-agent proxy-randomizer aiohttp asyncio

# Add virtual environment to PATH
ENV PATH="/opt/venv/bin:$PATH"

# Set display for headless browser
ENV DISPLAY=:99

# Fix Chrome sandbox issues in containers
ENV CHROME_BIN=/usr/bin/google-chrome
ENV CHROME_PATH=/usr/bin/google-chrome
ENV CHROME_FLAGS="--no-sandbox --disable-dev-shm-usage --disable-gpu --memory-pressure-off"

# Create app user for security
RUN useradd -r -s /bin/false rustify

# Set working directory
WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/web-backend/target/release/web-backend /app/rustify-server

# Copy the Selenium Python scripts
COPY web-backend/src/selenium_extractor.py /app/src/selenium_extractor.py
COPY web-backend/src/anti_detection.py /app/src/anti_detection.py

# Copy static files (dist folder)
COPY dist ./dist

# Create necessary directories
RUN mkdir -p /app/logs /app/downloads

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
ENV USE_SELENIUM=true
ENV DISPLAY=:99

# Create a startup script for proper initialization
COPY <<EOF /app/startup.sh
#!/bin/bash
set -e

# Start Xvfb for headless browser support
Xvfb :99 -screen 0 1920x1080x24 -nolisten tcp -dpi 96 &
XVFB_PID=\$!

# Wait a moment for Xvfb to start
sleep 2

# Function to cleanup on exit
cleanup() {
    echo "Shutting down..."
    kill \$XVFB_PID 2>/dev/null || true
    wait \$XVFB_PID 2>/dev/null || true
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Start the main application
exec ./rustify-server
EOF

# Make startup script executable
RUN chmod +x /app/startup.sh

# Start the application using the startup script
CMD ["/app/startup.sh"]
